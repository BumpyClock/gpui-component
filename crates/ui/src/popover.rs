use gpui::{
    Animation, AnimationExt as _, AnyElement, App, Bounds, Context, Corner, DismissEvent,
    ElementId, EventEmitter, FocusHandle, Focusable, InteractiveElement as _, IntoElement,
    KeyBinding, MouseButton, ParentElement, Pixels, Point, Render, RenderOnce, StyleRefinement,
    Styled, Subscription, Window, anchored, deferred, div, prelude::FluentBuilder as _, px,
};
use smol::Timer;
use std::rc::Rc;
use std::time::Duration;

use crate::{
    ActiveTheme as _, ElementExt, Selectable, StyledExt as _, SurfaceContext, SurfacePreset,
    actions::Cancel, animation::cubic_bezier, global_state::GlobalState, v_flex,
};

const CONTEXT: &str = "Popover";
const POPOVER_OPEN_DURATION: Duration = Duration::from_millis(180);
const POPOVER_CLOSE_DURATION: Duration = Duration::from_millis(140);
const POPOVER_MOTION_OFFSET: Pixels = px(6.);
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("escape", Cancel, Some(CONTEXT))])
}

/// A popover element that can be triggered by a button or any other element.
#[derive(IntoElement)]
pub struct Popover {
    id: ElementId,
    style: StyleRefinement,
    anchor: Corner,
    default_open: bool,
    open: Option<bool>,
    tracked_focus_handle: Option<FocusHandle>,
    trigger: Option<Box<dyn FnOnce(bool, &Window, &App) -> AnyElement + 'static>>,
    content: Option<
        Rc<
            dyn Fn(&mut PopoverState, &mut Window, &mut Context<PopoverState>) -> AnyElement
                + 'static,
        >,
    >,
    children: Vec<AnyElement>,
    /// Style for trigger element.
    /// This is used for hotfix the trigger element style to support w_full.
    trigger_style: Option<StyleRefinement>,
    mouse_button: MouseButton,
    appearance: bool,
    overlay_closable: bool,
    on_open_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl Popover {
    /// Create a new Popover with `view` mode.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            anchor: Corner::TopLeft,
            trigger: None,
            trigger_style: None,
            content: None,
            tracked_focus_handle: None,
            children: vec![],
            mouse_button: MouseButton::Left,
            appearance: true,
            overlay_closable: true,
            default_open: false,
            open: None,
            on_open_change: None,
        }
    }

    /// Set the anchor corner of the popover, default is `Corner::TopLeft`.
    pub fn anchor(mut self, anchor: Corner) -> Self {
        self.anchor = anchor;
        self
    }

    /// Set the mouse button to trigger the popover, default is `MouseButton::Left`.
    pub fn mouse_button(mut self, mouse_button: MouseButton) -> Self {
        self.mouse_button = mouse_button;
        self
    }

    /// Set the trigger element of the popover.
    pub fn trigger<T>(mut self, trigger: T) -> Self
    where
        T: Selectable + IntoElement + 'static,
    {
        self.trigger = Some(Box::new(|is_open, _, _| {
            let selected = trigger.is_selected();
            trigger.selected(selected || is_open).into_any_element()
        }));
        self
    }

    /// Set the default open state of the popover, default is `false`.
    ///
    /// This is only used to initialize the open state of the popover.
    ///
    /// And please note that if you use the `open` method, this value will be ignored.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Force set the open state of the popover.
    ///
    /// If this is set, the popover will be controlled by this value.
    ///
    /// NOTE: You must be used in conjunction with `on_open_change` to handle state changes.
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Add a callback to be called when the open state changes.
    ///
    /// The first `&bool` parameter is the **new open state**.
    ///
    /// This is useful when using the `open` method to control the popover state.
    pub fn on_open_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(&bool, &mut Window, &mut App) + 'static,
    {
        self.on_open_change = Some(Rc::new(callback));
        self
    }

    /// Set the style for the trigger element.
    pub fn trigger_style(mut self, style: StyleRefinement) -> Self {
        self.trigger_style = Some(style);
        self
    }

    /// Set whether clicking outside the popover will dismiss it, default is `true`.
    pub fn overlay_closable(mut self, closable: bool) -> Self {
        self.overlay_closable = closable;
        self
    }

    /// Set the content builder for content of the Popover.
    ///
    /// This callback will called every time on render the popover.
    /// So, you should avoid creating new elements or entities in the content closure.
    pub fn content<F, E>(mut self, content: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut PopoverState, &mut Window, &mut Context<PopoverState>) -> E + 'static,
    {
        self.content = Some(Rc::new(move |state, window, cx| {
            content(state, window, cx).into_any_element()
        }));
        self
    }

    /// Set whether the popover no style, default is `false`.
    ///
    /// If no style:
    ///
    /// - The popover will not have a bg, border, shadow, or padding.
    /// - The click out of the popover will not dismiss it.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    /// Bind the focus handle to receive focus when the popover is opened.
    /// If you not set this, a new focus handle will be created for the popover to
    ///
    /// If popover is opened, the focus will be moved to the focus handle.
    pub fn track_focus(mut self, handle: &FocusHandle) -> Self {
        self.tracked_focus_handle = Some(handle.clone());
        self
    }

    fn resolved_corner(anchor: Corner, bounds: Bounds<Pixels>) -> Point<Pixels> {
        bounds.corner(match anchor {
            Corner::TopLeft => Corner::BottomLeft,
            Corner::TopRight => Corner::BottomRight,
            Corner::BottomLeft => Corner::TopLeft,
            Corner::BottomRight => Corner::TopRight,
        }) + Point {
            x: px(0.),
            y: -bounds.size.height,
        }
    }
}

