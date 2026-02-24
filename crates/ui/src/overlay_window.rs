use gpui::{
    Bounds, DisplayId, Pixels, Point, Size, WindowBackgroundAppearance, WindowBounds, WindowKind,
    WindowOptions, px,
};

pub struct OverlayWindowOptions {
    initial_size: Size<Pixels>,
    origin: Point<Pixels>,
    app_id: Option<String>,
    display_id: Option<DisplayId>,
}

impl OverlayWindowOptions {
    pub fn new(initial_size: Size<Pixels>) -> Self {
        Self {
            initial_size,
            origin: Point {
                x: px(0.0),
                y: px(0.0),
            },
            app_id: None,
            display_id: None,
        }
    }

    pub fn app_id(mut self, app_id: impl Into<String>) -> Self {
        self.app_id = Some(app_id.into());
        self
    }

    pub fn origin(mut self, origin: Point<Pixels>) -> Self {
        self.origin = origin;
        self
    }

    pub fn display_id(mut self, display_id: Option<DisplayId>) -> Self {
        self.display_id = display_id;
        self
    }

    pub fn to_window_options(self) -> WindowOptions {
        WindowOptions {
            app_id: self.app_id,
            titlebar: None,
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: self.origin,
                size: self.initial_size,
            })),
            focus: false,
            show: false,
            kind: WindowKind::PopUp,
            is_movable: false,
            is_minimizable: false,
            is_resizable: false,
            display_id: self.display_id,
            window_background: WindowBackgroundAppearance::Transparent,
            window_decorations: None,
            window_min_size: None,
            has_shadow: Some(false),
            tabbing_identifier: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OverlayWindowOptions;
    use gpui::{
        DisplayId, Point, Size, WindowBackgroundAppearance, WindowBounds, WindowKind, px,
    };

    #[test]
    fn to_window_options_maps_overlay_window_fields() {
        let size = Size {
            width: px(200.0),
            height: px(48.0),
        };
        let origin = Point {
            x: px(12.0),
            y: px(24.0),
        };

        let options = OverlayWindowOptions::new(size)
            .origin(origin)
            .to_window_options();

        assert!(matches!(
            options.window_bounds,
            Some(WindowBounds::Windowed(bounds)) if bounds.origin == origin && bounds.size == size
        ));
        assert!(!options.show);
        assert_eq!(options.kind, WindowKind::PopUp);
        assert_eq!(
            options.window_background,
            WindowBackgroundAppearance::Transparent
        );
        assert_eq!(options.has_shadow, Some(false));
    }

    #[test]
    fn to_window_options_maps_app_id_and_display_id() {
        let display_id = Some(DisplayId::new(7));

        let options = OverlayWindowOptions::new(Size {
            width: px(200.0),
            height: px(48.0),
        })
        .app_id("com.adityasharma.ansible.overlay")
        .display_id(display_id)
        .to_window_options();

        assert_eq!(
            options.app_id.as_deref(),
            Some("com.adityasharma.ansible.overlay")
        );
        assert_eq!(options.display_id, display_id);
    }
}
