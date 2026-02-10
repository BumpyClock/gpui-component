use gpui::{App, Entity, Global};

use crate::text::TextViewState;

pub(crate) fn init(cx: &mut App) {
    cx.set_global(GlobalState::new());
}

impl Global for GlobalState {}

pub struct GlobalState {
    pub(crate) text_view_state_stack: Vec<Entity<TextViewState>>,
    /// Stack for blur_enabled context values.
    blur_enabled_stack: Vec<bool>,
    /// Stack for reduced_motion context values.
    reduced_motion_stack: Vec<bool>,
}

impl GlobalState {
    pub(crate) fn new() -> Self {
        Self {
            text_view_state_stack: Vec::new(),
            blur_enabled_stack: vec![true],    // Default to enabled
            reduced_motion_stack: vec![false], // Default to not reduced
        }
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    pub(crate) fn text_view_state(&self) -> Option<&Entity<TextViewState>> {
        self.text_view_state_stack.last()
    }

    /// Returns whether blur effects are enabled (from the context stack).
    pub fn blur_enabled(&self) -> bool {
        self.blur_enabled_stack.last().copied().unwrap_or(true)
    }

    /// Push a blur_enabled value onto the context stack.
    pub fn push_blur_enabled(&mut self, enabled: bool) {
        self.blur_enabled_stack.push(enabled);
    }

    /// Pop a blur_enabled value from the context stack.
    pub fn pop_blur_enabled(&mut self) {
        if self.blur_enabled_stack.len() > 1 {
            self.blur_enabled_stack.pop();
        }
    }

    /// Sets the base blur_enabled value (replaces the bottom of the stack).
    #[allow(dead_code)]
    pub fn set_blur_enabled(&mut self, enabled: bool) {
        if let Some(first) = self.blur_enabled_stack.first_mut() {
            *first = enabled;
        }
    }

    /// Returns whether reduced motion is enabled (from the context stack).
    #[allow(dead_code)]
    pub fn reduced_motion(&self) -> bool {
        self.reduced_motion_stack.last().copied().unwrap_or(false)
    }

    /// Push a reduced_motion value onto the context stack.
    pub fn push_reduced_motion(&mut self, reduced: bool) {
        self.reduced_motion_stack.push(reduced);
    }

    /// Pop a reduced_motion value from the context stack.
    pub fn pop_reduced_motion(&mut self) {
        if self.reduced_motion_stack.len() > 1 {
            self.reduced_motion_stack.pop();
        }
    }

    /// Sets the base reduced_motion value (replaces the bottom of the stack).
    #[allow(dead_code)]
    pub fn set_reduced_motion(&mut self, reduced: bool) {
        if let Some(first) = self.reduced_motion_stack.first_mut() {
            *first = reduced;
        }
    }
}
