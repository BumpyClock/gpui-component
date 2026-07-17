//! The sealed internal plugin mechanism (plan §3, §7 guardrail).
//!
//! Builder methods install internal plugins; the shell drives them through the
//! phase sequencer. The trait is **sealed** until all three real apps have
//! exercised it (Phase 2) — external crates cannot implement it yet, which keeps
//! the surface honest while the contract settles. In-crate services (window
//! manager, settings, theme) implement it via [`sealed::Sealed`].

use gpui::App;

use crate::error::AppShellError;
use crate::handles::{AppInfo, AppProxy};
use crate::lifecycle::AppEvent;

/// Boxed application event handler (the `on_event` sugar and `on_launch`).
pub(crate) type EventHandler = Box<dyn FnMut(&AppEvent, &mut App) -> Result<(), AppShellError>>;

pub(crate) mod sealed {
    /// Seals [`super::AppPlugin`]. Implemented only for in-crate plugin types.
    pub trait Sealed {}
}

/// Build-time context handed to a plugin's [`AppPlugin::configure`], before the
/// GPUI app exists. Lets a plugin read identity/capabilities and contribute to
/// startup without touching `&mut App`.
#[non_exhaustive]
pub struct BuildContext<'a> {
    /// The resolved application info (identity, paths, capabilities).
    pub info: &'a AppInfo,
}

/// The minimal shell seed handed to [`AppPlugin::init`]: the parts of the shell
/// that exist before the global is fully installed.
#[non_exhaustive]
pub struct ShellSeed {
    /// Immutable app info.
    pub info: AppInfo,
    /// Cross-thread dispatch proxy.
    pub proxy: AppProxy,
}

/// An internal shell plugin. Sealed; see the module docs.
///
/// Lifecycle: [`configure`](AppPlugin::configure) (pre-GPUI) →
/// [`init`](AppPlugin::init) (services up) → [`on_event`](AppPlugin::on_event)
/// (per lifecycle event) → [`shutdown`](AppPlugin::shutdown) (reverse order).
pub trait AppPlugin: sealed::Sealed + 'static {
    /// Pre-platform configuration. Default: no-op.
    fn configure(&mut self, _cx: &mut BuildContext<'_>) {}

    /// Initialize the plugin once core services are up. Required.
    fn init(&mut self, cx: &mut App, shell: &ShellSeed) -> Result<(), AppShellError>;

    /// Handle a lifecycle event. Default: ignore.
    fn on_event(&mut self, _event: &AppEvent, _cx: &mut App) -> Result<(), AppShellError> {
        Ok(())
    }

    /// Tear down. Default: no-op. Called in reverse init order.
    fn shutdown(&mut self, _cx: &mut App) {}
}
