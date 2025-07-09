mod advection;
mod curl;
mod divergence;
mod pressure;
mod gradient_subtract;
mod vorticity;
mod velocity_out;
mod fluid;
mod display;
mod compute_shader_game_of_life;

use bevy::{
    prelude::*,
    render::{
        render_resource::{AsBindGroup, ShaderRef, ShaderType},
        texture::ImageSampler
    },
    sprite::{Material2d, Material2dPlugin},
};
use bevy::pbr::MaterialPipeline;
use bevy::render::camera::RenderTarget;
use bevy::render::mesh::{Indices, MeshVertexBufferLayout};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{CommandEncoderDescriptor, CompareFunction, ComputePassDescriptor, DepthStencilState, Extent3d, ImageDataLayout, PipelineCache, RenderPipelineDescriptor, SpecializedMeshPipelineError, TextureDimension, TextureFormat, TextureUsages};
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::{Render, RenderPlugin};
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::settings::{Backends, WgpuSettings};
use bevy::render::texture::TextureFormatPixelInfo;
use bevy::sprite::{Material2dKey, MaterialMesh2dBundle};
use bevy::utils::petgraph::visit::NodeRef;
use bevy::window::PrimaryWindow;
use rand::seq::SliceRandom;
use crate::advection::{AdvectionBindGroup, AdvectionImage, AdvectionPipeline, AdvectionPlugin};
use crate::compute_shader_game_of_life::{GameOfLifeComputePlugin, GameOfLifeImage};
use crate::curl::{CurlBindGroup, CurlImage, CurlPipeline, CurlPlugin};
use crate::divergence::{DivergenceImage, DivergencePlugin};
use crate::gradient_subtract::{GradientSubtractBindGroup, GradientSubtractImage, GradientSubtractPipeline, GradientSubtractPlugin};
use crate::pressure::{PressureBindGroup, PressureImage, PressurePipeline, PressurePlugin};
use crate::velocity_out::{VelocityOutBindGroup, VelocityOutImage, VelocityOutPipeline, VelocityOutPlugin};
use crate::vorticity::{VorticityBindGroup, VorticityImage, VorticityPipeline, VorticityPlugin};

pub const WIDTH: u32 = 600;
pub const HEIGHT: u32 = 400;
pub const SIZE: (u32, u32) = (WIDTH, HEIGHT);
pub const WORKGROUP_SIZE: u32 = 8;
///平流(Advection)	初始速度场	更新速度场
// 涡度计算(Curl)	平流后的速度场	计算流体旋转
// 散度计算(Divergence)	速度场	计算不可压缩性
// 压力求解(Pressure)	散度场	校正速度场
// 梯度减法(Gradient Subtract)	压力场	最终速度场

// 纹理资源
#[derive(Resource,Default,ExtractResource,Clone)]
struct FluidTextures {
    velocity: (Handle<Image>, Handle<Image>),
    density: (Handle<Image>, Handle<Image>),
    pressure: (Handle<Image>, Handle<Image>),
    curl: Handle<Image>,
    divergence: Handle<Image>,
    burns: Handle<Image>,
    cells: Handle<Image>,
    velocity_out: Handle<Image>,
    output: Handle<Image>,
}

// 流体配置参数
#[derive(Resource,ExtractResource,Clone)]
struct FluidConfig {
    velocity_dissipation: f32,
    density_dissipation: f32,
    curl_strength: f32,
    pressure_dissipation: f32,
    pressure_iterations: u32,
}
impl Default for FluidConfig {
    fn default() -> Self {
        println!("FluidConfig_default");
        Self{
            velocity_dissipation: 0.0,
            density_dissipation:  0.0,
            curl_strength:  0.0,
            pressure_dissipation:  0.0,
            pressure_iterations:  0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash,Default)]
#[repr(u8)]
pub enum Species {
    #[default]
    Empty = 0,
    Wall = 1,
    Sand = 2,
    Water = 3,
    Stone = 13,
    Ice = 9,
    Gas = 4,
    Cloner = 5,
    Mite = 15,
    Wood = 7,
    Plant = 11,
    Fungus = 18,
    Seed = 19,
    Fire = 6,
    Lava = 8,
    Acid = 12,
    Dust = 14,
    Oil = 16,
    Rocket = 17,
}
impl Species {
    pub fn random_active() -> Species {
        let mut rng = rand::thread_rng();
        *Species::all_active()
            .choose(&mut rng)
            .expect("至少有一个活跃物质")
    }

