## Plan
Canonicalize theme serialization keys to dot-notation and remove the story-only mapper bridge.

## Acceptance
- Theme serialization uses canonical keys.
- Theme story reads canonical keys directly.
- mapper.rs is removed.
- Regression coverage validates key completeness/behavior.