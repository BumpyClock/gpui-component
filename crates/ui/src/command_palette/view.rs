//! View component for the Command Palette.

use super::provider::CommandPaletteProvider;
use super::state::{CommandPaletteEvent, CommandPaletteState};
use super::types::{CommandPaletteConfig, MatchedItem};
use crate::actions::{Cancel, Confirm, SelectDown, SelectUp};
use crate::global_state::GlobalState;
use crate::input::{Input, InputEvent, InputState};
use crate::kbd::Kbd;
use crate::{
    ActiveTheme, Icon, IconName, Sizable, Size, SurfaceContext, SurfacePreset,
    VirtualListScrollHandle, WindowExt as _, h_flex, v_flex, v_virtual_list,
};
use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyBinding, ParentElement, Pixels, Render, ScrollStrategy, SharedString, Size as GpuiSize,
    Styled, Subscription, Window, div, prelude::FluentBuilder, px,
};
use std::rc::Rc;
use std::sync::Arc;

const CONTEXT: &str = "CommandPalette";

// Height constants for layout calculations
const HEADER_HEIGHT: f32 = 52.0;
const FOOTER_HEIGHT: f32 = 36.0;

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
        KeyBinding::new("enter", Confirm { secondary: false }, Some(CONTEXT)),
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
    ]);
}

/// The Command Palette view component.
pub struct CommandPaletteView {
    /// The internal state entity.
    pub(crate) state: Entity<CommandPaletteState>,
    /// The search input state.
    input_state: Entity<InputState>,
    /// Focus handle for the palette.
    focus_handle: FocusHandle,
    /// Scroll handle for the list.
    scroll_handle: VirtualListScrollHandle,
    /// Item height for virtualization.
    item_height: Pixels,
    /// Tracks whether we've focused the input once after open.
    did_focus: bool,
    /// Subscriptions.
    _subscriptions: Vec<Subscription>,
}

