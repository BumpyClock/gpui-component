use gpui::{
    Action, AnyElement, AnyView, App, AppContext, Context, IntoElement, ParentElement, Render,
    SharedString, StyleRefinement, Styled, Window, div, prelude::FluentBuilder, px,
};

use crate::{
    ActiveTheme, StyledExt, SurfaceContext, SurfacePreset, global_state::GlobalState, h_flex,
    kbd::Kbd, text::Text,
};

enum TooltipContext {
    Text(Text),
    Element(Box<dyn Fn(&mut Window, &mut App) -> AnyElement>),
}

/// A Tooltip element that can display text or custom content,
/// with optional key binding information.
pub struct Tooltip {
    style: StyleRefinement,
    content: TooltipContext,
    key_binding: Option<Kbd>,
    action: Option<(Box<dyn Action>, Option<SharedString>)>,
}

impl Tooltip {
    /// Create a Tooltip with a text content.
    pub fn new(text: impl Into<Text>) -> Self {
        Self {
            style: StyleRefinement::default(),
            content: TooltipContext::Text(text.into()),
            key_binding: None,
            action: None,
        }
    }

    /// Create a Tooltip with a custom element.
    pub fn element<E, F>(builder: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        Self {
            style: StyleRefinement::default(),
            key_binding: None,
            action: None,
            content: TooltipContext::Element(Box::new(move |window, cx| {
                builder(window, cx).into_any_element()
            })),
        }
    }

    /// Set Action to display key binding information for the tooltip if it exists.
    pub fn action(mut self, action: &dyn Action, context: Option<&str>) -> Self {
        self.action = Some((action.boxed_clone(), context.map(SharedString::new)));
        self
    }

    /// Set KeyBinding information for the tooltip.
    pub fn key_binding(mut self, key_binding: Option<Kbd>) -> Self {
        self.key_binding = key_binding;
        self
    }

    /// Build the tooltip and return it as an `AnyView`.
    pub fn build(self, _: &mut Window, cx: &mut App) -> AnyView {
        cx.new(|_| self).into()
    }
}

impl FluentBuilder for Tooltip {}
impl Styled for Tooltip {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl Render for Tooltip {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let key_binding = if let Some(key_binding) = &self.key_binding {
            Some(key_binding.clone())
        } else {
            if let Some((action, context)) = &self.action {
                Kbd::binding_for_action(
                    action.as_ref(),
                    context.as_ref().map(|s| s.as_ref()),
                    window,
                )
            } else {
                None
            }
        };

        let window_size = window.bounds().size;
        let ctx = SurfaceContext {
            blur_enabled: GlobalState::global(cx).blur_enabled(),
        };

        let content = h_flex()
            .font_family(cx.theme().font_family.clone())
            .text_color(cx.theme().surface_elevated_foreground)
            .justify_between()
            .py_0p5()
            .px_2()
            .text_sm()
            .gap_3()
            .refine_style(&self.style)
            .map(|this| {
                this.child(div().map(|this| match self.content {
                    TooltipContext::Text(ref text) => this.child(text.clone()),
                    TooltipContext::Element(ref builder) => this.child(builder(window, cx)),
                }))
            })
            .when_some(key_binding, |this, kbd| {
                this.child(
                    div()
                        .text_xs()
                        .flex_shrink_0()
                        .text_color(cx.theme().muted_foreground)
                        .child(kbd.appearance(false)),
                )
            });

        div().child(
            SurfacePreset::flyout()
                .with_radius(px(6.))
                .wrap_with_bounds(
                    content,
                    window_size.width,
                    window_size.height,
                    window,
                    cx,
                    ctx,
                )
                .bg(cx.theme().overlay_tooltip)
                .m_3(),
        )
    }
}
