//! Playlist event handlers.

use crate::app_state::{Message, VideoState};
use iced::Task;

impl crate::app_state::App {
    /// Clear the playlist.
    pub fn handle_clear_playlist(&mut self) -> Task<Message> {
        self.playlist.clear();
        self.playlist_index = None;
        Task::none()
    }

    /// Play a specific item from the playlist.
    pub fn handle_play_playlist_item(&mut self, index: usize) -> Task<Message> {
        if index >= self.playlist.len() {
            return Task::none();
        }
        let path = self.playlist[index].clone();
        self.playlist_index = Some(index);
        self.open_video_file(&path)
    }

    /// Play the previous video in the playlist (Page Up).
    pub fn handle_playlist_prev(&mut self) -> Task<Message> {
        let current = self.playlist_index.unwrap_or(0);
        if current == 0 {
            return Task::none();
        }
        self.handle_play_playlist_item(current - 1)
    }

    /// Play the next video in the playlist (Page Down).
    pub fn handle_playlist_next(&mut self) -> Task<Message> {
        let current = self.playlist_index.unwrap_or(0);
        if current + 1 >= self.playlist.len() {
            return Task::none();
        }
        self.handle_play_playlist_item(current + 1)
    }

    /// Handle dropped files (drag-and-drop).
    pub fn handle_playlist_drop_files(&mut self, files: Vec<std::path::PathBuf>) -> Task<Message> {
        let mut videos: Vec<String> = Vec::new();
        for p in files {
            if p.is_file() && crate::playlist::is_video_file(&p) {
                videos.push(p.display().to_string());
            } else if p.is_dir() {
                videos.extend(crate::playlist::scan_directory_for_videos(&p));
            }
        }
        videos.sort();
        if videos.is_empty() {
            return Task::none();
        }
        self.playlist = videos;
        self.playlist_index = None;
        self.handle_play_playlist_item(0)
    }

    /// Internal helper: open a video file and update state.
    pub fn open_video_file(&mut self, path: &str) -> Task<Message> {
        self.persist_current_position();
        let ps = path.to_string();
        self.video = VideoState::Loading(ps.clone());
        self.current_file_path = Some(ps.clone());
        self.subtitle_text.clear();
        self.subtitle_image = None;
        self.clear_dictionary();
        self.pending_subtitle = None;
        self.pending_resume = None;
        self.subtitle_cues.clear();
        self.last_home_seek = None;
        let url = crate::app_handlers_subtitle::file_url_from_path(std::path::Path::new(&ps));
        Task::perform(
            async move {
                match iced_video_player::Video::new(&url) {
                    Ok(_) => Ok(ps),
                    Err(e) => Err(format!("{}", e)),
                }
            },
            Message::FileOpened,
        )
    }

    /// Auto-populate playlist when a video is opened and playlist is empty.
    pub fn auto_populate_playlist(&mut self, file_path: &str) {
        if !self.playlist.is_empty() {
            return;
        }
        let Some(dir) = crate::playlist::parent_directory(file_path) else {
            return;
        };
        let videos = crate::playlist::scan_directory_for_videos(&dir);
        if videos.is_empty() {
            return;
        }
        let index = videos.iter().position(|p| p == file_path);
        self.playlist = videos;
        self.playlist_index = index;
    }

    /// Handle a file dropped onto the window.
    ///
    /// Only processes the drop if:
    /// 1. The playlist tab is active
    /// 2. The playlist is currently empty
    pub fn handle_window_file_dropped(&mut self, path: std::path::PathBuf) -> Task<Message> {
        use crate::app_state::SidebarTab;

        // Only handle drops when playlist tab is active and playlist is empty
        if self.active_tab != SidebarTab::Playlist || !self.playlist.is_empty() {
            return Task::none();
        }

        // Reuse the existing drop handler
        self.handle_playlist_drop_files(vec![path])
    }
}
