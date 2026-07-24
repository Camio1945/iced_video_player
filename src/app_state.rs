use iced_video_player::Video;

use crate::dict::{DictResult, DictSection};
use crate::settings::AppSettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SidebarTab {
    Dictionary,
    Settings,
    Playlist,
}

impl Default for SidebarTab {
    fn default() -> Self {
        SidebarTab::Dictionary
    }
}

#[allow(dead_code)]
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
    SubtitleImage(Option<iced_video_player::pgs::PgsImage>),
    SubtitleExtracted(Result<std::path::PathBuf, String>),
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
    SwitchSidebarTab(SidebarTab),
    IncreaseSubtitleFont,
    DecreaseSubtitleFont,
    ToggleHistory,
    IncreaseHistoryMaxItems,
    DecreaseHistoryMaxItems,
    ClearHistory,
    RemoveHistoryItem(String),
    OpenHistoryItem(String),
    AdjustVolume(f64),
    Tick,
    /// Periodic auto-save of the current playback position (crash resilience).
    SavePosition,
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
    pub subtitle_image: Option<iced::widget::image::Handle>,
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
    pub active_tab: SidebarTab,
    pub settings: AppSettings,
    /// Deferred resume position (seconds) applied on the first rendered frame
    /// after a video is opened. Cleared once consumed.
    pub pending_resume: Option<f64>,
}

impl Default for App {
    fn default() -> Self {
        let settings = crate::settings::load();
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
            subtitle_image: None,
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
            active_tab: settings.active_tab,
            settings,
            pending_resume: None,
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        // Best-effort: persist the exact final playback position so the next
        // session resumes from here. This complements the periodic auto-save
        // that covers hard crashes (where `Drop` does not run). Errors are
        // swallowed inside `persist_current_position`.
        self.persist_current_position();
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

    /// Save the current video's playback position to settings (best-effort).
    /// Used by the periodic auto-save and on application close. The disk write
    /// is skipped when the position hasn't moved meaningfully since the last
    /// save (e.g. while paused), but the in-memory value is always kept fresh
    /// enough by the periodic timer to satisfy the "within 10 seconds" crash
    /// tolerance.
    pub fn persist_current_position(&mut self) {
        if !self.settings.history_enabled {
            return;
        }
        let Some(path) = self.current_file_path.clone() else {
            return;
        };
        let pos = match &self.video {
            VideoState::Ready(v) => v.position().as_secs_f64(),
            _ => return,
        };
        if pos <= 0.0 {
            return;
        }
        // Skip the disk write if the position is essentially unchanged since
        // the last save (e.g. the video is paused). The stored value is
        // already current in that case.
        let unchanged = self
            .settings
            .playback_positions
            .get(&path)
            .is_some_and(|s| (s - pos).abs() <= 0.5);
        if unchanged {
            return;
        }
        self.settings.set_resume_position(&path, pos);
        crate::settings::save(&self.settings);
    }
}
