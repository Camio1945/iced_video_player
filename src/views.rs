//! View builder functions extracted from main.rs.
//!
//! These functions construct the Iced widget tree for the video player UI.

use crate::NO_VIDEO_ICON;
use crate::app_state::{App, Message, VideoState};
use iced::{
    Color, Element, Length,
    alignment::{Horizontal, Vertical},
    border,
    widget::{
        Button, Column, Container, Image, MouseArea, PickList, Row, Slider, Space, Stack, Text,
        container, pick_list, text,
    },
};
use iced_video_player::{Video, VideoPlayer};

// ── Spotify color helpers ───────────────────────────────────────────────

pub(crate) const GREEN: Color = Color::from_rgb(0.118, 0.843, 0.376);
pub(crate) const SILVER: Color = Color::from_rgb(0.702, 0.702, 0.702);
pub(crate) const MUTED: Color = Color::from_rgb(0.55, 0.55, 0.58);

// ── view ──────────────────────────────────────────────────────────────────

pub(crate) fn view(app: &App) -> Element<'_, Message> {
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
        .push(crate::dict_view::build_dictionary_sidebar(app));

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
    .style(crate::styles::control_panel);

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
        .style(crate::styles::video_surface);

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
    Container::new(crate::subtitle_view::build_subtitle_with_clickable_words(
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
                    .style(crate::styles::ctrl_btn),
            )
            .push(
                Button::new(Text::new("SUBTITLE...").size(11))
                    .padding([4, 14])
                    .on_press_maybe(if has_video {
                        Some(Message::LoadSubtitle)
                    } else {
                        None
                    })
                    .style(crate::styles::ctrl_btn),
            )
            .push(Space::new().width(Length::Fill))
            .push(
                Text::new(crate::text_utils::format_time(position))
                    .size(14)
                    .color(GREEN),
            )
            .push(Text::new(" / ").size(12).color(SILVER))
            .push(
                Text::new(crate::text_utils::format_time(duration))
                    .size(12)
                    .color(SILVER),
            ),
    )
    .width(Length::Fill)
    .style(crate::styles::toolbar_bg)
    .into()
}

// ── Seek bar ────────────────────────────────────────────────────────────

fn build_seek_bar<'a>(position: f64, duration: f64) -> Container<'a, Message> {
    Container::new(
        Slider::new(0.0..=duration.max(0.01), position, Message::Seek)
            .step(0.5)
            .on_release(Message::SeekRelease)
            .width(Length::Fill)
            .style(crate::styles::slider_style),
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
        .push(crate::widgets::skip_back_10_btn())
        .push(crate::widgets::skip_back_5_btn())
        .push(Space::new().width(2))
        .push(crate::widgets::pause_play_btn(is_paused))
        .push(Space::new().width(2))
        .push(crate::widgets::skip_forward_5_btn())
        .push(crate::widgets::skip_forward_10_btn())
        .push(crate::widgets::frame_step_btn())
        .push(Space::new().width(Length::Fill))
        // ── right-side utility cluster ──
        .push(Text::new("Speed:").size(10).color(MUTED))
        .push(
            PickList::new(speeds, Some(app.speed), |s| Message::SetSpeed(s))
                .text_shaping(text::Shaping::Advanced)
                .handle(pick_list::Handle::Arrow { size: None })
                .width(Length::Fixed(72.0))
                .style(crate::styles::pick_list_style),
        )
        .push(Space::new().width(4))
        .push(crate::widgets::loop_btn(is_looping))
        .push(crate::widgets::mute_btn(app.muted))
        .push(
            Slider::new(0.0..=2.0, app.volume, Message::SetVolume)
                .step(0.05)
                .width(Length::Fixed(80.0))
                .style(crate::styles::slider_style),
        )
        .push(Space::new().width(2))
        .push(crate::widgets::content_fit_btn(app.content_fit))
        .push(crate::widgets::fullscreen_btn())
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
        .on_mouse_wheel_scrolled(Message::AdjustVolume)
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
    .style(crate::styles::placeholder)
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
    .style(crate::styles::placeholder)
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
                .style(crate::styles::main_btn),
        )
}

fn build_no_video_icon() -> Container<'static, Message> {
    fn icon_handle() -> &'static iced::widget::image::Handle {
        NO_VIDEO_ICON.get_or_init(|| {
            iced::widget::image::Handle::from_bytes(include_bytes!("../assets/icon.png") as &[u8])
        })
    }
    Container::new(Image::new(icon_handle().clone()).width(Length::Fixed(96.0)))
        .padding(20)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.10, 0.10, 0.10))),
            border: border::color(Color::from_rgba(0.118, 0.843, 0.376, 0.35))
                .width(2.0)
                .rounded(50.0),
            ..Default::default()
        })
}
