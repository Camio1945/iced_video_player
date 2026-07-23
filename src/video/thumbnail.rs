use super::{Internal, Video};
use crate::Error;
use iced::widget::image as img;
use std::num::NonZeroU8;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

impl Video {
    /// Generates a list of thumbnails based on a set of positions in the media, downscaled by a given factor.
    ///
    /// Slow; only needs to be called once for each instance.
    /// It's best to call this at the very start of playback, otherwise the position may shift.
    pub fn thumbnails<I>(
        &mut self,
        positions: I,
        downscale: NonZeroU8,
    ) -> Result<Vec<img::Handle>, Error>
    where
        I: IntoIterator<Item = super::Position>,
    {
        let downscale = u8::from(downscale) as u32;

        let paused = self.paused();
        let muted = self.muted();
        let pos = self.position();

        self.set_paused(false);
        self.set_muted(true);

        let out = {
            let inner = self.read();
            positions
                .into_iter()
                .map(|pos| Self::capture_thumbnail(&inner, pos, downscale))
                .collect()
        };

        self.set_paused(paused);
        self.set_muted(muted);
        self.seek(pos, true)?;

        out
    }

    fn capture_thumbnail(
        inner: &Internal,
        pos: super::Position,
        downscale: u32,
    ) -> Result<img::Handle, Error> {
        inner.seek(pos, true)?;
        inner.upload_frame.store(false, Ordering::SeqCst);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !inner.upload_frame.load(Ordering::SeqCst) {
            if !inner.alive.load(Ordering::SeqCst) {
                return Err(Error::Lock);
            }
            if Instant::now() > deadline {
                return Err(Error::Sync);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        let frame_guard = inner.frame.lock().map_err(|_| Error::Lock)?;
        let frame = frame_guard.readable().ok_or(Error::Lock)?;
        let stride = frame_guard.stride();
        Ok(img::Handle::from_rgba(
            inner.width as u32 / downscale,
            inner.height as u32 / downscale,
            yuv_to_rgba(
                frame.as_slice(),
                inner.width as _,
                inner.height as _,
                downscale,
                stride,
            ),
        ))
    }
}

fn nv12_pixel_to_rgba(yuv: &[u8], y_offset: usize, uv_offset: usize) -> [u8; 4] {
    let y = yuv[y_offset] as f32;
    let u = yuv[uv_offset] as f32;
    let v = yuv[uv_offset + 1] as f32;

    let r = 1.164 * (y - 16.0) + 1.596 * (v - 128.0);
    let g = 1.164 * (y - 16.0) - 0.813 * (v - 128.0) - 0.391 * (u - 128.0);
    let b = 1.164 * (y - 16.0) + 2.018 * (u - 128.0);

    [r as u8, g as u8, b as u8, 0xFF]
}

fn yuv_to_rgba(
    yuv: &[u8],
    width: u32,
    height: u32,
    downscale: u32,
    stride: Option<u32>,
) -> Vec<u8> {
    let stride = stride.unwrap_or(width);
    let uv_start = stride * height;
    let mut rgba = Vec::with_capacity(((width / downscale) * (height / downscale) * 4) as usize);

    for y in 0..height / downscale {
        for x in 0..width / downscale {
            let x_src = x * downscale;
            let y_src = y * downscale;
            let y_offset = (y_src * stride + x_src) as usize;
            let uv_offset = (uv_start + (y_src / 2) * stride + (x_src / 2) * 2) as usize;
            rgba.extend_from_slice(&nv12_pixel_to_rgba(yuv, y_offset, uv_offset));
        }
    }

    rgba
}
