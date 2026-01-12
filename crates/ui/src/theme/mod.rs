use crate::{
    highlighter::HighlightTheme, list::ListSettings, notification::NotificationSettings,
    scroll::ScrollbarShow, sheet::SheetSettings,
};
use gpui::{App, BoxShadow, Global, Hsla, Pixels, SharedString, Window, WindowAppearance, px};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::Arc,
};

mod color;
mod registry;
mod schema;
mod theme_color;

pub use color::*;
pub use registry::*;
pub use schema::*;
pub use theme_color::*;

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

#[derive(Debug, Clone, Default)]
pub struct ThemeElevation {
    pub xs: Vec<BoxShadow>,
    pub sm: Vec<BoxShadow>,
    pub md: Vec<BoxShadow>,
    pub lg: Vec<BoxShadow>,
    pub xl: Vec<BoxShadow>,
}

impl ThemeElevation {
    fn default_for_mode(mode: ThemeMode) -> Self {
        let (ambient, key) = if mode.is_dark() {
            ("rgba(0,0,0,0.24)", "rgba(0,0,0,0.28)")
        } else {
            ("rgba(0,0,0,0.12)", "rgba(0,0,0,0.14)")
        };
        let xs = format!("0 0 2px {ambient}, 0 1px 2px {key}");
        let sm = format!("0 0 2px {ambient}, 0 2px 4px {key}");
        let md = format!("0 0 2px {ambient}, 0 4px 8px {key}");
        let lg = format!("0 0 2px {ambient}, 0 8px 16px {key}");
        let xl = format!("0 0 8px {ambient}, 0 14px 28px {key}");
        Self {
            xs: try_parse_box_shadows(&xs).unwrap_or_default(),
            sm: try_parse_box_shadows(&sm).unwrap_or_default(),
            md: try_parse_box_shadows(&md).unwrap_or_default(),
            lg: try_parse_box_shadows(&lg).unwrap_or_default(),
            xl: try_parse_box_shadows(&xl).unwrap_or_default(),
        }
    }

    fn from_config(config: &ThemeConfig, default: &ThemeElevation) -> Self {
        let fallback = |value: &Option<SharedString>, base: &Vec<BoxShadow>| {
            value
                .as_ref()
                .and_then(|shadow| try_parse_box_shadows(shadow).ok())
                .unwrap_or_else(|| base.clone())
        };
        Self {
            xs: fallback(&config.elevation_xs, &default.xs),
            sm: fallback(&config.elevation_sm, &default.sm),
            md: fallback(&config.elevation_md, &default.md),
            lg: fallback(&config.elevation_lg, &default.lg),
            xl: fallback(&config.elevation_xl, &default.xl),
        }
    }
}

/// The global theme configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Theme {
    pub colors: ThemeColor,
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
    #[serde(skip)]
    #[schemars(skip)]
    pub elevation: ThemeElevation,
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

    pub fn elevation_xs(&self) -> &[BoxShadow] {
        &self.elevation.xs
    }

    pub fn elevation_sm(&self) -> &[BoxShadow] {
        &self.elevation.sm
    }

    pub fn elevation_md(&self) -> &[BoxShadow] {
        &self.elevation.md
    }

    pub fn elevation_lg(&self) -> &[BoxShadow] {
        &self.elevation.lg
    }

    pub fn elevation_xl(&self) -> &[BoxShadow] {
        &self.elevation.xl
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
            radius: px(12.),
            radius_lg: px(16.),
            shadow: true,
            elevation: ThemeElevation::default_for_mode(ThemeMode::default()),
            scrollbar_show: ScrollbarShow::default(),
            notification: NotificationSettings::default(),
            tile_grid_size: px(8.),
            tile_shadow: true,
            tile_radius: px(0.),
            list: ListSettings::default(),
            colors: *colors,
            light_theme: Rc::new(ThemeConfig::default()),
            dark_theme: Rc::new(ThemeConfig::default()),
            highlight_theme: HighlightTheme::default_light(),
            sheet: SheetSettings::default(),
        }
    }
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, PartialOrd, Eq, Hash, Serialize, Deserialize, JsonSchema,
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
