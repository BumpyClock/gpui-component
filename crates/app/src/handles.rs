//! Thread-affinity-explicit handles and the main-thread shell global (plan D3).
//!
//! Two handle kinds with *distinct, compile-tested* auto-trait contracts:
//!
//! - [`AppInfo`] and [`AppProxy`] are `Clone + Send + Sync` — they cross threads
//!   (tray, watchers, audio callbacks).
//! - [`ShellState`] is a main-thread-only GPUI [`gpui::Global`], reached through
//!   the [`AppShellExt`] extension trait. It is intentionally **not** `Send`.
//!
//! Callbacks always receive a raw `&mut gpui::App`; there is no context wrapper.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use gpui::App;
use gpui_component_manifest::schema::IdentityRef;
use gpui_component_storage::AppPaths;

use crate::capabilities::PlatformCapabilities;
use crate::error::AppClosed;
use crate::lifecycle::{AppEvent, OpenRequest, ShutdownReason};
use crate::liveness::{Liveness, ShellHold};
use crate::phases::PhaseTracker;
use crate::plugin::{AppPlugin, EventHandler};

/// How long the cross-thread drain loop sleeps between polls.
///
/// WART (flagged): the gpui fork exposes no dependency-free way to wake the main
/// thread from a `Send` context (see the report). The proxy therefore *polls*
/// its command channel on the foreground executor. A proper fix is a gpui
/// `spawn_on_main(impl FnOnce(&mut App) + Send)` primitive or an async channel.
const PROXY_POLL_INTERVAL: Duration = Duration::from_millis(16);

/// Immutable application identity, resolved paths, and capability snapshot.
///
/// `Clone + Send + Sync` (compile-asserted below) so background threads can read
/// identity/paths/capabilities without touching the main-thread global.
#[derive(Clone)]
pub struct AppInfo {
    inner: Arc<AppInfoInner>,
}

struct AppInfoInner {
    identity: IdentityRef,
    paths: AppPaths,
    capabilities: PlatformCapabilities,
}

impl AppInfo {
    pub(crate) fn new(
        identity: IdentityRef,
        paths: AppPaths,
        capabilities: PlatformCapabilities,
    ) -> Self {
        Self {
            inner: Arc::new(AppInfoInner {
                identity,
                paths,
                capabilities,
            }),
        }
    }

    /// The stable application id (e.g. `com.example.app`).
    pub fn app_id(&self) -> &str {
        self.inner.identity.app_id
    }

    /// The user-facing display name.
    pub fn display_name(&self) -> &str {
        self.inner.identity.display_name
    }

    /// The canonical SemVer version (`CARGO_PKG_VERSION` of the app).
    pub fn version(&self) -> &str {
        self.inner.identity.version
    }

    /// The compiled-in borrowed identity.
    pub fn identity(&self) -> IdentityRef {
        self.inner.identity
    }

    /// Resolved per-app directories.
    pub fn paths(&self) -> &AppPaths {
        &self.inner.paths
    }

    /// Snapshot of platform capabilities.
    pub fn capabilities(&self) -> &PlatformCapabilities {
        &self.inner.capabilities
    }
}

type ProxyCommand = Box<dyn FnOnce(&mut App) + Send + 'static>;

/// Cross-thread dispatch handle: schedules work onto the main thread.
///
/// `Clone + Send + Sync`. Serves tray/hotkey/watcher/audio callbacks alike.
/// After shutdown begins, [`AppProxy::dispatch`] returns [`AppClosed`].
#[derive(Clone)]
pub struct AppProxy {
    inner: Arc<ProxyInner>,
}

struct ProxyInner {
    /// `None` once closed. The closed state lives *inside* the mutex so the
    /// closed-check and the send are one atomic operation — nothing can be
    /// accepted after `close()` wins the lock.
    sender: Mutex<Option<Sender<ProxyCommand>>>,
}

impl AppProxy {
    /// Create a proxy and its receiver. The receiver is consumed by the
    /// main-thread drain loop; the proxy is cloned to background consumers.
    fn new() -> (Self, Receiver<ProxyCommand>) {
        let (tx, rx) = channel();
        let inner = Arc::new(ProxyInner {
            sender: Mutex::new(Some(tx)),
        });
        (Self { inner }, rx)
    }

