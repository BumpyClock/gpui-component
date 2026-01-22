//! WindowShell - A complete window primitive with layout modes, title bar, and sidebar support.
//!
//! This component provides a reusable layout structure for window-based applications,
//! organizing content into distinct regions with multiple layout strategies.
//!
//! # Layout Modes
//!
//! - **Standard**: Docked sidebars in a flex row; main fills remaining space.
//! - **FloatingPanels**: Inset glass panels using `SidebarShell`; main spans full window.
//! - **Overlay**: Sidebars overlay main content as absolute panels.
//! - **Split**: Docked layout with WindowShell-owned splitter bar.
//!
//! # Example
//!
//! ```ignore
//! WindowShell::new()
//!     .layout_mode(WindowLayoutMode::FloatingPanels)
//!     .title_bar_left(logo)
//!     .title_bar_center(title)
//!     .title_bar_right(actions)
//!     .sidebar_left(sidebar)
//!     .main(content)
//!     .background(noise_overlay)
//!     .overlay_children(dialog_layer)
//! ```
//!
//! # Platform Considerations
//!
//! - **macOS**: Native title bar with traffic lights. Default safe_area_left is 80px.
//! - **Windows/Linux**: Custom title bar with window controls. Uses `.occlude()` to prevent
//!   underlying content from intercepting clicks in the title bar region.

mod blur_scope;
mod reduced_motion_scope;

pub use blur_scope::BlurEnabledScope;
pub use reduced_motion_scope::ReducedMotionScope;

use std::rc::Rc;

use gpui::{
    AnyElement, App, Hsla, InteractiveElement, IntoElement, MouseButton, MouseMoveEvent,
    MouseUpEvent, ParentElement, Pixels, RenderOnce, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _, px, transparent_black,
};

use crate::{ActiveTheme, StyledExt, TITLE_BAR_HEIGHT, TitleBar};

/// Default safe area left padding for macOS traffic lights.
#[cfg(target_os = "macos")]
const DEFAULT_SAFE_AREA_LEFT: Pixels = px(80.0);
#[cfg(not(target_os = "macos"))]
const DEFAULT_SAFE_AREA_LEFT: Pixels = px(0.0);

/// Default safe area right padding (for Windows caption buttons).
#[cfg(target_os = "windows")]
const DEFAULT_SAFE_AREA_RIGHT: Pixels = px(138.0); // 3 buttons × 46px
#[cfg(not(target_os = "windows"))]
const DEFAULT_SAFE_AREA_RIGHT: Pixels = px(0.0);

/// Default splitter width for Split layout mode.
const DEFAULT_SPLITTER_WIDTH: Pixels = px(4.0);

/// Layout modes for WindowShell.
///
/// Controls how sidebars and main content are arranged within the window.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WindowLayoutMode {
    /// Docked sidebars in a flex row; main fills remaining space.
    /// Common desktop app layout (e.g., VS Code, Finder).
    #[default]
    Standard,

    /// Sidebars are inset glass panels (uses SidebarShell).
    /// Main content spans full window behind them.
    /// Modern, glassy or layered UI (e.g., Agent Term).
    FloatingPanels,

    /// Sidebars overlay main content as absolute panels.
    /// Main content always full width; sidebars cover it temporarily.
    /// Useful for smaller screens or temporary panels.
    Overlay,

    /// Docked layout with a draggable splitter between sidebar and main.
    /// IDE-style panes with persistent resizable layout.
    Split,
}

/// A complete window primitive with layout modes, title bar, and sidebar support.
///
/// WindowShell owns a `TitleBar` internally and provides:
/// - Multiple layout strategies via `WindowLayoutMode`
/// - Title bar with left/center/right slots and full override capability
/// - Safe-area offsets for platform window controls
/// - Mouse event forwarding for resize operations
/// - Background and overlay slots for custom effects and dialogs
///
/// # Z-Order (bottom to top)
///
/// 1. Background slot (noise, gradients)
/// 2. Main content + sidebars (layout-mode dependent)
/// 3. Overlay children (dialogs, sheets)
/// 4. Title bar overlay
#[derive(IntoElement)]
pub struct WindowShell {
    // Layout configuration
    layout_mode: WindowLayoutMode,
    title_bar_height: Pixels,
    inset: Pixels,
    blur_enabled: bool,
    reduced_motion: bool,

