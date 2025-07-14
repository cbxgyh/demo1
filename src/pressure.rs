use std::borrow::Cow;
use bevy::core_pipeline::core_2d::graph::{Core2d, Node2d};
use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::{render_graph, Render, RenderApp, RenderSet};
use bevy::render::render_graph::{NodeRunError, RenderGraph, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNodeRunner};
use bevy::render::render_resource::*;
use bevy::render::render_resource::binding_types::{sampler, texture_2d, texture_storage_2d, uniform_buffer};
use bevy::render::renderer::{RenderContext, RenderDevice};
use crate::{FluidConfig, HEIGHT, WIDTH, WORKGROUP_SIZE};
// ... 原有代码 ...
pub struct PressurePlugin;

impl Plugin for PressurePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<PressureImage>::default());

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_systems(
                Render,
                prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
            );


    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<PressurePipeline>();
    }
}


#[derive(Debug, Hash, PartialEq, Eq, Clone,RenderLabel)]
pub(crate) struct PressureComputeLabel;
// 压力求解所需的uniform数据
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable,ShaderType)]
struct PressureUniforms {
    texel_size: [f32; 2],
    alpha: f32,
    reciprocal_beta: f32,
}

// 存储压力求解管线的资源
#[derive(Resource)]
pub struct PressurePipeline {
    pub(crate) pipeline: CachedComputePipelineId,
    bind_group_layout: BindGroupLayout,
}

#[derive(Resource, Clone, ExtractResource, AsBindGroup)]
pub struct PressureImage {
    #[texture(0, visibility(compute))]
    pub(crate) pressure_tex: Handle<Image>,
    #[texture(1, visibility(compute))]
    pub(crate) divergence_tex: Handle<Image>,
    #[storage_texture(2, image_format = Rgba8Unorm, access = ReadWrite)]
    pub(crate) output_tex: Handle<Image>,
}

#[derive(Resource)]
pub struct PressureBindGroup(pub(crate) BindGroup);

fn prepare_bind_group(
    mut commands: Commands,
    gpu_images: Res<RenderAssets<Image>>,
    pressure_image: Res<PressureImage>,
    render_device: Res<RenderDevice>,
    pressure_pipeline: Res<PressurePipeline>,
    fluid_config: Res<FluidConfig>,
) {
    let pressure_tex_view = gpu_images.get(&pressure_image.pressure_tex).unwrap();
    let divergence_tex_view = gpu_images.get(&pressure_image.divergence_tex).unwrap();
    let output_tex_view = gpu_images.get(&pressure_image.output_tex).unwrap();

    let sampler = render_device.create_sampler(&SamplerDescriptor {
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..Default::default()
    });

    let uniforms = PressureUniforms {
        texel_size: [1.0 / WIDTH as f32, 1.0 / HEIGHT as f32],
        alpha: 1.0,  // 标准Gauss-Seidel迭代
        reciprocal_beta: 0.25,  // 对应2D网格的系数
    };

    let uniform_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("pressure_uniform_buffer"),
        contents: bytemuck::cast_slice(&[uniforms]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let bind_group = render_device.create_bind_group(
        "pressure_bind_group",
        &pressure_pipeline.bind_group_layout,
        &BindGroupEntries::sequential
            (
                (
                    &pressure_tex_view.texture_view,
                    &divergence_tex_view.texture_view,
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
    commands.insert_resource(PressureBindGroup(bind_group));
}



impl FromWorld for PressurePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let bind_group_layout = render_device.create_bind_group_layout(
            "curl_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<PressureUniforms>(false)
                )
            ));
        let shader = world
            .resource::<AssetServer>()
            .load("pressure.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("main"),
        });

        PressurePipeline {
            pipeline,
            bind_group_layout,
        }
    }
}

#[derive(Default)]
pub(crate) struct PressureComputeNode;

impl render_graph::Node for PressureComputeNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let pressure_pipeline = world.resource::<PressurePipeline>();
        let pressure_bind_group = world.resource::<PressureBindGroup>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("Pressure Compute Pass"),
                ..default()
            });

        if let Some(pipeline) = pipeline_cache.get_compute_pipeline(pressure_pipeline.pipeline) {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &pressure_bind_group.0, &[]);
            pass.dispatch_workgroups(WIDTH / WORKGROUP_SIZE, HEIGHT / WORKGROUP_SIZE, 1);
        }

        Ok(())
    }
}
