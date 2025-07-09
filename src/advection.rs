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

pub struct AdvectionPlugin;
impl Plugin for AdvectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<AdvectionImage>::default())

        ;
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
        );
        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node(AdvectionComputeLabel, AdvectionComputeNode::default());
        render_graph.add_node_edge(AdvectionComputeLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<AdvectionPipeline>();
    }
}



// 平流计算所需的uniform数据
#[repr(C)]
// #[derive(Debug, Copy, Clone, ShaderType)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable,ShaderType)]
pub struct AdvectionUniforms {
    pub(crate) texel_size: [f32; 2],
    pub(crate) dt: f32,
    dissipation: f32,
}

// 存储平流计算管线的资源
#[derive(Resource)]
pub struct AdvectionPipeline {
    pub(crate) pipeline: CachedComputePipelineId,
    bind_group_layout: BindGroupLayout,
}

impl FromWorld for AdvectionPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        // let bind_group_layout = AdvectionImage::bind_group_layout(render_device);
        let bind_group_layout = render_device.create_bind_group_layout(
            "advection_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<AdvectionUniforms>(false)
                )
        ));


        let shader = world
            .resource::<AssetServer>()
            .load("advection.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();


        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("main"),
        });



        AdvectionPipeline {
            pipeline,
            bind_group_layout
        }
    }
}
#[derive(Resource, Clone, ExtractResource, AsBindGroup)]
pub struct AdvectionImage {
    #[texture(0, visibility(compute))]
    pub(crate) velocity_tex: Handle<Image>,
    #[texture(1, visibility(compute))]
    pub(crate) source_tex: Handle<Image>,
    #[texture(2, visibility(compute))]
    pub(crate) wind_tex: Handle<Image>,
    #[storage_texture(3, image_format = Rgba8Unorm, access = ReadWrite)]
    pub(crate) output_tex: Handle<Image>,
}
#[derive(Resource)]
pub struct AdvectionBindGroup (pub(crate) BindGroup);

fn prepare_bind_group(
    mut commands: Commands,
    gpu_images: Res<RenderAssets<Image>>,
    advection_image: Res<AdvectionImage>,
    render_device: Res<RenderDevice>,
    advection_pipeline: Res<AdvectionPipeline>,
    time: Res<Time>,
    fluid_config: Res<FluidConfig>,
) {
    let velocity_tex_view = gpu_images.get(&advection_image.velocity_tex).unwrap();
    let source_tex_view = gpu_images.get(&advection_image.source_tex).unwrap();
    let wind_tex_view = gpu_images.get(&advection_image.wind_tex).unwrap();
    let output_tex_view = gpu_images.get(&advection_image.output_tex).unwrap();

    let sampler = render_device.create_sampler(&SamplerDescriptor {
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..Default::default()
    });

    let dt = time.delta_seconds().min(0.016);
    let dissipation = fluid_config.velocity_dissipation;
    let uniforms = AdvectionUniforms {
        texel_size: [1.0 / WIDTH as f32, 1.0 / HEIGHT as f32],
        dt,
        dissipation,
    };
    let uniform_buffer=render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: None,
        contents:bytemuck::cast_slice( & [uniforms]),
        usage:BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });
    let bind_group = render_device.create_bind_group(
        Some("advection_bind_group"),
        &advection_pipeline.bind_group_layout,
        &BindGroupEntries::sequential
            (
                (
                    &velocity_tex_view.texture_view,
                    &source_tex_view.texture_view,
                    &wind_tex_view.texture_view,
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

    commands.insert_resource(AdvectionBindGroup(bind_group));

}

#[derive(Default)]
struct AdvectionComputeNode;

#[derive(Debug, Hash, PartialEq, Eq, Clone,RenderLabel)]
struct AdvectionComputeLabel;

impl render_graph::Node for AdvectionComputeNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let advection_pipeline = world.resource::<AdvectionPipeline>();
        let advection_bind_group = world.resource::<AdvectionBindGroup>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());
        pass.set_bind_group(0, &advection_bind_group.0, &[]);
        if let CachedPipelineState::Ok(_) =
            pipeline_cache.get_compute_pipeline_state(advection_pipeline.pipeline)
        {
            let update_pipeline = pipeline_cache
                .get_compute_pipeline(advection_pipeline.pipeline)
                .unwrap();
            pass.set_pipeline(update_pipeline);
            pass.dispatch_workgroups(WIDTH / WORKGROUP_SIZE, HEIGHT / WORKGROUP_SIZE, 1);
        }
        // if let Some(pipeline) = pipeline_cache.get_compute_pipeline(advection_pipeline.pipeline) {
        //     pass.set_pipeline(pipeline);
        //     pass.set_bind_group(0, &advection_bind_group.0, &[]);
        //     pass.dispatch_workgroups(WIDTH / WORKGROUP_SIZE, HEIGHT / WORKGROUP_SIZE, 1);
        // }

        Ok(())
    }
}


