//! `app_shell` — single-window AppShell conformance example.
//!
//! Exercises the whole downstream chain in-repo: a `[package.metadata.gpui-app]`
//! table + `build.rs` produce the compiled-in `APP_IDENTITY`, and the shell
//! builder wires the wave-2 services (typed settings, theme, native menus) around
//! a single `gpui-component` window.
//!
//! Run it: `cargo run -p app_shell`. The `--smoke` flag makes it launch, then
//! request a clean quit after a few seconds with exit code 0 — the shape the CI
//! `native-launch-smoke` job depends on.
//!
//! Smoke contract: **exit 0 iff the window opened and the shell quit cleanly.**
//! A failed native launch must exit non-zero, so a broken window path cannot pass
//! CI. This matters because a `Started` handler error is only *logged* by the
//! shell (`deliver_event`), not propagated — returning `Err` from `on_launch`
//! would otherwise let startup idle-exit 0 with no window ever shown.

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use gpui_component_app::gpui::*;
use gpui_component_app::prelude::*;
use gpui_component_app::ui::{ActiveTheme as _, v_flex};
use serde::{Deserialize, Serialize};

gpui_component_app::include_identity!();

/// How long a `--smoke` run stays up before requesting a clean quit.
const SMOKE_LIFETIME: Duration = Duration::from_secs(3);
/// Hard upper bound: if the app never reaches launch, fail loudly instead of
/// hanging CI. Mirrors the watchdog in `crates/app/tests/headless.rs`.
const SMOKE_WATCHDOG: Duration = Duration::from_secs(30);

/// Set once the main window has actually opened. The `--smoke` quit path asserts
/// it: a run that never opened its window exits non-zero (see the module docs).
static WINDOW_READY: AtomicBool = AtomicBool::new(false);

/// A tiny persisted settings schema, proving the identity -> storage chain: it is
/// keyed by the app's `data_namespace`, which comes from `APP_IDENTITY`.
#[derive(Serialize, Deserialize, Default)]
struct ExampleSettings {
    /// Number of times this app has been launched.
    launch_count: u32,
}

impl AppSettings for ExampleSettings {
    const SCHEMA_VERSION: u32 = 1;
}

/// The window content: a themed placeholder so the theme service is visibly live.
struct MainView;

impl Render for MainView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child("App Shell conformance example")
    }
}

fn main() {
    let smoke = std::env::args().any(|arg| arg == "--smoke");
    if smoke {
        spawn_watchdog();
    }

    let result = AppShell::builder(APP_IDENTITY)
        .settings::<ExampleSettings>(StoreKey::PRIMARY)
        .theme(ThemeSource::registry())
        .menus(MenuPlan::standard().with_theme_menu())
        .on_launch(move |cx| {
            // Arm the smoke quit/verify thread first: even a startup that returns
            // early below still reaches a decision (and fails non-zero) instead
            // of idling to the watchdog.
            if smoke {
                schedule_smoke_quit(cx);
            }

            // Touch the settings store so persistence is actually exercised.
            // A returned `Err` is only *logged* by the shell and would let
            // startup idle-exit 0, so any startup failure fails hard instead.
            if let Err(err) = cx.update_settings(StoreKey::PRIMARY, |s: &mut ExampleSettings, _| {
                s.launch_count += 1;
            }) {
                eprintln!("app_shell settings update failed: {err}");
                std::process::exit(1);
            }

            match WindowManager::open(
                cx,
                WindowSpec::new("main").title("App Shell Example"),
                |_, cx| cx.new(|_| MainView),
            ) {
                Ok(_) => WINDOW_READY.store(true, Ordering::SeqCst),
                // A failed native launch must not pass the smoke gate as a clean
                // exit 0: fail hard here rather than let startup idle-exit.
                Err(err) => {
                    eprintln!("app_shell failed to open window: {err}");
                    std::process::exit(1);
                }
            }
            Ok(())
        })
        .run();

    // FORK REALITY: on macOS the platform terminates the process inside `run()`
    // (NSApp terminate -> exit(0)), so this is reached only on platforms whose
    // event loop returns (Linux/Windows). Do not rely on post-`run` code there.
    if let Err(err) = result {
        eprintln!("app_shell exited with error: {err}");
        std::process::exit(1);
    }
}

/// After [`SMOKE_LIFETIME`], either quit cleanly (window opened) or fail the run
/// (window never became ready) — the smoke contract's decision point.
fn schedule_smoke_quit(cx: &mut App) {
    let proxy = cx.app_proxy();
    thread::spawn(move || {
        thread::sleep(SMOKE_LIFETIME);
        if WINDOW_READY.load(Ordering::SeqCst) {
            // Quit through the single shutdown path so settings flush + plugin
            // shutdown run; a clean quit exits 0.
            let _ = proxy.dispatch(|cx| cx.request_quit());
        } else {
            eprintln!("app_shell smoke: window never became ready; failing");
            std::process::exit(1);
        }
    });
}

/// Fail the process (non-zero) if a `--smoke` run never boots and quits in time.
fn spawn_watchdog() {
    thread::spawn(|| {
        thread::sleep(SMOKE_WATCHDOG);
        eprintln!("app_shell smoke watchdog fired: shell did not boot/quit in time");
        std::process::exit(1);
    });
}
