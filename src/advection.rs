use std::borrow::Cow;
use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::*;
use bevy::render::render_resource::binding_types::{sampler, texture_2d, texture_storage_2d, uniform_buffer};
use bevy::render::{render_graph, Render, RenderApp, RenderSet};
use bevy::render::render_graph::{NodeRunError, RenderGraph, RenderGraphContext, RenderLabel};
use bevy::render::renderer::{RenderContext, RenderDevice};
use crate::{FluidConfig, FluidTextures, HEIGHT, WIDTH, WORKGROUP_SIZE};
use crate::universe::CellGrid;

pub struct AdvectionPlugin;
impl Plugin for AdvectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<VelocityAdvectionImage>::default())
        .add_plugins(ExtractResourcePlugin::<DensityAdvectionImage>::default())
            .add_systems(Update,
                         (
                             update_burns_and_cells_textures,
                             swap_velocity_buffer.after(update_burns_and_cells_textures),
                         // check_density_texture.after(swap_velocity_buffer)
            ));
        ;
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            (
                prepare_velocity_bind_group.in_set(RenderSet::PrepareBindGroups),
                prepare_density_bind_group.in_set(RenderSet::PrepareBindGroups),
            )
        )
        ;


    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<AdvectionPipeline>();
    }
}

// 定义渲染标签
#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct VelocityAdvectionLabel;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct DensityAdvectionLabel;

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
                    sampler(SamplerBindingType::Filtering),
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
            entry_point: Cow::from("advection_main"),
        });



        AdvectionPipeline {
            pipeline,
            bind_group_layout
        }
    }
}
#[derive(Resource, Clone, ExtractResource, AsBindGroup)]
pub struct VelocityAdvectionImage {
    #[texture(0, visibility(compute))]
    #[sampler(4)]
    pub(crate) velocity_tex: Handle<Image>,
    #[texture(1, visibility(compute))]
    #[sampler(5)]
    pub(crate) source_tex: Handle<Image>,
    #[texture(2, visibility(compute))]
    #[sampler(6)]
    pub(crate) wind_tex: Handle<Image>,
    #[storage_texture(3, image_format = Rgba8Unorm, access = ReadWrite)]
    pub(crate) output_tex: Handle<Image>,
}
#[derive(Resource, Clone, ExtractResource, AsBindGroup)]
pub struct DensityAdvectionImage {
    #[texture(0, visibility(compute))]
    #[sampler(4)]
    pub(crate) wind_tex: Handle<Image>,
    #[texture(1, visibility(compute))]
    #[sampler(5)]
    pub(crate) velocity_tex: Handle<Image>,
    #[texture(2, visibility(compute))]
    #[sampler(6)]
    pub(crate) source_tex: Handle<Image>,

    #[storage_texture(3, image_format = Rgba8Unorm, access = ReadWrite)]
    pub(crate) output_tex: Handle<Image>,
}
#[derive(Resource)]
pub struct VelocityAdvectionBindGroup (pub(crate) BindGroup);
#[derive(Resource)]
pub struct DensityAdvectionBindGroup (pub(crate) BindGroup);

