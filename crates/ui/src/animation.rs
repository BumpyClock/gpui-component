use gpui::{Animation, App, SharedString, Window, spring};
use std::time::Duration;

use crate::ThemeMotion;

/// A cubic bezier function like CSS `cubic-bezier`.
///
/// Builder:
///
/// https://cubic-bezier.com
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> impl Fn(f32) -> f32 {
    move |t: f32| {
        if !t.is_finite() {
            return 0.0;
        }
        let t = t.clamp(0.0, 1.0);
        let one_t = 1.0 - t;
        let one_t2 = one_t * one_t;
        let t2 = t * t;
        let t3 = t2 * t;

        // The Bezier curve function for x and y, where x0 = 0, y0 = 0, x3 = 1, y3 = 1
        let _x = 3.0 * x1 * one_t2 * t + 3.0 * x2 * one_t * t2 + t3;
        let y = 3.0 * y1 * one_t2 * t + 3.0 * y2 * one_t * t2 + t3;

        if y.is_finite() { y.clamp(0.0, 1.0) } else { 0.0 }
    }
}

/// A cubic bezier function without clamping the output.
pub fn cubic_bezier_unbounded(x1: f32, y1: f32, x2: f32, y2: f32) -> impl Fn(f32) -> f32 {
    move |t: f32| {
        if !t.is_finite() {
            return 0.0;
        }
        let t = t.clamp(0.0, 1.0);
        let one_t = 1.0 - t;
        let one_t2 = one_t * one_t;
        let t2 = t * t;
        let t3 = t2 * t;

        let _x = 3.0 * x1 * one_t2 * t + 3.0 * x2 * one_t * t2 + t3;
        let y = 3.0 * y1 * one_t2 * t + 3.0 * y2 * one_t * t2 + t3;
        if y.is_finite() { y } else { 0.0 }
    }
}

/// Parse a CSS cubic-bezier string into (x1, y1, x2, y2).
pub fn parse_cubic_bezier_easing(value: &str) -> Option<(f32, f32, f32, f32)> {
    let trimmed = value.trim();
    let body = trimmed
        .strip_prefix("cubic-bezier(")?
        .strip_suffix(')')?
        .trim();
    let mut parts = body.split(',').map(str::trim);
    let x1 = parts.next()?.parse::<f32>().ok()?;
    let y1 = parts.next()?.parse::<f32>().ok()?;
    let x2 = parts.next()?.parse::<f32>().ok()?;
    let y2 = parts.next()?.parse::<f32>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((x1, y1, x2, y2))
}

/// Apply a theme easing string to an Animation.
pub fn animation_with_theme_easing(animation: Animation, easing: &str) -> Animation {
    if easing.trim().eq_ignore_ascii_case("linear") {
        return animation.with_easing(|delta: f32| delta);
    }
    if let Some((x1, y1, x2, y2)) = parse_cubic_bezier_easing(easing) {
        let overshoot = y1 < 0.0 || y1 > 1.0 || y2 < 0.0 || y2 > 1.0;
        if overshoot {
            return animation.with_unbounded_easing(cubic_bezier_unbounded(x1, y1, x2, y2));
        }
        return animation.with_easing(cubic_bezier(x1, y1, x2, y2));
    }
    animation
}

/// Create a theme animation with the given duration and easing. Returns None if reduced_motion.
pub fn theme_animation(duration_ms: u16, easing: &str, reduced_motion: bool) -> Option<Animation> {
    if reduced_motion {
        return None;
    }
    let anim = Animation::new(Duration::from_millis(duration_ms as u64));
    Some(animation_with_theme_easing(anim, easing))
}

/// Fast invoke animation (187ms, fast_invoke_easing).
pub fn fast_invoke_animation(motion: &ThemeMotion, reduced_motion: bool) -> Option<Animation> {
    theme_animation(
        motion.fast_duration_ms,
        &motion.fast_invoke_easing,
        reduced_motion,
    )
}

/// Soft dismiss animation (167ms, soft_dismiss_easing).
pub fn soft_dismiss_animation(motion: &ThemeMotion, reduced_motion: bool) -> Option<Animation> {
    theme_animation(
        motion.soft_dismiss_duration_ms,
        &motion.soft_dismiss_easing,
        reduced_motion,
    )
}

/// Point-to-point animation (187ms, point_to_point_easing).
pub fn point_to_point_animation(motion: &ThemeMotion, reduced_motion: bool) -> Option<Animation> {
    theme_animation(
        motion.fast_duration_ms,
        &motion.point_to_point_easing,
        reduced_motion,
    )
}

/// Fade animation (83ms, linear).
pub fn fade_animation(motion: &ThemeMotion, reduced_motion: bool) -> Option<Animation> {
    theme_animation(motion.fade_duration_ms, &motion.fade_easing, reduced_motion)
}

/// Strong invoke animation (667ms, strong_invoke_easing with overshoot bounce).
pub fn strong_invoke_animation(motion: &ThemeMotion, reduced_motion: bool) -> Option<Animation> {
    theme_animation(
        motion.strong_invoke_duration_ms,
        &motion.strong_invoke_easing,
        reduced_motion,
    )
}

pub const DEFAULT_SPRING_DAMPING_RATIO: f32 = 0.75;
pub const DEFAULT_SPRING_FREQUENCY: f32 = 1.8;

