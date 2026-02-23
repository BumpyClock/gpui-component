//! FloatingSidebar combines SidebarShell and Sidebar with internal resize handling.

use std::rc::Rc;

use gpui::{
    App, Element, ElementId, Hsla, IntoElement, MouseMoveEvent, MouseUpEvent, ParentElement,
    Pixels, RenderOnce, SharedString, Style, StyleRefinement, Styled, Window, px,
};

use crate::{
    ElevationToken, Side, StyledExt,
    sidebar::{COLLAPSED_WIDTH, DEFAULT_WIDTH, Sidebar, SidebarItem},
    sidebar_shell::SidebarShell,
};

/// Default values for floating sidebar configuration.
const DEFAULT_MIN_WIDTH: Pixels = px(200.0);
const DEFAULT_MAX_WIDTH: Pixels = px(400.0);
const DEFAULT_RESIZER_WIDTH: Pixels = px(6.0);
const DEFAULT_INSET: Pixels = px(8.0);

#[derive(Clone)]
struct FloatingSidebarState {
    expanded_width: Pixels,
    resizing: bool,
    drag_origin_x: Pixels,
    drag_origin_width: Pixels,
}

impl FloatingSidebarState {
    fn new(width: Pixels) -> Self {
        Self {
            expanded_width: width,
            resizing: false,
            drag_origin_x: px(0.0),
            drag_origin_width: width,
        }
    }
}

/// A floating sidebar that composes SidebarShell and Sidebar with internal resize handling.
#[derive(IntoElement)]
pub struct FloatingSidebar<E: SidebarItem + 'static> {
    id: ElementId,
    sidebar: Sidebar<E>,
    style: StyleRefinement,
    side: Side,
    collapsed: bool,
    width: Pixels,
    min_width: Pixels,
    max_width: Pixels,
    resizer_width: Pixels,
    resizer_hover_bg: Option<Hsla>,
    inset: Option<Pixels>,
    top_inset: Pixels,
    blur_enabled: Option<bool>,
    elevation: ElevationToken,
    on_resize_end: Option<Rc<dyn Fn(Pixels, &mut Window, &mut App)>>,
}

impl<E: SidebarItem> FloatingSidebar<E> {
    /// Create a new FloatingSidebar with the given ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            sidebar: Sidebar::new(id),
            style: StyleRefinement::default(),
            side: Side::Left,
            collapsed: false,
            width: DEFAULT_WIDTH,
            min_width: DEFAULT_MIN_WIDTH,
            max_width: DEFAULT_MAX_WIDTH,
            resizer_width: DEFAULT_RESIZER_WIDTH,
            resizer_hover_bg: None,
            inset: Some(DEFAULT_INSET),
            top_inset: px(0.0),
            blur_enabled: None,
            elevation: ElevationToken::Lg,
            on_resize_end: None,
        }
    }

    /// Set the side of the floating sidebar.
    ///
    /// Default is `Side::Left`.
    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    /// Set the sidebar to be collapsible, default is true.
    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.sidebar = self.sidebar.collapsible(collapsible);
        self
    }

    /// Set the sidebar to be collapsed.
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Set the expanded width of the floating sidebar.
    ///
    /// The value is clamped to the current `[min_width, max_width]` constraints.
    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.width = width.into().max(self.min_width).min(self.max_width);
        self
    }

    /// Set the minimum width constraint for resizing.
    pub fn min_width(mut self, width: impl Into<Pixels>) -> Self {
        self.min_width = width.into();
        if self.min_width > self.max_width {
            self.max_width = self.min_width;
        }
        self.width = self.width.max(self.min_width).min(self.max_width);
        self
    }

    /// Set the maximum width constraint for resizing.
    pub fn max_width(mut self, width: impl Into<Pixels>) -> Self {
        self.max_width = width.into();
        if self.max_width < self.min_width {
            self.min_width = self.max_width;
        }
        self.width = self.width.max(self.min_width).min(self.max_width);
        self
    }

    /// Set the width of the resize handle.
    pub fn resizer_width(mut self, width: impl Into<Pixels>) -> Self {
        self.resizer_width = width.into();
        self
    }

    /// Set the hover background color for the resize handle.
    pub fn resizer_hover_bg(mut self, color: impl Into<Hsla>) -> Self {
        self.resizer_hover_bg = Some(color.into());
        self
    }

    /// Set the inset from window edges.
    ///
    /// Default is 8px.
    pub fn inset(mut self, inset: impl Into<Pixels>) -> Self {
        self.inset = Some(inset.into());
        self
    }

    /// Set additional top inset on top of the base inset.
    pub fn top_inset(mut self, inset: impl Into<Pixels>) -> Self {
        self.top_inset = inset.into();
        self
    }

    /// Explicitly set whether blur effects are enabled for the glass surface.
    pub fn blur_enabled(mut self, enabled: bool) -> Self {
        self.blur_enabled = Some(enabled);
        self
    }

    /// Set the shadow elevation level for the sidebar panel.
    pub fn elevation(mut self, elevation: ElevationToken) -> Self {
        self.elevation = elevation;
        self
    }

    /// Set a callback invoked when resizing ends with the final width.
    pub fn on_resize_end(
        mut self,
        callback: impl Fn(Pixels, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_resize_end = Some(Rc::new(callback));
        self
    }

    /// Set the header of the sidebar.
    pub fn header(mut self, header: impl IntoElement) -> Self {
        self.sidebar = self.sidebar.header(header);
        self
    }

    /// Set a dynamic header that receives the visual collapsed state.
    pub fn header_with<F, H>(mut self, builder: F) -> Self
    where
        F: Fn(bool, &mut Window, &mut App) -> H + 'static,
        H: IntoElement,
    {
        self.sidebar = self.sidebar.header_with(builder);
        self
    }

    /// Set the footer of the sidebar.
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.sidebar = self.sidebar.footer(footer);
        self
    }

    /// Set a dynamic footer that receives the visual collapsed state.
    pub fn footer_with<F, H>(mut self, builder: F) -> Self
    where
        F: Fn(bool, &mut Window, &mut App) -> H + 'static,
        H: IntoElement,
    {
        self.sidebar = self.sidebar.footer_with(builder);
        self
    }

    /// Add a child element to the sidebar, the child must implement `SidebarItem`.
    pub fn child(mut self, child: E) -> Self {
        self.sidebar = self.sidebar.child(child);
        self
    }

    /// Add multiple children to the sidebar, the children must implement `SidebarItem`.
    pub fn children(mut self, children: impl IntoIterator<Item = E>) -> Self {
        self.sidebar = self.sidebar.children(children);
        self
    }
}