    pub fn all_active() -> Vec<Species> {
        vec![
            Species::Sand,
            Species::Water,
            Species::Gas,
            Species::Cloner,
            Species::Fire,
            Species::Wood,
            Species::Lava,
            Species::Ice,
            Species::Plant,
            Species::Acid,
            Species::Stone,
            Species::Dust,
            Species::Mite,
            Species::Oil,
            Species::Rocket,
            Species::Fungus,
            Species::Seed,
        ]
    }

}
// 细胞结构
#[derive(Clone, Copy,Default,Debug)]
pub struct Cell {
    pub species: Species,
    pub ra: u8,
    pub rb: u8,
    pub clock: u8,
}

// 核心数据结构
#[derive(Resource)]
struct CellGrid {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
}

impl CellGrid {
    fn new(width: usize, height: usize) -> Self {
        let mut cells = Vec::with_capacity(width * height);

        for i in 0..width * height{
            // cells.push(Cell {
            //             species: Species::random_active(),
            //             ra: 0,
            //             rb: 0,
            //             clock: 0,
            //         });
            if i>width * height/2 {
                // if i%2==0 {
                cells.push(Cell {
                    species: Species::Water,
                    ra: 0,
                    rb: 0,
                    clock: 0,
                }
                );

            }else{
                cells.push( Cell {
                    species: Species::Empty,
                    ra: 0,
                    rb: 0,
                    clock: 0,
                });
            }
        };

        Self { width, height, cells }
    }

    fn get(&self, x: usize, y: usize) -> Option<&Cell> {
        if x < self.width && y < self.height {
            self.cells.get(y * self.width + x)
        } else {
            None
        }
    }

    fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut Cell> {
        if x < self.width && y < self.height {
            self.cells.get_mut(y * self.width + x)
        } else {
            None
        }
    }
}
// 自定义材质
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
struct CellMaterial {
    #[uniform(0)]  // 第一个绑定位置
    params: ShaderParams,
    #[texture(1)]  // 第二个绑定位置
    #[sampler(2)]  // 第三个绑定位置
    // #[storage_texture(1, image_format = Rgba8Unorm, access = ReadWrite)]
    data_tex: Handle<Image>,

}

#[derive(ShaderType, Clone, Debug)]
struct ShaderParams {
    time: f32,
    dpi: f32,
    resolution: Vec2,
    is_snapshot: u32,
}

#[derive(Component)]
struct Rotating {
    speed: f32,
}


impl Material2d for CellMaterial {




