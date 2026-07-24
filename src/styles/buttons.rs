//! Button style functions for the colorful design system.

use super::{
    BG, BLUE, BLUE_LIGHT, GREEN, GREEN_HOVER, GREEN_PRESSED, NEAR_WHITE, ORANGE, ORANGE_HOVER,
    ORANGE_LIGHT, ORANGE_PRESSED, PILL_RADIUS, PINK_HOVER, PURPLE, PURPLE_HOVER, PURPLE_LIGHT,
    PURPLE_PRESSED, RED, SILVER, TEAL, TEAL_HOVER, TEAL_PRESSED, WHITE,
};
use iced::{Background, Color, Theme, border, widget::button};

/// Primary CTA: Green circular button.
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

/// Active toggle (e.g. looping on): Green tint.
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

// ── Colorful active tab buttons ───────────────────────────────────────

/// Dictionary tab active - Purple theme
pub fn active_dict_tab_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.20, 0.15, 0.25),
        button::Status::Pressed => Color::from_rgb(0.15, 0.10, 0.18),
        _ => Color::from_rgb(0.15, 0.12, 0.20),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: PURPLE_LIGHT,
        border: border::color(PURPLE).width(2.0).rounded(0.0),
        ..Default::default()
    }
}

/// Settings tab active - Blue theme
pub fn active_settings_tab_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.15, 0.18, 0.25),
        button::Status::Pressed => Color::from_rgb(0.10, 0.12, 0.18),
        _ => Color::from_rgb(0.12, 0.15, 0.20),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: BLUE_LIGHT,
        border: border::color(BLUE).width(2.0).rounded(0.0),
        ..Default::default()
    }
}

/// Playlist tab active - Orange theme
pub fn active_playlist_tab_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.22, 0.16, 0.12),
        button::Status::Pressed => Color::from_rgb(0.16, 0.11, 0.08),
        _ => Color::from_rgb(0.18, 0.13, 0.10),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: ORANGE_LIGHT,
        border: border::color(ORANGE).width(2.0).rounded(0.0),
        ..Default::default()
    }
}

// ── Colorful pill buttons for sections ───────────────────────────────

/// Purple primary button for Dictionary section
pub fn purple_btn(_: &Theme, status: button::Status) -> button::Style {
    let (bg, fg) = match status {
        button::Status::Hovered => (PURPLE_HOVER, WHITE),
        button::Status::Pressed => (PURPLE_PRESSED, WHITE),
        _ => (PURPLE, WHITE),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: fg,
        border: border::rounded(PILL_RADIUS),
        ..Default::default()
    }
}

/// Orange primary button for Playlist section
pub fn orange_btn(_: &Theme, status: button::Status) -> button::Style {
    let (bg, fg) = match status {
        button::Status::Hovered => (ORANGE_HOVER, Color::from_rgb(0.10, 0.10, 0.10)),
        button::Status::Pressed => (ORANGE_PRESSED, Color::from_rgb(0.08, 0.08, 0.08)),
        _ => (ORANGE, Color::from_rgb(0.12, 0.12, 0.12)),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: fg,
        border: border::rounded(PILL_RADIUS),
        ..Default::default()
    }
}

/// Teal button for toolbar actions
pub fn teal_btn(_: &Theme, status: button::Status) -> button::Style {
    let (bg, fg) = match status {
        button::Status::Hovered => (TEAL_HOVER, Color::from_rgb(0.10, 0.10, 0.10)),
        button::Status::Pressed => (TEAL_PRESSED, Color::from_rgb(0.08, 0.08, 0.08)),
        _ => (TEAL, Color::from_rgb(0.12, 0.12, 0.12)),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: fg,
        border: border::rounded(PILL_RADIUS),
        ..Default::default()
    }
}

// ── Colorful small control buttons ───────────────────────────────

/// Blue control button for Settings section
pub fn blue_ctrl_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.22, 0.38, 0.55),
        button::Status::Pressed => Color::from_rgb(0.15, 0.30, 0.45),
        _ => Color::from_rgb(0.18, 0.33, 0.50),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: BLUE_LIGHT,
        border: border::rounded(PILL_RADIUS),
        ..Default::default()
    }
}

// ── Colorful control-bar buttons ─────────────────────────────────────

/// Rewind / skip-back: Blue-tinted control button.
pub fn rewind_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.20, 0.35, 0.52),
        button::Status::Pressed => Color::from_rgb(0.14, 0.27, 0.42),
        _ => Color::from_rgb(0.16, 0.30, 0.45),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: BLUE_LIGHT,
        border: border::rounded(PILL_RADIUS),
        ..Default::default()
    }
}

/// Skip-forward: Orange-tinted control button.
pub fn forward_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.50, 0.32, 0.18),
        button::Status::Pressed => Color::from_rgb(0.40, 0.25, 0.14),
        _ => Color::from_rgb(0.44, 0.28, 0.16),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: ORANGE_LIGHT,
        border: border::rounded(PILL_RADIUS),
        ..Default::default()
    }
}

/// Frame-step: Purple-tinted control button.
pub fn step_btn(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.34, 0.22, 0.48),
        button::Status::Pressed => Color::from_rgb(0.27, 0.17, 0.39),
        _ => Color::from_rgb(0.30, 0.19, 0.43),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: PURPLE_LIGHT,
        border: border::rounded(PILL_RADIUS),
        ..Default::default()
    }
}

/// Loop toggle (inactive): Pink-tinted control button.
pub fn loop_btn_style(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.48, 0.18, 0.30),
        button::Status::Pressed => Color::from_rgb(0.38, 0.14, 0.24),
        _ => Color::from_rgb(0.42, 0.16, 0.27),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: PINK_HOVER,
        border: border::rounded(PILL_RADIUS),
        ..Default::default()
    }
}

/// Mute toggle: Teal-tinted control button.
pub fn mute_btn_style(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.18, 0.42, 0.38),
        button::Status::Pressed => Color::from_rgb(0.14, 0.34, 0.30),
        _ => Color::from_rgb(0.16, 0.37, 0.33),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: TEAL_HOVER,
        border: border::rounded(PILL_RADIUS),
        ..Default::default()
    }
}

/// Muted state: Red-tinted to signal audio is off.
pub fn muted_btn_style(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.50, 0.16, 0.18),
        button::Status::Pressed => Color::from_rgb(0.40, 0.12, 0.14),
        _ => Color::from_rgb(0.36, 0.14, 0.15),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: RED,
        border: border::rounded(PILL_RADIUS),
        ..Default::default()
    }
}

/// Content-fit cycle: Blue-tinted control button.
pub fn fit_btn_style(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.20, 0.36, 0.52),
        button::Status::Pressed => Color::from_rgb(0.15, 0.28, 0.42),
        _ => Color::from_rgb(0.17, 0.31, 0.46),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: BLUE_LIGHT,
        border: border::rounded(PILL_RADIUS),
        ..Default::default()
    }
}

/// Fullscreen toggle: Purple-tinted control button.
pub fn fullscreen_btn_style(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb(0.33, 0.21, 0.47),
        button::Status::Pressed => Color::from_rgb(0.26, 0.16, 0.38),
        _ => Color::from_rgb(0.29, 0.18, 0.42),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: PURPLE_LIGHT,
        border: border::rounded(PILL_RADIUS),
        ..Default::default()
    }
}
