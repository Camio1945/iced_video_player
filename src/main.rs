#![windows_subsystem = "windows"]

mod app_dispatch;
mod app_handlers;
mod app_handlers_settings;
mod app_keyboard;
mod app_state;
mod boot;
mod dict;
mod dict_view;
mod dict_view_settings;
mod icons;
mod settings;
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
    border, keyboard,
    widget::{
        Button, Column, Container, Image, MouseArea, PickList, Row, Slider, Space, Stack, Text,
        container, pick_list, text,
    },
    window,
};
use iced_video_player::{Video, VideoPlayer};

// ── Spotify color helpers ───────────────────────────────────────────────

const GREEN: Color = Color::from_rgb(0.118, 0.843, 0.376);
const SILVER: Color = Color::from_rgb(0.702, 0.702, 0.702);
const MUTED: Color = Color::from_rgb(0.55, 0.55, 0.58);

// ── update ────────────────────────────────────────────────────────────────

fn update(app: &mut App, message: Message) -> Task<Message> {
    App::dispatch(app, message)
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
            background: Some(iced::Background::Color(Color::from_rgb(
                0.071, 0.071, 0.071,
            ))),
            ..Default::default()
        })
        .into()
}

fn build_player_column<'a>(
    app: &'a App,
    is_paused: bool,
    is_looping: bool,
) -> Element<'a, Message> {
    let bottom_panel = Container::new(
        Column::new()
            .width(Length::Fill)
            .push(build_seek_bar(app.position, app.video_duration()))
            .push(build_controls(is_paused, is_looping, app)),
    )
    .width(Length::Fill)
    .style(styles::control_panel);

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
        .height(Length::Fill)
        .padding(4)
        .style(styles::video_surface);

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
        stack = stack.push(build_text_subtitle_layer(
            &app.subtitle_text,
            app.settings.subtitle_font_size,
        ));
    }
    stack.into()
}

fn build_image_subtitle_layer(handle: &iced::widget::image::Handle) -> Container<'_, Message> {
    Container::new(Container::new(Image::new(handle.clone())).center_x(Length::Fill))
        .width(Length::Fill)
        .align_bottom(Length::Fill)
        .padding([0, 8])
}

fn build_text_subtitle_layer(text: &str, font_size: f32) -> Container<'_, Message> {
    Container::new(subtitle_view::build_subtitle_with_clickable_words(
        text, font_size,
    ))
    .width(Length::Fill)
    .align_bottom(Length::Fill)
    .padding([0, 48])
}

// ── Toolbar ─────────────────────────────────────────────────────────────

fn build_toolbar<'a>(has_video: bool, position: f64, duration: f64) -> Element<'a, Message> {
    Container::new(
        Row::new()
            .spacing(8)
            .padding([4, 12])
            .align_y(Vertical::Center)
            .push(
                Button::new(Text::new("OPEN").size(11))
                    .padding([4, 14])
                    .on_press(Message::OpenFile)
                    .style(styles::ctrl_btn),
            )
            .push(
                Button::new(Text::new("SUBTITLE...").size(11))
                    .padding([4, 14])
                    .on_press_maybe(if has_video {
                        Some(Message::LoadSubtitle)
                    } else {
                        None
                    })
                    .style(styles::ctrl_btn),
            )
            .push(Space::new().width(Length::Fill))
            .push(
                Text::new(text_utils::format_time(position))
                    .size(14)
                    .color(GREEN),
            )
            .push(Text::new(" / ").size(12).color(SILVER))
            .push(
                Text::new(text_utils::format_time(duration))
                    .size(12)
                    .color(SILVER),
            ),
    )
    .width(Length::Fill)
    .style(styles::toolbar_bg)
    .into()
}

// ── Seek bar ────────────────────────────────────────────────────────────

