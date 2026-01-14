use gpui::{App, Entity, Global};

use crate::text::TextViewState;

pub(crate) fn init(cx: &mut App) {
    cx.set_global(GlobalState::new());
}

impl Global for GlobalState {}

/// Global state for UI components that need to share context across the element tree.
///
/// This struct provides stack-based context passing for values that need to be
/// inherited by child elements, following GPUI's push/pop pattern during rendering.
pub struct GlobalState {
    pub(crate) text_view_state_stack: Vec<Entity<TextViewState>>,
    /// Stack of blur_enabled values, allowing parent components to provide
    /// blur context that children can inherit.
    blur_enabled_stack: Vec<bool>,
    /// Stack of reduced_motion values, allowing parent components to provide
    /// motion preference context that children can inherit.
    reduced_motion_stack: Vec<bool>,
}

impl GlobalState {
    pub(crate) fn new() -> Self {
        Self {
            text_view_state_stack: Vec::new(),
            blur_enabled_stack: Vec::new(),
            reduced_motion_stack: Vec::new(),
        }
    }

    /// Access the global state immutably.
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    /// Access the global state mutably.
    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    pub(crate) fn text_view_state(&self) -> Option<&Entity<TextViewState>> {
        self.text_view_state_stack.last()
    }

    /// Returns the current blur_enabled value from context.
    ///
    /// If no value has been pushed onto the stack, defaults to `true`.
    /// Child components should use this to inherit blur settings from
    /// their parent `WindowShell` or other context providers.
    pub fn blur_enabled(&self) -> bool {
        self.blur_enabled_stack.last().copied().unwrap_or(true)
    }

    /// Returns the current reduced_motion value from context.
    ///
    /// If no value has been pushed onto the stack, defaults to `false`.
    /// Child components should use this to inherit motion preferences from
    /// their parent `WindowShell` or other context providers.
    pub fn reduced_motion(&self) -> bool {
        self.reduced_motion_stack.last().copied().unwrap_or(false)
    }

    /// Push a blur_enabled value onto the context stack.
    ///
    /// Called by `BlurEnabledScope` before rendering children.
    pub(crate) fn push_blur_enabled(&mut self, enabled: bool) {
        self.blur_enabled_stack.push(enabled);
    }

    /// Pop a blur_enabled value from the context stack.
    ///
    /// Called by `BlurEnabledScope` after rendering children.
    pub(crate) fn pop_blur_enabled(&mut self) {
        self.blur_enabled_stack.pop();
    }

    /// Push a reduced_motion value onto the context stack.
    ///
    /// Called by `ReducedMotionScope` before rendering children.
    pub(crate) fn push_reduced_motion(&mut self, reduced_motion: bool) {
        self.reduced_motion_stack.push(reduced_motion);
    }

    /// Pop a reduced_motion value from the context stack.
    ///
    /// Called by `ReducedMotionScope` after rendering children.
    pub(crate) fn pop_reduced_motion(&mut self) {
        self.reduced_motion_stack.pop();
    }
}

/// Extension trait for easy access to blur context from `App`.
///
/// # Example
///
/// ```ignore
/// use gpui_component::BlurContext;
///
/// fn render(&self, window: &mut Window, cx: &mut App) -> impl IntoElement {
///     let blur = cx.blur_enabled();
///     // ...
/// }
/// ```
pub trait BlurContext {
    /// Returns the current blur_enabled value from context, defaulting to `true`.
    fn blur_enabled(&self) -> bool;
}

impl BlurContext for App {
    fn blur_enabled(&self) -> bool {
        GlobalState::global(self).blur_enabled()
    }
}

/// Extension trait for easy access to reduced motion context from `App`.
pub trait ReducedMotionContext {
    /// Returns the current reduced_motion value from context, defaulting to `false`.
    fn reduced_motion(&self) -> bool;
}

impl ReducedMotionContext for App {
    fn reduced_motion(&self) -> bool {
        GlobalState::global(self).reduced_motion()
    }
}
