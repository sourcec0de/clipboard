//! Central reactive application store, background clipboard monitor, and paste orchestration.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

use clipboard_core::AppSettings;
use clipboard_core::BlacklistEntry;
use clipboard_core::ClipboardHistory;
use clipboard_core::Clipping;
use clipboard_core::ClippingKind;
use clipboard_core::DockMenubarMode;
use clipboard_core::IngestOutcome;
use clipboard_core::KeyboardShortcut;
use clipboard_core::QuickPasteMenuStyle;
use clipboard_core::ShortcutAction;
use clipboard_core::TextFilter;
use clipboard_platform::ClipboardSnapshot;
use clipboard_platform::ClipboardWrite;
use clipboard_platform::InstalledApp;
use clipboard_platform::PlatformHost;
use clipboard_platform::SoundEffect;
use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use nsh_core::prelude::Signal;
use nsh_core::prelude::SignalEquals;
use nsh_core::prelude::set_interval;
use nsh_core::prelude::signal_with;
use uuid::Uuid;

/// Factory creating the active [`PlatformHost`].
pub type PlatformFactory = fn() -> Box<dyn PlatformHost>;

/// Launch configuration injected by `clipboard-app` or integration tests.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct LaunchConfig {
    /// Factory constructing the [`PlatformHost`].
    pub platform_factory: PlatformFactory,
    /// Whether to spawn the background OS clipboard polling thread.
    pub enable_background_polling: bool,
}

impl LaunchConfig {
    /// Creates a production [`LaunchConfig`] with background clipboard polling enabled.
    #[must_use]
    pub const fn new(platform_factory: PlatformFactory) -> Self {
        Self {
            platform_factory,
            enable_background_polling: true,
        }
    }

    /// Creates a headless test [`LaunchConfig`] with background polling disabled for determinism.
    #[must_use]
    pub const fn for_tests(platform_factory: PlatformFactory) -> Self {
        Self {
            platform_factory,
            enable_background_polling: false,
        }
    }
}

/// Top-level Preferences tabs matching Screenshots 1–4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum PrefTab {
    /// General settings (history size, retention slider, quick paste options, sounds, login).
    #[default]
    General,
    /// Global keyboard shortcuts (Actions + Paste Recent Clippings 1..10).
    Shortcuts,
    /// Ignored applications blacklist.
    Blacklist,
}

impl PrefTab {
    /// Display title for the Preferences window header.
    #[must_use]
    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Shortcuts => "Shortcuts",
            Self::Blacklist => "Blacklist",
        }
    }
}

/// Source view selector in the Quick Paste overlay header (`Clipboard ⌄`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum OverlaySource {
    /// Full clipboard history (most recent first).
    #[default]
    Clipboard,
    /// Sequential Paste FIFO queue.
    SequentialQueue,
    /// Pinned clippings only.
    Pinned,
}

impl OverlaySource {
    /// Display title in the overlay header.
    #[must_use]
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Clipboard => "Clipboard",
            Self::SequentialQueue => "Sequential Queue",
            Self::Pinned => "Pinned",
        }
    }

    /// Cycles to the next overlay source.
    #[must_use]
    pub(crate) const fn next(self) -> Self {
        match self {
            Self::Clipboard => Self::SequentialQueue,
            Self::SequentialQueue => Self::Pinned,
            Self::Pinned => Self::Clipboard,
        }
    }
}

/// Complete reactive UI state for `clipboard`.
#[derive(Debug, Clone)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "holds independent overlay/modal/mode toggle flags for the reactive UI store"
)]
pub(crate) struct AppState {
    /// Active Preferences tab.
    pub active_tab: PrefTab,
    /// Persisted local-only application settings.
    pub settings: AppSettings,
    /// Persisted local-only clipboard history and sequential queue.
    pub history: ClipboardHistory,
    /// Whether the floating Quick Paste overlay is currently visible.
    pub overlay_open: bool,
    /// Active overlay source (`Clipboard`, `SequentialQueue`, `Pinned`).
    pub overlay_source: OverlaySource,
    /// Live search query inside the Quick Paste overlay.
    pub search_query: String,
    /// Index of the currently highlighted row in the filtered overlay list.
    pub selected_index: usize,
    /// Optional range anchor when extending selection with Shift+Up / Shift+Down.
    pub selection_anchor: Option<usize>,
    /// Monotonic counter incremented whenever the overlay search input should be focused.
    pub search_focus_seq: u64,
    /// Whether keyboard focus is currently navigating the list rather than typing in search.
    pub list_focused: bool,
    /// Whether the inline detail preview drawer is expanded in the overlay.
    pub preview_open: bool,
    /// Whether the text filter picker is open in the overlay.
    pub filter_picker_open: bool,
    /// Clipping ID awaiting user confirmation before deletion (`Delete` key prompt).
    pub pending_delete_id: Option<Uuid>,
    /// OS handle (e.g. macOS PID / X11 window ID) of the app that had focus before opening the overlay.
    pub previous_app_target: Option<String>,
    /// Whether Sequential Paste Mode is currently recording copies into the FIFO queue.
    pub sequential_mode_active: bool,
    /// Currently recording global shortcut action on the Shortcuts tab.
    pub recording_shortcut: Option<ShortcutAction>,
    /// Selected row index on the Blacklist tab.
    pub selected_blacklist_index: Option<usize>,
    /// Whether the Add Blacklist App picker panel is expanded.
    pub add_blacklist_open: bool,
    /// Custom app name draft inside the Add Blacklist picker.
    pub custom_blacklist_draft: String,
    /// Locally discovered installed apps for the Blacklist picker.
    pub installed_apps: Vec<InstalledApp>,
    /// Brief status line message.
    pub status_message: String,
    /// Revision counter incremented when settings shortcuts change so global hotkeys re-sync.
    pub shortcuts_revision: u64,
    /// Revision counter incremented only when visible list items or row density change.
    pub list_data_revision: u64,
}

