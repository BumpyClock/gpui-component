use std::{cell::RefCell, rc::Rc, time::Duration};

use gpui::{
    Animation, AnimationExt as _, AnyElement, App, Context, Corner, DismissEvent, Element,
    ElementId, Entity, Focusable, GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels, Point,
    StyleRefinement, Styled, Subscription, Window, anchored, deferred, div,
    prelude::FluentBuilder, px,
};
use smol::Timer;

use crate::{animation::cubic_bezier, global_state::GlobalState, menu::PopupMenu};

const CONTEXT_MENU_OPEN_DURATION: Duration = Duration::from_millis(180);
const CONTEXT_MENU_CLOSE_DURATION: Duration = Duration::from_millis(140);
const CONTEXT_MENU_MOTION_OFFSET: Pixels = px(6.);

/// A extension trait for adding a context menu to an element.
pub trait ContextMenuExt: ParentElement + Styled {
    /// Add a context menu to the element.
    ///
    /// This will changed the element to be `relative` positioned, and add a child `ContextMenu` element.
    /// Because the `ContextMenu` element is positioned `absolute`, it will not affect the layout of the parent element.
    fn context_menu(
        self,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> ContextMenu<Self> {
        ContextMenu::new("context-menu", self).menu(f)
    }
}

impl<E: ParentElement + Styled> ContextMenuExt for E {}

/// A context menu that can be shown on right-click.
pub struct ContextMenu<E: ParentElement + Styled + Sized> {
    id: ElementId,
    element: Option<E>,
    menu: Option<Rc<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu>>,
    // This is not in use, just for style refinement forwarding.
    _ignore_style: StyleRefinement,
    anchor: Corner,
}

impl<E: ParentElement + Styled> ContextMenu<E> {
    /// Create a new context menu with the given ID.
    pub fn new(id: impl Into<ElementId>, element: E) -> Self {
        Self {
            id: id.into(),
            element: Some(element),
            menu: None,
            anchor: Corner::TopLeft,
            _ignore_style: StyleRefinement::default(),
        }
    }

    /// Build the context menu using the given builder function.
    #[must_use]
    fn menu<F>(mut self, builder: F) -> Self
    where
        F: Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    {
        self.menu = Some(Rc::new(builder));
        self
    }

    fn with_element_state<R>(
        &mut self,
        id: &GlobalElementId,
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(&mut Self, &mut ContextMenuState, &mut Window, &mut App) -> R,
    ) -> R {
        window.with_optional_element_state::<ContextMenuState, _>(
            Some(id),
            |element_state, window| {
                let mut element_state = element_state.unwrap().unwrap_or_default();
                let result = f(self, &mut element_state, window, cx);
                (result, Some(element_state))
            },
        )
    }
}

impl<E: ParentElement + Styled> ParentElement for ContextMenu<E> {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        if let Some(element) = &mut self.element {
            element.extend(elements);
        }
    }
}

impl<E: ParentElement + Styled> Styled for ContextMenu<E> {
    fn style(&mut self) -> &mut StyleRefinement {
        if let Some(element) = &mut self.element {
            element.style()
        } else {
            &mut self._ignore_style
        }
    }
}

impl<E: ParentElement + Styled + IntoElement + 'static> IntoElement for ContextMenu<E> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

struct ContextMenuSharedState {
    menu_view: Option<Entity<PopupMenu>>,
    open: bool,
    closing: bool,
    closing_id: u64,
    position: Point<Pixels>,
    _subscription: Option<Subscription>,
}

pub struct ContextMenuState {
    element: Option<AnyElement>,
    shared_state: Rc<RefCell<ContextMenuSharedState>>,
}

impl Default for ContextMenuState {
    fn default() -> Self {
        Self {
            element: None,
            shared_state: Rc::new(RefCell::new(ContextMenuSharedState {
                menu_view: None,
                open: false,
                closing: false,
                closing_id: 0,
                position: Default::default(),
                _subscription: None,
            })),
        }
    }
}

