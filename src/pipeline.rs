use iced_wgpu::primitive::Pipeline;
use iced_wgpu::wgpu;
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

#[repr(C)]
pub(crate) struct Uniforms {
    pub(crate) rect: [f32; 4],
    // because wgpu min_uniform_buffer_offset_alignment
    _pad: [u8; 240],
}

pub(crate) struct VideoEntry {
    pub(crate) texture_y: wgpu::Texture,
    pub(crate) texture_uv: wgpu::Texture,
    pub(crate) instances: wgpu::Buffer,
    pub(crate) bg0: wgpu::BindGroup,
    pub(crate) alive: Arc<AtomicBool>,
    pub(crate) prepare_index: AtomicUsize,
    pub(crate) render_index: AtomicUsize,
}

pub(crate) struct VideoPipeline {
    pipeline: wgpu::RenderPipeline,
    bg0_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    videos: BTreeMap<u64, VideoEntry>,
}

impl Pipeline for VideoPipeline {
    fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        use crate::pipeline_helpers;
        let shader = pipeline_helpers::create_shader_module(device);
        let bg0_layout = pipeline_helpers::create_bind_group_layout(device);
        let pipeline =
            pipeline_helpers::create_render_pipeline(device, &bg0_layout, &shader, format);
        let sampler = pipeline_helpers::create_sampler(device);
        VideoPipeline {
            pipeline,
            bg0_layout,
            sampler,
            videos: BTreeMap::new(),
        }
    }

    fn trim(&mut self) {
        let ids: Vec<_> = self
            .videos
            .iter()
            .filter_map(|(id, entry)| (!entry.alive.load(Ordering::SeqCst)).then_some(*id))
            .collect();
        for id in ids {
            if let Some(video) = self.videos.remove(&id) {
                video.texture_y.destroy();
                video.texture_uv.destroy();
                video.instances.destroy();
            }
        }
    }
}

impl VideoPipeline {
    pub(crate) fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        video_id: u64,
        alive: &Arc<AtomicBool>,
        (width, height): (u32, u32),
        frame: &[u8],
        stride: Option<u32>,
    ) {
        use crate::pipeline_helpers;
        let stride = stride.unwrap_or(width);
        if !self.videos.contains_key(&video_id) {
            let entry = pipeline_helpers::make_video_entry(
                device,
                &self.bg0_layout,
                &self.sampler,
                width,
                height,
                alive,
            );
            self.videos.insert(video_id, entry);
        }

        let VideoEntry {
            texture_y,
            texture_uv,
            ..
        } = self.videos.get(&video_id).unwrap();

        pipeline_helpers::write_y_texture(queue, texture_y, frame, stride, width, height);
        pipeline_helpers::write_uv_texture(queue, texture_uv, frame, stride, width, height);
    }

    pub(crate) fn prepare(&mut self, queue: &wgpu::Queue, video_id: u64, bounds: &iced::Rectangle) {
        if let Some(video) = self.videos.get_mut(&video_id) {
            let uniforms = Uniforms {
                rect: [
                    bounds.x,
                    bounds.y,
                    bounds.x + bounds.width,
                    bounds.y + bounds.height,
                ],
                _pad: [0; 240],
            };
            queue.write_buffer(
                &video.instances,
                (video.prepare_index.load(Ordering::Relaxed) * std::mem::size_of::<Uniforms>())
                    as u64,
                {
                    let ptr = &uniforms as *const Uniforms as *const u8;
                    let len = std::mem::size_of::<Uniforms>();
                    // SAFETY: Uniforms is #[repr(C)] and stack-local; the pointer
                    // and length produce a valid byte slice for the lifetime of uniforms.
                    unsafe { std::slice::from_raw_parts(ptr, len) }
                },
            );
            video.prepare_index.fetch_add(1, Ordering::Relaxed);
            video.render_index.store(0, Ordering::Relaxed);
        }
    }

    pub(crate) fn draw(
        &self,
        target: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        clip: &iced::Rectangle<u32>,
        video_id: u64,
    ) {
        use crate::pipeline_helpers;
        if let Some(video) = self.videos.get(&video_id) {
            let mut pass = pipeline_helpers::begin_video_render_pass(encoder, target);
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(
                0,
                &video.bg0,
                &[
                    (video.render_index.load(Ordering::Relaxed) * std::mem::size_of::<Uniforms>())
                        as u32,
                ],
            );
            pass.set_scissor_rect(clip.x as _, clip.y as _, clip.width as _, clip.height as _);
            pass.draw(0..6, 0..1);
            video.prepare_index.store(0, Ordering::Relaxed);
            video.render_index.fetch_add(1, Ordering::Relaxed);
        }
    }
}
