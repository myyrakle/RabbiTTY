use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::widget::{Operation, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::{
    Animation, Background, Border, Color, Element, Event, Length, Rectangle, Size, Vector, mouse,
};

use std::time::{Duration, Instant};

const TRANSITION: Duration = Duration::from_millis(120);

/// Background + border appearance for one end of a hover transition.
#[derive(Debug, Clone, Copy)]
pub struct HoverStyle {
    pub background: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub radius: f32,
}

/// Linearly interpolates between two colors.
fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    Color {
        r: from.r + (to.r - from.r) * t,
        g: from.g + (to.g - from.g) * t,
        b: from.b + (to.b - from.b) * t,
        a: from.a + (to.a - from.a) * t,
    }
}

/// Per-widget animation state held in the widget tree.
struct State {
    anim: Animation<bool>,
}

/// A widget that wraps a child `Element` and cross-fades a background +
/// border between a "rest" and a "hovered" appearance as the cursor enters
/// and leaves the widget bounds.
///
/// The wrapped child is fully delegated to for layout, events, drawing and
/// overlays, so interactive children (such as buttons) keep working — the
/// `HoverFade` only paints an animated quad *behind* the child.
pub struct HoverFade<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    rest: HoverStyle,
    hover: HoverStyle,
    enabled: bool,
}

/// Wraps `child` so its background + border smoothly fade on hover.
///
/// When `enabled` is `false` the transition is skipped entirely: the
/// appearance snaps instantly between `rest` and `hover` with no redraw
/// requests.
pub fn hover_fade<'a, Message, Theme, Renderer>(
    child: impl Into<Element<'a, Message, Theme, Renderer>>,
    rest: HoverStyle,
    hover: HoverStyle,
    enabled: bool,
) -> HoverFade<'a, Message, Theme, Renderer> {
    HoverFade {
        content: child.into(),
        rest,
        hover,
        enabled,
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for HoverFade<'_, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            anim: Animation::new(false)
                .duration(TRANSITION)
                .easing(iced::animation::Easing::EaseOut),
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();
        let now = Instant::now();
        let hovered = cursor.is_over(layout.bounds());

        if self.enabled {
            if state.anim.value() != hovered {
                state.anim.go_mut(hovered, now);
            }
            // Keep advancing the fade while it is in progress.
            if matches!(
                event,
                Event::Window(iced::window::Event::RedrawRequested(_))
            ) && state.anim.is_animating(now)
            {
                shell.request_redraw();
            }
        } else {
            // Animations disabled: snap instantly, no redraw requests.
            if state.anim.value() != hovered {
                state.anim.go_mut(hovered, now - TRANSITION);
            }
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();

        let t: f32 = if self.enabled {
            state.anim.interpolate(0.0, 1.0, Instant::now())
        } else if state.anim.value() {
            1.0
        } else {
            0.0
        };

        let background = lerp_color(self.rest.background, self.hover.background, t);
        let border_color = lerp_color(self.rest.border_color, self.hover.border_color, t);
        let border_width =
            self.rest.border_width + (self.hover.border_width - self.rest.border_width) * t;
        let radius = self.rest.radius + (self.hover.radius - self.rest.radius) * t;

        if background.a > 0.0 || (border_width > 0.0 && border_color.a > 0.0) {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: Border {
                        color: border_color,
                        width: border_width,
                        radius: radius.into(),
                    },
                    shadow: iced::Shadow::default(),
                    snap: true,
                },
                Background::Color(background),
            );
        }

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<HoverFade<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(widget: HoverFade<'a, Message, Theme, Renderer>) -> Self {
        Element::new(widget)
    }
}
