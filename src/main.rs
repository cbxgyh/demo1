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
use bevy::render::render_resource::{CompareFunction, DepthStencilState, Extent3d, ImageDataLayout, RenderPipelineDescriptor, SpecializedMeshPipelineError, TextureDimension, TextureFormat, TextureUsages};
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::{Render, RenderPlugin};
use bevy::render::settings::{Backends, WgpuSettings};
use bevy::render::texture::TextureFormatPixelInfo;
use bevy::sprite::{Material2dKey, MaterialMesh2dBundle};
use bevy::utils::petgraph::visit::NodeRef;
use bevy::window::PrimaryWindow;

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
                    species: Species::Gas,
                    ra: 0,
                    rb: 0,
                    clock: 0,
                });
            }
        };
        // cells.resize_with(width * height, || {
        //
        //     Cell {
        //
        //         species: Species::Water,
        //         ra: 0,
        //         rb: 0,
        //         clock: 0,
        //     }
        // });

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


    // fn specialize(
    //     descriptor: &mut RenderPipelineDescriptor,
    //     layout: &MeshVertexBufferLayout,
    //     key: Material2dKey<Self>,
    // ) -> Result<(), SpecializedMeshPipelineError> {
    //     descriptor.vertex.entry_point = "vs_main".into();
    //     if let Some(fragment) = &mut descriptor.fragment {
    //         fragment.entry_point = "fragment".into();
    //     }
    //     // descriptor.depth_stencil = Some(DepthStencilState {
    //     //     format: TextureFormat::Depth32Float,
    //     //     depth_write_enabled: false,
    //     //     depth_compare: CompareFunction::Always,
    //     //     stencil: Default::default(),
    //     //     bias: Default::default(),
    //     // });
    //     Ok(())
    // }

    fn fragment_shader() -> ShaderRef {
        "sand.wgsl".into()
    }
    fn vertex_shader() -> ShaderRef {
        "sand.wgsl".into()
    }
}

// 主应用
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
        ))
        .insert_resource(CellGrid::new(768, 768))
        .add_systems(Startup, setup)
        .insert_resource(Falg(0))
        // .add_systems(Render,update_texture_data)
        .add_systems(Update, (
            handle_input,
            // update_simulation.after(handle_input),
            update_texture_data,
            rotate_system,
                // .after(handle_input),
            // update_shader_params,
        ))
        .run();
}
#[derive(Resource)]
struct Falg(usize);


// 新的渲染系统
#[derive(Component)]
struct CellCanvas;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CellMaterial>>,
    mut images: ResMut<Assets<Image>>,
    // primary_window: Query<PrimaryWindow>,
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
    // 创建数据纹理
    let mut image = Image::new_fill(
        Extent3d { width: 768, height: 768, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0u8; 4],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();

    // image.texture_descriptor.usage =
    //      TextureUsages::COPY_SRC |
    //         TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING |
    // TextureUsages::COPY_DST;
    image.texture_descriptor.usage=TextureUsages::TEXTURE_BINDING
        | TextureUsages::COPY_DST;
        // | TextureUsages::RENDER_ATTACHMENT;
    let data_tex_handle = images.add(image); // 强引用在此处创建

    // 创建材质
    let material = materials.add(CellMaterial {
        data_tex: data_tex_handle.clone(),
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

                    pixels[idx] = cell.species as u8;
                    pixels[idx + 1] = cell.ra;
                    pixels[idx + 2] = cell.rb;
                    pixels[idx + 3] = cell.clock;
                    if cell.species == Species::Fire {
                        // let center_x = grid.width / 2;
                        // let center_y = grid.height / 2;
                        // let idx = (center_y * grid.width + center_x) * 4;
                        // println!("Center pixel: {:?}:cell: {:?}", &pixels[idx], cell);
                    }
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

fn handle_input(
    mut grid: ResMut<CellGrid>,
    windows: Query<&Window>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut images: ResMut<Assets<Image>>,
    mut falg: ResMut<Falg>
) {
    let window = windows.single();
    if let Some(pos) = window.cursor_position() {
        let x = (pos.x / window.width() * grid.width as f32) as usize;
        let x = x.min(grid.width - 1);

        let y = grid.height - 1 - (pos.y / window.height() * grid.height as f32) as usize;
        let y = y.min(grid.height - 1);

        if buttons.pressed(MouseButton::Left) {
            if let Some(cell) = grid.get_mut(x, y) {
                *cell = Cell {
                    species: Species::Fire,
                    ra: 0,
                    rb: 0,
                    clock: 0,
                };
                // println!("Set Fire at ({}, {})", x, y); // 调试输出
            }
        }


    }
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
                    Species::Fire => update_fire(&mut grid, x, y),
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
    if y == 0 { return; }

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
        if cell.clock > 30 { // 燃烧时间
            *cell = Cell::default();
        }
    }
}