impl AppState {
    /// Returns borrowed references to the filtered clippings currently visible in the overlay.
    #[must_use]
    pub(crate) fn visible_clipping_refs(&self) -> Vec<&Clipping> {
        let base: Vec<&Clipping> = match self.overlay_source {
            OverlaySource::Clipboard => self.history.items.iter().collect(),
            OverlaySource::SequentialQueue => self.history.sequential_items(),
            OverlaySource::Pinned => self.history.items.iter().filter(|c| c.pinned).collect(),
        };
        if self.search_query.trim().is_empty() {
            base
        } else {
            base.into_iter()
                .filter(|c| c.matches_query(&self.search_query))
                .collect()
        }
    }

    /// Returns the number of visible clippings without cloning any items.
    #[must_use]
    pub(crate) fn visible_count(&self) -> usize {
        self.visible_clipping_refs().len()
    }

    /// Returns the visible clipping at `index` by reference without cloning the whole list.
    #[must_use]
    pub(crate) fn visible_at(&self, index: usize) -> Option<&Clipping> {
        self.visible_clipping_refs().get(index).copied()
    }

    /// Returns the stable UUIDs of all visible clippings for `virtual_list::keys`.
    #[must_use]
    pub(crate) fn visible_ids(&self) -> Vec<Uuid> {
        self.visible_clipping_refs().iter().map(|c| c.id).collect()
    }

    /// Returns the inclusive `(start, end)` index range of currently selected overlay rows.
    #[must_use]
    pub(crate) fn selected_range(&self) -> (usize, usize) {
        let anchor = self.selection_anchor.unwrap_or(self.selected_index);
        (
            anchor.min(self.selected_index),
            anchor.max(self.selected_index),
        )
    }

    /// Returns the currently focused clipping in the overlay, if any.
    #[must_use]
    pub(crate) fn selected_clipping(&self) -> Option<Clipping> {
        self.visible_at(self.selected_index).cloned()
    }

    /// Returns all currently selected clippings in list order (1 item normally, N items with Shift-select).
    #[must_use]
    pub(crate) fn selected_clippings(&self) -> Vec<Clipping> {
        let items = self.visible_clipping_refs();
        if items.is_empty() {
            return Vec::new();
        }
        let (lo, hi) = self.selected_range();
        let end = hi.min(items.len() - 1);
        let start = lo.min(end);
        items[start..=end].iter().map(|c| (*c).clone()).collect()
    }
}

type ThumbnailEntry = Option<(u32, u32, nsh_core::element::ImageSrc)>;

static THUMBNAIL_CACHE: OnceLock<Mutex<HashMap<Uuid, ThumbnailEntry>>> = OnceLock::new();

/// Decodes and caches a bounded RGBA thumbnail for an image clipping so PNG decoding runs at most once.
#[must_use]
pub(crate) fn cached_image_thumbnail(
    item: &Clipping,
    max_w: u32,
    max_h: u32,
) -> Option<(u32, u32, nsh_core::element::ImageSrc)> {
    let cache = THUMBNAIL_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock()
        && let Some(cached) = guard.get(&item.id)
    {
        return cached.clone();
    }
    let decoded = item
        .decode_bounded_rgba(max_w, max_h)
        .map(|(w, h, rgba)| (w, h, nsh_core::element::ImageSrc::rgba(w, h, rgba)));
    if let Ok(mut guard) = cache.lock() {
        if guard.len() > 256 {
            guard.clear();
        }
        guard.insert(item.id, decoded.clone());
    }
    decoded
}

