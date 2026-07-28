//! Headless bootstrap test: drive the shell through the injected headless runner
//! on the process main thread and assert the lifecycle fires end-to-end.
//!
//! Uses `harness = false` (see this crate's `Cargo.toml`): GPUI panics unless
//! its `App` is constructed on the main thread, which the default test harness
//! (worker threads) cannot provide.
//!
//! FORK REALITY: the headless platform's post-`quit` behavior is not uniform
//! across platforms — macOS terminates the process via `NSApp terminate`, while
//! on Linux/Windows the run loop neither terminates nor returns to `main`. So all
//! verification happens *inside* the `Started` observer, and the test terminates
//! with an explicit `std::process::exit(0)` at the success point rather than
//! depending on those loop-return semantics. A watchdog thread bounds the run so
//! a boot that never reaches readiness fails nonzero instead of hanging CI.
//! Shutdown ordering is covered by the pure unit tests.

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
    // Watchdog: if the shell never reaches readiness, fail instead of hanging.
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(30));
        eprintln!("headless test watchdog fired: shell did not boot/quit in time");
        std::process::exit(1);
    });

    let start_completed = Arc::new(AtomicBool::new(false));
    let started_observer = Arc::clone(&start_completed);
    let started_observed = Arc::new(AtomicBool::new(false));
    let reporter_observed = Arc::clone(&started_observed);
    let result = AppShell::builder(test_identity())
        .runner(PlatformRunner::headless())
        // Tray-first shape: no window, passive activation, explicit exit.
        .initial_activation(InitialActivation::Passive)
        .exit_policy(ExitPolicy::Explicit)
        .start(move |launch, cx: &mut App| {
            // The shell global is installed and AppInfo is reachable via the
            // extension trait with a raw &mut App.
            let info = cx.app_info();
            assert_eq!(info.app_id(), "com.example.appshelltest");
            assert_eq!(info.version(), "0.0.0");
            assert_eq!(info.paths().namespace(), "appshelltest");
            assert!(!info.capabilities().credential_store.is_supported());
            assert!(launch.cwd.is_some());

            // A liveness lease can be taken and released.
            let hold = cx.shell().hold("test");
            assert_eq!(hold.reason(), "test");
            drop(hold);

            start_completed.store(true, Ordering::SeqCst);
            Ok(())
        })
        .on_event(|event, _cx| {
            if matches!(event, AppEvent::Started(_)) {
                return Err(anyhow::anyhow!("expected nonfatal lifecycle error").into());
            }
            Ok(())
        })
        .on_event(move |event, _cx| {
            if !matches!(event, AppEvent::Started(_)) {
                return Ok(());
            }
            assert!(
                started_observer.load(Ordering::SeqCst),
                "start must complete before Started is delivered"
            );
            started_observed.store(true, Ordering::SeqCst);
            Ok(())
        })
        .on_error(move |error, _cx| {
            assert_eq!(
                error.operation(),
                gpui_component_app::error::RuntimeOperation::Lifecycle
            );
            assert_eq!(error.event(), Some("started"));
            assert!(error.continued());
            assert!(
                reporter_observed.load(Ordering::SeqCst),
                "one handler error must not prevent later handlers"
            );
            println!("headless shell lifecycle: ok");

            // Every lifecycle assertion has passed — success is fully known here.
            // Terminate deterministically on every platform instead of driving a
            // quit and relying on loop-return semantics: `request_quit()` ends the
            // process only on macOS (via `NSApp terminate`); on Linux/Windows the
            // headless run loop neither terminates the process nor returns to
            // `main` after `cx.quit()`, so a quit-driven exit hangs until the
            // watchdog fires. Quit/teardown ordering is covered by the pure unit
            // tests; this test asserts `start` completed with a working shell
            // global before the lifecycle reached `Started`.
            std::process::exit(0);
        })
        .run();

    // The Started observer exits the process on success, so `run` only returns
    // here if the shell failed to boot far enough to deliver it.
    panic!("headless shell did not reach Started: run returned {result:?}");
}
