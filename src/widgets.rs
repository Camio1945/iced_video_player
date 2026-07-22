use iced::Length;
use iced::widget::{Button, Svg, Text};

use crate::app_state::Message;
use crate::icons;
use crate::styles;

/// Height of every icon SVG and its enclosing button, ensuring
/// the visual center of each icon lands on the same horizontal line.
const ICON_SIZE: f32 = 22.0;
const BTN_HEIGHT: f32 = 30.0;
const BTN_HORIZ_PAD: u16 = 6;

/// Small helper: SVG icon sized to ICON_SIZE, wrapped in a button
/// with fixed height and horizontal-only padding.
fn icon_btn(icon_data: &[u8]) -> Svg<'_> {
    Svg::new(icons::svg_handle(icon_data))
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE))
}

pub(crate) fn skip_back_10_btn() -> Button<'static, Message> {
    Button::new(icon_btn(icons::SKIP_BACK_10))
        .padding([0, BTN_HORIZ_PAD])
        .height(Length::Fixed(BTN_HEIGHT))
        .on_press(Message::SkipBack(10))
        .style(styles::ctrl_btn)
}

pub(crate) fn skip_back_5_btn() -> Button<'static, Message> {
    Button::new(icon_btn(icons::SKIP_BACK_5))
        .padding([0, BTN_HORIZ_PAD])
        .height(Length::Fixed(BTN_HEIGHT))
        .on_press(Message::SkipBack(5))
        .style(styles::ctrl_btn)
}

pub(crate) fn pause_play_btn(is_paused: bool) -> Button<'static, Message> {
    let icon = if is_paused { icons::PLAY } else { icons::PAUSE };
    Button::new(icon_btn(icon))
        .padding([0, BTN_HORIZ_PAD])
        .height(Length::Fixed(BTN_HEIGHT))
        .on_press(Message::TogglePause)
        .style(styles::main_btn)
}

pub(crate) fn skip_forward_5_btn() -> Button<'static, Message> {
    Button::new(icon_btn(icons::SKIP_FORWARD_5))
        .padding([0, BTN_HORIZ_PAD])
        .height(Length::Fixed(BTN_HEIGHT))
        .on_press(Message::SkipForward(5))
        .style(styles::ctrl_btn)
}

pub(crate) fn skip_forward_10_btn() -> Button<'static, Message> {
    Button::new(icon_btn(icons::SKIP_FORWARD_10))
        .padding([0, BTN_HORIZ_PAD])
        .height(Length::Fixed(BTN_HEIGHT))
        .on_press(Message::SkipForward(10))
        .style(styles::ctrl_btn)
}

pub(crate) fn frame_step_btn() -> Button<'static, Message> {
    Button::new(icon_btn(icons::FRAME_STEP))
        .padding([0, BTN_HORIZ_PAD])
        .height(Length::Fixed(BTN_HEIGHT))
        .on_press(Message::FrameStepForward)
        .style(styles::ctrl_btn)
}

pub(crate) fn loop_btn<'a>(is_looping: bool) -> Button<'a, Message> {
    Button::new(Text::new("\u{1F501}").size(14))
        .padding([4, 6])
        .on_press(Message::ToggleLoop)
        .style(if is_looping {
            styles::active_btn
        } else {
            styles::ctrl_btn
        })
}

pub(crate) fn mute_btn<'a>(muted: bool) -> Button<'a, Message> {
    Button::new(Text::new(if muted { "\u{1F507}" } else { "\u{1F50A}" }).size(14))
        .padding([4, 6])
        .on_press(Message::ToggleMute)
        .style(styles::ctrl_btn)
}

pub(crate) fn content_fit_btn<'a>(cf: iced::ContentFit) -> Button<'a, Message> {
    Button::new(Text::new(format!("{:?}", cf)).size(10))
        .padding([4, 6])
        .on_press(Message::CycleContentFit)
        .style(styles::ctrl_btn)
}

pub(crate) fn fullscreen_btn<'a>() -> Button<'a, Message> {
    Button::new(Text::new("\u{26F6}").size(14))
        .padding([4, 6])
        .on_press(Message::ToggleFullscreen)
        .style(styles::ctrl_btn)
}