    fn fragment_shader() -> ShaderRef {
        "sand.wgsl".into()
    }
    fn vertex_shader() -> ShaderRef {
        "sand.wgsl".into()
    }
}

// 主应用

#[derive(Resource)]
struct Falg(usize);


// 新的渲染系统
#[derive(Component)]
struct CellCanvas;

fn  setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CellMaterial>>,
    mut images: ResMut<Assets<Image>>,
    // primary_window: Query<PrimaryWindow>,
    mut fluid_textures: ResMut<FluidTextures>,
    mut fluid_config: ResMut<FluidConfig>,
)
{
    // 创建全屏四边形
    let mut mesh = Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleList,
                             RenderAssetUsages::all()
    );

    mesh.insert_attribute(

        Mesh::ATTRIBUTE_POSITION,

        vec![

            [-1.0, -1.0, 0.0],

            [1.0, -1.0, 0.0],

            [1.0, 1.0, 0.0],

            [-1.0, 1.0, 0.0],

        ],

    );

    mesh.insert_attribute(

        Mesh::ATTRIBUTE_UV_0,

        vec![

            [0.0, 0.0],

            [1.0, 0.0],

            [1.0, 1.0],

            [0.0, 1.0],

        ],

    );



    mesh.insert_indices(Indices::U32(vec![0, 1, 2, 0, 2, 3]));

    let quad = meshes.add(mesh);
    fn create_texture(images: &mut Assets<Image>) -> Handle<Image> {
        let mut image = Image::new_fill(
            Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
            TextureDimension::D2,
            &[0u8; 4],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::nearest();
        image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_DST;
        images.add(image)
    }

    // 创建存储纹理的函数
    fn create_storage_texture(images: &mut Assets<Image>) -> Handle<Image> {
        let mut image = Image::new_fill(
            Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 },
            TextureDimension::D2,
            &[0u8; 4],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::nearest();
        image.texture_descriptor.usage = TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_DST | TextureUsages::COPY_SRC;
        images.add(image)
    }

    // let data_tex_handle = images.add(image); // 强引用在此处创建
    let cc=create_texture(&mut images);
    // 创建材质
    let material = materials.add(CellMaterial {
        data_tex: cc.clone(),
        // data_tex: data_tex_handle.clone(),
        params: ShaderParams {
            time: 0.0,
            dpi: 2.0,
            resolution: Vec2::new(768.0, 768.0),
            is_snapshot: 0,
        },
    });

    commands.spawn((
        MaterialMesh2dBundle {
            mesh: quad.into(),
            material: material.clone(),
            transform: Transform::from_translation(Vec3::new(300.0, 0.0, 0.0)),
            ..default()
        },
        CellCanvas,
        Rotating { speed: 1.0 },
    ));

    commands.spawn(Camera2dBundle {
        camera: Camera {
            clear_color: Color::WHITE.into(),
            ..default()
        },
        ..default()
    });

    // 初始化流体配置
    fluid_config.velocity_dissipation = 0.99;
    fluid_config.density_dissipation = 0.99;
    fluid_config.curl_strength = 3.0;
    fluid_config.pressure_dissipation = 0.99;
    fluid_config.pressure_iterations = 20;

    // 创建流体模拟所需的纹理
    // let mut create_texture = || {
    //     let mut image = Image::new_fill(Extent3d { width: WIDTH, height: HEIGHT, depth_or_array_layers: 1 }, TextureDimension::D2, &[0, 0, 0, 0], TextureFormat::Rgba8Unorm, Default::default());
    //     image.texture_descriptor.usage =
    //         TextureUsages::TEXTURE_BINDING |
    //             TextureUsages::STORAGE_BINDING |
    //             TextureUsages::RENDER_ATTACHMENT;
    //     images.add(image)
    // };
    // 初始化所有纹理
    // fluid_textures.velocity = (create_texture(), create_texture());
    // fluid_textures.density = (create_texture(), create_texture());
    // fluid_textures.pressure = (create_texture(), create_texture());
    // fluid_textures.curl = create_texture();
    // fluid_textures.divergence = create_texture();
    // fluid_textures.burns = create_texture();
    // fluid_textures.cells = create_texture();
    // fluid_textures.velocity_out = create_texture();
    fluid_textures.velocity = (create_texture(&mut images),create_storage_texture(&mut images) );
    fluid_textures.density = (create_texture(&mut images), create_texture(&mut images));
    fluid_textures.pressure = (create_texture(&mut images), create_storage_texture(&mut images));
    fluid_textures.curl = create_storage_texture(&mut images);
    fluid_textures.divergence =  create_storage_texture(&mut images);
    fluid_textures.burns = create_texture(&mut images);
    fluid_textures.cells = create_texture(&mut images);
    fluid_textures.velocity_out = create_storage_texture(&mut images);

    // commands.insert_resource(GameOfLifeImage { texture: cc });
    // 初始化AdvectionImage资源
    commands.insert_resource(AdvectionImage {
        velocity_tex: fluid_textures.velocity.0.clone(),
        source_tex: fluid_textures.density.0.clone(),
        wind_tex: fluid_textures.cells.clone(),
        output_tex: fluid_textures.velocity.1.clone(),
    });
    // 初始化CurlImage资源
    commands.insert_resource(CurlImage {
        velocity_tex: fluid_textures.velocity.0.clone(),
        output_tex: fluid_textures.curl.clone(),
    });
    // 初始化VorticityImage资源
    commands.insert_resource(VorticityImage {
        velocity_tex: fluid_textures.velocity.0.clone(),
        curl_tex: fluid_textures.curl.clone(),
        output_tex: fluid_textures.velocity.1.clone(),
    });
    commands.insert_resource(DivergenceImage {
        velocity_tex: fluid_textures.velocity.0.clone(),
        output_tex: fluid_textures.divergence.clone(),
    });
    // 初始化PressureImage资源
    commands.insert_resource(PressureImage {
        pressure_tex: fluid_textures.pressure.0.clone(),
        divergence_tex: fluid_textures.divergence.clone(),
        output_tex: fluid_textures.pressure.1.clone(),
    });
    // 初始化VelocityOutImage资源
    commands.insert_resource(VelocityOutImage {
        velocity_tex: fluid_textures.velocity.0.clone(),
        pressure_tex: fluid_textures.pressure.0.clone(),
        output_tex: fluid_textures.velocity_out.clone(),
    });

    // 初始化GradientSubtractImage资源
    commands.insert_resource(GradientSubtractImage {
        pressure_tex: fluid_textures.pressure.0.clone(),
        velocity_tex: fluid_textures.velocity.0.clone(),
        wind_tex: fluid_textures.burns.clone(),
        cells_tex: fluid_textures.cells.clone(),
        output_tex: fluid_textures.velocity.1.clone(),
    });
}
fn rotate_system(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Rotating)>,
) {
    for (mut transform, rotating) in query.iter_mut() {
        let angle = (time.elapsed_seconds() * rotating.speed).sin() * 30.0; // -30度到+30度之间摆动
        transform.rotation = Quat::from_rotation_z(angle.to_radians());
    }
}
fn update_fluid_simulation_1(
    mut fluid_textures: ResMut<FluidTextures>,
    fluid_config: Res<FluidConfig>,
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut render_device: ResMut<RenderDevice>,
    advection_pipeline: Res<AdvectionPipeline>,
    advection_bind_group: Res<AdvectionBindGroup>,
    curl_pipeline: Res<CurlPipeline>,
    curl_bind_group: Res<CurlBindGroup>,
    vorticity_pipeline: Res<VorticityPipeline>,
    vorticity_bind_group: Res<VorticityBindGroup>,
    pressure_pipeline: Res<PressurePipeline>,
    pressure_bind_group: Res<PressureBindGroup>,
) {

    // 执行流体模拟的各个步骤
    // 注意：实际实现需要为每个计算步骤创建对应的计算着色器和Pipeline
    let mut encoder= render_device.create_command_encoder(&CommandEncoderDescriptor::default());
    // 1. 平流计算 advection #
    // 执行平流计算
    {

        let mut pass =encoder
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("Advection Compute Pass"),
                ..default()
            });

        if let Some(pipeline) = pipeline_cache.get_compute_pipeline(advection_pipeline.pipeline) {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &advection_bind_group.0, &[]);
            pass.dispatch_workgroups(WIDTH / WORKGROUP_SIZE, HEIGHT / WORKGROUP_SIZE, 1);
        }

    }

    // 2. 燃烧和细胞纹理更新
    // 这部分需要从WASM内存读取数据或通过其他方式更新

    // 3. 密度平流 advection #
    // run_advection_compute(
    //     &mut commands,
    //     &mut compute_pipelines,
    //     &pipeline_cache,
    //     &fluid_textures.density.0,
    //     &fluid_textures.density.1,
    //     dt,
    //     fluid_config.density_dissipation,
    // );

    // 4. 处理用户交互（喷溅）
    // 实现略...
    //
    // 5. 计算旋度 curl #
    // 执行旋度计算
    {

        let mut pass = encoder
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("Curl Compute Pass"),
                ..Default::default()
            });

        if let Some(pipeline) = pipeline_cache.get_compute_pipeline(curl_pipeline.pipeline) {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &curl_bind_group.0, &[]);
            pass.dispatch_workgroups(WIDTH / WORKGROUP_SIZE, HEIGHT / WORKGROUP_SIZE, 1);
        }

    }


    // 6. 应用涡度 vorticity #
    // 执行涡度应用计算
    {
        let mut pass = encoder
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("Vorticity Compute Pass"),
                ..default()
            });

        if let Some(pipeline) = pipeline_cache.get_compute_pipeline(vorticity_pipeline.pipeline) {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &vorticity_bind_group.0, &[]);
            pass.dispatch_workgroups(WIDTH / WORKGROUP_SIZE, HEIGHT / WORKGROUP_SIZE, 1);
        }
        // 交换速度场缓冲区
        let vel = &mut fluid_textures.velocity;
        std::mem::swap(&mut vel.0, &mut vel.1);

    }

    // 7. 计算散度  divergence #
    // 初始化DivergenceImage资源
    {
        let mut pass = encoder
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("Vorticity Compute Pass"),
                ..default()
            });

        if let Some(pipeline) = pipeline_cache.get_compute_pipeline(pressure_pipeline.pipeline) {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &pressure_bind_group.0, &[]);
            pass.dispatch_workgroups(WIDTH / WORKGROUP_SIZE, HEIGHT / WORKGROUP_SIZE, 1);
        }
        // 交换速度场缓冲区
        let vel = &mut fluid_textures.velocity;
        std::mem::swap(&mut vel.0, &mut vel.1);

    }

    // 8. 压力场求解（迭代） pressure #
    for _ in 0..fluid_config.pressure_iterations {
        let mut pass = encoder
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("Pressure Compute Pass"),
                ..default()
            });

        if let Some(pipeline) = pipeline_cache.get_compute_pipeline(pressure_pipeline.pipeline) {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &pressure_bind_group.0, &[]);
            pass.dispatch_workgroups(WIDTH / WORKGROUP_SIZE, HEIGHT / WORKGROUP_SIZE, 1);
        }


        // 交换读写缓冲区，为下一次迭代做准备
        let vel = &mut fluid_textures.pressure;
        std::mem::swap(&mut vel.0, &mut vel.1);
        // std::mem::swap(&mut fluid_textures.pressure.0, &mut fluid_textures.pressure.1);

    }


}
fn update_fluid_simulation_2(
    mut fluid_textures: ResMut<FluidTextures>,
    pipeline_cache: Res<PipelineCache>,
    mut render_device: ResMut<RenderDevice>,
    velocity_clamp_pipeline: Res<VelocityOutPipeline>,
    velocity_clamp_bind_group: Res<VelocityOutBindGroup>,
    gradient_subtract_pipeline: Res<GradientSubtractPipeline>,
    gradient_subtract_bind_group: Res<GradientSubtractBindGroup>,
) {
    let mut encoder= render_device.create_command_encoder(&CommandEncoderDescriptor::default());
    // 9. 速度场修正 velocity #
    // 执行速度场修正
    {
        let mut pass = encoder
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("Velocity Out Compute Pass"),
                ..default()
            });

        if let Some(pipeline) = pipeline_cache.get_compute_pipeline(velocity_clamp_pipeline.pipeline) {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &velocity_clamp_bind_group.0, &[]);
            pass.dispatch_workgroups(WIDTH / WORKGROUP_SIZE, HEIGHT / WORKGROUP_SIZE, 1);
        }

    }



    // 10. 梯度减法 gradient_subtract #
    // 执行梯度减法（速度场修正）
    {
        let mut pass = encoder
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("Gradient Subtract Compute Pass"),
                ..default()
            });

        if let Some(pipeline) = pipeline_cache.get_compute_pipeline(gradient_subtract_pipeline.pipeline) {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &gradient_subtract_bind_group.0, &[]);
            pass.dispatch_workgroups(WIDTH / WORKGROUP_SIZE, HEIGHT / WORKGROUP_SIZE, 1);
        }
    }
    let vel = &mut fluid_textures.velocity;
    std::mem::swap(&mut vel.0, &mut vel.1);
    // 11. 交换速度缓冲区
    // std::mem::swap(&mut fluid_textures.velocity.0, &mut fluid_textures.velocity.1);


    // // 12. 显示结果
    // update_display_texture(
    //     &mut commands,
    //     &mut render_pipelines,
    //     &pipeline_cache,
    //     &fluid_textures.density.0,
    //     gpu_images.single().unwrap(),
    // );
}


