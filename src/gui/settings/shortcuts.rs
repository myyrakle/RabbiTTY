use crate::config::AppConfig;
use crate::gui::app::Message;
use crate::gui::settings::{SettingsDraft, SettingsField, hint_text, input_row, section};
use crate::gui::theme::{Palette, SPACING_NORMAL};
use iced::widget::column;
use iced::{Element, Length};

pub fn view<'a>(
    _config: &'a AppConfig,
    draft: &'a SettingsDraft,
    palette: Palette,
) -> Element<'a, Message> {
    let app_section = section(
        "Application",
        column(vec![
            input_row(
                "New tab",
                &draft.shortcut_new_tab,
                SettingsField::ShortcutNewTab,
                palette,
            ),
            input_row(
                "Close tab",
                &draft.shortcut_close_tab,
                SettingsField::ShortcutCloseTab,
                palette,
            ),
            input_row(
                "Open settings",
                &draft.shortcut_open_settings,
                SettingsField::ShortcutOpenSettings,
                palette,
            ),
            input_row(
                "Next tab",
                &draft.shortcut_next_tab,
                SettingsField::ShortcutNextTab,
                palette,
            ),
            input_row(
                "Previous tab",
                &draft.shortcut_prev_tab,
                SettingsField::ShortcutPrevTab,
                palette,
            ),
            input_row(
                "Quit",
                &draft.shortcut_quit,
                SettingsField::ShortcutQuit,
                palette,
            ),
            hint_text(
                "Format: Command+T, Ctrl+W, Ctrl+PageDown, Command+Comma",
                palette,
            ),
        ])
        .spacing(SPACING_NORMAL)
        .width(Length::Fill)
        .into(),
        palette,
    );

    column(vec![app_section])
        .spacing(SPACING_NORMAL)
        .width(Length::Fill)
        .into()
}
