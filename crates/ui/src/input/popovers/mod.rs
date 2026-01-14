mod code_action_menu;
mod completion_menu;
mod context_menu;
mod diagnostic_popover;
mod hover_popover;

pub(crate) use code_action_menu::*;
pub(crate) use completion_menu::*;
pub(crate) use context_menu::*;
pub(crate) use diagnostic_popover::*;
pub(crate) use hover_popover::*;

use gpui::{
    App, Div, ElementId, Entity, InteractiveElement as _, IntoElement, SharedString, Stateful,
    StyleRefinement, Styled as _, Window, div, px, rems,
};

use crate::{
    ActiveTheme, ElevationToken, StyledExt as _, SurfaceContext, SurfacePreset,
    global_state::GlobalState,
    text::{TextView, TextViewStyle},
};

pub(crate) enum ContextMenu {
    Completion(Entity<CompletionMenu>),
    CodeAction(Entity<CodeActionMenu>),
    MouseContext(Entity<MouseContextMenu>),
}

impl ContextMenu {
    pub(crate) fn is_open(&self, cx: &App) -> bool {
        match self {
            ContextMenu::Completion(menu) => menu.read(cx).is_open(),
            ContextMenu::CodeAction(menu) => menu.read(cx).is_open(),
            ContextMenu::MouseContext(menu) => menu.read(cx).is_open(),
        }
    }

    pub(crate) fn render(&self) -> impl IntoElement {
        match self {
            ContextMenu::Completion(menu) => menu.clone().into_any_element(),
            ContextMenu::CodeAction(menu) => menu.clone().into_any_element(),
            ContextMenu::MouseContext(menu) => menu.clone().into_any_element(),
        }
    }
}

pub(super) fn render_markdown(
    id: impl Into<ElementId>,
    markdown: impl Into<SharedString>,
    _: &mut Window,
    cx: &mut App,
) -> TextView {
    TextView::markdown(id, markdown)
        .style(
            TextViewStyle::default()
                .paragraph_gap(rems(0.5))
                .heading_font_size(|level, rem_size| match level {
                    1..=3 => rem_size * 1,
                    4 => rem_size * 0.9,
                    _ => rem_size * 0.8,
                })
                .code_block(
                    StyleRefinement::default()
                        .bg(cx.theme().transparent)
                        .p_0()
                        .text_size(px(11.)),
                ),
        )
        .selectable(true)
}

pub(super) fn editor_popover(mut container: Stateful<Div>, window: &Window, cx: &App) -> Div {
    let window_size = window.bounds().size;
    let ctx = SurfaceContext {
        blur_enabled: GlobalState::global(cx).blur_enabled(),
    };

    container = container
        .flex_none()
        .occlude()
        .text_xs()
        .p_1()
        .text_color(cx.theme().surface_raised_foreground);

    SurfacePreset::flyout()
        .with_radius(cx.theme().radius)
        .with_elevation(ElevationToken::Md)
        .wrap_with_bounds(
            container,
            window_size.width,
            window_size.height,
            window,
            cx,
            ctx,
        )
}
