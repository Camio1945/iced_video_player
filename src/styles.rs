//! Spotify-inspired "content-first darkness" design system.
//!
//! Colors (`#121212`–`#1f1f1f`) create an immersive theater-like environment
//! where the video frame is the star. Spotify Green (`#1ed760`) is the sole
//! accent, used only for functional highlights (play, active, CTA).
//!
//! Covers every widget: buttons, containers, sliders, pick-lists, scrollbars.

use iced::widget::{pick_list, scrollable, slider};
use iced::{Background, Color, Theme, border, widget::button, widget::container};

// ── Spotify palette ─────────────────────────────────────────────────────

const BG: Color = Color::from_rgb(0.071, 0.071, 0.071); // #121212  deepest
const SURFACE: Color = Color::from_rgb(0.094, 0.094, 0.094); // #181818  cards
const MID: Color = Color::from_rgb(0.122, 0.122, 0.122); // #1f1f1f  interactive
const GREEN: Color = Color::from_rgb(0.118, 0.843, 0.376); // #1ed760  accent
const GREEN_HOVER: Color = Color::from_rgb(0.180, 0.890, 0.450);
const GREEN_PRESSED: Color = Color::from_rgb(0.090, 0.780, 0.310);
const WHITE: Color = Color::WHITE;
const SILVER: Color = Color::from_rgb(0.702, 0.702, 0.702); // #b3b3b3  secondary
const NEAR_WHITE: Color = Color::from_rgb(0.796, 0.796, 0.796); // #cbcbcb
const RED: Color = Color::from_rgb(0.953, 0.447, 0.498); // #f3727f  error
const BORDER: Color = Color::from_rgb(0.302, 0.302, 0.302); // #4d4d4d

const PILL_RADIUS: f32 = 500.0;
const CARD_RADIUS: f32 = 8.0;

// ── Button styles ───────────────────────────────────────────────────────

/// Standard control button: dark pill with white text.
pub fn ctrl_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.18, 0.18, 0.18),
        button::Status::Pressed => Color::from_rgb(0.10, 0.10, 0.10),
        _ => MID,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: WHITE,
        border: border::rounded(PILL_RADIUS),
        ..Default::default()
    }
}

/// Primary CTA: Spotify Green circular button.
pub fn main_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => GREEN_HOVER,
        button::Status::Pressed => GREEN_PRESSED,
        _ => GREEN,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::BLACK,
        border: border::rounded(50.0),
        ..Default::default()
    }
}

/// Active toggle (e.g. looping on): Spotify Green tint.
pub fn active_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.22, 0.90, 0.50),
        button::Status::Pressed => Color::from_rgb(0.10, 0.75, 0.35),
        _ => GREEN,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::BLACK,
        border: border::rounded(PILL_RADIUS),
        ..Default::default()
    }
}

/// Inline text link: transparent bg, silver default, green on hover.
pub fn text_link_btn(_: &Theme, status: button::Status) -> button::Style {
    let text_color = match status {
        button::Status::Hovered => GREEN,
        button::Status::Pressed => Color::from_rgb(0.10, 0.75, 0.35),
        _ => NEAR_WHITE,
    };
    button::Style {
        background: None,
        text_color,
        border: border::rounded(0.0),
        ..Default::default()
    }
}

/// Destructive action: dark pill with red tint.
pub fn danger_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.55, 0.15, 0.15),
        button::Status::Pressed => Color::from_rgb(0.40, 0.10, 0.10),
        _ => Color::from_rgb(0.20, 0.12, 0.12),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: RED,
        border: border::rounded(PILL_RADIUS),
        ..Default::default()
    }
}

/// Sidebar tab: inactive — recedes into the background.
pub fn tab_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.14, 0.14, 0.14),
        button::Status::Pressed => Color::from_rgb(0.10, 0.10, 0.10),
        _ => BG,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: SILVER,
        border: border::rounded(0.0),
        ..Default::default()
    }
}

/// Sidebar tab: active — elevated surface with white text, green bottom accent.
pub fn active_tab_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.20, 0.20, 0.20),
        button::Status::Pressed => Color::from_rgb(0.15, 0.15, 0.15),
        _ => SURFACE,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: WHITE,
        border: border::color(GREEN).width(2.0).rounded(0.0),
        ..Default::default()
    }
}

// ── Slider styles ───────────────────────────────────────────────────────

/// Seek bar & volume slider: green track, round green handle.
pub fn slider_style(_: &Theme, status: slider::Status) -> slider::Style {
    let handle_color = match status {
        slider::Status::Hovered | slider::Status::Dragged => GREEN_HOVER,
        slider::Status::Active => GREEN,
    };
    slider::Style {
        rail: slider::Rail {
            backgrounds: (
                Background::Color(Color::from_rgb(0.25, 0.25, 0.25)),
                Background::Color(GREEN),
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

// ── PickList styles ─────────────────────────────────────────────────────

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

// ── Scrollable styles ───────────────────────────────────────────────────

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
/// `rail_bg`: Some(color) for vertical (visible background), None for horizontal (invisible).
fn build_scrollbar_rail(rail_bg: Option<Color>, scroller_bg: Color) -> scrollable::Rail {
    let bg = rail_bg.map(|c| Background::Color(c));
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

// ── Container / surface styles ──────────────────────────────────────────

/// Placeholder surface for loading / no-video states.
pub fn placeholder(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG)),
        ..Default::default()
    }
}

/// Main sidebar container: deepest surface, subtle right border.
pub fn sidebar(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG)),
        border: border::color(Color::from_rgb(0.18, 0.18, 0.18))
            .width(1.0)
            .rounded(0.0),
        ..Default::default()
    }
}

/// Sidebar header / tab bar: elevated surface.
pub fn sidebar_header(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE)),
        border: border::color(Color::from_rgb(0.20, 0.20, 0.20))
            .width(1.0)
            .rounded(0.0),
        ..Default::default()
    }
}

/// Sidebar scrollable body: deepest surface.
pub fn sidebar_body(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG)),
        ..Default::default()
    }
}

/// Elevated card inside the sidebar (dict sections, settings blocks).
pub fn dict_section_card(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE)),
        border: border::color(BORDER).width(1.0).rounded(CARD_RADIUS),
        ..Default::default()
    }
}

/// Toolbar surface: dark bar separating menu from content.
pub fn toolbar_bg(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE)),
        border: border::color(Color::from_rgb(0.20, 0.20, 0.20))
            .width(1.0)
            .rounded(0.0),
        ..Default::default()
    }
}

/// Bottom control panel: elevated surface like Spotify's now-playing bar.
pub fn control_panel(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(SURFACE)),
        border: border::color(Color::from_rgb(0.20, 0.20, 0.20))
            .width(1.0)
            .rounded(0.0),
        ..Default::default()
    }
}

/// Video area wrapper: pure black surface for the video content.
pub fn video_surface(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::BLACK)),
        ..Default::default()
    }
}
