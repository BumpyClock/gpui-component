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
const SECTION_HEADER_HEIGHT: f32 = 28.0;

/// A render row for the command palette list.
#[derive(Clone)]
enum CommandPaletteRow {
    Header(SharedString),
    Item(usize),
}

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
        let state = self.state.read(cx);
        if let Some(index) = state.selected_index {
            let row_index = self.row_index_for_item(&state, index);
            self.scroll_handle
                .scroll_to_item(row_index, ScrollStrategy::Top);
        }
    }

    fn render_item(
        &self,
        item: &MatchedItem,
        item_index: usize,
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
            .id(SharedString::from(format!("cmd-item-{}", item_index)))
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
                let index = item_index;
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

    fn render_section_header(&self, title: SharedString, cx: &App) -> impl IntoElement {
        div()
            .w_full()
            .px_3()
            .py_1()
            .text_xs()
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(cx.theme().muted_foreground)
            .child(title)
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

    fn render_footer(&self, status_text: Option<SharedString>, cx: &App) -> impl IntoElement {
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
            .when_some(status_text, |this, status| {
                this.child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .text_color(cx.theme().muted_foreground)
                        .child(Icon::new(IconName::LoaderCircle).size_4())
                        .child(status),
                )
            })
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

    fn build_rows(
        &self,
        state: &CommandPaletteState,
        matched_items: &[MatchedItem],
    ) -> Vec<CommandPaletteRow> {
        let static_len = state.matched_static_len.min(matched_items.len());
        let async_len = matched_items.len().saturating_sub(static_len);
        let query_empty = state.query.is_empty();

        let mut rows = Vec::new();
        if query_empty {
            rows.extend((0..static_len).map(CommandPaletteRow::Item));
            return rows;
        }

        if static_len > 0 {
            if let Some(title) = state.config.commands_section_title.clone() {
                rows.push(CommandPaletteRow::Header(title));
            }
            rows.extend((0..static_len).map(CommandPaletteRow::Item));
        }

        if async_len > 0 {
            if let Some(title) = state.config.results_section_title.clone() {
                rows.push(CommandPaletteRow::Header(title));
            }
            rows.extend((static_len..matched_items.len()).map(CommandPaletteRow::Item));
        }

        rows
    }

    fn row_index_for_item(&self, state: &CommandPaletteState, item_index: usize) -> usize {
        let static_len = state.matched_static_len.min(state.matched_items.len());
        let async_len = state.matched_items.len().saturating_sub(static_len);
        let query_empty = state.query.is_empty();

        if query_empty {
            return item_index;
        }

        let commands_header = static_len > 0 && state.config.commands_section_title.is_some();
        let results_header = async_len > 0 && state.config.results_section_title.is_some();
        let mut row_index = item_index + usize::from(commands_header);
        if item_index >= static_len && results_header {
            row_index += 1;
        }
        row_index
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
        let rows = Rc::new(self.build_rows(&state, &matched_items));
        let row_count = rows.len();

        // Prepare item sizes for virtual list
        let item_sizes: Rc<Vec<GpuiSize<Pixels>>> = Rc::new(
            rows.iter()
                .map(|row| GpuiSize {
                    width: px(0.),
                    height: match row {
                        CommandPaletteRow::Header(_) => px(SECTION_HEADER_HEIGHT),
                        CommandPaletteRow::Item(_) => self.item_height,
                    },
                })
                .collect(),
        );

        let show_categories = config.show_categories_inline;
        let show_footer = config.show_footer;
        let footer_status = config
            .status_provider
            .as_ref()
            .and_then(|provider| provider(&state.query));
        let max_height = px(config.max_height);

        // Focus input once after opening to avoid render jitter
        if !self.did_focus {
            self.input_state.update(cx, |input, cx| {
                input.focus(window, cx);
            });
            self.did_focus = true;
        }

        // Compute height for surface bounds
        let list_content_height = if row_count == 0 {
            self.item_height
        } else {
            rows.iter().fold(px(0.0), |sum, row| {
                sum + match row {
                    CommandPaletteRow::Header(_) => px(SECTION_HEADER_HEIGHT),
                    CommandPaletteRow::Item(_) => self.item_height,
                }
            })
        };
        let list_height = max_height.min(list_content_height);
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
                    .when(row_count == 0, |this| this.child(self.render_empty(cx)))
                    .when(row_count > 0, |this| {
                        this.child(
                            v_virtual_list(cx.entity(), "command-palette-list", item_sizes, {
                                let matched_items = matched_items.clone();
                                let rows = rows.clone();
                                move |view, visible_range, window, cx| {
                                    visible_range
                                        .filter_map(|ix| {
                                            let row = rows.get(ix)?;
                                            match row {
                                                CommandPaletteRow::Header(title) => Some(
                                                    view.render_section_header(title.clone(), cx)
                                                        .into_any_element(),
                                                ),
                                                CommandPaletteRow::Item(item_index) => {
                                                    matched_items.get(*item_index).map(|item| {
                                                        view.render_item(
                                                            item,
                                                            *item_index,
                                                            selected_index == Some(*item_index),
                                                            show_categories,
                                                            window,
                                                            cx,
                                                        )
                                                        .into_any_element()
                                                    })
                                                }
                                            }
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
            .when(show_footer, |this| {
                this.child(self.render_footer(footer_status.clone(), cx))
            });

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
