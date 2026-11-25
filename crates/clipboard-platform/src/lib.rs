//! Cross-platform OS abstractions for `clipboard`.
//!
//! This crate intentionally contains **zero** networking dependencies.

#![forbid(unsafe_code)]

mod error;
mod fake;
mod native;
mod traits;
mod types;

pub use error::PlatformError;
pub use fake::FakePlatform;
pub use native::NativePlatform;
pub use traits::PlatformHost;
pub use types::ClipboardContent;
pub use types::ClipboardSnapshot;
pub use types::ClipboardWrite;
pub use types::CursorPoint;
pub use types::InstalledApp;
pub use types::SoundEffect;

/// Creates the production OS platform host for the current target operating system.
#[must_use]
pub fn native_platform() -> Box<dyn PlatformHost> {
    Box::new(NativePlatform::new())
}
