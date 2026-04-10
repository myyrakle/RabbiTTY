#![allow(unused)]

use iced::widget::{container, scrollable};
use iced::{Background, Border, Color, Shadow, color};

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub background: Color,
    pub surface: Color,
    pub text: Color,
    pub text_secondary: Color,
    pub accent: Color,
    pub success: Color,
    pub error: Color,
}

impl Palette {
    pub const DARK: Self = Self {
        background: color!(0x1e, 0x1e, 0x2e), // Catppuccin Mocha Base
        surface: color!(0x31, 0x32, 0x44),    // Surface0
        text: color!(0xcd, 0xd6, 0xf4),       // Text
        text_secondary: color!(0xa6, 0xad, 0xc8), // Subtext0
        accent: color!(0x89, 0xb4, 0xfa),     // Blue
        success: color!(0xa6, 0xe3, 0xa1),    // Green
        error: color!(0xf3, 0x8b, 0xa8),      // Red
    };
}

pub const SPACING_SMALL: f32 = 4.0;
pub const SPACING_NORMAL: f32 = 8.0;
pub const SPACING_LARGE: f32 = 16.0;

pub const RADIUS_SMALL: f32 = 4.0;
pub const RADIUS_NORMAL: f32 = 8.0;

pub fn scrollbar_style(_theme: &iced::Theme, status: scrollable::Status) -> scrollable::Style {
    let palette = Palette::DARK;
    let scroller_alpha = match status {
        scrollable::Status::Active { .. } => 0.45,
        scrollable::Status::Hovered { .. } => 0.65,
        scrollable::Status::Dragged { .. } => 0.8,
    };

    let rail = |visible: bool| scrollable::Rail {
        background: Some(Background::Color(if visible {
            Color {
                a: 0.08,
                ..palette.surface
            }
        } else {
            Color::TRANSPARENT
        })),
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: Background::Color(Color {
                a: if visible { scroller_alpha } else { 0.0 },
                ..palette.text_secondary
            }),
            border: Border {
                radius: RADIUS_SMALL.into(),
                ..Default::default()
            },
        },
    };

    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail(true),
        horizontal_rail: rail(true),
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: Background::Color(Color::TRANSPARENT),
            border: Border::default(),
            shadow: Shadow::default(),
            icon: Color::TRANSPARENT,
        },
    }
}
