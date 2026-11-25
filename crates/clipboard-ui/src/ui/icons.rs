//! Crisp, pixel-aligned vector SVG icons for the Quick Paste overlay, Preferences toolbar, and tray.

use nsh_core::element::ImageSrc;
use nsh_core::prelude::AdaptiveColor;
use nsh_core::prelude::IntoElement;
use nsh_core::prelude::Styled;
use nsh_core::prelude::px;
use nsh_core::prelude::svg;

/// Monochrome template clipboard icon for the macOS/Linux/Windows system status bar.
pub(crate) const TRAY_CLIPBOARD_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 18 18" width="18" height="18"><rect x="3.5" y="3.2" width="11" height="12.6" rx="2" fill="none" stroke="black" stroke-width="1.6"/><rect x="6.5" y="1.6" width="5" height="2.8" rx="1" fill="black"/><line x1="6.2" y1="8.2" x2="11.8" y2="8.2" stroke="black" stroke-width="1.4" stroke-linecap="round"/><line x1="6.2" y1="11.4" x2="10.4" y2="11.4" stroke="black" stroke-width="1.4" stroke-linecap="round"/></svg>"#;

/// Settings gear icon (overlay top-left).
pub(crate) const GEAR_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3.2"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h.01a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v.01a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>"#;

/// Dual up/down chevron next to `"Clipboard"` in the overlay header.
pub(crate) const CHEVRONS_UP_DOWN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="m7 15 5 5 5-5"/><path d="m7 9 5-5 5 5"/></svg>"#;

/// Search magnifying glass inside the overlay search bar.
pub(crate) const SEARCH_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="7.5"/><path d="m20 20-3.8-3.8"/></svg>"#;

/// Filter funnel icon (reserved for filter window trigger).
#[expect(dead_code, reason = "design-system filter icon")]
pub(crate) const FILTER_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><polygon points="3 4 21 4 14 12.8 14 19 10 21 10 12.8 3 4"/></svg>"#;

/// Share / Preview tray-with-up-arrow icon shown on the selected overlay row (matches Screenshot 5).
pub(crate) const SHARE_PREVIEW_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v12"/><path d="m7.5 7.5 4.5-4.5 4.5 4.5"/><path d="M5 12v7a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-7"/></svg>"#;

/// Preferences toolbar icon: General (switch panel).
pub(crate) const TAB_GENERAL_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="3" width="16" height="18" rx="3"/><rect x="9.5" y="7" width="5" height="10" rx="2.5"/><line x1="9.5" y1="12" x2="14.5" y2="12"/></svg>"#;

/// Preferences toolbar icon: Shortcuts (Command loop).
pub(crate) const TAB_SHORTCUTS_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M18 3a3 3 0 0 0-3 3v12a3 3 0 0 0 3 3 3 3 0 0 0 3-3 3 3 0 0 0-3-3H6a3 3 0 0 0-3 3 3 3 0 0 0 3 3 3 3 0 0 0 3-3V6a3 3 0 0 0-3-3 3 3 0 0 0-3 3 3 3 0 0 0 3 3h12a3 3 0 0 0 3-3 3 3 0 0 0-3-3z"/></svg>"#;

/// Preferences toolbar icon: Blacklist (prohibited circle).
pub(crate) const TAB_BLACKLIST_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"/><line x1="5.6" y1="18.4" x2="18.4" y2="5.6"/></svg>"#;

/// Renders a crisp vector SVG icon tinted with a reactive semantic theme color.
pub(crate) fn vector_icon(
    markup: &'static str,
    size: f32,
    tint: impl Fn() -> AdaptiveColor + 'static,
) -> impl IntoElement {
    svg(ImageSrc::svg_markup(markup))
        .w(px(size))
        .h(px(size))
        .text_color(tint)
}
