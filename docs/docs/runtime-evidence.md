---
title: "Testing and Runtime Evidence"
summary: "What automated tests establish, and the limits of their platform evidence."
order: -6
---

# Testing and Runtime Evidence

Stage 1 CI jobs are configured runtime checks, not historical evidence. This page
records their intended assertions and evidence limits; it does **not** record a
successful run or replace retained Stage 0 evidence. A platform result becomes
retained evidence only when a job
produces its JSONL, stderr, watchdog, and service logs and the conformance
validator accepts the terminal record.

## Evidence levels

| Level | What it establishes | What it does not establish |
|---|---|---|
| Unit proof | Isolated logic and deterministic state transitions. | An event loop, native window, or graphics API call. |
| Headless integration proof | AppShell lifecycle behavior through the injected headless runner, including normal return and startup failure. | A native OS event loop, native window, clipboard, renderer, or presentation. |
| Native event-loop proof | The application returns normally after an actual platform event loop processes the scenario. | That a native window or renderer was selected unless separate records prove it. |
| Native window proof | Matching pointer-free raw window and display classifications: AppKit/AppKit, Win32/Windows, Xcb/Xcb, or Wayland/Wayland. | Pointer values, a visible desktop frame, rendering, presentation, or display scanout. |
| Presentation API submission proof | The renderer reached its platform-specific first presentation API observation. For WGPU, this means `SurfaceTexture::present()` returned. | Backend acceptance, compositor presentation, display scanout, or user-visible pixels. |
| Backend-acceptance proof | The platform-specific Metal or D3D11 conformance path reports backend acceptance or scheduling. | Display scanout, presentation completion, or general GPU correctness. |
| Software-GPU proof | The selected adapter is software: D3D11 WARP or a CPU Vulkan adapter constrained to lavapipe with matching adapter-name evidence. | Physical GPU execution, hardware performance, or driver certification. |
| Hardware-GPU proof | The selected Metal adapter is classified as hardware. | Performance, thermal behavior, display scanout, or broad device compatibility. |
| Manual-only proof | A capability needs a human or dedicated external accessibility/input evaluation. | Automated certification. |

## Platform-specific automated scope

The workflow validates each native scenario JSONL stream with the explicit
profile for its target job rather than treating process status as sufficient
evidence. These configured checks are not retained platform evidence until a
validated CI run is retained.

| Job | Exact target assertions | Required presentation tag |
|---|---|---|
| `stage1-native-macos-metal` (`macos-metal`) | `app_kit` window and display; default Metal hardware adapter. | `backend_accepted` |
| `stage1-native-windows-warp` (`windows-warp`) | `win32` / `windows`; software D3D11; WARP-correlated adapter description; DirectComposition disabled. | `backend_accepted` |
| `stage1-native-linux-x11-lavapipe` (`linux-x11-lavapipe`) | exact `xcb` window and display under Xvfb; software Vulkan WGPU adapter constrained to lavapipe and named lavapipe/llvmpipe. | `api_submitted` only |
| `stage1-native-linux-wayland-lavapipe` (`linux-wayland-lavapipe`) | `wayland` window and display under isolated Weston; software Vulkan WGPU adapter constrained to lavapipe and named lavapipe/llvmpipe. | `api_submitted` only |

Profiles require exact tags and values; `backend_accepted` is not accepted as a
substitute for Linux `api_submitted`. Windows accepts adapter descriptions that
contain `WARP` or equal `Microsoft Basic Render Driver` case-insensitively.

The three `stage1-lifecycle-headless-*` jobs intentionally establish only the
headless lifecycle level. They do not create a native window and do not make
native renderer or presentation claims.

## Framework contract boundaries

The following are framework contracts and their maximum proof level. They are
not execution results: a listed proof level requires a retained, validated run
or focused test before it becomes platform evidence.

| Contract | Framework behavior under test | Maximum proof level | Limit |
|---|---|---|---|
| Window lifecycle | A keyed `WindowManager` window opens with a pointer-free native handle; explicit exit permits close and recreate without accidental app exit. | Native event loop + native window | A handle and lifecycle records do not prove visibility, compositor behavior, or scanout. |
| Menus | A registered semantic command and its enabled, checked, or unchecked state project into the application menu; an eligible command dispatches once through the framework command path. | Native event loop + native window | It does not prove a human activated an OS menu, menu rendering, or menu accessibility. |
| Platform clipboard | The application writes a payload through the platform clipboard and remains alive until an independent reader acknowledges it. | Configured native external handshake | `read_from_clipboard` or an in-process cache is not external proof; no clipboard claim exists before the OS-reader handshake succeeds. |
| Focus and text | Deterministic focus traversal reaches the expected target, and a test driver exercises text insertion, selection, and activation transitions. | Focused deterministic test | It does not prove physical keyboard routing, OS focus policy, or IME behavior. |
| Composition injection | Deterministic marked/preedit, replacement, commit, cancellation, and focus-loss transitions preserve the framework text contract. | Focused deterministic injection | Injected composition is not comprehensive IME, candidate-window, keyboard-layout, or platform input-method certification. Mouse caret relocation during marked text, Escape semantics, and blur while context-menu state is active remain product-policy choices. |
| Scaling | Logical/device conversions and rounding are checked at injected fractional and integral scale factors, including 1.25, 1.5, and 2.0. | Focused conversion test | It does not prove monitor-DPI changes, multi-display movement, or native compositor scaling. |
| AccessKit tree publication | The framework publishes roles, names, states, values, focus, and actions into the AccessKit tree; native adapter publication proves only that the adapter receives that tree. | Focused tree test / native adapter publication | It is not VoiceOver, Narrator, Orca, or other screen-reader certification. |

