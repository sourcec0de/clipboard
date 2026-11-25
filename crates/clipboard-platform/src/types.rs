//! Platform-agnostic data types exchanged across [`crate::PlatformHost`].

use std::path::PathBuf;

/// Payload read from the operating system clipboard.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClipboardContent {
    /// Plain text (optionally accompanied by HTML/rich-text representation).
    Text {
        /// Plain UTF-8 text.
        text: String,
        /// Optional rich HTML representation when available.
        html: Option<String>,
    },
    /// Raster image captured as PNG bytes with dimensions.
    Image {
        /// Encoded PNG bytes.
        png_bytes: Vec<u8>,
        /// Image width in pixels.
        width: u32,
        /// Image height in pixels.
        height: u32,
    },
}

impl ClipboardContent {
    /// Returns the approximate payload size in bytes.
    #[must_use]
    pub const fn byte_len(&self) -> usize {
        match self {
            Self::Text { text, html } => {
                let html_len = match html {
                    Some(h) => h.len(),
                    None => 0,
                };
                text.len() + html_len
            }
            Self::Image { png_bytes, .. } => png_bytes.len(),
        }
    }
}

/// Snapshot of the current system clipboard along with security metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ClipboardSnapshot {
    /// Captured clipboard payload.
    pub content: ClipboardContent,
    /// Name of the frontmost application that owned the copy, if detectable.
    pub source_app: Option<String>,
    /// True when the clipboard entry is marked transient, concealed, or password-manager-owned.
    pub is_transient_or_concealed: bool,
    /// Monotonic or content-derived change token used to skip unchanged polls.
    pub change_token: u64,
}

impl ClipboardSnapshot {
    /// Creates a new [`ClipboardSnapshot`].
    #[must_use]
    pub const fn new(
        content: ClipboardContent,
        source_app: Option<String>,
        is_transient_or_concealed: bool,
        change_token: u64,
    ) -> Self {
        Self {
            content,
            source_app,
            is_transient_or_concealed,
            change_token,
        }
    }
}

/// Payload written back to the operating system clipboard for pasting.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClipboardWrite {
    /// Write plain UTF-8 text only.
    PlainText(String),
    /// Write text with optional rich HTML payload.
    RichText {
        /// Plain text fallback.
        text: String,
        /// Optional rich HTML.
        html: Option<String>,
    },
    /// Write PNG image bytes.
    ImagePng(Vec<u8>),
}

/// An installed desktop application discovered for the blacklist picker.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct InstalledApp {
    /// Display name (for example `"Keychain Access.app"` or `"Firefox"`).
    pub name: String,
    /// Bundle ID or desktop entry identifier.
    pub identifier: String,
    /// Optional local path to the application bundle or desktop entry.
    pub path: Option<PathBuf>,
}

impl InstalledApp {
    /// Creates a new [`InstalledApp`].
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        identifier: impl Into<String>,
        path: Option<PathBuf>,
    ) -> Self {
        Self {
            name: name.into(),
            identifier: identifier.into(),
            path,
        }
    }
}

/// Screen-space mouse cursor coordinates in logical points.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct CursorPoint {
    /// Horizontal screen coordinate.
    pub x: f32,
    /// Vertical screen coordinate.
    pub y: f32,
}

impl CursorPoint {
    /// Creates a new [`CursorPoint`].
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Local UI interaction sound effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SoundEffect {
    /// Subtle sound when a clipping is captured.
    Copy,
    /// Subtle sound when a clipping is pasted.
    Paste,
    /// Subtle sound when an action cannot be performed.
    Error,
}
