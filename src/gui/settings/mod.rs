use crate::config::{AppConfig, AppConfigUpdates, SshProfile, parse_hex_color};
use crate::gui::app::Message;
use crate::gui::theme::{Palette, RADIUS_NORMAL, RADIUS_SMALL, SPACING_NORMAL};
use iced::widget::{column, container, row, rule, text, text_input, toggler};
use iced::{Alignment, Background, Border, Color, Element, Length};
use std::fmt;

pub mod shortcuts;
pub mod ssh;
pub mod terminal;
pub mod theme;
pub mod ui;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalFontOption {
    pub label: String,
    pub value: String,
    pub monospaced: bool,
}

impl fmt::Display for TerminalFontOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsField {
    UiWindowWidth,
    UiWindowHeight,
    TerminalFontSelection,
    TerminalFontSize,
    TerminalPaddingX,
    TerminalPaddingY,
    ThemeColorScheme,
    ThemeForeground,
    ThemeBackground,
    ThemeCursor,
    ThemeBackgroundOpacity,
    #[allow(dead_code)]
    ThemeMacosBlurRadius,
    ShortcutNewTab,
    ShortcutCloseTab,
    ShortcutOpenSettings,
    ShortcutNextTab,
    ShortcutPrevTab,
    ShortcutQuit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SshProfileField {
    Name,
    Host,
    Port,
    User,
    IdentityFile,
    Password,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsCategory {
    Ui,
    Terminal,
    Theme,
    Shortcuts,
    Ssh,
}

impl SettingsCategory {
    pub const ALL: [Self; 5] = [
        Self::Ui,
        Self::Terminal,
        Self::Theme,
        Self::Shortcuts,
        Self::Ssh,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Ui => "UI",
            Self::Terminal => "Terminal",
            Self::Theme => "Theme",
            Self::Shortcuts => "Shortcuts",
            Self::Ssh => "SSH",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Ui => "◫",
            Self::Terminal => "▣",
            Self::Theme => "◑",
            Self::Shortcuts => "⌘",
            Self::Ssh => "⇄",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SshProfileDraft {
    pub name: String,
    pub host: String,
    pub port: String,
    pub user: String,
    pub identity_file: String,
    pub password: String,
}

impl SshProfileDraft {
    pub fn from_profile(profile: &SshProfile) -> Self {
        Self {
            name: profile.name.clone(),
            host: profile.host.clone(),
            port: profile.port.to_string(),
            user: profile.user.clone(),
            identity_file: profile.identity_file.clone().unwrap_or_default(),
            password: profile.password.clone().unwrap_or_default(),
        }
    }

    pub fn to_profile(&self) -> Option<SshProfile> {
        let host = self.host.trim();
        if host.is_empty() {
            return None;
        }
        Some(SshProfile {
            name: self.name.trim().to_string(),
            host: host.to_string(),
            port: self.port.trim().parse().unwrap_or(22),
            user: self.user.trim().to_string(),
            identity_file: {
                let v = self.identity_file.trim();
                if v.is_empty() {
                    None
                } else {
                    Some(v.to_string())
                }
            },
            password: {
                let v = self.password.trim();
                if v.is_empty() {
                    None
                } else {
                    Some(v.to_string())
                }
            },
        })
    }
}

#[derive(Debug, Clone)]
pub struct SettingsDraft {
    pub window_width: String,
    pub window_height: String,
    pub terminal_font_selection: String,
    pub terminal_font_size: String,
    pub terminal_padding_x: String,
    pub terminal_padding_y: String,
    pub color_scheme: String,
    pub foreground: String,
    pub background: String,
    pub cursor: String,
    pub background_opacity: String,
    pub blur_enabled: bool,
    pub macos_blur_radius: String,
    pub shortcut_new_tab: String,
    pub shortcut_close_tab: String,
    pub shortcut_open_settings: String,
    pub shortcut_next_tab: String,
    pub shortcut_prev_tab: String,
    pub shortcut_quit: String,
    pub ssh_profiles: Vec<SshProfileDraft>,
}

impl SettingsDraft {
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            window_width: format!("{:.0}", config.ui.window_width),
            window_height: format!("{:.0}", config.ui.window_height),
            terminal_font_selection: config.terminal.font_selection.clone().unwrap_or_default(),
            terminal_font_size: format!("{:.1}", config.terminal.font_size),
            terminal_padding_x: format!("{:.1}", config.terminal.padding_x),
            terminal_padding_y: format!("{:.1}", config.terminal.padding_y),
            color_scheme: config.theme.color_scheme.clone(),
            foreground: format_rgb(config.theme.foreground),
            background: format_rgb(config.theme.background),
            cursor: format_rgb(config.theme.cursor),
            background_opacity: format!("{:.2}", config.theme.background_opacity),
            blur_enabled: config.theme.blur_enabled,
            macos_blur_radius: format!("{}", config.theme.macos_blur_radius),
            shortcut_new_tab: config.shortcuts.new_tab.clone(),
            shortcut_close_tab: config.shortcuts.close_tab.clone(),
            shortcut_open_settings: config.shortcuts.open_settings.clone(),
            shortcut_next_tab: config.shortcuts.next_tab.clone(),
            shortcut_prev_tab: config.shortcuts.prev_tab.clone(),
            shortcut_quit: config.shortcuts.quit.clone(),
            ssh_profiles: config
                .ssh_profiles
                .iter()
                .map(SshProfileDraft::from_profile)
                .collect(),
        }
    }

    /// Load passwords from OS keychain into SSH profile drafts.
    /// Call this only when the user opens SSH settings, not at app startup.
    pub fn load_ssh_passwords_from_keychain(&mut self, profiles: &[SshProfile]) {
        for (draft, profile) in self.ssh_profiles.iter_mut().zip(profiles.iter()) {
            if draft.password.is_empty()
                && let Some(pw) = crate::keychain::get_password(&profile.host, &profile.user)
            {
                draft.password = pw;
            }
        }
    }

    pub fn update_ssh_profile(&mut self, index: usize, field: SshProfileField, value: String) {
        if let Some(draft) = self.ssh_profiles.get_mut(index) {
            match field {
                SshProfileField::Name => draft.name = value,
                SshProfileField::Host => draft.host = value,
                SshProfileField::Port => draft.port = value,
                SshProfileField::User => draft.user = value,
                SshProfileField::IdentityFile => draft.identity_file = value,
                SshProfileField::Password => draft.password = value,
            }
        }
    }

    pub fn apply_ssh_profiles_to(&self, profiles: &mut Vec<SshProfile>) {
        *profiles = self
            .ssh_profiles
            .iter()
            .filter_map(SshProfileDraft::to_profile)
            .collect();
    }

    pub fn update(&mut self, field: SettingsField, value: String) {
        match field {
            SettingsField::UiWindowWidth => self.window_width = value,
            SettingsField::UiWindowHeight => self.window_height = value,
            SettingsField::TerminalFontSelection => self.terminal_font_selection = value,
            SettingsField::TerminalFontSize => self.terminal_font_size = value,
            SettingsField::TerminalPaddingX => self.terminal_padding_x = value,
            SettingsField::TerminalPaddingY => self.terminal_padding_y = value,
            SettingsField::ThemeColorScheme => {
                self.color_scheme = value.clone();
                if let Some(preset) = crate::terminal::theme::find_preset(&value) {
                    self.foreground = format_rgb(preset.fg);
                    self.background = format_rgb(preset.bg);
                    self.cursor = format_rgb(preset.cursor);
                }
            }
            SettingsField::ThemeForeground => self.foreground = value,
            SettingsField::ThemeBackground => self.background = value,
            SettingsField::ThemeCursor => self.cursor = value,
            SettingsField::ThemeBackgroundOpacity => self.background_opacity = value,
            SettingsField::ThemeMacosBlurRadius => self.macos_blur_radius = value,
            SettingsField::ShortcutNewTab => self.shortcut_new_tab = value,
            SettingsField::ShortcutCloseTab => self.shortcut_close_tab = value,
            SettingsField::ShortcutOpenSettings => self.shortcut_open_settings = value,
            SettingsField::ShortcutNextTab => self.shortcut_next_tab = value,
            SettingsField::ShortcutPrevTab => self.shortcut_prev_tab = value,
            SettingsField::ShortcutQuit => self.shortcut_quit = value,
        }
    }

    pub fn sync_window_size(&mut self, width: f32, height: f32) {
        self.window_width = format!("{width:.0}");
        self.window_height = format!("{height:.0}");
    }

    pub fn to_updates(&self) -> AppConfigUpdates {
        let ansi_colors = crate::terminal::theme::find_preset(&self.color_scheme).map(|p| p.ansi);

        let mut updates = AppConfigUpdates {
            window_width: parse_f32(&self.window_width),
            window_height: parse_f32(&self.window_height),
            terminal_font_selection: Some(self.terminal_font_selection.clone()),
            terminal_font_size: parse_f32(&self.terminal_font_size),
            terminal_padding_x: parse_f32(&self.terminal_padding_x),
            terminal_padding_y: parse_f32(&self.terminal_padding_y),
            color_scheme: Some(self.color_scheme.clone()),
            foreground: parse_hex_color(&self.foreground),
            background: parse_hex_color(&self.background),
            cursor: parse_hex_color(&self.cursor),
            ansi_colors,
            background_opacity: parse_f32(&self.background_opacity),
            blur_enabled: Some(self.blur_enabled),
            macos_blur_radius: self.macos_blur_radius.trim().parse::<i32>().ok(),
            ..Default::default()
        };

        if !self.shortcut_new_tab.trim().is_empty() {
            updates.shortcut_new_tab = Some(self.shortcut_new_tab.clone());
        }
        if !self.shortcut_close_tab.trim().is_empty() {
            updates.shortcut_close_tab = Some(self.shortcut_close_tab.clone());
        }
        if !self.shortcut_open_settings.trim().is_empty() {
            updates.shortcut_open_settings = Some(self.shortcut_open_settings.clone());
        }
        if !self.shortcut_next_tab.trim().is_empty() {
            updates.shortcut_next_tab = Some(self.shortcut_next_tab.clone());
        }
        if !self.shortcut_prev_tab.trim().is_empty() {
            updates.shortcut_prev_tab = Some(self.shortcut_prev_tab.clone());
        }
        if !self.shortcut_quit.trim().is_empty() {
            updates.shortcut_quit = Some(self.shortcut_quit.clone());
        }
        updates
    }
}

fn parse_f32(value: &str) -> Option<f32> {
    value.trim().parse::<f32>().ok()
}

pub fn view_category<'a>(
    category: SettingsCategory,
    config: &'a AppConfig,
    draft: &'a SettingsDraft,
    font_combo_state: &'a iced::widget::combo_box::State<TerminalFontOption>,
    show_all_fonts: bool,
    all_font_options: &'a [TerminalFontOption],
    palette: Palette,
) -> Element<'a, Message> {
    match category {
        SettingsCategory::Ui => ui::view(config, draft, palette),
        SettingsCategory::Terminal => {
            let selected_font = all_font_options
                .iter()
                .find(|o| o.value == draft.terminal_font_selection);
            terminal::view(
                config,
                draft,
                font_combo_state,
                show_all_fonts,
                selected_font,
                palette,
            )
        }
        SettingsCategory::Theme => theme::view(config, draft, palette),
        SettingsCategory::Shortcuts => shortcuts::view(config, draft, palette),
        SettingsCategory::Ssh => ssh::view(draft, palette),
    }
}

