//! Persistent local-only application settings, blacklist, and shortcut bindings.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

use crate::KeyboardShortcut;
use crate::ShortcutAction;
use crate::TextFilter;

/// Errors returned while loading or saving [`AppSettings`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SettingsError {
    /// Filesystem read/write failure.
    #[error("settings I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON serialization/deserialization failure.
    #[error("settings JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Visual density / layout mode for the Quick Paste overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub enum QuickPasteMenuStyle {
    /// Standard 2-line preview rows (matches the reference overlay).
    #[default]
    Normal,
    /// Compact single-line rows.
    Compact,
    /// Expanded rows with an inline preview footer.
    Expanded,
}

impl QuickPasteMenuStyle {
    /// All options in dropdown order.
    pub const ALL: &'static [Self] = &[Self::Normal, Self::Compact, Self::Expanded];

    /// Dropdown display label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Compact => "Compact",
            Self::Expanded => "Expanded",
        }
    }

    /// Stable string identifier for `SelectItem`.
    #[must_use]
    pub const fn as_id(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Compact => "compact",
            Self::Expanded => "expanded",
        }
    }

    /// Parses a [`QuickPasteMenuStyle`] from its string identifier.
    #[must_use]
    pub fn from_id(id: &str) -> Self {
        match id {
            "compact" => Self::Compact,
            "expanded" => Self::Expanded,
            _ => Self::Normal,
        }
    }
}

/// Dock and system tray visibility preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DockMenubarMode {
    /// Show both dock/taskbar icon and menubar/tray icon.
    #[default]
    DockAndMenubar,
    /// Show menubar/tray icon only.
    MenubarOnly,
    /// Show dock/taskbar icon only.
    DockOnly,
}

impl DockMenubarMode {
    /// All options in dropdown order.
    pub const ALL: &'static [Self] = &[Self::DockAndMenubar, Self::MenubarOnly, Self::DockOnly];

    /// Dropdown display label matching Screenshot 4.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::DockAndMenubar => "Display Dock and Menubar Icon",
            Self::MenubarOnly => "Display Menubar Icon Only",
            Self::DockOnly => "Display Dock Icon Only",
        }
    }

    /// Stable string identifier for `SelectItem`.
    #[must_use]
    pub const fn as_id(self) -> &'static str {
        match self {
            Self::DockAndMenubar => "dock_and_menubar",
            Self::MenubarOnly => "menubar_only",
            Self::DockOnly => "dock_only",
        }
    }

    /// Parses a [`DockMenubarMode`] from its string identifier.
    #[must_use]
    pub fn from_id(id: &str) -> Self {
        match id {
            "menubar_only" => Self::MenubarOnly,
            "dock_only" => Self::DockOnly,
            _ => Self::DockAndMenubar,
        }
    }
}

/// An application ignored by the clipboard monitor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct BlacklistEntry {
    /// Display name (for example `"Keychain Access.app"`).
    pub name: String,
    /// Optional bundle or desktop identifier.
    pub identifier: String,
}

impl BlacklistEntry {
    /// Creates a new [`BlacklistEntry`].
    #[must_use]
    pub fn new(name: impl Into<String>, identifier: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            identifier: identifier.into(),
        }
    }

    /// Returns `true` when `source_app` matches this blacklist entry (case-insensitive).
    #[must_use]
    pub fn matches_app(&self, source_app: &str) -> bool {
        let candidate = source_app.trim();
        if candidate.is_empty() {
            return false;
        }
        let candidate_stem = candidate.strip_suffix(".app").unwrap_or(candidate);
        let name_stem = self.name.strip_suffix(".app").unwrap_or(&self.name);
        candidate.eq_ignore_ascii_case(&self.name)
            || candidate.eq_ignore_ascii_case(&self.identifier)
            || candidate_stem.eq_ignore_ascii_case(name_stem)
    }
}

/// Complete local-only configuration for `clipboard`.
///
/// Intentionally omits any cloud/network sync option to preserve the zero-network guarantee.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "models independent user preference checkboxes on the General settings screen"
)]
#[non_exhaustive]
pub struct AppSettings {
    /// Maximum size of an individual clipping in megabytes (`1..=512`, default `25`).
    pub max_clipping_mb: u32,
    /// Maximum number of recent clippings kept in history (`100..=1000`, default `100`).
    pub history_limit: usize,
    /// Always strip rich formatting and paste plain text.
    pub always_paste_plain_text: bool,
    /// Quick Paste overlay visual density.
    pub quick_paste_menu: QuickPasteMenuStyle,
    /// Position the Quick Paste overlay near the mouse pointer when opened.
    pub appear_behind_mouse_cursor: bool,
    /// Paste the selected clipping when modifier keys are released.
    pub release_to_paste: bool,
    /// Play subtle local OS interaction sounds on copy/paste.
    pub enable_interaction_sounds: bool,
    /// Dock / menubar visibility preference.
    pub dock_menubar_mode: DockMenubarMode,
    /// Launch automatically when logging in.
    pub open_at_login: bool,
    /// Applications ignored by the clipboard monitor.
    pub blacklist: Vec<BlacklistEntry>,
    /// Global shortcut bindings keyed by [`ShortcutAction`].
    pub shortcuts: BTreeMap<ShortcutAction, KeyboardShortcut>,
    /// Last applied text filter (used by `Paste with Last Filter`).
    pub last_filter: TextFilter,
}

