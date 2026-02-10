# Animation Refactor + Strong Invoke Easing Fix

## Summary

Centralized duplicated animation utility functions (`parse_cubic_bezier_easing` and `animation_with_theme_easing`) into the shared `crate::animation` module, and fixed the `strong_invoke_easing` cubic-bezier value from `(0.13, 1, 0, 0.92)` to `(0.13, 1.62, 0, 0.92)` (overshoot bounce).

## Changes

### Step 1: Expanded `crates/ui/src/animation.rs`
Added the following public functions:
- `parse_cubic_bezier_easing(value: &str) -> Option<(f32, f32, f32, f32)>`
- `animation_with_theme_easing(animation: Animation, easing: &str) -> Animation`
- `theme_animation(duration_ms: u16, easing: &str, reduced_motion: bool) -> Option<Animation>`
- `fast_invoke_animation(motion: &ThemeMotion, reduced_motion: bool) -> Option<Animation>`
- `soft_dismiss_animation(motion: &ThemeMotion, reduced_motion: bool) -> Option<Animation>`
- `point_to_point_animation(motion: &ThemeMotion, reduced_motion: bool) -> Option<Animation>`
- `fade_animation(motion: &ThemeMotion, reduced_motion: bool) -> Option<Animation>`
- `strong_invoke_animation(motion: &ThemeMotion, reduced_motion: bool) -> Option<Animation>`

### Step 2: Removed duplicate functions from 8 files
Each file had its own copy of `parse_cubic_bezier_easing` and `animation_with_theme_easing`. Replaced with import `use crate::animation::animation_with_theme_easing;`.

Files modified:
- `/Users/adityasharma/Projects/gpui-component/crates/ui/src/switch.rs`
- `/Users/adityasharma/Projects/gpui-component/crates/ui/src/notification.rs`
- `/Users/adityasharma/Projects/gpui-component/crates/ui/src/progress/progress_circle.rs`
- `/Users/adityasharma/Projects/gpui-component/crates/ui/src/progress/progress.rs`
- `/Users/adityasharma/Projects/gpui-component/crates/ui/src/checkbox.rs`
- `/Users/adityasharma/Projects/gpui-component/crates/ui/src/dialog.rs`
- `/Users/adityasharma/Projects/gpui-component/crates/ui/src/sheet.rs`
- `/Users/adityasharma/Projects/gpui-component/crates/ui/src/command_palette/view.rs`

### Step 3: Fixed strong_invoke easing
Changed `cubic-bezier(0.13, 1, 0, 0.92)` to `cubic-bezier(0.13, 1.62, 0, 0.92)` in:
- `/Users/adityasharma/Projects/gpui-component/crates/ui/src/theme/fluent_tokens.rs` (line 15)
- `/Users/adityasharma/Projects/gpui-component/crates/ui/src/theme/default-theme.json` (both light and dark theme sections)

### Step 4: Verification
- `cargo clippy -p gpui-component -- --deny warnings` passes clean
- `cargo fmt -p gpui-component --check` passes clean

## Notes
- The convenience constructors (`fast_invoke_animation`, `fade_animation`, etc.) are now available for downstream tasks (surface animations, state transitions) to use instead of manually constructing `Animation::new(Duration::from_millis(...))` + `animation_with_theme_easing(...)` each time.
- The existing callsites in the 8 files still use `animation_with_theme_easing` directly (not the convenience constructors), since the task spec said to migrate "where possible" and the existing call patterns use custom duration/easing combinations that don't always map 1:1 to a convenience constructor.
- `cargo fmt` also reformatted some pre-existing style issues in the touched files.
