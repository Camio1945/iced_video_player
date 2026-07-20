mod app_handlers;
mod app_state;
mod dict;
mod styles;
mod text_utils;
mod widgets;

use app_state::{App, Message, VideoState};
use iced::{
    self, Color, Element, Length, Subscription, Task, Theme,
    alignment::{Horizontal, Vertical},
    keyboard,
    widget::{
        Button, Column, Container, PickList, Row, Scrollable, Slider, Space, Text, container,
        pick_list, rich_text, span, text,
    },
    window,
};
use iced_video_player::{Video, VideoPlayer};

// ── update ────────────────────────────────────────────────────────────────

fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::TogglePause => app.handle_toggle_pause(),
        Message::Seek(s) => app.handle_seek(s),
        Message::SeekRelease => app.handle_seek_release(),
        Message::SkipBack(s) => app.handle_skip_back(s),
        Message::SkipForward(s) => app.handle_skip_forward(s),
        Message::FrameStepForward => app.handle_frame_step_forward(),
        Message::FrameStepBackward => app.handle_frame_step_backward(),
        Message::EndOfStream => Task::none(),
        Message::NewFrame => app.handle_new_frame(),
        Message::PlaybackError(err) => {
            eprintln!("Playback error: {}", err);
            Task::none()
        }
        Message::OpenFile => app.handle_open_file(),
        Message::FileOpened(r) => app.handle_file_opened(r),
        Message::LoadSubtitle => app.handle_load_subtitle(),
        Message::SubtitleText(t) => app.handle_subtitle_text(t),
        Message::SearchWord(w) => app.handle_search_word(w),
        Message::DictionaryResult(r) => app.handle_dictionary_result(r),
        Message::CloseDictionary => app.handle_close_dictionary(),
        Message::ToggleLoop => app.handle_toggle_loop(),
        Message::ToggleMute => app.handle_toggle_mute(),
        Message::SetVolume(v) => app.handle_set_volume(v),
        Message::SetSpeed(s) => app.handle_set_speed(s),
        Message::ToggleFullscreen => app.handle_toggle_fullscreen(),
        Message::CycleContentFit => app.handle_cycle_content_fit(),
        Message::WindowOpened(id) => app.handle_window_opened(id),
        Message::KeyboardEvent(e) => app.handle_keyboard_event(e),
    }
}

// ── view ──────────────────────────────────────────────────────────────────

fn view(app: &App) -> Element<'_, Message> {
    let is_paused = app.with_video(|v: &Video| v.paused()).unwrap_or(true);
    let is_looping = app
        .with_video(|v: &Video| v.looping())
        .unwrap_or(app.looping);
    let has_video = matches!(&app.video, VideoState::Ready(_));

    let player_area = Column::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(0)
        .push(
            Container::new(build_video_area(app))
                .width(Length::Fill)
                .height(Length::Fill),
        );
    let player_area = if app.subtitle_text.is_empty() {
        player_area
    } else {
        player_area.push(build_subtitle_with_clickable_words(&app.subtitle_text))
    };
    let player_area = player_area
        .push(build_seek_bar(app.position, app.video_duration()))
        .push(build_controls(has_video, is_paused, is_looping, app));

    let main_row = Row::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(0)
        .push(
            Container::new(player_area)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .push(build_dictionary_sidebar(app));

    let layout = Column::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .push(build_toolbar(has_video, app.position, app.video_duration()))
        .push(main_row);

    Container::new(layout)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(0.1, 0.1, 0.12))),
            ..Default::default()
        })
        .into()
}

fn build_toolbar<'a>(has_video: bool, position: f64, duration: f64) -> Row<'a, Message> {
    Row::new()
        .spacing(4)
        .padding(4)
        .align_y(Vertical::Center)
        .push(
            Button::new(Text::new("Open").size(12))
                .padding([4, 8])
                .on_press(Message::OpenFile)
                .style(styles::ctrl_btn),
        )
        .push(
            Button::new(Text::new("Subtitle...").size(12))
                .padding([4, 8])
                .on_press_maybe(if has_video {
                    Some(Message::LoadSubtitle)
                } else {
                    None
                })
                .style(styles::ctrl_btn),
        )
        .push(Space::new().width(Length::Fill))
        .push(
            text(format!(
                "{} / {}",
                text_utils::format_time(position),
                text_utils::format_time(duration)
            ))
            .size(12),
        )
}

