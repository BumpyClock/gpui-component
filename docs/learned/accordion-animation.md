# Accordion Animation Notes

Date: 2026-02-10

## Problem

- Content reveal animated.
- Container/sibling reflow not perceived as animated.
- Collapse path removed content immediately (`when(self.open, ...)`), so no exit/layout transition.

## Fix Pattern

- Keep content mounted while closing.
- Track prior open state with `window.use_keyed_state`.
- On open state change:
  - animate with `with_animation`
  - update keyed state after animation duration
- Render condition: `open || (was_open && !reduced_motion)`.
- Progress: `delta` for opening, `1 - delta` for closing.

## Token Alignment

- Source: `theme.motion` defaults (Fluent-aligned).
- Applied curve: `point_to_point_easing`.
- Duration used: `fast_duration_ms`.

## Spring Variant

- Open transition: `Animation::new(fast_duration_ms).with_easing(bounce(ease_in_out))`.
- Close transition: keep `point_to_point_easing` (clean, quick dismiss).
- Height uses shaped progress (`powf(3.0)`) so sibling reflow does not finish too early with large max-height caps.

## Why It Works

- Height shrinks/expands over time, so parent box and following accordion items reflow continuously.
- Exit animation now visible before unmount.
