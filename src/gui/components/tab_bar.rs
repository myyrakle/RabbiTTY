use crate::gui::app::Message;
use crate::gui::theme::Palette;
#[cfg(target_os = "windows")]
use iced::widget::mouse_area;
use iced::widget::{button, container, row, scrollable, text};
use iced::{Background, Border, Color, Element, Length, Theme};

pub fn tab_bar<'a>(
    tabs: impl Iterator<Item = (&'a str, usize, bool)>,
    on_add: Message,
    on_settings: Message,
    bar_alpha: f32,
    tab_alpha: f32,
) -> Element<'a, Message> {
    let palette = Palette::DARK;

    let mut tab_elements: Vec<Element<Message>> = Vec::new();

    for (title, index, is_active) in tabs {
        let tab_item = browser_tab(title, index, is_active, tab_alpha);
        tab_elements.push(tab_item);
    }

    let icon_style = move |_theme: &Theme, status: button::Status| button::Style {
        background: Some(Background::Color(match status {
            button::Status::Hovered => Color {
                a: 0.1,
                ..palette.text
            },
            _ => Color::TRANSPARENT,
        })),
        text_color: match status {
            button::Status::Hovered => palette.text,
            _ => palette.text_secondary,
        },
        border: Border {
            radius: 6.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: iced::Shadow::default(),
        snap: true,
    };

    let add_btn = button(text("+").size(13))
        .on_press(on_add)
        .padding([6, 10])
        .style(icon_style);
    let settings_btn = button(text("⚙").size(13))
        .on_press(on_settings)
        .padding([6, 10])
        .style(icon_style);

    let tabs_row = row(tab_elements)
        .spacing(2)
        .align_y(iced::Alignment::Center);
    let tabs_scroll = scrollable(tabs_row)
        .id(crate::gui::app::update::TAB_BAR_SCROLLABLE_ID.clone())
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new().width(3).scroller_width(3),
        ))
        .on_scroll(|viewport: scrollable::Viewport| {
            Message::TabBarScrolled(viewport.absolute_offset().x)
        })
        .style(crate::gui::theme::scrollbar_style)
        .width(Length::Fill)
        .height(Length::Shrink);

    // macOS: left control buttons
    #[cfg(target_os = "macos")]
    let left_padding = 80.0;
    #[cfg(not(target_os = "macos"))]
    let left_padding = 0.0;

    let padding = iced::Padding::new(0.0).left(left_padding);

    // Windows: right control buttons
    #[cfg(target_os = "windows")]
    let window_controls = {
        let hover_subtle = Color {
            a: 0.15,
            ..palette.text
        };
        let hover_close = Color::from_rgb(0.9, 0.2, 0.2);

        let win_style = move |hover_color: Color| {
            move |_theme: &Theme, status: button::Status| button::Style {
                background: match status {
                    button::Status::Hovered => Some(Background::Color(hover_color)),
                    _ => Some(Background::Color(Color::TRANSPARENT)),
                },
                text_color: match status {
                    button::Status::Hovered => Color::WHITE,
                    _ => palette.text_secondary,
                },
                border: Border::default(),
                shadow: iced::Shadow::default(),
                snap: true,
            }
        };

        row![
            button(text("─").size(12))
                .on_press(Message::WindowMinimize)
                .padding([6, 12])
                .style(win_style(hover_subtle)),
            button(text("□").size(12))
                .on_press(Message::WindowMaximize)
                .padding([6, 12])
                .style(win_style(hover_subtle)),
            button(text("✕").size(12))
                .on_press(Message::Exit)
                .padding([6, 12])
                .style(win_style(hover_close)),
        ]
        .spacing(0)
    };

    #[cfg(target_os = "windows")]
    let content = row![tabs_scroll, add_btn, settings_btn, window_controls]
        .spacing(2)
        .align_y(iced::Alignment::Center);

    #[cfg(not(target_os = "windows"))]
    let content = row![tabs_scroll, add_btn, settings_btn]
        .spacing(2)
        .align_y(iced::Alignment::Center);

    let bar_alpha = bar_alpha.clamp(0.0, 1.0);
    let tab_bar_container = container(content)
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(Color {
                a: bar_alpha,
                ..palette.surface
            })),
            border: Border {
                radius: 0.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            ..Default::default()
        })
        .padding(padding)
        .width(Length::Fill);

    #[cfg(target_os = "windows")]
    return mouse_area(tab_bar_container)
        .on_press(Message::WindowDrag)
        .into();

    #[cfg(not(target_os = "windows"))]
    tab_bar_container.into()
}

fn browser_tab<'a>(
    title: &'a str,
    index: usize,
    is_active: bool,
    tab_alpha: f32,
) -> Element<'a, Message> {
    let palette = Palette::DARK;

    const MAX_TITLE_LEN: usize = 24;
    let display_title: std::borrow::Cow<'a, str> = if title.chars().count() > MAX_TITLE_LEN {
        let truncated: String = title.chars().take(MAX_TITLE_LEN - 1).collect();
        format!("{truncated}…").into()
    } else {
        title.into()
    };
    let tab_text = text(display_title).size(12);

    let close_btn = button(text("✕").size(9))
        .on_press(Message::CloseTab(index))
        .padding([2, 5])
        .style(
            move |_theme: &Theme, status: button::Status| button::Style {
                background: match status {
                    button::Status::Hovered => Some(Background::Color(Color {
                        a: 0.15,
                        ..palette.text
                    })),
                    _ => Some(Background::Color(Color::TRANSPARENT)),
                },
                text_color: match status {
                    button::Status::Hovered => palette.text,
                    _ => Color {
                        a: 0.5,
                        ..palette.text_secondary
                    },
                },
                border: Border {
                    radius: 4.0.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                shadow: iced::Shadow::default(),
                snap: true,
            },
        );

    let tab_content = row![tab_text, close_btn]
        .spacing(6)
        .align_y(iced::Alignment::Center);

    let inactive_alpha = tab_alpha.clamp(0.0, 1.0);
    let tab_button = button(tab_content)
        .on_press(Message::TabSelected(index))
        .padding([6, 12])
        .style(move |_theme: &Theme, status: button::Status| {
            if is_active {
                button::Style {
                    background: Some(Background::Color(Color {
                        a: inactive_alpha,
                        ..palette.background
                    })),
                    text_color: palette.text,
                    border: Border::default(),
                    shadow: iced::Shadow::default(),
                    snap: false,
                }
            } else {
                let hovered = matches!(status, button::Status::Hovered);
                button::Style {
                    background: Some(Background::Color(if hovered {
                        Color {
                            a: 0.08,
                            ..palette.text
                        }
                    } else {
                        Color::TRANSPARENT
                    })),
                    text_color: if hovered {
                        palette.text
                    } else {
                        palette.text_secondary
                    },
                    border: Border::default(),
                    shadow: iced::Shadow::default(),
                    snap: false,
                }
            }
        });

    if is_active {
        // Active tab with bottom accent indicator
        let indicator =
            container(text(""))
                .width(Length::Fill)
                .height(2)
                .style(move |_theme: &Theme| container::Style {
                    background: Some(Background::Color(palette.accent)),
                    ..Default::default()
                });

        iced::widget::column![tab_button, indicator]
            .width(Length::Shrink)
            .into()
    } else {
        tab_button.into()
    }
}
