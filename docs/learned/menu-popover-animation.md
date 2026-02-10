# Menu and Popover Animation Notes

Date: 2026-02-10

## Goals

- Add consistent motion to popover and popup menus.
- Animate submenu open/close instead of snapping.
- Keep reduced-motion behavior deterministic.
- Preserve interactive child controls in collapsed sidebar flyouts.

## Implementation

- Popover:
  - Enter uses `fast_invoke` curve.
  - Exit uses `point_to_point` curve.
  - Visuals: opacity + small anchor-aware `translate_y`.

- PopupMenu:
  - Root menu enter animation: opacity + small `translate_y`.
  - Submenu open/close uses `keyed_presence` with side-aware `translate_x`.

- Dropdown menu lifecycle:
  - Menu cache cleanup delay now matches dismiss animation timing.
  - Reduced-motion path clears immediately.

- Sidebar collapsed flyout:
  - Child items without interactive suffix render in `PopupMenu`.
  - Child items with interactive suffix render as live sidebar rows inside `Popover` content.

## Caveats

- Avoid overshoot/bounce easing for opacity/size/visibility transitions; use monotonic curves only.
- For transform-heavy effects, prefer bounded distances (`~4px`) to avoid blur jitter.