enum BgEvent {
    Snapshot(ClipboardSnapshot),
}

struct Runtime {
    host: Arc<dyn PlatformHost>,
    ignore_next_self_write_until_token: Arc<AtomicU64>,
    self_write_guard: Arc<AtomicBool>,
    rx: Receiver<BgEvent>,
}

static LAUNCH_CONFIG: Mutex<Option<LaunchConfig>> = Mutex::new(None);
static RUNTIME: OnceLock<Mutex<Option<Runtime>>> = OnceLock::new();

thread_local! {
    static STORE: std::cell::RefCell<Option<Signal<AppState>>> = const { std::cell::RefCell::new(None) };
}

/// Configures the [`PlatformHost`] factory before mounting [`crate::App`].
pub fn configure_launch(config: LaunchConfig) {
    if let Ok(mut guard) = LAUNCH_CONFIG.lock() {
        *guard = Some(config);
    }
    if let Ok(mut rt) = RUNTIME.get_or_init(|| Mutex::new(None)).lock() {
        *rt = None;
    }
}

/// Initializes the reactive store and starts the background local clipboard monitor once.
pub(crate) fn ensure_started() {
    let already_init = STORE.with(|cell| cell.borrow().is_some());
    if already_init {
        return;
    }

    let config = LAUNCH_CONFIG
        .lock()
        .ok()
        .and_then(|g| *g)
        .unwrap_or_else(|| LaunchConfig::new(clipboard_platform::native_platform));

    let host: Arc<dyn PlatformHost> = Arc::from((config.platform_factory)());
    let settings = AppSettings::load_from_dir(&host.config_dir()).unwrap_or_default();
    let history = ClipboardHistory::load_from_dir(&host.data_dir()).unwrap_or_default();
    let installed_apps = host.list_installed_apps();

    let (tx, rx): (Sender<BgEvent>, Receiver<BgEvent>) = crossbeam_channel::unbounded();
    let self_write_guard = Arc::new(AtomicBool::new(false));
    let ignore_token = Arc::new(AtomicU64::new(0));

    if config.enable_background_polling {
        let poll_host = Arc::clone(&host);
        let guard_flag = Arc::clone(&self_write_guard);
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(120));
                if guard_flag.swap(false, Ordering::SeqCst) {
                    let _ = poll_host.read_clipboard();
                    continue;
                }
                if let Ok(Some(snapshot)) = poll_host.read_clipboard()
                    && tx.send(BgEvent::Snapshot(snapshot)).is_err()
                {
                    break;
                }
            }
        });
    }

    let initial = AppState {
        active_tab: PrefTab::General,
        settings,
        history,
        overlay_open: false,
        overlay_source: OverlaySource::Clipboard,
        search_query: String::new(),
        selected_index: 0,
        selection_anchor: None,
        search_focus_seq: 0,
        list_focused: false,
        preview_open: false,
        filter_picker_open: false,
        pending_delete_id: None,
        previous_app_target: None,
        sequential_mode_active: false,
        recording_shortcut: None,
        selected_blacklist_index: Some(0),
        add_blacklist_open: false,
        custom_blacklist_draft: String::new(),
        installed_apps,
        status_message: format!("Local only · {}", host.platform_name()),
        shortcuts_revision: 1,
        list_data_revision: 1,
    };

    let sig = signal_with(initial, SignalEquals::Never);
    STORE.with(|cell| {
        *cell.borrow_mut() = Some(sig);
    });

    if let Ok(mut rt_slot) = RUNTIME.get_or_init(|| Mutex::new(None)).lock() {
        *rt_slot = Some(Runtime {
            host,
            ignore_next_self_write_until_token: ignore_token,
            self_write_guard,
            rx,
        });
    }

    if config.enable_background_polling {
        set_interval(Duration::from_millis(50), drain_background_events);
    }
}

/// Returns the thread-local reactive store signal.
pub(crate) fn store_signal() -> Signal<AppState> {
    ensure_started();
    STORE.with(|cell| {
        cell.borrow().unwrap_or_else(|| {
            signal_with(
                AppState {
                    active_tab: PrefTab::General,
                    settings: AppSettings::default(),
                    history: ClipboardHistory::new(),
                    overlay_open: false,
                    overlay_source: OverlaySource::Clipboard,
                    search_query: String::new(),
                    selected_index: 0,
                    selection_anchor: None,
                    search_focus_seq: 0,
                    list_focused: false,
                    preview_open: false,
                    filter_picker_open: false,
                    pending_delete_id: None,
                    previous_app_target: None,
                    sequential_mode_active: false,
                    recording_shortcut: None,
                    selected_blacklist_index: None,
                    add_blacklist_open: false,
                    custom_blacklist_draft: String::new(),
                    installed_apps: Vec::new(),
                    status_message: String::new(),
                    shortcuts_revision: 1,
                    list_data_revision: 1,
                },
                SignalEquals::Never,
            )
        })
    })
}

