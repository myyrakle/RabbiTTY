use crate::config::SshAuthMethod;
use crate::gui::app::Message;
use crate::gui::components::{button_primary, button_secondary};
use crate::gui::settings::{SettingsDraft, SshProfileDraft, SshProfileField, SshProfileModalMode};
use crate::gui::theme::{Palette, RADIUS_NORMAL, RADIUS_SMALL, SPACING_NORMAL, SPACING_SMALL};
use iced::widget::{button, center, column, container, mouse_area, row, stack, text, text_input};
use iced::{Alignment, Background, Border, Color, Element, Length};

pub fn view(draft: &SettingsDraft, palette: Palette) -> Element<'_, Message> {
    content(draft, palette)
}

pub fn modal_overlay<'a>(
    base: Element<'a, Message>,
    draft: &'a SettingsDraft,
    palette: Palette,
) -> Element<'a, Message> {
    if let Some(mode) = draft.ssh_profile_modal_mode {
        return modal_overlay_content(
            base,
            mode,
            &draft.ssh_profile_modal_draft,
            draft.ssh_profiles_error.as_deref(),
            palette,
        );
    }

    base
}

fn content(draft: &SettingsDraft, palette: Palette) -> Element<'_, Message> {
    let mut items: Vec<Element<Message>> = Vec::new();

    items.push(
        row![
            text("Profiles").size(16).color(palette.text),
            container("").width(Length::Fill),
            button_primary("+", palette).on_press(Message::AddSshProfile),
        ]
        .spacing(SPACING_SMALL)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into(),
    );

    if let Some(error) = &draft.ssh_profiles_error {
        items.push(status_banner(error, palette));
    }

    if draft.ssh_profiles.is_empty() {
        items.push(empty_state(palette));
    } else {
        for (index, profile) in draft.ssh_profiles.iter().enumerate() {
            items.push(profile_row(index, profile, palette));
        }
    }

    column(items)
        .spacing(SPACING_NORMAL)
        .width(Length::Fill)
        .into()
}

fn empty_state(palette: Palette) -> Element<'static, Message> {
    container(
        column![
            text("\u{21c4}").size(28).color(Color {
                a: 0.3,
                ..palette.text
            }),
            text("No SSH profiles yet").size(13).color(Color {
                a: 0.5,
                ..palette.text
            }),
            text("Add a profile to quickly connect to remote servers")
                .size(11)
                .color(palette.text_secondary),
        ]
        .spacing(6)
        .align_x(Alignment::Center),
    )
    .center_x(Length::Fill)
    .padding([24, 0])
    .into()
}

