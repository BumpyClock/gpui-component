# Learnings

## 2026-01-22
Context: popup/select glass surface styling and noise overlay consistency.
What worked:
- Use `SurfacePreset::flyout` on popover containers; keep content styling separate.
- Read noise texture via `ImageSource::Resource(Resource::Embedded(...))`, not `img("...")`.
Outcome: consistent opacity, border, elevation, and noise; no URI-loading failures.
Next time: default to flyout preset + embedded resource images for non-URI assets.

## 2026-02-10
Context: command palette animation stability and quality.
What worked:
- Clamp/sanitize custom easing output to `[0, 1]` to avoid `delta should always be between 0 and 1`.
- Stage motion: shell instant (`animate(false)`), then list/content reveal.
- Keep container expand and child reveal aligned; avoid conflicting opacity/height phases on translucent surfaces.
Outcome: no animation assert panic and cleaner open sequence without white flash.
Next time: add regression test for overshoot cubic-bezier curves; keep dismiss curves simpler than entrance curves.

## 2026-02-10
Context: collapsible/accordion motion felt abrupt.
What worked:
- Keep content mounted during close (delayed commit), animate both open and close.
- Use spring-style easing on open (`bounce(ease_in_out)`) and simpler close easing.
Outcome: smoother sibling reflow and playful open without heavy dismiss.
Next time: avoid conditional unmount when exit/layout animation is required.

## 2026-02-10
Context: command palette selection crash (`cannot update ... while it is already being updated`).
What worked: in `cx.subscribe(...)` callbacks, update `this` directly; remove nested `view.update(...)` on the same entity.
Outcome: no re-entrant lease panic; selection updates render normally.
Next time: treat subscribe callbacks as the update scope; avoid nested entity updates.

## 2026-02-10
Context: command palette empty state clipping/blankness during expand.
What worked:
- Use dedicated `EMPTY_STATE_HEIGHT` instead of row `item_height`.
- Top-align empty-state content (`pt_6`) instead of vertical centering.
Outcome: icon/text render fully and appear earlier during expansion.
Next time: design empty-state layout independently from row metrics when height is animated.

## 2026-02-10
Context: command palette still showed a blank strip under search before results reveal.
What worked:
- Make header row explicitly match `HEADER_HEIGHT` (`h + flex + items_center`) instead of relying on padding-only sizing.
Outcome: collapsed shell height and header layout now match; removed mismatch strip.
Next time: when using fixed layout constants, size the corresponding section explicitly to that constant.

## 2026-02-10
Context: blank strip persisted even after palette view-level fixes.
What worked: override dialog default minimum height for command palette (`.min_h(px(0.))`), because `Dialog` enforces `.min_h_24()` by default.
Outcome: command palette collapsed height now respects its own shell height during pre-reveal.
Next time: when embedding compact overlays inside `Dialog`, explicitly set min-height if default dialog floor is too large.

## 2026-02-10
Context: sidebar and nested menu sections had snap-open/snap-close behavior.
What worked:
- animate sidebar width from a dedicated expanded width source (`Sidebar::width(...)`) instead of style-only width overrides.
- keep a delayed visual-collapsed state so compact paddings/icon-only layout applies after close width animation.
- keep submenu mounted during close and unmount after animation duration.
Outcome: sidebar collapse/expand and submenu section open/close now animate as continuous layout motion; reduced-motion path remains instant.
Next time: if caret icon rotation should animate, avoid coupling icon transform to `Button::icon(...)` and render a custom caret container with direct animation hook.

## 2026-02-10
Context: recurring motion regressions (snap-back, reopen-on-close, no-exit) across dialog/sidebar/accordion/popover.
What worked:
- Introduce shared keyed presence state machine in `animation::keyed_presence` (Entering/Entered/Exiting/Exited).
- Use generation-guarded timers to ignore stale async transitions.
- Drive render + animation from presence phase (`should_render`, `transition_active`, `progress`) instead of ad-hoc booleans.
- For dropdown menus, delay menu entity reset until popover exit completes.
Outcome: motion lifecycle is centralized; avoids per-component timer drift and repeated transition bugs.
Next time: default new animated components to keyed presence first; avoid custom target/visible timer code.

