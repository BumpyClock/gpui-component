//! App identity schema, validation, and app-local build-time code generation.

pub mod build;
mod error;
pub mod parse;
pub mod schema;
pub mod versions;

pub use error::ManifestError;

/// Includes identity source generated for the consuming application.
///
/// Call this from the consuming application's own crate after its `build.rs` has
/// called [`build::emit_identity`]. Do not call it from a library crate: `OUT_DIR`
/// must belong to the application. The planned `gpui-component-app` crate will
/// re-export this macro for ergonomic access.
#[macro_export]
macro_rules! include_identity {
    () => {
        include!(concat!(env!("OUT_DIR"), "/gpui_app_identity.rs"));
    };
}
