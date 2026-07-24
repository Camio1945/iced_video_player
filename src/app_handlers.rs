use crate::app_state::{App, Message, SidebarTab, VideoState};
use iced::Task;
use std::time::Duration;

impl App {
    pub fn handle_toggle_pause(&mut self) -> Task<Message> {
        if let VideoState::Ready(ref mut v) = self.video {
            let p = v.paused();
            v.set_paused(!p);
        }
        Task::none()
    }

    pub fn handle_seek(&mut self, secs: f64) -> Task<Message> {
        self.dragging = true;
        self.position = secs;
        if let VideoState::Ready(ref mut v) = self.video {
            v.set_paused(true);
        }
        Task::none()
    }

    pub fn handle_seek_release(&mut self) -> Task<Message> {
        self.dragging = false;
        if let VideoState::Ready(ref mut v) = self.video {
            let _ = v.seek(Duration::from_secs_f64(self.position), false);
            v.set_paused(false);
        }
        Task::none()
    }

    pub fn handle_skip_back(&mut self, secs: i64) -> Task<Message> {
        if let VideoState::Ready(ref mut v) = self.video {
            let n = (v.position().as_secs_f64() - secs as f64).max(0.0);
            self.position = n;
            let _ = v.seek(Duration::from_secs_f64(n), false);
        }
        Task::none()
    }

    pub fn handle_skip_forward(&mut self, secs: i64) -> Task<Message> {
        if let VideoState::Ready(ref mut v) = self.video {
            let dur = v.duration().as_secs_f64();
            let n = (v.position().as_secs_f64() + secs as f64).min(dur);
            self.position = n;
            let _ = v.seek(Duration::from_secs_f64(n), false);
        }
        Task::none()
    }

    pub fn handle_frame_step_forward(&mut self) -> Task<Message> {
        if let VideoState::Ready(ref mut v) = self.video
            && v.paused()
        {
            v.step_one_frame();
        }
        Task::none()
    }

    pub fn handle_frame_step_backward(&mut self) -> Task<Message> {
        if let VideoState::Ready(ref mut v) = self.video {
            let fps = v.framerate();
            let n = (v.position().as_secs_f64() - 1.0 / fps).max(0.0);
            self.position = n;
            let _ = v.seek(Duration::from_secs_f64(n), true);
            v.set_paused(true);
        }
        Task::none()
    }

    pub fn handle_new_frame(&mut self) -> Task<Message> {
        // Apply a deferred resume-seek on the first rendered frame, after the
        // pipeline has finished prerolling. Doing it here (rather than right
        // after `Video::new`) avoids a race with synchronous subtitle loading,
        // which queries and resets the pipeline position.
        if let Some(pos) = self.pending_resume.take() {
            if let VideoState::Ready(ref mut v) = self.video {
                let _ = v.seek(Duration::from_secs_f64(pos), true);
                self.position = pos;
            }
            return Task::none();
        }
        if !self.dragging {
            self.position = self.current_pos();
        }
        Task::none()
    }

    /// Log a playback error reported by the video pipeline.
    pub fn handle_playback_error(&mut self, err: String) -> Task<Message> {
        eprintln!("Playback error: {}", err);
        Task::none()
    }

    pub fn handle_search_word(&mut self, word: String) -> Task<Message> {
        self.active_tab = SidebarTab::Dictionary;
        self.dict_word = word;
        self.dict_loading = false;
        self.dict_chinese.clear();
        self.dict_phonetic.clear();
        self.dict_sections.clear();
        self.dict_examples.clear();
        self.dict_error = None;
        // No API lookup – the webview handles the Youdao query directly.
        Task::none()
    }

    pub fn handle_dictionary_result(&mut self, result: crate::dict::DictResult) -> Task<Message> {
        // Kept for compatibility; the webview path bypasses this handler.
        self.dict_word = result.word;
        self.dict_chinese = result.chinese;
        self.dict_phonetic = result.phonetic;
        self.dict_sections = result.sections;
        self.dict_examples = result.examples;
        self.dict_error = result.error;
        self.dict_loading = false;
        Task::none()
    }

    pub fn handle_close_dictionary(&mut self) -> Task<Message> {
        self.clear_dictionary();
        // The webview popup will be hidden on the next tick (the webview
        // itself stays alive so future searches are instant).
        Task::none()
    }

    /// Periodic tick – drives the dictionary webview state machine.
    pub fn handle_tick(&mut self) -> Task<Message> {
        let is_dict_active = self.active_tab == SidebarTab::Dictionary;
        let word = if self.dict_word.is_empty() {
            None
        } else {
            Some(self.dict_word.as_str())
        };

        let title = match &self.current_file_path {
            Some(p) => {
                let name = std::path::Path::new(p)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(p);
                format!("{name} - Video Player")
            }
            None => "Video Player".to_string(),
        };

        crate::dict_webview::tick(is_dict_active, word, &title);
        Task::none()
    }

    pub fn handle_toggle_loop(&mut self) -> Task<Message> {
        if let VideoState::Ready(ref mut v) = self.video {
            v.set_looping(!v.looping());
            self.looping = v.looping();
        }
        Task::none()
    }

    pub fn handle_toggle_mute(&mut self) -> Task<Message> {
        self.muted = !self.muted;
        if let VideoState::Ready(ref mut v) = self.video {
            v.set_muted(self.muted);
        }
        Task::none()
    }

