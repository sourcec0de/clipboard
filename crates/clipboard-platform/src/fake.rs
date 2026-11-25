//! Deterministic in-memory [`crate::PlatformHost`] implementation for unit and headless UI tests.

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use crate::ClipboardContent;
use crate::ClipboardSnapshot;
use crate::ClipboardWrite;
use crate::CursorPoint;
use crate::InstalledApp;
use crate::PlatformError;
use crate::PlatformHost;
use crate::SoundEffect;

/// Fake platform host storing clipboard state, pastes, and sounds purely in memory.
#[derive(Debug)]
pub struct FakePlatform {
    base_dir: PathBuf,
    current_clipboard: Mutex<Option<ClipboardSnapshot>>,
    written_payloads: Mutex<Vec<ClipboardWrite>>,
    paste_count: AtomicU64,
    change_counter: AtomicU64,
    launch_at_login: AtomicBool,
    frontmost: Mutex<Option<String>>,
    installed_apps: Mutex<Vec<InstalledApp>>,
    played_sounds: Mutex<Vec<SoundEffect>>,
    cursor: Mutex<Option<CursorPoint>>,
}

impl FakePlatform {
    /// Creates a new [`FakePlatform`] rooted at `base_dir`.
    #[must_use]
    pub fn new(base_dir: PathBuf) -> Self {
        let seeded_apps = vec![
            InstalledApp::new("1Password.app", "com.1password.1password", None),
            InstalledApp::new("Keychain Access.app", "com.apple.keychainaccess", None),
            InstalledApp::new("Passwords.app", "com.apple.Passwords", None),
            InstalledApp::new("Terminal.app", "com.apple.Terminal", None),
            InstalledApp::new("Safari.app", "com.apple.Safari", None),
            InstalledApp::new("Notes.app", "com.apple.Notes", None),
        ];
        Self {
            base_dir,
            current_clipboard: Mutex::new(None),
            written_payloads: Mutex::new(Vec::new()),
            paste_count: AtomicU64::new(0),
            change_counter: AtomicU64::new(1),
            launch_at_login: AtomicBool::new(false),
            frontmost: Mutex::new(Some("Notes.app".to_owned())),
            installed_apps: Mutex::new(seeded_apps),
            played_sounds: Mutex::new(Vec::new()),
            cursor: Mutex::new(Some(CursorPoint::new(420.0, 260.0))),
        }
    }

    /// Simulates an external application copying text to the OS clipboard.
    pub fn push_external_text(
        &self,
        text: impl Into<String>,
        source_app: Option<String>,
        is_transient: bool,
    ) {
        let token = self.change_counter.fetch_add(1, Ordering::SeqCst);
        let snap = ClipboardSnapshot::new(
            ClipboardContent::Text {
                text: text.into(),
                html: None,
            },
            source_app,
            is_transient,
            token,
        );
        if let Ok(mut guard) = self.current_clipboard.lock() {
            *guard = Some(snap);
        }
    }

    /// Simulates an external application copying a PNG image to the OS clipboard.
    pub fn push_external_image(
        &self,
        png_bytes: Vec<u8>,
        width: u32,
        height: u32,
        source_app: Option<String>,
    ) {
        let token = self.change_counter.fetch_add(1, Ordering::SeqCst);
        let snap = ClipboardSnapshot::new(
            ClipboardContent::Image {
                png_bytes,
                width,
                height,
            },
            source_app,
            false,
            token,
        );
        if let Ok(mut guard) = self.current_clipboard.lock() {
            *guard = Some(snap);
        }
    }

    /// Returns all payloads written to the fake clipboard so far.
    #[must_use]
    pub fn written_payloads(&self) -> Vec<ClipboardWrite> {
        self.written_payloads
            .lock()
            .map_or_else(|_| Vec::new(), |g| g.clone())
    }

    /// Returns how many times [`PlatformHost::simulate_paste`] was invoked.
    #[must_use]
    pub fn paste_count(&self) -> u64 {
        self.paste_count.load(Ordering::SeqCst)
    }

    /// Returns whether launch-at-login is currently enabled.
    #[must_use]
    pub fn launch_at_login(&self) -> bool {
        self.launch_at_login.load(Ordering::SeqCst)
    }

    /// Returns all played interaction sound effects.
    #[must_use]
    pub fn played_sounds(&self) -> Vec<SoundEffect> {
        self.played_sounds
            .lock()
            .map_or_else(|_| Vec::new(), |g| g.clone())
    }
}

impl PlatformHost for FakePlatform {
    fn config_dir(&self) -> PathBuf {
        self.base_dir.join("config")
    }

    fn data_dir(&self) -> PathBuf {
        self.base_dir.join("data")
    }

    fn read_clipboard(&self) -> Result<Option<ClipboardSnapshot>, PlatformError> {
        let guard = self
            .current_clipboard
            .lock()
            .map_err(|err| PlatformError::ClipboardRead(err.to_string()))?;
        Ok(guard.clone())
    }

    fn write_clipboard(&self, payload: &ClipboardWrite) -> Result<(), PlatformError> {
        if let Ok(mut guard) = self.written_payloads.lock() {
            guard.push(payload.clone());
        }
        let token = self.change_counter.fetch_add(1, Ordering::SeqCst);
        let content = match payload {
            ClipboardWrite::PlainText(text) => ClipboardContent::Text {
                text: text.clone(),
                html: None,
            },
            ClipboardWrite::RichText { text, html } => ClipboardContent::Text {
                text: text.clone(),
                html: html.clone(),
            },
            ClipboardWrite::ImagePng(bytes) => ClipboardContent::Image {
                png_bytes: bytes.clone(),
                width: 1,
                height: 1,
            },
        };
        if let Ok(mut guard) = self.current_clipboard.lock() {
            *guard = Some(ClipboardSnapshot::new(
                content,
                Some("Clipboard.app".to_owned()),
                false,
                token,
            ));
        }
        Ok(())
    }

    fn simulate_paste(&self) -> Result<(), PlatformError> {
        self.paste_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn frontmost_app(&self) -> Option<String> {
        self.frontmost.lock().ok().and_then(|g| g.clone())
    }

    fn capture_frontmost_target(&self) -> Option<String> {
        Some("fake-target-app".to_owned())
    }

    fn restore_focus(&self, _target: Option<&str>) {}

    fn list_installed_apps(&self) -> Vec<InstalledApp> {
        self.installed_apps
            .lock()
            .map_or_else(|_| Vec::new(), |g| g.clone())
    }

    fn mouse_cursor_position(&self) -> Option<CursorPoint> {
        self.cursor.lock().ok().and_then(|g| *g)
    }

    fn set_launch_at_login(&self, enabled: bool) -> Result<(), PlatformError> {
        self.launch_at_login.store(enabled, Ordering::SeqCst);
        Ok(())
    }

    fn play_sound(&self, effect: SoundEffect) {
        if let Ok(mut guard) = self.played_sounds.lock() {
            guard.push(effect);
        }
    }

    fn platform_name(&self) -> &'static str {
        "Fake"
    }
}
