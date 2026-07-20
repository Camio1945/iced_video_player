use crate::pipeline::VideoPipeline;
use crate::video::Frame;
use iced_wgpu::primitive::Primitive;
use iced_wgpu::wgpu;
use std::sync::{Arc, Mutex, atomic::AtomicBool};

#[derive(Debug, Clone)]
pub(crate) struct VideoPrimitive {
    pub video_id: u64,
    pub alive: Arc<AtomicBool>,
    pub frame: Arc<Mutex<Frame>>,
    pub size: (u32, u32),
    pub upload_frame: bool,
}

impl VideoPrimitive {
    pub fn new(
        video_id: u64,
        alive: Arc<AtomicBool>,
        frame: Arc<Mutex<Frame>>,
        size: (u32, u32),
        upload_frame: bool,
    ) -> Self {
        VideoPrimitive {
            video_id,
            alive,
            frame,
            size,
            upload_frame,
        }
    }
}

impl Primitive for VideoPrimitive {
    type Pipeline = VideoPipeline;

    fn prepare(
        &self,
        pipeline: &mut VideoPipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &iced::Rectangle,
        viewport: &iced_wgpu::graphics::Viewport,
    ) {
        if self.upload_frame {
            let frame_guard = self.frame.lock().expect("lock frame mutex");
            let stride = frame_guard.stride();
            if let Some(readable) = frame_guard.readable() {
                pipeline.upload(
                    device,
                    queue,
                    self.video_id,
                    &self.alive,
                    self.size,
                    readable.as_slice(),
                    stride,
                );
            };
        }

        pipeline.prepare(
            queue,
            self.video_id,
            &(*bounds
                * iced::Transformation::orthographic(
                    viewport.logical_size().width as _,
                    viewport.logical_size().height as _,
                )),
        );
    }

    fn render(
        &self,
        pipeline: &Self::Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &iced::Rectangle<u32>,
    ) {
        pipeline.draw(target, encoder, clip_bounds, self.video_id);
    }
}
