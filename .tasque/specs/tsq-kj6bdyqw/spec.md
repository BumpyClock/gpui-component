## Goal
Remove deprecated `SettingField<T>` adapters from the public settings API and migrate in-repo docs/examples to `SettingControl`.

## Acceptance
- `crates/ui/src/setting/fields/mod.rs` no longer defines `SettingField<T>`.
- In-repo runtime/story code compiles without `SettingField<T>`.
- Docs/examples under `docs/` use `SettingControl` builders.
- Verification covers `gpui-component`, story crate, formatting, and no remaining `SettingField` references except `SettingFieldElement`.