fn build_seek_bar<'a>(position: f64, duration: f64) -> Container<'a, Message> {
    Container::new(
        Slider::new(0.0..=duration.max(0.01), position, Message::Seek)
            .step(0.5)
            .on_release(Message::SeekRelease)
            .width(Length::Fill),
    )
    .padding([0, 8])
}

fn build_controls<'a>(
    has_video: bool,
    is_paused: bool,
    is_looping: bool,
    app: &App,
) -> Row<'a, Message> {
    let speeds = vec![0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0, 4.0];
    Row::new()
        .spacing(6)
        .padding([4, 8])
        .align_y(Vertical::Center)
        .push(widgets::skip_back_10_btn(has_video))
        .push(widgets::skip_back_5_btn(has_video))
        .push(widgets::pause_play_btn(has_video, is_paused))
        .push(widgets::skip_forward_5_btn(has_video))
        .push(widgets::skip_forward_10_btn(has_video))
        .push(widgets::frame_step_btn(has_video && is_paused))
        .push(Space::new().width(Length::Fill))
        .push(Text::new("Speed:").size(11))
        .push(
            PickList::new(speeds, Some(app.speed), |s| Message::SetSpeed(s))
                .text_shaping(text::Shaping::Advanced)
                .handle(pick_list::Handle::Arrow { size: None })
                .width(Length::Fixed(80.0)),
        )
        .push(widgets::loop_btn(is_looping))
        .push(widgets::mute_btn(app.muted))
        .push(
            Slider::new(0.0..=2.0, app.volume, Message::SetVolume)
                .step(0.05)
                .width(Length::Fixed(90.0)),
        )
        .push(widgets::content_fit_btn(app.content_fit))
        .push(widgets::fullscreen_btn())
}

fn build_video_area(app: &App) -> Element<'_, Message> {
    match &app.video {
        VideoState::Ready(video) => build_video_player_widget(video, app.content_fit),
        VideoState::Loading(p) => build_loading_screen(p),
        VideoState::NoVideo => build_no_video_screen(),
    }
}

fn build_video_player_widget<'a>(
    video: &'a Video,
    content_fit: iced::ContentFit,
) -> Element<'a, Message> {
    VideoPlayer::new(video)
        .width(Length::Fill)
        .height(Length::Fill)
        .content_fit(content_fit)
        .on_end_of_stream(Message::EndOfStream)
        .on_new_frame(Message::NewFrame)
        .on_subtitle_text(|t: Option<String>| Message::SubtitleText(t.unwrap_or_default()))
        .on_error(|e: &glib::Error| Message::PlaybackError(e.to_string()))
        .into()
}

fn build_loading_screen(path: &str) -> Element<'_, Message> {
    Container::new(
        Column::new()
            .spacing(10)
            .align_x(Horizontal::Center)
            .push(Text::new("Loading video...").size(18))
            .push(Text::new(path).size(12)),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(styles::placeholder)
    .into()
}

fn build_no_video_screen() -> Element<'static, Message> {
    Container::new(
        Column::new()
            .spacing(12)
            .align_x(Horizontal::Center)
            .push(Text::new("\u{1F3AC}").size(48))
            .push(Text::new("No video loaded").size(18))
            .push(Text::new("Click \"Open\" or press O to load a video").size(14))
            .push(
                Button::new(Text::new("Open Video File"))
                    .on_press(Message::OpenFile)
                    .padding([8, 20]),
            ),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(styles::placeholder)
    .into()
}

// ── subtitle (clickable words) ───────────────────────────────────────────

use crate::text_utils::STOP_WORDS;

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

fn build_subtitle_with_clickable_words(text: &str) -> Element<'_, Message> {
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
    .style(styles::sub_bg)
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
            let piece: String = if c.is_whitespace() { " ".into() } else { c.to_string() };
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
        spans.push(
            span(trimmed.to_string())
                .color(default_color)
                .link(lower),
        );
    } else {
        spans.push(span(trimmed.to_string()).color(default_color));
    }
    if !trailing.is_empty() {
        spans.push(span(trailing).color(default_color));
    }
}

