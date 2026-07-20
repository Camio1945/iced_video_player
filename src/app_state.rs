use iced_video_player::Video;

use crate::dict::{DictResult, DictSection};

#[derive(Debug, Clone)]
pub enum Message {
    TogglePause,
    Seek(f64),
    SeekRelease,
    SkipBack(i64),
    SkipForward(i64),
    FrameStepForward,
    #[allow(dead_code)]
    FrameStepBackward,
    EndOfStream,
    NewFrame,
    PlaybackError(String),
    OpenFile,
    FilePicked(Option<std::path::PathBuf>),
    FileOpened(Result<String, String>),
    LoadSubtitle,
    SubtitlePicked(Option<std::path::PathBuf>),
    SubtitleText(String),
    SearchWord(String),
    DictionaryResult(DictResult),
    CloseDictionary,
    ToggleLoop,
    ToggleMute,
    SetVolume(f64),
    SetSpeed(f64),
    ToggleFullscreen,
    CycleContentFit,
    KeyboardEvent(iced::keyboard::Event),
    WindowOpened(iced::window::Id),
}

pub enum VideoState {
    NoVideo,
    Loading(String),
    Ready(Video),
}

pub struct App {
    pub video: VideoState,
    pub position: f64,
    pub dragging: bool,
    pub volume: f64,
    pub muted: bool,
    pub looping: bool,
    pub speed: f64,
    pub fullscreen: bool,
    pub content_fit: iced::ContentFit,
    pub subtitle_text: String,
    pub dict_word: String,
    pub dict_phonetic: String,
    pub dict_chinese: String,
    pub dict_sections: Vec<DictSection>,
    pub dict_examples: Vec<String>,
    pub dict_loading: bool,
    pub dict_error: Option<String>,
    pub current_file_path: Option<String>,
    pub window_id: Option<iced::window::Id>,
    pub pending_subtitle: Option<std::path::PathBuf>,
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
            dict_word: String::new(),
            dict_phonetic: String::new(),
            dict_chinese: String::new(),
            dict_sections: Vec::new(),
            dict_examples: Vec::new(),
            dict_loading: false,
            dict_error: None,
            current_file_path: None,
            window_id: None,
            pending_subtitle: None,
        }
    }
}

impl App {
    pub fn with_video<T>(&self, f: impl FnOnce(&Video) -> T) -> Option<T> {
        match &self.video {
            VideoState::Ready(v) => Some(f(v)),
            _ => None,
        }
    }

    pub fn with_video_mut<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Video) -> R,
    {
        match &mut self.video {
            VideoState::Ready(v) => Some(f(v)),
            _ => None,
        }
    }

    pub fn video_duration(&self) -> f64 {
        self.with_video(|v: &Video| v.duration().as_secs_f64())
            .unwrap_or(0.0)
    }

    pub fn current_pos(&self) -> f64 {
        self.with_video(|v: &Video| v.position().as_secs_f64())
            .unwrap_or(self.position)
    }

    pub fn clear_dictionary(&mut self) {
        self.dict_word.clear();
        self.dict_phonetic.clear();
        self.dict_chinese.clear();
        self.dict_sections.clear();
        self.dict_examples.clear();
        self.dict_loading = false;
        self.dict_error = None;
    }
}
