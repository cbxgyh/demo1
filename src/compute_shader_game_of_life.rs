//! A compute shader that simulates Conway's Game of Life.
//!
//! Compute shaders use the GPU for computing arbitrary information, that may be independent of what
//! is rendered to the screen.

use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssetUsages,
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
        Render, RenderApp, RenderSet,
    },
    window::WindowPlugin,
};
use std::borrow::Cow;
const SIZE: (u32, u32) = (500, 500);
const WORKGROUP_SIZE: u32 = 8;
pub struct GameOfLifeComputePlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct GameOfLifeLabel;

impl Plugin for GameOfLifeComputePlugin {
    fn build(&self, app: &mut App) {
        // Extract the game of life image resource from the main world into the render world
        // for operation on by the compute shader and display on the sprite.
        app.add_plugins(ExtractResourcePlugin::<GameOfLifeImage>::default());
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
        );

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node(GameOfLifeLabel, GameOfLifeNode::default());
        render_graph.add_node_edge(GameOfLifeLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<GameOfLifePipeline>();
    }
}

#[derive(Resource, Clone, Deref, ExtractResource, AsBindGroup)]
pub struct GameOfLifeImage {
    #[storage_texture(0, image_format = Rgba8Unorm, access = WriteOnly)]
    pub(crate) texture: Handle<Image>,
}

#[derive(Resource)]
pub struct GameOfLifeImageBindGroup(BindGroup);

fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<GameOfLifePipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    game_of_life_image: Res<GameOfLifeImage>,
    render_device: Res<RenderDevice>,
) {
    let view = gpu_images.get(&game_of_life_image.texture).unwrap();
    let bind_group = render_device.create_bind_group(
        None,
        &pipeline.texture_bind_group_layout,
        &BindGroupEntries::single(&view.texture_view),
    );
    commands.insert_resource(GameOfLifeImageBindGroup(bind_group));
}

#[derive(Resource)]
struct GameOfLifePipeline {
    texture_bind_group_layout: BindGroupLayout,
    // init_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId,
}

impl FromWorld for GameOfLifePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let texture_bind_group_layout = GameOfLifeImage::bind_group_layout(render_device);
        let shader = world
            .resource::<AssetServer>()
            .load("game_of_life.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        // let init_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        //     label: None,
        //     layout: vec![texture_bind_group_layout.clone()],
        //     push_constant_ranges: Vec::new(),
        //     shader: shader.clone(),
        //     shader_defs: vec![],
        //     entry_point: Cow::from("init"),
        // });
        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![texture_bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader,
            shader_defs: vec![],
            entry_point: Cow::from("update"),
        });

        GameOfLifePipeline {
            texture_bind_group_layout,
            // init_pipeline,
            update_pipeline,
        }
    }
}

enum GameOfLifeState {
    Loading,
    Init,
    Update,
}

struct GameOfLifeNode {
    state: GameOfLifeState,
}

impl Default for GameOfLifeNode {
    fn default() -> Self {
        Self {
            state: GameOfLifeState::Loading,
        }
    }
}

impl render_graph::Node for GameOfLifeNode {
    // fn update(&mut self, world: &mut World) {
    //     let pipeline = world.resource::<GameOfLifePipeline>();
    //     let pipeline_cache = world.resource::<PipelineCache>();
    //
    //     // if the corresponding pipeline has loaded, transition to the next stage
    //     match self.state {
    //         GameOfLifeState::Loading => {
    //             if let CachedPipelineState::Ok(_) =
    //                 pipeline_cache.get_compute_pipeline_state(pipeline.init_pipeline)
    //             {
    //                 self.state = GameOfLifeState::Init;
    //             }
    //         }
    //         GameOfLifeState::Init => {
    //             if let CachedPipelineState::Ok(_) =
    //                 pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline)
    //             {
    //                 self.state = GameOfLifeState::Update;
    //             }
    //         }
    //         GameOfLifeState::Update => {}
    //     }
    // }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let texture_bind_group = &world.resource::<GameOfLifeImageBindGroup>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<GameOfLifePipeline>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, texture_bind_group, &[]);

        // select the pipeline based on the current state
        // match self.state {
            // GameOfLifeState::Loading => {}
            // GameOfLifeState::Init => {
            //     let init_pipeline = pipeline_cache
            //         .get_compute_pipeline(pipeline.init_pipeline)
            //         .unwrap();
            //     pass.set_pipeline(init_pipeline);
            //     pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
            // }
            // GameOfLifeState::Update => {
        if let CachedPipelineState::Ok(_) =
                            pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline)
        {
            let update_pipeline = pipeline_cache
                .get_compute_pipeline(pipeline.update_pipeline)
                .unwrap();
            pass.set_pipeline(update_pipeline);
            pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
        }
        // }
        // }

        Ok(())
    }
}
