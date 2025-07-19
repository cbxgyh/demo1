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
mod universe;
mod fluidsimulation;
mod display2;
mod clear;

use std::collections::VecDeque;
use std::mem::swap;
use std::process::id;
use std::time::{Duration, Instant};
use crossbeam_channel::{bounded, Receiver};
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
use rand::{thread_rng, Rng};
use rand::seq::SliceRandom;
use crate::advection::{ AdvectionPipeline, AdvectionPlugin, DensityAdvectionImage, VelocityAdvectionImage};
use crate::clear::ClearImage;
use crate::compute_shader_game_of_life::{GameOfLifeComputePlugin, GameOfLifeImage};
use crate::curl::{CurlBindGroup, CurlImage, CurlPipeline, CurlPlugin};
use crate::display::DisplayTarget;
// use crate::display1::DisplayPlugin;

use crate::divergence::{DivergenceImage, DivergencePlugin};
use crate::fluidsimulation::FluidSimulationPlugin;
use crate::gradient_subtract::{GradientSubtractBindGroup, GradientSubtractImage, GradientSubtractPipeline, GradientSubtractPlugin};
use crate::pressure::{PressureBindGroup, PressureImage, PressurePipeline, PressurePlugin};
use crate::universe::{CellGrid, Species};
use crate::velocity_out::{VelocityOutBindGroup, VelocityOutImage, VelocityOutPipeline, VelocityOutPlugin};
use crate::vorticity::{VorticityBindGroup, VorticityImage, VorticityPipeline, VorticityPlugin};

pub const WIDTH: u32 = 600;
pub const HEIGHT: u32 = 600;
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

// 核心数据结构


// 风（Wind）和细胞（Cell）的数据结构以及 Universe（宇宙）的一部分实现
// // Wind 结构体表示风的特性，其中：
// //
// // dx 和 dy 分别表示风的水平和垂直方向的分量。这些数值通常会影响模拟中的细胞移动，或用于计算与其他细胞的相互作用。
// // pressure 和 density 可能用于表示风的强度和“浓度”，例如它影响哪些物种会被风吹动，或风是否可以推动某些细胞（例如沙子、火等）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wind {
    dx: u8,
    dy: u8,
    pressure: u8,
    density: u8,
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

#[derive(Resource,Deref)]
struct SeedPositionReceiver(Receiver<SeedPosition>);


#[derive(Debug, Clone, Copy)]
struct SeedPosition {
    x: i32,
    y: i32,
    size:f64,
    s:Species
}
// 新的渲染系统
#[derive(Component)]
struct CellCanvas;


fn rotate_system(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Rotating)>,
) {
    for (mut transform, rotating) in query.iter_mut() {
        let angle = (time.elapsed_seconds() * rotating.speed).sin() * 30.0; // -30度到+30度之间摆动
        transform.rotation = Quat::from_rotation_z(angle.to_radians());
    }
}