fn prepare_velocity_bind_group(
    mut commands: Commands,
    gpu_images: Res<RenderAssets<Image>>,
    advection_image: Res<VelocityAdvectionImage>,
    render_device: Res<RenderDevice>,
    advection_pipeline: Res<AdvectionPipeline>,
    time: Res<Time>,
    fluid_config: Res<FluidConfig>,
) {
    let velocity_tex_view = gpu_images.get(&advection_image.velocity_tex).unwrap();
    let source_tex_view = gpu_images.get(&advection_image.source_tex).unwrap();
    let wind_tex_view = gpu_images.get(&advection_image.wind_tex).unwrap();
    let output_tex_view = gpu_images.get(&advection_image.output_tex).unwrap();



    let source_sampler=sampler_create("source_sampler",&render_device);
    let wind_sampler =sampler_create("wind_sampler ",&render_device);
    let velocity_sampler =sampler_create("velocity_sampler ",&render_device);

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
                    &velocity_sampler,
                    &source_sampler,
                    &wind_sampler,
                    BindingResource::Buffer(BufferBinding {
                        buffer: &uniform_buffer,
                        offset: 0,
                        size: None,
                    })
                )
            )

    );

    commands.insert_resource(VelocityAdvectionBindGroup(bind_group));

}
fn sampler_create(str:&str,render_device:&Res<RenderDevice>) -> Sampler {
    render_device.create_sampler( &SamplerDescriptor {
        label: Some(str),
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..Default::default()
    })
}
fn prepare_density_bind_group(
    mut commands: Commands,
    gpu_images: Res<RenderAssets<Image>>,
    advection_image: Res<DensityAdvectionImage>,
    render_device: Res<RenderDevice>,
    advection_pipeline: Res<AdvectionPipeline>,
    time: Res<Time>,
    fluid_config: Res<FluidConfig>,
) {

    let wind_view  = gpu_images.get(&advection_image.wind_tex).unwrap();
    let velocity_view = gpu_images.get(&advection_image.velocity_tex).unwrap();
    let source_view  = gpu_images.get(&advection_image.source_tex).unwrap();
    let output_tex_view = gpu_images.get(&advection_image.output_tex).unwrap();

    // let sampler = render_device.create_sampler(&SamplerDescriptor {
    //     address_mode_u: AddressMode::ClampToEdge,
    //     address_mode_v: AddressMode::ClampToEdge,
    //     mag_filter: FilterMode::Linear,
    //     min_filter: FilterMode::Linear,
    //     ..Default::default()
    // });
    let wind_sampler =sampler_create("wind_sampler ",&render_device);
    let velocity_sampler  =sampler_create("velocity_sampler  ",&render_device);
    let source_sampler  =sampler_create("source_sampler  ",&render_device);

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
                    &wind_view.texture_view,
                    &velocity_view.texture_view,
                    &source_view.texture_view,
                    &output_tex_view.texture_view,
                    &wind_sampler,
                    &velocity_sampler,
                    &source_sampler,
                    BindingResource::Buffer(BufferBinding {
                        buffer: &uniform_buffer,
                        offset: 0,
                        size: None,
                    })
                )
            )

    );

    commands.insert_resource(DensityAdvectionBindGroup(bind_group));

}
#[derive(Default)]
pub(crate) struct VelocityAdvectionComputeNode;
#[derive(Default)]
pub(crate) struct DensityAdvectionComputeNode;

#[derive(Debug, Hash, PartialEq, Eq, Clone,RenderLabel)]
pub(crate) struct VelocityAdvectionComputeLabel;
#[derive(Debug, Hash, PartialEq, Eq, Clone,RenderLabel)]
pub(crate) struct DensityAdvectionComputeLabel;
impl render_graph::Node for VelocityAdvectionComputeNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let advection_pipeline = world.resource::<AdvectionPipeline>();
        let advection_bind_group = world.resource::<VelocityAdvectionBindGroup>();
        // println!("Velocity Compute Pass");
        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("Velocity Compute Pass"),
                ..default()});
        pass.set_bind_group(0, &advection_bind_group.0, &[]);
        if let CachedPipelineState::Ok(_) =
            pipeline_cache.get_compute_pipeline_state(advection_pipeline.pipeline)
        {

            let update_pipeline = pipeline_cache
                .get_compute_pipeline(advection_pipeline.pipeline)
                .unwrap();
            pass.set_pipeline(update_pipeline);
            pass.dispatch_workgroups(WIDTH / WORKGROUP_SIZE, HEIGHT / WORKGROUP_SIZE, 1);
        }else {

        }
       Ok(())
    }
}
fn swap_velocity_buffer(mut fluid_textures: ResMut<FluidTextures>) {
    let velocity = &mut fluid_textures.velocity;
    let prev_front = velocity.0.clone();
    let prev_back = velocity.1.clone();

    std::mem::swap(&mut velocity.0, &mut velocity.1);
    // info!("Velocity: {} -> {}, {} -> {}",
    //     prev_front, fluid_textures.velocity.1,
    //     prev_back, fluid_textures.velocity.0);
    let density = &mut fluid_textures.density;
    std::mem::swap(&mut density.0, &mut density.1);
}

