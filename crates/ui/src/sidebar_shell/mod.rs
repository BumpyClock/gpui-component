//! SidebarShell component providing a resizable sidebar panel with glass effects.
//!
//! This module provides a reusable sidebar shell that handles:
//! - Resizable panel with configurable min/max width constraints
//! - Built-in shadow elevation effects
//! - Glass surface effects via SurfacePreset
//! - Draggable resize handle on the inner edge
//! - Support for left/right placement
//!
//! # Resize Model
//!
//! SidebarShell uses a consumer-managed resize model. The component provides:
//! - `on_resize_start`: Called when the user starts dragging the resizer (mouse down)
//! - `on_resize_end`: Called when the user stops dragging (mouse up)
//!
//! The consumer is responsible for:
//! - Tracking drag state (resizing: bool, start_x, start_width)
//! - Handling mouse move events at the window/root level
//! - Calculating and applying the new width
//!
//! This model is necessary because GPUI's `RenderOnce` components cannot use
//! `cx.listener()` or `window.listener_for()` patterns.
//!
//! # Example
//!
//! ```rust,ignore
//! use ui::sidebar_shell::SidebarShell;
//!
//! SidebarShell::left(px(self.sidebar_width))
//!     .min_width(px(200.0))
//!     .max_width(px(400.0))
//!     .on_resize_start(move |width, x, _window, cx| {
//!         // Store resize start state: width, x position
//!     })
//!     .on_resize_end(move |_window, cx| {
//!         // Clear resize state
//!     })
//!     .child(sidebar_content)
//! ```

use std::rc::Rc;

use gpui::{
    AnyElement, App, BoxShadow, Hsla, InteractiveElement, IntoElement, ParentElement, Pixels,
    RenderOnce, StyleRefinement, Styled, Window, div, hsla, point, prelude::FluentBuilder, px,
};
use smallvec::SmallVec;

use crate::{
    ActiveTheme, ElevationToken, Side, StyledExt, SurfaceContext, SurfacePreset,
    global_state::GlobalState,
};

/// Default values for sidebar shell configuration.
const DEFAULT_MIN_WIDTH: f32 = 200.0;
const DEFAULT_MAX_WIDTH: f32 = 400.0;
const DEFAULT_RESIZER_WIDTH: f32 = 6.0;
const DEFAULT_INSET: f32 = 4.0;

/// Creates a 3-layer shadow effect for elevated sidebar panels.
///
/// This shadow configuration provides a natural depth effect with:
/// - Subtle near-edge shadow (4% opacity)
/// - Medium distance shadow (8% opacity)
/// - Far distance shadow (12% opacity)
pub fn sidebar_shadow() -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: hsla(0., 0., 0., 0.04),
            offset: point(px(0.0), px(1.0)),
            blur_radius: px(6.0),
            spread_radius: px(0.0),
        },
        BoxShadow {
            color: hsla(0., 0., 0., 0.08),
            offset: point(px(0.0), px(8.0)),
            blur_radius: px(22.0),
            spread_radius: px(0.0),
        },
        BoxShadow {
            color: hsla(0., 0., 0., 0.12),
            offset: point(px(0.0), px(22.0)),
            blur_radius: px(54.0),
            spread_radius: px(0.0),
        },
    ]
}

