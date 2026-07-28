use std::{cell::RefCell, rc::Rc, time::Duration};

use gpui::{
    AnimationExt as _, AnyElement, App, Context, Corner, DismissEvent, Element, ElementId, Entity,
    Focusable, GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels, Point, SharedString,
    StyleRefinement, Styled, Subscription, Window, anchored, deferred, div, prelude::FluentBuilder,
    px,
};

use crate::{
    ActiveTheme,
    animation::{
        PresenceOptions, PresencePhase, SpringPreset, keyed_presence, point_to_point_animation,
        spring_preset_animation, spring_preset_duration_ms,
    },
    global_state::GlobalState,
    menu::PopupMenu,
};

/// A extension trait for adding a context menu to an element.
pub trait ContextMenuExt: ParentElement + Styled {
    /// Add a context menu to the element.
    ///
    /// This will changed the element to be `relative` positioned, and add a child `ContextMenu` element.
    /// Because the `ContextMenu` element is positioned `absolute`, it will not affect the layout of the parent element.
    fn context_menu(
        self,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> ContextMenu<Self>
    where
        Self: Sized,
    {
        // Generate a unique ID based on the element's memory address to ensure
        // each context menu has its own state and doesn't share with others
        let id = format!("context-menu-{:p}", &self as *const _);
        ContextMenu::new(id, self).menu(f)
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
                let (position, open) = {
                    let shared_state = state.shared_state.borrow();
                    (shared_state.position, shared_state.open)
                };
                let menu_view = state.shared_state.borrow().menu_view.clone();
                let mut menu_element = None;
                let reduced_motion = GlobalState::global(cx).reduced_motion();
                let motion = cx.theme().motion.clone();
                let open_duration_ms = if reduced_motion {
                    motion.fast_duration_ms
                } else {
                    spring_preset_duration_ms(&motion, SpringPreset::Mild)
                        .max(motion.fast_duration_ms)
                };
                let presence = keyed_presence(
                    SharedString::from(format!("context-menu-presence-{:?}", this.id)),
                    open,
                    !reduced_motion,
                    Duration::from_millis(u64::from(open_duration_ms)),
                    Duration::from_millis(u64::from(motion.fast_duration_ms)),
                    PresenceOptions::default(),
                    window,
                    cx,
                );

                if presence.should_render() {
                    let has_menu_item = menu_view
                        .as_ref()
                        .map(|menu| !menu.read(cx).is_empty())
                        .unwrap_or(false);

                    if has_menu_item {
                        let open_opacity_animation =
                            point_to_point_animation(&motion, reduced_motion);
                        let open_transform_animation =
                            spring_preset_animation(&motion, reduced_motion, SpringPreset::Mild);
                        let close_animation = point_to_point_animation(&motion, reduced_motion);
                        let positioned_menu = anchored()
                            .position(position)
                            .snap_to_window_with_margin(px(8.))
                            .anchor(anchor)
                            .when_some(menu_view, |this, menu| {
                                if open && !menu.focus_handle(cx).contains_focused(window, cx) {
                                    menu.focus_handle(cx).focus(window, cx);
                                }

                                this.child(menu)
                            });
                        let animated_menu = if presence.transition_active() {
                            if matches!(presence.phase, PresencePhase::Entering) {
                                let transformed = if let Some(animation) = open_transform_animation
                                {
                                    div()
                                        .child(positioned_menu)
                                        .with_animation(
                                            "context-menu-enter-transform",
                                            animation,
                                            |menu, delta| menu.translate_y(px(4.0 * (1.0 - delta))),
                                        )
                                        .into_any_element()
                                } else {
                                    div().child(positioned_menu).into_any_element()
                                };

                                if let Some(animation) = open_opacity_animation {
                                    div()
                                        .child(transformed)
                                        .with_animation(
                                            "context-menu-enter-opacity",
                                            animation,
                                            move |menu, delta| {
                                                menu.opacity(
                                                    presence.progress(delta).clamp(0.0, 1.0),
                                                )
                                            },
                                        )
                                        .into_any_element()
                                } else {
                                    div()
                                        .child(transformed)
                                        .opacity(presence.progress(1.0))
                                        .into_any_element()
                                }
                            } else if let Some(animation) = close_animation {
                                div()
                                    .child(positioned_menu)
                                    .with_animation(
                                        "context-menu-exit",
                                        animation,
                                        move |menu, delta| {
                                            let progress = presence.progress(delta).clamp(0.0, 1.0);
                                            menu.opacity(progress)
                                                .translate_y(px(2.0 * (1.0 - progress)))
                                        },
                                    )
                                    .into_any_element()
                            } else {
                                div().child(positioned_menu).into_any_element()
                            }
                        } else {
                            div().child(positioned_menu).into_any_element()
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
                                        .child(animated_menu),
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
                                    move |_, _: &DismissEvent, window, _cx| {
                                        shared_state.borrow_mut().open = false;
                                        window.refresh();
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
