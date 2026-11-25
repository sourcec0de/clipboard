//! Error types returned by [`crate::PlatformHost`] implementations.

use thiserror::Error;

/// Errors produced by platform clipboard, paste, or OS integration calls.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PlatformError {
    /// Failed to read from the operating system clipboard.
    #[error("clipboard read failed: {0}")]
    ClipboardRead(String),
    /// Failed to write to the operating system clipboard.
    #[error("clipboard write failed: {0}")]
    ClipboardWrite(String),
    /// Failed to synthesize the OS paste shortcut into the frontmost application.
    #[error("paste simulation failed: {0}")]
    PasteSimulation(String),
    /// Failed to configure launch-at-login registration.
    #[error("launch-at-login update failed: {0}")]
    LaunchAtLogin(String),
    /// Local filesystem I/O error.
    #[error("local storage I/O error: {0}")]
    Io(#[from] std::io::Error),
}