impl CommandPaletteView {
    /// Create a new command palette view.
    pub fn new(
        config: CommandPaletteConfig,
        provider: Arc<dyn CommandPaletteProvider>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let placeholder = config.placeholder.clone();

        let state = cx.new(|cx| CommandPaletteState::new(config, provider, window, cx));

        let input_state = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));

        let focus_handle = cx.focus_handle();

        // Subscribe to input changes
        let input_subscription = cx.subscribe_in(&input_state, window, Self::on_input_event);

        // Subscribe to state events
        let state_subscription = cx.subscribe_in(&state, window, Self::on_state_event);

        Self {
            state,
            input_state,
            focus_handle,
            scroll_handle: VirtualListScrollHandle::new(),
            item_height: px(48.),
            did_focus: false,
            _subscriptions: vec![input_subscription, state_subscription],
        }
    }

    fn on_input_event(
        &mut self,
        _: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                let query = self.input_state.read(cx).value().to_string();
                self.state.update(cx, |state, cx| {
                    state.set_query(query, window, cx);
                });
            }
            InputEvent::PressEnter { .. } => {
                self.state.update(cx, |state, cx| {
                    state.confirm(cx);
                });
            }
            _ => {}
        }
    }

    fn on_state_event(
        &mut self,
        _: &Entity<CommandPaletteState>,
        event: &CommandPaletteEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Close the dialog on selection or dismissal
        match event {
            CommandPaletteEvent::Selected { .. } | CommandPaletteEvent::Dismissed => {
                window.close_dialog(cx);
            }
        }
        // Forward events
        cx.emit(event.clone());
    }

    fn on_action_cancel(&mut self, _: &Cancel, _: &mut Window, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| {
            state.dismiss(cx);
        });
    }

    fn on_action_confirm(&mut self, _: &Confirm, _: &mut Window, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| {
            state.confirm(cx);
        });
    }

    fn on_action_select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| {
            state.select_prev(cx);
        });
        self.scroll_to_selected(cx);
    }

    fn on_action_select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| {
            state.select_next(cx);
        });
        self.scroll_to_selected(cx);
    }

    fn scroll_to_selected(&mut self, cx: &App) {
        if let Some(index) = self.state.read(cx).selected_index {
            self.scroll_handle
                .scroll_to_item(index, ScrollStrategy::Top);
        }
    }

    fn render_item(
        &self,
        item: &MatchedItem,
        index: usize,
        selected: bool,
        show_category: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let item_data = item.item.clone();
        let match_info = item.match_info.clone();
        let disabled = item_data.disabled;

        let shortcut_element = item_data
            .shortcut
            .as_ref()
            .and_then(|s| gpui::Keystroke::parse(s).ok().map(|k| Kbd::new(k)));

        h_flex()
            .id(SharedString::from(format!("cmd-item-{}", index)))
            .w_full()
            .h(self.item_height)
            .px_3()
            .gap_3()
            .items_center()
            .rounded(cx.theme().radius)
            .cursor_pointer()
            .my_1()
            .when(disabled, |this| this.opacity(0.5).cursor_not_allowed())
            .when(selected && !disabled, |this| {
                this.bg(cx.theme().list_active)
                    .text_color(cx.theme().accent_foreground)
            })
            .when(!selected && !disabled, |this| {
                this.hover(|this| this.bg(cx.theme().list_hover))
            })
            .when(!disabled, |this| {
                let index = index;
                this.on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(move |view, _, _, cx| {
                        view.state.update(cx, |state, cx| {
                            state.select_index(index, cx);
                            state.confirm(cx);
                        });
                    }),
                )
            })
            // Icon
            .when_some(item_data.icon, |this, icon| {
                this.child(
                    Icon::new(icon)
                        .size_4()
                        .text_color(cx.theme().muted_foreground),
                )
            })
            // Title and subtitle
            .child(
                v_flex()
                    .flex_1()
                    .overflow_hidden()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(if selected {
                                cx.theme().accent_foreground
                            } else {
                                cx.theme().foreground
                            })
                            .truncate()
                            .child(self.render_highlighted_text(
                                &item_data.title,
                                &match_info.title_ranges,
                                cx,
                            )),
                    )
                    .when_some(item_data.subtitle.clone(), |this, subtitle| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(if selected {
                                    cx.theme().accent_foreground.opacity(0.8)
                                } else {
                                    cx.theme().muted_foreground
                                })
                                .truncate()
                                .child(subtitle),
                        )
                    }),
            )
            // Shortcut
            .when_some(shortcut_element, |this, kbd| this.child(kbd))
            // Category (inline, like old palette)
            .when(show_category && !item_data.category.is_empty(), |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(item_data.category.clone()),
                )
            })
    }

    fn render_highlighted_text(
        &self,
        text: &str,
        ranges: &[(usize, usize)],
        cx: &App,
    ) -> impl IntoElement {
        if ranges.is_empty() {
            return div().truncate().child(text.to_string()).into_any_element();
        }

        let mut elements = Vec::new();
        let mut last_end = 0;

        for &(start, end) in ranges {
            // Text before the highlight
            if start > last_end {
                elements.push(
                    div()
                        .child(text[last_end..start].to_string())
                        .into_any_element(),
                );
            }
            // Highlighted text
            let end = end.min(text.len());
            if start < end {
                elements.push(
                    div()
                        .text_color(cx.theme().accent)
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(text[start..end].to_string())
                        .into_any_element(),
                );
            }
            last_end = end;
        }

        // Remaining text
        if last_end < text.len() {
            elements.push(div().child(text[last_end..].to_string()).into_any_element());
        }

        h_flex().truncate().children(elements).into_any_element()
    }

    fn render_footer(&self, cx: &App) -> impl IntoElement {
        h_flex()
            .w_full()
            .px_3()
            .py_2()
            .border_t_1()
            .border_color(cx.theme().border_default)
            .justify_between()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(
                h_flex()
                    .gap_3()
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Kbd::new(gpui::Keystroke::parse("up").unwrap()).appearance(false),
                            )
                            .child(
                                Kbd::new(gpui::Keystroke::parse("down").unwrap()).appearance(false),
                            )
                            .child("to navigate"),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Kbd::new(gpui::Keystroke::parse("enter").unwrap())
                                    .appearance(false),
                            )
                            .child("to select"),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Kbd::new(gpui::Keystroke::parse("escape").unwrap())
                                    .appearance(false),
                            )
                            .child("to close"),
                    ),
            )
    }

    fn render_empty(&self, cx: &App) -> impl IntoElement {
        v_flex()
            .size_full()
            .justify_center()
            .items_center()
            .py_8()
            .gap_2()
            .text_color(cx.theme().muted_foreground)
            .child(Icon::new(IconName::Search).size_8().opacity(0.5))
            .child("No results found")
    }
}

