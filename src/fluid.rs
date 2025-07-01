// use std::borrow::Cow;
// use bevy::prelude::*;
// use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
// use bevy::render::render_asset::RenderAssets;
// use bevy::render::render_resource::*;
// use bevy::render::render_resource::binding_types::{sampler, texture_2d, texture_storage_2d, uniform_buffer};
// use bevy::render::{Render, RenderApp, RenderSet};
// use bevy::render::renderer::{RenderContext, RenderDevice};
// use crate::{FluidConfig, FluidTextures, HEIGHT, WIDTH};
//
// pub struct FluidPlugin;
// impl Plugin for FluidPlugin {
//     fn build(&self, app: &mut App) {
//         app.add_plugins(ExtractResourcePlugin::<FluidImage>::default());
//         let render_app = app.sub_app_mut(RenderApp);
//         render_app.add_systems(
//             Render,
//             prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
//         );
//
//     }
//
//     fn finish(&self, app: &mut App) {
//         let render_app = app.sub_app_mut(RenderApp);
//         render_app.init_resource::<FluidPipeline>();
//     }
// }
//
// // 平流计算所需的uniform数据
// #[repr(C)]
// // #[derive(Debug, Copy, Clone, ShaderType)]
// #[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// pub struct AdvectionUniforms {
//     texel_size: [f32; 2],
//     dt: f32,
//     dissipation: f32,
// }
//
// // 存储平流计算管线的资源
// #[derive(Resource)]
// pub struct FluidPipeline {
//     pub advection_pipeline: CachedComputePipelineId,
//     pub advection_bind_group_layout: BindGroupLayout,
//     pub curl_bind_group_layout: BindGroupLayout,
//     pub curl_pipeline: CachedComputePipelineId,
// }
//
// impl FromWorld for FluidPipeline {
//     fn from_world(world: &mut World) -> Self {
//         let render_device = world.resource::<RenderDevice>();
//         // let bind_group_layout = FluidImage::bind_group_layout(render_device);
//         let shader = world
//             .resource::<AssetServer>()
//             .load("Fluid.wgsl");
//         let pipeline_cache = world.resource::<PipelineCache>();
//
//         let advection_bind_group_layout = render_device.create_bind_group_layout(
//             "advection_bind_group_layout",
//             &BindGroupLayoutEntries::sequential(
//                 ShaderStages::COMPUTE,
//                 (
//                     texture_2d(TextureSampleType::Float { filterable: true }),
//                     texture_2d(TextureSampleType::Float { filterable: true }),
//                     texture_2d(TextureSampleType::Float { filterable: true }),
//                     texture_2d(TextureSampleType::Float { filterable: true }),
//                     sampler(SamplerBindingType::Filtering),
//                     uniform_buffer(false)
//                 )
//             ));
//         let advection_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
//             label: None,
//             layout: vec![advection_bind_group_layout.clone()],
//             push_constant_ranges: Vec::new(),
//             shader: shader.clone(),
//             shader_defs: vec![],
//             entry_point: Cow::from("advection"),
//         });
//
//         let curl_bind_group_layout = render_device.create_bind_group_layout(
//             "curl_bind_group_layout",
//             &BindGroupLayoutEntries::sequential(
//                 ShaderStages::COMPUTE,
//                 (
//                     texture_2d(TextureSampleType::Float { filterable: true }),
//                     texture_storage_2d(TextureFormat::R16Float, StorageTextureAccess::WriteOnly),
//                     sampler(SamplerBindingType::Filtering),
//                     uniform_buffer(false)
//                 )
//             ));
//         let curl_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
//             label: None,
//             layout: vec![curl_bind_group_layout.clone()],
//             push_constant_ranges: Vec::new(),
//             shader: shader.clone(),
//             shader_defs: vec![],
//             entry_point: Cow::from("curl"),
//         });
//         let  curl_bind_group_layout= render_device.create_bind_group_layout(
//             "curl_bind_group_layout",
//             &BindGroupLayoutEntries::sequential(
//                 ShaderStages::COMPUTE,
//                 (
//                     texture_2d(TextureSampleType::Float { filterable: true }),
//                     texture_storage_2d(TextureFormat::R16Float, StorageTextureAccess::WriteOnly),
//                     sampler(SamplerBindingType::Filtering),
//                     uniform_buffer(false)
//                 )
//             ));
//         let curl_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
//             label: None,
//             layout: vec![curl_bind_group_layout.clone()],
//             push_constant_ranges: Vec::new(),
//             shader: shader.clone(),
//             shader_defs: vec![],
//             entry_point: Cow::from("curl"),
//         });
//         let  divergence_bind_group_layout= render_device.create_bind_group_layout(
//             "divergence_bind_group_layout",
//             &BindGroupLayoutEntries::sequential(
//                 ShaderStages::COMPUTE,
//                 (
//                     texture_2d(TextureSampleType::Float { filterable: true }),
//                     texture_storage_2d(TextureFormat::R16Float, StorageTextureAccess::WriteOnly),
//                     sampler(SamplerBindingType::Filtering),
//                     uniform_buffer(false)
//                 )
//             ));
//         let divergence_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
//             label: None,
//             layout: vec![divergence_bind_group_layout.clone()],
//             push_constant_ranges: Vec::new(),
//             shader: shader.clone(),
//             shader_defs: vec![],
//             entry_point: Cow::from("divergence"),
//         });
//
//         FluidPipeline {
//             advection_pipeline,
//             advection_bind_group_layout,
//             curl_pipeline,
//             curl_bind_group_layout
//         }
//     }
// }
// #[derive(Resource, Clone, ExtractResource, AsBindGroup)]
// struct FluidImage {
//     velocity_tex: Handle<Image>,
//     source_tex: Handle<Image>,
//     wind_tex: Handle<Image>,
//     output_tex: Handle<Image>,
// }
// #[derive(Resource)]
// pub struct FluidBindGroup (pub(crate) BindGroup);
//
// fn prepare_bind_group(
//     mut commands: Commands,
//     gpu_images: Res<RenderAssets<Image>>,
//     Fluid_image: Res<FluidTextures>,
//     render_device: Res<RenderDevice>,
//     Fluid_pipeline: Res<FluidPipeline>,
//     time: Res<Time>,
//     fluid_config: Res<FluidConfig>,
// ) {
//     let velocity_tex_view = gpu_images.get(&Fluid_image.velocity.0).unwrap();
//     let source_tex_view = gpu_images.get(&Fluid_image.source).unwrap();
//     let wind_tex_view = gpu_images.get(&Fluid_image.wind).unwrap();
//     let output_tex_view = gpu_images.get(&Fluid_image.output).unwrap();
//
//
//     let sampler = SamplerDescriptor {
//         address_mode_u: AddressMode::ClampToEdge,
//         address_mode_v: AddressMode::ClampToEdge,
//         mag_filter: FilterMode::Linear,
//         min_filter: FilterMode::Linear,
//         ..Default::default()
//     };
//
//     let dt = time.delta_seconds().min(0.016);
//     let dissipation = fluid_config.velocity_dissipation;
//     let uniforms = FluidUniforms {
//         texel_size: [1.0 / WIDTH as f32, 1.0 / HEIGHT as f32],
//         dt,
//         dissipation,
//     };
//     let uniform_buffer=render_device.create_buffer_with_data(&BufferInitDescriptor {
//         label: None,
//         contents:bytemuck::cast_slice( & [uniforms]),
//         usage:BufferUsages::UNIFORM | BufferUsages::COPY_DST,
//     });
//     let bind_group = render_device.create_bind_group(
//         None,
//         &Fluid_pipeline.bind_group_layout,
//         &BindGroupEntries::sequential
//             (
//                 (
//                     &velocity_tex_view.texture_view,
//                     &source_tex_view.texture_view,
//                     &wind_tex_view.texture_view,
//                     &output_tex_view.texture_view,
//                     &sampler,
//                     BindingResource::Buffer(BufferBinding {
//                         buffer: &uniform_buffer,
//                         offset: 0,
//                         size: None,
//                     })
//                 )
//             )
//
//     );
//
//     commands.insert_resource(FluidBindGroup(bind_group));
//
// }
