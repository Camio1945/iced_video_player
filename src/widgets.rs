use iced::widget::{Button, Text};

use crate::app_state::Message;
use crate::styles;

pub(crate) fn skip_back_10_btn<'a>(has_video: bool) -> Button<'a, Message> {
    Button::new(Text::new("\u{23EA}").size(14))
        .padding([4, 6])
        .on_press_maybe(if has_video {
            Some(Message::SkipBack(10))
        } else {
            None
        })
        .style(styles::ctrl_btn)
}

pub(crate) fn skip_back_5_btn<'a>(has_video: bool) -> Button<'a, Message> {
    Button::new(Text::new("\u{23F4}").size(14))
        .padding([4, 6])
        .on_press_maybe(if has_video {
            Some(Message::SkipBack(5))
        } else {
            None
        })
        .style(styles::ctrl_btn)
}

pub(crate) fn pause_play_btn<'a>(has_video: bool, is_paused: bool) -> Button<'a, Message> {
    Button::new(Text::new(if is_paused { "\u{25B6}" } else { "\u{23F8}" }).size(18))
        .padding([4, 10])
        .on_press_maybe(if has_video {
            Some(Message::TogglePause)
        } else {
            None
        })
        .style(styles::main_btn)
}

pub(crate) fn skip_forward_5_btn<'a>(has_video: bool) -> Button<'a, Message> {
    Button::new(Text::new("\u{23F5}").size(14))
        .padding([4, 6])
        .on_press_maybe(if has_video {
            Some(Message::SkipForward(5))
        } else {
            None
        })
        .style(styles::ctrl_btn)
}

pub(crate) fn skip_forward_10_btn<'a>(has_video: bool) -> Button<'a, Message> {
    Button::new(Text::new("\u{23E9}").size(14))
        .padding([4, 6])
        .on_press_maybe(if has_video {
            Some(Message::SkipForward(10))
        } else {
            None
        })
        .style(styles::ctrl_btn)
}

pub(crate) fn frame_step_btn<'a>(enabled: bool) -> Button<'a, Message> {
    Button::new(Text::new("|\u{25B6}").size(12))
        .padding([4, 6])
        .on_press_maybe(if enabled {
            Some(Message::FrameStepForward)
        } else {
            None
        })
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