impl<E: SidebarItem> Styled for FloatingSidebar<E> {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl<E: SidebarItem> RenderOnce for FloatingSidebar<E> {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let FloatingSidebar {
            id,
            sidebar,
            style,
            side,
            collapsed,
            width,
            min_width,
            max_width,
            resizer_width,
            resizer_hover_bg,
            inset,
            top_inset,
            blur_enabled,
            elevation,
            on_resize_end,
        } = self;

        let initial_width = width.max(min_width).min(max_width);
        let state_key = SharedString::from(format!("{}-floating-sidebar-state", id));
        let state = window.use_keyed_state(state_key, cx, |_, _| {
            FloatingSidebarState::new(initial_width)
        });

        let expanded_width = {
            let current_width = state.read(cx).expanded_width;
            let clamped_width = current_width.max(min_width).min(max_width);
            if clamped_width != current_width {
                state.update(cx, |state, cx| {
                    state.expanded_width = clamped_width;
                    cx.notify();
                });
            }
            clamped_width
        };

        let shell_width = if collapsed {
            COLLAPSED_WIDTH
        } else {
            expanded_width
        };
        let resizer_width = if collapsed { px(0.0) } else { resizer_width };

        let end_resize = {
            let state = state.clone();
            let on_resize_end = on_resize_end.clone();
            Rc::new(move |window: &mut Window, cx: &mut App| {
                let is_resizing = state.read(cx).resizing;
                if !is_resizing {
                    return;
                }
                let width = state.read(cx).expanded_width;
                state.update(cx, |state, cx| {
                    state.resizing = false;
                    cx.notify();
                });
                if let Some(callback) = on_resize_end.as_ref() {
                    callback(width, window, cx);
                }
            })
        };

        let on_resize_start = {
            let state = state.clone();
            move |width: Pixels, x: Pixels, _window: &mut Window, cx: &mut App| {
                if collapsed {
                    return;
                }
                let width = width.max(min_width).min(max_width);
                state.update(cx, |state, cx| {
                    state.expanded_width = width;
                    state.drag_origin_width = width;
                    state.drag_origin_x = x;
                    state.resizing = true;
                    cx.notify();
                });
            }
        };

        let resize_tracker = FloatingSidebarResizeTracker {
            state: state.clone(),
            side,
            min_width,
            max_width,
            collapsed,
            end_resize: end_resize.clone(),
        };

        let mut shell = if side.is_left() {
            SidebarShell::left(shell_width)
        } else {
            SidebarShell::right(shell_width)
        };

        shell = shell
            .min_width(min_width)
            .max_width(max_width)
            .resizer_width(resizer_width)
            .top_inset(top_inset)
            .elevation(elevation)
            .on_resize_start(on_resize_start)
            .on_resize_end(move |window, cx| {
                end_resize(window, cx);
            });

