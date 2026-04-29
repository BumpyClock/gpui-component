## Plan
Harden dock persistence by making version mismatch an explicit incompatibility error, clearing omitted docks during load, and rejecting malformed restored tab payloads. Update story demo loaders to auto-reset and notify on incompatibility.

## Acceptance
- Incompatible saved layouts are not loaded.
- Missing left/right/bottom docks are cleared on restore.
- Invalid restored tab structures fail load instead of silently dropping panels.
- Story demos reset incompatible layouts and notify once.
- Regression tests cover round-trip and version mismatch.