#![windows_subsystem = "windows"]

mod app_handlers;
mod app_keyboard;
mod app_state;
mod boot;
mod dict;
mod dict_view;
mod icons;
mod styles;
mod subtitle_discovery;
mod subtitle_extract;
mod subtitle_view;
mod text_utils;
mod widgets;

use app_state::{App, Message, VideoState};
use iced::{
    self, Color, Element, Length, Subscription, Task, Theme,
    alignment::{Horizontal, Vertical},
    keyboard,
    widget::{
        Button, Column, Container, Image, MouseArea, PickList, Row, Slider, Space, Stack, Text,
        container, pick_list, text,
    },
    window,
};
use iced_video_player::{Video, VideoPlayer};

// ── update ────────────────────────────────────────────────────────────────
fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::TogglePause => app.handle_toggle_pause(),
        Message::Seek(s) => app.handle_seek(s),
        Message::SeekRelease => app.handle_seek_release(),
        Message::SkipBack(s) => app.handle_skip_back(s),
        Message::SkipForward(s) => app.handle_skip_forward(s),
        Message::FrameStepForward => app.handle_frame_step_forward(),
        Message::FrameStepBackward => app.handle_frame_step_backward(),
        Message::EndOfStream => Task::none(),
        Message::NewFrame => app.handle_new_frame(),
        Message::PlaybackError(err) => {
            eprintln!("Playback error: {}", err);
            Task::none()
        }
        Message::OpenFile => app.handle_open_file(),
        Message::FilePicked(p) => app.handle_file_picked(p),
        Message::FileOpened(r) => app.handle_file_opened(r),
        Message::LoadSubtitle => app.handle_load_subtitle(),
        Message::SubtitlePicked(p) => app.handle_subtitle_picked(p),
        Message::SubtitleText(t) => app.handle_subtitle_text(t),
        Message::SubtitleImage(i) => app.handle_subtitle_image(i),
        Message::SubtitleExtracted(r) => app.handle_subtitle_extracted(r),
        Message::SearchWord(w) => app.handle_search_word(w),
        Message::DictionaryResult(r) => app.handle_dictionary_result(r),
        Message::CloseDictionary => app.handle_close_dictionary(),
        Message::ToggleLoop => app.handle_toggle_loop(),
        Message::ToggleMute => app.handle_toggle_mute(),
        Message::SetVolume(v) => app.handle_set_volume(v),
        Message::SetSpeed(s) => app.handle_set_speed(s),
        Message::ToggleFullscreen => app.handle_toggle_fullscreen(),
        Message::CycleContentFit => app.handle_cycle_content_fit(),
        Message::WindowOpened(id) => app.handle_window_opened(id),
        Message::KeyboardEvent(e) => app.handle_keyboard_event(e),
    }
}

// ── view ──────────────────────────────────────────────────────────────────

fn view(app: &App) -> Element<'_, Message> {
    let is_paused = app.with_video(|v: &Video| v.paused()).unwrap_or(true);
    let is_looping = app
        .with_video(|v: &Video| v.looping())
        .unwrap_or(app.looping);
    let has_video = matches!(&app.video, VideoState::Ready(_));

    let main_row = Row::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(0)
        .push(
            Container::new(build_player_column(app, is_paused, is_looping))
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .push(dict_view::build_dictionary_sidebar(app));

    let layout = Column::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .push(build_toolbar(has_video, app.position, app.video_duration()))
        .push(main_row);

    Container::new(layout)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.1, 0.1, 0.12))),
            ..Default::default()
        })
        .into()
}

fn build_player_column<'a>(app: &'a App, is_paused: bool, is_looping: bool) -> Element<'a, Message> {
    let bottom_panel = Column::new()
        .width(Length::Fill)
        .push(build_seek_bar(app.position, app.video_duration()))
        .push(build_controls(is_paused, is_looping, app));

    Column::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .push(build_player_area(app))
        .push(bottom_panel)
        .into()
}

fn build_player_area(app: &App) -> Element<'_, Message> {
    let video_container = Container::new(build_video_area(app))
        .width(Length::Fill)
        .height(Length::Fill);

    let has_text_sub = !app.subtitle_text.is_empty();
    let has_image_sub = app.subtitle_image.is_some();
    if !has_text_sub && !has_image_sub {
        return video_container.into();
    }

    let mut stack = Stack::new().push(video_container);
    if let Some(ref handle) = app.subtitle_image {
        stack = stack.push(build_image_subtitle_layer(handle));
    }
    if has_text_sub {
        stack = stack.push(build_text_subtitle_layer(&app.subtitle_text));
    }
    stack.into()
}

fn build_image_subtitle_layer(handle: &iced::widget::image::Handle) -> Container<'_, Message> {
    Container::new(Container::new(Image::new(handle.clone())).center_x(Length::Fill))
        .width(Length::Fill)
        .align_bottom(Length::Fill)
        .padding(iced::Padding {
            top: 0.0,
            right: 0.0,
            bottom: 40.0,
            left: 0.0,
        })
}

