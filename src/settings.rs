//! Persistent application settings stored as JSON on disk.
//!
//! On Windows the config file lives at
//! `%APPDATA%\video-player\settings.json`; on other platforms it lives at
//! `$HOME/.config/video-player/settings.json`. The directory is created
//! automatically on first save.

use crate::app_state::SidebarTab;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    /// Which sidebar tab was active when the app was last closed.
    #[serde(default)]
    pub active_tab: SidebarTab,
    /// Last-known playback position (in seconds) for recently opened files,
    /// keyed by absolute file path. Used to resume playback where the user
    /// left off.
    #[serde(default)]
    pub playback_positions: HashMap<String, f64>,
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
            active_tab: SidebarTab::default(),
            playback_positions: HashMap::new(),
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
        self.prune_resume_positions();
    }

    pub fn decrease_max_history(&mut self) {
        self.max_history_items = (self.max_history_items.saturating_sub(Self::HISTORY_STEP))
            .max(Self::MIN_HISTORY_ITEMS);
        self.recent_files.truncate(self.max_history_items);
        self.prune_resume_positions();
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

    /// Record the last-known playback position (in seconds) for `path`.
    /// Entries for files no longer in the recent list are pruned so the map
    /// stays bounded by `max_history_items`.
    pub fn set_resume_position(&mut self, path: &str, position: f64) {
        if path.is_empty() {
            return;
        }
        self.playback_positions.insert(path.to_string(), position);
        self.prune_resume_positions();
    }

    /// Drop resume positions for files that are no longer tracked in the
    /// recent-files list.
    pub fn prune_resume_positions(&mut self) {
        self.playback_positions
            .retain(|k, _| self.recent_files.iter().any(|f| f == k));
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
///
/// The write is performed atomically (write to a temp file in the same
/// directory, then rename over the target) so that a crash or power loss
/// mid-write leaves the previous good file intact rather than a truncated or
/// partially-written one. This is important because the file is written
/// periodically (every few seconds) to track playback positions.
pub fn save(settings: &AppSettings) {
    let Ok(path) = config_path() else {
        return;
    };
    let Ok(json) = serde_json::to_string_pretty(settings) else {
        return;
    };

    // Atomic write: temp file + rename (same directory so the rename is
    // atomic on both POSIX and Windows NTFS).
    if let Some(dir) = path.parent() {
        let tmp = dir.join("settings.json.tmp");
        if std::fs::write(&tmp, &json).is_ok() && std::fs::rename(&tmp, &path).is_ok() {
            return;
        }
        // Remove a leftover temp file from a failed rename (best-effort).
        let _ = std::fs::remove_file(&tmp);
    }

    // Fallback: direct write (less crash-safe but better than nothing).
    if let Err(e) = std::fs::write(&path, &json) {
        log::warn!("failed to save settings to {}: {e}", path.display());
    }
}
