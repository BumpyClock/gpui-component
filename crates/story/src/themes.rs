use std::path::PathBuf;

use gpui::{Action, App, SharedString};
use gpui_component::{
    ActiveTheme, Theme, ThemeModePreference, ThemeRegistry, scroll::ScrollbarShow,
};
use serde::{Deserialize, Serialize};

const STATE_FILE: &str = "target/state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct State {
    theme_set: SharedString,
    mode_preference: ThemeModePreference,
    scrollbar_show: Option<ScrollbarShow>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            theme_set: "Default".into(),
            mode_preference: ThemeModePreference::System,
            scrollbar_show: None,
        }
    }
}

pub fn init(cx: &mut App) {
    // Load last theme state
    let json = std::fs::read_to_string(STATE_FILE).unwrap_or(String::default());
    tracing::info!("Load themes...");
    let state = serde_json::from_str::<State>(&json).unwrap_or_default();
    if let Err(err) = ThemeRegistry::watch_dir(PathBuf::from("./themes"), cx, move |cx| {
        if let Some(set) = ThemeRegistry::global(cx)
            .theme_sets()
            .get(&state.theme_set)
            .cloned()
        {
            Theme::apply_theme_set(&set, state.mode_preference, None, cx);
        }
    }) {
        tracing::error!("Failed to watch themes directory: {}", err);
    }

    if let Some(scrollbar_show) = state.scrollbar_show {
        Theme::global_mut(cx).scrollbar_show = scrollbar_show;
    }
    cx.refresh_windows();

    cx.observe_global::<Theme>(|cx| {
        let state = State {
            theme_set: cx.theme().theme_set_name.clone(),
            mode_preference: cx.theme().mode_preference,
            scrollbar_show: Some(cx.theme().scrollbar_show),
        };

        if let Ok(json) = serde_json::to_string_pretty(&state) {
            // Ignore write errors - if STATE_FILE doesn't exist or can't be written, do nothing
            let _ = std::fs::write(STATE_FILE, json);
        }
    })
    .detach();

    cx.on_action(|switch: &SwitchTheme, cx| {
        let set_name = switch.0.clone();
        if let Some(set) = ThemeRegistry::global(cx)
            .theme_sets()
            .get(&set_name)
            .cloned()
        {
            let preference = Theme::global(cx).mode_preference;
            Theme::apply_theme_set(&set, preference, None, cx);
        }
        cx.refresh_windows();
    });
    cx.on_action(|switch: &SwitchThemeMode, cx| {
        let preference = switch.0;
        let set_name = Theme::global(cx).theme_set_name.clone();
        if let Some(set) = ThemeRegistry::global(cx)
            .theme_sets()
            .get(&set_name)
            .cloned()
        {
            Theme::apply_theme_set(&set, preference, None, cx);
        }
        cx.refresh_windows();
    });
}

#[derive(Action, Clone, PartialEq)]
#[action(namespace = themes, no_json)]
pub(crate) struct SwitchTheme(pub(crate) SharedString);

#[derive(Action, Clone, PartialEq)]
#[action(namespace = themes, no_json)]
pub(crate) struct SwitchThemeMode(pub(crate) ThemeModePreference);
