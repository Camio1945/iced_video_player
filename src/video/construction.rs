use super::{Frame, Internal, SubtitleStreamInfo, Video};
use crate::Error;
use gstreamer as gst;
use gstreamer_app as gst_app;
use gstreamer_app::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

struct SubtitleShared {
    text: Arc<Mutex<Option<String>>>,
    upload_text: Arc<AtomicBool>,
    image: Arc<Mutex<Option<crate::pgs::PgsImage>>>,
    upload_image: Arc<AtomicBool>,
}

struct WorkerSetup {
    alive: Arc<AtomicBool>,
    worker: std::thread::JoinHandle<()>,
    frame: Arc<Mutex<Frame>>,
    upload_frame: Arc<AtomicBool>,
    last_frame_time: Arc<Mutex<Instant>>,
    subtitle: SubtitleShared,
}

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
        let pad = video_sink.pads().first().cloned().ok_or(Error::Caps)?;
        let pad = pad.dynamic_cast::<gst::GhostPad>().map_err(|_| Error::Cast)?;
        let bin = pad
            .parent_element()
            .ok_or(Error::Cast)?
            .downcast::<gst::Bin>()
            .map_err(|_| Error::Cast)?;
        let video_sink = bin.by_name("iced_video").ok_or(Error::AppSink("iced_video".into()))?;
        let video_sink = video_sink.downcast::<gst_app::AppSink>().map_err(|_| Error::Cast)?;

        let text_sink: gst::Element = pipeline.property("text-sink");
        let text_sink = text_sink.downcast::<gst_app::AppSink>().map_err(|_| Error::Cast)?;

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

        let pad = video_sink.pads().first().cloned().ok_or(Error::Caps)?;
        let props = Self::extract_pipeline_properties(&pipeline, &pad)?;
        let (builtin_text_subtitle, subtitle_streams) =
            Self::auto_select_english_streams(&pipeline);
        let setup = Self::setup_state_and_worker(video_sink, text_sink, &pipeline);

        Self::make_video_internal(
            id,
            pipeline,
            props,
            builtin_text_subtitle,
            subtitle_streams,
            setup,
        )
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
    ) -> WorkerSetup {
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
        let subtitle = Self::create_shared_subtitle_state();
        let worker = Self::spawn_frame_worker(
            video_sink,
            text_sink,
            pipeline.clone(),
            frame_ref,
            upload_frame_ref,
            alive_ref,
            last_frame_time_ref,
            Arc::clone(&subtitle.text),
            Arc::clone(&subtitle.upload_text),
            Arc::clone(&subtitle.image),
            Arc::clone(&subtitle.upload_image),
        );
        WorkerSetup {
            alive,
            worker,
            frame,
            upload_frame,
            last_frame_time,
            subtitle,
        }
    }

    fn make_video_internal(
        id: u64,
        pipeline: gst::Pipeline,
        props: (i32, i32, f64, Duration, bool),
        builtin_text_subtitle: bool,
        subtitle_streams: Vec<SubtitleStreamInfo>,
        setup: WorkerSetup,
    ) -> Result<Video, Error> {
        let (width, height, framerate, duration, sync_av) = props;
        let bus = pipeline.bus().ok_or(Error::Bus)?;
        #[rustfmt::skip]
        let internal = Internal {
            id, bus, source: pipeline,
            alive: setup.alive, worker: Some(setup.worker),
            width, height, framerate, duration,
            speed: 1.0, sync_av, builtin_text_subtitle, subtitle_streams,
            frame: setup.frame, upload_frame: setup.upload_frame,
            last_frame_time: setup.last_frame_time,
            looping: false, is_eos: false, restart_stream: false,
            sync_av_avg: 0, sync_av_counter: 0,
            subtitle_text: setup.subtitle.text,
            upload_text: setup.subtitle.upload_text,
            subtitle_image: setup.subtitle.image,
            upload_image: setup.subtitle.upload_image,
        };
        Ok(Video(RwLock::new(internal)))
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

    fn create_shared_subtitle_state() -> SubtitleShared {
        SubtitleShared {
            text: Arc::new(Mutex::new(None)),
            upload_text: Arc::new(AtomicBool::new(false)),
            image: Arc::new(Mutex::new(None)),
            upload_image: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Auto-select English audio and subtitle streams from the playbin pipeline.
    /// Called after the pipeline has transitioned to Playing state so that
    /// stream pads and tags are available.
    /// Returns whether a subtitle stream was selected, plus the probed list
    /// of all embedded subtitle streams.
    ///
    /// Uses the playbin2 API (`n-text`, `get-text-pad`, `get-text-tags`)
    /// because `playbin` in GStreamer 1.x is playbin2, which does not post
    /// stream-collection messages.
    fn auto_select_english_streams(pipeline: &gst::Pipeline) -> (bool, Vec<SubtitleStreamInfo>) {
        Self::select_english_audio(pipeline);
        let subs = Self::probe_all_subtitle_streams(pipeline);
        let selected = Self::select_english_subtitle(pipeline, &subs);
        (selected, subs)
    }

    fn select_english_audio(pipeline: &gst::Pipeline) {
        let n_audio: i32 = pipeline.property("n-audio");
        if n_audio <= 1 {
            return;
        }
        for i in 0..n_audio {
            let is_eng = Self::stream_language(pipeline, "get-audio-tags", i)
                .map(|l| Self::is_english(&l))
                .unwrap_or(false);
            if is_eng {
                pipeline.set_property("current-audio", i);
                log::info!("Auto-selected English audio stream #{i}");
                return;
            }
        }
    }

    fn select_english_subtitle(pipeline: &gst::Pipeline, subs: &[SubtitleStreamInfo]) -> bool {
        // Prefer text-based English, then PGS English, then any text, then any PGS.
        let chosen = subs
            .iter()
            .find(|s| s.english && s.is_text)
            .or_else(|| subs.iter().find(|s| s.english && s.is_pgs))
            .or_else(|| subs.iter().find(|s| s.is_text))
            .or_else(|| subs.iter().find(|s| s.is_pgs));
        let Some(sub) = chosen else {
            return false;
        };
        pipeline.set_property("current-text", sub.index);
        let kind = if sub.is_pgs { "PGS" } else { "text" };
        log::info!(
            "Auto-selected {kind} subtitle stream #{} (english={})",
            sub.index,
            sub.english
        );
        true
    }

    fn probe_all_subtitle_streams(pipeline: &gst::Pipeline) -> Vec<SubtitleStreamInfo> {
        let n_text: i32 = pipeline.property("n-text");
        (0..n_text)
            .map(|i| Self::probe_subtitle_stream(pipeline, i))
            .collect()
    }

    fn probe_subtitle_stream(pipeline: &gst::Pipeline, index: i32) -> SubtitleStreamInfo {
        let pad: Option<gst::Pad> = pipeline.emit_by_name("get-text-pad", &[&index]);
        let caps = pad.and_then(|p| p.current_caps());
        let is_text = caps
            .as_ref()
            .map(|c| Self::caps_is_text_subtitle(c))
            .unwrap_or(false);
        let is_pgs = caps
            .as_ref()
            .map(|c| Self::caps_is_pgs_subtitle(c))
            .unwrap_or(false);
        let language = Self::stream_language(pipeline, "get-text-tags", index);
        let english = language.as_deref().map(Self::is_english).unwrap_or(false);
        log::info!(
            "subtitle stream #{index}: english={english} text={is_text} pgs={is_pgs} caps={caps:?}"
        );
        SubtitleStreamInfo {
            index,
            english,
            is_text,
            is_pgs,
            language,
        }
    }

    fn stream_language(pipeline: &gst::Pipeline, signal: &str, index: i32) -> Option<String> {
        let tags: Option<gst::TagList> = pipeline.emit_by_name(signal, &[&index]);
        tags.and_then(|t| {
            t.get::<gst::tags::LanguageCode>()
                .map(|l| l.get().to_string())
        })
    }

    fn is_english(lang: &str) -> bool {
        let l = lang.to_lowercase();
        l == "en" || l == "eng" || l.starts_with("en-") || l.starts_with("eng-")
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
}
