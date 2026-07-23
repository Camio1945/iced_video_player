use iced::Length;
use iced::widget::{Button, Container, Svg, Text};

use crate::app_state::Message;
use crate::icons;
use crate::styles;

/// Height of every icon SVG and its enclosing button.
const ICON_SIZE: f32 = 22.0;
/// Button height for control-row buttons (pill shape).
const BTN_HEIGHT: f32 = 32.0;
/// Horizontal padding inside control buttons.
const BTN_HORIZ_PAD: u16 = 8;

/// SVG icon sized to ICON_SIZE.
fn icon_btn(icon_data: &[u8]) -> Svg<'_> {
    Svg::new(icons::svg_handle(icon_data))
        .width(Length::Fixed(ICON_SIZE))
        .height(Length::Fixed(ICON_SIZE))
}

// ── Transport controls ──────────────────────────────────────────────────

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

/// Circular green play/pause button — the hero control.
pub(crate) fn pause_play_btn(is_paused: bool) -> Button<'static, Message> {
    let icon = if is_paused { icons::PLAY } else { icons::PAUSE };
    let size = BTN_HEIGHT + 4.0;
    Button::new(icon_btn(icon))
        .padding(0)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
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

// ── Utility controls ────────────────────────────────────────────────────

pub(crate) fn loop_btn<'a>(is_looping: bool) -> Button<'a, Message> {
    let text = Text::new("\u{1F501}").size(14);
    let centered = Container::new(text)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);
    
    Button::new(centered)
        .padding([4, 8])
        .height(Length::Fixed(BTN_HEIGHT))
        .on_press(Message::ToggleLoop)
        .style(if is_looping {
            styles::active_btn
        } else {
            styles::ctrl_btn
        })
}

pub(crate) fn mute_btn<'a>(muted: bool) -> Button<'a, Message> {
    let icon = if muted { "\u{1F507}" } else { "\u{1F50A}" };
    let text = Text::new(icon).size(14);
    let centered = Container::new(text)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);
    
    Button::new(centered)
        .padding([4, 8])
        .height(Length::Fixed(BTN_HEIGHT))
        .on_press(Message::ToggleMute)
        .style(styles::ctrl_btn)
}

pub(crate) fn content_fit_btn<'a>(cf: iced::ContentFit) -> Button<'a, Message> {
    let text = Text::new(format!("{:?}", cf)).size(10);
    let centered = Container::new(text)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);
    
    Button::new(centered)
        .padding([4, 8])
        .height(Length::Fixed(BTN_HEIGHT))
        .on_press(Message::CycleContentFit)
        .style(styles::ctrl_btn)
}

pub(crate) fn fullscreen_btn<'a>() -> Button<'a, Message> {
    Button::new(Text::new("\u{26F6}").size(14))
        .padding([4, 8])
        .height(Length::Fixed(BTN_HEIGHT))
        .on_press(Message::ToggleFullscreen)
        .style(styles::ctrl_btn)
}