// fn handle_mouse_click(
//     mut images: ResMut<Assets<Image>>,
//     mut materials: ResMut<Assets<ColorMaterial>>,
//     buttons: Res<ButtonInput<MouseButton>>,
//     material_query: Query<&Handle<ColorMaterial>>,
// )
// {
//     if buttons.just_pressed(MouseButton::Left) {
//         for material_handle in material_query.iter() {
//             if let Some(material) = materials.get_mut(material_handle) {
//                 if let Some(image_handle) = material.texture.as_mut() {
//                     if let Some(image) = images.get_mut(image_handle.id()) {
//                     } } } } } }
// 更新纹理数据
fn update_texture_data(
    grid: Res<CellGrid>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<CellMaterial>>,
    material_query: Query<&Handle<CellMaterial>>,

) {
    // 在update_texture_data中添加对齐检查
    for material_handle in material_query.iter() {
        if let Some(material) = materials.get_mut(material_handle) {
            if let Some(image) = images.get_mut(&material.data_tex) {
                let format = image.texture_descriptor.format;
                let bytes_per_row = image.width() * format.pixel_size() as u32;
                 let pixels = image.data.as_mut_slice();


                for (i, cell) in grid.cells.iter().enumerate() {
                    let idx = i * 4;

                    // pixels[idx] = cell.species as u8;
                    // let s=Species::random_active() as u8;
                    let s=cell.species as u8;
                    pixels[idx] = s;
                    // pixels[idx + i*4] = s;
                    // pixels[idx +  i*8] = s;
                    // pixels[idx +  i*12] = s;
                    // pixels[idx + 1] = cell.ra;
                    // pixels[idx + 2] = cell.rb;
                    // pixels[idx + 3] = cell.clock;
                    // if cell.species == Species::Fire {
                        // let center_x = grid.width / 2;
                        // let center_y = grid.height / 2;
                        // let idx = (center_y * grid.width + center_x) * 4;
                        // println!("Center pixel: {:?}:cell: {:?}", &pixels[idx], cell);
                    // }
                }
                // let data_layout = ImageDataLayout {
                //     offset: 0,
                //     bytes_per_row: Some(bytes_per_row),
                //     rows_per_image: None,
                // };
                // let data_texture = render_device.create_texture(&image.texture_descriptor);
                //
                // render_queue.write_texture(
                //     data_texture.as_image_copy(),
                //     &image.data,
                //     data_layout,
                //     image.texture_descriptor.size,
                // );

            }
        }
    }
}
#[derive(Resource, Default)]
struct LastMousePos(Option<Vec2>);

