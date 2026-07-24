use crate::app_state::{App, Message};
use iced::Task;

impl App {
    /// Main message dispatcher — delegates to the appropriate handler.
    /// Settings and history messages are forwarded to `dispatch_settings`.
    pub fn dispatch(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TogglePause => self.handle_toggle_pause(),
            Message::Seek(s) => self.handle_seek(s),
            Message::SeekRelease => self.handle_seek_release(),
            Message::SkipBack(s) => self.handle_skip_back(s),
            Message::SkipForward(s) => self.handle_skip_forward(s),
            Message::FrameStepForward => self.handle_frame_step_forward(),
            Message::FrameStepBackward => self.handle_frame_step_backward(),
            Message::EndOfStream => Task::none(),
            Message::NewFrame => self.handle_new_frame(),
            Message::PlaybackError(err) => self.handle_playback_error(err),
            Message::OpenFile => self.handle_open_file(),
            Message::FilePicked(p) => self.handle_file_picked(p),
            Message::FileOpened(r) => self.handle_file_opened(r),
            Message::LoadSubtitle => self.handle_load_subtitle(),
            Message::SubtitlePicked(p) => self.handle_subtitle_picked(p),
            Message::SubtitleText(t) => self.handle_subtitle_text(t),
            Message::SubtitleImage(i) => self.handle_subtitle_image(i),
            Message::SubtitleExtracted(r) => self.handle_subtitle_extracted(r),
            Message::SearchWord(w) => self.handle_search_word(w),
            Message::DictionaryResult(r) => self.handle_dictionary_result(r),
            Message::CloseDictionary => self.handle_close_dictionary(),
            Message::ToggleLoop => self.handle_toggle_loop(),
            Message::ToggleMute => self.handle_toggle_mute(),
            Message::SetVolume(v) => self.handle_set_volume(v),
            Message::SetSpeed(s) => self.handle_set_speed(s),
            Message::ToggleFullscreen => self.handle_toggle_fullscreen(),
            Message::CycleContentFit => self.handle_cycle_content_fit(),
            Message::WindowOpened(id) => self.handle_window_opened(id),
            Message::KeyboardEvent(e) => self.handle_keyboard_event(e),
            Message::AdjustVolume(d) => self.handle_adjust_volume(d),
            Message::Tick => self.handle_tick(),
            Message::SavePosition => self.handle_save_position(),
            _ => self.dispatch_secondary(message),
        }
    }

    /// Secondary dispatcher for playlist and settings messages.
    fn dispatch_secondary(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ClearPlaylist => self.handle_clear_playlist(),
            Message::PlayPlaylistItem(i) => self.handle_play_playlist_item(i),
            Message::PlaylistPrev => self.handle_playlist_prev(),
            Message::PlaylistNext => self.handle_playlist_next(),
            Message::PlaylistDropFiles(files) => self.handle_playlist_drop_files(files),
            Message::WindowFileDropped(path) => self.handle_window_file_dropped(path),
            other => self.dispatch_settings(other),
        }
    }

    /// Handles sidebar, settings, and history messages.
    fn dispatch_settings(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SwitchSidebarTab(t) => self.handle_switch_sidebar_tab(t),
            Message::IncreaseSubtitleFont => self.handle_increase_subtitle_font(),
            Message::DecreaseSubtitleFont => self.handle_decrease_subtitle_font(),
            Message::ToggleHistory => self.handle_toggle_history(),
            Message::IncreaseHistoryMaxItems => self.handle_increase_history_max_items(),
            Message::DecreaseHistoryMaxItems => self.handle_decrease_history_max_items(),
            Message::ClearHistory => self.handle_clear_history(),
            Message::RemoveHistoryItem(p) => self.handle_remove_history_item(p),
            Message::OpenHistoryItem(p) => self.handle_open_history_item(p),
            other => {
                eprintln!("dispatch_settings: unexpected message variant {:?}", other);
                Task::none()
            }
        }
    }
}
