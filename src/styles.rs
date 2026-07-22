use iced::{Color, Theme, border, widget::button, widget::container};

pub fn ctrl_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.35, 0.35, 0.4),
        button::Status::Pressed => Color::from_rgb(0.25, 0.25, 0.3),
        _ => Color::from_rgb(0.2, 0.2, 0.25),
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: Color::from_rgb(0.85, 0.85, 0.85),
        border: border::rounded(4),
        ..Default::default()
    }
}

pub fn main_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.3, 0.55, 0.85),
        button::Status::Pressed => Color::from_rgb(0.2, 0.4, 0.7),
        _ => Color::from_rgb(0.2, 0.45, 0.75),
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: Color::WHITE,
        border: border::rounded(4),
        ..Default::default()
    }
}

pub fn active_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.3, 0.7, 0.4),
        button::Status::Pressed => Color::from_rgb(0.2, 0.55, 0.3),
        _ => Color::from_rgb(0.2, 0.6, 0.35),
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: Color::WHITE,
        border: border::rounded(4),
        ..Default::default()
    }
}

pub fn sub_bg(_: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgba(
            0.0, 0.0, 0.0, 0.50,
        ))),
        ..Default::default()
    }
}

pub fn placeholder(_: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgb(0.08, 0.08, 0.1))),
        ..Default::default()
    }
}

pub fn sidebar(_: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgb(0.13, 0.13, 0.17))),
        border: border::color(Color::from_rgb(0.25, 0.25, 0.32))
            .width(1.0)
            .rounded(0),
        ..Default::default()
    }
}

pub fn sidebar_header(_: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgb(0.16, 0.16, 0.22))),
        border: border::color(Color::from_rgb(0.3, 0.3, 0.4))
            .width(1.0)
            .rounded(0),
        ..Default::default()
    }
}

pub fn sidebar_body(_: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgb(0.13, 0.13, 0.17))),
        ..Default::default()
    }
}

pub fn dict_section_card(_: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(Color::from_rgba(
            0.22, 0.22, 0.3, 0.6,
        ))),
        border: border::color(Color::from_rgb(0.3, 0.3, 0.4))
            .width(1.0)
            .rounded(6),
        ..Default::default()
    }
}

pub fn tab_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.22, 0.22, 0.3),
        button::Status::Pressed => Color::from_rgb(0.2, 0.2, 0.27),
        _ => Color::from_rgb(0.16, 0.16, 0.22),
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: Color::from_rgb(0.72, 0.72, 0.78),
        border: border::rounded(0),
        ..Default::default()
    }
}

pub fn active_tab_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.32, 0.32, 0.42),
        button::Status::Pressed => Color::from_rgb(0.28, 0.28, 0.38),
        _ => Color::from_rgb(0.26, 0.26, 0.36),
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: Color::WHITE,
        border: border::rounded(0),
        ..Default::default()
    }
}

/// Transparent button used for clickable history file-name links.
pub fn text_link_btn(_: &Theme, status: button::Status) -> button::Style {
    let text_color = match status {
        button::Status::Hovered => Color::from_rgb(0.55, 0.72, 1.0),
        button::Status::Pressed => Color::from_rgb(0.45, 0.62, 0.9),
        _ => Color::from_rgb(0.75, 0.78, 0.85),
    };
    button::Style {
        background: None,
        text_color,
        border: border::rounded(0),
        ..Default::default()
    }
}

/// Small button for destructive actions (remove history item, clear all).
pub fn danger_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.6, 0.2, 0.2),
        button::Status::Pressed => Color::from_rgb(0.5, 0.15, 0.15),
        _ => Color::from_rgb(0.25, 0.15, 0.15),
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: Color::from_rgb(0.9, 0.55, 0.55),
        border: border::rounded(4),
        ..Default::default()
    }
}
