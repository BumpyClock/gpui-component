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
