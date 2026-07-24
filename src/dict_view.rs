use crate::app_state::{App, Message, SidebarTab};
use crate::dict::DictSection;
use iced::{
    Color, Element, Length,
    alignment::Horizontal,
    widget::{Button, Column, Container, Row, Scrollable, Space, Stack, Text},
};

// Colorful palette for Dictionary section
const SILVER: Color = Color::from_rgb(0.702, 0.702, 0.702); // #b3b3b3
const NEAR_WHITE: Color = Color::from_rgb(0.796, 0.796, 0.796); // #cbcbcb
const MUTED: Color = Color::from_rgb(0.55, 0.55, 0.58);
const ERROR_RED: Color = Color::from_rgb(0.953, 0.447, 0.498); // #f3727f
const PURPLE_LIGHT: Color = Color::from_rgb(0.733, 0.565, 0.925);

pub(crate) fn build_dictionary_sidebar(app: &App) -> Element<'_, Message> {
    let tabs = build_tab_bar(app.active_tab);

    let body: Element<'_, Message> = match app.active_tab {
        SidebarTab::Dictionary => build_dictionary_content(app),
        SidebarTab::Settings => crate::dict_view_settings::build_settings_content(app),
        SidebarTab::Playlist => build_playlist_content(app),
    };

    let body_container = Container::new(
        Scrollable::new(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(crate::styles::scrollbar_style),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(crate::styles::sidebar_body);

    Container::new(
        Column::new()
            .width(Length::Fill)
            .height(Length::Fill)
            .push(tabs)
            .push(body_container),
    )
    .width(Length::Fixed(360.0))
    .height(Length::Fill)
    .style(crate::styles::sidebar)
    .into()
}

fn build_tab_bar(active: SidebarTab) -> Container<'static, Message> {
    Container::new(
        Row::new()
            .width(Length::Fill)
            .spacing(0)
            .push(build_tab_btn("Dictionary", SidebarTab::Dictionary, active))
            .push(build_tab_btn("Settings", SidebarTab::Settings, active))
            .push(build_tab_btn("Playlist", SidebarTab::Playlist, active)),
    )
    .width(Length::Fill)
    .style(crate::styles::sidebar_header)
}

fn build_tab_btn(
    label: &'static str,
    tab: SidebarTab,
    active: SidebarTab,
) -> Element<'static, Message> {
    let is_active = tab == active;
    let btn = Button::new(Text::new(label).size(12).align_x(Horizontal::Center))
        .padding([10, 4])
        .width(Length::Fill)
        .on_press(Message::SwitchSidebarTab(tab))
        .style(if is_active {
            match tab {
                SidebarTab::Dictionary => crate::styles::active_dict_tab_btn,
                SidebarTab::Settings => crate::styles::active_settings_tab_btn,
                SidebarTab::Playlist => crate::styles::active_playlist_tab_btn,
            }
        } else {
            crate::styles::tab_btn
        });

    if is_active {
        Stack::new()
            .width(Length::Fill)
            .clip(true)
            .push(btn)
            .push(active_tab_cover(tab))
            .into()
    } else {
        btn.into()
    }
}

/// Overlay a colored strip over the bottom 2px of the button's
/// border, matching the color scheme for each tab.
fn active_tab_cover(tab: SidebarTab) -> Column<'static, Message> {
    let bg_color = match tab {
        SidebarTab::Dictionary => iced::Color::from_rgb(0.15, 0.12, 0.20),
        SidebarTab::Settings => iced::Color::from_rgb(0.12, 0.15, 0.20),
        SidebarTab::Playlist => iced::Color::from_rgb(0.18, 0.13, 0.10),
    };
    Column::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .push(Space::new().width(Length::Fill).height(Length::Fill))
        .push(
            Container::new(Space::new())
                .width(Length::Fill)
                .height(Length::Fixed(2.0))
                .style(move |_| iced::widget::container::Style {
                    background: Some(iced::Background::Color(bg_color)),
                    ..Default::default()
                }),
        )
}

// ── Dictionary tab ────────────────────────────────────────────────────────

/// Build the body of the Dictionary tab. When the dictionary webview is alive
/// it covers this entire area as a native child HWND, so we emit a transparent
/// placeholder of equal size so the layout engine still reserves the space.
fn build_dictionary_content(app: &App) -> Element<'_, Message> {
    let placeholder: Element<'_, Message> = if crate::dict_webview::has_webview() {
        // Transparent placeholder – the child webview is drawn on top by the OS.
        Space::new().width(Length::Fill).height(Length::Fill).into()
    } else if app.dict_loading {
        build_dict_loading_placeholder()
    } else if app.dict_word.is_empty() {
        build_dict_empty_placeholder()
    } else if !app.dict_chinese.is_empty()
        || !app.dict_sections.is_empty()
        || !app.dict_examples.is_empty()
    {
        build_dictionary_body(app)
    } else {
        build_dict_loading_placeholder()
    };

    placeholder.into()
}

