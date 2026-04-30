use crate::gui::app::Message;
use crate::gui::components::{button_primary, button_secondary};
use crate::gui::settings::{SettingsDraft, SshProfileDraft, SshProfileField};
use crate::gui::theme::{Palette, RADIUS_SMALL, SPACING_NORMAL, SPACING_SMALL};
use iced::widget::{column, container, row, text, text_input};
use iced::{Alignment, Background, Border, Color, Element, Length};

pub fn view(draft: &SettingsDraft, palette: Palette) -> Element<'_, Message> {
    let mut items: Vec<Element<Message>> = Vec::new();

    for (i, profile) in draft.ssh_profiles.iter().enumerate() {
        items.push(profile_card(i, profile, palette));
    }

    if draft.ssh_profiles.is_empty() {
        items.push(empty_state(palette));
    }

    items.push(
        row![
            button_primary("+ Add Profile", palette).on_press(Message::AddSshProfile),
            button_primary("Save All", palette).on_press(Message::SaveSshProfiles),
        ]
        .spacing(SPACING_SMALL)
        .into(),
    );

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

fn profile_card<'a>(
    index: usize,
    profile: &'a SshProfileDraft,
    palette: Palette,
) -> Element<'a, Message> {
    // Dynamic title: show preview of connection
    let title = if !profile.name.is_empty() {
        profile.name.clone()
    } else if profile.host.is_empty() {
        "New Profile".to_string()
    } else if profile.user.is_empty() {
        profile.host.clone()
    } else {
        format!("{}@{}", profile.user, profile.host)
    };

    let title_row = row![
        text(title).size(14).color(palette.text),
        container("").width(Length::Fill),
        button_secondary("Remove", palette).on_press(Message::RemoveSshProfile(index)),
    ]
    .align_y(Alignment::Center)
    .width(Length::Fill);

    let divider = container("")
        .width(Length::Fill)
        .height(Length::Fixed(1.0))
        .style(move |_theme: &iced::Theme| iced::widget::container::Style {
            background: Some(Background::Color(Color {
                a: 0.08,
                ..palette.text
            })),
            ..Default::default()
        });

    // Connection section: host:port on one row, user below
    let host_port_row = row![
        styled_input("Host", &profile.host, index, SshProfileField::Host, palette)
            .width(Length::Fill),
        text(":").size(13).color(palette.text_secondary),
        styled_input_small(
            "Port",
            &profile.port,
            index,
            SshProfileField::Port,
            60.0,
            palette
        ),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    let user_row = styled_input(
        "Username",
        &profile.user,
        index,
        SshProfileField::User,
        palette,
    )
    .width(Length::Fill);

    let name_row = styled_input(
        "Display Name (optional)",
        &profile.name,
        index,
        SshProfileField::Name,
        palette,
    )
    .width(Length::Fill);

    // Auth section
    let auth_label = text("Authentication")
        .size(11)
        .color(palette.text_secondary);

    let identity_row = styled_input(
        "Key File  (e.g. ~/.ssh/id_rsa)",
        &profile.identity_file,
        index,
        SshProfileField::IdentityFile,
        palette,
    )
    .width(Length::Fill);

    let password_row = styled_password("Password", &profile.password, index, palette);

    let auth_hint = text("Password is stored securely in your OS keychain")
        .size(10)
        .color(Color {
            a: 0.35,
            ..palette.text
        });

    container(
        column![
            title_row,
            divider,
            host_port_row,
            user_row,
            name_row,
            auth_label,
            identity_row,
            password_row,
            auth_hint,
        ]
        .spacing(8)
        .width(Length::Fill),
    )
    .padding([14, 16])
    .width(Length::Fill)
    .style(move |_theme: &iced::Theme| iced::widget::container::Style {
        background: Some(Background::Color(Color {
            a: 0.12,
            ..palette.surface
        })),
        border: Border {
            radius: RADIUS_SMALL.into(),
            width: 1.0,
            color: Color {
                a: 0.06,
                ..palette.text
            },
        },
        ..Default::default()
    })
    .into()
}

fn styled_input<'a>(
    placeholder: &'a str,
    value: &'a str,
    index: usize,
    field: SshProfileField,
    palette: Palette,
) -> text_input::TextInput<'a, Message> {
    text_input(placeholder, value)
        .on_input(move |next| Message::SshProfileFieldChanged(index, field, next))
        .padding([6, 10])
        .size(13)
        .style(move |_theme: &iced::Theme, status: text_input::Status| input_style(palette, status))
}

fn styled_input_small<'a>(
    placeholder: &'a str,
    value: &'a str,
    index: usize,
    field: SshProfileField,
    width: f32,
    palette: Palette,
) -> text_input::TextInput<'a, Message> {
    text_input(placeholder, value)
        .on_input(move |next| Message::SshProfileFieldChanged(index, field, next))
        .padding([6, 10])
        .size(13)
        .width(Length::Fixed(width))
        .style(move |_theme: &iced::Theme, status: text_input::Status| input_style(palette, status))
}

fn styled_password<'a>(
    placeholder: &'a str,
    value: &'a str,
    index: usize,
    palette: Palette,
) -> text_input::TextInput<'a, Message> {
    text_input(placeholder, value)
        .secure(true)
        .on_input(move |next| {
            Message::SshProfileFieldChanged(index, SshProfileField::Password, next)
        })
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
