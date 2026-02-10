# Agent Instructions

This project uses **bd** (beads) for issue tracking. Run `bd onboard` to get started.

## Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --status in_progress  # Claim work
bd close <id>         # Complete work
bd sync               # Sync with git
```

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

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
