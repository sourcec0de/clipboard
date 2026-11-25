//! Pure Rust domain logic for `clipboard`.
//!
//! Contains zero UI framework dependencies and zero network dependencies.

#![forbid(unsafe_code)]

mod clipping;
mod filter;
mod history;
mod settings;
mod shortcut;

pub use clipping::Clipping;
pub use clipping::ClippingKind;
pub use filter::TextFilter;
pub use history::ClipboardHistory;
pub use history::HistoryError;
pub use history::IngestOutcome;
pub use settings::AppSettings;
pub use settings::BlacklistEntry;
pub use settings::DockMenubarMode;
pub use settings::QuickPasteMenuStyle;
pub use settings::SettingsError;
pub use shortcut::KeyboardShortcut;
pub use shortcut::Modifiers;
pub use shortcut::ShortcutAction;
pub use shortcut::ShortcutGroup;
