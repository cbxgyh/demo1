use std::borrow::Cow;
use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::*;
use bevy::render::render_resource::binding_types::{sampler, texture_2d, texture_storage_2d, uniform_buffer};
use bevy::render::{render_graph, Render, RenderApp, RenderSet};
use bevy::render::render_graph::{NodeRunError, RenderGraph, RenderGraphContext, RenderLabel};
use bevy::render::renderer::{RenderContext, RenderDevice};
use bevy::ui::AlignSelf::Start;
use crate::{setup, FluidConfig, FluidTextures, HEIGHT, WIDTH, WORKGROUP_SIZE};


pub struct CurlPlugin;
impl Plugin for CurlPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<CurlImage>::default())
            .add_systems(Startup,init_velocity_field.after(setup))
            .add_systems(Update,
                         (debug_curl_texture,
                          debug_velocity_texture)
            );
        ;
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
        )
        ;

    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<CurlPipeline>();
    }
}

fn init_velocity_field( mut images: ResMut<Assets<Image>>,
                        fluid_textures: Res<FluidTextures>) {

    if let Some(image) = images.get_mut(&fluid_textures.velocity.0) {
        let center_x = WIDTH as f32 / 2.0;
        let center_y = HEIGHT as f32 / 2.0;
        let pixels = image.data.as_mut_slice();

        println!("init_velocity_field:{:?}",pixels.len());
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let dx = x as f32 - center_x;
                let dy = y as f32 - center_y;
                let distance = (dx * dx + dy * dy).sqrt().max(1.0);
                let vx = -dy / distance * 0.1;
                let vy = dx / distance * 0.1;
                let offset = ((y * WIDTH + x) * 4) as usize;
                pixels[offset] = (vx * 127.5 + 127.5) as u8;
                pixels[offset + 1] = (vy * 127.5 + 127.5) as u8;
                pixels[offset + 2] = 0; // 蓝色通道设为0
                pixels[offset + 3] = 255; // 完全不透明
            }
        }
    }
}
// Curl计算所需的uniform数据
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable,ShaderType)]
pub struct CurlUniforms {
    texel_size: [f32; 2],
}

// 存储Curl计算管线的资源
#[derive(Resource)]
pub struct CurlPipeline {
    pub(crate) pipeline: CachedComputePipelineId,
    bind_group_layout: BindGroupLayout,
}

impl FromWorld for CurlPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        // let bind_group_layout = AdvectionImage::bind_group_layout(render_device);
        let bind_group_layout = render_device.create_bind_group_layout(
            "curl_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<CurlUniforms>(false)
                )
            ));
        let shader = world
            .resource::<AssetServer>()
            .load("curl.wgsl");


        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(Cow::from("curl")),
            layout: vec![bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("curl_main"),
        });
        CurlPipeline {
            pipeline,
            bind_group_layout
        }
    }
}
// 用于调度Curl计算作业的资源
// struct CurlComputeJob {
//     pipeline: Handle<ComputePipeline>,
//     bind_group: BindGroup,
//     size: Extent3d,
// }

#[derive(Resource, Clone, ExtractResource, AsBindGroup)]
pub struct CurlImage {
    #[texture(0, visibility(compute))]
    #[sampler(2)]
    pub(crate) velocity_tex: Handle<Image>,
    #[storage_texture(1, image_format = Rgba8Unorm, access = WriteOnly)]
    pub(crate) output_tex: Handle<Image>,
}