/// A resizable sidebar panel with built-in shadow and glass surface effects.
///
/// SidebarShell provides a container for sidebar content that handles:
/// - Absolute positioning with configurable inset from window edges
/// - 3-layer shadow elevation for depth perception
/// - Glass blur and noise effects via SurfacePreset::panel()
/// - Draggable resize handle with hover feedback
///
/// The component uses a builder pattern for configuration and implements
/// `ParentElement` for adding child content, `Styled` for style refinement,
/// and `RenderOnce` for rendering.
///
/// # Resize Model
///
/// Uses consumer-managed resize. The component fires `on_resize_start` and
/// `on_resize_end` callbacks, but the consumer must handle mouse move events
/// at the window level to track the actual resize operation.
///
/// # Layout Structure
///
/// ```text
/// +-- Outer Container (absolute, inset from edges) --+
/// |  +-- Shadow Wrapper (3-layer shadow) ----------+ |
/// |  |  +-- Surface (glass effects) -------------+ | |
/// |  |  |                                        | | |
/// |  |  |  [Child Content]                       | | |
/// |  |  |                                        | | |
/// |  |  +----------------------------------------+ | |
/// |  +--------------------------------------------+ |
/// |  [Resizer Handle] (on inner edge)              |
/// +------------------------------------------------+
/// ```
#[derive(IntoElement)]
pub struct SidebarShell {
    /// Current width of the sidebar in pixels.
    width: Pixels,
    /// Minimum width constraint for resizing.
    min_width: Pixels,
    /// Maximum width constraint for resizing.
    max_width: Pixels,
    /// Width of the resize handle in pixels.
    resizer_width: Pixels,
    /// Optional override for resizer hover background color.
    resizer_hover_bg: Option<Hsla>,
    /// Callback invoked when resize starts (mouse down on resizer).
    /// Receives: (current_width, mouse_x, window, cx)
    on_resize_start: Option<Rc<dyn Fn(Pixels, Pixels, &mut Window, &mut App)>>,
    /// Callback invoked when resize ends (mouse up).
    on_resize_end: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    /// Shadow elevation level for the sidebar panel.
    /// Default: `ElevationToken::Lg` for a prominent floating appearance.
    elevation: ElevationToken,
    /// Placement side (left or right).
    side: Side,
    /// Inset from window edges in pixels.
    inset: Pixels,
    /// Whether blur effects are enabled for the glass surface.
    /// If `None`, the value is inherited from the parent context (e.g., WindowShell).
    /// If `Some(value)`, the explicit value is used.
    blur_enabled: Option<bool>,
    /// Child elements rendered inside the surface.
    children: SmallVec<[AnyElement; 1]>,
    /// Style refinement for the outer container.
    style: StyleRefinement,
}

impl SidebarShell {
    /// Creates a new left-aligned sidebar shell with the specified width.
    ///
    /// # Arguments
    ///
    /// * `width` - The initial width of the sidebar in pixels.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let sidebar = SidebarShell::left(px(260.0));
    /// ```
    pub fn left(width: impl Into<Pixels>) -> Self {
        Self::new(width, Side::Left)
    }

    /// Creates a new right-aligned sidebar shell with the specified width.
    ///
    /// # Arguments
    ///
    /// * `width` - The initial width of the sidebar in pixels.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let sidebar = SidebarShell::right(px(300.0));
    /// ```
    pub fn right(width: impl Into<Pixels>) -> Self {
        Self::new(width, Side::Right)
    }

    fn new(width: impl Into<Pixels>, side: Side) -> Self {
        Self {
            width: width.into(),
            min_width: px(DEFAULT_MIN_WIDTH),
            max_width: px(DEFAULT_MAX_WIDTH),
            resizer_width: px(DEFAULT_RESIZER_WIDTH),
            resizer_hover_bg: None,
            on_resize_start: None,
            on_resize_end: None,
            elevation: ElevationToken::Lg,
            side,
            inset: px(DEFAULT_INSET),
            blur_enabled: None, // Inherit from context by default
            children: SmallVec::new(),
            style: StyleRefinement::default(),
        }
    }

    /// Sets the minimum width constraint for resizing.
    ///
    /// The sidebar cannot be resized smaller than this width.
    /// Default: 200px.
    pub fn min_width(mut self, width: impl Into<Pixels>) -> Self {
        self.min_width = width.into();
        self
    }

    /// Sets the maximum width constraint for resizing.
    ///
    /// The sidebar cannot be resized larger than this width.
    /// Default: 400px.
    pub fn max_width(mut self, width: impl Into<Pixels>) -> Self {
        self.max_width = width.into();
        self
    }

    /// Sets the width of the resize handle.
    ///
    /// Default: 6px.
    pub fn resizer_width(mut self, width: impl Into<Pixels>) -> Self {
        self.resizer_width = width.into();
        self
    }

    /// Sets the hover background color for the resize handle.
    ///
    /// If not set, defaults to theme foreground at 20% opacity.
    pub fn resizer_hover_bg(mut self, color: impl Into<Hsla>) -> Self {
        self.resizer_hover_bg = Some(color.into());
        self
    }

