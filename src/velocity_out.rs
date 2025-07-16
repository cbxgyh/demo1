use std::borrow::Cow;
use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::{render_graph, Render, RenderApp, RenderSet};
use bevy::render::render_graph::{NodeRunError, RenderGraph, RenderGraphContext, RenderLabel};
use bevy::render::render_resource::*;
use bevy::render::render_resource::binding_types::{sampler, texture_2d, texture_storage_2d, uniform_buffer};
use bevy::render::renderer::{RenderContext, RenderDevice};
use crate::{FluidConfig, HEIGHT, WIDTH, WORKGROUP_SIZE};

// 速度场修正所需的uniform数据
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable,ShaderType)]
struct VelocityOutUniforms {
    min_value: f32,
    max_value: f32,
    scale_factor: f32,
    _padding: f32,
    offset: [f32; 2],
}

// 存储速度场修正管线的资源
#[derive(Resource)]
pub struct VelocityOutPipeline {
    pub(crate) pipeline: CachedComputePipelineId,
    bind_group_layout: BindGroupLayout,
}


impl FromWorld for VelocityOutPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let bind_group_layout = render_device.create_bind_group_layout(
            "velocity_out_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<VelocityOutUniforms>(false)
                )
            )
        );

        let shader = world
            .resource::<AssetServer>()
            .load("velocity_Out.wgsl");

        let pipeline_cache = world.resource::<PipelineCache>();

        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(Cow::from("velocity_out_pipeline")),
            layout: vec![bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("velocity_out_main"),
        });

        VelocityOutPipeline {
            pipeline,
            bind_group_layout,
        }
    }
}

#[derive(Resource, Clone, ExtractResource, AsBindGroup)]
pub struct VelocityOutImage {
    #[texture(0, visibility(compute))]
    pub(crate) velocity_tex: Handle<Image>,
    #[texture(1, visibility(compute))]
    pub(crate) pressure_tex: Handle<Image>,
    #[storage_texture(2, image_format = Rgba8Unorm, access = ReadWrite)]
    pub(crate) output_tex: Handle<Image>,
}

#[derive(Resource)]
pub struct VelocityOutBindGroup(pub(crate) BindGroup);

fn prepare_bind_group(
    mut commands: Commands,
    gpu_images: Res<RenderAssets<Image>>,
    velocity_Out_image: Res<VelocityOutImage>,
    render_device: Res<RenderDevice>,
    velocity_Out_pipeline: Res<VelocityOutPipeline>,
    fluid_config: Res<FluidConfig>,
) {
    let velocity_tex_view = gpu_images.get(&velocity_Out_image.velocity_tex).unwrap();
    let pressure_tex_view = gpu_images.get(&velocity_Out_image.pressure_tex).unwrap();
    let output_tex_view = gpu_images.get(&velocity_Out_image.output_tex).unwrap();

    let sampler = render_device.create_sampler(&SamplerDescriptor {
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..Default::default()
    });

    let uniforms = VelocityOutUniforms {
        min_value: -250.0,    // 速度最小值
        max_value: 250.0,     // 速度最大值
        scale_factor: 500.0,  // 缩放因子
        _padding: 0.,
        offset: [0.5, 0.5],   // 偏移量
    };

    let uniform_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("velocity_Out_uniform_buffer"),
        contents: bytemuck::cast_slice(&[uniforms]),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let bind_group = render_device.create_bind_group(
        "velocity_Out_bind_group",
        &velocity_Out_pipeline.bind_group_layout,
        &BindGroupEntries::sequential
            (
                (
                    &velocity_tex_view.texture_view,
                    &pressure_tex_view.texture_view,
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

    commands.insert_resource(VelocityOutBindGroup(bind_group));
}

#[derive(Default)]
pub(crate) struct VelocityOutComputeNode;

impl render_graph::Node for VelocityOutComputeNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let velocity_Out_pipeline = world.resource::<VelocityOutPipeline>();
        let velocity_Out_bind_group = world.resource::<VelocityOutBindGroup>();
        // println!("Velocity Out  Compute Pass");
        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("Velocity Out Compute Pass"),
                ..default()
            });

        if let Some(pipeline) = pipeline_cache.get_compute_pipeline(velocity_Out_pipeline.pipeline) {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &velocity_Out_bind_group.0, &[]);
            pass.dispatch_workgroups(WIDTH / WORKGROUP_SIZE, HEIGHT / WORKGROUP_SIZE, 1);
        }

        Ok(())
    }
}

pub struct VelocityOutPlugin;
impl Plugin for VelocityOutPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<VelocityOutImage>::default());

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_systems(
                Render,
                prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
            )
            ;

    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<VelocityOutPipeline>();
    }
}
#[derive(Debug, Hash, PartialEq, Eq, Clone,RenderLabel)]
pub(crate) struct VorticityOutLabel;
