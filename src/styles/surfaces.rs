//! Container and surface style functions for the colorful design system.

use super::{BG, BLUE, CARD_RADIUS, ORANGE, PURPLE, SURFACE};
use iced::{Background, Color, Theme, border, widget::container};

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

/// Bottom control panel: elevated surface.
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

// ── Colorful section-specific containers ───────────────────────────────

/// Dictionary section container - Purple tinted
pub fn dict_container(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb(0.09, 0.08, 0.11))),
        border: border::color(PURPLE).width(1.0).rounded(CARD_RADIUS),
        ..Default::default()
    }
}

/// Settings section container - Blue tinted
pub fn settings_container(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb(0.08, 0.09, 0.11))),
        border: border::color(BLUE).width(1.0).rounded(CARD_RADIUS),
        ..Default::default()
    }
}

/// Playlist section container - Orange tinted
pub fn playlist_container(_: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb(0.11, 0.09, 0.08))),
        border: border::color(ORANGE).width(1.0).rounded(CARD_RADIUS),
        ..Default::default()
    }
}
