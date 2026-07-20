mod dict;

use iced::{
    self, Color, Element, Length, Subscription, Task, Theme,
    alignment::{Horizontal, Vertical},
    border,
    keyboard::{self, Key, key},
    widget::{
        Button, Column, Container, PickList, Row, Slider, Space, Text, button, container,
        pick_list, text,
    },
    window,
};
use iced_video_player::{Video, VideoPlayer};
use std::time::Duration;

// ── Messages ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Message {
    TogglePause,
    Seek(f64),
    SeekRelease,
    SkipBack(i64),
    SkipForward(i64),
    FrameStepForward,
    FrameStepBackward,
    EndOfStream,
    NewFrame,
    PlaybackError(String),
    OpenFile,
    FileOpened(Result<String, String>),
    LoadSubtitle,
    SubtitleText(String),
    SearchWord(String),
    DictionaryResult(String, String),
    CloseDictionary,
    ToggleLoop,
    ToggleMute,
    SetVolume(f64),
    SetSpeed(f64),
    ToggleFullscreen,
    CycleContentFit,
    KeyboardEvent(keyboard::Event),
    WindowOpened(window::Id),
}

// ── Application state ─────────────────────────────────────────────────────

enum VideoState {
    NoVideo,
    Loading(String),
    Ready(Video),
}

struct App {
    video: VideoState,
    position: f64,
    dragging: bool,
    volume: f64,
    muted: bool,
    looping: bool,
    speed: f64,
    fullscreen: bool,
    content_fit: iced::ContentFit,
    subtitle_text: String,
    recent_words: Vec<String>,
    dict_word: String,
    dict_result: String,
    dict_loading: bool,
    current_file_path: Option<String>,
    window_id: Option<window::Id>,
    pending_subtitle: Option<std::path::PathBuf>,
}

