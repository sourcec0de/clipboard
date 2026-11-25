//! Local clipboard history store, deduplication, blacklist/transient enforcement, and sequential queue.

use std::collections::VecDeque;
use std::fs;
use std::path::Path;

use clipboard_platform::ClipboardContent;
use clipboard_platform::ClipboardSnapshot;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

use crate::AppSettings;
use crate::Clipping;
use crate::settings::restrict_file_permissions;

/// Errors returned while loading or persisting [`ClipboardHistory`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum HistoryError {
    /// Local filesystem I/O error.
    #[error("history I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON serialization error.
    #[error("history JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Result of attempting to ingest a [`ClipboardSnapshot`] into [`ClipboardHistory`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IngestOutcome {
    /// Stored as a new clipping (or promoted an identical existing clipping to the top).
    Recorded,
    /// Ignored because the OS marked it transient or concealed (e.g., 1Password).
    IgnoredTransient,
    /// Ignored because the source application is in the user's blacklist.
    IgnoredBlacklisted,
    /// Ignored because the payload exceeded `max_clipping_mb`.
    IgnoredTooLarge,
    /// Ignored because the clipping was empty.
    IgnoredEmpty,
}

/// Ordered clipboard history and optional sequential paste queue.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClipboardHistory {
    /// Most-recent-first list of stored clippings.
    pub items: Vec<Clipping>,
    /// FIFO queue of clipping IDs captured while Sequential Paste Mode is active.
    pub sequential_queue: VecDeque<Uuid>,
}

