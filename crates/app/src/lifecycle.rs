//! Application lifecycle as an event stream (plan §3).
//!
//! Lifecycle is modelled as a stream of [`AppEvent`]s, not a single launch
//! callback. Raw platform listeners are registered *immediately* after platform
//! construction (before services exist); events that arrive before the shell is
//! ready are queued by [`EventQueue`] and drained after plugin init, in FIFO
//! order (the Electron/GApplication/Zed pattern). This reserves single-instance
//! and deep-link delivery without a future breaking change.
//!
//! The queue logic here is pure (no gpui) and unit-tested directly.

use std::collections::VecDeque;
use std::path::PathBuf;

/// A lifecycle event delivered to plugins and app event handlers.
///
/// `#[non_exhaustive]`: new seams (e.g. `SecondInstance`) may be added without
/// a breaking change.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum AppEvent {
    /// The application finished launching. Carries the initial launch request.
    Started(LaunchRequest),
    /// The application was activated (brought to foreground).
    Activated,
    /// The application was reopened (e.g. dock icon clicked with no windows).
    Reopened,
    /// The platform asked the app to open URLs or files. May arrive *before*
    /// [`AppEvent::Started`]; such events are queued and delivered after it.
    OpenRequested(OpenRequest),
    /// The last application window closed. Liveness policy decides whether this
    /// leads to exit.
    LastWindowClosed,
    /// The system is about to suspend. Reserved seam; may be unsupported.
    Suspend,
    /// The system resumed from suspend. Reserved seam; may be unsupported.
    Resume,
    /// A quit was requested through any path (menu, programmatic, tray, last
    /// window). Delivered once, before shutdown proceeds.
    ShutdownRequested(ShutdownReason),
    /// The process is about to exit; last chance for a bounded flush.
    WillExit,
}

/// The context in which the application was launched.
#[derive(Debug, Clone, Default)]
pub struct LaunchRequest {
    /// Process arguments (excluding argv\[0\]).
    pub args: Vec<String>,
    /// Working directory at launch, if resolvable.
    pub cwd: Option<PathBuf>,
    /// URLs/files supplied at launch (e.g. `open`-with, deep link).
    pub urls: Vec<String>,
}

impl LaunchRequest {
    /// Build a launch request from the current process environment.
    pub fn from_env() -> Self {
        Self {
            args: std::env::args().skip(1).collect(),
            cwd: std::env::current_dir().ok(),
            urls: Vec::new(),
        }
    }
}

/// A request to open one or more URLs or files.
#[derive(Debug, Clone, Default)]
pub struct OpenRequest {
    /// The URLs or file paths to open.
    pub urls: Vec<String>,
}

/// Why the application is shutting down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShutdownReason {
    /// The last window closed and liveness policy chose to exit.
    LastWindowClosed,
    /// Shutdown was requested programmatically via `request_quit`.
    Requested,
    /// The platform initiated quit (OS logout, `Cmd+Q`, tray "Quit").
    PlatformQuit,
}

/// Buffers lifecycle events until the shell is ready, then delivers FIFO.
///
/// Before [`EventQueue::mark_ready`], [`EventQueue::push`] stores the event and
/// returns `false` (queued). After readiness, `push` returns `true` (the caller
/// should deliver immediately). [`EventQueue::drain`] empties the buffer in
/// arrival order.
#[derive(Debug, Default)]
pub struct EventQueue {
    ready: bool,
    pending: VecDeque<AppEvent>,
}

impl EventQueue {
    /// A new, not-yet-ready queue.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the shell has been marked ready.
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Record an event.
    ///
    /// Returns `true` if the shell is already ready and the caller should
    /// deliver `event` now; returns `false` if it was buffered for later drain.
    #[must_use]
    pub fn push(&mut self, event: AppEvent) -> bool {
        if self.ready {
            true
        } else {
            self.pending.push_back(event);
            false
        }
    }

    /// Mark the shell ready. Does not itself drain — call [`EventQueue::drain`].
    pub fn mark_ready(&mut self) {
        self.ready = true;
    }