fn handle_input(
    mut grid: ResMut<CellGrid>,
    windows: Query<&Window>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut last_pos: ResMut<LastMousePos>,
) {
    let window = windows.single();

    // 获取当前鼠标位置
    let current_pos = if let Some(pos) = window.cursor_position() {
        pos
    } else {
        last_pos.0 = None;
        return;
    };

    // 转换坐标为网格坐标


    if buttons.pressed(MouseButton::Left) {
        let to_grid_pos = |pos: Vec2| -> (usize, usize) {
            let x = (pos.x / window.width() * grid.width as f32) as usize;
            let y = grid.height - 1 - (pos.y / window.height() * grid.height as f32) as usize;
            (x.clamp(0, grid.width-1), y.clamp(0, grid.height-1))
        };
        // 如果有上一帧位置，进行插值
        if let Some(last) = last_pos.0 {
            // 计算两点间距离
            let current_grid = to_grid_pos(current_pos);
            let last_grid = to_grid_pos(last);

            // 使用 Bresenham 算法绘制连续线段
            for (x, y) in line_drawing::Bresenham::new(
                (last_grid.0 as i32, last_grid.1 as i32),
                (current_grid.0 as i32, current_grid.1 as i32)
            ) {
                let x = x.clamp(0, grid.width as i32 - 1) as usize;
                let y = y.clamp(0, grid.height as i32 - 1) as usize;

                if let Some(cell) = grid.get_mut(x, y) {
                    *cell = Cell {
                        species: Species::Water,
                        ra: 0,
                        rb: 0,
                        clock: 0,
                    };
                }
            }
        }else{
            // 绘制当前点
            let (x, y) = to_grid_pos(current_pos);
            if let Some(cell) = grid.get_mut(x, y) {
                *cell = Cell {
                    species: Species::Water,
                    ra: 0,
                    rb: 0,
                    clock: 0,
                };
            }
        }
    }

    // 更新上一帧位置
    last_pos.0 = Some(current_pos);
}

