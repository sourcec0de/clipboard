//! Global keyboard shortcut definitions matching the Shortcuts preferences screen.

use serde::Deserialize;
use serde::Serialize;

/// Keyboard modifier bitflags for global shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "represents the four independent hardware keyboard modifier keys"
)]
#[non_exhaustive]
pub struct Modifiers {
    /// Control modifier (`⌃`).
    pub control: bool,
    /// Option / Alt modifier (`⌥`).
    pub option: bool,
    /// Shift modifier (`⇧`).
    pub shift: bool,
    /// Command / Super modifier (`⌘`).
    pub command: bool,
}

impl Modifiers {
    /// Constructs a [`Modifiers`] value.
    #[must_use]
    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "constructor accepts the four independent keyboard modifier flags"
    )]
    pub const fn new(control: bool, option: bool, shift: bool, command: bool) -> Self {
        Self {
            control,
            option,
            shift,
            command,
        }
    }

    /// Returns `true` when no modifiers are pressed.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        !self.control && !self.option && !self.shift && !self.command
    }
}

/// A configurable global keyboard shortcut.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct KeyboardShortcut {
    /// Primary key label (for example `"V"`, `"C"`, `"1"`). Empty when unset.
    pub key: String,
    /// Modifier flags.
    pub modifiers: Modifiers,
}

impl KeyboardShortcut {
    /// Creates an unset shortcut.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            key: String::new(),
            modifiers: Modifiers::new(false, false, false, false),
        }
    }

    /// Creates a keyboard shortcut with key and modifiers.
    #[must_use]
    pub fn keyboard(key: impl Into<String>, modifiers: Modifiers) -> Self {
        Self {
            key: key.into(),
            modifiers,
        }
    }

    /// Returns `true` when no shortcut is assigned.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.key.is_empty() && self.modifiers.is_empty()
    }

    /// Converts a DOM `key` / `code` pair and modifiers from a `KeyEvent` into a [`KeyboardShortcut`].
    #[must_use]
    pub fn from_dom_key(key: &str, code: &str, modifiers: Modifiers) -> Option<Self> {
        let normalized = normalize_dom_code(code).or_else(|| normalize_dom_key(key))?;
        Some(Self::keyboard(normalized, modifiers))
    }

    /// Formats this shortcut for NSH's `register_global_shortcut` (`"shift+mod+v"`).
    #[must_use]
    pub fn to_nsh_keystroke(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }
        let token = nsh_key_token(&self.key)?;
        let mut parts: Vec<&str> = Vec::with_capacity(5);
        if self.modifiers.control {
            parts.push("ctrl");
        }
        if self.modifiers.option {
            parts.push("alt");
        }
        if self.modifiers.shift {
            parts.push("shift");
        }
        if self.modifiers.command {
            parts.push("mod");
        }
        parts.push(token);
        Some(parts.join("+"))
    }

    /// Formats the shortcut using compact macOS/desktop modifier glyphs (`⇧⌘V`, `⌃⌘C`).
    #[must_use]
    pub fn display_string(&self, is_macos: bool) -> String {
        if self.is_empty() {
            return String::new();
        }
        if is_macos {
            let mut out = String::new();
            if self.modifiers.control {
                out.push('⌃');
            }
            if self.modifiers.option {
                out.push('⌥');
            }
            if self.modifiers.shift {
                out.push('⇧');
            }
            if self.modifiers.command {
                out.push('⌘');
            }
            out.push_str(&self.key);
            out
        } else {
            let mut parts: Vec<&str> = Vec::with_capacity(5);
            if self.modifiers.control {
                parts.push("Ctrl");
            }
            if self.modifiers.option {
                parts.push("Alt");
            }
            if self.modifiers.shift {
                parts.push("Shift");
            }
            if self.modifiers.command {
                parts.push("Super");
            }
            parts.push(&self.key);
            parts.join("+")
        }
    }
}

/// Section grouping in the Shortcuts preferences screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShortcutGroup {
    /// Core clipboard actions (`Show Clipboard`, `Sequential Paste`, etc.).
    Actions,
    /// Direct paste slots (`Paste Clipping 1` .. `Paste Clipping 10`).
    PasteRecentClippings,
}

/// All bindable global shortcut actions shown in the Shortcuts preferences tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ShortcutAction {
    /// Open or toggle the floating Quick Paste Clipboard overlay (`⇧⌘V`).
    ShowClipboard,
    /// Bring the main Clipboard preferences/history window to the front.
    ShowMainWindow,
    /// Toggle Sequential Paste Mode (`⌃⌘C`).
    ToggleSequentialPaste,
    /// Paste the next item from the Sequential Queue (`⌃⌘V`).
    PasteFromSequentialQueue,
    /// Open the Filter picker window.
    OpenFilterWindow,
    /// Paste the most recent clipping using the last selected filter.
    PasteWithLastFilter,
    /// Directly paste clipping slot `1..=10`.
    PasteClipping(u8),
}