const LABEL_WIDTH: f32 = 160.0;

pub fn input_row<'a>(
    label: &'a str,
    value: &'a str,
    field: SettingsField,
    palette: Palette,
) -> Element<'a, Message> {
    row![
        text(label).size(13).width(Length::Fixed(LABEL_WIDTH)),
        styled_text_input(value, move |next| Message::SettingsInputChanged(
            field, next
        ), palette),
    ]
    .align_y(Alignment::Center)
    .spacing(SPACING_NORMAL)
    .width(Length::Fill)
    .into()
}

pub fn input_row_with_suffix<'a>(
    label: &'a str,
    value: &'a str,
    field: SettingsField,
    suffix: &'a str,
    palette: Palette,
) -> Element<'a, Message> {
    row![
        text(label).size(13).width(Length::Fixed(LABEL_WIDTH)),
        styled_text_input(value, move |next| Message::SettingsInputChanged(
            field, next
        ), palette),
        text(suffix)
            .size(12)
            .color(palette.text_secondary)
            .width(Length::Shrink),
    ]
    .align_y(Alignment::Center)
    .spacing(SPACING_NORMAL)
    .width(Length::Fill)
    .into()
}

#[allow(dead_code)]
pub fn color_input_row<'a>(
    label: &'a str,
    value: &'a str,
    field: SettingsField,
    palette: Palette,
) -> Element<'a, Message> {
    let parsed = parse_hex_color(value);
    let dot_color = parsed
        .map(|rgb| Color::from_rgb8(rgb[0], rgb[1], rgb[2]))
        .unwrap_or(palette.error);

    row![
        text(label).size(13).width(Length::Fixed(LABEL_WIDTH)),
        container("")
            .width(Length::Fixed(18.0))
            .height(Length::Fixed(18.0))
            .style(move |_theme: &iced::Theme| container::Style {
                background: Some(Background::Color(dot_color)),
                border: Border {
                    radius: 9.0.into(),
                    width: 1.0,
                    color: Color {
                        a: 0.25,
                        ..palette.text
                    },
                },
                ..Default::default()
            }),
        styled_text_input(value, move |next| Message::SettingsInputChanged(
            field, next
        ), palette),
    ]
    .align_y(Alignment::Center)
    .spacing(SPACING_NORMAL)
    .width(Length::Fill)
    .into()
}

