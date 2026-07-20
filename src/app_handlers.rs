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

    pub fn handle_search_word(&mut self, word: String) -> Task<Message> {
        let w = word.clone();
        self.dict_word = word;
        self.dict_loading = true;
        self.dict_chinese.clear();
        self.dict_phonetic.clear();
        self.dict_sections.clear();
        self.dict_examples.clear();
        self.dict_error = None;
        Task::perform(async move { crate::dict::lookup(&w) }, Message::DictionaryResult)
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
        let path = rfd::FileDialog::new()
            .add_filter(
                "Video Files",
                &[
                    "mp4", "mkv", "avi", "mov", "webm", "wmv", "flv", "m4v", "mpg", "mpeg", "ogv",
                ],
            )
            .add_filter("All Files", &["*"])
            .pick_file();
        if let Some(path) = path {
            let ps = path.display().to_string();
            self.video = VideoState::Loading(ps.clone());
            self.current_file_path = Some(ps.clone());
            self.subtitle_text.clear();
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
                        self.video = VideoState::Ready(v);
                        self.position = 0.0;
                        // Apply any pending subtitle
                        if let Some(sp) = self.pending_subtitle.take() {
                            if let Ok(sub_url) = url::Url::from_file_path(&sp) {
                                if let VideoState::Ready(ref mut vv) = self.video {
                                    if let Err(e) = vv.set_subtitle_url(&sub_url) {
                                        eprintln!("Subtitle error: {}", e);
                                    }
                                }
                            }
                        }
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

    pub fn handle_load_subtitle(&mut self) -> Task<Message> {
        let path = rfd::FileDialog::new()
            .add_filter(
                "Subtitle Files",
                &["srt", "ass", "ssa", "vtt", "sub", "smi"],
            )
            .add_filter("All Files", &["*"])
            .pick_file();
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
