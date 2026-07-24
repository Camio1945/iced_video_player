//! Playlist sidebar view.

use crate::app_state::{App, Message};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Button, Container, button, column, scrollable, text};
use iced::{Alignment, Length, Renderer, Theme};

impl App {
    /// Build the playlist tab content.
    pub fn build_playlist_content(&self) -> iced::Element<'_, Message, Theme, Renderer> {
        let mut col = column![].spacing(4).padding(4);

        // Clear button (only when playlist has items)
        if !self.playlist.is_empty() {
            col = col.push(Self::build_clear_button());
        }

        // Playlist items
        for (i, path) in self.playlist.iter().enumerate() {
            col = col.push(Self::build_playlist_item(
                i,
                path.clone(),
                self.playlist_index,
            ));
        }

        // Empty state
        if self.playlist.is_empty() {
            col = col.push(Self::build_empty_state());
        }

        scrollable(col)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn build_clear_button() -> iced::Element<'static, Message, Theme, Renderer> {
        Button::new(
            Container::new(
                text("Clear Playlist")
                    .size(11)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center),
            )
            .width(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center),
        )
        .padding([8, 0])
        .width(Length::Fill)
        .on_press(Message::ClearPlaylist)
        .style(crate::styles::danger_btn)
        .into()
    }

    fn build_playlist_item(
        index: usize,
        path: String,
        current_index: Option<usize>,
    ) -> iced::Element<'static, Message, Theme, Renderer> {
        let name = std::path::Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or(path);

        let is_current = current_index == Some(index);
        button(text(name).size(12))
            .width(Length::Fill)
            .padding(4)
            .style(if is_current {
                button::primary
            } else {
                button::secondary
            })
            .on_press(Message::PlayPlaylistItem(index))
            .into()
    }

    fn build_empty_state() -> iced::Element<'static, Message, Theme, Renderer> {
        text("Play a video to auto-populate")
            .size(11)
            .align_x(Alignment::Center)
            .into()
    }
}