impl ShortcutAction {
    /// All action shortcuts in display order for the `"Actions"` section.
    pub const ACTIONS: &'static [Self] = &[
        Self::ShowClipboard,
        Self::ShowMainWindow,
        Self::ToggleSequentialPaste,
        Self::PasteFromSequentialQueue,
        Self::OpenFilterWindow,
        Self::PasteWithLastFilter,
    ];

    /// Direct clipping paste slots `1..=10` for `"Paste Recent Clippings"`.
    pub const RECENT_CLIPPINGS: &'static [Self] = &[
        Self::PasteClipping(1),
        Self::PasteClipping(2),
        Self::PasteClipping(3),
        Self::PasteClipping(4),
        Self::PasteClipping(5),
        Self::PasteClipping(6),
        Self::PasteClipping(7),
        Self::PasteClipping(8),
        Self::PasteClipping(9),
        Self::PasteClipping(10),
    ];

    /// Returns the section group for this action.
    #[must_use]
    pub const fn group(self) -> ShortcutGroup {
        match self {
            Self::PasteClipping(_) => ShortcutGroup::PasteRecentClippings,
            _ => ShortcutGroup::Actions,
        }
    }

    /// Display label matching the Shortcuts preferences screenshots.
    #[must_use]
    pub fn display_name(self) -> String {
        match self {
            Self::ShowClipboard => "Show Clipboard".to_owned(),
            Self::ShowMainWindow => "Show Main Window".to_owned(),
            Self::ToggleSequentialPaste => "Toggle Sequential Paste Mode".to_owned(),
            Self::PasteFromSequentialQueue => "Paste from Sequential Queue".to_owned(),
            Self::OpenFilterWindow => "Open Filter Window".to_owned(),
            Self::PasteWithLastFilter => "Paste with Last Filter".to_owned(),
            Self::PasteClipping(n) => format!("Paste Clipping {n}"),
        }
    }

    /// Default shortcut matching the screenshots (`⇧⌘V`, `⌃⌘C`, `⌃⌘V`).
    #[must_use]
    pub fn default_shortcut(self) -> KeyboardShortcut {
        match self {
            Self::ShowClipboard => {
                KeyboardShortcut::keyboard("V", Modifiers::new(false, false, true, true))
            }
            Self::ToggleSequentialPaste => {
                KeyboardShortcut::keyboard("C", Modifiers::new(true, false, false, true))
            }
            Self::PasteFromSequentialQueue => {
                KeyboardShortcut::keyboard("V", Modifiers::new(true, false, false, true))
            }
            _ => KeyboardShortcut::empty(),
        }
    }
}

fn normalize_dom_code(code: &str) -> Option<String> {
    if let Some(letter) = code.strip_prefix("Key")
        && letter.len() == 1
        && letter.chars().all(|c| c.is_ascii_alphabetic())
    {
        return Some(letter.to_ascii_uppercase());
    }
    if let Some(digit) = code.strip_prefix("Digit")
        && digit.len() == 1
        && digit.chars().all(|c| c.is_ascii_digit())
    {
        return Some(digit.to_owned());
    }
    let mapped = match code {
        "Space" => "Space",
        "Enter" | "NumpadEnter" => "Enter",
        "Tab" => "Tab",
        "Backspace" => "Backspace",
        "Delete" => "Delete",
        "ArrowUp" => "Up",
        "ArrowDown" => "Down",
        "ArrowLeft" => "Left",
        "ArrowRight" => "Right",
        "Minus" => "-",
        "Equal" => "=",
        "BracketLeft" => "[",
        "BracketRight" => "]",
        "Semicolon" => ";",
        "Quote" => "'",
        "Comma" => ",",
        "Period" => ".",
        "Slash" => "/",
        "Backquote" => "`",
        _ => {
            if let Some(num) = code.strip_prefix('F')
                && !num.is_empty()
                && num.chars().all(|c| c.is_ascii_digit())
            {
                return Some(code.to_owned());
            }
            return None;
        }
    };
    Some(mapped.to_owned())
}

fn normalize_dom_key(key: &str) -> Option<String> {
    match key {
        "" | "Unidentified" | "Shift" | "Control" | "Alt" | "Meta" | "CapsLock" | "Fn"
        | "Escape" => None,
        " " | "Spacebar" | "Space" => Some("Space".to_owned()),
        "Enter" => Some("Enter".to_owned()),
        "Tab" => Some("Tab".to_owned()),
        "Backspace" => Some("Backspace".to_owned()),
        "Delete" => Some("Delete".to_owned()),
        "ArrowUp" | "Up" => Some("Up".to_owned()),
        "ArrowDown" | "Down" => Some("Down".to_owned()),
        "ArrowLeft" | "Left" => Some("Left".to_owned()),
        "ArrowRight" | "Right" => Some("Right".to_owned()),
        other => {
            if let Some(num) = other.strip_prefix('F').or_else(|| other.strip_prefix('f'))
                && !num.is_empty()
                && num.chars().all(|c| c.is_ascii_digit())
            {
                return Some(format!("F{num}"));
            }
            let mut chars = other.chars();
            if let (Some(ch), None) = (chars.next(), chars.next())
                && ch.is_ascii_graphic()
            {
                return Some(ch.to_ascii_uppercase().to_string());
            }
            None
        }
    }
}

fn nsh_key_token(key: &str) -> Option<&str> {
    let token = match key {
        "Space" => "space",
        "Enter" => "enter",
        "Tab" => "tab",
        "Backspace" => "backspace",
        "Delete" => "delete",
        "Up" => "up",
        "Down" => "down",
        "Left" => "left",
        "Right" => "right",
        "+" => "plus",
        other if !other.is_empty() && other.is_ascii() => other,
        _ => return None,
    };
    Some(token)
}

#[cfg(test)]
mod tests {
    use super::ShortcutAction;

    #[test]
    fn default_shortcuts_match_screenshots() {
        let show = ShortcutAction::ShowClipboard.default_shortcut();
        assert_eq!(show.display_string(true), "⇧⌘V");
        assert_eq!(show.to_nsh_keystroke().as_deref(), Some("shift+mod+V"));

        let seq_toggle = ShortcutAction::ToggleSequentialPaste.default_shortcut();
        assert_eq!(seq_toggle.display_string(true), "⌃⌘C");

        let seq_paste = ShortcutAction::PasteFromSequentialQueue.default_shortcut();
        assert_eq!(seq_paste.display_string(true), "⌃⌘V");
    }
}
