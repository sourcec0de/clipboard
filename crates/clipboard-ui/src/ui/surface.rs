//! Surfaces, preference toolbar tabs, dividers, badges, and empty states.

use nsh_base::button as base_button;
use nsh_base::theme;
use nsh_core::element::ComponentBuilder;
use nsh_core::prelude::FontWeight;
use nsh_core::prelude::IntoElement;
use nsh_core::prelude::IntoReactive;
use nsh_core::prelude::ParentElement;
use nsh_core::prelude::SharedString;
use nsh_core::prelude::Styled;
use nsh_core::prelude::px;
use nsh_core::prelude::text;
use nsh_core::prelude::view;

use crate::ui::tokens;

/// Semantic tone for [`badge`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BadgeTone {
    /// Quiet neutral metadata badge.
    Neutral,
    /// Local-only / ready / active status.
    Success,
    /// Sequential mode / warning notice.
    Warning,
}

/// Single hairline horizontal separator.
pub(crate) fn divider() -> impl IntoElement {
    view().h(px(1.0)).bg(move || theme().border)
}

/// Top macOS-style Preferences toolbar tab (`General`, `Shortcuts`, `Blacklist`).
pub(crate) fn toolbar_tab(
    icon_svg: &'static str,
    label: &'static str,
    is_active: impl Fn() -> bool + Copy + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    base_button()
        .accessibility_label(label)
        .on_press(on_press)
        .min_w(px(72.0))
        .px_3()
        .py_1p5()
        .rounded(tokens::radius_control)
        .items_center()
        .justify_center()
        .gap_1()
        .bg(move || {
            if is_active() {
                tokens::toolbar_active()
            } else {
                tokens::toolbar().opacity(0.0)
            }
        })
        .hover(|s| s.bg(move || tokens::toolbar_active().opacity(0.65)))
        .transition_colors()
        .duration(tokens::DURATION_INSTANT)
        .focus_visible(|s| s.outline(px(2.0)).outline_color(move || theme().ring))
        .child(crate::ui::icons::vector_icon(icon_svg, 20.0, move || {
            if is_active() {
                theme().primary
            } else {
                theme().muted_foreground
            }
        }))
        .child(
            text(label)
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || {
                    if is_active() {
                        theme().primary
                    } else {
                        theme().muted_foreground
                    }
                }),
        )
}

/// Compact semantic status badge.
pub(crate) fn badge(
    label: impl IntoReactive<SharedString>,
    tone: impl Fn() -> BadgeTone + Copy + 'static,
) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .px_2()
        .py_0p5()
        .rounded(tokens::radius_inset)
        .bg(move || match tone() {
            BadgeTone::Neutral => theme().muted,
            BadgeTone::Success => tokens::success_surface(),
            BadgeTone::Warning => tokens::warning_surface(),
        })
        .child(
            text(label)
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || match tone() {
                    BadgeTone::Neutral => theme().muted_foreground,
                    BadgeTone::Success => tokens::success(),
                    BadgeTone::Warning => tokens::warning(),
                }),
        )
}

/// Centered empty state with headline and guidance.
pub(crate) fn empty_state(title: &'static str, detail: &'static str) -> impl IntoElement {
    view()
        .py_8()
        .px_4()
        .items_center()
        .justify_center()
        .gap_1()
        .child(
            text(title)
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || theme().foreground),
        )
        .child(
            text(detail)
                .text_xs()
                .text_color(move || theme().muted_foreground),
        )
}