fn profile_row<'a>(
    index: usize,
    profile: &'a SshProfileDraft,
    palette: Palette,
) -> Element<'a, Message> {
    let title = profile_title(profile);
    let subtitle = profile_subtitle(profile);

    container(
        row![
            column![
                text(title).size(14).color(palette.text),
                text(subtitle).size(12).color(palette.text_secondary),
            ]
            .spacing(4)
            .width(Length::Fill),
            row![
                icon_button("\u{270e}", palette).on_press(Message::EditSshProfile(index)),
                icon_button("\u{1f5d1}", palette).on_press(Message::RemoveSshProfile(index)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        ]
        .spacing(SPACING_NORMAL)
        .align_y(Alignment::Center)
        .width(Length::Fill),
    )
    .padding([12, 14])
    .width(Length::Fill)
    .style(move |_theme: &iced::Theme| container::Style {
        background: Some(Background::Color(Color {
            a: 0.10,
            ..palette.surface
        })),
        border: Border {
            radius: RADIUS_SMALL.into(),
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

fn profile_title(profile: &SshProfileDraft) -> String {
    if !profile.name.trim().is_empty() {
        profile.name.trim().to_string()
    } else if !profile.host.trim().is_empty() && !profile.user.trim().is_empty() {
        format!("{}@{}", profile.user.trim(), profile.host.trim())
    } else if !profile.host.trim().is_empty() {
        profile.host.trim().to_string()
    } else {
        "New Profile".to_string()
    }
}

fn profile_subtitle(profile: &SshProfileDraft) -> String {
    let endpoint = if profile.user.trim().is_empty() {
        format!(
            "{}:{}",
            empty_label(&profile.host),
            empty_label(&profile.port)
        )
    } else {
        format!(
            "{}@{}:{}",
            profile.user.trim(),
            empty_label(&profile.host),
            empty_label(&profile.port)
        )
    };
    let auth = match profile.auth_method {
        SshAuthMethod::KeyFile => "Key file",
        SshAuthMethod::Password => "Password",
    };
    let proxy = if profile.proxy_command.trim().is_empty() {
        ""
    } else {
        " · Proxy"
    };
    format!("{endpoint} · {auth}{proxy}")
}

fn empty_label(value: &str) -> &str {
    let value = value.trim();
    if value.is_empty() { "-" } else { value }
}

fn modal_overlay_content<'a>(
    base: Element<'a, Message>,
    mode: SshProfileModalMode,
    profile: &'a SshProfileDraft,
    error: Option<&'a str>,
    palette: Palette,
) -> Element<'a, Message> {
    let backdrop = mouse_area(
        container(text(""))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme: &iced::Theme| container::Style {
                background: Some(Background::Color(Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.50,
                })),
                ..Default::default()
            }),
    )
    .on_press(Message::CloseSshProfileModal);

    let title = match mode {
        SshProfileModalMode::Create => "Create SSH Profile",
        SshProfileModalMode::Edit(_) => "Edit SSH Profile",
    };

    let mut modal_items: Vec<Element<Message>> = Vec::new();
    modal_items.push(
        row![
            text(title).size(16).color(palette.text),
            container("").width(Length::Fill),
            icon_button("x", palette).on_press(Message::CloseSshProfileModal),
        ]
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into(),
    );
    if let Some(error) = error.filter(|message| *message != "SSH profiles saved.") {
        modal_items.push(status_banner(error, palette));
    }
    modal_items.push(profile_form(profile, palette));
    modal_items.push(
        row![
            container("").width(Length::Fill),
            button_secondary("Cancel", palette).on_press(Message::CloseSshProfileModal),
            button_primary("Save", palette).on_press(Message::SaveSshProfileModal),
        ]
        .spacing(SPACING_SMALL)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into(),
    );

    let modal = container(
        column(modal_items)
            .spacing(SPACING_NORMAL)
            .padding(20)
            .width(Length::Fixed(480.0)),
    )
    .style(move |_theme: &iced::Theme| container::Style {
        background: Some(Background::Color(palette.surface)),
        border: Border {
            radius: RADIUS_NORMAL.into(),
            width: 1.0,
            color: Color {
                a: 0.16,
                ..palette.text
            },
        },
        ..Default::default()
    });

    let modal_layer = mouse_area(modal).on_press(Message::Noop);

    stack![
        base,
        backdrop,
        center(modal_layer).width(Length::Fill).height(Length::Fill)
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn profile_form<'a>(profile: &'a SshProfileDraft, palette: Palette) -> Element<'a, Message> {
    let key_button = if matches!(profile.auth_method, SshAuthMethod::KeyFile) {
        button_primary("Key File", palette)
    } else {
        button_secondary("Key File", palette).on_press(Message::SshProfileModalFieldChanged(
            SshProfileField::AuthMethod,
            "key_file".into(),
        ))
    };

    let password_button = if matches!(profile.auth_method, SshAuthMethod::Password) {
        button_primary("Password", palette)
    } else {
        button_secondary("Password", palette).on_press(Message::SshProfileModalFieldChanged(
            SshProfileField::AuthMethod,
            "password".into(),
        ))
    };

    let auth_input: Element<'a, Message> = if matches!(profile.auth_method, SshAuthMethod::KeyFile)
    {
        modal_input(
            "Key File  (e.g. ~/.ssh/id_rsa)",
            &profile.identity_file,
            |next| Message::SshProfileModalFieldChanged(SshProfileField::IdentityFile, next),
            palette,
        )
        .into()
    } else {
        modal_password("Password", &profile.password, palette).into()
    };

    column![
        modal_input(
            "Display Name (optional)",
            &profile.name,
            |next| { Message::SshProfileModalFieldChanged(SshProfileField::Name, next) },
            palette
        ),
        row![
            modal_input(
                "Host",
                &profile.host,
                |next| { Message::SshProfileModalFieldChanged(SshProfileField::Host, next) },
                palette
            )
            .width(Length::Fill),
            text(":").size(13).color(palette.text_secondary),
            modal_input(
                "Port",
                &profile.port,
                |next| { Message::SshProfileModalFieldChanged(SshProfileField::Port, next) },
                palette
            )
            .width(Length::Fixed(80.0)),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .width(Length::Fill),
        modal_input(
            "Username",
            &profile.user,
            |next| { Message::SshProfileModalFieldChanged(SshProfileField::User, next) },
            palette
        ),
        text("Authentication")
            .size(11)
            .color(palette.text_secondary),
        row![key_button, password_button]
            .spacing(SPACING_SMALL)
            .width(Length::Fill),
        auth_input,
        text(if matches!(profile.auth_method, SshAuthMethod::Password) {
            "Password is stored securely in your OS keychain"
        } else {
            "Key file path is stored in config"
        })
        .size(10)
        .color(Color {
            a: 0.35,
            ..palette.text
        }),
        text("Proxy Command").size(11).color(palette.text_secondary),
        modal_input(
            "ProxyCommand  (e.g. cloudflared access ssh --hostname %h)",
            &profile.proxy_command,
            |next| Message::SshProfileModalFieldChanged(SshProfileField::ProxyCommand, next),
            palette,
        ),
        text("%h and %p are replaced with host and port")
            .size(10)
            .color(Color {
                a: 0.35,
                ..palette.text
            }),
    ]
    .spacing(8)
    .width(Length::Fill)
    .into()
}

fn status_banner<'a>(message: &'a str, palette: Palette) -> Element<'a, Message> {
    let is_saved = message == "SSH profiles saved.";
    let color = if is_saved {
        palette.accent
    } else {
        palette.error
    };

    container(text(message).size(12).color(Color { a: 0.95, ..color }))
        .padding([8, 10])
        .width(Length::Fill)
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(Background::Color(Color { a: 0.08, ..color })),
            border: Border {
                radius: RADIUS_SMALL.into(),
                width: 1.0,
                color: Color { a: 0.25, ..color },
            },
            ..Default::default()
        })
        .into()
}

