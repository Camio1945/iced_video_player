use crate::text_utils::STOP_WORDS;
use iced::{
    Color, Element, Length,
    alignment::Horizontal,
    mouse,
    widget::{Column, Container, MouseArea, Row, Text},
};

use crate::app_state::Message;

const SUBTITLE_FONT_SIZE: f32 = 20.0;
const MAX_CHARS_PER_LINE: usize = 50;

pub(crate) fn build_subtitle_with_clickable_words(text: &str) -> Element<'_, Message> {
    let tokens = tokenize(text);
    let lines = wrap_into_lines(&tokens, MAX_CHARS_PER_LINE);
    let mut column = Column::new()
        .spacing(4)
        .align_x(Horizontal::Center)
        .width(Length::Fill);
    for line in lines {
        let mut row = Row::new().spacing(0);
        for token in line {
            row = row.push(build_token(token));
        }
        column = column.push(row);
    }
    Container::new(column)
        .width(Length::Fill)
        .padding([8, 16])
        .style(crate::styles::sub_bg)
        .into()
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
            if is_clickable_word(trimmed) {
                row = row.push(make_clickable_word(trimmed.to_string()));
            } else {
                row = row.push(make_plain_text(trimmed.to_string()));
            }
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
