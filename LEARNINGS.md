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

## 2026-02-10
Context: opening command palette in story app panicked with `delta should always be between 0 and 1`.
What I tried: traced call path (`open_dialog` -> `dialog` slide animation -> `strong_invoke_easing`) and sampled default curve `cubic-bezier(0.13, 1.62, 0, 0.92)`.
Outcome: custom cubic-bezier easing returned values above `1.0`, triggering GPUI animation debug assert.
Next time: keep custom easing outputs bounded in `[0, 1]` (clamp/sanitize in easing helper) and add regression test for overshoot curves.

## 2026-02-10
Context: accordion content animated in, but container/sibling reflow felt abrupt.
What I tried: kept content mounted during close with keyed open-state + delayed state commit, animated both directions with `with_animation`.
Outcome: expand and collapse both animate height/opacity, sibling items move smoothly with the panel.
Next time: for collapsible UI, avoid conditional unmount on close if exit/layout animation needed.

## 2026-02-10
Context: wanted a fun spring feel without making dismiss motion heavy.
What I tried: spring easing only on open (`bounce(ease_in_out)`), point-to-point easing on close, shaped height progress with `powf(3.0)`.
Outcome: open feels playful; close remains crisp; sibling reflow reads longer and smoother.
Next time: split open/close curves for collapsibles instead of one shared easing.

## 2026-02-10
Context: command palette felt busy because shell enter and content expand happened together.
What I tried: added dialog-level `animate(false)` for command palette only, then kept delayed list reveal + separate children fade.
Outcome: shell appears instantly, content animates second, overall motion feels more fluid and intentional.
Next time: stage modal motion in phases (container first, list/items second) for search palettes.

## 2026-02-10
Context: command palette open still felt a bit mechanical after staging.
What I tried: switched expand phase easing to `bounce(ease_in_out)` while keeping shell instant and reduced-motion guard.
Outcome: open feels more playful without changing close behavior.
Next time: tune spring intensity per surface; keep dismissal curves simpler than entrance.

## 2026-02-10
Context: command palette story crashed on selection with `cannot update ... while it is already being updated`.
What I tried: removed nested `view.update(...)` calls from `cx.subscribe(...)` callbacks and updated `this` directly in the subscription closure.
Outcome: no re-entrant lease panic; selection updates render correctly.
Next time: in `cx.subscribe` callbacks, mutate `this` directly; avoid nested entity updates on the same entity.

## 2026-02-10
Context: command palette empty-state icon clipped when there are zero results.
What I tried: replaced zero-results list height from `item_height` to dedicated `EMPTY_STATE_HEIGHT`.
Outcome: empty-state icon/text render fully without clipping.
Next time: avoid reusing row height for empty states that have larger vertical content.
