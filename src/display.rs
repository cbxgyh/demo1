use bevy::{
    core_pipeline::{
        core_2d::graph::{Core2d, Node2d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    ecs::query::QueryItem,
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_graph::{
            NodeRunError, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::*,
        view::ViewTarget,
        Render, RenderApp, RenderSet,
    },
};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::binding_types::{sampler, texture_2d};
use bevy::render::renderer::{RenderContext, RenderDevice};
use bevy::render::texture::BevyDefault;

pub struct DisplayPlugin;

impl Plugin for DisplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<DisplayImage>::default());

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .add_systems(Startup, setup_display_pipeline)
            ;
    }
}

// System to initialize pipeline in render world
fn setup_display_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    mut pipeline_cache: ResMut<PipelineCache>,
) {
    // Create bind group layout
    let layout = render_device.create_bind_group_layout(
        "display_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );

    // Load shader
    let shader = asset_server.load("display.wgsl");

    // Queue pipeline
    let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("display_pipeline".into()),
        layout: vec![layout.clone()],
        vertex: fullscreen_shader_vertex_state(),
        fragment: Some(FragmentState {
            shader,
            shader_defs: vec![],
            entry_point: "fragment".into(),
            targets: vec![Some(ColorTargetState {
                format: TextureFormat::bevy_default(),
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
        }),
        primitive: PrimitiveState::default(),
        depth_stencil: None,
        multisample: MultisampleState::default(),
        push_constant_ranges: vec![],
    });

    commands.insert_resource(DisplayPipeline { layout, pipeline_id });
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub(crate) struct DisplayLabel;

#[derive(Default)]
pub(crate) struct DisplayNode;

impl ViewNode for DisplayNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static DisplayImage,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, display_image): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let display_pipeline = world.resource::<DisplayPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(display_pipeline.pipeline_id) else {
            return Ok(());
        };

        let gpu_images = world.resource::<RenderAssets<Image>>();
        let Some(gpu_image) = gpu_images.get(&display_image.texture) else {
            return Ok(());
        };

        let render_device = world.resource::<RenderDevice>();
        let sampler = render_device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });

        let bind_group = render_device.create_bind_group(
            "display_bind_group",
            &display_pipeline.layout,
            &BindGroupEntries::sequential((
                &gpu_image.texture_view,
                &sampler,
            )),
        );

        let post_process = view_target.post_process_write();
        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("display_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
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

#[derive(Component, Clone, ExtractComponent)]
pub struct DisplayImage {
    pub texture: Handle<Image>,
}

#[derive(Resource)]
struct DisplayPipeline {
    layout: BindGroupLayout,
    pipeline_id: CachedRenderPipelineId,
}