//! Semantic theme tokens, radii, typography helpers, and motion constants.
//!
//! All colors resolve from `theme.json` via [`nsh_base::theme`] so light, dark,
//! and custom host themes update natively without raw color literals.

use nsh_base::Theme;
use nsh_base::theme;
use nsh_core::element::AdaptiveColor;
use nsh_core::element::Tokens;
use nsh_core::prelude::Points;
use nsh_core::prelude::px;

/// Embedded `theme.json` containing semantic light and dark color tokens.
const THEME_JSON: &str = include_str!("../../../../theme.json");

/// Parses the embedded `theme.json` into NSH [`Tokens`] for app startup and tests.
#[must_use]
pub fn theme_tokens() -> Tokens {
    Theme::from_json(THEME_JSON).map_or_else(|_| Theme::default().into_tokens(), Theme::into_tokens)
}

// MARK: - Motion Tokens

/// Hover wash and quick color transitions (120 ms).
pub(crate) const DURATION_INSTANT: f32 = 120.0;
/// Press scale, checkbox fill, and control state transitions (180 ms).
pub(crate) const DURATION_FAST: f32 = 180.0;
/// Selection highlight and drawer transitions (240 ms).
#[expect(dead_code, reason = "design-system motion token")]
pub(crate) const DURATION_BASE: f32 = 240.0;
/// Standard button active press scale.
pub(crate) const PRESS_SCALE: f32 = 0.97;
/// Square icon button active press scale.
pub(crate) const ICON_PRESS_SCALE: f32 = 0.95;

// MARK: - Radius Tokens

/// Control corner radius (`radius`, default 7 pt).
#[must_use]
pub(crate) fn radius_control() -> Points {
    theme().radius
}

/// Card / panel corner radius (`radius-card`, default 10 pt).
#[must_use]
#[expect(dead_code, reason = "design-system radius token")]
pub(crate) fn radius_card() -> Points {
    px(theme().number("radius-card").unwrap_or(10.0))
}

/// Inner badge / checkbox corner radius (`radius-inset`, default 5 pt).
#[must_use]
pub(crate) fn radius_inset() -> Points {
    px(theme().number("radius-inset").unwrap_or(5.0))
}

/// Floating Quick Paste overlay corner radius (`radius-overlay`, default 12 pt).
#[must_use]
pub(crate) fn radius_overlay() -> Points {
    px(theme().number("radius-overlay").unwrap_or(12.0))
}

// MARK: - Custom Semantic Color Roles

/// Top preference toolbar background.
#[must_use]
pub(crate) fn toolbar() -> AdaptiveColor {
    theme().color("toolbar").unwrap_or(theme().secondary)
}

/// Active preference toolbar tab pill background.
#[must_use]
pub(crate) fn toolbar_active() -> AdaptiveColor {
    theme().color("toolbar-active").unwrap_or(theme().accent)
}

/// Alternating table row fill.
#[must_use]
pub(crate) fn row_alt() -> AdaptiveColor {
    theme().color("row-alt").unwrap_or(theme().muted)
}

/// Grouped table section header fill (e.g. `"Actions"`, `"Paste Recent Clippings"`).
#[must_use]
pub(crate) fn section_header_bg() -> AdaptiveColor {
    theme().color("section-header").unwrap_or(theme().secondary)
}

/// Dark/light capsule background for shortcut recorder pills.
#[must_use]
pub(crate) fn pill_surface() -> AdaptiveColor {
    theme().color("pill-surface").unwrap_or(theme().secondary)
}

/// Accent circle inside shortcut recorder pills (`+` / `×`).
#[must_use]
pub(crate) fn pill_accent() -> AdaptiveColor {
    theme().color("pill-accent").unwrap_or(theme().primary)
}

/// Positive status accent.
#[must_use]
pub(crate) fn success() -> AdaptiveColor {
    theme().color("success").unwrap_or(theme().primary)
}

/// Soft surface for positive badges.
#[must_use]
pub(crate) fn success_surface() -> AdaptiveColor {
    theme().color("success-surface").unwrap_or(theme().muted)
}

/// Warning status accent.
#[must_use]
pub(crate) fn warning() -> AdaptiveColor {
    theme().color("warning").unwrap_or(theme().destructive)
}

/// Soft surface for warning badges.
#[must_use]
pub(crate) fn warning_surface() -> AdaptiveColor {
    theme().color("warning-surface").unwrap_or(theme().muted)
}

/// Soft surface for destructive buttons/alerts.
#[must_use]
pub(crate) fn destructive_surface() -> AdaptiveColor {
    theme()
        .color("destructive-surface")
        .unwrap_or(theme().muted)
}

/// Returns the fixed-width monospace font family appropriate for the target OS.
#[must_use]
pub(crate) const fn mono_font() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "Menlo"
    }
    #[cfg(target_os = "windows")]
    {
        "Consolas"
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        "monospace"
    }
}
