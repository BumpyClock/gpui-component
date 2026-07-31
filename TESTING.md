# Testing and CI

CI reports validation by level. A compile-only job is not a runtime test.

## Unit and doctest

Command:

```bash
cargo test --workspace --all-targets --locked --features test-support
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

## Legacy native smoke

`native-launch-smoke` remains a Stage 0 launch check. It builds AppShell
conformance examples on macOS, Windows, and Linux, and runs these smoke paths
on macOS:

```bash
cargo run -p app_shell -- --asset-smoke
cargo run -p app_shell -- --smoke
cargo run -p app_shell_background -- --smoke
```

It is not renderer-presentation evidence. Windows retains only its no-window
transactional-start failure smoke in that legacy job.

## Stage 1 lifecycle headless

`stage1-lifecycle-headless-macos`, `stage1-lifecycle-headless-windows`, and
`stage1-lifecycle-headless-linux` are separate jobs configured to run:

```bash
cargo test --locked -p gpui-component-app --test headless --features test-support
```

Each command runs under an external bounded watchdog and retains target metadata,
stdout, stderr, and watchdog logs. `RUST_BACKTRACE=1` is configured for retained
failure diagnostics. The target uses the injected headless runner, so these jobs
prove lifecycle behavior only: normal return, startup failure, shutdown ordering,
and zero-window liveness. They make no native-event-loop, native-window,
clipboard, renderer, or presentation claim.

## Stage 1 native runtime conformance

The independently visible native jobs are:

- `stage1-native-macos-metal`
- `stage1-native-windows-warp`
- `stage1-native-linux-x11-lavapipe`
- `stage1-native-linux-wayland-lavapipe`

They are configured to build `gpui-component-conformance`, record target
metadata, run its native scenarios under the same external watchdog, retain
stdout JSONL/stderr/logs, and validate each terminal JSONL stream with
`gpui-component-conformance --validate <scenario> --profile <profile>`, using
the profile named after its native job. Validation is deliberately a hard gate:
the workflow must fail if that interface is absent or rejects the stream, rather
than fall back to process status or text matching.

Every native window contributes one ordered, pointer-free evidence group:

1. `native_window_handle`
2. `native_display_handle`
3. `renderer_info`
4. `frame_presented`

The handle records serialize only native kinds, never pointer or integer handle
values. A display classification proves that GPUI exposed the matching native
display family; neither handle record proves rendering or presentation. Profile
validation requires exactly one group for `lifecycle-clean`, `menu-command`, and
`clipboard`, exactly two for `window-cycle`, and zero for
`lifecycle-startup-failure` and `lifecycle-background-quit`. Missing, reordered,
mismatched, incomplete, or extra groups fail. Unknown record or payload fields,
scenario-invalid events, duplicate lifecycle milestones, and known failure or
rejection records also invalidate an otherwise passed trace.

Each native job also configures `stage1_clipboard_harness.py` for the `clipboard`
scenario. It streams JSONL until the scenario declares its expected payload and
loopback acknowledgement address, uses the platform's independent clipboard
reader while the app remains alive, compares normalized output exactly, sends
`verified\n`, requires a subsequent `clipboard_acknowledged` record, then
requires an orderly terminal record and validator success. A missing scenario,
malformed readiness record, reader mismatch, or absent acknowledgement is a
failure, not a clipboard claim. On Windows, the clipboard scenario is configured
to start in an owned kill-on-close Job Object before scenario code executes;
that provides watchdog cleanup containment, not clipboard evidence.

Profiles are exact contracts, not a strength ordering:

| Profile | Window / display kinds | Renderer contract | Presentation tag |
|---|---|---|---|
| `macos-metal` | `app_kit` / `app_kit` | default Metal hardware adapter | `backend_accepted` |
| `windows-warp` | `win32` / `windows` | software D3D11 adapter whose description contains `WARP` or is exactly `Microsoft Basic Render Driver` (case-insensitive) | `backend_accepted` |
| `linux-x11-lavapipe` | `xcb` / `xcb` | software Vulkan WGPU adapter whose name contains `lavapipe` or `llvmpipe` | `api_submitted` |
| `linux-wayland-lavapipe` | `wayland` / `wayland` | software Vulkan WGPU adapter whose name contains `lavapipe` or `llvmpipe` | `api_submitted` |

Windows sets `GPUI_RENDERER=software` and
`GPUI_DISABLE_DIRECT_COMPOSITION=1`. Linux X11 starts Xvfb and unsets
`WAYLAND_DISPLAY`; Xlib evidence is not accepted by this profile. Linux jobs
constrain `VK_ICD_FILENAMES` to the lavapipe ICD and separately require adapter
name evidence. This proves software-GPU selection, not hardware-GPU execution.

Linux Wayland first starts a normal Weston 16 headless/Pixman compositor for
`lifecycle-clean`, `lifecycle-startup-failure`,
`lifecycle-background-quit`, `window-cycle`, and `menu-command`. It stops that
compositor before starting the official private client-test fixture. Only the
clipboard scenario runs inside the 320x240 Pixman test-desktop fixture. Its C
fixture owns one Bash clipboard-orchestrator child, which owns separate GPUI
conformance and external clipboard-reader descendants. The reader uses the
private protocol only to focus its own surface, then transfers the selection
through the ordinary `wl_data_device` protocol.

After first presentation, the GPUI child asks the private fixture to activate
its own `wl_surface`, waits for matching `wl_keyboard.enter`, injects pressed and
released Linux `KEY_A`, receives the compositor's normal `wl_keyboard.key`
serial, records that serial through GPUI's ordinary `SerialTracker`, and handles
the resulting non-held `KeyDownEvent`. The key callback writes the clipboard;
the request completes only after normal input dispatch accepts the event. The
external reader then focuses its own surface and reads the selection before the
orchestrator sends its loopback acknowledgement. This proves the
focus/key/serial path through GPUI selection handling. Weston 16 does
not validate the selection serial itself, so this does not prove rejection of an
invalid serial. It also does not prove physical input, arbitrary compositor
support, or production support for `weston_test`.

Linux target validation requires `api_submitted` only; WGPU
`SurfaceTexture::present()` does not prove backend acceptance. macOS and Windows
require exact `backend_accepted`, which still does not prove display scanout.

All native-job artifact uploads use `if: always()`. These are configured checks,
not a record that any Stage 1 job has run. They do not replace retained Stage 0
matrix evidence; Stage 1 renderer/presentation status remains `not-verified`
until a retained, validated run establishes that platform evidence. See [Testing
and Runtime Evidence](docs/docs/runtime-evidence.md) for evidence levels, OS
clipboard-reader requirements, and explicit non-claims.

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
