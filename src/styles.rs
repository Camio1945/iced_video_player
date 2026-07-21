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
