use super::pipeline::{Uniforms, VideoEntry};
use iced_wgpu::wgpu;
use std::{
    num::NonZero,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize},
    },
};

pub(crate) fn create_shader_module(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("iced_video_player shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    })
}

pub(crate) fn texture_binding_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

pub(crate) fn sampler_binding_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

pub(crate) fn uniform_binding_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: true,
            min_binding_size: None,
        },
        count: None,
    }
}

pub(crate) fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let entries = [
        texture_binding_entry(0),
        texture_binding_entry(1),
        sampler_binding_entry(2),
        uniform_binding_entry(3),
    ];
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("iced_video_player bind group 0 layout"),
        entries: &entries,
    })
}

pub(crate) fn create_pipeline_layout(
    device: &wgpu::Device,
    bg0_layout: &wgpu::BindGroupLayout,
) -> wgpu::PipelineLayout {
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("iced_video_player pipeline layout"),
        bind_group_layouts: &[bg0_layout],
        push_constant_ranges: &[],
    })
}

pub(crate) fn create_render_pipeline(
    device: &wgpu::Device,
    bg0_layout: &wgpu::BindGroupLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let layout = create_pipeline_layout(device, bg0_layout);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("iced_video_player pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        multiview: None,
        cache: None,
    })
}

pub(crate) fn create_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("iced_video_player sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        lod_min_clamp: 0.0,
        lod_max_clamp: 1.0,
        compare: None,
        anisotropy_clamp: 1,
        border_color: None,
    })
}

pub(crate) fn create_y_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("iced_video_player texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

pub(crate) fn create_uv_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("iced_video_player texture"),
        size: wgpu::Extent3d {
            width: width / 2,
            height: height / 2,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rg8Unorm,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

pub(crate) fn create_default_texture_view(texture: &wgpu::Texture) -> wgpu::TextureView {
    texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("iced_video_player texture view"),
        format: None,
        dimension: None,
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None,
        usage: None,
    })
}

pub(crate) fn create_instance_buffer(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("iced_video_player uniform buffer"),
        size: 256 * std::mem::size_of::<Uniforms>() as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        mapped_at_creation: false,
    })
}

pub(crate) fn create_video_bind_group(
    device: &wgpu::Device,
    bg0_layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    view_y: &wgpu::TextureView,
    view_uv: &wgpu::TextureView,
    instances: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("iced_video_player bind group"),
        layout: bg0_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view_y),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(view_uv),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: instances,
                    offset: 0,
                    size: Some(NonZero::new(std::mem::size_of::<Uniforms>() as _).unwrap()),
                }),
            },
        ],
    })
}

pub(crate) fn write_y_texture(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    frame: &[u8],
    stride: u32,
    width: u32,
    height: u32,
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &frame[..(stride * height) as usize],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(stride),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}

pub(crate) fn write_uv_texture(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    frame: &[u8],
    stride: u32,
    width: u32,
    height: u32,
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &frame[(stride * height) as usize..],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(stride),
            rows_per_image: Some(height / 2),
        },
        wgpu::Extent3d {
            width: width / 2,
            height: height / 2,
            depth_or_array_layers: 1,
        },
    );
}

pub(crate) fn make_video_entry(
    device: &wgpu::Device,
    bg0_layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    width: u32,
    height: u32,
    alive: &Arc<AtomicBool>,
) -> VideoEntry {
    let texture_y = create_y_texture(device, width, height);
    let texture_uv = create_uv_texture(device, width, height);
    let view_y = create_default_texture_view(&texture_y);
    let view_uv = create_default_texture_view(&texture_uv);
    let instances = create_instance_buffer(device);
    let bind_group =
        create_video_bind_group(device, bg0_layout, sampler, &view_y, &view_uv, &instances);
    VideoEntry {
        texture_y,
        texture_uv,
        instances,
        bg0: bind_group,
        alive: Arc::clone(alive),
        prepare_index: AtomicUsize::new(0),
        render_index: AtomicUsize::new(0),
    }
}

pub(crate) fn begin_video_render_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    target: &'a wgpu::TextureView,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("iced_video_player render pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    })
}