    /// Schedule `f` to run on the main thread with `&mut App`.
    ///
    /// Returns [`AppClosed`] once shutdown has begun (the proxy is closed at the
    /// `ShutdownRequested` boundary). The closed-check and send are atomic under
    /// one lock, so no callback is enqueued past that boundary.
    pub fn dispatch(&self, f: impl FnOnce(&mut App) + Send + 'static) -> Result<(), AppClosed> {
        let guard = self.inner.sender.lock().expect("proxy sender poisoned");
        match guard.as_ref() {
            Some(tx) => tx.send(Box::new(f)).map_err(|_| AppClosed),
            None => Err(AppClosed),
        }
    }

    /// Whether the proxy has been closed.
    pub fn is_closed(&self) -> bool {
        self.inner
            .sender
            .lock()
            .expect("proxy sender poisoned")
            .is_none()
    }

    /// Close the proxy: reject all future dispatches and let the drain loop
    /// observe disconnection and exit. Idempotent.
    fn close(&self) {
        *self.inner.sender.lock().expect("proxy sender poisoned") = None;
    }
}

/// Raw platform events captured before services exist. Shared between the early
/// listeners (registered pre-`run`) and the main-thread drain.
#[derive(Default)]
pub(crate) struct PendingEvents {
    pub(crate) queue: Mutex<VecDeque<AppEvent>>,
    /// Set only once the shell is ready; its presence tells post-ready listeners
    /// they may dispatch a drain instead of relying on the startup drain.
    pub(crate) proxy: OnceLock<AppProxy>,
}

impl PendingEvents {
    pub(crate) fn push(&self, event: AppEvent) {
        self.queue
            .lock()
            .expect("pending queue poisoned")
            .push_back(event);
        if let Some(proxy) = self.proxy.get() {
            let _ = proxy.dispatch(drain_pending);
        }
    }
}

/// Main-thread shell state. A GPUI [`gpui::Global`]; intentionally not `Send`.
pub struct ShellState {
    app_info: AppInfo,
    proxy: AppProxy,
    liveness: Liveness,
    plugins: Vec<Box<dyn AppPlugin>>,
    handlers: Vec<EventHandler>,
    phases: PhaseTracker,
    pending: Arc<PendingEvents>,
    state: HashMap<TypeId, Box<dyn Any>>,
    subscriptions: Vec<gpui::Subscription>,
    /// Re-entrancy guard for `deliver_event`.
    delivery: crate::lifecycle::ReentrantQueue,
    /// The reason to attribute if the next idle evaluation triggers an exit.
    /// Set by the window-close observer (which fires before a window-manager
    /// plugin drops the window's hold), consumed by `evaluate_exit`.
    pending_exit_reason: Option<ShutdownReason>,
    shutdown_requested: bool,
    will_exit_done: bool,
    _drain_task: gpui::Task<()>,
}

impl gpui::Global for ShellState {}

impl ShellState {
    /// Number of live plugins (for diagnostics/tests).
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    /// The recorded phase progress.
    pub fn phases(&self) -> &PhaseTracker {
        &self.phases
    }

    pub(crate) fn record_phase(&mut self, phase: crate::phases::Phase) {
        self.phases.complete(phase);
    }

    /// Move plugins out for a `&mut App` re-entrant call (init).
    pub(crate) fn take_plugins(&mut self) -> Vec<Box<dyn AppPlugin>> {
        std::mem::take(&mut self.plugins)
    }

    /// Restore plugins taken by [`ShellState::take_plugins`].
    pub(crate) fn restore_plugins(&mut self, plugins: Vec<Box<dyn AppPlugin>>) {
        self.plugins = plugins;
    }
}

/// A lightweight, cloneable handle for taking liveness leases and driving quit.
#[derive(Clone)]
pub struct ShellHandle {
    proxy: AppProxy,
    holds: Arc<std::sync::atomic::AtomicUsize>,
}

impl ShellHandle {
    /// Acquire a liveness lease. The shell stays alive until it is dropped (and
    /// no windows remain, under [`crate::ExitPolicy::WhenIdle`]).
    pub fn hold(&self, reason: &'static str) -> ShellHold {
        crate::liveness::acquire_hold(Arc::clone(&self.holds), self.proxy.clone(), reason)
    }

