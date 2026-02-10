# Token Integration: Shadow, Color, Typography

## Summary

Implemented three parts of the Fluent token integration into the gpui-component theme system:

### Part A: Computed Shadow Helper
- Created `/Users/adityasharma/Projects/gpui-component/crates/ui/src/theme/elevation.rs`
- Added `computed_shadow(level, is_dark)` method to `ThemeElevation`
- Returns `SmallVec<[BoxShadow; 2]>` with Fluent elevation math (directional + ambient shadows)
- Level ranges: 0-2 no shadow, 3-32 directional only, 33+ directional + ambient, 128 special active window

### Part B: New ThemeColor Fields
- Added 5 Fluent color tokens to `ThemeColor`: `disabled_foreground`, `control_stroke`, `card`, `card_foreground`, `solid_background`
- Added corresponding config fields to `ThemeConfigColors` in schema.rs with serde rename attrs
- Added `apply_color!` calls with appropriate fallbacks in `ThemeColor::apply_config`
- Added default values in `default-theme.json` for both light and dark themes

### Part C: Typography Ramp
- Created `/Users/adityasharma/Projects/gpui-component/crates/ui/src/theme/typography.rs`
- Defined `ThemeTypography` with 9-step Fluent type ramp (caption through display)
- `TypeRampToken` stores `size: Pixels`, `line_height: Pixels`, `weight: FontWeight`
- `ThemeTypographyConfig` / `TypeRampTokenConfig` for JSON config overrides
- `apply_config()` method with macro-based field application
- Added `typography` field to `Theme` struct
- Wired config into `ThemeConfig` and `Theme::apply_config`
- Added 8 `StyledExt` convenience methods: `fluent_caption`, `fluent_body`, `fluent_body_strong`, `fluent_body_large`, `fluent_subtitle`, `fluent_title`, `fluent_title_large`, `fluent_display`

## Files Modified
- `/Users/adityasharma/Projects/gpui-component/crates/ui/src/theme/mod.rs` - Added `mod elevation`, `mod typography`, `pub use typography::*`, `typography` field on `Theme`
- `/Users/adityasharma/Projects/gpui-component/crates/ui/src/theme/theme_color.rs` - Added 5 new `Hsla` fields
- `/Users/adityasharma/Projects/gpui-component/crates/ui/src/theme/schema.rs` - Added config fields, apply calls, typography integration
- `/Users/adityasharma/Projects/gpui-component/crates/ui/src/theme/default-theme.json` - Added light+dark color defaults
- `/Users/adityasharma/Projects/gpui-component/crates/ui/src/styled.rs` - Added 8 fluent typography `StyledExt` methods

## Files Created
- `/Users/adityasharma/Projects/gpui-component/crates/ui/src/theme/elevation.rs`
- `/Users/adityasharma/Projects/gpui-component/crates/ui/src/theme/typography.rs`

## Issues / Notes
- Build currently has unrelated errors from the animation-refactor teammate (`with_animation` method not found in badge, popover, tooltip, select, tab, accordion, date_picker). None of these are from the token integration changes.
- `FontWeight` in gpui is `FontWeight(pub f32)`, not `u16` as suggested in the task spec. Adjusted `TypeRampTokenConfig.weight` to `Option<f32>` accordingly.
- `Pixels` implements `Into<DefiniteLength>` directly, so `line_height(t.line_height)` works without conversion.
- Fixed a potential clippy `collapsible_else_if` warning in elevation.rs.