fn with_host<R>(f: impl FnOnce(&dyn PlatformHost) -> R) -> Option<R> {
    let host = RUNTIME
        .get()?
        .lock()
        .ok()?
        .as_ref()
        .map(|rt| Arc::clone(&rt.host))?;
    Some(f(host.as_ref()))
}

fn mark_self_write() {
    if let Some(slot) = RUNTIME.get()
        && let Ok(guard) = slot.lock()
        && let Some(rt) = guard.as_ref()
    {
        rt.self_write_guard.store(true, Ordering::SeqCst);
        rt.ignore_next_self_write_until_token
            .fetch_add(1, Ordering::SeqCst);
    }
}

fn drain_background_events() {
    let events: Vec<BgEvent> = RUNTIME
        .get()
        .and_then(|slot| slot.lock().ok())
        .and_then(|guard| {
            let rt = guard.as_ref()?;
            let mut out = Vec::new();
            while let Ok(ev) = rt.rx.try_recv() {
                out.push(ev);
            }
            Some(out)
        })
        .unwrap_or_default();

    for ev in events {
        match ev {
            BgEvent::Snapshot(snapshot) => ingest_snapshot(snapshot),
        }
    }
}

/// Polls the platform host once immediately (useful in tests and manual refresh).
pub fn poll_clipboard_now() {
    if let Some(Ok(Some(snapshot))) = with_host(|h| h.read_clipboard()) {
        ingest_snapshot(snapshot);
    }
}

/// Returns `true` when running as the real desktop app (not in headless tests).
pub(crate) fn is_background_polling_enabled() -> bool {
    LAUNCH_CONFIG
        .lock()
        .ok()
        .and_then(|g| *g)
        .is_none_or(|c| c.enable_background_polling)
}

/// Ingests a captured [`ClipboardSnapshot`] into the store and persists history locally.
///
/// Copy ingestion is intentionally silent (never plays a sound).
pub(crate) fn ingest_snapshot(snapshot: ClipboardSnapshot) {
    store_signal().update(|s| {
        let outcome = s
            .history
            .ingest(snapshot, &s.settings, s.sequential_mode_active);
        match outcome {
            IngestOutcome::Recorded => {
                s.status_message = format!("{} clippings stored locally", s.history.items.len());
                s.list_data_revision = s.list_data_revision.saturating_add(1);
                persist_history(&s.history);
            }
            IngestOutcome::IgnoredTransient => {
                "Ignored transient sensitive clipping".clone_into(&mut s.status_message);
            }
            IngestOutcome::IgnoredBlacklisted => {
                "Ignored clipping from blacklisted app".clone_into(&mut s.status_message);
            }
            IngestOutcome::IgnoredTooLarge => {
                s.status_message = format!(
                    "Ignored clipping larger than {} MB",
                    s.settings.max_clipping_mb
                );
            }
            _ => {}
        }
    });
}

fn persist_settings(settings: &AppSettings) {
    let _ = with_host(|h| {
        if let Err(err) = settings.save_to_dir(&h.config_dir()) {
            log::warn!("failed to save settings: {err}");
        }
    });
}

fn persist_history(history: &ClipboardHistory) {
    let _ = with_host(|h| {
        if let Err(err) = history.save_to_dir(&h.data_dir()) {
            log::warn!("failed to save history: {err}");
        }
    });
}

// MARK: - Overlay Actions

/// Opens or toggles the floating Quick Paste overlay.
pub(crate) fn toggle_overlay() {
    let is_open = store_signal().with(|s| s.overlay_open);
    if is_open {
        close_overlay();
    } else {
        open_overlay();
    }
}

/// Opens the floating Quick Paste overlay, captures the previously focused app, and focuses search.
pub(crate) fn open_overlay() {
    let prev_target = if is_background_polling_enabled() {
        with_host(|h| h.capture_frontmost_target()).flatten()
    } else {
        None
    };
    log::info!("open_overlay: captured previous_app_target={prev_target:?}");
    store_signal().update(|s| {
        s.overlay_open = true;
        s.search_query.clear();
        s.selected_index = 0;
        s.selection_anchor = None;
        s.list_focused = false;
        s.filter_picker_open = false;
        s.pending_delete_id = None;
        s.previous_app_target = prev_target;
        s.list_data_revision = s.list_data_revision.saturating_add(1);
        s.search_focus_seq = s.search_focus_seq.saturating_add(1);
    });
}

