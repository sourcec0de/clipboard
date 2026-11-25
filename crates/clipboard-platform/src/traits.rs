//! Cross-platform OS host trait implemented per operating system and by [`crate::FakePlatform`].

use std::fmt::Debug;
use std::path::PathBuf;

use crate::ClipboardSnapshot;
use crate::ClipboardWrite;
use crate::CursorPoint;
use crate::InstalledApp;
use crate::PlatformError;
use crate::SoundEffect;

/// Abstraction boundary between `clipboard` domain/UI code and the host operating system.
///
/// Implementations must never make network calls.
pub trait PlatformHost: Send + Sync + Debug {
    /// Returns the local directory for settings files (`settings.json`).
    fn config_dir(&self) -> PathBuf;

    /// Returns the local directory for clipboard history and cached local image blobs.
    fn data_dir(&self) -> PathBuf;

    /// Reads the current system clipboard if non-empty.
    ///
    /// # Errors
    /// Returns [`PlatformError::ClipboardRead`] if the OS clipboard cannot be accessed.
    fn read_clipboard(&self) -> Result<Option<ClipboardSnapshot>, PlatformError>;

    /// Writes a clipping payload to the system clipboard.
    ///
    /// # Errors
    /// Returns [`PlatformError::ClipboardWrite`] if the OS clipboard rejects the payload.
    fn write_clipboard(&self, payload: &ClipboardWrite) -> Result<(), PlatformError>;

    /// Synthesizes the platform paste shortcut (`Cmd+V` on macOS, `Ctrl+V` on Linux/Windows).
    ///
    /// # Errors
    /// Returns [`PlatformError::PasteSimulation`] if Accessibility/input permissions are missing.
    fn simulate_paste(&self) -> Result<(), PlatformError>;

    /// Returns the display name of the currently frontmost application, if available.
    fn frontmost_app(&self) -> Option<String>;

    /// Captures an OS-specific handle (e.g. macOS PID or X11 window ID) for the currently
    /// frontmost external application so focus can be restored when the overlay closes.
    fn capture_frontmost_target(&self) -> Option<String>;

    /// Yields focus from our process and restores focus to `target` (when provided).
    fn restore_focus(&self, target: Option<&str>);

    /// Lists locally installed desktop applications for the Blacklist picker.
    fn list_installed_apps(&self) -> Vec<InstalledApp>;

    /// Returns the current pointer location in screen points when available.
    fn mouse_cursor_position(&self) -> Option<CursorPoint>;

    /// Enables or disables automatic launch at user login.
    ///
    /// # Errors
    /// Returns [`PlatformError::LaunchAtLogin`] if the OS startup entry cannot be written.
    fn set_launch_at_login(&self, enabled: bool) -> Result<(), PlatformError>;

    /// Plays a subtle local interaction sound if supported by the OS.
    fn play_sound(&self, effect: SoundEffect);

    /// Short human-readable platform identifier (`"macOS"`, `"Linux"`, `"Windows"`).
    fn platform_name(&self) -> &'static str;
}
