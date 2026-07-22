use iced::{
    Color, Element, Length,
    alignment::Horizontal,
    mouse,
    widget::{Column, Container, MouseArea, Row, Text},
};

use crate::app_state::Message;

const MAX_CHARS_PER_LINE: usize = 80;
const LINE_SPACING: f32 = 2.0;
const SUBTITLE_TEXT_COLOR: Color = Color::WHITE;

pub(crate) fn build_subtitle_with_clickable_words(
    text: &str,
    font_size: f32,
) -> Element<'_, Message> {
    let mut column = Column::new()
        .spacing(LINE_SPACING)
        .align_x(Horizontal::Center)
        .width(Length::Fill);
    let source_lines = parse_subtitle_lines(text);
    for display_line in merge_short_lines(&source_lines, MAX_CHARS_PER_LINE) {
        let tokens = tokenize(&display_line);
        for wrapped_line in wrap_into_lines(&tokens, MAX_CHARS_PER_LINE) {
            let mut row = Row::new().spacing(0);
            for token in wrapped_line {
                row = row.push(build_token(token, font_size));
            }
            column = column.push(row);
        }
    }
    Container::new(column)
        .width(Length::Fill)
        .padding(iced::Padding {
            top: 6.0,
            right: 16.0,
            bottom: 4.0,
            left: 16.0,
        })
        .style(|_| iced::widget::container::Style {
            background: Some(iced::Background::Color(Color::from_rgba(
                0.0, 0.0, 0.0, 0.45,
            ))),
            border: iced::border::rounded(6.0),
            ..Default::default()
        })
        .into()
}

/// Splits subtitle text into trimmed, non-empty lines.
fn parse_subtitle_lines(text: &str) -> Vec<&str> {
    text.split('\n')
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect()
}

fn merge_short_lines(lines: &[&str], max_chars: usize) -> Vec<String> {
    let mut merged: Vec<String> = Vec::new();
    for &line in lines {
        if let Some(last) = merged.last_mut() {
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

fn build_token(token: Token, font_size: f32) -> Element<'static, Message> {
    match token {
        Token::Word(w) => {
            let (trimmed, trailing) = split_word_token(&w);
            let mut row = Row::new().spacing(0);
            row = row.push(make_clickable_word(trimmed.to_string(), font_size));
            if !trailing.is_empty() {
                row = row.push(make_plain_text(trailing, font_size));
            }
            row.into()
        }
        Token::Punct(p) => make_plain_text(p, font_size).into(),
    }
}

fn split_word_token(word: &str) -> (&str, String) {
    let trimmed = word.trim_end_matches(|c: char| c == '\'' || c == '-');
    let trailing: String = word.chars().skip(trimmed.chars().count()).collect();
    (trimmed, trailing)
}

/// Plain text with shadow for readability against bright backgrounds.
fn make_plain_text(content: String, size: f32) -> Text<'static, iced::Theme, iced::Renderer> {
    Text::new(content)
        .size(size)
        .color(SUBTITLE_TEXT_COLOR)
        .shaping(iced::widget::text::Shaping::Advanced)
}

fn make_clickable_word(
    word: String,
    size: f32,
) -> MouseArea<'static, Message, iced::Theme, iced::Renderer> {
    let lower = word.to_lowercase();
    MouseArea::new(make_plain_text(word, size))
        .on_press(Message::SearchWord(lower))
        .interaction(mouse::Interaction::Pointer)
}