// ── dictionary side panel ────────────────────────────────────────────────

fn build_dictionary_sidebar(app: &App) -> Element<'_, Message> {
    let title_text = if app.dict_word.is_empty() {
        Text::new("\u{1F4D6}  Dictionary").size(14)
    } else {
        Text::new(format!("\u{1F4D6}  {}", app.dict_word))
            .size(15)
            .color(Color::from_rgb(0.95, 0.9, 0.6))
    };

    let mut header_row = Row::new()
        .align_y(Vertical::Center)
        .padding([8, 10])
        .push(title_text)
        .push(Space::new().width(Length::Fill));
    if !app.dict_word.is_empty() {
        header_row = header_row.push(
            Button::new(Text::new("\u{2715}").size(13))
                .padding([1, 6])
                .on_press(Message::CloseDictionary)
                .style(styles::ctrl_btn),
        );
    }
    let header = Container::new(header_row)
        .width(Length::Fill)
        .style(styles::sidebar_header);

    let body: Element<'_, Message> = if app.dict_loading {
        Container::new(
            Column::new()
                .spacing(6)
                .align_x(Horizontal::Center)
                .padding([24, 12])
                .push(Text::new("\u{23F3}").size(22))
                .push(Text::new("Looking up...").size(12)),
        )
        .width(Length::Fill)
        .into()
    } else if app.dict_word.is_empty() {
        Container::new(
            Column::new()
                .spacing(8)
                .padding([24, 14])
                .align_x(Horizontal::Center)
                .push(
                    Text::new("\u{1F448}")
                        .size(28)
                        .color(Color::from_rgb(0.7, 0.7, 0.75)),
                )
                .push(
                    Text::new("Click a word in the subtitle")
                        .size(13)
                        .color(Color::from_rgb(0.85, 0.85, 0.9)),
                )
                .push(
                    Text::new("The Chinese meaning will appear here.")
                        .size(11)
                        .color(Color::from_rgb(0.6, 0.6, 0.65)),
                ),
        )
        .width(Length::Fill)
        .into()
    } else {
        build_dictionary_body(app)
    };

    let body_container = Container::new(
        Scrollable::new(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme, status| iced::widget::scrollable::default(_theme, status)),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(styles::sidebar_body);

    Container::new(
        Column::new()
            .width(Length::Fill)
            .height(Length::Fill)
            .push(header)
            .push(body_container),
    )
    .width(Length::Fixed(360.0))
    .height(Length::Fill)
    .style(styles::sidebar)
    .into()
}

