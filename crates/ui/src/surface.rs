//! Surface module providing a unified API for glass/blur/noise surface effects.
//!
//! This module consolidates surface rendering logic into a preset-based system that
//! handles backdrop blur, noise overlays, elevation shadows, and border styling.
//!
//! # Example
//!
//! ```rust,ignore
//! use ui::surface::{SurfacePreset, SurfaceContext};
//!
//! let surface = SurfacePreset::flyout()
//!     .with_transparency_factor(0.9)
//!     .wrap_with_bounds(content, width, height, window, cx, ctx);
//! ```

use gpui::{
    App, Div, Hsla, IntoElement, ObjectFit, ParentElement, Pixels, Styled, StyledImage, Window,
    div, img, px,
};

use crate::{ActiveTheme, StyledExt};

const GLASS_NOISE_ASSET_PATH: &str = "NoiseAsset_256.png";
const GLASS_NOISE_TILE_SIZE_BASE: f32 = 128.0;

/// Runtime context for surface rendering decisions.
#[derive(Debug, Clone, Copy, Default)]
pub struct SurfaceContext {
    /// Whether blur effects are enabled (controlled by app settings).
    pub blur_enabled: bool,
}

/// Semantic categorization of surface types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SurfaceKind {
    #[default]
    Base,
    Flyout,
    Panel,
    Card,
}

/// Noise overlay intensity presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NoiseIntensity {
    None,
    #[default]
    Subtle,
    Heavy,
}

impl NoiseIntensity {
    /// Returns the opacity value for this noise intensity.
    pub fn opacity(&self) -> f32 {
        match self {
            NoiseIntensity::None => 0.0,
            NoiseIntensity::Subtle => 0.02,
            NoiseIntensity::Heavy => 0.04,
        }
    }
}

/// Maps to existing theme elevation levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ElevationToken {
    #[default]
    None,
    Xs,
    Sm,
    Md,
    Lg,
    Xl,
}

impl ElevationToken {
    /// Applies the elevation shadow to the given element.
    pub fn apply<E: Styled + StyledExt>(&self, element: E, _cx: &App) -> E {
        match self {
            ElevationToken::None => element,
            ElevationToken::Xs => element.shadow_sm(),
            ElevationToken::Sm => element.shadow_sm(),
            ElevationToken::Md => element.shadow_md(),
            ElevationToken::Lg => element.shadow_lg(),
            ElevationToken::Xl => element.shadow_xl(),
        }
    }
}

/// Source for surface background color from theme.
#[derive(Debug, Clone, Copy, Default)]
pub enum SurfaceColorSource {
    #[default]
    Popover,
    White,
    Sidebar,
    Background,
}

/// Background configuration with light/dark mode variants.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceBackground {
    pub color_source: SurfaceColorSource,
    pub light_opacity: f32,
    pub dark_opacity: f32,
}

impl Default for SurfaceBackground {
    fn default() -> Self {
        Self {
            color_source: SurfaceColorSource::Popover,
            light_opacity: 0.85,
            dark_opacity: 0.90,
        }
    }
}

impl SurfaceBackground {
    /// Resolves the background color based on theme mode and opacity settings.
    pub fn resolve(&self, cx: &App) -> Hsla {
        let base = match self.color_source {
            SurfaceColorSource::Popover => cx.theme().popover,
            SurfaceColorSource::White => gpui::white(),
            SurfaceColorSource::Sidebar => cx.theme().sidebar,
            SurfaceColorSource::Background => cx.theme().background,
        };
        let opacity = if cx.theme().mode.is_dark() {
            self.dark_opacity
        } else {
            self.light_opacity
        };
        base.opacity(opacity)
    }
}

/// Border/stroke color options.
#[derive(Debug, Clone, Copy, Default)]
pub enum StrokeColor {
    #[default]
    Subtle,
    Default,
    Strong,
    SubtleWithOpacity(f32),
}

/// Border/stroke specification.
#[derive(Debug, Clone, Copy)]
pub struct StrokeSpec {
    pub width: Pixels,
    pub color: StrokeColor,
}

impl StrokeSpec {
    /// Creates a subtle stroke specification with 1px width.
    pub fn subtle() -> Self {
        Self {
            width: px(1.0),
            color: StrokeColor::Subtle,
        }
    }