// 更新纹理数据
fn update_texture_data(
    // cell_grid: Res<CellGrid>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<CellMaterial>>,
    material_query: Query<&Handle<CellMaterial>>,
    mut cell_grid: ResMut<CellGrid>,

) {
    cell_grid.tick();
    // 在update_texture_data中添加对齐检查
    for material_handle in material_query.iter() {
        if let Some(material) = materials.get_mut(material_handle) {
            if let Some(image) = images.get_mut(&material.data_tex) {
    //             let format = image.texture_descriptor.format;
    //             let bytes_per_row = image.width() * format.pixel_size() as u32;
                let pixels = image.data.as_mut_slice();
                for (i, cell) in cell_grid.cells.iter().enumerate() {
                    let idx = i * 4;
                    //             //
                    if cell.species==Species::Seed {
                        let (x,y)=cell_grid.get_x_y(i as i32);

                    }
                    pixels[idx] = cell.species as u8;
                    //             //     // let s=Species::random_active() as u8;
                    //             //     let s=cell.species as u8;
                    //             //     pixels[idx] = s;
                    //             //     // pixels[idx + i*4] = s;
                    //             //     // pixels[idx +  i*8] = s;
                    //             //     // pixels[idx +  i*12] = s;
                    //             //     // pixels[idx + 1] = cell.ra;
                    //             //     // pixels[idx + 2] = cell.rb;
                    //             //     // pixels[idx + 3] = cell.clock;
                    //             //     // if cell.species == Species::Fire {
                    //             //         // let center_x = grid.width / 2;
                    //             //         // let center_y = grid.height / 2;
                    //             //         // let idx = (center_y * grid.width + center_x) * 4;
                    //             //         // println!("Center pixel: {:?}:cell: {:?}", &pixels[idx], cell);
                    //             //     // }
                    //             // }
                    //             // let data_layout = ImageDataLayout {
                    //             //     offset: 0,
                    //             //     bytes_per_row: Some(bytes_per_row),
                    //             //     rows_per_image: None,
                    //             // };
                    //             // let data_texture = render_device.create_texture(&image.texture_descriptor);
                    //             //
                    //             // render_queue.write_texture(
                    //             //     data_texture.as_image_copy(),
                    //             //     &image.data,
                    //             //     data_layout,
                    //             //     image.texture_descriptor.size,
                    //             // );
                }
            }
        }
    }
}
#[derive(Resource, Default)]
struct LastMousePos(Option<Vec2>);

// fn handle_input(
//     mut grid: ResMut<CellGrid>,
//     windows: Query<&Window>,
//     buttons: Res<ButtonInput<MouseButton>>,
//     mut last_pos: ResMut<LastMousePos>,
// )
// {
//     let window = windows.single();
//
//     // 获取当前鼠标位置
//     let current_pos = if let Some(pos) = window.cursor_position() {
//         pos
//     } else {
//         last_pos.0 = None;
//         return;
//     };
//
//     // 转换坐标为网格坐标
//
//
//     if buttons.pressed(MouseButton::Left) {
//         let to_grid_pos = |pos: Vec2| -> (usize, usize) {
//             let x = (pos.x / window.width() * grid.width as f32) as usize;
//             let y = grid.height - 1 - (pos.y / window.height() * grid.height as f32) as usize;
//             (x.clamp(0, grid.width-1), y.clamp(0, grid.height-1))
//         };
//         // 如果有上一帧位置，进行插值
//         if let Some(last) = last_pos.0 {
//             // 计算两点间距离
//             let current_grid = to_grid_pos(current_pos);
//             let last_grid = to_grid_pos(last);
//
//             // 使用 Bresenham 算法绘制连续线段
//             for (x, y) in line_drawing::Bresenham::new(
//                 (last_grid.0 as i32, last_grid.1 as i32),
//                 (current_grid.0 as i32, current_grid.1 as i32)
//             ) {
//                 let x = x.clamp(0, grid.width as i32 - 1) as usize;
//                 let y = y.clamp(0, grid.height as i32 - 1) as usize;
//
//                 if let Some(cell) = grid.get_mut(x, y) {
//                     *cell = Cell {
//                         species: Species::Water,
//                         ra: 0,
//                         rb: 0,
//                         clock: 0,
//                     };
//                 }
//             }
//         }else{
//             // 绘制当前点
//             let (x, y) = to_grid_pos(current_pos);
//             if let Some(cell) = grid.get_mut(x, y) {
//                 *cell = Cell {
//                     species: Species::Water,
//                     ra: 0,
//                     rb: 0,
//                     clock: 0,
//                 };
//             }
//         }
//     }
//
//     // 更新上一帧位置
//     last_pos.0 = Some(current_pos);
// }