    /// Remove and return all buffered events in arrival order.
    pub fn drain(&mut self) -> Vec<AppEvent> {
        self.pending.drain(..).collect()
    }

    /// Number of buffered events.
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

/// Guards event delivery against re-entrancy.
///
/// `deliver_event` moves plugins/handlers out of the shell global for the whole
/// pass, so a callback that itself triggers a delivery (e.g. `request_quit()`
/// from a `Started` handler) would otherwise deliver to empty subscriber lists.
/// While a pass is active, a nested event is buffered here and drained after the
/// current pass, in arrival order. Pure logic, unit-tested directly.
#[derive(Debug, Default)]
pub(crate) struct ReentrantQueue {
    delivering: bool,
    deferred: VecDeque<AppEvent>,
}

impl ReentrantQueue {
    /// A new, idle queue.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Begin a delivery pass for `event`.
    ///
    /// Returns `true` if the caller should deliver `event` now (no pass was
    /// active). Returns `false` if a pass is already active — `event` is
    /// buffered and the caller must return without delivering.
    #[must_use]
    pub(crate) fn try_enter(&mut self, event: &AppEvent) -> bool {
        if self.delivering {
            self.deferred.push_back(event.clone());
            false
        } else {
            self.delivering = true;
            true
        }
    }

    /// After delivering one event, return the next buffered event, or `None` to
    /// end the pass (clearing the in-progress flag).
    pub(crate) fn take_next(&mut self) -> Option<AppEvent> {
        match self.deferred.pop_front() {
            Some(event) => Some(event),
            None => {
                self.delivering = false;
                None
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn is_delivering(&self) -> bool {
        self.delivering
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open(url: &str) -> AppEvent {
        AppEvent::OpenRequested(OpenRequest {
            urls: vec![url.to_string()],
        })
    }

    #[test]
    fn buffers_until_ready_then_drains_fifo() {
        let mut q = EventQueue::new();
        assert!(!q.push(open("a")));
        assert!(!q.push(open("b")));
        assert_eq!(q.len(), 2);
        assert!(!q.is_ready());

        q.mark_ready();
        let drained = q.drain();
        assert_eq!(drained.len(), 2);
        match (&drained[0], &drained[1]) {
            (AppEvent::OpenRequested(a), AppEvent::OpenRequested(b)) => {
                assert_eq!(a.urls, vec!["a".to_string()]);
                assert_eq!(b.urls, vec!["b".to_string()]);
            }
            _ => panic!("unexpected drained events"),
        }
        assert!(q.is_empty());
    }

    #[test]
    fn push_after_ready_signals_immediate_delivery() {
        let mut q = EventQueue::new();
        q.mark_ready();
        assert!(
            q.push(open("late")),
            "post-ready push must ask for delivery"
        );
        assert!(q.is_empty(), "post-ready push must not buffer");
    }

    #[test]
    fn reentrant_queue_defers_nested_events_and_preserves_order() {
        let mut q = ReentrantQueue::new();
        // Outer pass begins and delivers now.
        assert!(q.try_enter(&AppEvent::Started(LaunchRequest::default())));
        assert!(q.is_delivering());
        // Events raised during the pass are buffered, not delivered.
        assert!(!q.try_enter(&AppEvent::Reopened));
        assert!(!q.try_enter(&AppEvent::Activated));
        // Drained in arrival order after the pass.
        assert!(matches!(q.take_next(), Some(AppEvent::Reopened)));
        assert!(matches!(q.take_next(), Some(AppEvent::Activated)));
        assert!(q.take_next().is_none());
        assert!(!q.is_delivering());
        // A fresh pass can start again.
        assert!(q.try_enter(&AppEvent::Activated));
    }

    #[test]
    fn early_reopen_is_buffered_until_ready() {
        // A reopen that arrives during startup (before the shell global exists)
        // must be buffered and delivered after `Started`, not dropped.
        let mut q = EventQueue::new();
        assert!(!q.push(AppEvent::Reopened));
        q.mark_ready();
        let drained = q.drain();
        assert!(matches!(drained.as_slice(), [AppEvent::Reopened]));
    }
}