impl Default for AppSettings {
    fn default() -> Self {
        let mut shortcuts = BTreeMap::new();
        for &action in ShortcutAction::ACTIONS
            .iter()
            .chain(ShortcutAction::RECENT_CLIPPINGS.iter())
        {
            let default = action.default_shortcut();
            if !default.is_empty() {
                shortcuts.insert(action, default);
            }
        }
        Self {
            max_clipping_mb: 25,
            history_limit: 100,
            always_paste_plain_text: false,
            quick_paste_menu: QuickPasteMenuStyle::Normal,
            appear_behind_mouse_cursor: false,
            release_to_paste: false,
            enable_interaction_sounds: false,
            dock_menubar_mode: DockMenubarMode::DockAndMenubar,
            open_at_login: true,
            blacklist: vec![
                BlacklistEntry::new("Keychain Access.app", "com.apple.keychainaccess"),
                BlacklistEntry::new("Passwords.app", "com.apple.Passwords"),
            ],
            shortcuts,
            last_filter: TextFilter::PlainText,
        }
    }
}

impl AppSettings {
    /// Minimum configurable history count.
    pub const MIN_HISTORY_LIMIT: usize = 100;
    /// Maximum configurable history count.
    pub const MAX_HISTORY_LIMIT: usize = 1000;

    /// Returns the shortcut currently bound to `action`, or an empty shortcut if unset.
    #[must_use]
    pub fn shortcut_for(&self, action: ShortcutAction) -> KeyboardShortcut {
        self.shortcuts
            .get(&action)
            .cloned()
            .unwrap_or_else(KeyboardShortcut::empty)
    }

    /// Sets or clears the shortcut bound to `action`.
    pub fn set_shortcut(&mut self, action: ShortcutAction, shortcut: KeyboardShortcut) {
        if shortcut.is_empty() {
            self.shortcuts.remove(&action);
        } else {
            self.shortcuts.insert(action, shortcut);
        }
    }

    /// Returns `true` if `source_app` is present in the user's blacklist.
    #[must_use]
    pub fn is_app_blacklisted(&self, source_app: &str) -> bool {
        self.blacklist
            .iter()
            .any(|entry| entry.matches_app(source_app))
    }

    /// Adds an application to the blacklist if not already present.
    pub fn add_to_blacklist(&mut self, entry: BlacklistEntry) -> bool {
        if self.is_app_blacklisted(&entry.name) {
            return false;
        }
        self.blacklist.push(entry);
        true
    }

    /// Removes the blacklist entry at `index` if in bounds.
    pub fn remove_from_blacklist(&mut self, index: usize) -> bool {
        if index < self.blacklist.len() {
            self.blacklist.remove(index);
            true
        } else {
            false
        }
    }

    /// Loads settings from `dir/settings.json`, returning [`Self::default`] when missing.
    ///
    /// # Errors
    /// Returns [`SettingsError`] if an existing settings file cannot be read or parsed.
    pub fn load_from_dir(dir: &Path) -> Result<Self, SettingsError> {
        let path = dir.join("settings.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(&path)?;
        let mut parsed: Self = serde_json::from_str(&raw)?;
        parsed.history_limit = parsed
            .history_limit
            .clamp(Self::MIN_HISTORY_LIMIT, Self::MAX_HISTORY_LIMIT);
        parsed.max_clipping_mb = parsed.max_clipping_mb.clamp(1, 512);
        Ok(parsed)
    }

    /// Saves settings to `dir/settings.json` with owner-only permissions on Unix.
    ///
    /// # Errors
    /// Returns [`SettingsError`] if the file cannot be created or written.
    pub fn save_to_dir(&self, dir: &Path) -> Result<(), SettingsError> {
        fs::create_dir_all(dir)?;
        let path = dir.join("settings.json");
        let payload = serde_json::to_string_pretty(self)?;
        fs::write(&path, payload)?;
        restrict_file_permissions(&path);
        Ok(())
    }
}

pub(crate) fn restrict_file_permissions(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}