        if let Some(color) = resizer_hover_bg {
            shell = shell.resizer_hover_bg(color);
        }
        if let Some(inset) = inset {
            shell = shell.inset(inset);
        }
        if let Some(blur_enabled) = blur_enabled {
            shell = shell.blur_enabled(blur_enabled);
        }

        shell
            .child(
                sidebar
                    .side(side)
                    .collapsed(collapsed)
                    .width(expanded_width)
                    .animate_width(false)
                    .refine_style(&style),
            )
            .child(resize_tracker)
    }
}

struct FloatingSidebarResizeTracker {
    state: gpui::Entity<FloatingSidebarState>,
    side: Side,
    min_width: Pixels,
    max_width: Pixels,
    collapsed: bool,
    end_resize: Rc<dyn Fn(&mut Window, &mut App)>,
}

impl IntoElement for FloatingSidebarResizeTracker {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for FloatingSidebarResizeTracker {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        (window.request_layout(Style::default(), None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: gpui::Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        ()
    }

    fn paint(
        &mut self,
        _: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: gpui::Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        window.on_mouse_event({
            let state = self.state.clone();
            let side = self.side;
            let min_width = self.min_width;
            let max_width = self.max_width;
            let collapsed = self.collapsed;
            move |event: &MouseMoveEvent, phase, _window, cx| {
                if !phase.bubble() || collapsed {
                    return;
                }
                let (resizing, start_x, start_width, current_width) = {
                    let state = state.read(cx);
                    (
                        state.resizing,
                        state.drag_origin_x,
                        state.drag_origin_width,
                        state.expanded_width,
                    )
                };
                if !resizing {
                    return;
                }
                let delta = if side.is_left() {
                    event.position.x - start_x
                } else {
                    start_x - event.position.x
                };
                let next_width = (start_width + delta).max(min_width).min(max_width);
                if next_width == current_width {
                    return;
                }
                state.update(cx, |state, cx| {
                    state.expanded_width = next_width;
                    cx.notify();
                });
            }
        });

        window.on_mouse_event({
            let state = self.state.clone();
            let end_resize = self.end_resize.clone();
            move |_: &MouseUpEvent, phase, window, cx| {
                if !phase.bubble() {
                    return;
                }
                if state.read(cx).resizing {
                    end_resize(window, cx);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sidebar::SidebarMenu;
    use gpui::hsla;

    #[gpui::test]
    fn test_floating_sidebar_builder(_cx: &mut gpui::TestAppContext) {
        let sidebar = FloatingSidebar::<SidebarMenu>::new("floating-sidebar")
            .side(Side::Right)
            .collapsed(true)
            .width(px(280.0))
            .min_width(px(220.0))
            .max_width(px(420.0))
            .resizer_width(px(8.0))
            .resizer_hover_bg(hsla(0.0, 0.0, 0.0, 0.2))
            .inset(px(6.0))
            .top_inset(px(12.0))
            .blur_enabled(false)
            .elevation(ElevationToken::Md)
            .on_resize_end(|_, _, _| {});

        assert_eq!(sidebar.side, Side::Right);
        assert!(sidebar.collapsed);
        assert_eq!(sidebar.width, px(280.0));
        assert_eq!(sidebar.min_width, px(220.0));
        assert_eq!(sidebar.max_width, px(420.0));
        assert_eq!(sidebar.resizer_width, px(8.0));
        assert!(sidebar.resizer_hover_bg.is_some());
        assert_eq!(sidebar.inset, Some(px(6.0)));
        assert_eq!(sidebar.top_inset, px(12.0));
        assert_eq!(sidebar.blur_enabled, Some(false));
        assert!(matches!(sidebar.elevation, ElevationToken::Md));
        assert!(sidebar.on_resize_end.is_some());
    }

    #[gpui::test]
    fn test_floating_sidebar_width_constraints(_cx: &mut gpui::TestAppContext) {
        let raised_min = FloatingSidebar::<SidebarMenu>::new("raised-min").min_width(px(450.0));
        assert_eq!(raised_min.min_width, px(450.0));
        assert_eq!(raised_min.max_width, px(450.0));
        assert_eq!(raised_min.width, px(450.0));

        let lowered_max = FloatingSidebar::<SidebarMenu>::new("lowered-max").max_width(px(180.0));
        assert_eq!(lowered_max.min_width, px(180.0));
        assert_eq!(lowered_max.max_width, px(180.0));
        assert_eq!(lowered_max.width, px(180.0));

        let clamped_width = FloatingSidebar::<SidebarMenu>::new("clamped-width")
            .min_width(px(220.0))
            .max_width(px(420.0))
            .width(px(100.0));
        assert_eq!(clamped_width.width, px(220.0));
    }
}
