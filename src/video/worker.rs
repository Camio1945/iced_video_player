use super::{Frame, Video};
use gstreamer as gst;
use gstreamer_app as gst_app;
use gstreamer_app::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

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

    pub(crate) fn spawn_frame_worker(
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
                match Self::process_single_video_frame(
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

    pub(crate) fn clear_expired_subtitles(
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

    pub(crate) fn pull_subtitle_sample(
        text_sink: Option<&gst_app::AppSink>,
        subtitle_text_ref: &Arc<Mutex<Option<String>>>,
        upload_text_ref: &Arc<AtomicBool>,
        frame_pts: gst::ClockTime,
        clear_subtitles_at: &mut Option<gst::ClockTime>,
    ) -> Result<(), gst::FlowError> {
        let text = text_sink.and_then(|sink| sink.try_pull_sample(gst::ClockTime::from_seconds(0)));
        if let Some(text) = text {
            let buffer = text.buffer().ok_or(gst::FlowError::Error)?;
            let text_duration = buffer.duration().unwrap_or(gst::ClockTime::from_seconds(4));
            let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;
            if let Ok(text) = std::str::from_utf8(map.as_slice()) {
                *subtitle_text_ref
                    .lock()
                    .map_err(|_| gst::FlowError::Error)? = Some(text.to_string());
                upload_text_ref.store(true, Ordering::SeqCst);
                *clear_subtitles_at = Some(frame_pts + text_duration);
            }
        }
        Ok(())
    }
}
