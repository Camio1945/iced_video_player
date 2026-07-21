use iced::{
    Color, Element, Length,
    alignment::Horizontal,
    mouse,
    widget::{Column, Container, MouseArea, Row, Text},
};

use crate::app_state::Message;

const SUBTITLE_FONT_SIZE: f32 = 20.0;
const MAX_CHARS_PER_LINE: usize = 80;
const LINE_SPACING: f32 = 2.0;

pub(crate) fn build_subtitle_with_clickable_words(text: &str) -> Element<'_, Message> {
    let mut column = Column::new()
        .spacing(LINE_SPACING)
        .align_x(Horizontal::Center)
        .width(Length::Fill);

    // Collect non-empty source lines so we can try to merge short ones.
    let source_lines: Vec<&str> = text
        .split('\n')
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    for display_line in merge_short_lines(&source_lines, MAX_CHARS_PER_LINE) {
        let tokens = tokenize(&display_line);
        for wrapped_line in wrap_into_lines(&tokens, MAX_CHARS_PER_LINE) {
            let mut row = Row::new().spacing(0);
            for token in wrapped_line {
                row = row.push(build_token(token));
            }
            column = column.push(row);
        }
    }

    Container::new(column)
        .width(Length::Fill)
        .padding([8, 16])
        .style(crate::styles::sub_bg)
        .into()
}

/// Greedily merge consecutive source lines into display lines.
/// Two adjacent source lines are combined (with a space) as long as the
/// result fits within `max_chars`.  This avoids forcing every source
/// line break into a separate visual row when the subtitle background is
/// wide enough to hold them on one line.
fn merge_short_lines(lines: &[&str], max_chars: usize) -> Vec<String> {
    let mut merged: Vec<String> = Vec::new();
    for &line in lines {
        if let Some(last) = merged.last_mut() {
            // +1 for the space that would join the two lines
            if last.chars().count() + 1 + line.chars().count() <= max_chars {
                last.push(' ');
                last.push_str(line);
                continue;
            }
        }
        merged.push(line.to_string());
    }
    merged
}

#[derive(Debug, Clone)]
enum Token {
    Word(String),
    Punct(String),
}

fn tokenize(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut buf = String::new();
    for c in text.chars() {
        let is_word_char = c.is_alphabetic() || c == '\'' || c == '-';
        if is_word_char {
            buf.push(c);
        } else {
            if !buf.is_empty() {
                tokens.push(Token::Word(buf.clone()));
                buf.clear();
            }
            tokens.push(Token::Punct(c.to_string()));
        }
    }
    if !buf.is_empty() {
        tokens.push(Token::Word(buf));
    }
    tokens
}

fn wrap_into_lines(tokens: &[Token], max_chars: usize) -> Vec<Vec<Token>> {
    let mut lines: Vec<Vec<Token>> = Vec::new();
    let mut current_line: Vec<Token> = Vec::new();
    let mut current_len = 0;
    for token in tokens {
        let token_len = match token {
            Token::Word(w) => w.chars().count(),
            Token::Punct(p) => p.chars().count(),
        };
        // A single token larger than max_chars is forced onto its own line
        // rather than dropped.
        if current_len + token_len > max_chars && !current_line.is_empty() {
            lines.push(current_line.clone());
            current_line.clear();
            current_len = 0;
        }
        current_line.push(token.clone());
        current_len += token_len;
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    lines
}

fn build_token(token: Token) -> Element<'static, Message> {
    match token {
        Token::Word(w) => {
            let (trimmed, trailing) = split_word_token(&w);
            let mut row = Row::new().spacing(0);
            // Every word is clickable, including short ones ("it", "me", "go"),
            // words with apostrophes ("It's", "Don't"), and common words
            // ("the"). The user expects to be able to look up any word.
            row = row.push(make_clickable_word(trimmed.to_string()));
            if !trailing.is_empty() {
                row = row.push(make_plain_text(trailing));
            }
            row.into()
        }
        Token::Punct(p) => make_plain_text(p).into(),
    }
}

/// Split a word token into its alphabetic core and any trailing apostrophes/dashes
/// that should remain non-clickable (e.g. the trailing "s" in "it's,").
fn split_word_token(word: &str) -> (&str, String) {
    let trimmed = word.trim_end_matches(|c: char| c == '\'' || c == '-');
    let trailing: String = word.chars().skip(trimmed.chars().count()).collect();
    (trimmed, trailing)
}

fn make_plain_text(content: String) -> Text<'static, iced::Theme, iced::Renderer> {
    Text::new(content)
        .size(SUBTITLE_FONT_SIZE)
        .color(Color::WHITE)
}

fn make_clickable_word(word: String) -> MouseArea<'static, Message, iced::Theme, iced::Renderer> {
    let lower = word.to_lowercase();
    MouseArea::new(make_plain_text(word))
        .on_press(Message::SearchWord(lower))
        .interaction(mouse::Interaction::Pointer)
}