// fn setup_advection_pipeline(
//     mut pipelines: ResMut<Assets<ComputePipeline>>,
//     mut shader_assets: ResMut<Assets<Shader>>,
//     mut bind_group_layouts: ResMut<Assets<BindGroupLayout>>,
//     mut commands: &mut Commands,
// )
// {
//     let shader = shader_assets.add(Shader::from_wgsl(include_str!("advection.wgsl")));
//
//     let bind_group_layout = bind_group_layouts.add(BindGroupLayout::new(
//         "advection_bind_group_layout",
//         &[
//             BindingType::Texture {
//                 sample_type: TextureSampleType::Float { filterable: true },
//                 view_dimension: TextureViewDimension::D2,
//                 multisampled: false,
//             },
//             BindingType::Texture {
//                 sample_type: TextureSampleType::Float { filterable: true },
//                 view_dimension: TextureViewDimension::D2,
//                 multisampled: false,
//             },
//             BindingType::Texture {
//                 sample_type: TextureSampleType::Float { filterable: true },
//                 view_dimension: TextureViewDimension::D2,
//                 multisampled: false,
//             },
//             BindingType::StorageTexture {
//                 access: StorageTextureAccess::WriteOnly,
//                 format: TextureFormat::Rgba8Unorm,
//                 view_dimension: TextureViewDimension::D2,
//             },
//             BindingType::Sampler(SamplerBindingType::Filtering),
//             BindingType::Buffer {
//                 ty: BufferBindingType::Uniform,
//                 has_dynamic_offset: false,
//                 min_binding_size: None,
//             },
//         ],
//     ));
//
//     let pipeline = pipelines.add(ComputePipeline::new(
//         ComputePipelineDescriptor {
//             label: Some("Advection Pipeline"),
//             layout: Some(vec![bind_group_layout.clone()]),
//             shader: shader.clone(),
//             shader_defs: vec![],
//             entry_point: "main".into(),
//         },
//     ));
//
//     commands.insert_resource(AdvectionPipeline {
//         pipeline,
//         bind_group_layout,
//     });
// }
// 更新平流计算函数实现
// pub fn run_advection_compute(
//     commands: &mut Commands,
//     advection_pipeline: Res<AdvectionPipeline>,
//     pipeline_cache:Res<PipelineCache>
// )
// {
//     AdvectionComputeJob{
//         pipeline: pipeline_cache.get_compute_pipeline(advection_pipeline.init_pipeline),
//         bind_group,
//         size: Extent3d {
//             width: (WIDTH + 7) / 8,
//             height: (HEIGHT + 7) / 8,
//             depth_or_array_layers: 1,
//         },
//     }
// }
// 更新平流计算函数实现
// pub fn run_advection_compute(
//     commands: &mut Commands,
//     pipeline: &AdvectionPipeline,
//     mut pipelines: ResMut<Assets<ComputePipeline>>,
//     mut images: ResMut<Assets<Image>>,
//     mut buffers: ResMut<Assets<Buffer>>,
//     velocity_tex: &Handle<Image>,
//     source_tex: &Handle<Image>,
//     wind_tex: &Handle<Image>,
//     output_tex: &Handle<Image>,
//     dt: f32,
//     dissipation: f32,
//     render_device:Res<RenderDevice>,
// ) {
//     // 创建uniform缓冲区
//     let uniforms = AdvectionUniforms {
//         texel_size: [1.0 / WIDTH as f32, 1.0 / HEIGHT as f32],
//         dt,
//         dissipation,
//     };
//
//     // let uniform_buffer = buffers.add(
//     //     Buffer::from_data(
//     //         bytemuck::cast_slice(&[uniforms]),
//     //         BufferUsages::UNIFORM | BufferUsages::COPY_DST,
//     //     )
//     // );
//     let uniform_buffer = buffers.add(
//         render_device.create_buffer_with_data(&BufferInitDescriptor {
//             label: None,
//             contents:bytemuck::cast_slice( & [uniforms]),
//             usage:BufferUsages::UNIFORM | BufferUsages::COPY_DST,
//         })
//     );
//
//
//     // 创建采样器
//     let sampler = SamplerDescriptor {
//         address_mode_u: AddressMode::ClampToEdge,
//         address_mode_v: AddressMode::ClampToEdge,
//         mag_filter: FilterMode::Linear,
//         min_filter: FilterMode::Linear,
//         ..Default::default()
//     };
//     // 创建绑定组
//     let bind_group = BindGroup::new(
//         &pipeline.bind_group_layout,
//         &[
//             BindingResource::TextureView(&images.get(velocity_tex).unwrap().texture_view),
//             BindingResource::TextureView(&images.get(source_tex).unwrap().texture_view),
//             BindingResource::TextureView(&images.get(wind_tex).unwrap().texture_view),
//             BindingResource::TextureView(&images.get(output_tex).unwrap().texture_view),
//             BindingResource::Sampler(sampler),
//             BindingResource::Buffer(BufferBinding {
//                 buffer: &uniform_buffer,
//                 offset: 0,
//                 size: None,
//             }),
//         ],
//     );
//
//     // 调度计算着色器
//     commands.insert_resource(AdvectionComputeJob {
//         pipeline: pipeline.pipeline.clone(),
//         bind_group,
//         size: Extent3d {
//             width: (WIDTH + 7) / 8,
//             height: (HEIGHT + 7) / 8,
//             depth_or_array_layers: 1,
//         },
//     });
// }



