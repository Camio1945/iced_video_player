use crate::app_state::{App, Message, VideoState};
use crate::text_utils;
use iced::Task;
use iced::keyboard::{self, Key, key};
use std::time::Duration;

impl App {
    pub fn handle_toggle_pause(&mut self) -> Task<Message> {
        if let Some(_) = self.with_video_mut(|v| {
            let p = v.paused();
            v.set_paused(!p);
        }) {}
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
        if let VideoState::Ready(ref mut v) = self.video {
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
        if !self.dragging {
            self.position = self.current_pos();
        }
        Task::none()
    }

    pub fn handle_subtitle_text(&mut self, text: String) -> Task<Message> {
        self.subtitle_text = text_utils::clean_subtitle_text(&text);
        Task::none()
    }

    pub fn handle_subtitle_image(
        &mut self,
        img: Option<iced_video_player::pgs::PgsImage>,
    ) -> Task<Message> {
        self.subtitle_image =
            img.map(|i| iced::widget::image::Handle::from_rgba(i.width, i.height, i.rgba));
        Task::none()
    }

    pub fn handle_search_word(&mut self, word: String) -> Task<Message> {
        let w = word.clone();
        self.dict_word = word;
        self.dict_loading = true;
        self.dict_chinese.clear();
        self.dict_phonetic.clear();
        self.dict_sections.clear();
        self.dict_examples.clear();
        self.dict_error = None;
        Task::perform(
            async move { crate::dict::lookup(&w) },
            Message::DictionaryResult,
        )
    }

    pub fn handle_dictionary_result(&mut self, result: crate::dict::DictResult) -> Task<Message> {
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
        if let Some(path) = path {
            let ps = path.display().to_string();
            self.video = VideoState::Loading(ps.clone());
            self.current_file_path = Some(ps.clone());
            self.subtitle_text.clear();
            self.subtitle_image = None;
            self.clear_dictionary();
            self.pending_subtitle = None;
            let url = url::Url::from_file_path(&path).unwrap();
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
                let url = url::Url::from_file_path(std::path::Path::new(ps)).unwrap();
                match iced_video_player::Video::new(&url) {
                    Ok(v) => {
                        let has_builtin = v.has_builtin_subtitles();
                        self.video = VideoState::Ready(v);
                        self.position = 0.0;
                        self.apply_subtitle_auto(ps, has_builtin);
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

    fn apply_subtitle_auto(&mut self, video_path: &str, has_builtin: bool) {
        // 1) Explicit subtitle from CLI takes priority
        if let Some(sp) = self.pending_subtitle.take() {
            if let Ok(sub_url) = url::Url::from_file_path(&sp) {
                if let VideoState::Ready(ref mut vv) = self.video {
                    if let Err(e) = vv.set_subtitle_url(&sub_url) {
                        eprintln!("Subtitle error: {}", e);
                    }
                }
            }
            return;
        }

        // 2) If no built-in English subtitles, auto-discover external files
        if !has_builtin {
            if let Some(sub_path) =
                crate::subtitle_discovery::find_english_subtitle_file(video_path)
            {
                if let Ok(sub_url) = url::Url::from_file_path(&sub_path) {
                    if let VideoState::Ready(ref mut vv) = self.video {
                        if let Err(e) = vv.set_subtitle_url(&sub_url) {
                            eprintln!("Subtitle error: {}", e);
                        }
                    }
                }
            }
        }
    }

    pub fn handle_load_subtitle(&mut self) -> Task<Message> {
        // Use AsyncFileDialog for the same reason as handle_open_file: the
        // synchronous dialog blocks the UI thread and causes the Windows
        // shell file enumeration to stall.
        Task::perform(
            async move {
                rfd::AsyncFileDialog::new()
                    .add_filter(
                        "Subtitle Files",
                        &["srt", "ass", "ssa", "vtt", "sub", "smi"],
                    )
                    .add_filter("All Files", &["*"])
                    .pick_file()
                    .await
                    .map(|handle| handle.path().to_path_buf())
            },
            Message::SubtitlePicked,
        )
    }

    pub fn handle_subtitle_picked(&mut self, path: Option<std::path::PathBuf>) -> Task<Message> {
        if let Some(path) = path {
            let url = url::Url::from_file_path(&path).unwrap();
            if let Some(Err(e)) = self.with_video_mut(|v| v.set_subtitle_url(&url)) {
                eprintln!("Failed to load subtitle: {}", e);
            }
        }
        Task::none()
    }

    pub fn handle_keyboard_event(&mut self, event: keyboard::Event) -> Task<Message> {
        match event {
            keyboard::Event::KeyPressed { key, .. } => match &key {
                Key::Named(key::Named::Space) => self.handle_toggle_pause(),
                Key::Named(key::Named::ArrowLeft) => self.handle_skip_back(5),
                Key::Named(key::Named::ArrowRight) => self.handle_skip_forward(5),
                Key::Named(key::Named::ArrowUp) => {
                    let v = (self.volume + 0.05).min(2.0);
                    self.handle_set_volume(v)
                }
                Key::Named(key::Named::ArrowDown) => {
                    let v = (self.volume - 0.05).max(0.0);
                    self.handle_set_volume(v)
                }
                Key::Character(c) => self.handle_character_key(c.as_str()),
                Key::Named(key::Named::Escape) => {
                    if self.fullscreen {
                        self.handle_toggle_fullscreen()
                    } else if !self.dict_word.is_empty() {
                        self.handle_close_dictionary()
                    } else {
                        Task::none()
                    }
                }
                _ => Task::none(),
            },
            _ => Task::none(),
        }
    }

    fn handle_character_key(&mut self, c: &str) -> Task<Message> {
        match c {
            "f" | "F" => self.handle_toggle_fullscreen(),
            "m" | "M" => self.handle_toggle_mute(),
            "l" | "L" => self.handle_toggle_loop(),
            "[" => {
                let s = (self.speed - 0.25).max(0.25);
                self.handle_set_speed(s)
            }
            "]" => {
                let s = (self.speed + 0.25).min(4.0);
                self.handle_set_speed(s)
            }
            "," => self.handle_frame_step_backward(),
            "." => self.handle_frame_step_forward(),
            "o" | "O" => self.handle_open_file(),
            "s" | "S" => self.handle_load_subtitle(),
            "c" | "C" => self.handle_cycle_content_fit(),
            _ => Task::none(),
        }
    }
}
