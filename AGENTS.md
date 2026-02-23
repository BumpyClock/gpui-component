# Agent Instructions

This project uses **tsq** (tasque) for task management. Use the tasque skill. 

**Platform parity** is critical. When implementing a feature, we must implement it across macOS, Windows, and Linux. If a feature is not supported on a platform, we must document the reason and any potential workarounds.

## Animation Guardrails (GPUI)

- Failure mode: open animation flashes (open -> collapse -> open).
- Root cause: using `bounce(...)` easing for reveal/size/opacity. `bounce` is forward-then-reverse; at `delta=1` it returns ~0, so the end frame collapses.
- Rule: only use monotonic easings (fast_invoke/point_to_point/soft_dismiss) for reveal/size/opacity.
- If you want “spring” feel: simulate with a snappier monotonic curve or staged animations; avoid `bounce` unless you want ping-pong.
- Required pattern for open/close motion:
  1. Keep `target_state` (source-of-truth open/closed).
  2. Keep `visual_state` (mounted/visible during exit animation).
  3. Compute `transition_active = target_changed || (visual_state != target_state)`.
  4. Run `with_animation(...)` **only** when `transition_active`.
  5. On close, delay `visual_state=false` until close duration elapses; guard timer with latest `target_state`.
- For dialogs/surfaces: do not unmount on close request if animation enabled; mark as closing, remove after timer.
- Reduced motion / `animate(false)`: bypass delay + animation, apply final state immediately.
