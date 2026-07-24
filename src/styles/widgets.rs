//! Slider, pick-list, and scrollbar style functions for the colorful design system.

use super::{BG, BORDER, GREEN, GREEN_HOVER, MID, NEAR_WHITE, PILL_RADIUS, SILVER};
use iced::{
    Background, Color, Theme, border,
    widget::container,
    widget::{pick_list, scrollable, slider},
};

/// Seek bar & volume slider: green track, round green handle.
pub fn slider_style(_: &Theme, status: slider::Status) -> slider::Style {
    let handle_color = match status {
        slider::Status::Hovered | slider::Status::Dragged => GREEN_HOVER,
        slider::Status::Active => GREEN,
    };
    slider::Style {
        rail: slider::Rail {
            backgrounds: (
                Background::Color(GREEN),
                Background::Color(Color::from_rgb(0.25, 0.25, 0.25)),
            ),
            width: 4.0,
            border: border::rounded(2.0),
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 6.0 },
            background: Background::Color(handle_color),
            border_width: 2.0,
            border_color: MID,
        },
    }
}

/// Speed picker: dark pill surface, silver text, green accent handle.
pub fn pick_list_style(_: &Theme, status: pick_list::Status) -> pick_list::Style {
    let handle_color = match status {
        pick_list::Status::Opened { .. } => GREEN,
        _ => SILVER,
    };
    pick_list::Style {
        text_color: NEAR_WHITE,
        placeholder_color: SILVER,
        handle_color,
        background: Background::Color(MID),
        border: border::color(BORDER).width(1.0).rounded(PILL_RADIUS),
    }
}

/// Sidebar scrollbar: thin, subtle, dark scroller.
pub fn scrollbar_style(_: &Theme, status: scrollable::Status) -> scrollable::Style {
    let is_hovered = matches!(
        status,
        scrollable::Status::Hovered {
            is_vertical_scrollbar_hovered: true,
            ..
        }
    );
    let scroller_bg = if is_hovered {
        Color::from_rgb(0.40, 0.40, 0.40)
    } else {
        Color::from_rgb(0.25, 0.25, 0.25)
    };
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: build_scrollbar_rail(Some(BG), scroller_bg),
        horizontal_rail: build_scrollbar_rail(None, scroller_bg),
        gap: None,
        auto_scroll: build_auto_scroll(),
    }
}

/// Constructs a single scrollbar rail (vertical or horizontal).
fn build_scrollbar_rail(rail_bg: Option<Color>, scroller_bg: Color) -> scrollable::Rail {
    let bg = rail_bg.map(Background::Color);
    scrollable::Rail {
        background: bg,
        border: border::rounded(2.0),
        scroller: scrollable::Scroller {
            background: Background::Color(scroller_bg),
            border: border::rounded(2.0),
        },
    }
}

/// Dark semi-transparent overlay shown while auto-scrolling.
fn build_auto_scroll() -> scrollable::AutoScroll {
    scrollable::AutoScroll {
        background: Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.6)),
        border: border::rounded(8.0),
        shadow: iced::Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
            offset: iced::Vector { x: 0.0, y: 2.0 },
            blur_radius: 8.0,
        },
        icon: GREEN,
    }
}