## 2026-02-10
Context: open animations flashed (open -> collapse -> open) on Accordion/Sidebar/Dialog.
What worked:
- Remove `bounce(...)` easing from reveal/size/opacity transitions.
- Use monotonic easings (fast_invoke/point_to_point) for open/close.
Outcome: no end-frame collapse; open state stays stable.
Next time: avoid `bounce` for reveal/size/opacity; it is forward-then-reverse.

## 2026-02-10
Context: vendoring GPUI for local patching.
What worked:
- Add `vendor/gpui` git submodule pinned to current `gpui` rev.
- Keep workspace dependency as git+rev; direct path dependency failed due Zed workspace dependency inheritance.
- Document patch workflow in `docs/learned/gpui-submodule.md`.
Outcome: local GPUI source available for edits/review without breaking build.
Next time: after GPUI patch commit, bump both `Cargo.toml` rev and submodule SHA together.

## 2026-02-10
Context: spring/overshoot easing support.
What worked:
- Add unbounded easing support in GPUI (`Animation::with_unbounded_easing` + bounds).
- Route overshoot cubic-bezier curves to unbounded easing in `crates/ui/src/animation.rs`.
Outcome: Fluent strong-invoke curve can overshoot without debug assert; spring-style easings now supported.
Next time: use unbounded easing only for transform-like properties.

## 2026-02-10
Context: Fluent animation tokens source-of-truth.
What worked: read `/Users/adityasharma/Projects/fluent-tokens/tokens/animation.md` for actual curves/durations.
Outcome: no spring/damping/frequency tokens; only cubic-bezier and duration sets.
Next time: when adding springs, pick local defaults or extend tokens explicitly.

## 2026-02-10
Context: trying to patch GPUI from local submodule during app build.
What worked: path override does not work here because `gpui` uses workspace-inherited deps.
Outcome: proper flow is commit/push submodule changes, then bump `Cargo.toml` `gpui` `rev` to the new commit.
Next time: skip `[patch]` path attempts for this repo; use git `rev` updates only.

## 2026-02-11
Context: implementing GPUI transform foundation for spring-style motion.
What worked:
- Added `Window::with_element_transform(...)` stack and carried it through deferred draw replay.
- Added transformed hitbox insertion + inverse-mapped hit testing (`insert_hitbox_transformed`, `contains_window_point`, `window_to_local`).
- Applied transform-aware bounds/content-mask handling in paint paths and render transform composition for glyph/SVG.
Outcome: transform context now survives prepaint/paint flow and pointer hit testing remains stable under transforms.
Next time: if visual fidelity for rotated/scaled quads/images is needed, add true shader-space transform for those primitives.

## 2026-02-11
Context: spring open motion for popovers/menus without reopen-collapse glitches.
What worked:
- Use `spring_invoke_animation` only for entering transform (translate), keep close with monotonic easing.
- Keep opacity derived from clamped `presence.progress(delta)` while transform uses raw spring delta.
- Align dropdown cached-menu cleanup delay to popover fade close duration.
Outcome: spring feel on open with stable visibility lifecycle and no opacity bounce artifacts.
Next time: avoid feeding unbounded spring progress directly into visibility/height/opacity properties.

## 2026-02-10
Context: collapsed sidebar items with children were not navigable.
What worked: wrap collapsed parent in `Popover` and render children via `PopupMenu` (recursive submenu builder); cache menu entity with keyed state and clear on dismiss.
Outcome: collapsed sidebar click now exposes child actions without expanding the sidebar.
Next time: for collapsed navigation groups, prefer popover menu over inline open-state toggles.

## 2026-02-10
Context: menu/popover motion and collapsed flyout control behavior.
What worked:
- Added popover motion as opacity + small anchor-aware translate using monotonic curves.
- Added popup submenu keyed-presence transitions with side-aware translate.
- For collapsed sidebar flyouts, render live sidebar rows (not popup command rows) when suffix controls exist.
Outcome: smoother menu motion; suffix controls like `Switch` keep normal interaction/animation behavior.
Next time: use `PopupMenu` for command rows, but switch to live popover content for embedded interactive controls.
