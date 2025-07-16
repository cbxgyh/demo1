use std::borrow::Cow;
use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::{render_graph, Render, RenderApp, RenderSet};
use bevy::render::render_graph::{NodeRunError, RenderGraph, RenderGraphContext, RenderLabel};
use bevy::render::render_resource::*;
use bevy::render::render_resource::binding_types::{sampler, texture_2d, texture_storage_2d, uniform_buffer};
use bevy::render::renderer::{RenderContext, RenderDevice};
use crate::{FluidConfig, FluidTextures, HEIGHT, WIDTH, WORKGROUP_SIZE};
use crate::universe::CellGrid;
// ... 原有代码 ...

// 梯度减法所需的uniform数据
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable,ShaderType)]
struct GradientSubtractUniforms {
    texel_size: [f32; 2],
    wind_strength: f32,
    damping: f32,
}
#[derive(Resource)]
// 存储梯度减法管线的资源
pub struct GradientSubtractPipeline {
    pub(crate) pipeline: CachedComputePipelineId,
    bind_group_layout: BindGroupLayout,
}

impl FromWorld for GradientSubtractPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let bind_group_layout = render_device.create_bind_group_layout(
            "gradient_subtract_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<GradientSubtractUniforms>(false)
                )
            ));


        let shader = world
            .resource::<AssetServer>()
            .load("gradient_subtract.wgsl");

        let pipeline_cache = world.resource::<PipelineCache>();

        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(Cow::from("gradient_subtract_pipeline")),
            layout: vec![bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("gradient_subtract_main"),
        });

        GradientSubtractPipeline {
            pipeline,
            bind_group_layout,
        }
    }
}

#[derive(Resource, Clone, ExtractResource, AsBindGroup)]
pub struct GradientSubtractImage {
    #[texture(0, visibility(compute))]
    pub(crate) pressure_tex: Handle<Image>,
    #[texture(1, visibility(compute))]
    pub(crate) velocity_tex: Handle<Image>,
    #[texture(2, visibility(compute))]
    pub(crate) wind_tex: Handle<Image>,
    #[texture(3, visibility(compute))]
    pub(crate) cells_tex: Handle<Image>,
    #[storage_texture(4, image_format = Rgba8Unorm, access = ReadWrite)]
    pub(crate) output_tex: Handle<Image>,
}

#[derive(Resource)]
pub struct GradientSubtractBindGroup(pub(crate) BindGroup);

fn prepare_bind_group(
    mut commands: Commands,
    gpu_images: Res<RenderAssets<Image>>,
    gradient_subtract_image: Res<GradientSubtractImage>,
    render_device: Res<RenderDevice>,
    gradient_subtract_pipeline: Res<GradientSubtractPipeline>,
    fluid_config: Res<FluidConfig>,
) {
    let pressure_tex_view = gpu_images.get(&gradient_subtract_image.pressure_tex).unwrap();
    let velocity_tex_view = gpu_images.get(&gradient_subtract_image.velocity_tex).unwrap();
    let wind_tex_view = gpu_images.get(&gradient_subtract_image.wind_tex).unwrap();
    let cells_tex_view = gpu_images.get(&gradient_subtract_image.cells_tex).unwrap();
    let output_tex_view = gpu_images.get(&gradient_subtract_image.output_tex).unwrap();

    let sampler = render_device.create_sampler(&SamplerDescriptor {
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..Default::default()
    });

    let uniforms = GradientSubtractUniforms {
        texel_size: [1.0 / WIDTH as f32, 1.0 / HEIGHT as f32],
        wind_strength: -25.0,  // 风力强度
        damping: 0.95,         // 阻尼系数
    };

    let uniform_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("gradient_subtract_uniform_buffer"),
        contents: bytemuck::cast_slice(&[uniforms]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let bind_group = render_device.create_bind_group(
        "gradient_subtract_bind_group",
        &gradient_subtract_pipeline.bind_group_layout,
        &BindGroupEntries::sequential
            (
                (
                    &pressure_tex_view.texture_view,
                    &velocity_tex_view.texture_view,
                    &wind_tex_view.texture_view,
                    &cells_tex_view.texture_view,
                    &output_tex_view.texture_view,
                    &sampler,
                    BindingResource::Buffer(BufferBinding {
                        buffer: &uniform_buffer,
                        offset: 0,
                        size: None,
                    })
                )
            )
    );

    commands.insert_resource(GradientSubtractBindGroup(bind_group));
}

#[derive(Default)]
pub(crate) struct GradientSubtractComputeNode;

impl render_graph::Node for GradientSubtractComputeNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let gradient_subtract_pipeline = world.resource::<GradientSubtractPipeline>();
        let gradient_subtract_bind_group = world.resource::<GradientSubtractBindGroup>();
        // println!("Gradient Subtract Compute Pass");
        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("Gradient Subtract Compute Pass"),
                ..default()
            });

        if let Some(pipeline) = pipeline_cache.get_compute_pipeline(gradient_subtract_pipeline.pipeline) {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &gradient_subtract_bind_group.0, &[]);
            pass.dispatch_workgroups(WIDTH / WORKGROUP_SIZE, HEIGHT / WORKGROUP_SIZE, 1);
        }

        Ok(())
    }
}

pub struct GradientSubtractPlugin;

impl Plugin for GradientSubtractPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<GradientSubtractImage>::default())
            .add_systems(Update,swap_velocity_buffer)
        ;

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_systems(
                Render,
                prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
            );



    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<GradientSubtractPipeline>();
    }
}
fn swap_velocity_buffer(mut fluid_textures: ResMut<FluidTextures>) {
    let velocity = &mut fluid_textures.velocity;
    std::mem::swap(&mut velocity.0, &mut velocity.1);

}
#[derive(Debug, Hash, PartialEq, Eq, Clone,RenderLabel)]
pub(crate) struct GradientLabel;
//
// fn update_image(
//     gradient_subtract_image: ResMut<GradientSubtractImage>,
//     cell_grid: Res<CellGrid>,
//     mut images: ResMut<Assets<Image>>,
// ){
//     println!("Gradient Subtract Image");
//     if let Some(image) = images.get_mut(&gradient_subtract_image.cells_tex) {
//         let pixels = image.data.as_mut_slice();
//         println!("Gradient Subtract Image111");
//         for (i, cell) in cell_grid.cells.iter().enumerate() {
//             let idx = i * 4;
//             pixels[idx] = cell.species as u8; // R: 细胞类型
//             pixels[idx + 1] = cell.ra;        // G: 附加数据1
//             pixels[idx + 2] = cell.rb;        // B: 附加数据2
//             pixels[idx + 3] = cell.clock;     // A: 生命周期时钟
//         }
//     }
//     if let Some(image) = images.get_mut(&gradient_subtract_image.wind_tex) {
//         println!("Gradient Subtract Image222");
//         let pixels =   image.data.as_mut_slice();
//         for (i, cell) in cell_grid.winds.iter().enumerate() {
//             let idx = i * 4;
//             pixels[idx] = cell.dx;
//             pixels[idx+1] = cell.dy ;
//             pixels[idx+2] = cell.pressure;
//             pixels[idx+3] = cell.density;
//         }
//
//     }
// }