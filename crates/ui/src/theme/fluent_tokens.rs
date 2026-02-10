use gpui::{Hsla, px};

use crate::{ThemeElevation, ThemeMaterial, ThemeMotion, ThemeShadowToken, try_parse_color};

pub(crate) fn theme_motion_defaults() -> ThemeMotion {
    ThemeMotion {
        // Fluent animation tokens: 187 / 333 / 500 ms cadence
        fast_duration_ms: 187,
        normal_duration_ms: 333,
        slow_duration_ms: 500,
        strong_invoke_duration_ms: 667,
        soft_dismiss_duration_ms: 167,
        fade_duration_ms: 83,
        fast_invoke_easing: "cubic-bezier(0, 0, 0, 1)".into(),
        strong_invoke_easing: "cubic-bezier(0.13, 1, 0, 0.92)".into(),
        fast_dismiss_easing: "cubic-bezier(0, 0, 0, 1)".into(),
        soft_dismiss_easing: "cubic-bezier(1, 0, 1, 1)".into(),
        point_to_point_easing: "cubic-bezier(0.55, 0.55, 0, 1)".into(),
        fade_easing: "linear".into(),
    }
}

pub(crate) fn theme_elevation_defaults() -> ThemeElevation {
    ThemeElevation {
        // Fluent elevation levels
        control_level: 2,
        card_rest_level: 8,
        tooltip_level: 16,
        flyout_level: 32,
        dialog_level: 128,
        shell_level: 36,
        inactive_window_level: 64,
        active_window_level: 128,
        // Preserve current gpui-component surface shadow behavior
        surface_flyout_shadow: ThemeShadowToken::Sm,
        surface_panel_shadow: ThemeShadowToken::Lg,
        surface_card_shadow: ThemeShadowToken::Sm,
    }
}

pub(crate) fn theme_material_defaults() -> ThemeMaterial {
    ThemeMaterial {
        // Preserve existing surface behavior when no config is supplied
        flyout_blur_radius: px(60.0),
        panel_blur_radius: px(120.0),
        flyout_light_opacity: 0.75,
        flyout_dark_opacity: 0.85,
        panel_light_opacity: 0.85,
        panel_dark_opacity: 0.90,
        card_light_opacity: 0.70,
        card_dark_opacity: 0.05,
        subtle_stroke_light_opacity: 0.5,
        subtle_stroke_dark_opacity: 0.5,

        // Fluent layering + material palette tokens
        smoke_light: fluent_color("#0000004D"),
        smoke_dark: fluent_color("#0000004D"),
        layer_light: fluent_color("#FFFFFF80"),
        layer_dark: fluent_color("#3A3A3A4C"),
        layer_alt_light: fluent_color("#FFFFFFFF"),
        layer_alt_dark: fluent_color("#FFFFFF0D"),
        mica_base_light: fluent_color("#F3F3F3"),
        mica_base_dark: fluent_color("#202020"),
        mica_base_alt_light: fluent_color("#DADADA80"),
        mica_base_alt_dark: fluent_color("#0A0A0A00"),
        acrylic_base_light: fluent_color("#F3F3F3"),
        acrylic_base_dark: fluent_color("#202020"),
        acrylic_default_light: fluent_color("#FCFCFC"),
        acrylic_default_dark: fluent_color("#2C2C2C"),
    }
}

fn fluent_color(value: &str) -> Hsla {
    try_parse_color(value).unwrap_or_else(|_| gpui::transparent_black())
}