impl Default for App {
    fn default() -> Self {
        App {
            video: VideoState::NoVideo,
            position: 0.0,
            dragging: false,
            volume: 1.0,
            muted: false,
            looping: false,
            speed: 1.0,
            fullscreen: false,
            content_fit: iced::ContentFit::Contain,
            subtitle_text: String::new(),
            recent_words: Vec::new(),
            dict_word: String::new(),
            dict_result: String::new(),
            dict_loading: false,
            current_file_path: None,
            window_id: None,
            pending_subtitle: None,
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn format_time(secs: f64) -> String {
    let total = secs as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{}:{:02}", m, s)
    }
}

fn clean_subtitle_text(raw: &str) -> String {
    let mut s = raw.to_string();
    // Strip SSA/ASS style tags
    s = s
        .replace("<b>", "")
        .replace("</b>", "")
        .replace("<i>", "")
        .replace("</i>", "")
        .replace("<u>", "")
        .replace("</u>", "")
        .replace("<font", "X")
        .replace("</font>", "")
        .replace("\\N", "\n")
        .replace("\\n", "\n")
        .replace("\\h", " ");
    // Decode common HTML entities
    s = s
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&amp;", "&")
        .replace("&#38;", "&")
        .replace("&lt;", "<")
        .replace("&#60;", "<")
        .replace("&gt;", ">")
        .replace("&#62;", ">")
        .replace("&nbsp;", " ")
        .replace("&#160;", " ");
    s.trim().to_string()
}

fn extract_words(text: &str) -> Vec<String> {
    // Common English stop words to exclude from language learning
    const STOP_WORDS: &[&str] = &[
        "a", "an", "the", "and", "or", "but", "if", "then", "else", "when", "at", "by", "for",
        "from", "in", "of", "on", "to", "with", "as", "is", "was", "are", "were", "be", "been",
        "being", "am", "do", "does", "did", "done", "have", "has", "had", "having", "will",
        "would", "should", "could", "can", "may", "might", "must", "shall", "it", "its", "this",
        "that", "these", "those", "i", "me", "my", "we", "us", "our", "you", "your", "he", "him",
        "his", "she", "her", "they", "them", "their", "what", "which", "who", "whom", "whose",
        "not", "no", "nor", "so", "too", "very", "just", "up", "down", "out", "about", "into",
        "over", "after", "before", "between", "through", "during", "above", "below", "re", "ve",
        "ll", "s", "t", "don", "didn", "doesn", "won", "isn", "aren", "couldn", "shouldn",
        "wouldn", "wasn", "weren", "hasn", "haven", "hadn", "mustn", "mightn", "apos", "ndash",
        "quot", "amp", "lt", "gt",
    ];
    let lower = text.to_lowercase();
    let words: Vec<String> = lower
        .split(|c: char| !c.is_alphabetic() && c != '\'')
        .map(|w| w.trim_matches(|c: char| !c.is_alphabetic()).to_string())
        .filter(|w| w.len() >= 3 && !STOP_WORDS.contains(&w.as_str()))
        .collect();
    let mut seen = std::collections::HashSet::new();
    let mut unique = Vec::new();
    for w in words {
        if seen.insert(w.clone()) {
            unique.push(w);
        }
    }
    unique.truncate(25);
    unique
}

// ── Video helpers ─────────────────────────────────────────────────────────

impl App {
    fn with_video<T>(&self, f: impl FnOnce(&Video) -> T) -> Option<T> {
        match &self.video {
            VideoState::Ready(v) => Some(f(v)),
            _ => None,
        }
    }
    fn with_video_mut<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Video) -> R,
    {
        match &mut self.video {
            VideoState::Ready(v) => Some(f(v)),
            _ => None,
        }
    }
    fn video_duration(&self) -> f64 {
        self.with_video(|v| v.duration().as_secs_f64())
            .unwrap_or(0.0)
    }
    fn current_pos(&self) -> f64 {
        self.with_video(|v| v.position().as_secs_f64())
            .unwrap_or(self.position)
    }
}

// ── update ────────────────────────────────────────────────────────────────

fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::TogglePause => {
            if let Some(_) = app.with_video_mut(|v| {
                let p = v.paused();
                v.set_paused(!p);
            }) {}
            Task::none()
        }
        Message::Seek(secs) => {
            app.dragging = true;
            app.position = secs;
            if let VideoState::Ready(ref mut v) = app.video {
                v.set_paused(true);
            }
            Task::none()
        }
        Message::SeekRelease => {
            app.dragging = false;
            if let VideoState::Ready(ref mut v) = app.video {
                let _ = v.seek(Duration::from_secs_f64(app.position), false);
                v.set_paused(false);
            }
            Task::none()
        }
        Message::SkipBack(secs) => {
            if let VideoState::Ready(ref mut v) = app.video {
                let n = (v.position().as_secs_f64() - secs as f64).max(0.0);
                app.position = n;
                let _ = v.seek(Duration::from_secs_f64(n), false);
            }
            Task::none()
        }
        Message::SkipForward(secs) => {
            if let VideoState::Ready(ref mut v) = app.video {
                let dur = v.duration().as_secs_f64();
                let n = (v.position().as_secs_f64() + secs as f64).min(dur);
                app.position = n;
                let _ = v.seek(Duration::from_secs_f64(n), false);
            }
            Task::none()
        }
        Message::FrameStepForward => {
            if let VideoState::Ready(ref mut v) = app.video {
                v.step_one_frame();
            }
            Task::none()
        }
        Message::FrameStepBackward => {
            if let VideoState::Ready(ref mut v) = app.video {
                let fps = v.framerate();
                let n = (v.position().as_secs_f64() - 1.0 / fps).max(0.0);
                app.position = n;
                let _ = v.seek(Duration::from_secs_f64(n), true);
                v.set_paused(true);
            }
            Task::none()
        }
        Message::EndOfStream => Task::none(),
        Message::NewFrame => {
            if !app.dragging {
                app.position = app.current_pos();
            }
            Task::none()
        }
        Message::PlaybackError(err) => {
            eprintln!("Playback error: {}", err);
            Task::none()
        }
        Message::OpenFile => {
            let path = rfd::FileDialog::new()
                .add_filter(
                    "Video Files",
                    &[
                        "mp4", "mkv", "avi", "mov", "webm", "wmv", "flv", "m4v", "mpg", "mpeg",
                        "ogv",
                    ],
                )
                .add_filter("All Files", &["*"])
                .pick_file();
            if let Some(path) = path {
                let ps = path.display().to_string();
                app.video = VideoState::Loading(ps.clone());
                app.current_file_path = Some(ps.clone());
                app.subtitle_text.clear();
                app.recent_words.clear();
                app.dict_result.clear();
                app.dict_word.clear();
                app.pending_subtitle = None;
                let url = url::Url::from_file_path(&path).unwrap();
                Task::perform(
                    async move {
                        match Video::new(&url) {
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
        Message::FileOpened(result) => {
            match result {
                Ok(ref ps) => {
                    let url = url::Url::from_file_path(std::path::Path::new(ps)).unwrap();
                    match Video::new(&url) {
                        Ok(v) => {
                            app.video = VideoState::Ready(v);
                            app.position = 0.0;
                            // Apply any pending subtitle
                            if let Some(sp) = app.pending_subtitle.take() {
                                if let Ok(sub_url) = url::Url::from_file_path(&sp) {
                                    if let VideoState::Ready(ref mut vv) = app.video {
                                        if let Err(e) = vv.set_subtitle_url(&sub_url) {
                                            eprintln!("Subtitle error: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            app.video = VideoState::NoVideo;
                            eprintln!("Error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    app.video = VideoState::NoVideo;
                    eprintln!("Error: {}", e);
                }
            }
            Task::none()
        }
        Message::LoadSubtitle => {
            let path = rfd::FileDialog::new()
                .add_filter(
                    "Subtitle Files",
                    &["srt", "ass", "ssa", "vtt", "sub", "smi"],
                )
                .add_filter("All Files", &["*"])
                .pick_file();
            if let Some(path) = path {
                let url = url::Url::from_file_path(&path).unwrap();
                if let Some(Err(e)) = app.with_video_mut(|v| v.set_subtitle_url(&url)) {
                    eprintln!("Failed to load subtitle: {}", e);
                }
            }
            Task::none()
        }
        Message::SubtitleText(text) => {
            app.subtitle_text = clean_subtitle_text(&text);
            app.recent_words = extract_words(&app.subtitle_text);
            Task::none()
        }
        Message::SearchWord(word) => {
            let w = word.clone();
            app.dict_word = word;
            app.dict_loading = true;
            app.dict_result = String::from("Loading...");
            Task::perform(async move { (w.clone(), dict::lookup(&w)) }, |(wd, def)| {
                Message::DictionaryResult(wd, def)
            })
        }
        Message::DictionaryResult(word, def) => {
            app.dict_word = word;
            app.dict_result = def;
            app.dict_loading = false;
            Task::none()
        }
        Message::CloseDictionary => {
            app.dict_word.clear();
            app.dict_result.clear();
            app.dict_loading = false;
            Task::none()
        }
        Message::ToggleLoop => {
            if let VideoState::Ready(ref mut v) = app.video {
                v.set_looping(!v.looping());
                app.looping = v.looping();
            }
            Task::none()
        }
        Message::ToggleMute => {
            app.muted = !app.muted;
            if let VideoState::Ready(ref mut v) = app.video {
                v.set_muted(app.muted);
            }
            Task::none()
        }
        Message::SetVolume(vol) => {
            app.volume = vol;
            if let VideoState::Ready(ref mut v) = app.video {
                v.set_volume(vol);
            }
            Task::none()
        }
        Message::SetSpeed(s) => {
            app.speed = s;
            if let VideoState::Ready(ref mut v) = app.video {
                let _ = v.set_speed(s);
            }
            Task::none()
        }
        Message::ToggleFullscreen => {
            app.fullscreen = !app.fullscreen;
            let mode = if app.fullscreen {
                window::Mode::Fullscreen
            } else {
                window::Mode::Windowed
            };
            if let Some(id) = app.window_id {
                return window::set_mode(id, mode);
            }
            Task::none()
        }
        Message::CycleContentFit => {
            app.content_fit = match app.content_fit {
                iced::ContentFit::Contain => iced::ContentFit::Cover,
                iced::ContentFit::Cover => iced::ContentFit::Fill,
                iced::ContentFit::Fill => iced::ContentFit::None,
                iced::ContentFit::None => iced::ContentFit::ScaleDown,
                iced::ContentFit::ScaleDown => iced::ContentFit::Contain,
            };
            Task::none()
        }
        Message::WindowOpened(id) => {
            app.window_id = Some(id);
            Task::none()
        }
        Message::KeyboardEvent(event) => {
            match event {
                keyboard::Event::KeyPressed { key, .. } => match &key {
                    Key::Named(key::Named::Space) => return update(app, Message::TogglePause),
                    Key::Named(key::Named::ArrowLeft) => return update(app, Message::SkipBack(5)),
                    Key::Named(key::Named::ArrowRight) => {
                        return update(app, Message::SkipForward(5));
                    }
                    Key::Named(key::Named::ArrowUp) => {
                        let v = (app.volume + 0.05).min(2.0);
                        return update(app, Message::SetVolume(v));
                    }
                    Key::Named(key::Named::ArrowDown) => {
                        let v = (app.volume - 0.05).max(0.0);
                        return update(app, Message::SetVolume(v));
                    }
                    Key::Character(c) => match c.as_str() {
                        "f" | "F" => return update(app, Message::ToggleFullscreen),
                        "m" | "M" => return update(app, Message::ToggleMute),
                        "l" | "L" => return update(app, Message::ToggleLoop),
                        "[" => {
                            let s = (app.speed - 0.25).max(0.25);
                            return update(app, Message::SetSpeed(s));
                        }
                        "]" => {
                            let s = (app.speed + 0.25).min(4.0);
                            return update(app, Message::SetSpeed(s));
                        }
                        "," => return update(app, Message::FrameStepBackward),
                        "." => return update(app, Message::FrameStepForward),
                        "o" | "O" => return update(app, Message::OpenFile),
                        "s" | "S" => return update(app, Message::LoadSubtitle),
                        "c" | "C" => return update(app, Message::CycleContentFit),
                        _ => {}
                    },
                    Key::Named(key::Named::Escape) => {
                        if app.fullscreen {
                            return update(app, Message::ToggleFullscreen);
                        }
                        if !app.dict_word.is_empty() {
                            return update(app, Message::CloseDictionary);
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
            Task::none()
        }
    }
}

// ── view ──────────────────────────────────────────────────────────────────

fn view(app: &App) -> Element<'_, Message> {
    let is_paused = app.with_video(|v| v.paused()).unwrap_or(true);
    let is_looping = app.with_video(|v| v.looping()).unwrap_or(app.looping);
    let has_video = matches!(&app.video, VideoState::Ready(_));
    let duration = app.video_duration();

    let toolbar = Row::new()
        .spacing(4)
        .padding(4)
        .align_y(Vertical::Center)
        .push(
            Button::new(Text::new("Open").size(12))
                .padding([4, 8])
                .on_press(Message::OpenFile)
                .style(ctrl_btn),
        )
        .push(
            Button::new(Text::new("Subtitle...").size(12))
                .padding([4, 8])
                .on_press_maybe(if has_video {
                    Some(Message::LoadSubtitle)
                } else {
                    None
                })
                .style(ctrl_btn),
        )
        .push(Space::new().width(Length::Fill))
        .push(
            text(format!(
                "{} / {}",
                format_time(app.position),
                format_time(duration)
            ))
            .size(12),
        );

    let seek_bar = Container::new(
        Slider::new(0.0..=duration.max(0.01), app.position, Message::Seek)
            .step(0.5)
            .on_release(Message::SeekRelease)
            .width(Length::Fill),
    )
    .padding([0, 8]);

    let speeds = vec![0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0, 4.0];

    let controls = Row::new()
        .spacing(6)
        .padding([4, 8])
        .align_y(Vertical::Center)
        .push(
            Button::new(Text::new("\u{23EA}").size(14))
                .padding([4, 6])
                .on_press_maybe(if has_video {
                    Some(Message::SkipBack(10))
                } else {
                    None
                })
                .style(ctrl_btn),
        )
        .push(
            Button::new(Text::new("\u{23F4}").size(14))
                .padding([4, 6])
                .on_press_maybe(if has_video {
                    Some(Message::SkipBack(5))
                } else {
                    None
                })
                .style(ctrl_btn),
        )
        .push(
            Button::new(Text::new(if is_paused { "\u{25B6}" } else { "\u{23F8}" }).size(18))
                .padding([4, 10])
                .on_press_maybe(if has_video {
                    Some(Message::TogglePause)
                } else {
                    None
                })
                .style(main_btn),
        )
        .push(
            Button::new(Text::new("\u{23F5}").size(14))
                .padding([4, 6])
                .on_press_maybe(if has_video {
                    Some(Message::SkipForward(5))
                } else {
                    None
                })
                .style(ctrl_btn),
        )
        .push(
            Button::new(Text::new("\u{23E9}").size(14))
                .padding([4, 6])
                .on_press_maybe(if has_video {
                    Some(Message::SkipForward(10))
                } else {
                    None
                })
                .style(ctrl_btn),
        )
        .push(
            Button::new(Text::new("|\u{25B6}").size(12))
                .padding([4, 6])
                .on_press_maybe(if has_video && is_paused {
                    Some(Message::FrameStepForward)
                } else {
                    None
                })
                .style(ctrl_btn),
        )
        .push(Space::new().width(Length::Fill))
        .push(Text::new("Speed:").size(11))
        .push(
            PickList::new(speeds, Some(app.speed), |s| Message::SetSpeed(s))
                .text_shaping(text::Shaping::Advanced)
                .handle(pick_list::Handle::Arrow { size: None })
                .width(Length::Fixed(80.0)),
        )
        .push(
            Button::new(Text::new("\u{1F501}").size(14))
                .padding([4, 6])
                .on_press(Message::ToggleLoop)
                .style(if is_looping { active_btn } else { ctrl_btn }),
        )
        .push(
            Button::new(Text::new(if app.muted { "\u{1F507}" } else { "\u{1F50A}" }).size(14))
                .padding([4, 6])
                .on_press(Message::ToggleMute)
                .style(ctrl_btn),
        )
        .push(
            Slider::new(0.0..=2.0, app.volume, Message::SetVolume)
                .step(0.05)
                .width(Length::Fixed(90.0)),
        )
        .push(
            Button::new(Text::new(format!("{:?}", app.content_fit)).size(10))
                .padding([4, 6])
                .on_press(Message::CycleContentFit)
                .style(ctrl_btn),
        )
        .push(
            Button::new(Text::new("\u{26F6}").size(14))
                .padding([4, 6])
                .on_press(Message::ToggleFullscreen)
                .style(ctrl_btn),
        );

    let video_area: Element<Message> = match &app.video {
        VideoState::Ready(video) => VideoPlayer::new(video)
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(app.content_fit)
            .on_end_of_stream(Message::EndOfStream)
            .on_new_frame(Message::NewFrame)
            .on_subtitle_text(|t| Message::SubtitleText(t.unwrap_or_default()))
            .on_error(|e| Message::PlaybackError(e.to_string()))
            .into(),
        VideoState::Loading(p) => Container::new(
            Column::new()
                .spacing(10)
                .align_x(Horizontal::Center)
                .push(Text::new("Loading video...").size(18))
                .push(Text::new(p.as_str()).size(12)),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(placeholder)
        .into(),
        VideoState::NoVideo => Container::new(
            Column::new()
                .spacing(12)
                .align_x(Horizontal::Center)
                .push(Text::new("\u{1F3AC}").size(48))
                .push(Text::new("No video loaded").size(18))
                .push(Text::new("Click \"Open\" or press O to load a video").size(14))
                .push(
                    Button::new(Text::new("Open Video File"))
                        .on_press(Message::OpenFile)
                        .padding([8, 20]),
                ),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(placeholder)
        .into(),
    };

    // Wrap the video in a Container with Length::Fill so it
    // participates in Iced's flex layout as a fluid child.
    let video_area: Element<Message> = Container::new(video_area)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

    let sub: Option<Element<Message>> = if !app.subtitle_text.is_empty() {
        Some(
            Container::new(
                text(&app.subtitle_text)
                    .size(18)
                    .color(Color::WHITE)
                    .align_x(Horizontal::Center),
            )
            .width(Length::Fill)
            .padding([4, 8])
            .style(sub_bg)
            .into(),
        )
    } else {
        None
    };

    let lang: Option<Element<Message>> = if !app.recent_words.is_empty() {
        let mut btn_row = Row::new().spacing(4).align_y(Vertical::Center);
        for w in &app.recent_words {
            btn_row = btn_row.push(
                Button::new(Text::new(w.as_str()).size(11))
                    .padding([2, 6])
                    .on_press(Message::SearchWord(w.clone()))
                    .style(word_btn),
            );
        }
        Some(
            Container::new(
                Column::new()
                    .spacing(4)
                    .padding([4, 8])
                    .push(Text::new("\u{1F4DA} Click a word to look up").size(12))
                    .push(btn_row),
            )
            .width(Length::Fill)
            .style(word_pnl)
            .into(),
        )
    } else {
        None
    };

    let dict: Option<Element<Message>> = if !app.dict_word.is_empty() {
        Some(
            Container::new(
                Column::new()
                    .spacing(6)
                    .padding(10)
                    .push(
                        Row::new()
                            .push(Text::new(format!("\u{1F516} {}", app.dict_word)).size(16))
                            .push(Space::new().width(Length::Fill))
                            .push(
                                Button::new(Text::new("\u{2715}").size(12))
                                    .padding([2, 6])
                                    .on_press(Message::CloseDictionary)
                                    .style(ctrl_btn),
                            ),
                    )
                    .push(
                        Text::new(if app.dict_loading {
                            "Looking up..."
                        } else {
                            &app.dict_result
                        })
                        .size(12),
                    ),
            )
            .width(Length::Fill)
            .style(dict_pop)
            .into(),
        )
    } else {
        None
    };

    // Layout strategy: controls (Shrink) pushed first so they
    // always get their content size; the video (Fill) pushed LAST
    // so it takes whatever space remains.  This avoids the Iced
    // pitfall where a Fill child before a Shrink child squeezes
    // the Shrink child to zero height.
    let mut main = Column::new().push(toolbar);
    if let Some(s) = sub {
        main = main.push(s);
    }
    main = main.push(seek_bar).push(controls);
    if let Some(l) = lang {
        main = main.push(l);
    }
    if let Some(d) = dict {
        main = main.push(d);
    }
    main = main.push(video_area);

    Container::new(main)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.1, 0.1, 0.12))),
            ..Default::default()
        })
        .into()
}

// ── subscription ──────────────────────────────────────────────────────────

fn subscription(_app: &App) -> Subscription<Message> {
    let keyboard_sub = keyboard::listen().map(Message::KeyboardEvent);
    let window_sub = window::open_events().map(Message::WindowOpened);
    Subscription::batch([keyboard_sub, window_sub])
}

// ── Styles ────────────────────────────────────────────────────────────────

fn ctrl_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.35, 0.35, 0.4),
        button::Status::Pressed => Color::from_rgb(0.25, 0.25, 0.3),
        _ => Color::from_rgb(0.2, 0.2, 0.25),
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: Color::from_rgb(0.85, 0.85, 0.85),
        border: border::rounded(4),
        ..Default::default()
    }
}
fn main_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.3, 0.55, 0.85),
        button::Status::Pressed => Color::from_rgb(0.2, 0.4, 0.7),
        _ => Color::from_rgb(0.2, 0.45, 0.75),
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: Color::WHITE,
        border: border::rounded(4),
        ..Default::default()
    }
}
fn active_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.3, 0.7, 0.4),
        button::Status::Pressed => Color::from_rgb(0.2, 0.55, 0.3),
        _ => Color::from_rgb(0.2, 0.6, 0.35),
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: Color::WHITE,
        border: border::rounded(4),
        ..Default::default()
    }
}
fn word_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.3, 0.45, 0.7),
        button::Status::Pressed => Color::from_rgb(0.2, 0.35, 0.6),
        _ => Color::from_rgb(0.18, 0.22, 0.35),
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: Color::from_rgb(0.85, 0.85, 0.9),
        border: border::rounded(3),
        ..Default::default()
    }
}
fn sub_bg(_: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgba(
            0.0, 0.0, 0.0, 0.65,
        ))),
        ..Default::default()
    }
}
fn word_pnl(_: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgb(0.12, 0.12, 0.18))),
        border: border::color(Color::from_rgb(0.25, 0.25, 0.35))
            .width(1.0)
            .rounded(4),
        ..Default::default()
    }
}
fn dict_pop(_: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgb(0.08, 0.1, 0.18))),
        border: border::color(Color::from_rgb(0.3, 0.5, 0.8))
            .width(1.5)
            .rounded(6),
        ..Default::default()
    }
}
fn placeholder(_: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgb(0.08, 0.08, 0.1))),
        ..Default::default()
    }
}