pub fn toggle_row<'a>(label: &'a str, value: bool) -> Element<'a, Message> {
    row![
        text(label).size(13).width(Length::Fixed(LABEL_WIDTH)),
        toggler(value)
            .on_toggle(Message::SettingsBlurToggled)
            .size(18)
    ]
    .align_y(Alignment::Center)
    .spacing(SPACING_NORMAL)
    .width(Length::Fill)
    .into()
}

pub fn hint_text<'a>(msg: &'a str, palette: Palette) -> Element<'a, Message> {
    text(msg).size(11).color(palette.text_secondary).into()
}

#[allow(dead_code)]
pub fn divider<'a>(palette: Palette) -> Element<'a, Message> {
    container(
        rule::horizontal(1).style(move |_theme: &iced::Theme| rule::Style {
            color: Color {
                a: 0.10,
                ..palette.text
            },
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: false,
        }),
    )
    .padding([4, 0])
    .width(Length::Fill)
    .into()
}

pub fn section<'a>(title: &'a str, body: Element<'a, Message>, palette: Palette) -> Element<'a, Message> {
    container(
        column(vec![
            text(title).size(14).color(palette.accent).into(),
            container("")
                .width(Length::Fill)
                .height(Length::Fixed(1.0))
                .style(move |_theme: &iced::Theme| container::Style {
                    background: Some(Background::Color(Color {
                        a: 0.15,
                        ..palette.accent
                    })),
                    ..Default::default()
                })
                .into(),
            body,
        ])
        .spacing(SPACING_NORMAL)
        .width(Length::Fill),
    )
    .padding([16, 16])
    .width(Length::Fill)
    .style(move |_theme: &iced::Theme| container::Style {
        background: Some(Background::Color(Color {
            a: 0.18,
            ..palette.surface
        })),
        border: Border {
            radius: RADIUS_NORMAL.into(),
            width: 1.0,
            color: Color {
                a: 0.08,
                ..palette.text
            },
        },
        ..Default::default()
    })
    .into()
}

