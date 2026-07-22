/// Inline SVG icons drawn on a uniform 24×24 viewBox.
/// Every shape is centered at viewBox center (12, 12) so that when each
/// SVG is rendered at the same size, the icon centers line up exactly.

pub const SKIP_BACK_10: &[u8] = br#"<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
  <path d="M5 12L11 4V20Z M13 12L19 4V20Z" fill="currentColor"/>
</svg>"#;

pub const SKIP_BACK_5: &[u8] = br#"<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
  <path d="M5 12L15 4V20Z" fill="currentColor"/>
</svg>"#;

pub const PLAY: &[u8] = br#"<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
  <path d="M8 4V20L21 12Z" fill="currentColor"/>
</svg>"#;

pub const PAUSE: &[u8] = br#"<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
  <path d="M7 4H11V20H7Z M13 4H17V20H13Z" fill="currentColor"/>
</svg>"#;

pub const SKIP_FORWARD_5: &[u8] = br#"<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
  <path d="M19 12L9 4V20Z" fill="currentColor"/>
</svg>"#;

pub const SKIP_FORWARD_10: &[u8] = br#"<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
  <path d="M11 12L5 4V20Z M19 12L13 4V20Z" fill="currentColor"/>
</svg>"#;

pub const FRAME_STEP: &[u8] = br#"<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
  <path d="M3 4H7V20H3Z M11 4V20L21 12Z" fill="currentColor"/>
</svg>"#;

/// Build an Iced SVG handle from inline data.
pub fn svg_handle(data: &[u8]) -> iced::widget::svg::Handle {
    iced::widget::svg::Handle::from_memory(Vec::from(data))
}