impl<E: ParentElement + Styled + IntoElement + 'static> Element for ContextMenu<E> {
    type RequestLayoutState = ContextMenuState;
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let anchor = self.anchor;

        self.with_element_state(
            id.unwrap(),
            window,
            cx,
            |this, state: &mut ContextMenuState, window, cx| {
                let (position, open, closing, menu_view) = {
                    let shared_state = state.shared_state.borrow();
                    (
                        shared_state.position,
                        shared_state.open,
                        shared_state.closing,
                        shared_state.menu_view.clone(),
                    )
                };
                let mut menu_element = None;
                if open || closing {
                    let has_menu_item = menu_view
                        .as_ref()
                        .map(|menu| !menu.read(cx).is_empty())
                        .unwrap_or(false);

                    if has_menu_item {
                        let reduced_motion = GlobalState::global(cx).reduced_motion();
                        let motion_direction = match anchor {
                            Corner::TopLeft | Corner::TopRight => 1.0,
                            Corner::BottomLeft | Corner::BottomRight => -1.0,
                        };
                        menu_element = Some(
                            deferred(
                                anchored().child(
                                    div()
                                        .w(window.bounds().size.width)
                                        .h(window.bounds().size.height)
                                        .on_scroll_wheel(|_, _, cx| {
                                            cx.stop_propagation();
                                        })
                                        .child(
                                            anchored()
                                                .position(position)
                                                .snap_to_window_with_margin(px(8.))
                                                .anchor(anchor)
                                                .when_some(menu_view, |this, menu| {
                                                    // Focus the menu, so that can be handle the action.
                                                    if !menu
                                                        .focus_handle(cx)
                                                        .contains_focused(window, cx)
                                                    {
                                                        menu.focus_handle(cx).focus(window, cx);
                                                    }
                                                    let menu = menu.clone().into_any_element();
                                                    let menu = if reduced_motion {
                                                        menu
                                                    } else {
                                                        let is_closing = closing;
                                                        let duration = if is_closing {
                                                            CONTEXT_MENU_CLOSE_DURATION
                                                        } else {
                                                            CONTEXT_MENU_OPEN_DURATION
                                                        };
                                                        let easing = cubic_bezier(0.25, 1.0, 0.5, 1.0);
                                                        div()
                                                            .relative()
                                                            .child(menu)
                                                            .with_animation(
                                                                ElementId::NamedInteger(
                                                                    "context-menu-motion".into(),
                                                                    is_closing as u64,
                                                                ),
                                                                Animation::new(duration)
                                                                    .with_easing(easing),
                                                                move |this, delta| {
                                                                    let offset = CONTEXT_MENU_MOTION_OFFSET
                                                                        * motion_direction;
                                                                    if is_closing {
                                                                        this.opacity(1.0 - delta)
                                                                            .top(offset * delta)
                                                                    } else {
                                                                        this.opacity(delta).top(
                                                                            offset * (1.0 - delta),
                                                                        )
                                                                    }
                                                                },
                                                            )
                                                            .into_any_element()
                                                    };

                                                    this.child(menu)
                                                }),
                                        ),
                                ),
                            )
                            .with_priority(1)
                            .into_any(),
                        );
                    }
                }

                let mut element = this
                    .element
                    .take()
                    .expect("Element should exists.")
                    .children(menu_element)
                    .into_any_element();

                let layout_id = element.request_layout(window, cx);

                (
                    layout_id,
                    ContextMenuState {
                        element: Some(element),
                        ..Default::default()
                    },
                )
            },
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: gpui::Bounds<gpui::Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if let Some(element) = &mut request_layout.element {
            element.prepaint(window, cx);
        }
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: gpui::Bounds<gpui::Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(element) = &mut request_layout.element {
            element.paint(window, cx);
        }

        // Take the builder before setting up element state to avoid borrow issues
        let builder = self.menu.clone();

        self.with_element_state(
            id.unwrap(),
            window,
            cx,
            |_view, state: &mut ContextMenuState, window, _| {
                let shared_state = state.shared_state.clone();

                let hitbox = hitbox.clone();
                // When right mouse click, to build content menu, and show it at the mouse position.
                window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                    if phase.bubble()
                        && event.button == MouseButton::Right
                        && hitbox.is_hovered(window)
                    {
                        {
                            let mut shared_state = shared_state.borrow_mut();
                            // Clear any existing menu view to allow immediate replacement
                            // Set the new position and open the menu
                            shared_state.menu_view = None;
                            shared_state._subscription = None;
                            shared_state.position = event.position;
                            shared_state.open = true;
                            shared_state.closing = false;
                            shared_state.closing_id = shared_state.closing_id.wrapping_add(1);
                        }

                        // Use defer to build the menu in the next frame, avoiding race conditions
                        window.defer(cx, {
                            let shared_state = shared_state.clone();
                            let builder = builder.clone();
                            move |window, cx| {
                                let menu = PopupMenu::build(window, cx, move |menu, window, cx| {
                                    let Some(build) = &builder else {
                                        return menu;
                                    };
                                    build(menu, window, cx)
                                });

                                // Set up the subscription for dismiss handling
                                let _subscription = window.subscribe(&menu, cx, {
                                    let shared_state = shared_state.clone();
                                    move |_, _: &DismissEvent, window, cx| {
                                        let reduced_motion =
                                            GlobalState::global(cx).reduced_motion();
                                        let closing_id = {
                                            let mut state = shared_state.borrow_mut();
                                            if reduced_motion {
                                                state.open = false;
                                                state.closing = false;
                                                state.menu_view = None;
                                                state.closing_id = state.closing_id.wrapping_add(1);
                                                window.refresh();
                                                return;
                                            }

                                            state.open = false;
                                            state.closing = true;
                                            state.closing_id = state.closing_id.wrapping_add(1);
                                            let closing_id = state.closing_id;
                                            window.refresh();
                                            closing_id
                                        };

                                        let shared_state = shared_state.clone();
                                        window
                                            .spawn(cx, async move |cx| {
                                                Timer::after(CONTEXT_MENU_CLOSE_DURATION).await;
                                                cx.update(|_, _| {
                                                    let mut state = shared_state.borrow_mut();
                                                    if state.closing && state.closing_id == closing_id
                                                    {
                                                        state.closing = false;
                                                        state.menu_view = None;
                                                    }
                                                })
                                                .ok();
                                            })
                                            .detach();
                                    }
                                });

                                // Update the shared state with the built menu and subscription
                                {
                                    let mut state = shared_state.borrow_mut();
                                    state.menu_view = Some(menu.clone());
                                    state._subscription = Some(_subscription);
                                    window.refresh();
                                }
                            }
                        });
                    }
                });
            },
        );
    }
}
