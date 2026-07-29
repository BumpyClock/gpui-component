# Testing and CI

CI reports validation by level. A compile-only job is not a runtime test.

## Unit and doctest

Command:

```bash
cargo test --workspace --all-targets --locked
cargo test --workspace --doc --locked
```

Maintained unit targets include GPUI Component builders, layout and text logic,
theme and asset handling; AppShell commands, lifecycle, settings, storage, and
window planning; application-manifest parsing, versioning, and doctor behavior;
the request client; and compatibility-tool tests.

## Headless integration

The same command runs deterministic integration targets without a native
presentation surface, including:

- `crates/app/tests/headless.rs` for AppShell lifecycle behavior;
- `crates/app-manifest/tests/downstream.rs` for downstream manifest use;
- `crates/app-storage/tests/process_lock.rs` for cross-process storage locking.

`process_lock` has two `#[ignore]` child-role tests. They are intentionally not
standalone targets: its driver starts them explicitly with `--ignored --exact`.
They are covered when the integration driver runs.

The doctest lane includes the `gpui-component-assets` `Assets` example. Its
manifest declares the required GPUI platform dependency for doctest builds.

## Native smoke

`native-launch-smoke` builds AppShell conformance examples on macOS, Windows,
and Linux. It opens a real window and executes smoke paths on macOS only:

```bash
cargo run -p app_shell -- --asset-smoke
cargo run -p app_shell -- --smoke
cargo run -p app_shell_background -- --smoke
```

Windows runs only the no-window transactional-start failure smoke. Native
DirectX presentation remains blocked by the hosted runner's lack of a reliable
present surface. Linux native smoke remains blocked by the lack of a stable
display plus Vulkan software-renderer setup; Stage 1 needs an `xvfb` and
lavapipe-capable runner. Linux and Windows native-runtime evidence must remain
`not-verified` until those blockers are resolved.

## Packaging and compatibility

```bash
cargo xtask compatibility check
cargo xtask publish-plan
cargo xtask release-check
```

`release-check` validates source build, unit/headless tests, package file lists,
and normalized manifests. `--require-registry` additionally requires published
exact GPUI engine packages; its failure is expected until those prerequisites
exist.

## Compile-only

The `Compile and lint` CI matrix runs Clippy on macOS, Windows, and Linux. It
compiles all targets and features but does not establish native event-loop,
window, or renderer behavior. Platform evidence and support maturity live in
the generated [compatibility matrix](docs/COMPATIBILITY.md).
