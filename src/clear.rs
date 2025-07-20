use std::borrow::Cow;
use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::*;
use bevy::render::render_resource::binding_types::{sampler, texture_2d, texture_storage_2d, uniform_buffer};
use bevy::render::{render_graph, Render, RenderApp, RenderSet};
use bevy::render::render_graph::{NodeRunError, RenderGraph, RenderGraphContext, RenderLabel};
use bevy::render::renderer::{RenderContext, RenderDevice};
use crate::{FluidConfig, HEIGHT, WIDTH, WORKGROUP_SIZE};


pub struct ClearPlugin;
impl Plugin for ClearPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<ClearImage>::default());
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
        )
        ;

    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<ClearPipeline>();
    }
}
// Clear计算所需的uniform数据
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable,ShaderType)]
pub struct ClearUniforms {
    value: f32,
}

// 存储Clear计算管线的资源
#[derive(Resource)]
pub struct ClearPipeline {
    pub(crate) pipeline: CachedComputePipelineId,
    bind_group_layout: BindGroupLayout,
}

impl FromWorld for ClearPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        // let bind_group_layout = AdvectionImage::bind_group_layout(render_device);
        let bind_group_layout = render_device.create_bind_group_layout(
            "clear_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
                    sampler(SamplerBindingType::Filtering),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<ClearUniforms>(false)
                )
            ));
        let shader = world
            .resource::<AssetServer>()
            .load("clear.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(Cow::from("clear")),
            layout: vec![bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("clear_main"),
        });
        ClearPipeline {
            pipeline,
            bind_group_layout
        }
    }
}

#[derive(Resource, Clone, ExtractResource, AsBindGroup)]
pub struct ClearImage {
    #[texture(0, visibility(compute))]
    #[sampler(3)]
    pub(crate) u_texture_tex: Handle<Image>,
    #[texture(1, visibility(compute))]
    #[sampler(4)]
    pub(crate) u_wind_tex: Handle<Image>,
    #[storage_texture(2, image_format = Rgba8Unorm, access = WriteOnly)]
    pub(crate) output_tex: Handle<Image>,
}


#[derive(Resource)]
pub struct ClearBindGroup (pub(crate) BindGroup);
fn prepare_bind_group(
    mut commands: Commands,
    gpu_images: Res<RenderAssets<Image>>,
    advection_image: Res<ClearImage>,
    render_device: Res<RenderDevice>,
    advection_pipeline: Res<ClearPipeline>,
    time: Res<Time>,
    fluid_config: Res<FluidConfig>,
) {
    let u_texture_tex_view = gpu_images.get(&advection_image.u_texture_tex).unwrap();
    let u_wind_tex_view = gpu_images.get(&advection_image.u_wind_tex).unwrap();
    let output_tex_view = gpu_images.get(&advection_image.output_tex).unwrap();


    let u_wind_sampler = render_device.create_sampler(&SamplerDescriptor {
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..Default::default()
    });
    let u_texture_sampler = render_device.create_sampler(&SamplerDescriptor {
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..Default::default()
    });
    let uniforms = ClearUniforms {
        value: 0.8
    };
    let uniform_buffer=render_device.create_buffer_with_data(&BufferInitDescriptor {
        label:  Some("clear_uniform_buffer"),
        contents:bytemuck::cast_slice( & [uniforms]),
        usage:BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });
    let bind_group = render_device.create_bind_group(
        "clear_bind_group",
        &advection_pipeline.bind_group_layout,
        &BindGroupEntries::sequential
            (
                (
                    &u_texture_tex_view.texture_view,
                    &u_wind_tex_view.texture_view,
                    &output_tex_view.texture_view,
                    &u_texture_sampler,
                    &u_wind_sampler,
                    BindingResource::Buffer(BufferBinding {
                        buffer: &uniform_buffer,
                        offset: 0,
                        size: None,
                    })
                )
            )

    );

    commands.insert_resource(ClearBindGroup(bind_group));
}
#[derive(Debug, Hash, PartialEq, Eq, Clone,RenderLabel)]
pub(crate) struct ClearComputeLabel;
#[derive(Default)]
pub(crate) struct ClearComputeNode;

impl render_graph::Node for ClearComputeNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let Clear_pipeline = world.resource::<ClearPipeline>();
        let Clear_bind_group = world.resource::<ClearBindGroup>();

        // println!("Clear Compute Pass");
        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("clear Compute Pass"),
                ..Default::default()
            });

        if let Some(pipeline) = pipeline_cache.get_compute_pipeline(Clear_pipeline.pipeline) {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &Clear_bind_group.0, &[]);
            pass.dispatch_workgroups(WIDTH / WORKGROUP_SIZE, HEIGHT / WORKGROUP_SIZE, 1);
        }

        Ok(())
    }
}

