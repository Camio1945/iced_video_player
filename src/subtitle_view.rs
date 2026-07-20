use crate::text_utils::STOP_WORDS;
use iced::{
    Color, Element, Length,
    alignment::Horizontal,
    widget::{Container, rich_text, span},
};

use crate::app_state::Message;

pub(crate) fn build_subtitle_with_clickable_words(text: &str) -> Element<'_, Message> {
    let spans: Vec<_> = build_subtitle_spans(text);
    Container::new(
        rich_text(spans)
            .on_link_click(|w: String| Message::SearchWord(w))
            .size(17)
            .align_x(Horizontal::Center)
            .line_height(iced::widget::text::LineHeight::Relative(1.3))
            .width(Length::Fill),
    )
    .width(Length::Fill)
    .padding([8, 12])
    .style(crate::styles::sub_bg)
    .into()
}

fn build_subtitle_spans(text: &str) -> Vec<iced::widget::text::Span<'static, String>> {
    let mut spans: Vec<iced::widget::text::Span<'static, String>> = Vec::new();
    let mut buf = String::new();
    let word_color = Color::WHITE;
    for c in text.chars() {
        let is_word_char = c.is_alphabetic() || c == '\'' || c == '-';
        if is_word_char {
            buf.push(c);
        } else {
            if !buf.is_empty() {
                push_word_span(&mut spans, &buf, word_color);
                buf.clear();
            }
            // Punctuation / whitespace stays as plain text. Collapse
            // multiple whitespace chars into a single space so that we
            // don't get huge gaps from newlines stripped earlier.
            let piece: String = if c.is_whitespace() {
                " ".into()
            } else {
                c.to_string()
            };
            spans.push(span(piece).color(word_color));
        }
    }
    if !buf.is_empty() {
        push_word_span(&mut spans, &buf, word_color);
    }
    spans
}

fn push_word_span(
    spans: &mut Vec<iced::widget::text::Span<'static, String>>,
    word: &str,
    default_color: Color,
) {
    // Trim a trailing apostrophe / dash that may be part of surrounding
    // punctuation (e.g. "don't" inside "don't," should remain attached).
    let trimmed = word.trim_end_matches(|c: char| c == '\'' || c == '-');
    let trailing: String = word.chars().skip(trimmed.chars().count()).collect();

    if is_clickable_word(trimmed) {
        let lower = trimmed.to_lowercase();
        // Make the link invisible: same white color, no underline.
        // The user can still click the word, but it doesn't look
        // visually distinct from non-clickable text.
        spans.push(span(trimmed.to_string()).color(default_color).link(lower));
    } else {
        spans.push(span(trimmed.to_string()).color(default_color));
    }
    if !trailing.is_empty() {
        spans.push(span(trailing).color(default_color));
    }
}

fn is_clickable_word(w: &str) -> bool {
    if w.len() < 3 {
        return false;
    }
    let lower = w.to_lowercase();
    if !lower.chars().all(|c| c.is_alphabetic()) {
        return false;
    }
    !STOP_WORDS.contains(&lower.as_str())
}
