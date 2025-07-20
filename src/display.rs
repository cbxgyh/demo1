use std::borrow::Cow;
use bevy::{
    core_pipeline::{
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    ecs::query::QueryItem,
    prelude::*,
    render::{
        extract_component::{ExtractComponent},
        render_graph::{
            NodeRunError, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::*,
        view::ViewTarget,
        Render, RenderApp, RenderSet,
    },
};
use bevy::render::extract_component::ExtractComponentPlugin;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_graph::RenderGraph;
use bevy::render::render_resource::binding_types::{sampler, texture_2d};
use bevy::render::renderer::{RenderContext, RenderDevice};
use bevy::render::texture::BevyDefault;
use crate::FluidTextures;

pub struct DisplayPlugin;

impl Plugin for DisplayPlugin {
    fn build(&self, app: &mut App) {
       app.add_plugins(ExtractResourcePlugin::<DisplayTarget>::default())
           // .add_systems(Update,display_log)
       ;

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            // .init_resource::<DisplayPipeline>()
            // .add_systems(Startup, setup_display_pipeline)
            ;
    }
    fn finish(&self, app: &mut App) {
                let render_app = app.sub_app_mut(RenderApp);
                render_app.init_resource::<DisplayPipeline>()

                ;
            }
}
fn display_log(
    mut fluid_textures: ResMut<FluidTextures>,
    images: Res<Assets<Image>>,
){
    println!("display_log");
    fluid_textures.log(&images);
}
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub(crate) struct DisplayLabel;

// Component, Clone, ExtractComponent
#[derive(Resource,Clone,ExtractResource)]
pub struct DisplayTarget {
    pub(crate) image: Handle<Image>,
}

#[derive(Default)]
pub(crate) struct DisplayNode;

impl ViewNode for DisplayNode {
    type ViewQuery =
    &'static ViewTarget
        // &'static DisplayTarget

     ;

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        view_target: QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {

        let pipeline_cache = world.resource::<PipelineCache>();
        let render_device = world.resource::<RenderDevice>();
        let display_pipeline = world.resource::<DisplayPipeline>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(display_pipeline.pipeline_id) else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();
        let sampler = render_device.create_sampler(&SamplerDescriptor {
                    address_mode_u: AddressMode::ClampToEdge,
                    address_mode_v: AddressMode::ClampToEdge,
                    mag_filter: FilterMode::Linear,
                    min_filter: FilterMode::Linear,
                    ..Default::default()
                });
        // let gpu_images = world.resource::<RenderAssets<Image>>();
        // let Some(output_tex) = gpu_images.get(&display_target.image) else {
        //     return Ok(());
        // };
        let bind_group = render_context.render_device().create_bind_group(
            "post_process_bind_group",
            &display_pipeline.layout,
            // It's important for this to match the BindGroupLayout defined in the PostProcessPipeline
            &BindGroupEntries::sequential((
                // 使用主渲染纹理作为输入
                post_process.source,
                // view_target.main_texture_view(),
                // Use the sampler created for the pipeline
                &sampler
            )),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("display_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                // 输出到独立纹理
                // view:  &output_tex.texture_view,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

#[derive(Resource)]
struct DisplayPipeline {
    layout: BindGroupLayout,
    pipeline_id: CachedRenderPipelineId,
}

impl FromWorld for DisplayPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        // let bind_group_layout = AdvectionImage::bind_group_layout(render_device);
        let bind_group_layout = render_device.create_bind_group_layout(
            "display_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering)
                )
            ));
        //
        // let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        let shader = world
            .resource::<AssetServer>()
            .load("display.wgsl");

        let pipeline =world
            .resource_mut::<PipelineCache>()
            // This will add the pipeline to the cache and queue it's creation
            .queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("post_process_pipeline".into()),
                layout: vec![bind_group_layout.clone()],
                // This will setup a fullscreen triangle for the vertex state
                vertex: fullscreen_shader_vertex_state(),
                fragment: Some(FragmentState {
                    shader,
                    shader_defs: vec![],
                    // Make sure this matches the entry point of your shader.
                    // It can be anything as long as it matches here and in the shader.
                    entry_point: "fragment".into(),
                    targets: vec![Some(ColorTargetState {
                        format: TextureFormat::bevy_default(),
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                // All of the following properties are not important for this effect so just use the default values.
                // This struct doesn't have the Default trait implemented because not all field can have a default value.
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                push_constant_ranges: vec![],
            });

        DisplayPipeline {
            layout: bind_group_layout,
            pipeline_id:pipeline,
        }
    }
}