    /// The cross-thread proxy.
    pub fn proxy(&self) -> AppProxy {
        self.proxy.clone()
    }
}

/// Extension trait exposing shell services on the raw `gpui::App`.
pub trait AppShellExt {
    /// Immutable app identity/paths/capabilities.
    fn app_info(&self) -> &AppInfo;
    /// A cross-thread dispatch proxy.
    fn app_proxy(&self) -> AppProxy;
    /// A handle for liveness leases and quit.
    fn shell(&self) -> ShellHandle;
    /// Typed pre-platform state registered via `AppShellBuilder::state`.
    fn app_state<T: 'static>(&self) -> Option<&T>;
    /// Route a quit through the single shutdown path.
    fn request_quit(&mut self);
}

impl AppShellExt for App {
    fn app_info(&self) -> &AppInfo {
        &self.global::<ShellState>().app_info
    }

    fn app_proxy(&self) -> AppProxy {
        self.global::<ShellState>().proxy.clone()
    }

    fn shell(&self) -> ShellHandle {
        let st = self.global::<ShellState>();
        ShellHandle {
            proxy: st.proxy.clone(),
            holds: st.liveness.holds_arc(),
        }
    }

    fn app_state<T: 'static>(&self) -> Option<&T> {
        self.global::<ShellState>()
            .state
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref::<T>())
    }

    fn request_quit(&mut self) {
        request_quit(self);
    }
}

/// Install the shell global and start the cross-thread drain loop.
///
/// Called during the `CoreServices` phase with the constructed `AppInfo` and the
/// plugins/handlers/state accumulated by the builder.
#[allow(clippy::too_many_arguments)]
pub(crate) fn install(
    cx: &mut App,
    app_info: AppInfo,
    liveness: Liveness,
    plugins: Vec<Box<dyn AppPlugin>>,
    handlers: Vec<EventHandler>,
    pending: Arc<PendingEvents>,
    state: HashMap<TypeId, Box<dyn Any>>,
    phases: PhaseTracker,
) -> AppProxy {
    let (proxy, rx) = AppProxy::new();
    let drain_task = spawn_drain_loop(cx, proxy.clone(), rx);

    cx.set_global(ShellState {
        app_info,
        proxy: proxy.clone(),
        liveness,
        plugins,
        handlers,
        phases,
        pending,
        state,
        subscriptions: Vec::new(),
        delivery: crate::lifecycle::ReentrantQueue::new(),
        pending_exit_reason: None,
        shutdown_requested: false,
        will_exit_done: false,
        _drain_task: drain_task,
    });
    proxy
}

