use crate::gui::theme::Palette;
use iced::widget::button;
use iced::{Background, Border, Color, Shadow, Theme};

const RADIUS: f32 = 6.0;

pub fn primary(text: &str) -> button::Button<'_, crate::gui::app::Message> {
    button(iced::widget::text(text).size(13))
        .padding([7, 16])
        .style(|_theme: &Theme, status: button::Status| {
            let palette = Palette::DARK;
            let bg = match status {
                button::Status::Hovered => Color {
                    r: palette.accent.r * 1.1,
                    g: palette.accent.g * 1.1,
                    b: palette.accent.b * 1.1,
                    a: 1.0,
                },
                button::Status::Pressed => Color {
                    a: 0.85,
                    ..palette.accent
                },
                button::Status::Disabled => palette.surface,
                _ => palette.accent,
            };
            let text_color = match status {
                button::Status::Disabled => palette.text_secondary,
                _ => palette.background,
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color,
                border: Border {
                    radius: RADIUS.into(),
                    width: 0.0,
                    color: Color::TRANSPARENT,
                },
                shadow: Shadow::default(),
                snap: true,
            }
        })
}

pub fn secondary(text: &str) -> button::Button<'_, crate::gui::app::Message> {
    button(iced::widget::text(text).size(13))
        .padding([7, 16])
        .style(|_theme: &Theme, status: button::Status| {
            let palette = Palette::DARK;
            let (bg, border_alpha) = match status {
                button::Status::Hovered => (
                    Color {
                        a: 0.12,
                        ..palette.text
                    },
                    0.2,
                ),
                button::Status::Pressed => (
                    Color {
                        a: 0.08,
                        ..palette.text
                    },
                    0.25,
                ),
                button::Status::Disabled => (
                    Color {
                        a: 0.04,
                        ..palette.text
                    },
                    0.05,
                ),
                _ => (Color::TRANSPARENT, 0.1),
            };
            let text_color = match status {
                button::Status::Disabled => palette.text_secondary,
                _ => palette.text,
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color,
                border: Border {
                    radius: RADIUS.into(),
                    width: 1.0,
                    color: Color {
                        a: border_alpha,
                        ..palette.text
                    },
                },
                shadow: Shadow::default(),
                snap: true,
            }
        })
}
