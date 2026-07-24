// Use "windows" subsystem for release builds (no console window).
// Keep the console in debug builds so eprintln! messages are visible.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(unsafe_code)]
#![allow(static_mut_refs)]

mod app_dispatch;
mod app_handlers;
mod app_handlers_playlist;
mod app_handlers_settings;
mod app_handlers_subtitle;
mod app_keyboard;
mod app_state;
mod boot;
mod dict;
mod dict_view;
mod dict_view_settings;
mod dict_webview;
mod icons;
mod playlist;
mod playlist_view;
mod settings;
mod styles;
mod subtitle_discovery;
mod subtitle_extract;
mod subtitle_parse;
mod subtitle_view;
mod text_utils;
mod views;
mod widgets;

use app_state::{App, Message, VideoState};
use iced::{self, Subscription, Task, Theme, keyboard, window};
use std::sync::OnceLock;
use std::time::Duration;

// ── Static assets ───────────────────────────────────────────────────────

/// Cached image handle for the no-video welcome icon.
///
/// `iced::widget::image::Handle::from_bytes` generates a *new unique* `Id`
/// on every call, so creating the handle inside the view function made the
/// wgpu renderer treat the icon as a brand-new image on every redraw and
/// re-upload it asynchronously — during which the image is drawn as
/// nothing for a frame, producing a visible flicker whenever the user
/// hovered the OPEN VIDEO FILE button (any pointer move triggers a view
/// rebuild). Caching the handle in a `OnceLock` gives it a stable `Id` so
/// the renderer's atlas entry is reused across frames.
pub(crate) static NO_VIDEO_ICON: OnceLock<iced::widget::image::Handle> = OnceLock::new();

// ── update ────────────────────────────────────────────────────────────────

fn update(app: &mut App, message: Message) -> Task<Message> {
    App::dispatch(app, message)
}

// ── view ──────────────────────────────────────────────────────────────────

fn view(app: &App) -> iced::Element<'_, Message> {
    views::view(app)
}

// ── subscription ──────────────────────────────────────────────────────────

fn subscription(app: &App) -> Subscription<Message> {
    let keyboard_sub = keyboard::listen().map(Message::KeyboardEvent);
    let window_sub = window::open_events().map(Message::WindowOpened);

    // Listen for file drop events (drag-and-drop files/folders onto the window).
    // When files are dropped while the playlist tab is active and empty,
    // they are added to the playlist.
    let file_drop_sub = window::events().filter_map(|(_id, event)| {
        match event {
            window::Event::FileDropped(path) => Some(Message::WindowFileDropped(path)),
            _ => None,
        }
    });

    // Only run the dictionary-webview tick when a lookup is active or a
    // webview still exists. This avoids re-rendering the no-video screen
    // every tick and fixes the flicker reported when no video is open.
    let needs_tick = !app.dict_word.is_empty() || crate::dict_webview::has_webview();
    let tick_sub = if needs_tick {
        iced::time::every(Duration::from_millis(200)).map(|_| Message::Tick)
    } else {
        Subscription::none()
    };

    // Periodically persist the current playback position so a crash never
    // loses more than a few seconds of progress. Only active while a video is
    // actually loaded.
    let position_sub = if matches!(app.video, VideoState::Ready(_)) {
        iced::time::every(Duration::from_secs(5)).map(|_| Message::SavePosition)
    } else {
        Subscription::none()
    };

    Subscription::batch([keyboard_sub, window_sub, file_drop_sub, tick_sub, position_sub])
}

// ── Entry point ───────────────────────────────────────────────────────────

fn main() -> iced::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (video_arg, subtitle_arg) = boot::parse_cli_args(&args);
    let boot = boot::create_boot_closure(video_arg, subtitle_arg);

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
            size: iced::Size::new(1280.0, 760.0),
            min_size: Some(iced::Size::new(800.0, 480.0)),
            maximized: true,
            icon: boot::load_window_icon(),
            ..Default::default()
        })
        .run()
}