fn icon_button(icon: &str, palette: Palette) -> button::Button<'_, Message> {
    button(text(icon).size(15)).padding([5, 8]).style(
        move |_theme: &iced::Theme, status: button::Status| {
            let (bg, border_alpha) = match status {
                button::Status::Hovered => (
                    Color {
                        a: 0.12,
                        ..palette.text
                    },
                    0.20,
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
                _ => (Color::TRANSPARENT, 0.10),
            };
            let text_color = match status {
                button::Status::Disabled => palette.text_secondary,
                _ => palette.text,
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color,
                border: Border {
                    radius: RADIUS_SMALL.into(),
                    width: 1.0,
                    color: Color {
                        a: border_alpha,
                        ..palette.text
                    },
                },
                shadow: iced::Shadow::default(),
                snap: true,
            }
        },
    )
}

fn modal_input<'a, F>(
    placeholder: &'a str,
    value: &'a str,
    on_input: F,
    palette: Palette,
) -> text_input::TextInput<'a, Message>
where
    F: 'a + Fn(String) -> Message,
{
    text_input(placeholder, value)
        .on_input(on_input)
        .padding([6, 10])
        .size(13)
        .width(Length::Fill)
        .style(move |_theme: &iced::Theme, status: text_input::Status| input_style(palette, status))
}

fn modal_password<'a>(
    placeholder: &'a str,
    value: &'a str,
    palette: Palette,
) -> text_input::TextInput<'a, Message> {
    text_input(placeholder, value)
        .secure(true)
        .on_input(move |next| Message::SshProfileModalFieldChanged(SshProfileField::Password, next))
        .padding([6, 10])
        .size(13)
        .width(Length::Fill)
        .style(move |_theme: &iced::Theme, status: text_input::Status| input_style(palette, status))
}

fn input_style(palette: Palette, status: text_input::Status) -> text_input::Style {
    let focused = matches!(status, text_input::Status::Focused { .. });
    text_input::Style {
        background: Background::Color(Color {
            a: 0.25,
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
                    a: 0.08,
                    ..palette.text
                }
            },
        },
        icon: palette.text_secondary,
        placeholder: Color {
            a: 0.3,
            ..palette.text
        },
        value: palette.text,
        selection: Color {
            a: 0.3,
            ..palette.accent
        },
    }
}