// 移除原有的render_cells系统，修改handle_input和update_simulation保持不变...
fn swap_cells(grid: &mut CellGrid, x1: usize, y1: usize, x2: usize, y2: usize) {
    let idx1 = y1 * grid.width + x1;
    let idx2 = y2 * grid.width + x2;
    grid.cells.swap(idx1, idx2);
}
// 修改粒子行为逻辑（示例）

fn update_simulation(
    mut grid: ResMut<CellGrid>,
    mut images: ResMut<Assets<Image>>,

) {

    // grid.set_changed();
    for y in 0..grid.height {
        for x in 0..grid.width {
            if let Some(cell) = grid.get_mut(x, y) {
                match cell.species {
                    Species::Sand => update_sand(&mut grid, x, y),
                    Species::Water => update_water(&mut grid, x, y),
                    Species::Empty => update_sand(&mut grid, x, y),
                    // 其他粒子类型...
                    _ => {}
                }
            }
        }
    }

}

fn update_sand(grid: &mut CellGrid, x: usize, y: usize) {
    if y == 0 { return; }
    // println!("update_sand");
    let below = grid.get(x, y-1);
    if let Some(below_cell) = below {

        if below_cell.species == Species::Empty {
            swap_cells(grid, x, y, x, y-1);
        } else if below_cell.species == Species::Water {
            // 添加沙水交互
            swap_cells(grid, x, y, x, y-1);
        }
    }
}