/// Closes the floating Quick Paste overlay and resets transient overlay drawers.
pub(crate) fn close_overlay() {
    store_signal().update(|s| {
        s.overlay_open = false;
        s.filter_picker_open = false;
        s.preview_open = false;
        s.pending_delete_id = None;
        s.list_focused = false;
    });
}

/// Closes the overlay without pasting and restores focus to the previously active application.
pub(crate) fn dismiss_overlay_and_restore_focus() {
    let target = store_signal().with(|s| s.previous_app_target.clone());
    close_overlay();
    if is_background_polling_enabled()
        && let Some(host) = RUNTIME
            .get()
            .and_then(|slot| slot.lock().ok())
            .and_then(|guard| guard.as_ref().map(|rt| Arc::clone(&rt.host)))
    {
        thread::spawn(move || {
            host.restore_focus(target.as_deref());
        });
    }
}

/// Cycles the overlay source (`Clipboard` -> `Sequential Queue` -> `Pinned`).
pub(crate) fn cycle_overlay_source() {
    store_signal().update(|s| {
        s.overlay_source = s.overlay_source.next();
        s.selected_index = 0;
        s.selection_anchor = None;
        s.list_data_revision = s.list_data_revision.saturating_add(1);
    });
}

/// Updates the overlay search query and resets selection to the first matching row.
pub(crate) fn set_search_query(query: &str) {
    store_signal().update(|s| {
        query.clone_into(&mut s.search_query);
        s.selected_index = 0;
        s.selection_anchor = None;
        s.list_focused = false;
        s.list_data_revision = s.list_data_revision.saturating_add(1);
    });
}

/// Appends typed text when a printable key is pressed while navigating the list, refocusing search.
#[expect(dead_code, reason = "helper for programmatic search input injection")]
pub(crate) fn append_search_char(ch: &str) {
    store_signal().update(|s| {
        s.search_query.push_str(ch);
        s.selected_index = 0;
        s.list_focused = false;
        s.search_focus_seq = s.search_focus_seq.saturating_add(1);
    });
}

/// Removes the last character from the search query and refocuses the search input.
#[expect(dead_code, reason = "helper for programmatic search input editing")]
pub(crate) fn backspace_search() {
    store_signal().update(|s| {
        s.search_query.pop();
        s.selected_index = 0;
        s.list_focused = false;
        s.search_focus_seq = s.search_focus_seq.saturating_add(1);
    });
}

/// Moves the overlay selection by `delta` (`-1` for Up, `+1` for Down), clearing any multi-selection.
pub(crate) fn move_selection(delta: isize) {
    store_signal().update(|s| {
        let len = s.visible_count();
        if len == 0 {
            s.selected_index = 0;
            s.selection_anchor = None;
            return;
        }
        let next = if delta < 0 {
            s.selected_index.saturating_sub(delta.unsigned_abs())
        } else {
            (s.selected_index + delta.unsigned_abs()).min(len - 1)
        };
        s.selected_index = next;
        s.selection_anchor = None;
        s.list_focused = true;
    });
}

/// Extends the overlay selection range by `delta` while holding Shift (`Shift+Up` / `Shift+Down`).
pub(crate) fn extend_selection(delta: isize) {
    store_signal().update(|s| {
        let len = s.visible_count();
        if len == 0 {
            s.selected_index = 0;
            s.selection_anchor = None;
            return;
        }
        if s.selection_anchor.is_none() {
            s.selection_anchor = Some(s.selected_index);
        }
        let next = if delta < 0 {
            s.selected_index.saturating_sub(delta.unsigned_abs())
        } else {
            (s.selected_index + delta.unsigned_abs()).min(len - 1)
        };
        s.selected_index = next;
        s.list_focused = true;
    });
}

/// Directly selects a single row by index in the filtered overlay list.
pub(crate) fn select_index(index: usize) {
    store_signal().update(|s| {
        let len = s.visible_count();
        if len > 0 {
            s.selected_index = index.min(len - 1);
            s.selection_anchor = None;
            s.list_focused = true;
        }
    });
}

/// Toggles the inline preview drawer for the currently selected clipping.
pub(crate) fn toggle_preview() {
    store_signal().update(|s| {
        s.preview_open = !s.preview_open;
        if s.preview_open {
            s.filter_picker_open = false;
        }
    });
}

/// Toggles the filter picker drawer for the currently selected clipping.
pub(crate) fn toggle_filter_picker() {
    store_signal().update(|s| {
        s.filter_picker_open = !s.filter_picker_open;
        if s.filter_picker_open {
            s.preview_open = false;
        }
    });
}

