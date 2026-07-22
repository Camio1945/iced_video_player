use super::{Frame, Video};
use gstreamer as gst;
use gstreamer_app as gst_app;
use gstreamer_app::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub(crate) struct SubtitleRefs {
    pub text: Arc<Mutex<Option<String>>>,
    pub upload_text: Arc<AtomicBool>,
    pub image: Arc<Mutex<Option<crate::pgs::PgsImage>>>,
    pub upload_image: Arc<AtomicBool>,
}

impl Video {
    pub(crate) fn try_pull_video_sample(
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn spawn_frame_worker(
        video_sink: gst_app::AppSink,
        text_sink: Option<gst_app::AppSink>,
        pipeline_ref: gst::Pipeline,
        frame_ref: Arc<Mutex<Frame>>,
        upload_frame_ref: Arc<AtomicBool>,
        alive_ref: Arc<AtomicBool>,
        last_frame_time_ref: Arc<Mutex<Instant>>,
        subtitle_text: Arc<Mutex<Option<String>>>,
        upload_text: Arc<AtomicBool>,
        subtitle_image: Arc<Mutex<Option<crate::pgs::PgsImage>>>,
        upload_image: Arc<AtomicBool>,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let sub_refs = SubtitleRefs {
                text: subtitle_text,
                upload_text,
                image: subtitle_image,
                upload_image,
            };
            let mut clear_subtitles_at = None;
            while alive_ref.load(Ordering::Acquire) {
                match Self::process_single_video_frame(
                    &video_sink,
                    text_sink.as_ref(),
                    &pipeline_ref,
                    &frame_ref,
                    &upload_frame_ref,
                    &last_frame_time_ref,
                    &sub_refs,
                    &mut clear_subtitles_at,
                ) {
                    Ok(()) | Err(gst::FlowError::Eos) => {}
                    Err(e) => log::error!("error pulling frame: {e:?}"),
                }
            }
        })
    }

    pub(crate) fn process_single_video_frame(
        video_sink: &gst_app::AppSink,
        text_sink: Option<&gst_app::AppSink>,
        pipeline_ref: &gst::Pipeline,
        frame_ref: &Arc<Mutex<Frame>>,
        upload_frame_ref: &Arc<AtomicBool>,
        last_frame_time_ref: &Arc<Mutex<Instant>>,
        sub_refs: &SubtitleRefs,
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

        Self::clear_expired_subtitles(frame_pts, clear_subtitles_at, sub_refs)?;
        Self::pull_subtitle_sample(text_sink, sub_refs, frame_pts, clear_subtitles_at)?;
        Ok(())
    }

    pub(crate) fn clear_expired_subtitles(
        frame_pts: gst::ClockTime,
        clear_subtitles_at: &mut Option<gst::ClockTime>,
        sub_refs: &SubtitleRefs,
    ) -> Result<(), gst::FlowError> {
        if let Some(at) = clear_subtitles_at
            && frame_pts >= *at
        {
            *sub_refs.text.lock().map_err(|_| gst::FlowError::Error)? = None;
            sub_refs.upload_text.store(true, Ordering::SeqCst);
            *sub_refs.image.lock().map_err(|_| gst::FlowError::Error)? = None;
            sub_refs.upload_image.store(true, Ordering::SeqCst);
            *clear_subtitles_at = None;
        }
        Ok(())
    }

    pub(crate) fn pull_subtitle_sample(
        text_sink: Option<&gst_app::AppSink>,
        sub_refs: &SubtitleRefs,
        frame_pts: gst::ClockTime,
        clear_subtitles_at: &mut Option<gst::ClockTime>,
    ) -> Result<(), gst::FlowError> {
        let sample =
            text_sink.and_then(|sink| sink.try_pull_sample(gst::ClockTime::from_seconds(0)));
        let Some(sample) = sample else {
            return Ok(());
        };
        let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
        let duration = buffer.duration().unwrap_or(gst::ClockTime::from_seconds(4));
        let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;
        let data = map.as_slice();

        if let Ok(text) = std::str::from_utf8(data) {
            *sub_refs.text.lock().map_err(|_| gst::FlowError::Error)? = Some(text.to_string());
            sub_refs.upload_text.store(true, Ordering::SeqCst);
            // A text subtitle supersedes any bitmap subtitle (e.g. after an
            // external SRT replaced an embedded PGS stream).
            *sub_refs.image.lock().map_err(|_| gst::FlowError::Error)? = None;
            sub_refs.upload_image.store(true, Ordering::SeqCst);
            *clear_subtitles_at = Some(frame_pts + duration);
        } else if let Some(img) = crate::pgs::decode(data) {
            *sub_refs.image.lock().map_err(|_| gst::FlowError::Error)? = Some(img);
            sub_refs.upload_image.store(true, Ordering::SeqCst);
            *sub_refs.text.lock().map_err(|_| gst::FlowError::Error)? = None;
            sub_refs.upload_text.store(true, Ordering::SeqCst);
            *clear_subtitles_at = Some(frame_pts + duration);
        }
        Ok(())
    }
}
