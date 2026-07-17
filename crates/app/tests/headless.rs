//! Headless bootstrap test: drive the shell through the injected headless runner
//! on the process main thread and assert the lifecycle fires end-to-end.
//!
//! Uses `harness = false` (see this crate's `Cargo.toml`): GPUI panics unless
//! its `App` is constructed on the main thread, which the default test harness
//! (worker threads) cannot provide.
//!
//! FORK REALITY: on macOS the headless platform runs `on_finish_launching()`
//! then `CFRunLoopRun()`, and `quit()` terminates via `NSApp terminate` (which
//! `exit()`s). `Application::run` therefore never returns on macOS, so all
//! verification happens *inside* the launch closure, before the shell quits.
//! A watchdog thread bounds the run so a boot that never reaches `on_launch`
//! fails as a non-zero exit instead of hanging CI. Shutdown ordering is covered
//! by the pure unit tests.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use gpui_component_app::gpui::App;
use gpui_component_app::prelude::*;
use gpui_component_manifest::schema::IdentityRef;

fn test_identity() -> IdentityRef {
    IdentityRef {
        app_id: "com.example.appshelltest",
        display_name: "App Shell Test",
        data_namespace: "appshelltest",
        binary_name: None,
        org: None,
        publisher: None,
        url_schemes: &[],
        categories: &[],
        macos: None,
        linux: None,
        windows: None,
        legacy_ids: &[],
        min_os: None,
        version: "0.0.0",
        cfbundle_short_version: "0.0.0",
        msix_version: "0.0.0.0",
    }
}

fn main() {
    // Watchdog: if the shell never reaches `on_launch` (and thus never quits),
    // fail loudly instead of hanging.
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(30));
        eprintln!("headless test watchdog fired: shell did not boot/quit in time");
        std::process::exit(1);
    });

    let launched = Arc::new(AtomicBool::new(false));
    let launched_cb = Arc::clone(&launched);

    let result = AppShell::builder(test_identity())
        .runner(PlatformRunner::headless())
        // Tray-first shape: no window, passive activation, explicit exit.
        .initial_activation(InitialActivation::Passive)
        .exit_policy(ExitPolicy::Explicit)
        .on_launch(move |cx: &mut App| {
            launched_cb.store(true, Ordering::SeqCst);

            // The shell global is installed and AppInfo is reachable via the
            // extension trait with a raw &mut App.
            let info = cx.app_info();
            assert_eq!(info.app_id(), "com.example.appshelltest");
            assert_eq!(info.version(), "0.0.0");
            assert_eq!(info.paths().namespace(), "appshelltest");
            assert!(!info.capabilities().credential_store.is_supported());

            // A liveness lease can be taken and released.
            let hold = cx.shell().hold("test");
            assert_eq!(hold.reason(), "test");
            drop(hold);

            println!("headless shell lifecycle: ok");

            // Zero-window shell: quit through the single path to end the loop.
            // On macOS this terminates the process, so nothing after `run`
            // executes.
            cx.request_quit();
            Ok(())
        })
        .run();

    // Reached only on platforms whose headless `run` returns (not macOS).
    assert!(result.is_ok(), "run returned error: {result:?}");
    assert!(launched.load(Ordering::SeqCst), "on_launch never ran");
}
