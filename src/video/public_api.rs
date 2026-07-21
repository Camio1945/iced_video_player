use super::{Internal, Video};
use crate::Error;
use gstreamer as gst;
use gstreamer::prelude::*;
use std::ops::{Deref, DerefMut};
use std::time::Duration;

impl Video {
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
    pub fn seek(
        &mut self,
        position: impl Into<super::Position>,
        accurate: bool,
    ) -> Result<(), Error> {
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
    /// The video continues playing from its current position without interruption.
    pub fn set_subtitle_url(&mut self, url: &url::Url) -> Result<(), Error> {
        let paused = self.paused();
        let inner = self.get_mut();

        // Save the current playback position before the state transition,
        // since transitioning to Ready resets the position.
        let position = inner
            .source
            .query_position::<gst::ClockTime>()
            .unwrap_or(gst::ClockTime::ZERO);

        inner.source.set_state(gst::State::Ready)?;
        inner.source.set_property("suburi", url.as_str());

        // Go to Paused first to preroll the pipeline.  If we went directly to
        // Playing the pipeline would start playback from 0 before the seek.
        inner.source.set_state(gst::State::Paused)?;
        let _ = inner.source.state(gst::ClockTime::from_seconds(5));

        if position != gst::ClockTime::ZERO {
            inner.seek_to_position_and_wait(position)?;
        }

        if !paused {
            inner.source.set_state(gst::State::Playing)?;
            let _ = inner.source.state(gst::ClockTime::from_seconds(5));
        }

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
}
