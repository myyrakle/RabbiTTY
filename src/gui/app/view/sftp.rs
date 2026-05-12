//! SFTP drawer rendering.

use crate::gui::app::Message;
use crate::gui::sftp::{self, SftpDrawerState};
use crate::gui::theme::{Palette, RADIUS_NORMAL, RADIUS_SMALL, SPACING_NORMAL, SPACING_SMALL};
use crate::ssh::sftp::Entry;
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

const DRAWER_TOP_BORDER: f32 = 1.0;
const ROW_HEIGHT: f32 = 26.0;

pub fn drawer<'a>(
    state: &'a SftpDrawerState,
    tab_id: u64,
    palette: Palette,
) -> Element<'a, Message> {
    let header = drawer_header(state, palette);
    let body = drawer_body(state, tab_id, palette);

    container(
        column(vec![header, body])
            .spacing(SPACING_SMALL)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .padding([SPACING_NORMAL, SPACING_NORMAL])
    .width(Length::Fill)
    .height(Length::Fill)
    .style(move |_theme: &Theme| container::Style {
        background: Some(Background::Color(Color {
            a: 0.96,
            ..palette.surface
        })),
        border: Border {
            radius: 0.0.into(),
            width: DRAWER_TOP_BORDER,
            color: Color {
                a: 0.2,
                ..palette.text
            },
        },
        ..Default::default()
    })
    .into()
}

fn drawer_header<'a>(state: &'a SftpDrawerState, palette: Palette) -> Element<'a, Message> {
    let path_label = text(if state.current_path.is_empty() {
        "."
    } else {
        state.current_path.as_str()
    })
    .size(13)
    .color(palette.text);

    let status = if state.opening {
        Some("Opening…")
    } else if state.loading {
        Some("Loading…")
    } else {
        None
    };
    let status_text: Element<Message> = match status {
        Some(s) => text(s).size(12).color(palette.text_secondary).into(),
        None => Space::new()
            .width(Length::Shrink)
            .height(Length::Shrink)
            .into(),
    };

    let icon_style = move |_theme: &Theme, status: button::Status| button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => Color {
                a: 0.12,
                ..palette.text
            },
            _ => Color::TRANSPARENT,
        })),
        text_color: palette.text_secondary,
        border: Border {
            radius: RADIUS_NORMAL.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: iced::Shadow::default(),
        snap: true,
    };
    let refresh_btn = button(text("\u{27F3}").size(13))
        .on_press(Message::SftpRefresh)
        .padding([4, 10])
        .style(icon_style);
    let close_btn = button(text("\u{2715}").size(12))
        .on_press(Message::SftpToggleDrawer)
        .padding([4, 10])
        .style(icon_style);

    row![
        text("SFTP").size(14).color(palette.accent),
        path_label,
        Space::new().width(Length::Fill),
        status_text,
        refresh_btn,
        close_btn,
    ]
    .spacing(SPACING_NORMAL)
    .align_y(Alignment::Center)
    .width(Length::Fill)
    .into()
}

fn drawer_body<'a>(
    state: &'a SftpDrawerState,
    tab_id: u64,
    palette: Palette,
) -> Element<'a, Message> {
    if let Some(error) = state.error.as_deref() {
        return container(text(error).size(12).color(palette.error))
            .padding([SPACING_NORMAL, 0.0])
            .into();
    }

    if state.opening || (state.loading && state.entries.is_empty()) {
        return container(text("Loading…").size(12).color(palette.text_secondary))
            .padding([SPACING_NORMAL, 0.0])
            .width(Length::Fill)
            .into();
    }

    let mut rows: Vec<Element<Message>> = Vec::with_capacity(state.entries.len() + 1);

    if let Some(parent) = sftp::parent_path(&state.current_path) {
        rows.push(parent_row(tab_id, parent, palette));
    }
    for entry in &state.entries {
        rows.push(entry_row(state, tab_id, entry, palette));
    }

    scrollable(
        column(rows)
            .spacing(2)
            .width(Length::Fill)
            .padding([SPACING_SMALL, 0.0]),
    )
    .style(crate::gui::theme::scrollbar_style(palette))
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn parent_row<'a>(tab_id: u64, parent: String, palette: Palette) -> Element<'a, Message> {
    let row_style = row_style_factory(palette);
    button(
        row![
            text("\u{2191}").size(13).color(palette.text_secondary),
            text("..").size(13).color(palette.text),
        ]
        .spacing(SPACING_NORMAL)
        .align_y(Alignment::Center),
    )
    .on_press(Message::SftpNavigate {
        tab_id,
        path: parent,
    })
    .padding([4.0, SPACING_NORMAL])
    .width(Length::Fill)
    .style(row_style)
    .into()
}

fn entry_row<'a>(
    state: &'a SftpDrawerState,
    tab_id: u64,
    entry: &'a Entry,
    palette: Palette,
) -> Element<'a, Message> {
    let icon = if entry.is_dir {
        "\u{1F4C1}"
    } else if entry.is_symlink {
        "\u{2937}"
    } else {
        "\u{2024}"
    };
    let name_color = if entry.is_dir {
        palette.accent
    } else {
        palette.text
    };
    let size_text = if entry.is_dir {
        "—".to_string()
    } else {
        humanize_bytes(entry.size)
    };

    let row_content = row![
        text(icon).size(13).color(palette.text_secondary),
        text(entry.name.as_str()).size(13).color(name_color),
        Space::new().width(Length::Fill),
        text(size_text).size(12).color(palette.text_secondary),
    ]
    .spacing(SPACING_NORMAL)
    .align_y(Alignment::Center);

    let target_path = if entry.is_dir {
        Some(sftp::join_path(&state.current_path, &entry.name))
    } else {
        None
    };

    let row_style = row_style_factory(palette);

    let mut btn = button(row_content)
        .padding([4.0, SPACING_NORMAL])
        .width(Length::Fill)
        .style(row_style)
        .height(Length::Fixed(ROW_HEIGHT));
    if let Some(path) = target_path {
        btn = btn.on_press(Message::SftpNavigate { tab_id, path });
    }
    btn.into()
}

fn row_style_factory(palette: Palette) -> impl Fn(&Theme, button::Status) -> button::Style + Copy {
    move |_theme: &Theme, status: button::Status| button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => Color {
                a: 0.08,
                ..palette.text
            },
            _ => Color::TRANSPARENT,
        })),
        text_color: palette.text,
        border: Border {
            radius: RADIUS_SMALL.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

fn humanize_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.1} {}", size, UNITS[unit])
    }
}
