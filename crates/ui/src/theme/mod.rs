use crate::{
    highlighter::HighlightTheme, list::ListSettings, notification::NotificationSettings,
    scroll::ScrollbarShow, sheet::SheetSettings,
};
use gpui::{App, Global, Hsla, Pixels, SharedString, Window, WindowAppearance, px};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::Arc,
};

mod color;
mod elevation;
mod fluent_tokens;
mod registry;
mod schema;
mod theme_color;
mod typography;

pub use color::*;
pub use registry::*;
pub use schema::*;
pub use theme_color::*;
pub use typography::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ThemeShadowToken {
    #[default]
    None,
    Xs,
    Sm,
    Md,
    Lg,
    Xl,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ThemeMotion {
    pub fast_duration_ms: u16,
    pub normal_duration_ms: u16,
    pub slow_duration_ms: u16,
    pub strong_invoke_duration_ms: u16,
    pub soft_dismiss_duration_ms: u16,
    pub fade_duration_ms: u16,
    pub spring_mild_duration_ms: u16,
    pub spring_medium_duration_ms: u16,
    pub spring_mild_damping_ratio: f32,
    pub spring_medium_damping_ratio: f32,
    pub spring_mild_frequency: f32,
    pub spring_medium_frequency: f32,
    pub fast_invoke_easing: SharedString,
    pub strong_invoke_easing: SharedString,
    pub fast_dismiss_easing: SharedString,
    pub soft_dismiss_easing: SharedString,
    pub point_to_point_easing: SharedString,
    pub fade_easing: SharedString,
}

impl Default for ThemeMotion {
    fn default() -> Self {
        fluent_tokens::theme_motion_defaults()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ThemeElevation {
    pub control_level: usize,
    pub card_rest_level: usize,
    pub tooltip_level: usize,
    pub flyout_level: usize,
    pub dialog_level: usize,
    pub shell_level: usize,
    pub inactive_window_level: usize,
    pub active_window_level: usize,
    pub surface_flyout_shadow: ThemeShadowToken,
    pub surface_panel_shadow: ThemeShadowToken,
    pub surface_card_shadow: ThemeShadowToken,
}

impl Default for ThemeElevation {
    fn default() -> Self {
        fluent_tokens::theme_elevation_defaults()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ThemeMaterial {
    pub flyout_blur_radius: Pixels,
    pub panel_blur_radius: Pixels,
    pub flyout_light_opacity: f32,
    pub flyout_dark_opacity: f32,
    pub panel_light_opacity: f32,
    pub panel_dark_opacity: f32,
    pub card_light_opacity: f32,
    pub card_dark_opacity: f32,
    pub subtle_stroke_light_opacity: f32,
    pub subtle_stroke_dark_opacity: f32,
    pub smoke_light: Hsla,
    pub smoke_dark: Hsla,
    pub layer_light: Hsla,
    pub layer_dark: Hsla,
    pub layer_alt_light: Hsla,
    pub layer_alt_dark: Hsla,
    pub mica_base_light: Hsla,
    pub mica_base_dark: Hsla,
    pub mica_base_alt_light: Hsla,
    pub mica_base_alt_dark: Hsla,
    pub acrylic_base_light: Hsla,
    pub acrylic_base_dark: Hsla,
    pub acrylic_default_light: Hsla,
    pub acrylic_default_dark: Hsla,
}

impl Default for ThemeMaterial {
    fn default() -> Self {
        fluent_tokens::theme_material_defaults()
    }
}

pub fn init(cx: &mut App) {
    registry::init(cx);

    Theme::sync_system_appearance(None, cx);
    Theme::sync_scrollbar_appearance(cx);
}

pub trait ActiveTheme {
    fn theme(&self) -> &Theme;
}

impl ActiveTheme for App {
    #[inline(always)]
    fn theme(&self) -> &Theme {
        Theme::global(self)
    }
}

/// The global theme configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Theme {
    pub colors: ThemeColor,
    pub motion: ThemeMotion,
    pub elevation: ThemeElevation,
    pub material: ThemeMaterial,
    pub typography: ThemeTypography,
    pub highlight_theme: Arc<HighlightTheme>,
    pub light_theme: Rc<ThemeConfig>,
    pub dark_theme: Rc<ThemeConfig>,

    pub mode: ThemeMode,
    /// The font family for the application, default is `.SystemUIFont`.
    pub font_family: SharedString,
    /// The base font size for the application, default is 16px.
    pub font_size: Pixels,
    /// The monospace font family for the application.
    ///
    /// Defaults to:
    ///
    /// - macOS: `Menlo`
    /// - Windows: `Consolas`
    /// - Linux: `DejaVu Sans Mono`
    pub mono_font_family: SharedString,
    /// The monospace font size for the application, default is 13px.
    pub mono_font_size: Pixels,
    /// Radius for the general elements.
    pub radius: Pixels,
    /// Radius for the large elements, e.g.: Dialog, Notification border radius.
    pub radius_lg: Pixels,
    pub shadow: bool,
    pub transparent: Hsla,
    /// Show the scrollbar mode, default: Scrolling
    pub scrollbar_show: ScrollbarShow,
    /// The notification setting.
    pub notification: NotificationSettings,
    /// Tile grid size, default is 4px.
    pub tile_grid_size: Pixels,
    /// The shadow of the tile panel.
    pub tile_shadow: bool,
    /// The border radius of the tile panel, default is 0px.
    pub tile_radius: Pixels,
    /// The list settings.
    pub list: ListSettings,
    /// The sheet settings.
    pub sheet: SheetSettings,
}

impl Default for Theme {
    fn default() -> Self {
        Self::from(&ThemeColor::default())
    }
}

impl Deref for Theme {
    type Target = ThemeColor;

    fn deref(&self) -> &Self::Target {
        &self.colors
    }
}

impl DerefMut for Theme {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.colors
    }
}

impl Global for Theme {}

impl Theme {
    /// Returns the global theme reference
    #[inline(always)]
    pub fn global(cx: &App) -> &Theme {
        cx.global::<Theme>()
    }

    /// Returns the global theme mutable reference
    #[inline(always)]
    pub fn global_mut(cx: &mut App) -> &mut Theme {
        cx.global_mut::<Theme>()
    }

    /// Returns true if the theme is dark.
    #[inline(always)]
    pub fn is_dark(&self) -> bool {
        self.mode.is_dark()
    }

    /// Returns the current theme name.
    pub fn theme_name(&self) -> &SharedString {
        if self.is_dark() {
            &self.dark_theme.name
        } else {
            &self.light_theme.name
        }
    }

    /// Sync the theme with the system appearance
    pub fn sync_system_appearance(window: Option<&mut Window>, cx: &mut App) {
        // Better use window.appearance() for avoid error on Linux.
        // https://github.com/longbridge/gpui-component/issues/104
        let appearance = window
            .as_ref()
            .map(|window| window.appearance())
            .unwrap_or_else(|| cx.window_appearance());

        Self::change(appearance, window, cx);
    }

    /// Sync the Scrollbar showing behavior with the system
    pub fn sync_scrollbar_appearance(cx: &mut App) {
        Theme::global_mut(cx).scrollbar_show = if cx.should_auto_hide_scrollbars() {
            ScrollbarShow::Scrolling
        } else {
            ScrollbarShow::Hover
        };
    }

    /// Change the theme mode.
    pub fn change(mode: impl Into<ThemeMode>, window: Option<&mut Window>, cx: &mut App) {
        let mode = mode.into();
        if !cx.has_global::<Theme>() {
            let mut theme = Theme::default();
            theme.light_theme = ThemeRegistry::global(cx).default_light_theme().clone();
            theme.dark_theme = ThemeRegistry::global(cx).default_dark_theme().clone();
            cx.set_global(theme);
        }

        let theme = cx.global_mut::<Theme>();
        theme.mode = mode;
        if mode.is_dark() {
            theme.apply_config(&theme.dark_theme.clone());
        } else {
            theme.apply_config(&theme.light_theme.clone());
        }

        if let Some(window) = window {
            window.refresh();
        }
    }

    /// Get the editor background color, if not set, use the theme background color.
    #[inline]
    pub(crate) fn editor_background(&self) -> Hsla {
        self.highlight_theme
            .style
            .editor_background
            .unwrap_or(self.background)
    }
}

impl From<&ThemeColor> for Theme {
    fn from(colors: &ThemeColor) -> Self {
        Theme {
            mode: ThemeMode::default(),
            transparent: Hsla::transparent_black(),
            font_family: ".SystemUIFont".into(),
            font_size: px(16.),
            mono_font_family: if cfg!(target_os = "macos") {
                // https://en.wikipedia.org/wiki/Menlo_(typeface)
                "Menlo".into()
            } else if cfg!(target_os = "windows") {
                "Consolas".into()
            } else {
                "DejaVu Sans Mono".into()
            },
            mono_font_size: px(13.),
            radius: px(6.),
            radius_lg: px(8.),
            shadow: true,
            scrollbar_show: ScrollbarShow::default(),
            notification: NotificationSettings::default(),
            tile_grid_size: px(8.),
            tile_shadow: true,
            tile_radius: px(0.),
            list: ListSettings::default(),
            colors: *colors,
            motion: ThemeMotion::default(),
            elevation: ThemeElevation::default(),
            material: ThemeMaterial::default(),
            typography: ThemeTypography::default(),
            light_theme: Rc::new(ThemeConfig::default()),
            dark_theme: Rc::new(ThemeConfig::default()),
            highlight_theme: HighlightTheme::default_light(),
            sheet: SheetSettings::default(),
        }
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    PartialOrd,
    Eq,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

impl ThemeMode {
    #[inline(always)]
    pub fn is_dark(&self) -> bool {
        matches!(self, Self::Dark)
    }

    /// Return lower_case theme name: `light`, `dark`.
    pub fn name(&self) -> &'static str {
        match self {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        }
    }
}

impl From<WindowAppearance> for ThemeMode {
    fn from(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::Dark,
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::Light,
        }
    }
}
