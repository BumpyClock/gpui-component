# Compatibility Matrix

Single source of truth for which gpui fork revision each gpui-component release is
built and tested against. Maintained by the `/update-gpui` skill — every gpui bump
updates this table, the workspace version, and the release tag together.

Consumers (agent-term, ansible, Andromeda, and future apps) should:

- pin one tag of this workspace's crates (`gpui-component`, `gpui-component-app`,
  `gpui-component-storage`, `gpui-component-manifest` release together at the
  workspace version);
- declare gpui/gpui_platform (where needed directly) at the **same rev** listed here,
  centralized in `[workspace.dependencies]`;
- run a CI lint over `cargo metadata` rejecting more than one resolved
  source/rev for `gpui`, `gpui_platform`, or `gpui-component`
  (see docs/learned/app-platform-plan.md, D6).

| gpui-component version (tag) | gpui fork rev | Date | Notes |
|---|---|---|---|
| 0.5.1 (`v0.5.1`) | `4332ea7deae4838c12bad6ea64292ca22a33cf98` | 2026-07-16 | First tracked pairing; app-platform crates (storage, manifest) introduced. |
| 0.6.0 (`v0.6.0`) | `2a03ae6e789b77e98f9d9bd5489758a082313c75` | 2026-07-17 | MainThreadPoster; AppProxy wake-on-send. |

## Release discipline

1. gpui fork commits that gpui-component adopts get an annotated tag in the fork repo
   (`gpui-vYYYY.MM.DD-<shortsha>`).
2. This workspace shares one `[workspace.package] version`; bumping it (via
   `/update-gpui` or a release) creates the matching annotated tag `v<version>` here.
3. A gpui rev bump or breaking API change ⇒ minor bump while pre-1.0.
4. Release-candidate CI builds the three app repos against the proposed rev before the
   tag is blessed (plan §6 gate 8).