impl gpui::EventEmitter<CommandPaletteEvent> for CommandPaletteView {}

impl Focusable for CommandPaletteView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CommandPaletteView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state.read(cx);
        let config = state.config.clone();
        let matched_items = state.matched_items.clone();
        let selected_index = state.selected_index;
        let items_count = matched_items.len();

        // Prepare item sizes for virtual list
        let item_sizes: Rc<Vec<GpuiSize<Pixels>>> = Rc::new(
            (0..items_count)
                .map(|_| GpuiSize {
                    width: px(0.),
                    height: self.item_height,
                })
                .collect(),
        );

        let show_categories = config.show_categories_inline;
        let show_footer = config.show_footer;
        let max_height = px(config.max_height);

        // Focus input once after opening to avoid render jitter
        if !self.did_focus {
            self.input_state.update(cx, |input, cx| {
                input.focus(window, cx);
            });
            self.did_focus = true;
        }

        // Compute height for surface bounds
        let list_height =
            max_height.min(px(items_count.max(1) as f32 * (self.item_height / px(1.0))));
        let content_height = px(HEADER_HEIGHT)
            + list_height
            + if show_footer {
                px(FOOTER_HEIGHT)
            } else {
                px(0.0)
            };

        let surface_ctx = SurfaceContext {
            blur_enabled: GlobalState::global(cx).blur_enabled(),
        };

        let content = v_flex()
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_action_cancel))
            .on_action(cx.listener(Self::on_action_confirm))
            .on_action(cx.listener(Self::on_action_select_up))
            .on_action(cx.listener(Self::on_action_select_down))
            .h(content_height)
            .w_full()
            .overflow_hidden()
            // Search input
            .child(
                div()
                    .w_full()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(cx.theme().border_default)
                    .child(
                        Input::new(&self.input_state)
                            .with_size(Size::Medium)
                            .prefix(
                                Icon::new(IconName::Search)
                                    .size_4()
                                    .text_color(cx.theme().muted_foreground),
                            )
                            .appearance(false)
                            .cleanable(true),
                    ),
            )
            // Results list
            .child(
                div()
                    .w_full()
                    .h(list_height)
                    .overflow_hidden()
                    .when(items_count == 0, |this| this.child(self.render_empty(cx)))
                    .when(items_count > 0, |this| {
                        this.child(
                            v_virtual_list(cx.entity(), "command-palette-list", item_sizes, {
                                let matched_items = matched_items.clone();
                                move |view, visible_range, window, cx| {
                                    visible_range
                                        .filter_map(|ix| {
                                            matched_items.get(ix).map(|item| {
                                                view.render_item(
                                                    item,
                                                    ix,
                                                    selected_index == Some(ix),
                                                    show_categories,
                                                    window,
                                                    cx,
                                                )
                                                .into_any_element()
                                            })
                                        })
                                        .collect()
                                }
                            })
                            .track_scroll(&self.scroll_handle)
                            .py_1(),
                        )
                    }),
            )
            // Footer
            .when(show_footer, |this| this.child(self.render_footer(cx)));

        // Wrap in glassmorphic surface
        SurfacePreset::flyout()
            .wrap_with_bounds(
                content,
                px(config.width),
                content_height,
                window,
                cx,
                surface_ctx,
            )
            .h(content_height)
            .w(px(config.width))
    }
}
