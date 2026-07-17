//! `app_shell_tray` — tray-first AppShell conformance example.
//!
//! A tray/menu-bar app launches with **no window** and must not exit just because
//! zero windows are open. This example simulates that shape without pulling in a
//! real tray crate:
//!
//! - [`InitialActivation::Passive`] — launch without stealing focus, zero windows.
//! - A fake "tray" service takes a [`ShellHold`] liveness lease so the app stays
//!   alive at zero windows ([`ExitPolicy::WhenIdle`] would otherwise exit).
//! - A timer stands in for a tray-icon click: it opens a singleton window
//!   (`open_singleton`), then releases the tray hold so the window's own lease
//!   keeps the app alive. Closing that window then drops to zero holds / zero
//!   windows and the shell exits — demonstrating the hold/release exit contract.
//!
//! A REAL tray app would instead keep the tray hold for the whole session (so
//! closing the last window returns to the tray rather than quitting) and open the
//! window from the status-item callback rather than a timer.
//!
//! Run it: `cargo run -p app_shell_tray`. The `--smoke` flag requests a clean
//! quit after a few seconds with exit code 0, for the CI `native-launch-smoke`
//! job.
//!
//! Smoke contract: **exit 0 iff the window opened and the shell quit cleanly.**
//! A failed `open_singleton` fails the run hard (see [`open_main_window`]) rather
//! than dropping the tray hold and letting the shell idle-exit 0 with no window.

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

gpui_component_app::include_identity!();

/// How long the app stays window-less before the fake tray opens its window.
/// Kept below [`SMOKE_LIFETIME`] so a `--smoke` run still exercises the open path.
const TRAY_OPEN_DELAY: Duration = Duration::from_secs(1);
/// How long a `--smoke` run stays up before requesting a clean quit.
const SMOKE_LIFETIME: Duration = Duration::from_secs(3);
/// Hard upper bound: fail loudly instead of hanging CI if the app never quits.
const SMOKE_WATCHDOG: Duration = Duration::from_secs(30);

/// Set once the tray's window has actually opened. The `--smoke` quit path asserts
/// it: a run that never opened its window exits non-zero (see the module docs).
static WINDOW_READY: AtomicBool = AtomicBool::new(false);

/// The window the fake tray opens; themed so the theme service is visibly live.
struct TrayWindow;

impl Render for TrayWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child("Opened from the (simulated) tray")
    }
}

fn main() {
    let smoke = std::env::args().any(|arg| arg == "--smoke");
    if smoke {
        spawn_watchdog();
    }

    let result = AppShell::builder(APP_IDENTITY)
        .initial_activation(InitialActivation::Passive)
        .exit_policy(ExitPolicy::WhenIdle)
        .theme(ThemeSource::registry())
        .menus(MenuPlan::standard().with_theme_menu())
        .on_launch(move |cx| {
            // Arm the smoke quit/verify thread first: a run that never opens its
            // window must fail non-zero, not idle-exit 0.
            if smoke {
                schedule_smoke_quit(cx);
            }

            // The "tray" takes a liveness lease: with zero windows and this hold
            // outstanding, `ExitPolicy::WhenIdle` keeps the app alive.
            let hold = cx.shell().hold("tray");
            let proxy = cx.app_proxy();

            // Stand in for a tray-icon click after a short delay.
            thread::spawn(move || {
                thread::sleep(TRAY_OPEN_DELAY);
                let _ = proxy.dispatch(move |cx| open_main_window(cx, hold));
            });

            Ok(())
        })
        .run();

    // FORK REALITY: on macOS the platform terminates the process inside `run()`,
    // so this is reached only where the event loop returns (Linux/Windows).
    if let Err(err) = result {
        eprintln!("app_shell_tray exited with error: {err}");
        std::process::exit(1);
    }
}

/// Open the singleton main window, consuming the tray `hold`.
///
/// On success the window's own liveness lease takes over, so the tray hold is
/// released. On failure the smoke contract requires a hard non-zero exit rather
/// than dropping the hold and letting the shell idle-exit 0 with no window.
fn open_main_window(cx: &mut App, hold: ShellHold) {
    match WindowManager::open_singleton(
        cx,
        WindowSpec::new("main").title("App Shell Tray Example"),
        |_, cx| cx.new(|_| TrayWindow),
    ) {
        Ok(_) => {
            WINDOW_READY.store(true, Ordering::SeqCst);
            // The window now keeps the app alive; a real tray app keeps this
            // hold for the whole session instead.
            drop(hold);
        }
        Err(err) => {
            eprintln!("app_shell_tray failed to open window: {err}");
            std::process::exit(1);
        }
    }
}

/// After [`SMOKE_LIFETIME`], either quit cleanly (window opened) or fail the run
/// (window never became ready) — the smoke contract's decision point.
fn schedule_smoke_quit(cx: &mut App) {
    let proxy = cx.app_proxy();
    thread::spawn(move || {
        thread::sleep(SMOKE_LIFETIME);
        if WINDOW_READY.load(Ordering::SeqCst) {
            // Quit through the single shutdown path so plugin shutdown runs
            // regardless of the tray hold; a clean quit exits 0.
            let _ = proxy.dispatch(|cx| cx.request_quit());
        } else {
            eprintln!("app_shell_tray smoke: window never became ready; failing");
            std::process::exit(1);
        }
    });
}

/// Fail the process (non-zero) if a `--smoke` run never boots and quits in time.
fn spawn_watchdog() {
    thread::spawn(|| {
        thread::sleep(SMOKE_WATCHDOG);
        eprintln!("app_shell_tray smoke watchdog fired: shell did not boot/quit in time");
        std::process::exit(1);
    });
}
