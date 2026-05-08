use crate::gui::app::Message;
use crate::gui::theme::{Palette, RADIUS_SMALL};
use iced::widget::{button, column, container, mouse_area, stack, text};
use iced::{Background, Border, Color, Element, Length, Padding};

pub struct ContextMenuItem {
    pub label: &'static str,
    pub message: Message,
}

pub fn context_menu<'a>(
    base: impl Into<Element<'a, Message>>,
    items: Vec<ContextMenuItem>,
    position: iced::Point,
    on_dismiss: Message,
    palette: Palette,
) -> Element<'a, Message> {
    let menu_items: Vec<Element<Message>> = items
        .into_iter()
        .map(|item| {
            button(text(item.label).size(13))
                .on_press(item.message)
                .padding([7, 14])
                .width(Length::Fill)
                .style(
                    move |_theme: &iced::Theme, status: button::Status| button::Style {
                        background: Some(Background::Color(
                            if matches!(status, button::Status::Hovered) {
                                Color {
                                    a: 0.1,
                                    ..palette.text
                                }
                            } else {
                                Color::TRANSPARENT
                            },
                        )),
                        text_color: palette.text,
                        border: Border {
                            radius: RADIUS_SMALL.into(),
                            width: 0.0,
                            color: Color::TRANSPARENT,
                        },
                        shadow: iced::Shadow::default(),
                        snap: false,
                    },
                )
                .into()
        })
        .collect();

    let menu = container(column(menu_items).padding([4, 4]))
        .width(Length::Fixed(140.0))
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(Background::Color(Color {
                a: 0.97,
                ..palette.surface
            })),
            border: Border {
                radius: RADIUS_SMALL.into(),
                width: 1.0,
                color: Color {
                    a: 0.15,
                    ..palette.text
                },
            },
            shadow: iced::Shadow {
                color: Color {
                    a: 0.3,
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                },
                offset: iced::Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            ..Default::default()
        });

    let backdrop = mouse_area(container(text("")).width(Length::Fill).height(Length::Fill))
        .on_press(on_dismiss.clone())
        .on_right_press(on_dismiss);

    let positioned = container(menu)
        .padding(Padding::new(0.0).top(position.y).left(position.x))
        .width(Length::Fill)
        .height(Length::Fill);

    stack![base.into(), backdrop, positioned]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