## Artifact and validation contract

A native scenario writes schema-versioned JSONL to stdout. The CI watchdog
captures that file separately from stderr and its own timeout/process log. CI
then supplies the JSONL to:

```text
gpui-component-conformance --validate <scenario> --profile <profile>
```

The validation command must accept terminal JSONL from stdin. If that interface
is unavailable, a native job must fail rather than replace it with ad-hoc text
matching. Each native window must emit one contiguous ordered group:
`native_window_handle`, `native_display_handle`, `renderer_info`, then
`frame_presented`. The validator requires one group for `lifecycle-clean`,
`menu-command`, and `clipboard`; two for `window-cycle`; and zero for
`lifecycle-startup-failure` and `lifecycle-background-quit`. It rejects missing,
reordered, incomplete, mismatched, or extra groups and rejects known failure or
rejection records in passed traces. Protocol parsing also rejects unknown
record or payload fields, scenario-invalid event names, and duplicate lifecycle
milestones. Handle records contain kinds only, never addresses or numeric
handles.

The headless lifecycle target is a Rust integration executable rather than the
native conformance protocol, so its artifacts are stdout/stderr and watchdog
logs only.

Every Stage 1 job uploads its directory even after a failed command. Linux
native artifacts additionally retain Xvfb readiness/service logs or pinned
Weston metadata, TAP, compositor logs, and `vulkaninfo` output. Xvfb uses an
active readiness probe. The Wayland job runs non-clipboard scenarios on a normal
Weston headless/Pixman desktop-shell compositor and stops it before the private
fixture starts. Only `clipboard` runs in Weston's 320x240 Pixman test-desktop
client-test fixture. The fixture owns a Bash clipboard-orchestrator child, which
owns separate GPUI conformance and clipboard-reader descendants. The prepared
private fixture relaxes Weston's single-test-client guard while retaining the
first client as harness owner. The reader uses `weston_test` only to focus its
own surface, then transfers the selection through the ordinary `wl_data_device`
protocol. Elapsed time is never treated as readiness.

## Clipboard, input, and accessibility boundaries

The required native clipboard flow must emit a `clipboard_ready` JSONL record
with a nonempty UTF-8 `expected_payload` with no trailing line break and an
`ack_address` of the form `127.0.0.1:<port>` while the application remains alive.
The external harness must read that payload with `pbpaste` on macOS, PowerShell
`Get-Clipboard -Raw` on Windows, `xclip -selection clipboard -o` on Linux X11,
or the fixture-built external `gpui-wayland-clipboard-reader` on Linux Wayland;
normalize reader line endings and exact-compare the result; connect to the
loopback address; and send `verified\n`.
The scenario must emit `clipboard_acknowledged` only after receiving that exact
acknowledgement; only then may it quit, emit its terminal record, and be
validated. An arbitrary delay, GPUI self-read, or an in-process clipboard cache is not native
clipboard proof. Until this handshake runs successfully, CI makes no external
clipboard claim.

The Linux Wayland clipboard path uses Weston's private test protocol only in the
pinned source-built client-test fixture. After first-presentation evidence, the
GPUI child requests activation for its own `wl_surface`, waits for the matching
`wl_keyboard.enter`, asks Weston to inject pressed and released Linux `KEY_A`,
then receives the compositor's ordinary `wl_keyboard.key` event. GPUI records
that event's genuine compositor serial in its normal `SerialTracker`, dispatches
a non-held `KeyDownEvent` for `a`, and writes the clipboard synchronously inside
that callback. The request completes only after normal input dispatch accepts
the event. Because the test-desktop shell does not activate newly created client
surfaces, the external reader separately requests focus for its own surface
before reading the selection through `wl_data_device`.

This establishes synthetic compositor-test focus/key/serial propagation through
GPUI's ordinary Wayland selection path. Weston 16 does not validate that
selection serial, so it does not prove rejection of an invalid serial. It also
does not establish physical keyboard input, arbitrary compositor support, or a
production serial bypass.

Focused tests can cover deterministic composition state, focus traversal, scale
conversion, and accessibility-tree contents. Deterministic composition injection
does not constitute comprehensive IME coverage, and AccessKit tree or native
adapter publication does not constitute screen-reader certification. Those remain
manual-only validation work.

## Explicit non-claims

At this source checkpoint, native Windows runtime, live Linux X11 runtime, live
Linux Wayland GPUI runtime, and GitHub Actions are unrun. Source inspection,
unit tests, compile checks, or a Weston fixture probe must not be reported as
those runtime results.

- Native window construction and matching display-handle classification are not presentation evidence.
- First-presentation evidence is not display scanout evidence.
- Linux WGPU/lavapipe jobs claim `ApiSubmitted`, not WGPU backend acceptance.
- WARP and lavapipe are software adapters, not hardware GPU evidence.
- The automated suite does not certify comprehensive IME behavior.
- The automated suite does not certify VoiceOver, Narrator, Orca, or any other
  screen-reader integration.

See the repository `TESTING.md` inventory for commands and CI job coverage. The
generated [compatibility matrix](../COMPATIBILITY.md) remains the source for
published platform-status evidence.
