mod construction;
mod public_api;
mod thumbnail;
mod worker;

use crate::Error;
use gstreamer as gst;
use gstreamer::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

/// Position in the media.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Position {
    /// Position based on time.
    ///
    /// Not the most accurate format for videos.
    Time(Duration),
    /// Position based on nth frame.
    Frame(u64),
}

impl From<Position> for gst::GenericFormattedValue {
    fn from(pos: Position) -> Self {
        match pos {
            Position::Time(t) => gst::ClockTime::from_nseconds(t.as_nanos() as _).into(),
            Position::Frame(f) => gst::format::Default::from_u64(f).into(),
        }
    }
}

impl From<Duration> for Position {
    fn from(t: Duration) -> Self {
        Position::Time(t)
    }
}

impl From<u64> for Position {
    fn from(f: u64) -> Self {
        Position::Frame(f)
    }
}

#[derive(Debug)]
pub(crate) struct Frame(gst::Sample);

impl Frame {
    pub fn empty() -> Self {
        Self(gst::Sample::builder().build())
    }

    pub fn readable(&'_ self) -> Option<gst::BufferMap<'_, gst::buffer::Readable>> {
        self.0.buffer().and_then(|x| x.map_readable().ok())
    }

    /// Get the Y-plane stride (line pitch) in bytes from the frame's VideoMeta.
    /// This is critical for proper NV12 decoding, as the stride may differ from width.
    pub fn stride(&self) -> Option<u32> {
        self.0.buffer().and_then(|buffer| {
            buffer
                .meta::<gstreamer_video::VideoMeta>()
                .map(|meta| meta.stride()[0] as u32)
        })
    }
}

#[derive(Debug)]
pub(crate) struct Internal {
    pub(crate) id: u64,

    pub(crate) bus: gst::Bus,
    pub(crate) source: gst::Pipeline,
    pub(crate) alive: Arc<AtomicBool>,
    pub(crate) worker: Option<std::thread::JoinHandle<()>>,

    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) framerate: f64,
    pub(crate) duration: Duration,
    pub(crate) speed: f64,
    pub(crate) sync_av: bool,
    pub(crate) builtin_text_subtitle: bool,
    pub(crate) subtitle_streams: Vec<SubtitleStreamInfo>,

    pub(crate) frame: Arc<Mutex<Frame>>,
    pub(crate) upload_frame: Arc<AtomicBool>,
    pub(crate) last_frame_time: Arc<Mutex<Instant>>,
    pub(crate) looping: bool,
    pub(crate) is_eos: bool,
    pub(crate) restart_stream: bool,
    pub(crate) sync_av_avg: u64,
    pub(crate) sync_av_counter: u64,

    pub(crate) subtitle_text: Arc<Mutex<Option<String>>>,
    pub(crate) upload_text: Arc<AtomicBool>,
    pub(crate) subtitle_image: Arc<Mutex<Option<crate::pgs::PgsImage>>>,
    pub(crate) upload_image: Arc<AtomicBool>,
}

impl Internal {
    pub(crate) fn seek(&self, position: impl Into<Position>, accurate: bool) -> Result<(), Error> {
        let position = position.into();
        let flags = gst::SeekFlags::FLUSH
            | if accurate {
                gst::SeekFlags::ACCURATE
            } else {
                gst::SeekFlags::empty()
            };

        // gstreamer complains if the start & end value types aren't the same
        match &position {
            Position::Time(_) => self.source.seek(
                self.speed,
                flags,
                gst::SeekType::Set,
                gst::GenericFormattedValue::from(position),
                gst::SeekType::Set,
                gst::ClockTime::NONE,
            )?,
            Position::Frame(_) => self.source.seek(
                self.speed,
                flags,
                gst::SeekType::Set,
                gst::GenericFormattedValue::from(position),
                gst::SeekType::Set,
                gst::format::Default::NONE,
            )?,
        };

        self.clear_subtitles();
        Ok(())
    }

    fn clear_subtitles(&self) {
        *self.subtitle_text.lock().expect("lock subtitle_text") = None;
        self.upload_text.store(true, Ordering::SeqCst);
        *self.subtitle_image.lock().expect("lock subtitle_image") = None;
        self.upload_image.store(true, Ordering::SeqCst);
    }

