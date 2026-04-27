use super::{App, Message};
use iced::futures::StreamExt;
use iced::futures::channel::mpsc;
use iced::futures::sink::SinkExt;
use iced::stream;
use iced::time::Instant;
use iced::{Event, Subscription, event, keyboard, mouse, time, window};

impl App {
    pub fn subscription(&self) -> Subscription<Message> {
        let now = Instant::now();
        let has_animation = self.shell_picker_anim.is_animating(now);

        let animation_tick = if has_animation {
            time::every(std::time::Duration::from_millis(16)).map(|_| Message::AnimationTick)
        } else {
            Subscription::none()
        };

        Subscription::batch([
            animation_tick,
            Subscription::run(|| {
                stream::channel(100, async |mut output| {
                    let (sender, mut receiver) = mpsc::unbounded();
                    let _ = output.send(Message::PtySenderReady(sender)).await;

                    while let Some(first) = receiver.next().await {
                        let mut batch = vec![first];
                        while let Ok(event) = receiver.try_recv() {
                            batch.push(event);
                        }
                        if batch.len() == 1 {
                            if output
                                .send(Message::PtyOutput(batch.pop().unwrap()))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        } else if output.send(Message::PtyOutputBatch(batch)).await.is_err() {
                            break;
                        }
                    }
                })
            }),
            event::listen_with(|event, _status, _id| match event {
                Event::Window(window::Event::CloseRequested) => Some(Message::Exit),
                Event::Window(window::Event::Resized(size)) => Some(Message::WindowResized(size)),
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key,
                    modifiers,
                    text,
                    ..
                }) => Some(Message::KeyPressed {
                    key,
                    modifiers,
                    text: text.map(|s| s.to_string()),
                }),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    Some(Message::TabDragRelease)
                }
                Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                    let (lines_y, pixels_x) = match delta {
                        mouse::ScrollDelta::Lines { x, y } => (y, x * 30.0),
                        mouse::ScrollDelta::Pixels { x, y } => (y / 20.0, x),
                    };
                    if lines_y.abs() > 0.01 {
                        Some(Message::TerminalWheelScroll(lines_y))
                    } else if pixels_x.abs() > 0.1 {
                        Some(Message::TabBarScroll(pixels_x))
                    } else {
                        None
                    }
                }
                _ => None,
            }),
        ])
    }
}