fn build_seek_bar<'a>(position: f64, duration: f64) -> Container<'a, Message> {
    Container::new(
        Slider::new(0.0..=duration.max(0.01), position, Message::Seek)
            .step(0.5)
            .on_release(Message::SeekRelease)
            .width(Length::Fill)
            .style(styles::slider_style),
    )
    .padding(iced::Padding {
        top: 8.0,
        right: 16.0,
        bottom: 0.0,
        left: 16.0,
    })
}

// ── Controls ────────────────────────────────────────────────────────────

fn build_controls<'a>(is_paused: bool, is_looping: bool, app: &App) -> Row<'a, Message> {
    let speeds = vec![0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0, 4.0];
    Row::new()
        .spacing(4)
        .padding([8, 12])
        .align_y(Vertical::Center)
        .push(widgets::skip_back_10_btn())
        .push(widgets::skip_back_5_btn())
        .push(Space::new().width(2))
        .push(widgets::pause_play_btn(is_paused))
        .push(Space::new().width(2))
        .push(widgets::skip_forward_5_btn())
        .push(widgets::skip_forward_10_btn())
        .push(widgets::frame_step_btn())
        .push(Space::new().width(Length::Fill))
        // ── right-side utility cluster ──
        .push(Text::new("Speed:").size(10).color(MUTED))
        .push(
            PickList::new(speeds, Some(app.speed), |s| Message::SetSpeed(s))
                .text_shaping(text::Shaping::Advanced)
                .handle(pick_list::Handle::Arrow { size: None })
                .width(Length::Fixed(72.0))
                .style(styles::pick_list_style),
        )
        .push(Space::new().width(4))
        .push(widgets::loop_btn(is_looping))
        .push(widgets::mute_btn(app.muted))
        .push(
            Slider::new(0.0..=2.0, app.volume, Message::SetVolume)
                .step(0.05)
                .width(Length::Fixed(80.0))
                .style(styles::slider_style),
        )
        .push(Space::new().width(2))
        .push(widgets::content_fit_btn(app.content_fit))
        .push(widgets::fullscreen_btn())
}

// ── Video area ──────────────────────────────────────────────────────────

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

// ── Loading screen ──────────────────────────────────────────────────────

fn build_loading_screen(path: &str) -> Element<'_, Message> {
    Container::new(
        Container::new(
            Column::new()
                .spacing(16)
                .align_x(Horizontal::Center)
                .push(Text::new("\u{23F3}").size(32))
                .push(Text::new("Loading video...").size(16).color(Color::WHITE))
                .push(Text::new(path).size(11).color(MUTED)),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(styles::placeholder)
    .into()
}

// ── No-video welcome screen ─────────────────────────────────────────────

fn build_no_video_screen() -> Element<'static, Message> {
    Container::new(
        Container::new(build_no_video_content())
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(styles::placeholder)
    .into()
}

fn build_no_video_content() -> Column<'static, Message> {
    Column::new()
        .spacing(16)
        .align_x(Horizontal::Center)
        .push(build_no_video_icon())
        .push(Text::new("No video loaded").size(18).color(Color::WHITE))
        .push(
            Text::new("Click OPEN or press O to load a video")
                .size(12)
                .color(SILVER),
        )
        .push(Space::new().height(4))
        .push(
            Button::new(Text::new("OPEN VIDEO FILE").size(12))
                .on_press(Message::OpenFile)
                .padding([10, 28])
                .style(styles::main_btn),
        )
}

fn build_no_video_icon() -> Container<'static, Message> {
    Container::new(
        Image::new(iced::widget::image::Handle::from_bytes(
            include_bytes!("../assets/icon.png") as &[u8],
        ))
        .width(Length::Fixed(96.0)),
    )
    .padding(20)
    .style(|_| container::Style {
        background: Some(iced::Background::Color(Color::from_rgb(0.10, 0.10, 0.10))),
        border: border::color(Color::from_rgba(0.118, 0.843, 0.376, 0.35))
            .width(2.0)
            .rounded(50.0),
        ..Default::default()
    })
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
