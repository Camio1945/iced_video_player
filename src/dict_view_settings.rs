use crate::app_state::{App, Message};
use crate::settings::AppSettings;
use iced::{
    Color, Element, Length,
    alignment::{Horizontal, Vertical},
    widget::{Button, Column, Container, Row, Space, Text},
};

// Colorful palette - Blue for Settings
const WHITE: Color = Color::WHITE;
const SILVER: Color = Color::from_rgb(0.702, 0.702, 0.702);
const MUTED: Color = Color::from_rgb(0.55, 0.55, 0.58);
const BLUE_LIGHT: Color = Color::from_rgb(0.588, 0.761, 0.988);
const HEADER: Color = BLUE_LIGHT;

// ── Settings tab ──────────────────────────────────────────────────────────

pub(crate) fn build_settings_content(app: &App) -> Element<'_, Message> {
    Column::new()
        .width(Length::Fill)
        .spacing(16)
        .padding([20, 16])
        .push(build_settings_title())
        .push(build_subtitle_font_size_section(&app.settings))
        .push(build_history_section(&app.settings))
        .push(build_settings_note())
        .into()
}

fn build_settings_title<'a>() -> Text<'a> {
    Text::new("Settings").size(16).color(HEADER)
}

fn build_settings_note<'a>() -> Text<'a> {
    Text::new("Settings are saved automatically.")
        .size(11)
        .color(MUTED)
}

fn build_subtitle_font_size_section(settings: &AppSettings) -> Container<'static, Message> {
    let size = settings.subtitle_font_size;
    let can_increase = size < AppSettings::MAX_FONT_SIZE;
    let can_decrease = size > AppSettings::MIN_FONT_SIZE;
    Container::new(
        Column::new()
            .width(Length::Fill)
            .spacing(10)
            .push(Text::new("Subtitle Font Size").size(13).color(BLUE_LIGHT))
            .push(build_font_size_row(size, can_increase, can_decrease))
            .push(
                Text::new(format!(
                    "Range: {} \u{2013} {} px",
                    AppSettings::MIN_FONT_SIZE as i32,
                    AppSettings::MAX_FONT_SIZE as i32
                ))
                .size(10)
                .color(MUTED),
            ),
    )
    .width(Length::Fill)
    .padding([12, 12])
    .style(crate::styles::settings_container)
}

const FONT_SIZE_BTN_HEIGHT: f32 = 36.0;
const FONT_SIZE_BTN_WIDTH: f32 = 40.0;
const FONT_SIZE_LABEL_WIDTH: f32 = 80.0;

fn build_font_size_row(size: f32, can_increase: bool, can_decrease: bool) -> Row<'static, Message> {
    let minus_btn =
        build_font_size_step_btn("\u{2212}", Message::DecreaseSubtitleFont, can_decrease);
    let plus_btn = build_font_size_step_btn("+", Message::IncreaseSubtitleFont, can_increase);

    let size_label: Element<'_, Message> = Container::new(
        Text::new(format!("{} px", size as i32))
            .size(16)
            .color(WHITE)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fixed(FONT_SIZE_LABEL_WIDTH))
    .height(Length::Fixed(FONT_SIZE_BTN_HEIGHT))
    .into();

    Row::new()
        .spacing(10)
        .align_y(Vertical::Center)
        .height(Length::Fixed(FONT_SIZE_BTN_HEIGHT))
        .push(minus_btn)
        .push(size_label)
        .push(plus_btn)
        .push(Space::new().width(Length::Fill))
}

fn build_font_size_step_btn(
    label: &'static str,
    message: Message,
    enabled: bool,
) -> Button<'static, Message> {
    let mut btn = Button::new(
        Text::new(label)
            .size(18)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fixed(FONT_SIZE_BTN_WIDTH))
    .height(Length::Fixed(FONT_SIZE_BTN_HEIGHT))
    .style(crate::styles::blue_ctrl_btn);
    if enabled {
        btn = btn.on_press(message);
    }
    btn
}

// ── History section (inside Settings) ─────────────────────────────────────