fn styled_text_input<'a, F>(
    value: &'a str,
    on_input: F,
    palette: Palette,
) -> text_input::TextInput<'a, Message>
where
    F: 'a + Fn(String) -> Message,
{
    text_input("", value)
        .on_input(on_input)
        .padding([6, 10])
        .width(Length::Fill)
        .style(move |_theme: &iced::Theme, status: text_input::Status| {
            let focused = matches!(status, text_input::Status::Focused { .. });
            text_input::Style {
                background: Background::Color(Color {
                    a: 0.35,
                    ..palette.background
                }),
                border: Border {
                    radius: RADIUS_SMALL.into(),
                    width: 1.0,
                    color: if focused {
                        Color {
                            a: 0.5,
                            ..palette.accent
                        }
                    } else {
                        Color {
                            a: 0.12,
                            ..palette.text
                        }
                    },
                },
                icon: palette.text_secondary,
                placeholder: palette.text_secondary,
                value: palette.text,
                selection: Color {
                    a: 0.3,
                    ..palette.accent
                },
            }
        })
}

pub fn styled_text_input_small<'a, F>(
    value: &'a str,
    on_input: F,
    palette: Palette,
) -> text_input::TextInput<'a, Message>
where
    F: 'a + Fn(String) -> Message,
{
    text_input("", value)
        .on_input(on_input)
        .padding([4, 8])
        .width(Length::Fixed(100.0))
        .style(move |_theme: &iced::Theme, status: text_input::Status| {
            let focused = matches!(status, text_input::Status::Focused { .. });
            text_input::Style {
                background: Background::Color(Color {
                    a: 0.35,
                    ..palette.background
                }),
                border: Border {
                    radius: RADIUS_SMALL.into(),
                    width: 1.0,
                    color: if focused {
                        Color {
                            a: 0.5,
                            ..palette.accent
                        }
                    } else {
                        Color {
                            a: 0.12,
                            ..palette.text
                        }
                    },
                },
                icon: palette.text_secondary,
                placeholder: palette.text_secondary,
                value: palette.text,
                selection: Color {
                    a: 0.3,
                    ..palette.accent
                },
            }
        })
}