    /// Sets the callback invoked when resize starts (mouse down on resizer).
    ///
    /// The callback receives the current width and mouse X position.
    /// The consumer should store these to calculate width delta during mouse move.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// SidebarShell::left(px(260.0))
    ///     .on_resize_start(|width, x, window, cx| {
    ///         // Store: resizing = true, start_width = width, start_x = x
    ///     })
    /// ```
    pub fn on_resize_start(
        mut self,
        callback: impl Fn(Pixels, Pixels, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_resize_start = Some(Rc::new(callback));
        self
    }

    /// Sets the callback invoked when resize ends (mouse up).
    ///
    /// The consumer should clear their resize state.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// SidebarShell::left(px(260.0))
    ///     .on_resize_end(|window, cx| {
    ///         // Store: resizing = false
    ///     })
    /// ```
    pub fn on_resize_end(mut self, callback: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_resize_end = Some(Rc::new(callback));
        self
    }

    /// Sets the inset from window edges.
    ///
    /// This creates space between the sidebar and the window bounds.
    /// Default: 4px.
    pub fn inset(mut self, inset: impl Into<Pixels>) -> Self {
        self.inset = inset.into();
        self
    }

    /// Explicitly sets whether blur effects are enabled for the glass surface.
    ///
    /// When set, this value overrides any inherited context from the parent
    /// `WindowShell`. When not called, the sidebar inherits `blur_enabled`
    /// from the parent context.
    ///
    /// When disabled, the surface will not render backdrop blur or noise
    /// overlays, which can improve performance on systems that don't
    /// support blur effects well.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Inherit from WindowShell context (default behavior)
    /// SidebarShell::left(px(260.0))
    ///     .child(content)
    ///
    /// // Explicitly override to disable blur
    /// SidebarShell::left(px(260.0))
    ///     .blur_enabled(false)
    ///     .child(content)
    /// ```
    pub fn blur_enabled(mut self, enabled: bool) -> Self {
        self.blur_enabled = Some(enabled);
        self
    }

    /// Sets the shadow elevation level for the sidebar panel.
    ///
    /// Controls the shadow depth and intensity using the theme's elevation
    /// system. Higher values create more pronounced floating appearance.
    /// Default: `ElevationToken::Lg`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// SidebarShell::left(px(260.0))
    ///     .elevation(ElevationToken::Md)  // Medium shadow
    ///     .child(content)
    /// ```
    pub fn elevation(mut self, elevation: ElevationToken) -> Self {
        self.elevation = elevation;
        self
    }
}

impl ParentElement for SidebarShell {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for SidebarShell {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for SidebarShell {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let resizer_hover_bg = self
            .resizer_hover_bg
            .unwrap_or_else(|| cx.theme().foreground.alpha(0.20));

        let window_bounds = window.window_bounds().get_bounds();
        let window_height = window_bounds.size.height;
        let sidebar_height = window_height - self.inset * 2.0;
        let sidebar_width = self.width;

        // Use explicit value if set, otherwise inherit from context
        let blur_enabled = self
            .blur_enabled
            .unwrap_or_else(|| GlobalState::global(cx).blur_enabled());

        let sidebar_surface = SurfacePreset::panel()
            .wrap_with_bounds(
                div(),
                sidebar_width,
                sidebar_height,
                window,
                cx,
                SurfaceContext { blur_enabled },
            )
            .children(self.children)
            .id("sidebar-shell-surface")
            .size_full();

        let resizer_half = self.resizer_width / 2.0;
        let resizer_left = if self.side.is_left() {
            self.width - resizer_half
        } else {
            -resizer_half
        };

        let is_left = self.side.is_left();
        let on_resize_start = self.on_resize_start.clone();
        let on_resize_end = self.on_resize_end.clone();

        let outer = div()
            .id("sidebar-shell")
            .absolute()
            .top(self.inset)
            .bottom(self.inset)
            .w(self.width)
            .map(|el| {
                if is_left {
                    el.left(self.inset)
                } else {
                    el.right(self.inset)
                }
            })
            .child(
                self.elevation
                    .apply(div().id("sidebar-shell-shadow-wrapper").size_full(), cx)
                    .child(sidebar_surface),
            )
            .child(
                div()
                    .id("sidebar-shell-resizer")
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(resizer_left)
                    .w(self.resizer_width)
                    .rounded(px(999.0))
                    .bg(gpui::transparent_black())
                    .cursor_col_resize()
                    .hover(move |s| s.bg(resizer_hover_bg))
                    .when_some(on_resize_start, move |el, callback| {
                        el.on_mouse_down(gpui::MouseButton::Left, move |event, window, cx| {
                            cx.stop_propagation();
                            callback(sidebar_width, event.position.x, window, cx);
                        })
                    })
                    .when_some(on_resize_end, move |el, callback| {
                        el.on_mouse_up(gpui::MouseButton::Left, move |_event, window, cx| {
                            callback(window, cx);
                        })
                    }),
            )
            .refine_style(&self.style);

        outer
    }
}
