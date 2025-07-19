use bevy::core_pipeline::core_2d;
use bevy::core_pipeline::core_2d::graph::{Core2d, Node2d};
use bevy::core_pipeline::core_2d::Transparent2d;
use bevy::ecs::query::QueryItem;
use bevy::prelude::*;
use bevy::render::{Render, RenderApp, RenderSet};
use bevy::render::camera::{ExtractedCamera, Viewport};
use bevy::render::render_graph::{NodeRunError, RenderGraph, RenderGraphApp, RenderGraphContext, RenderLabel, ViewNode, ViewNodeRunner};
use bevy::render::render_phase::{DrawFunctions, RenderPhase};
use bevy::render::render_resource::{LoadOp, Operations, PipelineCache, RenderPassDescriptor, StoreOp};
use bevy::render::renderer::RenderContext;
use bevy::render::view::{ExtractedView, ViewTarget};
use crate::advection::{AdvectionPlugin, DensityAdvectionComputeLabel, DensityAdvectionComputeNode, VelocityAdvectionComputeLabel, VelocityAdvectionComputeNode};
use crate::clear::{ClearComputeLabel, ClearComputeNode, ClearPlugin};
use crate::curl::{CurlComputeLabel, CurlComputeNode, CurlPlugin};
use crate::display::{DisplayLabel, DisplayNode, DisplayPlugin};
use crate::divergence::{DivergencComputeLabel, DivergenceComputeNode, DivergencePlugin};
use crate::gradient_subtract::{GradientLabel, GradientSubtractComputeNode, GradientSubtractPlugin};
use crate::pressure::{PressureComputeLabel, PressureComputeNode, PressurePlugin};
use crate::velocity_out::{VelocityOutComputeNode, VelocityOutPlugin, VorticityOutLabel};
use crate::vorticity::{VorticityComputeNode, VorticityLabel, VorticityPlugin};

pub struct FluidSimulationPlugin;
impl Plugin for FluidSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            AdvectionPlugin,
            CurlPlugin,
            VorticityPlugin,
            DivergencePlugin,
            ClearPlugin,
            PressurePlugin,
            VelocityOutPlugin,
            GradientSubtractPlugin,
            DisplayPlugin,
        ));
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        render_app
            // 添加所有计算节点到Core2d子图
            .add_render_graph_node::<VelocityAdvectionComputeNode>(
                Core2d,
                VelocityAdvectionComputeLabel,
            )
            .add_render_graph_node::<DensityAdvectionComputeNode>(
                Core2d,
                DensityAdvectionComputeLabel,
            )
            .add_render_graph_node::<CurlComputeNode>(
                Core2d,
                CurlComputeLabel,
            )
            .add_render_graph_node::<VorticityComputeNode>(
                Core2d,
                VorticityLabel,
            )
            .add_render_graph_node::<DivergenceComputeNode>(
                Core2d,
                DivergencComputeLabel,
            )
            .add_render_graph_node::<ClearComputeNode>(
                Core2d,
                ClearComputeLabel,
            )
            .add_render_graph_node::<PressureComputeNode>(
                Core2d,
                PressureComputeLabel,
            )
            .add_render_graph_node::<VelocityOutComputeNode>(
                Core2d,
                VorticityOutLabel,
            )
            .add_render_graph_node::<GradientSubtractComputeNode>(
                Core2d,
                GradientLabel,
            )

            // 设置计算节点顺序
            .add_render_graph_edges(
                Core2d,
                (
                    VelocityAdvectionComputeLabel,
                    DensityAdvectionComputeLabel,
                    CurlComputeLabel,
                    VorticityLabel,
                    DivergencComputeLabel,
                    ClearComputeLabel,
                    PressureComputeLabel,
                    VorticityOutLabel,
                    GradientLabel,
                ),
            )

            // 将计算节点链连接到主通道之前
            .add_render_graph_edge(
                Core2d,
                GradientLabel,
                Node2d::MainPass,
            )

            // 添加显示节点（后处理） - 现在放在主通道之后，但在材质渲染之前
            .add_render_graph_node::<ViewNodeRunner<DisplayNode>>(
                Core2d,
                DisplayLabel,
            )

            // 添加材质渲染节点
            .add_render_graph_node::<ViewNodeRunner<CellMaterialNode>>(
                Core2d,
                CellMaterialLabel,
            )

            // 设置正确的执行顺序：
            // 1. 主通道 (Node2d::MainPass)
            // 2. 显示节点 (DisplayLabel)
            // 3. 材质渲染 (CellMaterialLabel)
            .add_render_graph_edges(
                Core2d,
                (
                    Node2d::MainPass,
                    DisplayLabel,
                    CellMaterialLabel,
                    Node2d::EndMainPassPostProcessing,
                ),
            );
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct CellMaterialLabel;
#[derive(Default)]
struct CellMaterialNode;

