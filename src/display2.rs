// use std::borrow::Cow;
// use bevy::core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state;
// use bevy::ecs::query::QueryItem;
// use bevy::prelude::*;
// use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
// use bevy::render::render_asset::RenderAssets;
// use bevy::render::render_resource::*;
// use bevy::render::render_resource::binding_types::{sampler, texture_2d, texture_storage_2d, uniform_buffer};
// use bevy::render::{render_graph, Render, RenderApp, RenderSet};
// use bevy::render::render_graph::{NodeRunError, RenderGraph, RenderGraphContext, RenderGraphError, RenderLabel, ViewNode};
// use bevy::render::renderer::{RenderContext, RenderDevice};
// use bevy::render::texture::BevyDefault;
// use bevy::render::view::ViewTarget;
// use crate::{FluidConfig, HEIGHT, WIDTH, WORKGROUP_SIZE};
//
//
// pub struct DisPlayPlugin;
// impl Plugin for DisPlayPlugin {
//     fn build(&self, app: &mut App) {
//         app.add_plugins(ExtractResourcePlugin::<DisPlayImage>::default());
//         let render_app = app.sub_app_mut(RenderApp);
//         render_app.add_systems(
//             Render,
//             (prepare_bind_group).in_set(RenderSet::PrepareBindGroups),
//         )
//         ;
//
//     }
//
//     fn finish(&self, app: &mut App) {
//         let render_app = app.sub_app_mut(RenderApp);
//         render_app.init_resource::<DisPlayPipeline>()
//             .init_resource::<FullScreenPipeline>()
//         ;
//     }
// }
//
//
// // 存储DisPlay计算管线的资源
// #[derive(Resource)]
// pub struct DisPlayPipeline {
//     pub(crate) pipeline: CachedComputePipelineId,
//     bind_group_layout: BindGroupLayout,
// }
//
// impl FromWorld for DisPlayPipeline {
//     fn from_world(world: &mut World) -> Self {
//         let render_device = world.resource::<RenderDevice>();
//         // let bind_group_layout = AdvectionImage::bind_group_layout(render_device);
//         let bind_group_layout = render_device.create_bind_group_layout(
//             "display_bind_group_layout",
//             &BindGroupLayoutEntries::sequential(
//                 ShaderStages::COMPUTE,
//                 (
//                     texture_2d(TextureSampleType::Float { filterable: true }),
//                     texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
//                     sampler(SamplerBindingType::Filtering)
//                 )
//             ));
//         let shader = world
//             .resource::<AssetServer>()
//             .load("display.wgsl");
//         let pipeline_cache = world.resource::<PipelineCache>();
//         let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
//             label: Some(Cow::from("DisPlay")),
//             layout: vec![bind_group_layout.clone()],
//             push_constant_ranges: Vec::new(),
//             shader: shader.clone(),
//             shader_defs: vec![],
//             entry_point: Cow::from("display_main"),
//         });
//         DisPlayPipeline {
//             pipeline,
//             bind_group_layout
//         }
//     }
// }
//
//
// #[derive(Resource, Clone, ExtractResource, AsBindGroup)]
// pub struct DisPlayImage {
//     #[texture(0, visibility(compute))]
//     pub(crate) density_tex: Handle<Image>,
//     #[storage_texture(1, image_format = Rgba8Unorm, access = ReadWrite)]
//     pub(crate) output_tex: Handle<Image>,
// }
//
//
// #[derive(Resource)]
// pub struct DisPlayBindGroup (pub(crate) BindGroup);
// fn prepare_bind_group(
//     mut commands: Commands,
//     gpu_images: Res<RenderAssets<Image>>,
//     advection_image: Res<DisPlayImage>,
//     render_device: Res<RenderDevice>,
//     advection_pipeline: Res<DisPlayPipeline>,
// ) {
//     let velocity_tex_view = gpu_images.get(&advection_image.density_tex).unwrap();
//     let output_tex_view = gpu_images.get(&advection_image.output_tex).unwrap();
//
//
//     let sampler = render_device.create_sampler(&SamplerDescriptor {
//         address_mode_u: AddressMode::ClampToEdge,
//         address_mode_v: AddressMode::ClampToEdge,
//         mag_filter: FilterMode::Linear,
//         min_filter: FilterMode::Linear,
//         ..Default::default()
//     });
//
//     let bind_group = render_device.create_bind_group(
//         "display_bind_group",
//         &advection_pipeline.bind_group_layout,
//         &BindGroupEntries::sequential
//             (
//                 (
//                     &velocity_tex_view.texture_view,
//                     &output_tex_view.texture_view,
//                     &sampler
//                 )
//             )
//
//     );
//
//     commands.insert_resource(DisPlayBindGroup(bind_group));
// }
// #[derive(Debug, Hash, PartialEq, Eq, Clone,RenderLabel)]
// pub(crate) struct DisPlayComputeLabel;
// #[derive(Default)]
// pub(crate) struct DisPlayComputeNode;
//
// impl render_graph::Node for DisPlayComputeNode {
//     fn run(
//         &self,
//         graph: &mut RenderGraphContext,
//         render_context: &mut RenderContext,
//         world: &World,
//     ) -> Result<(), NodeRunError> {
//         let pipeline_cache = world.resource::<PipelineCache>();
//         let display_pipeline = world.resource::<DisPlayPipeline>();
//         let display_bind_group = world.resource::<DisPlayBindGroup>();
//
//         println!("DisPlay Compute Pass");
//         let mut pass = render_context
//             .command_encoder()
//             .begin_compute_pass(&ComputePassDescriptor {
//                 label: Some("DisPlay Compute Pass"),
//                 ..Default::default()
//             });
//
//         if let Some(pipeline) = pipeline_cache.get_compute_pipeline(display_pipeline.pipeline) {
//             pass.set_pipeline(pipeline);
//             pass.set_bind_group(0, &display_bind_group.0, &[]);
//             // pass.dispatch_workgroups(WIDTH / WORKGROUP_SIZE, HEIGHT / WORKGROUP_SIZE, 1);
//             let workgroup_size = WORKGROUP_SIZE as u32;
//             let workgroup_count_x = (WIDTH as u32 + workgroup_size - 1) / workgroup_size;
//             let workgroup_count_y = (HEIGHT as u32 + workgroup_size - 1) / workgroup_size;
//
//             pass.dispatch_workgroups(workgroup_count_x, workgroup_count_y, 1);
//         }
//
//         Ok(())
//     }
// }
//
//
//
// //
// // #[derive(Debug, Hash, PartialEq, Eq, Clone,RenderLabel)]
// // pub struct FullScreenRenderLabel;
// // #[derive(Default)]
// // pub struct FullScreenRenderNode;
// //
// // impl ViewNode for FullScreenRenderNode {
// //     type ViewQuery =
// //         &'static ViewTarget
// //     ;
// //     fn run(
// //         &self,
// //         _graph: &mut RenderGraphContext,
// //         render_context: &mut RenderContext,
// //         view_target: QueryItem<Self::ViewQuery>,
// //         world: &World,
// //     ) -> Result<(), NodeRunError> {
// //         let post_process_pipeline = world.resource::<FullScreenPipeline>();
// //         let pipeline_cache = world.resource::<PipelineCache>();
// //         let render_device = world.resource::<RenderDevice>();
// //
// //         // Get the pipeline from the cache
// //         let Some(pipeline) = pipeline_cache.get_render_pipeline(post_process_pipeline.pipeline)
// //
// //         else {
// //             return Ok(());
// //         };
// //
// //
// //
// //         let post_process = view_target.post_process_write();
// //         // 开始渲染通道
// //         let sampler = render_device.create_sampler(&SamplerDescriptor {
// //             address_mode_u: AddressMode::ClampToEdge,
// //             address_mode_v: AddressMode::ClampToEdge,
// //             mag_filter: FilterMode::Linear,
// //             min_filter: FilterMode::Linear,
// //             ..Default::default()
// //         });
// //         let bind_group = render_context.render_device().create_bind_group(
// //             "post_process_bind_group",
// //             &post_process_pipeline.bind_group_layout,
// //             // It's important for this to match the BindGroupLayout defined in the PostProcessPipeline
// //             &BindGroupEntries::sequential((
// //                 // Make sure to use the source view
// //                 post_process.source,
// //                 // Use the sampler created for the pipeline
// //                 &sampler,
// //             )),
// //         );
// //
// //         // 开始渲染通道
// //         let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
// //             label: Some("full_screen_pass"),
// //             color_attachments: &[Some(RenderPassColorAttachment {
// //                 view: post_process.destination,
// //                 resolve_target: None,
// //                 ops: Operations {
// //                     load: LoadOp::Load,
// //                     store: StoreOp::Store,
// //                 },
// //             })],
// //             depth_stencil_attachment: None,
// //             timestamp_writes: None,
// //             occlusion_query_set: None,
// //         });
// //
// //         // 设置全屏渲染管线
// //         render_pass.set_render_pipeline(pipeline);
// //         render_pass.set_bind_group(0, &bind_group, &[]);
// //         render_pass.draw(0..3, 0..1);
// //         println!("FullScreenRender ViewQuery");
// //
// //         Ok(())
// //     }
// // }
// //
// // // 全屏渲染管线
// // #[derive(Resource)]
// // pub struct FullScreenPipeline {
// //     pipeline: CachedRenderPipelineId,
// //     pub bind_group_layout: BindGroupLayout,
// // }
// //
// // impl FromWorld for FullScreenPipeline {
// //     fn from_world(world: &mut World) -> Self {
// //         let render_device = world.resource::<RenderDevice>();
// //         let pipeline_cache = world.resource::<PipelineCache>();
// //         // 创建全屏着色器
// //
// //         let bind_group_layout = render_device.create_bind_group_layout(
// //             "fullscreen_bind_group_layout",
// //             &BindGroupLayoutEntries::sequential(
// //                 ShaderStages::FRAGMENT,
// //                 (
// //                     texture_2d(TextureSampleType::Float { filterable: true }),
// //                     sampler(SamplerBindingType::Filtering),
// //                 ),
// //             ),
// //         );
// //             // shaders.add(Shader::from_wgsl(include_str!("full_screen.wgsl")));
// //
// //         let pipeline = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
// //             label: Some("full_screen_pipeline".into()),
// //             layout: vec![bind_group_layout.clone()],
// //             vertex:fullscreen_shader_vertex_state(),
// //             fragment: Some(FragmentState {
// //                 shader: world.resource::<AssetServer>().load("full_screen.wgsl"),
// //                 shader_defs: vec![],
// //                 entry_point: "fs_main".into(),
// //                 targets: vec![Some(ColorTargetState {
// //                     format: TextureFormat::bevy_default(),
// //                     blend: None,
// //                     write_mask: ColorWrites::ALL,
// //                 })],
// //             }),
// //             primitive: PrimitiveState::default(),
// //             depth_stencil: None,
// //             multisample: MultisampleState::default(),
// //             push_constant_ranges: Vec::new(),
// //         });
// //         println!("FullScreenPipeline loadd");
// //         FullScreenPipeline { pipeline,bind_group_layout, }
// //     }
// // }
//
//
// // fn prepare_fullscreen_bind_group(
// //     mut commands: Commands,
// //     gpu_images: Res<RenderAssets<Image>>,
// //     display_image: Res<DisPlayImage>,
// //     render_device: Res<RenderDevice>,
// //     fullscreen_pipeline: Res<FullScreenPipeline>,
// // ) {
// //     let output_tex_view = match gpu_images.get(&display_image.output_tex) {
// //         Some(view) => &view.texture_view,
// //         None => return,
// //     };
// //
// //     let sampler = render_device.create_sampler(&SamplerDescriptor {
// //         address_mode_u: AddressMode::ClampToEdge,
// //         address_mode_v: AddressMode::ClampToEdge,
// //         mag_filter: FilterMode::Linear,
// //         min_filter: FilterMode::Linear,
// //         ..Default::default()
// //     });
// //
// //     let bind_group = render_device.create_bind_group(
// //         "fullscreen_bind_group",
// //         &fullscreen_pipeline.bind_group_layout,
// //         &BindGroupEntries::sequential((output_tex_view, &sampler)),
// //     );
// //
// //     commands.insert_resource(FullScreenBindGroup(bind_group));
// // }
