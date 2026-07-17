//! Stable public error type for the application shell.
//!
//! Library callbacks and shell APIs return [`AppShellError`] rather than
//! `anyhow::Error` so downstream apps can match on failure modes across
//! releases (plan §3, "public API errors are a stable `AppShellError`").
//! `anyhow` is still accepted *into* the shell (via `From`) so application
//! callbacks may use `?` freely.

use thiserror::Error;

/// Errors surfaced by the application shell.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AppShellError {
    /// The compiled-in [`crate::AppIdentity`] failed validation.
    #[error("invalid app identity: {0}")]
    Identity(String),

    /// Per-app directories could not be resolved from the identity namespace.
    #[error("failed to resolve application paths")]
    Paths(#[source] gpui_component_storage::StorageError),

    /// A required startup service failed and startup cannot continue.
    ///
    /// Degradable services (theme watcher, file logging) must not use this;
    /// they log and continue. Only services declared *required* abort startup.
    #[error("required startup service `{service}` failed")]
    Service {
        /// Stable identifier of the failing service.
        service: &'static str,
        /// Underlying cause.
        #[source]
        source: anyhow::Error,
    },

    /// A cross-thread dispatch was attempted after shutdown began.
    #[error(transparent)]
    Closed(#[from] AppClosed),

    /// An application-supplied callback returned an error.
    ///
    /// This is the `?`-ergonomic bridge: `anyhow::Error` from a user closure
    /// converts here automatically.
    #[error("application callback failed")]
    Callback(#[source] anyhow::Error),
}

impl From<anyhow::Error> for AppShellError {
    fn from(source: anyhow::Error) -> Self {
        AppShellError::Callback(source)
    }
}

impl From<gpui_component_storage::StorageError> for AppShellError {
    fn from(source: gpui_component_storage::StorageError) -> Self {
        AppShellError::Paths(source)
    }
}

/// Returned by [`crate::AppProxy::dispatch`] once the shell has begun shutting
/// down and can no longer accept main-thread work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("the application has shut down and can no longer accept dispatched work")]
pub struct AppClosed;
