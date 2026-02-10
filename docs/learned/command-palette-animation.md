# Command Palette Animation Notes

Date: 2026-02-10

## Goal

- No simultaneous shell-enter + list-expand motion.
- Shell visible immediately.
- Then results/children reveal.
- Keep shortcut keycap pinned at row right edge.

## Changes

- Added per-dialog animation toggle (`Dialog::animate(bool)`).
- Command palette opens dialog with `animate(false)` so shell appears instantly.
- Kept delayed reveal + expand animation for list area.
- Added list children fade reveal animation.
- Row layout: category text then shortcut in trailing right group.

## Result

- Open sequence feels staged: shell first, content second.
- Shortcut keycaps align to right edge consistently.

## Crash Note

- Story crash (`cannot update ... while it is already being updated`) was caused by nested `view.update(...)` inside `cx.subscribe(...)`.
- Fix: update `this` directly in subscription callback; no nested lease of same entity.