    /// Creates a default border specification with 1px width.
    pub fn default_border() -> Self {
        Self {
            width: px(1.0),
            color: StrokeColor::Default,
        }
    }

    /// Resolves the stroke color based on the current theme.
    pub fn resolve_color(&self, cx: &App) -> Hsla {
        match self.color {
            StrokeColor::Subtle => cx.theme().border.opacity(0.5),
            StrokeColor::Default => cx.theme().border,
            StrokeColor::Strong => cx.theme().border,
            StrokeColor::SubtleWithOpacity(opacity) => cx.theme().border.opacity(opacity),
        }
    }
}

/// A preset configuration for surface appearance.
///
/// Surfaces are the foundational visual containers in the UI. This struct provides
/// a declarative way to configure backdrop blur, noise overlays, elevation shadows,
/// borders, and background colors.
#[derive(Debug, Clone)]
pub struct SurfacePreset {
    pub kind: SurfaceKind,
    pub blur_radius: Option<Pixels>,
    pub noise_intensity: NoiseIntensity,
    pub background: SurfaceBackground,
    pub elevation: ElevationToken,
    pub stroke: Option<StrokeSpec>,
    pub transparency_factor: f32,
    pub radius: Option<Pixels>,
}

impl SurfacePreset {
    /// Creates a base surface preset for primary content areas.
    ///
    /// - No blur
    /// - Heavy noise (only rendered when blur_enabled)
    /// - No background color (transparent)
    /// - No elevation or stroke
    pub fn base() -> Self {
        Self {
            kind: SurfaceKind::Base,
            blur_radius: None,
            noise_intensity: NoiseIntensity::Heavy,
            background: SurfaceBackground {
                color_source: SurfaceColorSource::Background,
                light_opacity: 0.0,
                dark_opacity: 0.0,
            },
            elevation: ElevationToken::None,
            stroke: None,
            transparency_factor: 1.0,
            radius: None,
        }
    }

    /// Creates a flyout surface preset for menus and dropdowns.
    ///
    /// - 60px blur radius
    /// - Subtle noise
    /// - Popover background at 0.85/0.90 opacity
    /// - Small elevation with subtle stroke
    /// - 12px border radius
    pub fn flyout() -> Self {
        Self {
            kind: SurfaceKind::Flyout,
            blur_radius: Some(px(60.0)),
            noise_intensity: NoiseIntensity::Subtle,
            background: SurfaceBackground {
                color_source: SurfaceColorSource::Popover,
                light_opacity: 0.85,
                dark_opacity: 0.90,
            },
            elevation: ElevationToken::Sm,
            stroke: Some(StrokeSpec::subtle()),
            transparency_factor: 1.0,
            radius: Some(px(12.0)),
        }
    }

    /// Creates a panel surface preset for sidebars and navigation.
    ///
    /// - 120px blur radius
    /// - Heavy noise
    /// - Sidebar background at 0.85/0.90 opacity
    /// - Large elevation with subtle stroke
    /// - 16px border radius
    pub fn panel() -> Self {
        Self {
            kind: SurfaceKind::Panel,
            blur_radius: Some(px(120.0)),
            noise_intensity: NoiseIntensity::Heavy,
            background: SurfaceBackground {
                color_source: SurfaceColorSource::Sidebar,
                light_opacity: 0.85,
                dark_opacity: 0.90,
            },
            elevation: ElevationToken::Lg,
            stroke: Some(StrokeSpec::subtle()),
            transparency_factor: 1.0,
            radius: Some(px(16.0)),
        }
    }

    /// Creates a card surface preset for content cards.
    ///
    /// - No blur
    /// - No noise
    /// - White background at 0.70/0.05 opacity
    /// - Small elevation with default border
    /// - Uses theme radius
    pub fn card() -> Self {
        Self {
            kind: SurfaceKind::Card,
            blur_radius: None,
            noise_intensity: NoiseIntensity::None,
            background: SurfaceBackground {
                color_source: SurfaceColorSource::White,
                light_opacity: 0.70,
                dark_opacity: 0.05,
            },
            elevation: ElevationToken::Sm,
            stroke: Some(StrokeSpec::default_border()),
            transparency_factor: 1.0,
            radius: None,
        }
    }