impl ViewNode for CellMaterialNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static ExtractedView,
        &'static RenderPhase<Transparent2d>,
    );

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, view, transparent_phase): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let draw_functions = world.resource::<DrawFunctions<Transparent2d>>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let pass_descriptor = RenderPassDescriptor {
            label: Some("cell_material_pass"),
            color_attachments: &[Some(view_target.get_color_attachment())],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        let mut render_pass = render_context.begin_tracked_render_pass(pass_descriptor);

        let viewport = Viewport {
            physical_position: UVec2::new(view.viewport[0], view.viewport[1]),
            physical_size: UVec2::new(view.viewport[2], view.viewport[3]),
            depth: Default::default(),
        };
        render_pass.set_camera_viewport(&viewport);

        let view_entity = graph.view_entity();
        transparent_phase.render(&mut render_pass, world, view_entity);

        Ok(())
    }
}
// let render_app = app.sub_app_mut(RenderApp);
//
// let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
// render_graph.add_node(crate::advection::VelocityAdvectionComputeLabel, crate::advection::VelocityAdvectionComputeNode::default());
// render_graph.add_node_edge(crate::advection::VelocityAdvectionComputeLabel, bevy::render::graph::CameraDriverLabel);
//
// render_graph.add_node(crate::advection::DensityAdvectionComputeLabel, crate::advection::VelocityAdvectionComputeNode::default());
// render_graph.add_node_edge(crate::advection::DensityAdvectionComputeLabel, bevy::render::graph::CameraDriverLabel);
//
// render_graph.add_node(crate::curl::CurlComputeLabel, crate::curl::CurlComputeNode::default());
// render_graph.add_node_edge(crate::curl::CurlComputeLabel, bevy::render::graph::CameraDriverLabel);
//
// render_graph.add_node(crate::vorticity::VorticityLabel, crate::vorticity::VorticityComputeNode::default());
// render_graph.add_node_edge(crate::vorticity::VorticityLabel, bevy::render::graph::CameraDriverLabel);
//
// render_graph.add_node(crate::divergence::DivergencComputeLabel,crate::divergence::DivergenceComputeNode::default());
// render_graph.add_node_edge(crate::divergence::DivergencComputeLabel, bevy::render::graph::CameraDriverLabel);
//
// render_graph.add_node(crate::pressure::PressureComputeLabel,  crate::pressure::PressureComputeNode::default());
// render_graph.add_node_edge(crate::pressure::PressureComputeLabel, bevy::render::graph::CameraDriverLabel);
//
// render_graph.add_node(crate::velocity_out::VorticityOutLabel, crate::velocity_out::VelocityOutComputeNode::default());
// render_graph.
// (crate::velocity_out::VorticityOutLabel, bevy::render::graph::CameraDriverLabel);
//
// render_graph.add_node(crate::gradient_subtract::GradientLabel, crate::gradient_subtract::GradientSubtractComputeNode::default());
// render_graph.add_node_edge(crate::gradient_subtract::GradientLabel, bevy::render::graph::CameraDriverLabel);
//
// render_app.add_render_graph_node::<ViewNodeRunner<crate::display::DisplayNode>>(
//     Core2d,
//     crate::display::DisplayLabel,
// )
//     .add_render_graph_edges(
//         Core2d,
//         (
//             Node2d::Tonemapping,
//             crate::display::DisplayLabel,
//             Node2d::EndMainPassPostProcessing,
//         ),
//     );