fn build_history_section(settings: &AppSettings) -> Container<'static, Message> {
    let mut col = Column::new()
        .width(Length::Fill)
        .spacing(10)
        .push(build_history_header(settings.history_enabled));

    if settings.history_enabled {
        col = col.push(build_history_max_items_row(settings.max_history_items));
        if settings.recent_files.is_empty() {
            col = col.push(build_history_empty_hint());
        } else {
            col = col.push(build_history_list(&settings.recent_files));
            col = col.push(build_clear_history_button());
        }
    }

    Container::new(col)
        .width(Length::Fill)
        .padding([12, 12])
        .style(crate::styles::settings_container)
}

fn build_history_header(enabled: bool) -> Row<'static, Message> {
    let toggle_text = if enabled {
        "\u{2713} History"
    } else {
        "\u{2717} History"
    };
    Row::new()
        .align_y(Vertical::Center)
        .push(Text::new(toggle_text).size(13).color(BLUE_LIGHT))
        .push(Space::new().width(Length::Fill))
        .push(
            Button::new(Text::new(if enabled { "Disable" } else { "Enable" }).size(11))
                .padding([4, 12])
                .on_press(Message::ToggleHistory)
                .style(crate::styles::blue_ctrl_btn),
        )
}

fn build_history_max_items_row(max_items: usize) -> Row<'static, Message> {
    let can_increase = max_items < AppSettings::MAX_HISTORY_ITEMS;
    let can_decrease = max_items > AppSettings::MIN_HISTORY_ITEMS;

    let dec_btn = build_small_step_btn("\u{2212}", Message::DecreaseHistoryMaxItems, can_decrease);
    let inc_btn = build_small_step_btn("+", Message::IncreaseHistoryMaxItems, can_increase);

    Row::new()
        .spacing(6)
        .align_y(Vertical::Center)
        .push(Text::new("Max Items:").size(11).color(SILVER))
        .push(dec_btn)
        .push(
            Text::new(format!("{}", max_items))
                .size(13)
                .color(WHITE)
                .width(Length::Fixed(36.0))
                .align_x(Horizontal::Center),
        )
        .push(inc_btn)
}

fn build_small_step_btn(
    label: &'static str,
    message: Message,
    enabled: bool,
) -> Button<'static, Message> {
    let mut btn = Button::new(
        Text::new(label)
            .size(14)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fixed(30.0))
    .height(Length::Fixed(28.0))
    .style(crate::styles::blue_ctrl_btn);
    if enabled {
        btn = btn.on_press(message);
    }
    btn
}

fn build_history_empty_hint<'a>() -> Text<'a> {
    Text::new("No recent files yet.").size(11).color(MUTED)
}

fn build_history_list(paths: &[String]) -> Column<'static, Message> {
    let mut col = Column::new()
        .spacing(2)
        .push(Text::new("Recently Opened:").size(11).color(SILVER));
    for path in paths {
        col = col.push(build_history_row(path));
    }
    col
}

fn build_history_row(path: &str) -> Row<'static, Message> {
    let file_name = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
        .to_string();
    Row::new()
        .align_y(Vertical::Center)
        .push(
            Button::new(
                Text::new(file_name)
                    .size(11)
                    .wrapping(iced::widget::text::Wrapping::Word),
            )
            .padding([3, 6])
            .on_press(Message::OpenHistoryItem(path.to_string()))
            .style(crate::styles::text_link_btn)
            .width(Length::Fill),
        )
        .push(
            Button::new(
                Text::new("\u{2715}")
                    .size(10)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center),
            )
            .padding(0)
            .width(Length::Fixed(20.0))
            .height(Length::Fixed(20.0))
            .on_press(Message::RemoveHistoryItem(path.to_string()))
            .style(crate::styles::danger_btn),
        )
}

fn build_clear_history_button<'a>() -> Button<'a, Message> {
    Button::new(
        Container::new(
            Text::new("Clear All History")
                .size(11)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center),
        )
        .width(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center),
    )
    .padding([8, 0])
    .width(Length::Fill)
    .on_press(Message::ClearHistory)
    .style(crate::styles::danger_btn)
}