/// Opens the overlay with the filter picker immediately expanded.
pub(crate) fn open_filter_window() {
    store_signal().update(|s| {
        s.overlay_open = true;
        s.filter_picker_open = true;
        s.preview_open = false;
    });
}

/// Pastes the currently selected clipping(s) in the overlay and closes the overlay.
///
/// When multiple items are Shift-selected, joins them with newlines and pastes all at once.
pub(crate) fn paste_selected(plain_override: bool) {
    let (selected, always_plain, play_sound) = store_signal().with(|s| {
        (
            s.selected_clippings(),
            s.settings.always_paste_plain_text,
            s.settings.enable_interaction_sounds,
        )
    });
    if selected.is_empty() {
        return;
    }
    close_overlay();
    if selected.len() == 1 {
        deliver_clipping(
            &selected[0],
            plain_override || always_plain,
            None,
            play_sound,
        );
    } else {
        let combined = combine_clippings_text(&selected, None);
        deliver_payload(ClipboardWrite::PlainText(combined), play_sound);
    }
}

/// Pastes the currently selected clipping(s) after applying `filter`.
pub(crate) fn paste_selected_with_filter(filter: TextFilter) {
    let (selected, play_sound) =
        store_signal().with(|s| (s.selected_clippings(), s.settings.enable_interaction_sounds));
    store_signal().update(|s| {
        s.settings.last_filter = filter;
        persist_settings(&s.settings);
    });
    if selected.is_empty() {
        return;
    }
    close_overlay();
    if selected.len() == 1 {
        deliver_clipping(&selected[0], true, Some(filter), play_sound);
    } else {
        let combined = combine_clippings_text(&selected, Some(filter));
        deliver_payload(ClipboardWrite::PlainText(combined), play_sound);
    }
}