fn build_dictionary_body(app: &App) -> Element<'_, Message> {
    let mut col = Column::new()
        .spacing(10)
        .padding([12, 12]);

    // Chinese translation (primary result)
    if !app.dict_chinese.is_empty() {
        col = col.push(
            Container::new(
                Column::new()
                    .spacing(2)
                    .push(
                        Text::new("\u{4E2D}\u{6587}")
                            .size(10)
                            .color(Color::from_rgb(0.75, 0.75, 0.8)),
                    )
                    .push(
                        Text::new(app.dict_chinese.clone())
                            .size(20)
                            .color(Color::from_rgb(1.0, 0.82, 0.45)),
                    ),
            )
            .width(Length::Fill)
            .padding([10, 10])
            .style(styles::dict_section_card),
        );
    }

    // Phonetic
    if !app.dict_phonetic.is_empty() {
        col = col.push(
            Text::new(format!("/{}/", app.dict_phonetic))
                .size(13)
                .color(Color::from_rgb(0.75, 0.8, 0.9)),
        );
    }

    // English definitions grouped by part of speech
    for section in &app.dict_sections {
        let mut sec_col = Column::new().spacing(3);
        sec_col = sec_col.push(
            Text::new(format!("[{}]", section.part_of_speech))
                .size(11)
                .color(Color::from_rgb(0.55, 0.85, 0.6)),
        );
        for (i, (def, example)) in section.definitions.iter().enumerate() {
            sec_col = sec_col.push(
                Text::new(format!("{}. {}", i + 1, def))
                    .size(12)
                    .color(Color::from_rgb(0.88, 0.88, 0.92))
                    .wrapping(iced::widget::text::Wrapping::Word)
                    .width(Length::Fill),
            );
            if let Some(ex) = example {
                sec_col = sec_col.push(
                    Text::new(format!("    \u{201C}{}\u{201D}", ex))
                        .size(11)
                        .color(Color::from_rgb(0.62, 0.62, 0.7))
                        .wrapping(iced::widget::text::Wrapping::Word)
                        .width(Length::Fill),
                );
            }
        }
        col = col.push(sec_col);
    }

    // Examples (if any are present that aren't already shown inline)
    if !app.dict_examples.is_empty() {
        let mut ex_col = Column::new().spacing(2);
        ex_col = ex_col.push(
            Text::new("Examples:")
                .size(11)
                .color(Color::from_rgb(0.75, 0.75, 0.8)),
        );
        for ex in &app.dict_examples {
            ex_col = ex_col.push(
                Text::new(format!("\u{2022} {}", ex))
                    .size(11)
                    .color(Color::from_rgb(0.78, 0.78, 0.85))
                    .wrapping(iced::widget::text::Wrapping::Word)
                    .width(Length::Fill),
            );
        }
        col = col.push(ex_col);
    }

    // Error message
    if let Some(err) = &app.dict_error {
        col = col.push(
            Text::new(err)
                .size(12)
                .color(Color::from_rgb(1.0, 0.55, 0.55)),
        );
    }

    Container::new(col)
        .width(Length::Fill)
        .into()
}

// ── subscription ──────────────────────────────────────────────────────────

fn subscription(_app: &App) -> Subscription<Message> {
    let keyboard_sub = keyboard::listen().map(Message::KeyboardEvent);
    let window_sub = window::open_events().map(Message::WindowOpened);
    Subscription::batch([keyboard_sub, window_sub])
}

// ── Entry point ───────────────────────────────────────────────────────────

fn main() -> iced::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (video_arg, subtitle_arg) = parse_cli_args(&args);

    let boot = create_boot_closure(video_arg, subtitle_arg);

    iced::application(boot, update, view)
        .title(|app: &App| {
            let base = "Video Player";
            match &app.current_file_path {
                Some(p) => {
                    let name = std::path::Path::new(p)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(p);
                    format!("{} - {}", name, base)
                }
                None => base.to_string(),
            }
        })
        .subscription(subscription)
        .theme(|_: &App| Theme::Dark)
        .window(window::Settings {
            size: iced::Size::new(1280.0, 760.0),
            min_size: Some(iced::Size::new(800.0, 480.0)),
            ..Default::default()
        })
        .run()
}

fn parse_cli_args(args: &[String]) -> (Option<String>, Option<String>) {
    match args.len() {
        0 => (None, None),
        1 => (Some(args[0].clone()), None),
        _ => (Some(args[0].clone()), Some(args[1].clone())),
    }
}

fn create_boot_closure(
    video_arg: Option<String>,
    subtitle_arg: Option<String>,
) -> impl Fn() -> (App, Task<Message>) {
    move || {
        let mut app = App::default();
        let mut initial_task = Task::none();

        if let Some(ref path) = video_arg {
            let path_str = std::path::Path::new(path).display().to_string();
            app.video = VideoState::Loading(path_str.clone());
            app.current_file_path = Some(path_str);

            if let Some(ref sp) = subtitle_arg {
                let sub_path = std::path::Path::new(sp).to_path_buf();
                app.pending_subtitle = Some(sub_path);
            }

            let url = url::Url::from_file_path(path)
                .unwrap_or_else(|_| url::Url::parse(&format!("file:///{}", path)).unwrap());
            let path_owned = path.clone();
            initial_task = Task::perform(
                async move {
                    let path_buf = std::path::PathBuf::from(&path_owned);
                    match Video::new(&url) {
                        Ok(_) => Ok(path_buf.display().to_string()),
                        Err(e) => Err(format!("Failed to open: {}", e)),
                    }
                },
                Message::FileOpened,
            );
        }
        (app, initial_task)
    }
}