fn update_water(grid: &mut CellGrid, x: usize, y: usize) {
    // if y == 0 { return; }

    let directions = [
        (0, -1),  // 下
        (-1, -1), // 左下
        (1, -1),  // 右下
        (-1, 0),   // 左
        (1, 0),   // 右
    ];

    for (dx, dy) in directions {
        let nx = x as isize + dx;
        let ny = y as isize + dy;

        if nx >= 0 && ny >= 0 {
            if let Some(cell) = grid.get_mut(nx as usize, ny as usize) {
                if cell.species == Species::Empty {
                    swap_cells(grid, x, y, nx as usize, ny as usize);
                    return;
                }
            }
        }
    }
}

fn update_fire(grid: &mut CellGrid, x: usize, y: usize) {
    if let Some(cell) = grid.get_mut(x, y) {
        cell.clock += 1;
        // if cell.clock > 30 { // 燃烧时间
            *cell = Cell::default();
        // }
    }
}


fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(RenderPlugin {
                    render_creation: WgpuSettings {
                        backends: Some(Backends::VULKAN),
                        ..default()
                    }
                        .into(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: (768.0, 768.0).into(),
                        title: "Cell Simulation".to_string(),
                        ..default()
                    }),
                    ..default()
                }),
            Material2dPlugin::<CellMaterial>::default(),

            ExtractResourcePlugin::<FluidConfig>::default(),
            ExtractResourcePlugin::<FluidTextures>::default(),

        ))
        .insert_resource(CellGrid::new(WIDTH as usize, HEIGHT as usize))
        .init_resource::<LastMousePos>()
        .init_resource::<FluidTextures>()
        .init_resource::<FluidConfig>()
        // .add_plugins( GameOfLifeComputePlugin)
        .add_plugins(AdvectionPlugin)   // 平流插件
        .add_plugins(CurlPlugin)       // 旋度插件
        .add_plugins(VorticityPlugin)  // 涡度应用插件
        //
        .add_plugins(DivergencePlugin) // 散度插件
        .add_plugins(PressurePlugin)   // 压力求解插件
        .add_plugins(VelocityOutPlugin)    // 速度场修正插件
        .add_plugins(GradientSubtractPlugin) // 梯度减法插件
        .add_systems(Startup, setup)
        .insert_resource(Falg(0))
        // .add_systems(Render,update_texture_data)
        .add_systems(Update, (
            // handle_input,
            // update_simulation,
            // .after(handle_input),
            update_texture_data,
            update_simulation,
            // rotate_system,
            // .after(handle_input),
            // update_shader_params,
        ))
        .add_systems(Render,(update_fluid_simulation_1,
                     update_fluid_simulation_2
        ).chain())
        .run();
}