fn spawn_drain_loop(cx: &App, proxy: AppProxy, rx: Receiver<ProxyCommand>) -> gpui::Task<()> {
    cx.spawn(async move |cx| {
        loop {
            // Drain queued commands, applying each on the main thread. Re-check
            // closed state BETWEEN callbacks: if one triggers shutdown (closing
            // the proxy), discard everything still queued behind it rather than
            // running it mid-teardown.
            loop {
                match rx.try_recv() {
                    Ok(cmd) => {
                        cx.update(|app| cmd(app));
                        if proxy.is_closed() {
                            return;
                        }
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => return,
                }
            }
            if proxy.is_closed() {
                return;
            }
            cx.background_executor().timer(PROXY_POLL_INTERVAL).await;
        }
    })
}

/// Register lifecycle observers (window-closed, app-quit) after readiness.
pub(crate) fn register_observers(cx: &mut App) {
    let window_closed = cx.on_window_closed(|app| {
        if app.windows().is_empty() {
            // Record the reason now. The actual exit may only happen on a later
            // tick, after a window-manager plugin's reconcile drops the closed
            // window's liveness hold — this observer runs before that. Recording
            // it lets the eventual `evaluate_exit` attribute the exit to
            // LastWindowClosed instead of the generic Requested, and avoids a
            // premature exit here while the hold is still counted.
            app.global_mut::<ShellState>().pending_exit_reason =
                Some(ShutdownReason::LastWindowClosed);
            deliver_event(app, &AppEvent::LastWindowClosed);
        }
        evaluate_exit(app);
    });
    let app_quit = cx.on_app_quit(|app| {
        run_will_exit(app);
        std::future::ready(())
    });
    let st = cx.global_mut::<ShellState>();
    st.subscriptions.push(window_closed);
    st.subscriptions.push(app_quit);
}

/// Deliver `event` to every plugin then every app event handler.
///
/// Re-entrancy-safe: a delivery moves plugins/handlers out of the global for the
/// pass, so a callback that itself delivers an event (e.g. `request_quit()` from
/// a `Started` handler emitting `ShutdownRequested`) would otherwise hit empty
/// subscriber lists. Such nested events are buffered and drained after the
/// current pass, in order (see [`crate::lifecycle::ReentrantQueue`]).
pub(crate) fn deliver_event(cx: &mut App, event: &AppEvent) {
    if !cx.has_global::<ShellState>() {
        return;
    }
    if !cx.global_mut::<ShellState>().delivery.try_enter(event) {
        // A delivery is already in progress; this event is now buffered and will
        // be drained by the active pass below.
        return;
    }
    let mut current = event.clone();
    loop {
        deliver_one(cx, &current);
        match cx.global_mut::<ShellState>().delivery.take_next() {
            Some(next) => current = next,
            None => break,
        }
    }
}

/// Deliver a single event to plugins then handlers, moving them out of the
/// global for the call (so they can receive `&mut App`) and restoring them.
/// Handler errors are logged, not fatal.
fn deliver_one(cx: &mut App, event: &AppEvent) {
    let (mut plugins, mut handlers) = {
        let st = cx.global_mut::<ShellState>();
        (
            std::mem::take(&mut st.plugins),
            std::mem::take(&mut st.handlers),
        )
    };

    for plugin in &mut plugins {
        if let Err(err) = plugin.on_event(event, cx) {
            log::error!("plugin failed handling {event:?}: {err}");
        }
    }
    for handler in &mut handlers {
        if let Err(err) = handler(event, cx) {
            log::error!("event handler failed handling {event:?}: {err}");
        }
    }

    let st = cx.global_mut::<ShellState>();
    st.plugins = plugins;
    st.handlers = handlers;
}

/// Drain events buffered by early platform listeners and deliver them.
pub(crate) fn drain_pending(cx: &mut App) {
    if !cx.has_global::<ShellState>() {
        return;
    }
    let pending = cx.global::<ShellState>().pending.clone();
    let events: Vec<AppEvent> = pending
        .queue
        .lock()
        .expect("pending queue poisoned")
        .drain(..)
        .collect();
    for event in &events {
        deliver_event(cx, event);
    }
}

/// Re-evaluate the exit policy; quit through the single path if idle.
///
/// If an exit is triggered, it is attributed to any `pending_exit_reason`
/// recorded by the window-close observer (consumed here), else
/// [`ShutdownReason::Requested`]. This makes attribution robust to the ordering
/// between this observer and a window-manager plugin's hold-drop: the reason is
/// recorded on close and consumed by whichever evaluation actually exits.
pub(crate) fn evaluate_exit(cx: &mut App) {
    if !cx.has_global::<ShellState>() {
        return;
    }
    let (policy, holds) = {
        let st = cx.global::<ShellState>();
        if st.shutdown_requested {
            return;
        }
        (st.liveness.exit_policy(), st.liveness.hold_count())
    };
    let windows = cx.windows().len();
    if crate::liveness::should_exit(policy, holds, windows) {
        let reason = cx
            .global_mut::<ShellState>()
            .pending_exit_reason
            .take()
            .unwrap_or(ShutdownReason::Requested);
        request_quit_with(cx, reason);
    } else if windows > 0 {
        // The app is alive because a window exists; clear any stale window-close
        // reason so a later idle exit (e.g. via hold-drop) is not misattributed.
        cx.global_mut::<ShellState>().pending_exit_reason = None;
    }
}

/// The single public quit path. Idempotent. Attributes
/// [`ShutdownReason::Requested`].
pub(crate) fn request_quit(cx: &mut App) {
    request_quit_with(cx, ShutdownReason::Requested);
}

/// The single quit path with an explicit reason. Idempotent.
pub(crate) fn request_quit_with(cx: &mut App, reason: ShutdownReason) {
    if !cx.has_global::<ShellState>() {
        cx.quit();
        return;
    }
    {
        let st = cx.global_mut::<ShellState>();
        if st.shutdown_requested {
            return;
        }
        st.shutdown_requested = true;
    }
    // Stop accepting cross-thread work at the shutdown boundary, before any
    // teardown runs — background producers must not enqueue callbacks that would
    // land mid-shutdown.
    cx.global::<ShellState>().proxy.close();
    deliver_event(cx, &AppEvent::ShutdownRequested(reason));
    // Platform quit fires `on_app_quit`, which runs `run_will_exit`.
    cx.quit();
}

/// Final teardown, delivered for every quit cause via `on_app_quit`.
fn run_will_exit(cx: &mut App) {
    if !cx.has_global::<ShellState>() {
        return;
    }
    {
        let st = cx.global_mut::<ShellState>();
        if st.will_exit_done {
            return;
        }
        st.will_exit_done = true;
    }
    // Close the proxy before any teardown so nothing is accepted past the
    // boundary. Idempotent: on the `request_quit` path it is already closed; on a
    // platform-initiated quit (Cmd+Q / OS logout) this is the earliest hook.
    cx.global::<ShellState>().proxy.close();
    // A platform-initiated quit never went through `request_quit`; surface a
    // uniform `ShutdownRequested` first.
    let already_requested = cx.global::<ShellState>().shutdown_requested;
    if !already_requested {
        cx.global_mut::<ShellState>().shutdown_requested = true;
        deliver_event(
            cx,
            &AppEvent::ShutdownRequested(ShutdownReason::PlatformQuit),
        );
    }
    deliver_event(cx, &AppEvent::WillExit);
    shutdown_plugins(cx);
}

/// Shut plugins down in reverse init order.
fn shutdown_plugins(cx: &mut App) {
    let mut plugins = std::mem::take(&mut cx.global_mut::<ShellState>().plugins);
    for plugin in plugins.iter_mut().rev() {
        plugin.shutdown(cx);
    }
    // Plugins are done; they are not restored.
}

/// Convenience for early listeners that need to synthesize an `OpenRequest`.
pub(crate) fn open_event(urls: Vec<String>) -> AppEvent {
    AppEvent::OpenRequested(OpenRequest { urls })
}

// ---------------------------------------------------------------------------
// Auto-trait contract assertions (plan §3, gate 3). Hand-rolled, no new deps.
// ---------------------------------------------------------------------------

const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    fn assert_clone<T: Clone>() {}
    assert_send_sync::<AppInfo>();
    assert_send_sync::<AppProxy>();
    assert_send_sync::<ShellHandle>();
    assert_clone::<AppInfo>();
    assert_clone::<AppProxy>();
    assert_clone::<ShellHandle>();
    assert_send_sync::<PendingEvents>();
};

// Assert `ShellState` is NOT `Send` (it is a main-thread global). The two
// blanket impls below are ambiguous for any `Send` type, so the inference-driven
// reference resolves only when `ShellState: !Send`.
#[allow(dead_code)]
trait AmbiguousIfSend<A> {
    fn deconflict() {}
}
impl<T: ?Sized> AmbiguousIfSend<()> for T {}
impl<T: ?Sized + Send> AmbiguousIfSend<u8> for T {}

const _: fn() = || {
    let _ = <ShellState as AmbiguousIfSend<_>>::deconflict;
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_enqueues_until_closed() {
        let (proxy, rx) = AppProxy::new();
        assert!(!proxy.is_closed());
        proxy.dispatch(|_app| {}).expect("dispatch before close");
        assert!(rx.try_recv().is_ok(), "command should be queued");

        proxy.close();
        assert!(proxy.is_closed());
        assert_eq!(proxy.dispatch(|_app| {}), Err(AppClosed));
    }

    #[test]
    fn dispatch_fails_when_receiver_dropped() {
        let (proxy, rx) = AppProxy::new();
        drop(rx);
        // Sender still present but disconnected → AppClosed.
        assert_eq!(proxy.dispatch(|_app| {}), Err(AppClosed));
    }
}
