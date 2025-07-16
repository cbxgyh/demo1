use bevy::core_pipeline::core_2d;
use bevy::core_pipeline::core_2d::graph::{Core2d, Node2d};
use bevy::prelude::*;
use bevy::render::{Render, RenderApp, RenderSet};
use bevy::render::render_graph::{RenderGraph, RenderGraphApp, ViewNodeRunner};
use crate::advection::AdvectionPlugin;
use crate::curl::CurlPlugin;
use crate::display::DisplayPlugin;
use crate::divergence::DivergencePlugin;
use crate::gradient_subtract::GradientSubtractPlugin;
use crate::pressure::PressurePlugin;
use crate::velocity_out::VelocityOutPlugin;
use crate::vorticity::VorticityPlugin;

pub struct FluidSimulationPlugin;


impl Plugin for FluidSimulationPlugin {
    fn build(&self, app: &mut App) {

        app.add_plugins(AdvectionPlugin)   // 平流插件
            .add_plugins(CurlPlugin)       // 旋度插件
            .add_plugins(VorticityPlugin)  // 涡度应用插件
            //
            .add_plugins(DivergencePlugin) // 散度插件
            .add_plugins(PressurePlugin)   // 压力求解插件
            .add_plugins(VelocityOutPlugin)    // 速度场修正插件
            .add_plugins(GradientSubtractPlugin) // 梯度减法插件
            .add_plugins(DisplayPlugin) // 梯度减法插件
        ;
    }

    fn finish(&self, app: &mut App) {

        let render_app = app.sub_app_mut(RenderApp);
        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();

        // 正确添加节点
        render_graph.add_node(
            crate::advection::VelocityAdvectionComputeLabel,
            crate::advection::VelocityAdvectionComputeNode::default()
        );

        render_graph.add_node(
            crate::advection::DensityAdvectionComputeLabel,
            crate::advection::DensityAdvectionComputeNode::default() // 修正为密度节点
        );

        render_graph.add_node(
            crate::curl::CurlComputeLabel,
            crate::curl::CurlComputeNode::default()
        );

        render_graph.add_node(
            crate::vorticity::VorticityLabel,
            crate::vorticity::VorticityComputeNode::default()
        );

        render_graph.add_node(
            crate::divergence::DivergencComputeLabel,
            crate::divergence::DivergenceComputeNode::default()
        );

        render_graph.add_node(
            crate::pressure::PressureComputeLabel,
            crate::pressure::PressureComputeNode::default()
        );

        render_graph.add_node(
            crate::velocity_out::VorticityOutLabel, // 修正标签名称
            crate::velocity_out::VelocityOutComputeNode::default()
        );

        render_graph.add_node(
            crate::gradient_subtract::GradientLabel,
            crate::gradient_subtract::GradientSubtractComputeNode::default()
        );

        // render_graph.add_node(
        //     crate::display2::DisPlayComputeLabel,
        //     crate::display2::DisPlayComputeNode::default()
        // );

        // render_graph.add_node(
        //     crate::display::FullScreenRenderLabel,
        //     crate::display::FullScreenRenderNode::default()
        // );

        // 建立正确的执行顺序
        render_graph.add_node_edge(
            crate::advection::VelocityAdvectionComputeLabel,
            crate::advection::DensityAdvectionComputeLabel
        );

        render_graph.add_node_edge(
            crate::advection::DensityAdvectionComputeLabel,
            crate::curl::CurlComputeLabel
        );

        render_graph.add_node_edge(
            crate::curl::CurlComputeLabel,
            crate::vorticity::VorticityLabel
        );

        render_graph.add_node_edge(
            crate::vorticity::VorticityLabel,
            crate::divergence::DivergencComputeLabel
        );

        render_graph.add_node_edge(
            crate::divergence::DivergencComputeLabel,
            crate::pressure::PressureComputeLabel
        );

        render_graph.add_node_edge(
            crate::pressure::PressureComputeLabel,
            crate::velocity_out::VorticityOutLabel
        );

        render_graph.add_node_edge(
            crate::velocity_out::VorticityOutLabel,
            crate::gradient_subtract::GradientLabel
        );


        // 只有最后一个节点连接到相机驱动
        render_graph.add_node_edge(
            crate::gradient_subtract::GradientLabel,
            bevy::render::graph::CameraDriverLabel
            // crate::display::DisPlayComputeLabel
        );


        // render_graph.add_node_edge(
        //     crate::display::DisPlayComputeLabel,
        //     bevy::render::graph::CameraDriverLabel
        // );

        // render_graph.add_node_edge(
        //     crate::display::FullScreenRenderLabel,
        //     bevy::render::graph::CameraDriverLabel
        // );

        render_app
            .add_render_graph_node::<ViewNodeRunner<crate::display::DisplayNode>>(
                Core2d,
                crate::display::DisplayLabel,
            )
            .add_render_graph_edges(
                Core2d,
                (
                    Node2d::Tonemapping,
                    crate::display::DisplayLabel,
                    Node2d::EndMainPassPostProcessing,
                ),
            );
        // 添加显示节点到Core2d渲染图
        // render_app
        //     .add_render_graph_node::<ViewNodeRunner<crate::display1::DisplayNode>>(
        //         Core2d,
        //         crate::display1::DisplayLabel,
        //     )
        //     .add_render_graph_edges(
        //         Core2d,
        //         (
        //             Node2d::Tonemapping,
        //             crate::display1::DisplayLabel,
        //             Node2d::EndMainPassPostProcessing,
        //         ),
        //     );
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