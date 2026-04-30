pub mod button;
pub mod container;
pub mod tab_bar;

use crate::gui::theme::Palette;

pub fn button_primary(
    text: &str,
    palette: Palette,
) -> iced::widget::button::Button<'_, crate::gui::app::Message> {
    button::primary(text, palette)
}

pub fn button_secondary(
    text: &str,
    palette: Palette,
) -> iced::widget::button::Button<'_, crate::gui::app::Message> {
    button::secondary(text, palette)
}

pub use container::panel;
pub use tab_bar::tab_bar;