fn build_text_subtitle_layer(text: &str) -> Container<'_, Message> {
    Container::new(subtitle_view::build_subtitle_with_clickable_words(text))
        .width(Length::Fill)
        .align_bottom(Length::Fill)
        .padding([0, 48])
}

fn build_toolbar<'a>(has_video: bool, position: f64, duration: f64) -> Row<'a, Message> {
    Row::new()
        .spacing(4)
        .padding(4)
        .align_y(Vertical::Center)
        .push(
            Button::new(Text::new("Open").size(12))
                .padding([4, 8])
                .on_press(Message::OpenFile)
                .style(styles::ctrl_btn),
        )
        .push(
            Button::new(Text::new("Subtitle...").size(12))
                .padding([4, 8])
                .on_press_maybe(if has_video {
                    Some(Message::LoadSubtitle)
                } else {
                    None
                })
                .style(styles::ctrl_btn),
        )
        .push(Space::new().width(Length::Fill))
        .push(
            text(format!(
                "{} / {}",
                text_utils::format_time(position),
                text_utils::format_time(duration)
            ))
            .size(12),
        )
}

fn build_seek_bar<'a>(position: f64, duration: f64) -> Container<'a, Message> {
    Container::new(
        Slider::new(0.0..=duration.max(0.01), position, Message::Seek)
            .step(0.5)
            .on_release(Message::SeekRelease)
            .width(Length::Fill),
    )
    .padding([0, 8])
}

fn build_controls<'a>(is_paused: bool, is_looping: bool, app: &App) -> Row<'a, Message> {
    let speeds = vec![0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0, 4.0];
    Row::new()
        .spacing(6)
        .padding([4, 8])
        .align_y(Vertical::Center)
        .push(widgets::skip_back_10_btn())
        .push(widgets::skip_back_5_btn())
        .push(widgets::pause_play_btn(is_paused))
        .push(widgets::skip_forward_5_btn())
        .push(widgets::skip_forward_10_btn())
        .push(widgets::frame_step_btn())
        .push(Space::new().width(Length::Fill))
        .push(Text::new("Speed:").size(11))
        .push(
            PickList::new(speeds, Some(app.speed), |s| Message::SetSpeed(s))
                .text_shaping(text::Shaping::Advanced)
                .handle(pick_list::Handle::Arrow { size: None })
                .width(Length::Fixed(80.0)),
        )
        .push(widgets::loop_btn(is_looping))
        .push(widgets::mute_btn(app.muted))
        .push(
            Slider::new(0.0..=2.0, app.volume, Message::SetVolume)
                .step(0.05)
                .width(Length::Fixed(90.0)),
        )
        .push(widgets::content_fit_btn(app.content_fit))
        .push(widgets::fullscreen_btn())
}

fn build_video_area(app: &App) -> Element<'_, Message> {
    match &app.video {
        VideoState::Ready(video) => {
            MouseArea::new(build_video_player_widget(video, app.content_fit))
                .on_press(Message::TogglePause)
                .on_double_click(Message::ToggleFullscreen)
                .into()
        }
        VideoState::Loading(p) => build_loading_screen(p),
        VideoState::NoVideo => build_no_video_screen(),
    }
}

fn build_video_player_widget<'a>(
    video: &'a Video,
    content_fit: iced::ContentFit,
) -> Element<'a, Message> {
    VideoPlayer::new(video)
        .width(Length::Fill)
        .height(Length::Fill)
        .content_fit(content_fit)
        .on_end_of_stream(Message::EndOfStream)
        .on_new_frame(Message::NewFrame)
        .on_subtitle_text(|t: Option<String>| Message::SubtitleText(t.unwrap_or_default()))
        .on_subtitle_image(Message::SubtitleImage)
        .on_error(|e: &glib::Error| Message::PlaybackError(e.to_string()))
        .into()
}

fn build_loading_screen(path: &str) -> Element<'_, Message> {
    Container::new(
        Column::new()
            .spacing(10)
            .align_x(Horizontal::Center)
            .push(Text::new("Loading video...").size(18))
            .push(Text::new(path).size(12)),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(styles::placeholder)
    .into()
}

fn build_no_video_screen() -> Element<'static, Message> {
    Container::new(
        Column::new()
            .spacing(12)
            .align_x(Horizontal::Center)
            .push(
                Image::new(iced::widget::image::Handle::from_bytes(include_bytes!(
                    "../assets/icon.png"
                )
                    as &[u8]))
                .width(Length::Fixed(140.0)),
            )
            .push(Text::new("No video loaded").size(18))
            .push(Text::new("Click \"Open\" or press O to load a video").size(14))
            .push(
                Button::new(Text::new("Open Video File"))
                    .on_press(Message::OpenFile)
                    .padding([8, 20]),
            ),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(styles::placeholder)
    .into()
}

// ── subscription ──────────────────────────────────────────────────────────

fn subscription(_app: &App) -> Subscription<Message> {
    let keyboard_sub = keyboard::listen().map(Message::KeyboardEvent);
    let window_sub = window::open_events().map(Message::WindowOpened);
    Subscription::batch([keyboard_sub, window_sub])
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