// 移除原有的render_cells系统，修改handle_input和update_simulation保持不变...
fn swap_cells(grid: &mut CellGrid, x1: usize, y1: usize, x2: usize, y2: usize) {
    let idx1 = y1  * grid.width as usize + x1;
    let idx2 = y2  * grid.width as usize+ x2;
    grid.cells.swap(idx1 , idx2);
}
// 修改粒



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
                        resolution: (WIDTH as f32, HEIGHT as f32).into(),
                        title: "Cell Simulation".to_string(),
                        ..default()
                    }),
                    ..default()
                }),
            Material2dPlugin::<CellMaterial>::default(),

            ExtractResourcePlugin::<FluidConfig>::default(),
            ExtractResourcePlugin::<FluidTextures>::default(),

        ))
        .insert_resource(CellGrid::new(WIDTH as i32, HEIGHT as i32))
        .init_resource::<LastMousePos>()
        .init_resource::<FluidTextures>()
        .init_resource::<FluidConfig>()
        // .add_plugins( GameOfLifeComputePlugin)
        .add_plugins( FluidSimulationPlugin)

        .add_systems(Startup, setup)
        .insert_resource(Falg(0))
        // .add_systems(Render,update_texture_data)
        .add_systems(Update, (
            // handle_input,
            // update_simulation,
            // .after(handle_input),
            // debug_cameras,
            apply_seed_positions,
            update_texture_data.after(apply_seed_positions),
            // update_image.after(update_texture_data),
            // update_simulation,
            // rotate_system,
            // .after(handle_input),
            // update_shader_params,
        ))
        // .add_systems(Render,(update_fluid_simulation_1,
        //              update_fluid_simulation_2
        // ).chain())
        .run();
}

fn apply_seed_positions(
    mut receiver: Res<SeedPositionReceiver>,
    mut cell_grid: ResMut<CellGrid>,
) {
    for position in receiver.try_iter() {
        cell_grid.paint(position.x, position.y, position.size as i32, position.s);
    }
    // 处理所有可用的位置
    // while let Ok(position) = receiver.0.try_recv() {
    //     println!("rrrrrrrrrrr");
    //     cell_grid.paint(position.x, position.y, position.size as i32, position.s);
    // }
    // cell_grid.winds()
}

