# Theme Motion Search Notes

Date: 2026-02-10

## Tokens

- `ThemeMotion` tokens: durations + easing strings. `crates/ui/src/theme/mod.rs`
- Defaults in `crates/ui/src/theme/fluent_tokens.rs` and `crates/ui/src/theme/default-theme.json`.
- Theme overrides in `crates/ui/src/theme/schema.rs` (`ThemeMotionConfig`).

## Usage

- Helper fns: `crates/ui/src/animation.rs` (`fast_invoke_animation`, `soft_dismiss_animation`, `point_to_point_animation`, `fade_animation`, `strong_invoke_animation`).
- Components using theme motion: accordion, badge, checkbox, collapsible, dialog, menu/context_menu, notification, popover, progress, select, sheet, sidebar, switch, tab, time/date_picker, tooltip, command_palette (durations only). See `crates/ui/src/*`.

## Spring-like patterns

- `gentle_spring` easing in command palette. `crates/ui/src/command_palette/view.rs`
- `strong_invoke_easing` is overshoot (cubic-bezier y>1). `crates/ui/src/theme/fluent_tokens.rs`
- `bounce(ease_in_out)` used for skeleton shimmer. `crates/ui/src/skeleton.rs`

## Doc mismatch

- `docs/docs/components/sidebar.md` says submenu expand uses `bounce(ease_in_out)`, but code uses `fast_invoke_animation`.
