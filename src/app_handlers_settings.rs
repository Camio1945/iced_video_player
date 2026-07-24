use crate::app_state::{App, Message, SidebarTab};
use iced::Task;

impl App {
    pub fn handle_switch_sidebar_tab(&mut self, tab: SidebarTab) -> Task<Message> {
        self.active_tab = tab;
        self.settings.active_tab = tab;
        crate::settings::save(&self.settings);
        Task::none()
    }

    pub fn handle_increase_subtitle_font(&mut self) -> Task<Message> {
        self.settings.increase_font();
        crate::settings::save(&self.settings);
        Task::none()
    }

    pub fn handle_decrease_subtitle_font(&mut self) -> Task<Message> {
        self.settings.decrease_font();
        crate::settings::save(&self.settings);
        Task::none()
    }

    pub fn handle_toggle_history(&mut self) -> Task<Message> {
        self.settings.history_enabled = !self.settings.history_enabled;
        crate::settings::save(&self.settings);
        Task::none()
    }

    pub fn handle_increase_history_max_items(&mut self) -> Task<Message> {
        self.settings.increase_max_history();
        crate::settings::save(&self.settings);
        Task::none()
    }

    pub fn handle_decrease_history_max_items(&mut self) -> Task<Message> {
        self.settings.decrease_max_history();
        crate::settings::save(&self.settings);
        Task::none()
    }

    pub fn handle_clear_history(&mut self) -> Task<Message> {
        self.settings.recent_files.clear();
        self.settings.playback_positions.clear();
        crate::settings::save(&self.settings);
        Task::none()
    }

    pub fn handle_remove_history_item(&mut self, path: String) -> Task<Message> {
        self.settings.recent_files.retain(|f| f != &path);
        self.settings.playback_positions.remove(&path);
        crate::settings::save(&self.settings);
        Task::none()
    }

    pub fn handle_open_history_item(&mut self, path: String) -> Task<Message> {
        let path_buf = std::path::PathBuf::from(&path);
        self.handle_file_picked(Some(path_buf))
    }
}