impl render_graph::Node for DensityAdvectionComputeNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let advection_pipeline = world.resource::<AdvectionPipeline>();
        let advection_bind_group = world.resource::<DensityAdvectionBindGroup>();
        // println!("Density Compute Pass");
        // 创建计算通道
        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("Density Compute Pass"),
                ..default()});

        pass.set_bind_group(0, &advection_bind_group.0, &[]);

        if let CachedPipelineState::Ok(_) =
            pipeline_cache.get_compute_pipeline_state(advection_pipeline.pipeline)
        {
            if let Some(update_pipeline) = pipeline_cache.get_compute_pipeline(advection_pipeline.pipeline) {
                pass.set_pipeline(update_pipeline);
                pass.dispatch_workgroups(WIDTH / WORKGROUP_SIZE, HEIGHT / WORKGROUP_SIZE, 1);
            } else {
                warn!("Compute pipeline not found in cache");
            }
        } else {
            warn!("Pipeline not ready");
        }


        Ok(())
    }
}

// 更新燃烧和细胞纹理的系统
fn update_burns_and_cells_textures(
    mut images: ResMut<Assets<Image>>,
    fluid_textures: Res<FluidTextures>,
    cell_grid: Res<CellGrid>,
) {
     // 更新燃烧纹理
    if let Some(image) = images.get_mut(&fluid_textures.burns) {
        let pixels = image.data.as_mut_slice();
        for (i, wind) in cell_grid.winds.iter().enumerate() {
            let idx = i * 4;
            pixels[idx] = wind.dx;
            pixels[idx + 1] = wind.dy;
            pixels[idx + 2] = wind.pressure;
            pixels[idx + 3] = wind.density;

        }
    }

    // 更新细胞纹理
    if let Some(image) = images.get_mut(&fluid_textures.cells) {
        let pixels = image.data.as_mut_slice();
        for (i, cell) in cell_grid.cells.iter().enumerate() {
            let idx = i * 4;
            pixels[idx] = cell.species as u8;
            pixels[idx + 1] = cell.ra;
            pixels[idx + 2] = cell.rb;
            pixels[idx + 3] = cell.clock;
        }
    }
}
fn check_density_texture(
    images: Res<Assets<Image>>,
    fluid_textures: Res<FluidTextures>,
    frame_count: Local<u32>,
) {
    let frame = *frame_count;
    if frame % 60 == 0 { // 每秒检查一次
        if let Some(image) = images.get(&fluid_textures.density.0) {
            let sample_pos = 0; // 检查第一个像素
            if image.data.len() >= 4 {
                let r = image.data[sample_pos];
                let g = image.data[sample_pos + 1];
                let b = image.data[sample_pos + 2];
                let a = image.data[sample_pos + 3];
                info!("Density texture sample: R:{}, G:{}, B:{}, A:{}", r, g, b, a);
            }
        }

        if let Some(image) = images.get(&fluid_textures.density.1) {
            let sample_pos = 0;
            if image.data.len() >= 4 {
                let r = image.data[sample_pos];
                let g = image.data[sample_pos + 1];
                let b = image.data[sample_pos + 2];
                let a = image.data[sample_pos + 3];
                info!("Density texture (back buffer) sample: R:{}, G:{}, B:{}, A:{}", r, g, b, a);
            }
        }
    }
}
// 假设的数据资源
#[derive(Resource)]
pub struct BurnsData(pub Vec<u8>);

#[derive(Resource)]
pub struct CellsData(pub Vec<u8>);