impl ClipboardHistory {
    /// Creates an empty [`ClipboardHistory`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            items: Vec::new(),
            sequential_queue: VecDeque::new(),
        }
    }

    /// Ingests a [`ClipboardSnapshot`] respecting transient flags, blacklist, size limit, and history cap.
    pub fn ingest(
        &mut self,
        snapshot: ClipboardSnapshot,
        settings: &AppSettings,
        sequential_mode_active: bool,
    ) -> IngestOutcome {
        if snapshot.is_transient_or_concealed {
            return IngestOutcome::IgnoredTransient;
        }
        if let Some(app) = snapshot.source_app.as_deref()
            && settings.is_app_blacklisted(app)
        {
            return IngestOutcome::IgnoredBlacklisted;
        }
        let max_bytes = usize::try_from(settings.max_clipping_mb)
            .unwrap_or(25)
            .saturating_mul(1024 * 1024);
        if snapshot.content.byte_len() > max_bytes {
            return IngestOutcome::IgnoredTooLarge;
        }

        let clipping = match snapshot.content {
            ClipboardContent::Text { text, html } => {
                if text.is_empty() {
                    return IngestOutcome::IgnoredEmpty;
                }
                Clipping::from_text(text, html, snapshot.source_app)
            }
            ClipboardContent::Image {
                png_bytes,
                width,
                height,
            } => {
                if png_bytes.is_empty() {
                    return IngestOutcome::IgnoredEmpty;
                }
                Clipping::from_image(&png_bytes, width, height, snapshot.source_app)
            }
            _ => return IngestOutcome::IgnoredEmpty,
        };

        let id = self.push_clipping(clipping, settings.history_limit);
        if sequential_mode_active {
            self.sequential_queue.push_back(id);
        }
        IngestOutcome::Recorded
    }

    /// Inserts or promotes a [`Clipping`] to the top of history and enforces `history_limit`.
    pub fn push_clipping(&mut self, mut clipping: Clipping, history_limit: usize) -> Uuid {
        if let Some(pos) = self
            .items
            .iter()
            .position(|existing| existing.content_hash == clipping.content_hash)
        {
            let mut existing = self.items.remove(pos);
            existing.created_at_epoch_ms = clipping.created_at_epoch_ms;
            if clipping.source_app.is_some() {
                existing.source_app = clipping.source_app;
            }
            let id = existing.id;
            self.items.insert(0, existing);
            return id;
        }

        let id = clipping.id;
        clipping.pinned = false;
        self.items.insert(0, clipping);
        self.enforce_limit(history_limit);
        id
    }

    /// Prunes oldest unpinned items so total unpinned count does not exceed `limit`.
    pub fn enforce_limit(&mut self, limit: usize) {
        let cap = limit.max(1);
        while self.items.len() > cap {
            if let Some(pos) = self.items.iter().rposition(|item| !item.pinned) {
                let removed = self.items.remove(pos);
                self.sequential_queue.retain(|id| *id != removed.id);
            } else {
                break;
            }
        }
    }

    /// Filters history items matching `query`, preserving recency order.
    #[must_use]
    pub fn search(&self, query: &str) -> Vec<&Clipping> {
        self.items
            .iter()
            .filter(|item| item.matches_query(query))
            .collect()
    }

    /// Returns items currently queued in Sequential Paste Mode (FIFO order).
    #[must_use]
    pub fn sequential_items(&self) -> Vec<&Clipping> {
        self.sequential_queue
            .iter()
            .filter_map(|id| self.items.iter().find(|c| c.id == *id))
            .collect()
    }

    /// Pops and returns the next clipping from the Sequential Paste Queue.
    pub fn pop_sequential(&mut self) -> Option<Clipping> {
        while let Some(id) = self.sequential_queue.pop_front() {
            if let Some(found) = self.items.iter().find(|c| c.id == id) {
                return Some(found.clone());
            }
        }
        None
    }

    /// Toggles the pinned flag on the clipping with `id`.
    pub fn toggle_pin(&mut self, id: Uuid) -> bool {
        if let Some(item) = self.items.iter_mut().find(|c| c.id == id) {
            item.pinned = !item.pinned;
            return true;
        }
        false
    }

    /// Removes the clipping with `id` from history and the sequential queue.
    pub fn remove(&mut self, id: Uuid) -> bool {
        let before = self.items.len();
        self.items.retain(|c| c.id != id);
        self.sequential_queue.retain(|queued| *queued != id);
        self.items.len() != before
    }

    /// Clears all unpinned clippings and resets the sequential queue.
    pub fn clear_unpinned(&mut self) {
        self.items.retain(|c| c.pinned);
        self.sequential_queue.clear();
    }

    /// Loads history from `dir/history.json`, returning an empty history if missing.
    ///
    /// # Errors
    /// Returns [`HistoryError`] if an existing history file cannot be read or parsed.
    pub fn load_from_dir(dir: &Path) -> Result<Self, HistoryError> {
        let path = dir.join("history.json");
        if !path.exists() {
            return Ok(Self::new());
        }
        let raw = fs::read_to_string(&path)?;
        let history: Self = serde_json::from_str(&raw)?;
        Ok(history)
    }

    /// Persists history to `dir/history.json` with owner-only permissions on Unix.
    ///
    /// # Errors
    /// Returns [`HistoryError`] if the file cannot be written.
    pub fn save_to_dir(&self, dir: &Path) -> Result<(), HistoryError> {
        fs::create_dir_all(dir)?;
        let path = dir.join("history.json");
        let payload = serde_json::to_string(self)?;
        fs::write(&path, payload)?;
        restrict_file_permissions(&path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clipboard_platform::ClipboardContent;
    use clipboard_platform::ClipboardSnapshot;

    use super::ClipboardHistory;
    use super::IngestOutcome;
    use crate::AppSettings;

    #[test]
    fn ignores_transient_blacklisted_and_oversized_clippings() {
        let settings = AppSettings::default();
        let mut history = ClipboardHistory::new();

        let transient = ClipboardSnapshot::new(
            ClipboardContent::Text {
                text: "s3cr3t".to_owned(),
                html: None,
            },
            Some("1Password.app".to_owned()),
            true,
            1,
        );
        assert_eq!(
            history.ingest(transient, &settings, false),
            IngestOutcome::IgnoredTransient
        );

        let blacklisted = ClipboardSnapshot::new(
            ClipboardContent::Text {
                text: "keychain-secret".to_owned(),
                html: None,
            },
            Some("Keychain Access.app".to_owned()),
            false,
            2,
        );
        assert_eq!(
            history.ingest(blacklisted, &settings, false),
            IngestOutcome::IgnoredBlacklisted
        );

        let mut tiny_limit = settings.clone();
        tiny_limit.max_clipping_mb = 1;
        let big = ClipboardSnapshot::new(
            ClipboardContent::Text {
                text: "x".repeat(2 * 1024 * 1024),
                html: None,
            },
            Some("Notes.app".to_owned()),
            false,
            3,
        );
        assert_eq!(
            history.ingest(big, &tiny_limit, false),
            IngestOutcome::IgnoredTooLarge
        );
        assert!(history.items.is_empty());
    }

    #[test]
    fn deduplicates_searches_and_supports_sequential_queue() {
        let settings = AppSettings::default();
        let mut history = ClipboardHistory::new();

        for (i, text) in ["first", "second", "third"].into_iter().enumerate() {
            let snap = ClipboardSnapshot::new(
                ClipboardContent::Text {
                    text: text.to_owned(),
                    html: None,
                },
                Some("Notes.app".to_owned()),
                false,
                i as u64 + 1,
            );
            assert_eq!(
                history.ingest(snap, &settings, true),
                IngestOutcome::Recorded
            );
        }

        // Most recent first in history
        assert_eq!(history.items[0].text, "third");
        assert_eq!(history.search("sec").len(), 1);
        assert_eq!(history.search("sec")[0].text, "second");

        // FIFO order in sequential queue
        assert_eq!(
            history.pop_sequential().map(|c| c.text),
            Some("first".to_owned())
        );
        assert_eq!(
            history.pop_sequential().map(|c| c.text),
            Some("second".to_owned())
        );
        assert_eq!(
            history.pop_sequential().map(|c| c.text),
            Some("third".to_owned())
        );
        assert!(history.pop_sequential().is_none());
    }
}
