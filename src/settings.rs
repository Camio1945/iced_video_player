//! Persistent application settings stored as JSON on disk.
//!
//! On Windows the config file lives at
//! `%APPDATA%\video-player\settings.json`; on other platforms it lives at
//! `$HOME/.config/video-player/settings.json`. The directory is created
//! automatically on first save.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Font size (in pixels) used for on-screen subtitles.
    pub subtitle_font_size: f32,
    /// Whether the file-open history feature is active.
    #[serde(default = "default_history_enabled")]
    pub history_enabled: bool,
    /// Maximum number of recent files to retain.
    #[serde(default = "default_max_history_items")]
    pub max_history_items: usize,
    /// Recently opened video file paths, most-recent first.
    #[serde(default)]
    pub recent_files: Vec<String>,
}

fn default_history_enabled() -> bool {
    true
}
fn default_max_history_items() -> usize {
    100
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            subtitle_font_size: 20.0,
            history_enabled: true,
            max_history_items: 100,
            recent_files: Vec::new(),
        }
    }
}

impl AppSettings {
    pub const MIN_FONT_SIZE: f32 = 12.0;
    pub const MAX_FONT_SIZE: f32 = 48.0;
    pub const FONT_STEP: f32 = 2.0;

    pub const MIN_HISTORY_ITEMS: usize = 10;
    pub const MAX_HISTORY_ITEMS: usize = 1000;
    pub const HISTORY_STEP: usize = 10;

    pub fn increase_font(&mut self) {
        self.subtitle_font_size =
            (self.subtitle_font_size + Self::FONT_STEP).min(Self::MAX_FONT_SIZE);
    }

    pub fn decrease_font(&mut self) {
        self.subtitle_font_size =
            (self.subtitle_font_size - Self::FONT_STEP).max(Self::MIN_FONT_SIZE);
    }

    pub fn increase_max_history(&mut self) {
        self.max_history_items =
            (self.max_history_items + Self::HISTORY_STEP).min(Self::MAX_HISTORY_ITEMS);
        self.recent_files.truncate(self.max_history_items);
    }

    pub fn decrease_max_history(&mut self) {
        self.max_history_items = (self.max_history_items.saturating_sub(Self::HISTORY_STEP))
            .max(Self::MIN_HISTORY_ITEMS);
        self.recent_files.truncate(self.max_history_items);
    }

    /// Record a successfully opened file. The path is moved to the front;
    /// duplicates are removed. Stale entries beyond `max_history_items`
    /// are dropped.
    pub fn add_recent_file(&mut self, path: &str) {
        if !self.history_enabled || path.is_empty() {
            return;
        }
        self.recent_files.retain(|f| f != path);
        self.recent_files.insert(0, path.to_string());
        self.recent_files.truncate(self.max_history_items);
    }
}

fn config_dir() -> std::io::Result<PathBuf> {
    let base = if cfg!(target_os = "windows") {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::NotFound, "APPDATA not set"))?
    } else {
        let home = std::env::var("HOME")
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::NotFound, "HOME not set"))?;
        let mut p = PathBuf::from(home);
        p.push(".config");
        p
    };
    let dir = base.join("video-player");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn config_path() -> std::io::Result<PathBuf> {
    Ok(config_dir()?.join("settings.json"))
}

/// Load settings from disk. Returns defaults if the file is missing,
/// unreadable, or contains invalid JSON.
pub fn load() -> AppSettings {
    config_path()
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<AppSettings>(&s).ok())
        .unwrap_or_default()
}

/// Persist settings to disk. Errors are intentionally swallowed because
/// settings are convenience state — a failed write should not crash the app.
pub fn save(settings: &AppSettings) {
    if let Ok(path) = config_path()
        && let Ok(json) = serde_json::to_string_pretty(settings)
    {
        if let Err(e) = std::fs::write(&path, json) {
            log::warn!("failed to save settings to {}: {e}", path.display());
        }
    }
}