impl ParentElement for Popover {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Popover {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

pub struct PopoverState {
    focus_handle: FocusHandle,
    pub(crate) tracked_focus_handle: Option<FocusHandle>,
    trigger_bounds: Option<Bounds<Pixels>>,
    open: bool,
    closing: bool,
    closing_id: u64,
    on_open_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,

    _dismiss_subscription: Option<Subscription>,
}

impl PopoverState {
    pub fn new(default_open: bool, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            tracked_focus_handle: None,
            trigger_bounds: None,
            open: default_open,
            closing: false,
            closing_id: 0,
            on_open_change: None,
            _dismiss_subscription: None,
        }
    }

    /// Check if the popover is open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Dismiss the popover if it is open.
    pub fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.set_open(false, window, cx);
        }
    }

    /// Open the popover if it is closed.
    pub fn show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            self.set_open(true, window, cx);
        }
    }

    fn toggle_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.set_open(!self.open, window, cx);
    }

    fn set_open(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        if open {
            if self.open && !self.closing {
                return;
            }
            self.open = true;
            self.closing = false;
            self.closing_id = self.closing_id.wrapping_add(1);

            let state = cx.entity();
            let focus_handle = if let Some(tracked_focus_handle) = self.tracked_focus_handle.clone()
            {
                tracked_focus_handle
            } else {
                self.focus_handle.clone()
            };
            focus_handle.focus(window, cx);

            self._dismiss_subscription =
                Some(
                    window.subscribe(&cx.entity(), cx, move |_, _: &DismissEvent, window, cx| {
                        state.update(cx, |state, cx| {
                            state.dismiss(window, cx);
                        });
                        window.refresh();
                    }),
                );
        } else {
            if !self.open && !self.closing {
                return;
            }
            self.open = false;
            self._dismiss_subscription = None;

            if GlobalState::global(cx).reduced_motion() {
                self.closing = false;
            } else {
                self.closing = true;
                self.closing_id = self.closing_id.wrapping_add(1);
                let closing_id = self.closing_id;
                cx.spawn(async move |view, cx| {
                    Timer::after(POPOVER_CLOSE_DURATION).await;
                    cx.update(|cx| {
                        if let Some(view) = view.upgrade() {
                            view.update(cx, |state, cx| {
                                if state.closing && state.closing_id == closing_id {
                                    state.closing = false;
                                    cx.notify();
                                }
                            });
                        }
                    })
                })
                .detach();
            }
        }

        if let Some(callback) = self.on_open_change.as_ref() {
            callback(&self.open, window, cx);
        }
        cx.notify();
    }

    fn on_action_cancel(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        self.dismiss(window, cx);
    }
}

impl Focusable for PopoverState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PopoverState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

impl EventEmitter<DismissEvent> for PopoverState {}

impl RenderOnce for Popover {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let force_open = self.open;
        let default_open = self.default_open;
        let tracked_focus_handle = self.tracked_focus_handle.clone();
        let state = window.use_keyed_state(self.id.clone(), cx, |_, cx| {
            PopoverState::new(default_open, cx)
        });

