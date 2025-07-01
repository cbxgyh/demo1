use std::borrow::Cow;
use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::{render_graph, Render, RenderApp, RenderSet};
use bevy::render::render_graph::{NodeRunError, RenderGraph, RenderGraphContext, RenderLabel};
use bevy::render::render_resource::*;
use bevy::render::render_resource::binding_types::{sampler, texture_2d, texture_storage_2d, uniform_buffer};
use bevy::render::renderer::{RenderContext, RenderDevice};
use bytemuck::{Pod, Zeroable};
use crate::{FluidConfig, HEIGHT, WIDTH, WORKGROUP_SIZE};
use crate::advection::{AdvectionImage, AdvectionPipeline};
// ... 原有代码 ...

pub struct VorticityPlugin;

impl Plugin for VorticityPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<VorticityImage>::default());

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_systems(
                Render,
                prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
            );
        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node(VorticityLabel, VorticityComputeNode::default());
        render_graph.add_node_edge(VorticityLabel, bevy::render::graph::CameraDriverLabel);

    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<VorticityPipeline>();
    }
}
#[derive(Debug, Hash, PartialEq, Eq, Clone,RenderLabel)]
struct VorticityLabel;

// 涡度应用所需的uniform数据
#[repr(C)]
#[derive(Debug, Copy, Clone, ShaderType,Pod,Zeroable)]
struct VorticityUniforms {
    texel_size: [f32; 2],
    curl_strength: f32,
    dt: f32,
}


#[derive(Resource, Clone, ExtractResource, AsBindGroup)]
pub struct VorticityImage {
    #[texture(0, visibility(compute))]
    pub(crate) velocity_tex: Handle<Image>,
    #[texture(1, visibility(compute))]
    pub(crate) curl_tex: Handle<Image>,
    #[storage_texture(2, image_format = Rgba8Unorm, access = ReadWrite)]
    pub(crate) output_tex: Handle<Image>,
}

// 存储涡度应用管线的资源
#[derive(Resource)]
pub struct VorticityPipeline {
    pub(crate) pipeline: CachedComputePipelineId,
    bind_group_layout: BindGroupLayout,
}
#[derive(Resource)]
pub struct VorticityBindGroup (pub(crate) BindGroup);
impl FromWorld for VorticityPipeline {
    fn from_world(world: &mut World) -> Self {

        println!("VorticityPipeline_from_world");
        let render_device = world.resource::<RenderDevice>();
        // let bind_group_layout = AdvectionImage::bind_group_layout(render_device);
        let bind_group_layout = render_device.create_bind_group_layout(
            "vorticity_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<VorticityUniforms>(false)
                )
            ));


        let shader = world
            .resource::<AssetServer>()
            .load("vorticity.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();


        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("main"),
        });

        VorticityPipeline {
            pipeline,
            bind_group_layout
        }
    }
}

#[derive(Default)]
struct VorticityComputeNode;

impl render_graph::Node for VorticityComputeNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let vorticity_pipeline = world.resource::<VorticityPipeline>();
        let vorticity_bind_group = world.resource::<VorticityBindGroup>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("Vorticity Compute Pass"),
                ..default()
            });

        if let Some(pipeline) = pipeline_cache.get_compute_pipeline(vorticity_pipeline.pipeline) {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &vorticity_bind_group.0, &[]);
            pass.dispatch_workgroups(WIDTH / WORKGROUP_SIZE, HEIGHT / WORKGROUP_SIZE, 1);
        }

        Ok(())
    }
}
fn prepare_bind_group(
    mut commands: Commands,
    gpu_images: Res<RenderAssets<Image>>,
    vorticity_image: Res<VorticityImage>,
    render_device: Res<RenderDevice>,
    vorticity_pipeline: Res<VorticityPipeline>,
    time: Res<Time>,
    fluid_config: Res<FluidConfig>,
) {
    let velocity_tex_view = gpu_images.get(&vorticity_image.velocity_tex).unwrap();
    let curl_tex_view = gpu_images.get(&vorticity_image.curl_tex).unwrap();
    let output_tex_view = gpu_images.get(&vorticity_image.output_tex).unwrap();

    let sampler = render_device.create_sampler(&SamplerDescriptor {
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..Default::default()
    });

    let dt = time.delta_seconds().min(0.016);
    let curl_strength = fluid_config.curl_strength;

    let uniforms = VorticityUniforms {
        texel_size: [1.0 / WIDTH as f32, 1.0 / HEIGHT as f32],
        curl_strength,
        dt,
    };

    let uniform_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("vorticity_uniform_buffer"),
        contents: bytemuck::cast_slice(&[uniforms]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let bind_group = render_device.create_bind_group(
        "vorticity_bind_group",
        &vorticity_pipeline.bind_group_layout,
        &BindGroupEntries::sequential
            (
                (
                    &velocity_tex_view.texture_view,
                    &curl_tex_view.texture_view,
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
    commands.insert_resource(VorticityBindGroup(bind_group));

}


