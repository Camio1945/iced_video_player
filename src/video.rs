use crate::Error;
use gstreamer as gst;
use gstreamer_app as gst_app;
use gstreamer_app::prelude::*;
use gstreamer_video::VideoMeta;
use iced::widget::image as img;
use std::num::NonZeroU8;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
                .meta::<VideoMeta>()
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
}

impl Internal {
    pub(crate) fn seek(&self, position: impl Into<Position>, accurate: bool) -> Result<(), Error> {
        let position = position.into();

        // gstreamer complains if the start & end value types aren't the same
        match &position {
            Position::Time(_) => self.source.seek(
                self.speed,
                gst::SeekFlags::FLUSH
                    | if accurate {
                        gst::SeekFlags::ACCURATE
                    } else {
                        gst::SeekFlags::empty()
                    },
                gst::SeekType::Set,
                gst::GenericFormattedValue::from(position),
                gst::SeekType::Set,
                gst::ClockTime::NONE,
            )?,
            Position::Frame(_) => self.source.seek(
                self.speed,
                gst::SeekFlags::FLUSH
                    | if accurate {
                        gst::SeekFlags::ACCURATE
                    } else {
                        gst::SeekFlags::empty()
                    },
                gst::SeekType::Set,
                gst::GenericFormattedValue::from(position),
                gst::SeekType::Set,
                gst::format::Default::NONE,
            )?,
        };

        *self.subtitle_text.lock().expect("lock subtitle_text") = None;
        self.upload_text.store(true, Ordering::SeqCst);

        Ok(())
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
            self.sync_av_counter += 1;
            self.sync_av_avg = self.sync_av_avg * (self.sync_av_counter - 1) / self.sync_av_counter
                + offset.as_nanos() as u64 / self.sync_av_counter;
            if self.sync_av_counter.is_multiple_of(128) {
                self.source
                    .set_property("av-offset", -(self.sync_av_avg as i64));
            }
        }
    }
}

/// A multimedia video loaded from a URI (e.g., a local file path or HTTP stream).
#[derive(Debug)]
pub struct Video(pub(crate) RwLock<Internal>);