        state.update(cx, |state, _| {
            if let Some(tracked_focus_handle) = tracked_focus_handle {
                state.tracked_focus_handle = Some(tracked_focus_handle);
            }
            state.on_open_change = self.on_open_change.clone();
            if let Some(force_open) = force_open {
                state.open = force_open;
            }
        });

        let open = state.read(cx).open;
        let closing = state.read(cx).closing;
        let is_open = open || closing;
        let focus_handle = state.read(cx).focus_handle.clone();
        let trigger_bounds = state.read(cx).trigger_bounds;

        let Some(trigger) = self.trigger else {
            return div().id("empty");
        };

        let parent_view_id = window.current_view();

        let el = div()
            .id(self.id)
            .child((trigger)(is_open, window, cx))
            .on_mouse_up(self.mouse_button, {
                let state = state.clone();
                move |_, window, cx| {
                    cx.stop_propagation();
                    state.update(cx, |state, cx| {
                        // We force set open to false to toggle it correctly.
                        // Because if the mouse down out will toggle open first.
                        state.open = open;
                        state.toggle_open(window, cx);
                    });
                    cx.notify(parent_view_id);
                }
            })
            .on_prepaint({
                let state = state.clone();
                move |bounds, _, cx| {
                    state.update(cx, |state, _| {
                        state.trigger_bounds = Some(bounds);
                    })
                }
            });

        if !is_open {
            return el;
        }

        let window_size = window.bounds().size;
        let ctx = SurfaceContext {
            blur_enabled: GlobalState::global(cx).blur_enabled(),
        };
        let reduced_motion = GlobalState::global(cx).reduced_motion();
        let is_closing = closing;
        let motion_direction = match self.anchor {
            Corner::TopLeft | Corner::TopRight => 1.0,
            Corner::BottomLeft | Corner::BottomRight => -1.0,
        };

        el.child(
            deferred(
                anchored()
                    .snap_to_window_with_margin(px(8.))
                    .anchor(self.anchor)
                    .when_some(trigger_bounds, |this, trigger_bounds| {
                        this.position(Self::resolved_corner(self.anchor, trigger_bounds))
                    })
                    .child({
                        let content = v_flex()
                            .id("content")
                            .track_focus(&focus_handle)
                            .key_context(CONTEXT)
                            .on_action(window.listener_for(&state, PopoverState::on_action_cancel))
                            .size_full()
                            .occlude()
                            .tab_group()
                            .map(|this| match self.anchor {
                                Corner::TopLeft | Corner::TopRight => this.top_1(),
                                Corner::BottomLeft | Corner::BottomRight => this.bottom_1(),
                            })
                            .when_some(self.content, |this, content| {
                                this.child(
                                    state.update(cx, |state, cx| (content)(state, window, cx)),
                                )
                            })
                            .children(self.children)
                            .when(self.overlay_closable, |this| {
                                this.on_mouse_up_out(MouseButton::Left, {
                                    let state = state.clone();
                                    move |_, window, cx| {
                                        state.update(cx, |state, cx| {
                                            state.dismiss(window, cx);
                                        });
                                        cx.notify(parent_view_id);
                                    }
                                })
                            })
                            .refine_style(&self.style);

                        let mut container = if self.appearance {
                            SurfacePreset::flyout()
                                .with_radius(cx.theme().radius)
                                .wrap_with_bounds(
                                    content
                                        .text_color(cx.theme().surface_raised_foreground)
                                        .p_3(),
                                    window_size.width,
                                    window_size.height,
                                    window,
                                    cx,
                                    ctx,
                                )
                                .into_any_element()
                        } else {
                            content.into_any_element()
                        };

                        if !reduced_motion {
                            let duration = if is_closing {
                                POPOVER_CLOSE_DURATION
                            } else {
                                POPOVER_OPEN_DURATION
                            };
                            let easing = cubic_bezier(0.25, 1.0, 0.5, 1.0);
                            container = div()
                                .relative()
                                .child(container)
                                .with_animation(
                                    ElementId::NamedInteger(
                                        "popover-motion".into(),
                                        is_closing as u64,
                                    ),
                                    Animation::new(duration).with_easing(easing),
                                    move |this, delta| {
                                        let offset = POPOVER_MOTION_OFFSET * motion_direction;
                                        if is_closing {
                                            this.opacity(1.0 - delta).top(offset * delta)
                                        } else {
                                            this.opacity(delta).top(offset * (1.0 - delta))
                                        }
                                    },
                                )
                                .into_any_element();
                        }

                        container
                    }),
            )
            .with_priority(1),
        )
    }
}
