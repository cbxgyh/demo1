use std::borrow::Cow;
use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_resource::*;
use bevy::render::{render_graph, Render, RenderApp, RenderSet};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_graph::{NodeRunError, RenderGraph, RenderGraphContext, RenderLabel};
use bevy::render::render_resource::binding_types::{sampler, texture_2d, texture_storage_2d, uniform_buffer};
use bevy::render::renderer::{RenderContext, RenderDevice};
use crate::{HEIGHT, WIDTH, WORKGROUP_SIZE};

pub struct DivergencePlugin;

impl Plugin for DivergencePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<DivergenceImage>::default());
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
        );
        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node(DivergencComputeLabel,DivergenceComputeNode::default());
        render_graph.add_node_edge(DivergencComputeLabel, bevy::render::graph::CameraDriverLabel);

    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<DivergencePipeline>();
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable,ShaderType)]
struct DivergenceUniforms {
    texel_size: [f32; 2],
}
// 存储散度计算管线的资源
#[derive(Resource)]
struct DivergencePipeline {
    pub(crate) pipeline: CachedComputePipelineId,
    bind_group_layout: BindGroupLayout,
}


// 散度计算所需的uniform数据

impl FromWorld for DivergencePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let bind_group_layout = render_device.create_bind_group_layout(
            "divergence_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<DivergenceUniforms>(false)
                )
            ));
        let shader = world
            .resource::<AssetServer>()
            .load("divergence.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(Cow::from("divergence_pipeline")),
            layout: vec![bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("main"),
        });
        DivergencePipeline {
            pipeline,
            bind_group_layout
        }
    }
}
#[derive(Resource, Clone, ExtractResource, AsBindGroup)]
pub struct DivergenceImage{
    #[texture(0, visibility(compute))]
    pub(crate) velocity_tex: Handle<Image>,
    #[storage_texture(1, image_format = Rgba8Unorm, access = ReadWrite)]
    pub(crate) output_tex: Handle<Image>,
}

#[derive(Resource)]
pub struct DivergenceBindGroup(pub(crate) BindGroup);

fn prepare_bind_group(
    mut commands: Commands,
    gpu_images: Res<RenderAssets<Image>>,
    divergence_image: Res<DivergenceImage>,
    render_device: Res<RenderDevice>,
    divergence_pipeline: Res<DivergencePipeline>,
) {
    let velocity_tex_view = gpu_images.get(&divergence_image.velocity_tex).unwrap();
    let output_tex_view = gpu_images.get(&divergence_image.output_tex).unwrap();

    let sampler = render_device.create_sampler(&SamplerDescriptor {
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..Default::default()
    });

    let uniforms = DivergenceUniforms {
        texel_size: [1.0 / WIDTH as f32, 1.0 / HEIGHT as f32],
    };

    let uniform_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("divergence_uniform_buffer"),
        contents: bytemuck::cast_slice(&[uniforms]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let bind_group = render_device.create_bind_group(
        Some("divergence_bind_group"),
        &divergence_pipeline.bind_group_layout,
        &BindGroupEntries::sequential
            (
                (
                    &velocity_tex_view.texture_view,
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
    commands.insert_resource(DivergenceBindGroup(bind_group));
}

#[derive(Debug, Hash, PartialEq, Eq, Clone,RenderLabel)]
struct DivergencComputeLabel;


#[derive(Default)]
struct DivergenceComputeNode;

impl render_graph::Node for DivergenceComputeNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let divergence_pipeline = world.resource::<DivergencePipeline>();
        let divergence_bind_group = world.resource::<DivergenceBindGroup>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("Divergence Compute Pass"),
                ..default()
            });

        if let Some(pipeline) = pipeline_cache.get_compute_pipeline(divergence_pipeline.pipeline) {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &divergence_bind_group.0, &[]);
            pass.dispatch_workgroups(WIDTH / WORKGROUP_SIZE, HEIGHT / WORKGROUP_SIZE, 1);
        }

        Ok(())
    }
}