fn build_dict_loading_placeholder<'a>() -> Element<'a, Message> {
    Container::new(
        Column::new()
            .spacing(6)
            .align_x(Horizontal::Center)
            .padding([24, 12])
            .push(Text::new("\u{23F3}").size(22).color(PURPLE_LIGHT))
            .push(Text::new("Looking up...").size(12).color(SILVER)),
    )
    .width(Length::Fill)
    .into()
}

fn build_dict_empty_placeholder<'a>() -> Element<'a, Message> {
    Container::new(
        Column::new()
            .spacing(8)
            .padding([24, 14])
            .align_x(Horizontal::Center)
            .push(Text::new("\u{1F448}").size(28).color(PURPLE_LIGHT))
            .push(
                Text::new("Click a word in the subtitle")
                    .size(13)
                    .color(PURPLE_LIGHT),
            )
            .push(
                Text::new("The Chinese meaning will appear here.")
                    .size(11)
                    .color(MUTED),
            ),
    )
    .width(Length::Fill)
    .into()
}

fn build_dictionary_body(app: &App) -> Element<'_, Message> {
    let mut col = Column::new().spacing(10).padding([12, 12]);

    if !app.dict_chinese.is_empty() {
        col = col.push(build_dict_chinese_section(&app.dict_chinese));
    }

    if !app.dict_phonetic.is_empty() {
        col = col.push(
            Text::new(format!("/{}/", app.dict_phonetic))
                .size(13)
                .color(SILVER),
        );
    }

    if !app.dict_sections.is_empty() {
        col = col.push(build_dict_definitions_sections(&app.dict_sections));
    }

    if !app.dict_examples.is_empty() {
        col = col.push(build_dict_examples_section(&app.dict_examples));
    }

    if let Some(err) = &app.dict_error {
        col = col.push(Text::new(err).size(12).color(ERROR_RED));
    }

    Container::new(col).width(Length::Fill).into()
}

fn build_dict_chinese_section<'a>(chinese: &str) -> Container<'a, Message> {
    Container::new(
        Column::new()
            .spacing(2)
            .push(Text::new("\u{4E2D}\u{6587}").size(10).color(MUTED))
            .push(Text::new(chinese.to_string()).size(20).color(PURPLE_LIGHT)),
    )
    .width(Length::Fill)
    .padding([10, 10])
    .style(crate::styles::dict_container)
}

fn build_dict_definitions_sections<'a>(sections: &[DictSection]) -> Column<'a, Message> {
    let mut outer = Column::new().spacing(10);
    // lighter purple tint for part-of-speech labels
    let pos_purple = Color::from_rgb(0.70, 0.52, 0.85);
    for section in sections {
        let mut sec_col = Column::new().spacing(3);
        sec_col = sec_col.push(
            Text::new(format!("[{}]", section.part_of_speech))
                .size(11)
                .color(pos_purple),
        );
        for (i, (def, example)) in section.definitions.iter().enumerate() {
            sec_col = sec_col.push(
                Text::new(format!("{}. {}", i + 1, def))
                    .size(12)
                    .color(NEAR_WHITE)
                    .wrapping(iced::widget::text::Wrapping::Word)
                    .width(Length::Fill),
            );
            if let Some(ex) = example {
                sec_col = sec_col.push(
                    Text::new(format!("    \u{201C}{}\u{201D}", ex))
                        .size(11)
                        .color(SILVER)
                        .wrapping(iced::widget::text::Wrapping::Word)
                        .width(Length::Fill),
                );
            }
        }
        outer = outer.push(sec_col);
    }
    outer
}

fn build_dict_examples_section<'a>(examples: &[String]) -> Column<'a, Message> {
    let mut ex_col = Column::new().spacing(2);
    ex_col = ex_col.push(Text::new("Examples:").size(11).color(MUTED));
    for ex in examples {
        ex_col = ex_col.push(
            Text::new(format!("\u{2022} {}", ex))
                .size(11)
                .color(NEAR_WHITE)
                .wrapping(iced::widget::text::Wrapping::Word)
                .width(Length::Fill),
        );
    }
    ex_col
}

// ── Playlist tab ──────────────────────────────────────────────────────────

fn build_playlist_content(app: &App) -> Element<'_, Message> {
    app.build_playlist_content()
}
