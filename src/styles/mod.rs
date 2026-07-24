//! Colorful multi-accent design system.
//!
//! Each major section has its own color identity:
//! - Dictionary: Vibrant purple/violet tones
//! - Settings: Cool blue tones
//! - Playlist: Warm orange/amber tones
//! - Video controls: Classic green accent
//! - Main: Gradient-rich colorful elements

mod buttons;
mod surfaces;
mod widgets;

pub use buttons::*;
pub use surfaces::*;
pub use widgets::*;

use iced::Color;

// ── Base palette ─────────────────────────────────────────────────────

pub(crate) const BG: Color = Color::from_rgb(0.071, 0.071, 0.071); // #121212  deepest
pub(crate) const SURFACE: Color = Color::from_rgb(0.094, 0.094, 0.094); // #181818  cards
pub(crate) const MID: Color = Color::from_rgb(0.122, 0.122, 0.122); // #1f1f1f  interactive
pub(crate) const WHITE: Color = Color::WHITE;
pub(crate) const SILVER: Color = Color::from_rgb(0.702, 0.702, 0.702); // #b3b3b3  secondary
pub(crate) const NEAR_WHITE: Color = Color::from_rgb(0.796, 0.796, 0.796); // #cbcbcb
pub(crate) const BORDER: Color = Color::from_rgb(0.302, 0.302, 0.302); // #4d4d4d

// ── Colorful accent palettes ───────────────────────────────────────

// Green (Video controls - classic)
pub(crate) const GREEN: Color = Color::from_rgb(0.118, 0.843, 0.376); // #1ed760
pub(crate) const GREEN_HOVER: Color = Color::from_rgb(0.180, 0.890, 0.450);
pub(crate) const GREEN_PRESSED: Color = Color::from_rgb(0.090, 0.780, 0.310);

// Purple/Violet (Dictionary tab)
pub(crate) const PURPLE: Color = Color::from_rgb(0.612, 0.392, 0.867); // #9c63dd
pub(crate) const PURPLE_HOVER: Color = Color::from_rgb(0.682, 0.459, 0.902);
pub(crate) const PURPLE_PRESSED: Color = Color::from_rgb(0.542, 0.333, 0.831);
pub(crate) const PURPLE_LIGHT: Color = Color::from_rgb(0.733, 0.565, 0.925);

// Blue (Settings tab)
pub(crate) const BLUE: Color = Color::from_rgb(0.255, 0.576, 0.969); // #4192f7
pub(crate) const BLUE_LIGHT: Color = Color::from_rgb(0.588, 0.761, 0.988);

// Orange/Amber (Playlist tab)
pub(crate) const ORANGE: Color = Color::from_rgb(0.976, 0.573, 0.125); // #f99220
pub(crate) const ORANGE_HOVER: Color = Color::from_rgb(0.988, 0.659, 0.216);
pub(crate) const ORANGE_PRESSED: Color = Color::from_rgb(0.965, 0.490, 0.059);
pub(crate) const ORANGE_LIGHT: Color = Color::from_rgb(0.988, 0.757, 0.459);

// Red (Errors)
pub(crate) const RED: Color = Color::from_rgb(0.953, 0.447, 0.498); // #f3727f

// Teal (Main toolbar)
pub(crate) const TEAL: Color = Color::from_rgb(0.255, 0.780, 0.710); // #41c7b5
pub(crate) const TEAL_HOVER: Color = Color::from_rgb(0.337, 0.827, 0.769);
pub(crate) const TEAL_PRESSED: Color = Color::from_rgb(0.176, 0.733, 0.651);

// Pink (Loop toggle)
pub(crate) const PINK_HOVER: Color = Color::from_rgb(0.961, 0.475, 0.643);

pub(crate) const PILL_RADIUS: f32 = 500.0;
pub(crate) const CARD_RADIUS: f32 = 8.0;
