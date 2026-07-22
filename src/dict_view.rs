use crate::app_state::{App, Message, SidebarTab};
use crate::dict::DictSection;
use iced::{
    Color, Element, Length,
    alignment::{Horizontal, Vertical},
    widget::{Button, Column, Container, Row, Scrollable, Space, Text},
};

pub(crate) fn build_dictionary_sidebar(app: &App) -> Element<'_, Message> {
    let tabs = build_tab_bar(app.active_tab);

    let body: Element<'_, Message> = match app.active_tab {
        SidebarTab::Dictionary => build_dictionary_content(app),
        SidebarTab::Settings => crate::dict_view_settings::build_settings_content(app),
        SidebarTab::Playlist => build_playlist_content(),
    };

    let body_container = Container::new(
        Scrollable::new(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme, status| iced::widget::scrollable::default(_theme, status)),
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
) -> Button<'static, Message> {
    let is_active = tab == active;
    let style = if is_active {
        crate::styles::active_tab_btn
    } else {
        crate::styles::tab_btn
    };
    Button::new(Text::new(label).size(12).align_x(Horizontal::Center))
        .padding([10, 4])
        .width(Length::Fill)
        .on_press(Message::SwitchSidebarTab(tab))
        .style(style)
}

// ── Dictionary tab ────────────────────────────────────────────────────────

fn build_dictionary_content(app: &App) -> Element<'_, Message> {
    let header = build_dict_header(app);

    let body: Element<'_, Message> = if app.dict_loading {
        build_dict_loading_placeholder()
    } else if app.dict_word.is_empty() {
        build_dict_empty_placeholder()
    } else {
        build_dictionary_body(app)
    };

    Column::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .push(header)
        .push(body)
        .into()
}

fn build_dict_header(app: &App) -> Container<'static, Message> {
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
                .style(crate::styles::ctrl_btn),
        );
    }
    Container::new(header_row)
        .width(Length::Fill)
        .style(crate::styles::sidebar_header)
}

fn build_dict_loading_placeholder<'a>() -> Element<'a, Message> {
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
}

fn build_dict_empty_placeholder<'a>() -> Element<'a, Message> {
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
                .color(Color::from_rgb(0.75, 0.8, 0.9)),
        );
    }

    if !app.dict_sections.is_empty() {
        col = col.push(build_dict_definitions_sections(&app.dict_sections));
    }

    if !app.dict_examples.is_empty() {
        col = col.push(build_dict_examples_section(&app.dict_examples));
    }

    if let Some(err) = &app.dict_error {
        col = col.push(
            Text::new(err)
                .size(12)
                .color(Color::from_rgb(1.0, 0.55, 0.55)),
        );
    }

    Container::new(col).width(Length::Fill).into()
}

fn build_dict_chinese_section<'a>(chinese: &str) -> Container<'a, Message> {
    Container::new(
        Column::new()
            .spacing(2)
            .push(
                Text::new("\u{4E2D}\u{6587}")
                    .size(10)
                    .color(Color::from_rgb(0.75, 0.75, 0.8)),
            )
            .push(
                Text::new(chinese.to_string())
                    .size(20)
                    .color(Color::from_rgb(1.0, 0.82, 0.45)),
            ),
    )
    .width(Length::Fill)
    .padding([10, 10])
    .style(crate::styles::dict_section_card)
}

fn build_dict_definitions_sections<'a>(sections: &[DictSection]) -> Column<'a, Message> {
    let mut outer = Column::new().spacing(10);
    for section in sections {
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
        outer = outer.push(sec_col);
    }
    outer
}

fn build_dict_examples_section<'a>(examples: &[String]) -> Column<'a, Message> {
    let mut ex_col = Column::new().spacing(2);
    ex_col = ex_col.push(
        Text::new("Examples:")
            .size(11)
            .color(Color::from_rgb(0.75, 0.75, 0.8)),
    );
    for ex in examples {
        ex_col = ex_col.push(
            Text::new(format!("\u{2022} {}", ex))
                .size(11)
                .color(Color::from_rgb(0.78, 0.78, 0.85))
                .wrapping(iced::widget::text::Wrapping::Word)
                .width(Length::Fill),
        );
    }
    ex_col
}

// ── Playlist tab ──────────────────────────────────────────────────────────

fn build_playlist_content<'a>() -> Element<'a, Message> {
    Container::new(
        Column::new()
            .spacing(8)
            .padding([24, 14])
            .align_x(Horizontal::Center)
            .push(
                Text::new("\u{1F3B5}")
                    .size(28)
                    .color(Color::from_rgb(0.7, 0.7, 0.75)),
            )
            .push(
                Text::new("Playlist")
                    .size(14)
                    .color(Color::from_rgb(0.85, 0.85, 0.9)),
            )
            .push(
                Text::new("Coming soon...")
                    .size(11)
                    .color(Color::from_rgb(0.6, 0.6, 0.65)),
            ),
    )
    .width(Length::Fill)
    .into()
}
