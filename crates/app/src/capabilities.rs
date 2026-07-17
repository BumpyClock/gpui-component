//! Runtime platform-capability reporting (plan D9, task P1.8).
//!
//! The gpui fork has several *silent stubs* — URL-scheme registration is
//! unimplemented on Windows and Linux, and X11 overlay click-through is a no-op
//! (plan §1.5, §7). Rather than wrap those as if they worked, the shell reports
//! honest capabilities and fallible APIs return [`Capability::Unsupported`] with
//! a reason. Capabilities are *runtime* values, not `cfg!` constants, so that
//! session-dependent facts (Wayland vs X11, tray availability) can be refined
//! here later without an API change.

/// Whether a given platform capability is available in this process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    /// The capability is available and expected to work.
    Supported,
    /// The capability is not available; `reason` explains why (missing fork
    /// implementation, wrong session type, disabled feature, …).
    Unsupported {
        /// Human-readable, stable-ish explanation for logs and diagnostics.
        reason: &'static str,
    },
}

impl Capability {
    /// Convenience: an `Unsupported` value with a static reason.
    pub const fn unsupported(reason: &'static str) -> Self {
        Capability::Unsupported { reason }
    }

    /// Whether the capability is [`Capability::Supported`].
    pub const fn is_supported(&self) -> bool {
        matches!(self, Capability::Supported)
    }
}

/// A snapshot of platform capabilities relevant to app-shell features.
///
/// Cloned into [`crate::AppInfo`] so background threads can inspect it before
/// committing to a shape (e.g. a tray-first app checking `tray` before choosing
/// a zero-window launch).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PlatformCapabilities {
    /// Click-through overlay surfaces (`open_overlay_surface`).
    pub overlay_surface: Capability,
    /// System tray / status-item icon.
    pub tray: Capability,
    /// Right-click dock menu (macOS) / jump list (Windows).
    pub dock_menu: Capability,
    /// OS credential store (keychain / libsecret / credential manager).
    pub credential_store: Capability,
    /// Registering the app as a handler for custom URL schemes.
    pub url_schemes: Capability,
    /// Restoring a window to an exact previous position.
    pub precise_window_positioning: Capability,
}

impl PlatformCapabilities {
    /// Detect capabilities for the current build target.
    ///
    /// Judgements (each traceable to a plan finding or verified fork stub):
    ///
    /// - `url_schemes`: registration is implemented only on macOS. Windows
    ///   (`gpui_windows/src/platform.rs`) and Linux (`gpui_linux/src/platform.rs`)
    ///   are verified stubs, so both report `Unsupported` (plan §1.5, D9).
    /// - `overlay_surface`: macOS only for now. X11 click-through is a silent
    ///   no-op and Wayland is unverified, so non-macOS reports `Unsupported`
    ///   until end-to-end verification (plan §7).
    /// - `tray`: macOS and Windows report `Supported`; Linux is `Unsupported`
    ///   because tray reliability depends on the desktop session
    ///   (Wayland/GNOME) and needs a runtime, not compile-time, check (plan §7).
    /// - `dock_menu`: macOS dock menu / Windows jump list only.
    /// - `credential_store`: not yet wired through the shell on any platform; the
    ///   credential-store abstraction is deferred (plan §3, Phase 3).
    /// - `precise_window_positioning`: `Unsupported` on Linux because Wayland
    ///   does not expose absolute window positions.
    pub fn detect() -> Self {
        Self {
            overlay_surface: overlay_surface(),
            tray: tray(),
            dock_menu: dock_menu(),
            credential_store: Capability::unsupported(
                "credential storage is not yet wired through the shell (deferred)",
            ),
            url_schemes: url_schemes(),
            precise_window_positioning: precise_window_positioning(),
        }
    }
}

const fn overlay_surface() -> Capability {
    #[cfg(target_os = "macos")]
    {
        Capability::Supported
    }
    #[cfg(not(target_os = "macos"))]
    {
        Capability::unsupported(
            "overlay click-through is unverified off macOS (X11 no-op, Wayland untested)",
        )
    }
}

const fn tray() -> Capability {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        Capability::Supported
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Capability::unsupported(
            "tray reliability depends on the Linux desktop session; verify at runtime",
        )
    }
}

const fn dock_menu() -> Capability {
    #[cfg(target_os = "macos")]
    {
        Capability::Supported
    }
    #[cfg(target_os = "windows")]
    {
        // The OS has jump lists, but the shell's dock projection
        // (`set_dock_menu`) is only implemented on macOS today.
        Capability::unsupported("jump-list projection is not implemented in the shell yet")
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Capability::unsupported("no dock menu / jump list equivalent on this platform")
    }
}

const fn url_schemes() -> Capability {
    #[cfg(target_os = "macos")]
    {
        Capability::Supported
    }
    #[cfg(not(target_os = "macos"))]
    {
        Capability::unsupported(
            "URL-scheme registration is an unimplemented fork stub on Windows and Linux",
        )
    }
}

const fn precise_window_positioning() -> Capability {
    #[cfg(target_os = "linux")]
    {
        Capability::unsupported("absolute window position is unavailable under Wayland")
    }
    #[cfg(not(target_os = "linux"))]
    {
        Capability::Supported
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_is_internally_consistent() {
        let caps = PlatformCapabilities::detect();
        // credential_store is always deferred for now.
        assert!(!caps.credential_store.is_supported());

        // url_schemes support only ever claimed where registration is real.
        #[cfg(target_os = "macos")]
        assert!(caps.url_schemes.is_supported());
        #[cfg(not(target_os = "macos"))]
        assert!(!caps.url_schemes.is_supported());
    }

    #[test]
    fn unsupported_carries_reason() {
        let cap = Capability::unsupported("nope");
        match cap {
            Capability::Unsupported { reason } => assert_eq!(reason, "nope"),
            Capability::Supported => panic!("expected unsupported"),
        }
    }
}
