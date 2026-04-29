## Plan
Redesign the public settings API around typed controls rather than TypeId-based dispatch, while preserving custom render rows.

## Acceptance
- New primary public API is typed and makes unsupported control combinations unrepresentable.
- TypeId is no longer the main routing path.
- Settings story is migrated to the new API.
- Legacy adapters, if retained, are thin compatibility shims only.