/// Spring invoke animation (uses unbounded easing, transform-only).
pub fn spring_invoke_animation(motion: &ThemeMotion, reduced_motion: bool) -> Option<Animation> {
    if reduced_motion {
        return None;
    }
    Some(
        Animation::new(Duration::from_millis(u64::from(motion.fast_duration_ms)))
            .with_unbounded_easing(spring(DEFAULT_SPRING_DAMPING_RATIO, DEFAULT_SPRING_FREQUENCY)),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresencePhase {
    Entering,
    Entered,
    Exiting,
    Exited,
}

#[derive(Clone, Copy, Debug)]
pub struct PresenceTransition {
    pub phase: PresencePhase,
}

impl PresenceTransition {
    pub fn transition_active(self) -> bool {
        matches!(self.phase, PresencePhase::Entering | PresencePhase::Exiting)
    }

    pub fn should_render(self) -> bool {
        self.phase != PresencePhase::Exited
    }

    pub fn progress(self, delta: f32) -> f32 {
        let delta = delta.clamp(0.0, 1.0);
        match self.phase {
            PresencePhase::Entering => delta,
            PresencePhase::Exiting => 1.0 - delta,
            PresencePhase::Entered => 1.0,
            PresencePhase::Exited => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PresenceOptions {
    pub animate_on_mount: bool,
}

/// Shared mount/open/close presence state machine keyed by element id.
///
/// - `target_open=true` moves to Entering/Entered
/// - `target_open=false` moves to Exiting/Exited
/// - stale async timers are ignored via generation guard
pub fn keyed_presence(
    key_base: SharedString,
    target_open: bool,
    animate: bool,
    open_duration: Duration,
    close_duration: Duration,
    options: PresenceOptions,
    window: &mut Window,
    cx: &mut App,
) -> PresenceTransition {
    let initial_open = if options.animate_on_mount && animate {
        false
    } else {
        target_open
    };
    let target_key = SharedString::from(format!("{}-presence-target", key_base));
    let phase_key = SharedString::from(format!("{}-presence-phase", key_base));
    let generation_key = SharedString::from(format!("{}-presence-generation", key_base));
    let target_state = window.use_keyed_state(target_key, cx, |_, _| initial_open);
    let phase_state = window.use_keyed_state(phase_key, cx, |_, _| {
        if initial_open {
            PresencePhase::Entered
        } else {
            PresencePhase::Exited
        }
    });
    let generation_state = window.use_keyed_state(generation_key, cx, |_, _| 0_u64);

    let previous_target = *target_state.read(cx);
    let target_changed = previous_target != target_open;
    if target_changed {
        target_state.update(cx, |state, _| *state = target_open);
        let generation = generation_state.update(cx, |state, _| {
            *state += 1;
            *state
        });

        if !animate {
            let next_phase = if target_open {
                PresencePhase::Entered
            } else {
                PresencePhase::Exited
            };
            phase_state.update(cx, |state, _| *state = next_phase);
        } else if target_open {
            phase_state.update(cx, |state, _| *state = PresencePhase::Entering);
            cx.spawn({
                let target_state = target_state.clone();
                let phase_state = phase_state.clone();
                let generation_state = generation_state.clone();
                async move |cx| {
                    cx.background_executor().timer(open_duration).await;
                    let still_latest = generation_state.update(cx, |state, _| *state == generation);
                    if !still_latest {
                        return;
                    }
                    let still_open = target_state.update(cx, |state, _| *state);
                    if still_open {
                        _ = phase_state.update(cx, |state, cx| {
                            *state = PresencePhase::Entered;
                            cx.notify();
                        });
                    }
                }
            })
            .detach();
        } else {
            phase_state.update(cx, |state, _| *state = PresencePhase::Exiting);
            cx.spawn({
                let target_state = target_state.clone();
                let phase_state = phase_state.clone();
                let generation_state = generation_state.clone();
                async move |cx| {
                    cx.background_executor().timer(close_duration).await;
                    let still_latest = generation_state.update(cx, |state, _| *state == generation);
                    if !still_latest {
                        return;
                    }
                    let still_closed = target_state.update(cx, |state, _| !*state);
                    if still_closed {
                        _ = phase_state.update(cx, |state, cx| {
                            *state = PresencePhase::Exited;
                            cx.notify();
                        });
                    }
                }
            })
            .detach();
        }
    }

    PresenceTransition {
        phase: *phase_state.read(cx),
    }
}

#[cfg(test)]
mod tests {
    use super::cubic_bezier;

    #[test]
    fn strong_invoke_curve_is_bounded() {
        let easing = cubic_bezier(0.13, 1.62, 0.0, 0.92);
        for i in 0..=1_000 {
            let t = i as f32 / 1_000.0;
            let y = easing(t);
            assert!(
                (0.0..=1.0).contains(&y),
                "expected output in [0, 1], got {y} at t={t}"
            );
        }
    }

    #[test]
    fn cubic_bezier_non_finite_input_returns_zero() {
        let easing = cubic_bezier(0.0, 0.0, 1.0, 1.0);
        assert_eq!(easing(f32::NAN), 0.0);
        assert_eq!(easing(f32::INFINITY), 0.0);
        assert_eq!(easing(f32::NEG_INFINITY), 0.0);
    }
}
