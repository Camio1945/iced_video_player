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
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            subtitle_font_size: 20.0,
        }
    }
}

impl AppSettings {
    pub const MIN_FONT_SIZE: f32 = 12.0;
    pub const MAX_FONT_SIZE: f32 = 48.0;
    pub const FONT_STEP: f32 = 2.0;

    pub fn increase_font(&mut self) {
        self.subtitle_font_size =
            (self.subtitle_font_size + Self::FONT_STEP).min(Self::MAX_FONT_SIZE);
    }

    pub fn decrease_font(&mut self) {
        self.subtitle_font_size =
            (self.subtitle_font_size - Self::FONT_STEP).max(Self::MIN_FONT_SIZE);
    }
}

fn config_dir() -> std::io::Result<PathBuf> {
    let base = if cfg!(target_os = "windows") {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "APPDATA not set")
            })?
    } else {
        let home = std::env::var("HOME").map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "HOME not set")
        })?;
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
        let _ = std::fs::write(path, json);
    }
}