// ── Entry point ───────────────────────────────────────────────────────────

fn main() -> iced::Result {
    // Collect CLI args:
    //   video-player <file.mp4> [<file.srt>]
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (video_arg, subtitle_arg) = match args.len() {
        0 => (None, None),
        1 => (Some(args[0].clone()), None),
        _ => (Some(args[0].clone()), Some(args[1].clone())),
    };

    let boot = move || {
        let mut app = App::default();
        let mut initial_task = Task::none();

        if let Some(path) = video_arg.clone() {
            let path_str = std::path::Path::new(&path).display().to_string();
            app.video = VideoState::Loading(path_str.clone());
            app.current_file_path = Some(path_str);

            if let Some(sp) = subtitle_arg.clone() {
                let sub_path = std::path::Path::new(&sp).to_path_buf();
                app.pending_subtitle = Some(sub_path);
            }

            let url = url::Url::from_file_path(&path)
                .unwrap_or_else(|_| url::Url::parse(&format!("file:///{}", path)).unwrap());
            initial_task = Task::perform(
                async move {
                    let path_buf = std::path::PathBuf::from(&path);
                    match Video::new(&url) {
                        Ok(_) => Ok(path_buf.display().to_string()),
                        Err(e) => Err(format!("Failed to open: {}", e)),
                    }
                },
                Message::FileOpened,
            );
        }
        (app, initial_task)
    };

    iced::application(boot, update, view)
        .title(|app: &App| {
            let base = "Video Player";
            match &app.current_file_path {
                Some(p) => {
                    let name = std::path::Path::new(p)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(p);
                    format!("{} - {}", name, base)
                }
                None => base.to_string(),
            }
        })
        .subscription(subscription)
        .theme(|_: &App| Theme::Dark)
        .window(window::Settings {
            size: iced::Size::new(1200.0, 750.0),
            min_size: Some(iced::Size::new(640.0, 400.0)),
            ..Default::default()
        })
        .run()
}