impl Drop for Video {
    fn drop(&mut self) {
        let inner = self.0.get_mut().expect("failed to lock");

        inner
            .source
            .set_state(gst::State::Null)
            .expect("failed to set state");

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

type SharedFrameState = (
    Arc<AtomicBool>,
    std::thread::JoinHandle<()>,
    Arc<Mutex<Frame>>,
    Arc<AtomicBool>,
    Arc<Mutex<Instant>>,
    Arc<Mutex<Option<String>>>,
    Arc<AtomicBool>,
);

impl Video {
    /// Create a new video player from a given video which loads from `uri`.
    /// Note that live sources will report the duration to be zero.
    pub fn new(uri: &url::Url) -> Result<Self, Error> {
        gst::init()?;

        let pipeline = format!(
            "playbin uri=\"{}\" text-sink=\"appsink name=iced_text sync=true drop=true\" video-sink=\"videoscale ! videoconvert ! appsink name=iced_video drop=true caps=video/x-raw,format=NV12,pixel-aspect-ratio=1/1\"",
            uri.as_str()
        );
        let pipeline = gst::parse::launch(pipeline.as_ref())?
            .downcast::<gst::Pipeline>()
            .map_err(|_| Error::Cast)?;

        let video_sink: gst::Element = pipeline.property("video-sink");
        let pad = video_sink.pads().first().cloned().unwrap();
        let pad = pad.dynamic_cast::<gst::GhostPad>().unwrap();
        let bin = pad
            .parent_element()
            .unwrap()
            .downcast::<gst::Bin>()
            .unwrap();
        let video_sink = bin.by_name("iced_video").unwrap();
        let video_sink = video_sink.downcast::<gst_app::AppSink>().unwrap();

        let text_sink: gst::Element = pipeline.property("text-sink");
        let text_sink = text_sink.downcast::<gst_app::AppSink>().unwrap();

        Self::from_gst_pipeline(pipeline, video_sink, Some(text_sink))
    }

    /// Creates a new video based on an existing GStreamer pipeline and appsink.
    /// Expects an `appsink` plugin with `caps=video/x-raw,format=NV12`.
    ///
    /// An optional `text_sink` can be provided, which enables subtitle messages
    /// to be emitted.
    ///
    /// **Note:** Many functions of [`Video`] assume a `playbin` pipeline.
    /// Non-`playbin` pipelines given here may not have full functionality.
    pub fn from_gst_pipeline(
        pipeline: gst::Pipeline,
        video_sink: gst_app::AppSink,
        text_sink: Option<gst_app::AppSink>,
    ) -> Result<Self, Error> {
        gst::init()?;
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);

        let pad = video_sink.pads().first().cloned().unwrap();
        let (width, height, framerate, duration, sync_av) =
            Self::extract_pipeline_properties(&pipeline, &pad)?;

        let (alive, worker, frame, upload_frame, last_frame_time, subtitle_text, upload_text) =
            Self::setup_state_and_worker(video_sink, text_sink, &pipeline);

        Ok(Self::make_video_internal(
            id,
            pipeline,
            alive,
            worker,
            width,
            height,
            framerate,
            duration,
            sync_av,
            frame,
            upload_frame,
            last_frame_time,
            subtitle_text,
            upload_text,
        ))
    }

    fn extract_pipeline_properties(
        pipeline: &gst::Pipeline,
        pad: &gst::Pad,
    ) -> Result<(i32, i32, f64, Duration, bool), Error> {
        macro_rules! cleanup {
            ($expr:expr) => {
                $expr.map_err(|e| {
                    let _ = pipeline.set_state(gst::State::Null);
                    e
                })
            };
        }

        cleanup!(pipeline.set_state(gst::State::Playing))?;
        cleanup!(pipeline.state(gst::ClockTime::from_seconds(5)).0)?;

        let caps = cleanup!(pad.current_caps().ok_or(Error::Caps))?;
        let s = cleanup!(caps.structure(0).ok_or(Error::Caps))?;
        let width = cleanup!(s.get::<i32>("width").map_err(|_| Error::Caps))?;
        let height = cleanup!(s.get::<i32>("height").map_err(|_| Error::Caps))?;
        let framerate = cleanup!(s.get::<gst::Fraction>("framerate").map_err(|_| Error::Caps))?;
        let framerate = Self::validate_framerate(framerate, pipeline)?;

        let duration = Duration::from_nanos(
            pipeline
                .query_duration::<gst::ClockTime>()
                .map(|duration| duration.nseconds())
                .unwrap_or(0),
        );
        let sync_av = pipeline.has_property("av-offset", None);

        Ok((width, height, framerate, duration, sync_av))
    }

    fn validate_framerate(
        framerate: gst::Fraction,
        pipeline: &gst::Pipeline,
    ) -> Result<f64, Error> {
        let f = framerate.numer() as f64 / framerate.denom() as f64;
        if f.is_nan() || f.is_infinite() || f < 0.0 || f.abs() < f64::EPSILON {
            let _ = pipeline.set_state(gst::State::Null);
            return Err(Error::Framerate(f));
        }
        Ok(f)
    }

    fn setup_state_and_worker(
        video_sink: gst_app::AppSink,
        text_sink: Option<gst_app::AppSink>,
        pipeline: &gst::Pipeline,
    ) -> SharedFrameState {
        let (
            frame,
            upload_frame,
            alive,
            last_frame_time,
            frame_ref,
            upload_frame_ref,
            alive_ref,
            last_frame_time_ref,
        ) = Self::create_shared_video_state();
        let (subtitle_text, upload_text, subtitle_text_ref, upload_text_ref) =
            Self::create_shared_subtitle_state();
        let worker = Self::spawn_frame_worker(
            video_sink,
            text_sink,
            pipeline.clone(),
            frame_ref,
            upload_frame_ref,
            alive_ref,
            last_frame_time_ref,
            subtitle_text_ref,
            upload_text_ref,
        );
        (
            alive,
            worker,
            frame,
            upload_frame,
            last_frame_time,
            subtitle_text,
            upload_text,
        )
    }

    fn make_video_internal(
        id: u64,
        pipeline: gst::Pipeline,
        alive: Arc<AtomicBool>,
        worker: std::thread::JoinHandle<()>,
        width: i32,
        height: i32,
        framerate: f64,
        duration: Duration,
        sync_av: bool,
        frame: Arc<Mutex<Frame>>,
        upload_frame: Arc<AtomicBool>,
        last_frame_time: Arc<Mutex<Instant>>,
        subtitle_text: Arc<Mutex<Option<String>>>,
        upload_text: Arc<AtomicBool>,
    ) -> Video {
        Self::construct_video_from_internal(Self::build_internal_fields(
            id,
            pipeline,
            alive,
            worker,
            width,
            height,
            framerate,
            duration,
            sync_av,
            frame,
            upload_frame,
            last_frame_time,
            subtitle_text,
            upload_text,
        ))
    }

    fn build_internal_fields(
        id: u64,
        pipeline: gst::Pipeline,
        alive: Arc<AtomicBool>,
        worker: std::thread::JoinHandle<()>,
        width: i32,
        height: i32,
        framerate: f64,
        duration: Duration,
        sync_av: bool,
        frame: Arc<Mutex<Frame>>,
        upload_frame: Arc<AtomicBool>,
        last_frame_time: Arc<Mutex<Instant>>,
        subtitle_text: Arc<Mutex<Option<String>>>,
        upload_text: Arc<AtomicBool>,
    ) -> std::sync::RwLock<Internal> {
        let bus = pipeline.bus().unwrap();
        RwLock::new(Self::assemble_internal(
            id,
            bus,
            pipeline,
            alive,
            worker,
            width,
            height,
            framerate,
            duration,
            sync_av,
            frame,
            upload_frame,
            last_frame_time,
            subtitle_text,
            upload_text,
        ))
    }

    fn assemble_internal(
        id: u64,
        bus: gst::Bus,
        pipeline: gst::Pipeline,
        alive: Arc<AtomicBool>,
        worker: std::thread::JoinHandle<()>,
        width: i32,
        height: i32,
        framerate: f64,
        duration: Duration,
        sync_av: bool,
        frame: Arc<Mutex<Frame>>,
        upload_frame: Arc<AtomicBool>,
        last_frame_time: Arc<Mutex<Instant>>,
        subtitle_text: Arc<Mutex<Option<String>>>,
        upload_text: Arc<AtomicBool>,
    ) -> Internal {
        Internal {
            id,
            bus,
            source: pipeline,
            alive,
            worker: Some(worker),
            width,
            height,
            framerate,
            duration,
            speed: 1.0,
            sync_av,
            frame,
            upload_frame,
            last_frame_time,
            looping: false,
            is_eos: false,
            restart_stream: false,
            sync_av_avg: 0,
            sync_av_counter: 0,
            subtitle_text,
            upload_text,
        }
    }

    fn construct_video_from_internal(rw: std::sync::RwLock<Internal>) -> Video {
        Video(rw)
    }

    fn try_pull_video_sample(
        video_sink: &gst_app::AppSink,
        pipeline_ref: &gst::Pipeline,
    ) -> Result<gst::Sample, gst::FlowError> {
        if pipeline_ref.state(gst::ClockTime::ZERO).1 != gst::State::Playing {
            video_sink
                .try_pull_preroll(gst::ClockTime::from_mseconds(16))
                .ok_or(gst::FlowError::Eos)
        } else {
            video_sink
                .try_pull_sample(gst::ClockTime::from_mseconds(16))
                .ok_or(gst::FlowError::Eos)
        }
    }

    fn create_shared_video_state() -> (
        Arc<Mutex<Frame>>,
        Arc<AtomicBool>,
        Arc<AtomicBool>,
        Arc<Mutex<Instant>>,
        Arc<Mutex<Frame>>,
        Arc<AtomicBool>,
        Arc<AtomicBool>,
        Arc<Mutex<Instant>>,
    ) {
        let frame = Arc::new(Mutex::new(Frame::empty()));
        let upload_frame = Arc::new(AtomicBool::new(false));
        let alive = Arc::new(AtomicBool::new(true));
        let last_frame_time = Arc::new(Mutex::new(Instant::now()));

        let frame_ref = Arc::clone(&frame);
        let upload_frame_ref = Arc::clone(&upload_frame);
        let alive_ref = Arc::clone(&alive);
        let last_frame_time_ref = Arc::clone(&last_frame_time);

        (
            frame,
            upload_frame,
            alive,
            last_frame_time,
            frame_ref,
            upload_frame_ref,
            alive_ref,
            last_frame_time_ref,
        )
    }

    fn create_shared_subtitle_state() -> (
        Arc<Mutex<Option<String>>>,
        Arc<AtomicBool>,
        Arc<Mutex<Option<String>>>,
        Arc<AtomicBool>,
    ) {
        let subtitle_text = Arc::new(Mutex::new(None));
        let upload_text = Arc::new(AtomicBool::new(false));
        let subtitle_text_ref = Arc::clone(&subtitle_text);
        let upload_text_ref = Arc::clone(&upload_text);
        (
            subtitle_text,
            upload_text,
            subtitle_text_ref,
            upload_text_ref,
        )
    }

    fn spawn_frame_worker(
        video_sink: gst_app::AppSink,
        text_sink: Option<gst_app::AppSink>,
        pipeline_ref: gst::Pipeline,
        frame_ref: Arc<Mutex<Frame>>,
        upload_frame_ref: Arc<AtomicBool>,
        alive_ref: Arc<AtomicBool>,
        last_frame_time_ref: Arc<Mutex<Instant>>,
        subtitle_text_ref: Arc<Mutex<Option<String>>>,
        upload_text_ref: Arc<AtomicBool>,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let mut clear_subtitles_at = None;
            while alive_ref.load(Ordering::Acquire) {
                if let Err(e) = Self::process_single_video_frame(
                    &video_sink,
                    text_sink.as_ref(),
                    &pipeline_ref,
                    &frame_ref,
                    &upload_frame_ref,
                    &last_frame_time_ref,
                    &subtitle_text_ref,
                    &upload_text_ref,
                    &mut clear_subtitles_at,
                ) {
                    log::error!("error pulling frame: {e:?}");
                }
            }
        })
    }

    fn process_single_video_frame(
        video_sink: &gst_app::AppSink,
        text_sink: Option<&gst_app::AppSink>,
        pipeline_ref: &gst::Pipeline,
        frame_ref: &Arc<Mutex<Frame>>,
        upload_frame_ref: &Arc<AtomicBool>,
        last_frame_time_ref: &Arc<Mutex<Instant>>,
        subtitle_text_ref: &Arc<Mutex<Option<String>>>,
        upload_text_ref: &Arc<AtomicBool>,
        clear_subtitles_at: &mut Option<gst::ClockTime>,
    ) -> Result<(), gst::FlowError> {
        let sample = Self::try_pull_video_sample(video_sink, pipeline_ref)?;
        *last_frame_time_ref
            .lock()
            .map_err(|_| gst::FlowError::Error)? = Instant::now();

        let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
        let frame_pts = buffer.pts().ok_or(gst::FlowError::Error)?;
        {
            let mut frame_guard = frame_ref.lock().map_err(|_| gst::FlowError::Error)?;
            *frame_guard = Frame(sample);
        }
        upload_frame_ref.swap(true, Ordering::SeqCst);

        Self::clear_expired_subtitles(
            frame_pts,
            clear_subtitles_at,
            subtitle_text_ref,
            upload_text_ref,
        )?;
        Self::pull_subtitle_sample(
            text_sink,
            subtitle_text_ref,
            upload_text_ref,
            frame_pts,
            clear_subtitles_at,
        )?;
        Ok(())
    }

    fn clear_expired_subtitles(
        frame_pts: gst::ClockTime,
        clear_subtitles_at: &mut Option<gst::ClockTime>,
        subtitle_text_ref: &Arc<Mutex<Option<String>>>,
        upload_text_ref: &Arc<AtomicBool>,
    ) -> Result<(), gst::FlowError> {
        if let Some(at) = clear_subtitles_at
            && frame_pts >= *at
        {
            *subtitle_text_ref
                .lock()
                .map_err(|_| gst::FlowError::Error)? = None;
            upload_text_ref.store(true, Ordering::SeqCst);
            *clear_subtitles_at = None;
        }
        Ok(())
    }

    fn pull_subtitle_sample(
        text_sink: Option<&gst_app::AppSink>,
        subtitle_text_ref: &Arc<Mutex<Option<String>>>,
        upload_text_ref: &Arc<AtomicBool>,
        frame_pts: gst::ClockTime,
        clear_subtitles_at: &mut Option<gst::ClockTime>,
    ) -> Result<(), gst::FlowError> {
        let text = text_sink.and_then(|sink| sink.try_pull_sample(gst::ClockTime::from_seconds(0)));
        if let Some(text) = text {
            let text = text.buffer().ok_or(gst::FlowError::Error)?;
            let text_duration = text.duration().ok_or(gst::FlowError::Error)?;
            let map = text.map_readable().map_err(|_| gst::FlowError::Error)?;
            let text = std::str::from_utf8(map.as_slice())
                .map_err(|_| gst::FlowError::Error)?
                .to_string();
            *subtitle_text_ref
                .lock()
                .map_err(|_| gst::FlowError::Error)? = Some(text);
            upload_text_ref.store(true, Ordering::SeqCst);
            *clear_subtitles_at = Some(frame_pts + text_duration);
        }
        Ok(())
    }

    pub(crate) fn read(&self) -> impl Deref<Target = Internal> + '_ {
        self.0.read().expect("lock")
    }

    pub(crate) fn write(&self) -> impl DerefMut<Target = Internal> + '_ {
        self.0.write().expect("lock")
    }

    pub(crate) fn get_mut(&mut self) -> impl DerefMut<Target = Internal> + '_ {
        self.0.get_mut().expect("lock")
    }

    /// Get the size/resolution of the video as `(width, height)`.
    pub fn size(&self) -> (i32, i32) {
        (self.read().width, self.read().height)
    }

    /// Get the framerate of the video as frames per second.
    pub fn framerate(&self) -> f64 {
        self.read().framerate
    }

    /// Set the volume multiplier of the audio.
    /// `0.0` = 0% volume, `1.0` = 100% volume.
    ///
    /// This uses a linear scale, for example `0.5` is perceived as half as loud.
    pub fn set_volume(&mut self, volume: f64) {
        self.get_mut().source.set_property("volume", volume);
        self.set_muted(self.muted()); // for some reason gstreamer unmutes when changing volume?
    }

    /// Get the volume multiplier of the audio.
    pub fn volume(&self) -> f64 {
        self.read().source.property("volume")
    }

    /// Set if the audio is muted or not, without changing the volume.
    pub fn set_muted(&mut self, muted: bool) {
        self.get_mut().source.set_property("mute", muted);
    }

    /// Get if the audio is muted or not.
    pub fn muted(&self) -> bool {
        self.read().source.property("mute")
    }

    /// Get if the stream ended or not.
    pub fn eos(&self) -> bool {
        self.read().is_eos
    }

    /// Get if the media will loop or not.
    pub fn looping(&self) -> bool {
        self.read().looping
    }

    /// Set if the media will loop or not.
    pub fn set_looping(&mut self, looping: bool) {
        self.get_mut().looping = looping;
    }

    /// Set if the media is paused or not.
    pub fn set_paused(&mut self, paused: bool) {
        self.get_mut().set_paused(paused)
    }

    /// Get if the media is paused or not.
    pub fn paused(&self) -> bool {
        self.read().paused()
    }

    /// Jumps to a specific position in the media.
    /// Passing `true` to the `accurate` parameter will result in more accurate seeking,
    /// however, it is also slower. For most seeks (e.g., scrubbing) this is not needed.
    pub fn seek(&mut self, position: impl Into<Position>, accurate: bool) -> Result<(), Error> {
        self.get_mut().seek(position, accurate)
    }

    /// Steps forward exactly one frame in playback.
    /// This can be especially useful while the video is paused to make pipeline changes visible, without resuming playback.
    pub fn step_one_frame(&mut self) {
        self.get_mut().source.send_event(gst::event::Step::new(
            gst::GenericFormattedValue::Buffers(Some(gst::format::Buffers::from_u64(1))),
            1.0,
            true,
            false,
        ));
    }

    /// Set the playback speed of the media.
    /// The default speed is `1.0`.
    pub fn set_speed(&mut self, speed: f64) -> Result<(), Error> {
        self.get_mut().set_speed(speed)
    }

    /// Get the current playback speed.
    pub fn speed(&self) -> f64 {
        self.read().speed
    }

    /// Get the current playback position in time.
    pub fn position(&self) -> Duration {
        Duration::from_nanos(
            self.read()
                .source
                .query_position::<gst::ClockTime>()
                .map_or(0, |pos| pos.nseconds()),
        )
    }

    /// Get the media duration.
    pub fn duration(&self) -> Duration {
        self.read().duration
    }

    /// Restarts a stream; seeks to the first frame and unpauses, sets the `eos` flag to false.
    pub fn restart_stream(&mut self) -> Result<(), Error> {
        self.get_mut().restart_stream()
    }

    /// Set the subtitle URL to display.
    pub fn set_subtitle_url(&mut self, url: &url::Url) -> Result<(), Error> {
        let paused = self.paused();
        let mut inner = self.get_mut();
        inner.source.set_state(gst::State::Ready)?;
        inner.source.set_property("suburi", url.as_str());
        inner.set_paused(paused);
        Ok(())
    }

    /// Get the current subtitle URL.
    pub fn subtitle_url(&self) -> Option<url::Url> {
        url::Url::parse(
            &self
                .read()
                .source
                .property::<Option<String>>("current-suburi")?,
        )
        .ok()
    }

    /// Get the underlying GStreamer pipeline.
    pub fn pipeline(&self) -> gst::Pipeline {
        self.read().source.clone()
    }

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
        I: IntoIterator<Item = Position>,
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
        pos: Position,
        downscale: u32,
    ) -> Result<img::Handle, Error> {
        inner.seek(pos, true)?;
        inner.upload_frame.store(false, Ordering::SeqCst);
        while !inner.upload_frame.load(Ordering::SeqCst) {
            std::hint::spin_loop();
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
