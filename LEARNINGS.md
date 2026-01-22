# Learnings

## 2026-01-22
Context: restore glass styling for popup menus and selects.
What I tried: wrapped menu content with SurfacePreset::flyout and used blur_enabled from GlobalState.
Outcome: popover surfaces now handle opacity, border, elevation, and noise consistently.
Next time: prefer SurfacePreset::flyout for popover containers and keep content styling separate.

## 2026-01-22
Context: noise asset failing to load for surface overlays.
What I tried: used `img("NoiseAsset_256.png")` with gpui assets.
Outcome: gpui treated the path as a URI; loading failed until using `ImageSource::Resource(Resource::Embedded(...))`.
Next time: use explicit embedded resource sources for non-URI image assets.
