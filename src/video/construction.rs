use super::{Frame, Internal, Video};
use crate::Error;
use gstreamer as gst;
use gstreamer_app as gst_app;
use gstreamer_app::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

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

        let builtin_text_subtitle = Self::auto_select_english_streams(&pipeline);

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
            builtin_text_subtitle,
            frame,
            upload_frame,
            last_frame_time,
            subtitle_text,
            upload_text,
        ))
    }

    /// Disable playbin's text flag before preroll so a sparse bitmap subtitle
    /// (PGS/DVD) linked into the text appsink cannot stall the pipeline.
    /// NOTE: `flags` is a GstPlayFlags GFlags value, not a gint — use the
    /// string form to avoid a "Value type mismatch" panic.
    fn disable_text_flag(pipeline: &gst::Pipeline) {
        pipeline.set_property_from_str("flags", "video+audio");
    }

    /// Re-enable playbin's text flag (safe string-form GFlags assignment).
    fn enable_text_flag(pipeline: &gst::Pipeline) {
        pipeline.set_property_from_str("flags", "video+audio+text");
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

        Self::disable_text_flag(pipeline);

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
        builtin_text_subtitle: bool,
        frame: Arc<Mutex<Frame>>,
        upload_frame: Arc<AtomicBool>,
        last_frame_time: Arc<Mutex<Instant>>,
        subtitle_text: Arc<Mutex<Option<String>>>,
        upload_text: Arc<AtomicBool>,
    ) -> Video {
        #[rustfmt::skip]
        let internal = Internal {
            id, bus: pipeline.bus().unwrap(), source: pipeline, alive,
            worker: Some(worker), width, height, framerate, duration,
            speed: 1.0, sync_av, builtin_text_subtitle,
            frame, upload_frame, last_frame_time,
            looping: false, is_eos: false, restart_stream: false,
            sync_av_avg: 0, sync_av_counter: 0, subtitle_text, upload_text,
        };
        Video(RwLock::new(internal))
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

    /// Auto-select English audio and subtitle streams from the playbin pipeline.
    /// Called after the pipeline has transitioned to Playing state so that
    /// the stream collection is available.
    /// Returns `true` when a text-based subtitle stream was selected and
    /// playbin's text flag was re-enabled.
    fn auto_select_english_streams(pipeline: &gst::Pipeline) -> bool {
        let Some(collection) = Self::find_stream_collection(pipeline) else {
            return false;
        };
        let mut streams = Vec::new();
        for i in 0..collection.len() {
            if let Some(s) = collection.stream(i as u32) {
                let stype = s.stream_type();
                let is_eng = Self::stream_is_english(&s);
                let is_text = s
                    .caps()
                    .map(|c| Self::caps_is_text_subtitle(&c))
                    .unwrap_or(false);
                let is_pgs = s
                    .caps()
                    .map(|c| Self::caps_is_pgs_subtitle(&c))
                    .unwrap_or(false);
                log::info!(
                    "stream #{i}: type={stype:?} english={is_eng} text_subtitle={is_text} pgs={is_pgs}"
                );
                streams.push((i as i32, stype, is_eng, is_text));
            }
        }
        Self::select_english_audio(pipeline, &streams);
        Self::select_english_subtitle(pipeline, &streams)
    }

    fn find_stream_collection(pipeline: &gst::Pipeline) -> Option<gst::StreamCollection> {
        let bus = pipeline.bus()?;
        let mut msg_iter = bus.iter_filtered(&[gst::MessageType::Element]);
        msg_iter.find_map(|msg| match msg.view() {
            gst::MessageView::Element(ev) => ev
                .structure()
                .and_then(|s| s.get::<gst::StreamCollection>("collection").ok()),
            _ => None,
        })
    }

    fn stream_is_english(stream: &gst::Stream) -> bool {
        stream
            .tags()
            .and_then(|tags| tags.get::<gst::tags::LanguageCode>())
            .map(|lang| {
                let l = lang.get().to_lowercase();
                l == "en" || l.starts_with("en-")
            })
            .unwrap_or(false)
    }

    /// Whether the caps describe a text-based subtitle format.
    fn caps_is_text_subtitle(caps: &gst::Caps) -> bool {
        caps.structure(0)
            .map(|s| {
                let n = s.name();
                n.starts_with("text/")
                    || n.starts_with("application/x-subtitle")
                    || n.starts_with("application/x-ssa")
                    || n.starts_with("application/x-ass")
                    || n.starts_with("subtitle/x-")
                    || n.starts_with("application/x-teletext")
            })
            .unwrap_or(false)
    }

    /// Whether the caps describe a PGS (Blu-ray presentation graphics) stream.
    fn caps_is_pgs_subtitle(caps: &gst::Caps) -> bool {
        caps.structure(0)
            .map(|s| s.name() == "subpicture/x-pgs")
            .unwrap_or(false)
    }

    fn select_english_audio(
        pipeline: &gst::Pipeline,
        streams: &[(i32, gst::StreamType, bool, bool)],
    ) {
        let audio: Vec<_> = streams
            .iter()
            .filter(|(_, t, _, _)| *t == gst::StreamType::AUDIO)
            .collect();
        if audio.len() <= 1 {
            return;
        }
        if let Some(&(idx, ..)) = audio.iter().find(|(_, _, eng, _)| *eng) {
            pipeline.set_property("current-audio", idx);
            log::info!("Auto-selected English audio stream #{idx}");
        }
    }

    fn select_english_subtitle(
        pipeline: &gst::Pipeline,
        streams: &[(i32, gst::StreamType, bool, bool)],
    ) -> bool {
        let text_subs: Vec<(i32, bool)> = streams
            .iter()
            .filter(|(_, t, _, is_text)| *t == gst::StreamType::TEXT && *is_text)
            .map(|&(idx, _, eng, _)| (idx, eng))
            .collect();
        if text_subs.is_empty() {
            // Only bitmap (PGS/DVD/DVB) or no subtitle streams.  Keep the
            // text flag disabled so the pipeline is not linked to a sparse
            // subpicture stream that would stall playback.
            return false;
        }
        // Re-enable playbin's text flag, disabled before preroll.
        Self::enable_text_flag(pipeline);
        let chosen = text_subs
            .iter()
            .find(|(_, eng)| *eng)
            .map(|&(idx, _)| idx)
            .unwrap_or_else(|| text_subs[0].0);
        pipeline.set_property("current-text", chosen);
        log::info!("Auto-selected text subtitle stream #{chosen}");
        true
    }
}