fn  setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CellMaterial>>,
    mut images: ResMut<Assets<Image>>,
    // primary_window: Query<PrimaryWindow>,
    mut fluid_textures: ResMut<FluidTextures>,
    mut fluid_config: ResMut<FluidConfig>,
    mut cell_grid: ResMut<CellGrid>,
)
{
    let (tx, rx) =  bounded::<SeedPosition>(5);
    // // 获取异步计算线程池
    // let task_pool = AsyncComputeTaskPool::get();
    let tx2=tx.clone();
    cell_grid.paint(300, 50, 60, Species::Water);
    cell_grid.paint(550, 50, 60, Species::Fire);
    // 提交异步任务 - 只计算位置，不修改资源
    // let task =  task_pool.spawn(async move {
    std::thread::spawn(move || {
        // 提交异步任务
        let width = WIDTH as i32;
        let height = HEIGHT as i32;
        let mut rng = rand::thread_rng();
        for x in (5..width - 5).step_by(10) {
            let y = height - 40 + (5.0 * (x as f64 / 20.0).sin()).floor() as i32;
            let size = rng.gen_range(10.0..16.0);
            // cell_grid.paint(x, y, size as i32, Species::Sand);
            let start_time = Instant::now();
            let duration = Duration::from_secs_f32(rng.gen_range(0.0..0.1));
            while start_time.elapsed() < duration {
                // Spinning for 'duration', simulating doing hard work!
            }
            if tx.send(SeedPosition { x, y, size, s: Species::Sand }).is_err() {
                break; // 如果接收端关闭则退出
            }
            // println!("sand:{:?},{:?}",x,y);
        }
    });
   std::thread::spawn(move || {
       let width = WIDTH as i32;
       let height = HEIGHT as i32;
       let mut rng = rand::thread_rng();
        let mut x: i32 = 40;  // 起始位置

        while x <= width - 40 {
            // 计算波动效果的位置
            let y = (height as f64 / 2.0 + 20.0 * (x as f64 / 20.0).sin()).floor() as i32;
            // println!("seed:{:?},{:?}",x,y);
            // 绘制种子
            // cell_grid.paint(
            //     x,
            //     y,
            //     6,
            //     Species::Seed
            // );
            let start_time = Instant::now();
            let duration = Duration::from_secs_f32(rng.gen_range(0.0..0.1));
            while start_time.elapsed() < duration {
                // Spinning for 'duration', simulating doing hard work!
            }
            if tx2.send(SeedPosition { x, y, size:6.0,s:Species::Seed}).is_err() {
                break; // 如果接收端关闭则退出
            }
            // 生成随机步长 (50-60)
            let step = 50.0 + rng.gen::<f64>() * 10.0;
            x += step as i32;
        }
    });
    // *seed_position_receiver = SeedPositionReceiver(Arc::new(Mutex::new(Some(rx))));
    commands.insert_resource(SeedPositionReceiver(rx));
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
    // 初始化流体配置
    fluid_config.velocity_dissipation = 0.99;
    fluid_config.density_dissipation = 0.99;
    fluid_config.curl_strength = 3.0;
    fluid_config.pressure_dissipation = 0.99;
    fluid_config.pressure_iterations = 20;

    let (mut x,mut y) =(create_texture(&mut images),create_texture(&mut images));
    {
        println!("x_y_{:?},{:?}", (&x).id(),(&y).id());
    }
    {
        swap(&mut x,&mut y);
    }
    {
        println!("x_y_{:?},{:?}", x.id(), y.id());
    }
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
    fluid_textures.density = (create_texture(&mut images), create_storage_texture(&mut images));
    fluid_textures.pressure = (create_texture(&mut images), create_storage_texture(&mut images));
    fluid_textures.curl = create_storage_texture(&mut images);
    fluid_textures.divergence =  create_storage_texture(&mut images);
    fluid_textures.burns = create_texture(&mut images);
    fluid_textures.cells = create_storage_texture(&mut images);
    fluid_textures.velocity_out = create_storage_texture(&mut images);
    // let data_tex_handle = images.add(image); // 强引用在此处创建
    let cc=create_texture(&mut images);
    // 创建材质
    let material = materials.add(CellMaterial {
        data_tex: fluid_textures.cells.clone(),
        // data_tex: cc.clone(),
        // data_tex: data_tex_handle.clone(),
        params: ShaderParams {
            time: 0.0,
            dpi: 1.0,
            resolution: Vec2::new(WIDTH as f32, WIDTH as f32),
            is_snapshot: 0,
        },
    });

    commands.spawn((
        MaterialMesh2dBundle {
            mesh: quad.into(),
            material: material.clone(),
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, 0.0),
                scale: Vec3::new(WIDTH as f32, HEIGHT as f32 ,1.0),
                ..default()
            },

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
        transform: Transform::from_xyz(0.0, 0.0, 100.0),
        ..default()
    });



    commands.insert_resource(GameOfLifeImage { texture:  fluid_textures.cells.clone() });
    // 初始化AdvectionImage资源
    commands.insert_resource(VelocityAdvectionImage {
        velocity_tex: fluid_textures.velocity.0.clone(),
        source_tex: fluid_textures.density.0.clone(),
        // 注意：这里使用cells作为风场
        wind_tex: fluid_textures.cells.clone(),
        // 输出到速度的写纹理
        output_tex: fluid_textures.velocity.1.clone(),
    });
    commands.insert_resource(DensityAdvectionImage {
        // 使用burns作为风场
        wind_tex: fluid_textures.burns.clone(),
        velocity_tex: fluid_textures.velocity.0.clone(),
        // 使用密度作为源
        source_tex: fluid_textures.density.0.clone(),
        // 输出到密度的写纹理
        output_tex: fluid_textures.density.1.clone(),
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
    commands.insert_resource(ClearImage {
        u_texture_tex: fluid_textures.burns.clone(),
        u_wind_tex: fluid_textures.pressure.0.clone(),
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
    // commands.insert_resource(DisplayImage {
    //     density_tex: fluid_textures.density.0.clone(),
    //     output_tex: create_storage_texture(&mut images),
    // });
    commands.insert_resource(DisplayTarget {
            image: fluid_textures.cells.clone()
        });
}