    pub fn handle_set_volume(&mut self, vol: f64) -> Task<Message> {
        self.volume = vol;
        if let VideoState::Ready(ref mut v) = self.video {
            v.set_volume(vol);
        }
        Task::none()
    }

    pub fn handle_adjust_volume(&mut self, delta: f64) -> Task<Message> {
        let new_vol = (self.volume + delta * 0.1).clamp(0.0, 2.0);
        self.handle_set_volume(new_vol)
    }

    pub fn handle_set_speed(&mut self, s: f64) -> Task<Message> {
        self.speed = s;
        if let VideoState::Ready(ref mut v) = self.video {
            let _ = v.set_speed(s);
        }
        Task::none()
    }

    pub fn handle_toggle_fullscreen(&mut self) -> Task<Message> {
        self.fullscreen = !self.fullscreen;
        let mode = if self.fullscreen {
            iced::window::Mode::Fullscreen
        } else {
            iced::window::Mode::Windowed
        };
        if let Some(id) = self.window_id {
            return iced::window::set_mode(id, mode);
        }
        Task::none()
    }

    pub fn handle_cycle_content_fit(&mut self) -> Task<Message> {
        self.content_fit = match self.content_fit {
            iced::ContentFit::Contain => iced::ContentFit::Cover,
            iced::ContentFit::Cover => iced::ContentFit::Fill,
            iced::ContentFit::Fill => iced::ContentFit::None,
            iced::ContentFit::None => iced::ContentFit::ScaleDown,
            iced::ContentFit::ScaleDown => iced::ContentFit::Contain,
        };
        Task::none()
    }

    pub fn handle_window_opened(&mut self, id: iced::window::Id) -> Task<Message> {
        self.window_id = Some(id);
        Task::none()
    }

    pub fn handle_open_file(&mut self) -> Task<Message> {
        // Use AsyncFileDialog so the dialog runs on a dedicated thread with
        // proper COM apartment initialization. Calling the synchronous
        // rfd::FileDialog::pick_file() on the Iced UI thread blocks the event
        // loop and prevents the Windows shell from receiving COM messages,
        // which causes the file list to take a very long time to load
        // ("Working on it...").
        Task::perform(
            async move {
                rfd::AsyncFileDialog::new()
                    .add_filter(
                        "Video Files",
                        &[
                            "mp4", "mkv", "avi", "mov", "webm", "wmv", "flv", "m4v", "mpg", "mpeg",
                            "ogv",
                        ],
                    )
                    .add_filter("All Files", &["*"])
                    .pick_file()
                    .await
                    .map(|handle| handle.path().to_path_buf())
            },
            Message::FilePicked,
        )
    }

    pub fn handle_file_picked(&mut self, path: Option<std::path::PathBuf>) -> Task<Message> {
        // Persist the outgoing video's position before it is replaced, so
        // switching files (without closing the app) still records progress.
        self.persist_current_position();

        if let Some(path) = path {
            let ps = path.display().to_string();
            self.video = VideoState::Loading(ps.clone());
            self.current_file_path = Some(ps.clone());
            self.subtitle_text.clear();
            self.subtitle_image = None;
            self.clear_dictionary();
            self.pending_subtitle = None;
            self.pending_resume = None;
            let url = crate::app_handlers_subtitle::file_url_from_path(&path);
            Task::perform(
                async move {
                    match iced_video_player::Video::new(&url) {
                        Ok(_) => Ok(ps),
                        Err(e) => Err(format!("{}", e)),
                    }
                },
                Message::FileOpened,
            )
        } else {
            Task::none()
        }
    }

    pub fn handle_file_opened(&mut self, result: Result<String, String>) -> Task<Message> {
        match result {
            Ok(ref ps) => {
                let url =
                    crate::app_handlers_subtitle::file_url_from_path(std::path::Path::new(ps));
                match iced_video_player::Video::new(&url) {
                    Ok(v) => {
                        self.video = VideoState::Ready(v);
                        self.position = 0.0;
                        self.settings.add_recent_file(ps);
                        crate::settings::save(&self.settings);
                        // Schedule a resume-seek for the first rendered frame.
                        self.pending_resume = self.resume_position_for(ps);
                        return self.apply_subtitle_auto(ps);
                    }
                    Err(e) => {
                        self.video = VideoState::NoVideo;
                        eprintln!("Error: {}", e);
                    }
                }
            }
            Err(e) => {
                self.video = VideoState::NoVideo;
                eprintln!("Error: {}", e);
            }
        }
        Task::none()
    }

    /// Determine the position (in seconds) to resume from for `path`, if any.
    /// Returns `None` when history is disabled, no position is recorded, the
    /// recording is too early to bother, or it lies within the last 10 seconds
    /// of the video (so a fully-watched file starts fresh instead of
    /// immediately hitting end-of-stream).
    fn resume_position_for(&self, path: &str) -> Option<f64> {
        if !self.settings.history_enabled {
            return None;
        }
        let saved = self.settings.playback_positions.get(path).copied()?;
        if saved <= 1.0 {
            return None;
        }
        let duration = self.video_duration();
        if duration > 0.0 && saved >= duration - 10.0 {
            return None;
        }
        Some(saved)
    }

    /// Periodic auto-save of the current playback position. Driven by a
    /// subscription that fires every few seconds while a video is loaded, so
    /// that a hard crash never loses more than a few seconds of progress.
    pub fn handle_save_position(&mut self) -> Task<Message> {
        self.persist_current_position();
        Task::none()
    }
}