    /// Sets the transparency factor for the background.
    pub fn with_transparency_factor(mut self, factor: f32) -> Self {
        self.transparency_factor = factor;
        self
    }

    /// Sets the blur radius for the backdrop blur effect.
    pub fn with_blur_radius(mut self, radius: Option<Pixels>) -> Self {
        self.blur_radius = radius;
        self
    }

    /// Sets the border radius for the surface.
    pub fn with_radius(mut self, radius: Pixels) -> Self {
        self.radius = Some(radius);
        self
    }

    /// Sets the noise overlay intensity.
    pub fn with_noise(mut self, intensity: NoiseIntensity) -> Self {
        self.noise_intensity = intensity;
        self
    }

    /// Sets the elevation level for shadow effects.
    pub fn with_elevation(mut self, elevation: ElevationToken) -> Self {
        self.elevation = elevation;
        self
    }

    /// Sets the stroke/border specification.
    pub fn with_stroke(mut self, stroke: Option<StrokeSpec>) -> Self {
        self.stroke = stroke;
        self
    }

    /// Wraps content in a surface container with all configured effects.
    ///
    /// This method creates a complete surface element with:
    /// - Background color with transparency
    /// - Backdrop blur (if enabled and configured)
    /// - Border/stroke styling
    /// - Elevation shadows
    /// - Noise overlay (if blur is enabled)
    pub fn wrap_with_bounds(
        &self,
        content: impl IntoElement,
        width: Pixels,
        height: Pixels,
        window: &Window,
        cx: &App,
        ctx: SurfaceContext,
    ) -> Div {
        let radius = self.radius.unwrap_or(cx.theme().radius);
        let scale_factor = window.scale_factor();

        let bg_color = self
            .background
            .resolve(cx)
            .opacity(self.transparency_factor);
        let noise_opacity = self.noise_intensity.opacity();
        let should_render_noise = ctx.blur_enabled && noise_opacity > 0.0;

        let mut surface = div().relative().rounded(radius).overflow_hidden();

        if bg_color.a > 0.0 {
            surface = surface.bg(bg_color);
        }

        // Note: backdrop_blur is not available in this GPUI version.
        // The blur_radius configuration is preserved for future compatibility.
        let _ = ctx.blur_enabled;
        let _ = self.blur_radius;

        if let Some(ref stroke) = self.stroke {
            surface = surface
                .border(stroke.width)
                .border_color(stroke.resolve_color(cx));
        }

        surface = self.elevation.apply(surface, cx);

        if should_render_noise {
            surface = surface.child(render_noise_overlay(
                width,
                height,
                radius,
                noise_opacity,
                scale_factor,
            ));
        }

        surface.child(content)
    }
}

/// Renders a tiled noise overlay for glass effects.
///
/// This is exposed publicly for cases where the full `wrap_with_bounds` API
/// is too restrictive due to borrow checker constraints.
pub fn render_noise_overlay(
    width: Pixels,
    height: Pixels,
    radius: Pixels,
    opacity: f32,
    scale_factor: f32,
) -> impl IntoElement {
    let tile_size_value = (GLASS_NOISE_TILE_SIZE_BASE / scale_factor.max(1.0)).round();
    let tile_size = px(tile_size_value);
    let cols = ((width / tile_size).max(0.0).ceil() as usize).max(1) + 12;
    let rows = ((height / tile_size).max(0.0).ceil() as usize).max(1) + 12;
    let tiled_width = px(tile_size_value * cols as f32);
    let tiled_height = px(tile_size_value * rows as f32);
    let tiles = cols.saturating_mul(rows);

    div()
        .absolute()
        .inset_0()
        .size_full()
        .overflow_hidden()
        .rounded(radius)
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .w(tiled_width)
                .h(tiled_height)
                .flex()
                .flex_wrap()
                .items_start()
                .justify_start()
                .children((0..tiles).map(move |_| {
                    img(GLASS_NOISE_ASSET_PATH)
                        .w(tile_size)
                        .h(tile_size)
                        .flex_none()
                        .object_fit(ObjectFit::Cover)
                        .opacity(opacity)
                })),
        )
}