    pub(crate) fn set_speed(&mut self, speed: f64) -> Result<(), Error> {
        let Some(position) = self.source.query_position::<gst::ClockTime>() else {
            return Err(Error::Caps);
        };
        if speed > 0.0 {
            self.source.seek(
                speed,
                gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
                gst::SeekType::Set,
                position,
                gst::SeekType::End,
                gst::ClockTime::from_seconds(0),
            )?;
        } else {
            self.source.seek(
                speed,
                gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
                gst::SeekType::Set,
                gst::ClockTime::from_seconds(0),
                gst::SeekType::Set,
                position,
            )?;
        }
        self.speed = speed;
        Ok(())
    }

    /// Seek to a `ClockTime` position with FLUSH | ACCURATE flags and block until the
    /// pipeline finishes prerolling at the new position.  Used after state transitions
    /// (e.g. loading subtitles) where the pipeline needs a moment to stabilise.
    pub(crate) fn seek_to_position_and_wait(&self, position: gst::ClockTime) -> Result<(), Error> {
        self.source.seek(
            self.speed,
            gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
            gst::SeekType::Set,
            position,
            gst::SeekType::End,
            gst::ClockTime::from_seconds(0),
        )?;
        // The FLUSH seek makes the pipeline go through Ready -> Paused again to
        // preroll at the new position.  Without this wait the subsequent
        // set_state(Playing) will return Async before the preroll finishes,
        // leaving the pipeline stuck in Paused with no frames.
        let _ = self.source.state(gst::ClockTime::from_seconds(5));
        Ok(())
    }

    pub(crate) fn restart_stream(&mut self) -> Result<(), Error> {
        self.is_eos = false;
        self.set_paused(false);
        self.seek(0, false)?;
        Ok(())
    }

    pub(crate) fn set_paused(&mut self, paused: bool) {
        self.source
            .set_state(if paused {
                gst::State::Paused
            } else {
                gst::State::Playing
            })
            .unwrap(/* state was changed in ctor; state errors caught there */);

        // Set restart_stream flag to make the stream restart on the next Message::NextFrame
        if self.is_eos && !paused {
            self.restart_stream = true;
        }
    }

    pub(crate) fn paused(&self) -> bool {
        self.source.state(gst::ClockTime::ZERO).1 == gst::State::Paused
    }

    /// Syncs audio with video when there is (inevitably) latency presenting the frame.
    pub(crate) fn set_av_offset(&mut self, offset: Duration) {
        if self.sync_av {
            self.sync_av_counter = self.sync_av_counter.saturating_add(1);
            let prev = self.sync_av_avg;
            let n = self.sync_av_counter;
            let delta = offset.as_nanos() as i128;
            // Use i128 to avoid overflow and retain precision via rounding
            self.sync_av_avg = ((prev as i128 * (n - 1) as i128 + delta) / n as i128) as u64;
            if n.is_multiple_of(128) {
                self.source
                    .set_property("av-offset", -(self.sync_av_avg as i64));
            }
        }
    }
}

/// A multimedia video loaded from a URI (e.g., a local file path or HTTP stream).
#[derive(Debug)]
pub struct Video(pub(crate) RwLock<Internal>);

/// Information about one embedded subtitle (text) stream in the media file.
#[derive(Debug, Clone)]
pub struct SubtitleStreamInfo {
    /// Index of this stream among the file's subtitle streams (ffmpeg `0:s:N`).
    pub index: i32,
    /// Whether the stream's language tag is English.
    pub english: bool,
    /// Whether the stream is a text-based format (SRT/ASS/VTT...).
    pub is_text: bool,
    /// Whether the stream is a PGS (Blu-ray bitmap) format.
    pub is_pgs: bool,
    /// The stream's language code, if tagged.
    pub language: Option<String>,
}

impl Drop for Video {
    fn drop(&mut self) {
        let inner = match self.0.get_mut() {
            Ok(i) => i,
            Err(_) => {
                // RwLock is poisoned — best-effort cleanup via a write lock
                let mut i = match self.0.write() {
                    Ok(i) => i,
                    Err(_) => return, // hopeless; let the OS reclaim
                };
                let _ = i.source.set_state(gst::State::Null);
                i.alive.store(false, Ordering::SeqCst);
                if let Some(worker) = i.worker.take() {
                    let _ = worker.join();
                }
                return;
            }
        };

        let _ = inner.source.set_state(gst::State::Null);
        inner.alive.store(false, Ordering::SeqCst);
        if let Some(worker) = inner.worker.take()
            && let Err(err) = worker.join()
        {
            match err.downcast_ref::<String>() {
                Some(e) => log::error!("Video thread panicked: {e}"),
                None => log::error!("Video thread panicked with unknown reason"),
            }
        }
    }
}