fn combine_clippings_text(items: &[Clipping], filter: Option<TextFilter>) -> String {
    items
        .iter()
        .map(|item| filter.map_or_else(|| item.text.clone(), |f| f.apply(&item.text)))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Pastes the most recent clipping using the last chosen [`TextFilter`].
pub(crate) fn paste_with_last_filter() {
    let (clipping, filter, play_sound) = store_signal().with(|s| {
        (
            s.history.items.first().cloned(),
            s.settings.last_filter,
            s.settings.enable_interaction_sounds,
        )
    });
    if let Some(item) = clipping {
        deliver_clipping(&item, true, Some(filter), play_sound);
    }
}

/// Pastes the 1-based recent clipping slot (`1..=10`).
pub(crate) fn paste_recent_slot(slot_one_based: u8) {
    let idx = usize::from(slot_one_based.saturating_sub(1));
    let (clipping, always_plain, play_sound) = store_signal().with(|s| {
        (
            s.history.items.get(idx).cloned(),
            s.settings.always_paste_plain_text,
            s.settings.enable_interaction_sounds,
        )
    });
    if let Some(item) = clipping {
        deliver_clipping(&item, always_plain, None, play_sound);
    }
}

/// Toggles Sequential Paste Mode (`⌃⌘C`).
pub(crate) fn toggle_sequential_mode() {
    store_signal().update(|s| {
        s.sequential_mode_active = !s.sequential_mode_active;
        if s.sequential_mode_active {
            "Sequential Paste Mode active".clone_into(&mut s.status_message);
        } else {
            "Sequential Paste Mode off".clone_into(&mut s.status_message);
        }
    });
}

/// Pops and pastes the next item from the Sequential Paste Queue (`⌃⌘V`).
pub(crate) fn paste_from_sequential_queue() {
    let mut popped: Option<Clipping> = None;
    let mut always_plain = false;
    let mut play_sound = false;
    store_signal().update(|s| {
        popped = s.history.pop_sequential();
        always_plain = s.settings.always_paste_plain_text;
        play_sound = s.settings.enable_interaction_sounds;
        if popped.is_some() {
            persist_history(&s.history);
        }
    });
    if let Some(item) = popped {
        deliver_clipping(&item, always_plain, None, play_sound);
    } else if play_sound {
        with_host(|h| h.play_sound(SoundEffect::Error));
    }
}

/// Toggles pinned status on a clipping.
pub(crate) fn toggle_pin(id: Uuid) {
    store_signal().update(|s| {
        if s.history.toggle_pin(id) {
            persist_history(&s.history);
        }
    });
}

/// Opens the delete confirmation prompt for the currently selected clipping.
pub(crate) fn request_delete_selected() {
    store_signal().update(|s| {
        if let Some(item) = s.selected_clipping() {
            s.pending_delete_id = Some(item.id);
            s.filter_picker_open = false;
        }
    });
}

/// Cancels the pending delete confirmation prompt without removing anything.
pub(crate) fn cancel_delete_pending() {
    store_signal().update(|s| {
        s.pending_delete_id = None;
    });
}

/// Confirms and removes the clipping awaiting deletion.
pub(crate) fn confirm_delete_pending() {
    let pending = store_signal().with(|s| s.pending_delete_id);
    if let Some(id) = pending {
        remove_clipping(id);
        store_signal().update(|s| {
            s.pending_delete_id = None;
        });
    }
}

/// Deletes a single clipping by ID.
pub(crate) fn remove_clipping(id: Uuid) {
    store_signal().update(|s| {
        if s.history.remove(id) {
            let len = s.visible_count();
            if s.selected_index >= len && len > 0 {
                s.selected_index = len - 1;
            }
            s.selection_anchor = None;
            s.list_data_revision = s.list_data_revision.saturating_add(1);
            persist_history(&s.history);
        }
    });
}

/// Clears all unpinned history clippings.
pub(crate) fn clear_history() {
    store_signal().update(|s| {
        s.history.clear_unpinned();
        s.selected_index = 0;
        s.selection_anchor = None;
        s.list_data_revision = s.list_data_revision.saturating_add(1);
        "Clipboard history cleared".clone_into(&mut s.status_message);
        persist_history(&s.history);
    });
}

fn deliver_clipping(
    clipping: &Clipping,
    plain_text: bool,
    filter: Option<TextFilter>,
    play_sound: bool,
) {
    let payload = match clipping.kind {
        ClippingKind::Text => {
            let text = filter.map_or_else(|| clipping.text.clone(), |f| f.apply(&clipping.text));
            if plain_text || filter.is_some() {
                ClipboardWrite::PlainText(text)
            } else {
                ClipboardWrite::RichText {
                    text,
                    html: clipping.html.clone(),
                }
            }
        }
        ClippingKind::Image => {
            let Some(png_bytes) = clipping.image_png_bytes() else {
                return;
            };
            ClipboardWrite::ImagePng(png_bytes)
        }
        _ => return,
    };

    deliver_payload(payload, play_sound);
}

fn deliver_payload(payload: ClipboardWrite, play_sound: bool) {
    let target_app = store_signal().with(|s| s.previous_app_target.clone());

    mark_self_write();
    let Some(host) = RUNTIME
        .get()
        .and_then(|slot| slot.lock().ok())
        .and_then(|guard| guard.as_ref().map(|rt| Arc::clone(&rt.host)))
    else {
        return;
    };

    // Perform clipboard write, restore focus to original app, and simulate paste off the UI thread.
    // Pasting is intentionally silent (never plays a sound).
    let _ = play_sound;
    thread::spawn(move || {
        log::info!("deliver_clipping: restoring focus to target={target_app:?} and pasting");
        let res = host.write_clipboard(&payload).and_then(|()| {
            host.restore_focus(target_app.as_deref());
            host.simulate_paste()
        });
        if let Err(ref err) = res {
            log::warn!("paste failed: {err}");
        } else {
            log::info!("deliver_clipping: paste succeeded");
        }
    });
    store_signal().update(|s| {
        "Pasted clipping".clone_into(&mut s.status_message);
    });
}

/// Returns the mouse cursor position from the active [`PlatformHost`] when available.
#[expect(dead_code, reason = "reserved for cursor-anchored positioning")]
pub(crate) fn mouse_cursor_position() -> Option<clipboard_platform::CursorPoint> {
    with_host(|h| h.mouse_cursor_position()).flatten()
}

// MARK: - Preferences Actions

/// Switches the active Preferences tab.
pub(crate) fn set_tab(tab: PrefTab) {
    store_signal().update(|s| {
        s.active_tab = tab;
        s.recording_shortcut = None;
    });
}

/// Updates `Ignore clippings larger than X MB`.
pub(crate) fn set_max_clipping_mb_str(raw: &str) {
    let digits: String = raw.chars().filter(char::is_ascii_digit).collect();
    let Ok(val) = digits.parse::<u32>() else {
        return;
    };
    let clamped = val.clamp(1, 512);
    store_signal().update(|s| {
        s.settings.max_clipping_mb = clamped;
        persist_settings(&s.settings);
    });
}

/// Updates the history retention limit (`100..=1000`) and prunes excess unpinned items.
pub(crate) fn set_history_limit(limit: usize) {
    let clamped = limit.clamp(
        AppSettings::MIN_HISTORY_LIMIT,
        AppSettings::MAX_HISTORY_LIMIT,
    );
    store_signal().update(|s| {
        s.settings.history_limit = clamped;
        s.history.enforce_limit(clamped);
        persist_settings(&s.settings);
        persist_history(&s.history);
    });
}

/// Sets `Always Paste Plain Text`.
pub(crate) fn set_always_paste_plain_text(enabled: bool) {
    store_signal().update(|s| {
        s.settings.always_paste_plain_text = enabled;
        persist_settings(&s.settings);
    });
}

/// Sets the Quick Paste Menu style.
pub(crate) fn set_quick_paste_menu(style: QuickPasteMenuStyle) {
    store_signal().update(|s| {
        s.settings.quick_paste_menu = style;
        s.list_data_revision = s.list_data_revision.saturating_add(1);
        persist_settings(&s.settings);
    });
}

/// Sets `Appear Behind Mouse Cursor`.
pub(crate) fn set_appear_behind_mouse_cursor(enabled: bool) {
    store_signal().update(|s| {
        s.settings.appear_behind_mouse_cursor = enabled;
        persist_settings(&s.settings);
    });
}

/// Sets `Release to Paste`.
pub(crate) fn set_release_to_paste(enabled: bool) {
    store_signal().update(|s| {
        s.settings.release_to_paste = enabled;
        persist_settings(&s.settings);
    });
}

/// Sets `Enable Interaction Sounds`.
pub(crate) fn set_enable_interaction_sounds(enabled: bool) {
    store_signal().update(|s| {
        s.settings.enable_interaction_sounds = enabled;
        persist_settings(&s.settings);
    });
}

/// Sets `Dock and Menubar Icon` mode.
pub(crate) fn set_dock_menubar_mode(mode: DockMenubarMode) {
    store_signal().update(|s| {
        s.settings.dock_menubar_mode = mode;
        persist_settings(&s.settings);
    });
}

/// Sets `Open automatically at login` and applies it through [`PlatformHost`].
pub(crate) fn set_open_at_login(enabled: bool) {
    let result = with_host(|h| h.set_launch_at_login(enabled));
    store_signal().update(|s| {
        s.settings.open_at_login = enabled;
        if let Some(Err(err)) = result {
            s.status_message = err.to_string();
        }
        persist_settings(&s.settings);
    });
}

// MARK: - Shortcuts Actions

/// Begins live key capture for `action`.
pub(crate) fn start_recording_shortcut(action: ShortcutAction) {
    store_signal().update(|s| {
        s.recording_shortcut = Some(action);
    });
}

/// Cancels live key capture.
pub(crate) fn cancel_recording_shortcut() {
    store_signal().update(|s| {
        s.recording_shortcut = None;
    });
}

/// Assigns `shortcut` to `action`, persists settings, and triggers global hotkey re-sync.
pub(crate) fn apply_shortcut(action: ShortcutAction, shortcut: KeyboardShortcut) {
    store_signal().update(|s| {
        s.settings.set_shortcut(action, shortcut);
        s.recording_shortcut = None;
        s.shortcuts_revision = s.shortcuts_revision.saturating_add(1);
        persist_settings(&s.settings);
    });
}

/// Clears the shortcut for `action`.
pub(crate) fn clear_shortcut(action: ShortcutAction) {
    apply_shortcut(action, KeyboardShortcut::empty());
}

// MARK: - Blacklist Actions

/// Selects a blacklist row by index.
pub(crate) fn select_blacklist_index(index: usize) {
    store_signal().update(|s| {
        s.selected_blacklist_index = Some(index);
    });
}

/// Toggles the Add Application picker on the Blacklist screen.
pub(crate) fn toggle_add_blacklist_picker() {
    store_signal().update(|s| {
        s.add_blacklist_open = !s.add_blacklist_open;
        if s.add_blacklist_open {
            s.custom_blacklist_draft.clear();
        }
    });
}

/// Updates the custom application name input in the Add Blacklist picker.
pub(crate) fn set_custom_blacklist_draft(value: &str) {
    store_signal().update(|s| {
        value.clone_into(&mut s.custom_blacklist_draft);
    });
}

/// Adds an application to the blacklist and persists settings.
pub(crate) fn add_blacklist_entry(entry: BlacklistEntry) {
    store_signal().update(|s| {
        if s.settings.add_to_blacklist(entry) {
            let last = s.settings.blacklist.len().saturating_sub(1);
            s.selected_blacklist_index = Some(last);
            persist_settings(&s.settings);
        }
        s.add_blacklist_open = false;
        s.custom_blacklist_draft.clear();
    });
}

/// Removes the currently selected blacklist entry.
pub(crate) fn remove_selected_blacklist_entry() {
    store_signal().update(|s| {
        if let Some(idx) = s.selected_blacklist_index
            && s.settings.remove_from_blacklist(idx)
        {
            s.selected_blacklist_index = if s.settings.blacklist.is_empty() {
                None
            } else {
                Some(idx.min(s.settings.blacklist.len() - 1))
            };
            persist_settings(&s.settings);
        }
    });
}