    // Safe area offsets
    safe_area_left: Pixels,
    safe_area_right: Pixels,

    // Content slots
    sidebar_left: Option<AnyElement>,
    sidebar_right: Option<AnyElement>,
    main: Option<AnyElement>,

    // Title bar slots
    title_bar_left: Option<AnyElement>,
    title_bar_center: Option<AnyElement>,
    title_bar_right: Option<AnyElement>,

    // Title bar override
    title_bar_override: Option<Box<dyn FnOnce(TitleBar) -> TitleBar>>,

    // Additional slots
    background: Option<AnyElement>,
    overlay_children: Option<AnyElement>,

    // Mouse event forwarding
    on_mouse_move: Option<Rc<dyn Fn(&MouseMoveEvent, &mut Window, &mut App)>>,
    on_mouse_up: Option<Rc<dyn Fn(&MouseUpEvent, &mut Window, &mut App)>>,

    // Split mode configuration
    on_split_resize: Option<Rc<dyn Fn(Pixels, &mut Window, &mut App)>>,
    splitter_width: Pixels,
    splitter_style: StyleRefinement,

    // Root style
    style: StyleRefinement,
}

impl Default for WindowShell {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowShell {
    /// Create a new WindowShell with default configuration.
    ///
    /// Default values:
    /// - `layout_mode`: Standard
    /// - `title_bar_height`: TITLE_BAR_HEIGHT (34px)
    /// - `inset`: 4.0px
    /// - `blur_enabled`: true
    /// - `reduced_motion`: false
    /// - `safe_area_left`: 80px on macOS, 0 otherwise
    /// - `safe_area_right`: 138px on Windows, 0 otherwise
    pub fn new() -> Self {
        Self {
            layout_mode: WindowLayoutMode::default(),
            title_bar_height: TITLE_BAR_HEIGHT,
            inset: px(4.0),
            blur_enabled: true,
            reduced_motion: false,
            safe_area_left: DEFAULT_SAFE_AREA_LEFT,
            safe_area_right: DEFAULT_SAFE_AREA_RIGHT,
            sidebar_left: None,
            sidebar_right: None,
            main: None,
            title_bar_left: None,
            title_bar_center: None,
            title_bar_right: None,
            title_bar_override: None,
            background: None,
            overlay_children: None,
            on_mouse_move: None,
            on_mouse_up: None,
            on_split_resize: None,
            splitter_width: DEFAULT_SPLITTER_WIDTH,
            splitter_style: StyleRefinement::default(),
            style: StyleRefinement::default(),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Layout configuration
    // ─────────────────────────────────────────────────────────────────────────────

    /// Set the layout mode.
    pub fn layout_mode(mut self, mode: WindowLayoutMode) -> Self {
        self.layout_mode = mode;
        self
    }

    /// Set the height of the title bar region.
    pub fn title_bar_height(mut self, height: impl Into<Pixels>) -> Self {
        self.title_bar_height = height.into();
        self
    }

    /// Set the outer inset for floating panels (used in FloatingPanels mode).
    pub fn inset(mut self, inset: impl Into<Pixels>) -> Self {
        self.inset = inset.into();
        self
    }

    /// Set whether blur effects are enabled.
    pub fn blur_enabled(mut self, enabled: bool) -> Self {
        self.blur_enabled = enabled;
        self
    }

    /// Set whether reduced motion is enabled.
    pub fn reduced_motion(mut self, reduced_motion: bool) -> Self {
        self.reduced_motion = reduced_motion;
        self
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Safe area configuration
    // ─────────────────────────────────────────────────────────────────────────────

    /// Set the left safe area offset (e.g., for macOS traffic lights).
    pub fn safe_area_left(mut self, offset: impl Into<Pixels>) -> Self {
        self.safe_area_left = offset.into();
        self
    }

    /// Set the right safe area offset (e.g., for Windows caption buttons).
    pub fn safe_area_right(mut self, offset: impl Into<Pixels>) -> Self {
        self.safe_area_right = offset.into();
        self
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Content slots
    // ─────────────────────────────────────────────────────────────────────────────

    /// Set the left sidebar content.
    pub fn sidebar_left(mut self, element: impl IntoElement) -> Self {
        self.sidebar_left = Some(element.into_any_element());
        self
    }

    /// Set the right sidebar content.
    pub fn sidebar_right(mut self, element: impl IntoElement) -> Self {
        self.sidebar_right = Some(element.into_any_element());
        self
    }

    /// Set the main content area.
    pub fn main(mut self, element: impl IntoElement) -> Self {
        self.main = Some(element.into_any_element());
        self
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Title bar slots
    // ─────────────────────────────────────────────────────────────────────────────

    /// Set the left slot of the title bar.
    pub fn title_bar_left(mut self, element: impl IntoElement) -> Self {
        self.title_bar_left = Some(element.into_any_element());
        self
    }

    /// Set the center slot of the title bar.
    pub fn title_bar_center(mut self, element: impl IntoElement) -> Self {
        self.title_bar_center = Some(element.into_any_element());
        self
    }

    /// Set the right slot of the title bar.
    pub fn title_bar_right(mut self, element: impl IntoElement) -> Self {
        self.title_bar_right = Some(element.into_any_element());
        self
    }

    /// Provide a full override for the TitleBar.
    ///
    /// The closure receives a default TitleBar and can modify or replace it entirely.
    /// This allows complete customization of drag regions, styling, and behavior.
    ///
    /// # Example
    ///
    /// ```ignore
    /// WindowShell::new()
    ///     .title_bar_override(|tb| {
    ///         tb.bg(custom_color)
    ///             .on_close_window(|_, window, _| { /* custom close */ })
    ///     })
    /// ```
    pub fn title_bar_override(mut self, f: impl FnOnce(TitleBar) -> TitleBar + 'static) -> Self {
        self.title_bar_override = Some(Box::new(f));
        self
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Additional slots
    // ─────────────────────────────────────────────────────────────────────────────

    /// Set the background content (rendered behind everything else).
    ///
    /// Use for noise overlays, gradients, or other visual effects.
    pub fn background(mut self, element: impl IntoElement) -> Self {
        self.background = Some(element.into_any_element());
        self
    }

    /// Set overlay children (rendered above content but below title bar).
    ///
    /// Use for dialogs, sheets, or other modal content.
    pub fn overlay_children(mut self, element: impl IntoElement) -> Self {
        self.overlay_children = Some(element.into_any_element());
        self
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Mouse event forwarding
    // ─────────────────────────────────────────────────────────────────────────────

    /// Set the mouse move handler for the root container.
    ///
    /// Use for tracking resize operations at the window level.
    pub fn on_mouse_move(
        mut self,
        handler: impl Fn(&MouseMoveEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_mouse_move = Some(Rc::new(handler));
        self
    }

    /// Set the mouse up handler for the root container.
    ///
    /// Use for ending resize operations at the window level.
    pub fn on_mouse_up(
        mut self,
        handler: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_mouse_up = Some(Rc::new(handler));
        self
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Split mode configuration
    // ─────────────────────────────────────────────────────────────────────────────

    /// Set the callback for split resize operations (Split mode only).
    ///
    /// The callback receives the new sidebar width.
    pub fn on_split_resize(
        mut self,
        handler: impl Fn(Pixels, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_split_resize = Some(Rc::new(handler));
        self
    }

    /// Set the width of the splitter bar (Split mode only).
    pub fn splitter_width(mut self, width: impl Into<Pixels>) -> Self {
        self.splitter_width = width.into();
        self
    }

    /// Set custom styles for the splitter bar (Split mode only).
    pub fn splitter_style(mut self, style: StyleRefinement) -> Self {
        self.splitter_style = style;
        self
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Getters
    // ─────────────────────────────────────────────────────────────────────────────

    /// Returns the configured layout mode.
    pub fn get_layout_mode(&self) -> WindowLayoutMode {
        self.layout_mode
    }

    /// Returns the configured title bar height.
    pub fn get_title_bar_height(&self) -> Pixels {
        self.title_bar_height
    }

    /// Returns the configured inset.
    pub fn get_inset(&self) -> Pixels {
        self.inset
    }

    /// Returns whether blur effects are enabled.
    pub fn is_blur_enabled(&self) -> bool {
        self.blur_enabled
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Layout rendering
    // ─────────────────────────────────────────────────────────────────────────────

    fn render_standard_layout(
        sidebar_left: Option<AnyElement>,
        sidebar_right: Option<AnyElement>,
        main: Option<AnyElement>,
        title_bar_height: Pixels,
    ) -> impl IntoElement {
        div()
            .id("window-shell-standard-layout")
            .absolute()
            .top(title_bar_height)
            .left_0()
            .right_0()
            .bottom_0()
            .flex()
            .flex_row()
            .when_some(sidebar_left, |el, sidebar| el.child(sidebar))
            .when_some(main, |el, main| {
                el.child(div().flex_1().size_full().child(main))
            })
            .when_some(sidebar_right, |el, sidebar| el.child(sidebar))
    }

    fn render_floating_panels_layout(
        sidebar_left: Option<AnyElement>,
        sidebar_right: Option<AnyElement>,
        main: Option<AnyElement>,
    ) -> impl IntoElement {
        // In FloatingPanels mode, sidebars are expected to be SidebarShell instances
        // which handle their own absolute positioning and insets.
        div()
            .id("window-shell-floating-layout")
            .absolute()
            .inset_0()
            .when_some(main, |el, main| el.child(main))
            .when_some(sidebar_left, |el, sidebar| el.child(sidebar))
            .when_some(sidebar_right, |el, sidebar| el.child(sidebar))
    }

    fn render_overlay_layout(
        sidebar_left: Option<AnyElement>,
        sidebar_right: Option<AnyElement>,
        main: Option<AnyElement>,
        title_bar_height: Pixels,
    ) -> impl IntoElement {
        // Main content fills full area; sidebars overlay as absolute panels
        div()
            .id("window-shell-overlay-layout")
            .absolute()
            .top(title_bar_height)
            .left_0()
            .right_0()
            .bottom_0()
            .relative()
            .when_some(main, |el, main| {
                el.child(div().id("overlay-main").size_full().child(main))
            })
            .when_some(sidebar_left, |el, sidebar| {
                el.child(
                    div()
                        .id("overlay-sidebar-left")
                        .absolute()
                        .top_0()
                        .left_0()
                        .bottom_0()
                        .child(sidebar),
                )
            })
            .when_some(sidebar_right, |el, sidebar| {
                el.child(
                    div()
                        .id("overlay-sidebar-right")
                        .absolute()
                        .top_0()
                        .right_0()
                        .bottom_0()
                        .child(sidebar),
                )
            })
    }

    fn render_split_layout(
        sidebar_left: Option<AnyElement>,
        sidebar_right: Option<AnyElement>,
        main: Option<AnyElement>,
        title_bar_height: Pixels,
        splitter_width: Pixels,
        splitter_style: StyleRefinement,
        on_split_resize: Option<Rc<dyn Fn(Pixels, &mut Window, &mut App)>>,
        cx: &App,
    ) -> impl IntoElement {
        let splitter_hover_bg = cx.theme().border_default;

        div()
            .id("window-shell-split-layout")
            .absolute()
            .top(title_bar_height)
            .left_0()
            .right_0()
            .bottom_0()
            .flex()
            .flex_row()
            .when_some(sidebar_left, |el, sidebar| {
                el.child(sidebar).child(Self::render_splitter(
                    "left",
                    splitter_width,
                    splitter_style.clone(),
                    splitter_hover_bg,
                    on_split_resize.clone(),
                ))
            })
            .when_some(main, |el, main| {
                el.child(div().flex_1().size_full().child(main))
            })
            .when_some(sidebar_right, |el, sidebar| {
                el.child(Self::render_splitter(
                    "right",
                    splitter_width,
                    splitter_style,
                    splitter_hover_bg,
                    on_split_resize,
                ))
                .child(sidebar)
            })
    }

    fn render_splitter(
        id: &'static str,
        width: Pixels,
        style: StyleRefinement,
        hover_bg: Hsla,
        _on_resize: Option<Rc<dyn Fn(Pixels, &mut Window, &mut App)>>,
    ) -> impl IntoElement {
        div()
            .id(format!("window-shell-splitter-{}", id))
            .w(width)
            .h_full()
            .flex_shrink_0()
            .cursor_col_resize()
            .bg(transparent_black())
            .hover(move |s| s.bg(hover_bg))
            .refine_style(&style)
        // Note: Actual drag handling requires consumer to use on_mouse_move at root level
        // or implement via on_drag. The splitter is styled and ready for interaction.
    }
}

impl Styled for WindowShell {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for WindowShell {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let title_bar_height = self.title_bar_height;
        let titlebar_bg = cx.theme().transparent;

        // Build the title bar
        let mut title_bar = TitleBar::new().bg(transparent_black()).border_b_0();

        if let Some(left) = self.title_bar_left {
            title_bar = title_bar.child(left);
        }
        if let Some(center) = self.title_bar_center {
            title_bar = title_bar.child(center);
        }
        if let Some(right) = self.title_bar_right {
            title_bar = title_bar.child(right);
        }

        // Apply override if provided
        if let Some(override_fn) = self.title_bar_override {
            title_bar = override_fn(title_bar);
        }

        // Build layout based on mode
        let content_layer = match self.layout_mode {
            WindowLayoutMode::Standard => Self::render_standard_layout(
                self.sidebar_left,
                self.sidebar_right,
                self.main,
                title_bar_height,
            )
            .into_any_element(),

            WindowLayoutMode::FloatingPanels => Self::render_floating_panels_layout(
                self.sidebar_left,
                self.sidebar_right,
                self.main,
            )
            .into_any_element(),

            WindowLayoutMode::Overlay => Self::render_overlay_layout(
                self.sidebar_left,
                self.sidebar_right,
                self.main,
                title_bar_height,
            )
            .into_any_element(),

            WindowLayoutMode::Split => Self::render_split_layout(
                self.sidebar_left,
                self.sidebar_right,
                self.main,
                title_bar_height,
                self.splitter_width,
                self.splitter_style,
                self.on_split_resize,
                cx,
            )
            .into_any_element(),
        };

        // Wrap content layer with blur and reduced motion context so child components can inherit
        let content_layer = BlurEnabledScope::new(self.blur_enabled, content_layer);
        let content_layer = ReducedMotionScope::new(self.reduced_motion, content_layer);

        // Clone handlers for use in closures
        let on_mouse_move = self.on_mouse_move.clone();
        let on_mouse_up = self.on_mouse_up.clone();

        div()
            .id("window-shell")
            .size_full()
            .relative()
            .refine_style(&self.style)
            // Mouse event forwarding
            .when_some(on_mouse_move, |el, handler| {
                el.on_mouse_move(move |event, window, cx| {
                    handler(event, window, cx);
                })
            })
            .when_some(on_mouse_up, |el, handler| {
                el.on_mouse_up(MouseButton::Left, move |event, window, cx| {
                    handler(event, window, cx);
                })
            })
            // Background layer
            .when_some(self.background, |el, bg| {
                el.child(
                    div()
                        .id("window-shell-background")
                        .absolute()
                        .inset_0()
                        .child(bg),
                )
            })
            // Title bar background strip
            .child(
                div()
                    .id("window-shell-titlebar-bg")
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(title_bar_height)
                    .bg(titlebar_bg),
            )
            // Content layer
            .child(content_layer)
            // Overlay children
            .when_some(self.overlay_children, |el, overlay| {
                el.child(
                    div()
                        .id("window-shell-overlay")
                        .absolute()
                        .inset_0()
                        .child(overlay),
                )
            })
            // Title bar overlay
            .child(
                div()
                    .id("window-shell-titlebar")
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(title_bar_height)
                    .when(cfg!(not(target_os = "macos")), |el| el.occlude())
                    .child(title_bar),
            )
    }
}
