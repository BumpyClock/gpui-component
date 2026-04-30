---
title: "Theme"
summary: "How to use GPUI Component theme colors, theme registry, and runtime theme switching."
order: -4
---

# Theme

All components support theming through the built-in Theme system, the [ActiveTheme] trait provides access to the current theme colors:

```rs
use gpui_component::{ActiveTheme as _};

// Access theme colors in your components
cx.theme().primary
cx.theme().background
cx.theme().foreground
```

So if you want use the colors from the current theme, you should keep your component or view have [App] context.

## Theme Registry

There have more than 20 built-in themes available in [themes](https://github.com/BumpyClock/gpui-component/tree/main/themes) folder.

https://github.com/BumpyClock/gpui-component/tree/main/themes

And we have a [ThemeRegistry] to help us to load themes.

```rs
use std::path::PathBuf;
use gpui::{App, SharedString};
use gpui_component::{Theme, ThemeRegistry};

pub fn init(cx: &mut App) {
    let theme_name = SharedString::from("Ayu Light");
    // Load and watch themes from ./themes directory
    if let Err(err) = ThemeRegistry::watch_dir(PathBuf::from("./themes"), cx, move |cx| {
        if let Some(theme) = ThemeRegistry::global(cx)
            .themes()
            .get(&theme_name)
            .cloned()
        {
            Theme::global_mut(cx).apply_config(&theme);
        }
    }) {
        tracing::error!("Failed to watch themes directory: {}", err);
    }
}
```

## Theme Sets

Theme files contain a [ThemeSet] with one or more variants (light and/or dark). The [ThemeRegistry] groups these by theme name, allowing users to select a color theme (e.g., "Solarized") rather than individual variants like "Solarized Light" or "Solarized Dark".

This design allows themes to automatically adapt to the user's appearance preference without requiring separate theme selection for light and dark modes.

## Mode Preference

Users can control how theme variants are applied using [ThemeModePreference]:

- **`System`**: Automatically follows the OS appearance setting. When the system switches between light and dark mode, the theme updates accordingly.
- **`Light`**: Always uses the light variant of the selected theme set.
- **`Dark`**: Always uses the dark variant of the selected theme set.

## Fallback Behavior

If a theme set only provides one variant (e.g., only dark), that variant is used for both light and dark modes. This ensures themes always render correctly even if they don't provide both variants.

## Usage Example

```rs
use gpui_component::{Theme, ThemeModePreference, ThemeRegistry};

// Apply a theme set with System mode (auto light/dark switching)
if let Some(set) = ThemeRegistry::global(cx).theme_sets().get("Solarized") {
    Theme::apply_theme_set(set, ThemeModePreference::System, Some(window), cx);
}

// Or apply with a fixed mode
if let Some(set) = ThemeRegistry::global(cx).theme_sets().get("Ayu") {
    Theme::apply_theme_set(set, ThemeModePreference::Dark, Some(window), cx);
}
```

[ActiveTheme]: https://docs.rs/gpui-component/latest/gpui_component/theme/trait.ActiveTheme.html
[ThemeRegistry]: https://docs.rs/gpui-component/latest/gpui_component/theme/struct.ThemeRegistry.html
[ThemeSet]: https://docs.rs/gpui-component/latest/gpui_component/theme/struct.ThemeSet.html
[ThemeModePreference]: https://docs.rs/gpui-component/latest/gpui_component/theme/enum.ThemeModePreference.html
[App]: https://docs.rs/gpui/latest/gpui/struct.App.html
