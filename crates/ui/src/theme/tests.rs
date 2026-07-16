use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use gpui::Hsla;

use super::{Colorize as _, ThemeColor, ThemeConfig, ThemeSet, try_parse_color};

const MIN_TEXT_CONTRAST: f32 = 4.5;

fn bundled_theme_configs() -> Vec<(PathBuf, ThemeConfig)> {
    let themes_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../themes");
    let mut paths = fs::read_dir(&themes_dir)
        .expect("bundled themes directory should be readable")
        .map(|entry| {
            entry
                .expect("theme directory entry should be readable")
                .path()
        })
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    paths.sort();

    paths
        .into_iter()
        .flat_map(|path| {
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
            let set: ThemeSet = serde_json::from_str(&source)
                .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()));
            set.themes
                .into_iter()
                .map(move |config| (path.clone(), config))
        })
        .collect()
}

fn resolve_colors(config: &ThemeConfig) -> ThemeColor {
    let defaults = if config.mode.is_dark() {
        ThemeColor::dark()
    } else {
        ThemeColor::light()
    };
    let mut colors = ThemeColor::default();
    colors.apply_config(config, defaults.as_ref());
    colors
}

fn hsla_to_rgba(color: Hsla) -> [f32; 4] {
    let chroma = (1. - (2. * color.l - 1.).abs()) * color.s;
    let hue = color.h * 6.;
    let secondary = chroma * (1. - (hue.rem_euclid(2.) - 1.).abs());
    let (red, green, blue) = match hue as usize {
        0 => (chroma, secondary, 0.),
        1 => (secondary, chroma, 0.),
        2 => (0., chroma, secondary),
        3 => (0., secondary, chroma),
        4 => (secondary, 0., chroma),
        _ => (chroma, 0., secondary),
    };
    let match_lightness = color.l - chroma / 2.;
    [
        red + match_lightness,
        green + match_lightness,
        blue + match_lightness,
        color.a,
    ]
}

fn composite(foreground: [f32; 4], background: [f32; 4]) -> [f32; 4] {
    let alpha = foreground[3] + background[3] * (1. - foreground[3]);
    let channel = |index| {
        (foreground[index] * foreground[3]
            + background[index] * background[3] * (1. - foreground[3]))
            / alpha
    };
    [channel(0), channel(1), channel(2), alpha]
}

fn relative_luminance(color: [f32; 4]) -> f32 {
    let linear = |channel: f32| {
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color[0]) + 0.7152 * linear(color[1]) + 0.0722 * linear(color[2])
}

fn contrast_ratio(foreground: Hsla, surface: Hsla, background: Hsla) -> f32 {
    let background = hsla_to_rgba(background);
    let surface = composite(hsla_to_rgba(surface), background);
    let foreground = composite(hsla_to_rgba(foreground), surface);
    let foreground_luminance = relative_luminance(foreground);
    let surface_luminance = relative_luminance(surface);
    (foreground_luminance.max(surface_luminance) + 0.05)
        / (foreground_luminance.min(surface_luminance) + 0.05)
}

#[test]
fn bundled_themes_meet_text_contrast_floor() {
    let mut failures = Vec::new();

    for (path, config) in bundled_theme_configs() {
        let colors = resolve_colors(&config);
        let pairs = [
            ("muted", colors.muted_foreground, colors.background),
            ("primary", colors.primary_foreground, colors.primary),
            ("danger", colors.danger_foreground, colors.danger),
            ("success", colors.success_foreground, colors.success),
            ("warning", colors.warning_foreground, colors.warning),
            ("info", colors.info_foreground, colors.info),
        ];

        for (name, foreground, surface) in pairs {
            let ratio = contrast_ratio(foreground, surface, colors.background);
            if ratio < MIN_TEXT_CONTRAST {
                failures.push(format!(
                    "{} / {} / {name}: {ratio:.2}:1",
                    path.display(),
                    config.name
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "bundled theme contrast failures:\n{}",
        failures.join("\n")
    );
}

#[test]
fn bundled_themes_define_distinct_chart_palettes() {
    let mut failures = Vec::new();

    for (path, config) in bundled_theme_configs() {
        let configured = [
            config.colors.chart_1.as_ref(),
            config.colors.chart_2.as_ref(),
            config.colors.chart_3.as_ref(),
            config.colors.chart_4.as_ref(),
            config.colors.chart_5.as_ref(),
        ];
        if configured.iter().any(|color| color.is_none()) {
            failures.push(format!(
                "{} / {}: missing chart.1 through chart.5",
                path.display(),
                config.name
            ));
            continue;
        }
        if let Some(invalid) = configured
            .iter()
            .flatten()
            .find(|color| try_parse_color(color).is_err())
        {
            failures.push(format!(
                "{} / {}: invalid chart color {invalid}",
                path.display(),
                config.name
            ));
            continue;
        }

        let colors = resolve_colors(&config);
        let palette = [
            colors.chart_1.to_hex(),
            colors.chart_2.to_hex(),
            colors.chart_3.to_hex(),
            colors.chart_4.to_hex(),
            colors.chart_5.to_hex(),
        ];
        if palette.iter().collect::<BTreeSet<_>>().len() != palette.len() {
            failures.push(format!(
                "{} / {}: duplicate chart colors {palette:?}",
                path.display(),
                config.name
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "bundled theme chart palette failures:\n{}",
        failures.join("\n")
    );
}