#[derive(Resource)]
pub struct CurlBindGroup (pub(crate) BindGroup);
fn prepare_bind_group(
    mut commands: Commands,
    gpu_images: Res<RenderAssets<Image>>,
    advection_image: Res<CurlImage>,
    render_device: Res<RenderDevice>,
    advection_pipeline: Res<CurlPipeline>,
    time: Res<Time>,
    fluid_config: Res<FluidConfig>,
) {
    let velocity_tex_view = gpu_images.get(&advection_image.velocity_tex).unwrap();
    let output_tex_view = gpu_images.get(&advection_image.output_tex).unwrap();


    let sampler = render_device.create_sampler(&SamplerDescriptor {
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..Default::default()
    });

    let uniforms = CurlUniforms {
        texel_size: [1.0 / WIDTH as f32, 1.0 / HEIGHT as f32]
    };
    let uniform_buffer=render_device.create_buffer_with_data(&BufferInitDescriptor {
        label:  Some("curl_uniform_buffer"),
        contents:bytemuck::cast_slice( & [uniforms]),
        usage:BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });
    let bind_group = render_device.create_bind_group(
        "curl_bind_group",
        &advection_pipeline.bind_group_layout,
        &BindGroupEntries::sequential
            (
                (
                    &velocity_tex_view.texture_view,
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

    commands.insert_resource(CurlBindGroup(bind_group));
}
#[derive(Debug, Hash, PartialEq, Eq, Clone,RenderLabel)]
pub(crate) struct CurlComputeLabel;
#[derive(Default)]
pub(crate) struct CurlComputeNode;

impl render_graph::Node for CurlComputeNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        println!("CurlComputeNode");
        let pipeline_cache = world.resource::<PipelineCache>();
        let curl_pipeline = world.resource::<CurlPipeline>();
        let curl_bind_group = world.resource::<CurlBindGroup>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor {
                label: Some("Curl Compute Pass"),
                ..Default::default()
            });

        if let Some(pipeline) = pipeline_cache.get_compute_pipeline(curl_pipeline.pipeline) {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &curl_bind_group.0, &[]);
            pass.dispatch_workgroups(WIDTH / WORKGROUP_SIZE, HEIGHT / WORKGROUP_SIZE, 1);
        }

        Ok(())
    }
}
fn debug_curl_texture(
    images: Res<Assets<Image>>,
    curl_image: Res<CurlImage>,
    mut frame_count: Local<u32>,
) {
    println!("debug_curl_texture");
    *frame_count += 1;

    // 每10帧检查一次
    if *frame_count % 10 != 0 {
        return;
    }

    if let Some(image) = images.get(&curl_image.output_tex) {
        // 检查多个位置的值
        for (x, y) in [(100, 100), (300, 300), (500, 500)] {
            let offset = (y * WIDTH as usize + x) * 4;
            if let Some(data) = image.data.get(offset..offset + 4) {
                let r = data[0] as f32 / 255.0;
                let g = data[1] as f32 / 255.0;
                let b = data[2] as f32 / 255.0;
                let a = data[3] as f32 / 255.0;
                // info!(
                //     "Curl texture at ({}, {}): R={:.4}, G={:.4}, B={:.4}, A={:.4}",
                //     x, y, r, g, b, a
                // );
            } else {
                warn!("Invalid texture coordinates: ({}, {})", x, y);
            }
        }

        // 检查整个纹理是否非零
        let is_all_zero = image.data.iter().all(|&b| b == 0);
        info!("Curl texture is all zero: {}", is_all_zero);
    } else {
        warn!("Curl output texture not found");
    }
}
fn debug_velocity_texture(
    images: Res<Assets<Image>>,
    curl_image: Res<CurlImage>,
) {
    if let Some(image) = images.get(&curl_image.velocity_tex) {
        // 检查中心点值
        let center_x = WIDTH / 2;
        let center_y = HEIGHT / 2;
        let offset =( (center_y * WIDTH + center_x) * 4 )as usize;

        if let Some(data) = image.data.get(offset..offset+4) {
            // let r = data[0] as f32 / 255.0;
            // let g = data[1] as f32 / 255.0;
            // let b = data[2] as f32 / 255.0;
        //     info!(

        //     "Velocity texture at center: R={:.4}, G={:.4}, B={:.4}",
        //     r, g, b
        // );
        }
        let is_all_zero = image.data.iter().all(|&b| b == 0);
        // info!("Velocity texture is all zero: {}", is_all_zero);
    }

}