pub fn format_rgb(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SshProfile;

    #[test]
    fn ssh_draft_roundtrip_with_password() {
        let profile = SshProfile {
            name: "prod".into(),
            host: "10.0.0.1".into(),
            port: 2222,
            user: "deploy".into(),
            identity_file: Some("~/.ssh/id_rsa".into()),
            password: Some("s3cret".into()),
        };

        let draft = SshProfileDraft::from_profile(&profile);
        assert_eq!(draft.name, "prod");
        assert_eq!(draft.host, "10.0.0.1");
        assert_eq!(draft.port, "2222");
        assert_eq!(draft.user, "deploy");
        assert_eq!(draft.identity_file, "~/.ssh/id_rsa");
        assert_eq!(draft.password, "s3cret");

        let back = draft.to_profile().unwrap();
        assert_eq!(back.password.as_deref(), Some("s3cret"));
        assert_eq!(back.port, 2222);
    }

    #[test]
    fn ssh_draft_empty_password_becomes_none() {
        let draft = SshProfileDraft {
            name: "test".into(),
            host: "host".into(),
            port: "22".into(),
            user: "".into(),
            identity_file: "".into(),
            password: "  ".into(),
        };
        let profile = draft.to_profile().unwrap();
        assert!(profile.password.is_none());
        assert!(profile.identity_file.is_none());
    }

    #[test]
    fn ssh_draft_empty_host_returns_none() {
        let draft = SshProfileDraft {
            name: "test".into(),
            host: "  ".into(),
            port: "22".into(),
            user: "".into(),
            identity_file: "".into(),
            password: "pass".into(),
        };
        assert!(draft.to_profile().is_none());
    }

    #[test]
    fn update_ssh_profile_password_field() {
        let config = crate::config::AppConfig {
            ssh_profiles: vec![SshProfile {
                name: "srv".into(),
                host: "h".into(),
                port: 22,
                user: "u".into(),
                identity_file: None,
                password: None,
            }],
            ..Default::default()
        };
        let mut draft = SettingsDraft::from_config(&config);
        assert_eq!(draft.ssh_profiles[0].password, "");

        draft.update_ssh_profile(0, SshProfileField::Password, "newpass".into());
        assert_eq!(draft.ssh_profiles[0].password, "newpass");
    }
}
