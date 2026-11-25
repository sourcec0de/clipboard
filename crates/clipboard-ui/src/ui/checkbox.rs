//! Styled checkbox control built on [`nsh_base::checkbox`].

use nsh_base::checkbox as base_checkbox;
use nsh_base::theme;
use nsh_core::element::ComponentBuilder;
use nsh_core::element::IntoProp;
use nsh_core::prelude::FontWeight;
use nsh_core::prelude::IntoElement;
use nsh_core::prelude::ParentElement;
use nsh_core::prelude::Styled;
use nsh_core::prelude::px;
use nsh_core::prelude::text;
use nsh_core::prelude::view;
use nsh_core::prelude::when;

use crate::ui::tokens;

/// Controlled macOS-inspired checkbox row with optional helper description underneath.
pub(crate) fn checkbox_row<M>(
    label: &'static str,
    description: Option<&'static str>,
    checked: impl IntoProp<bool, M> + Copy + 'static,
    is_checked_fn: impl Fn() -> bool + Copy + 'static,
    on_change: impl Fn(&bool) + 'static,
) -> impl IntoElement {
    view()
        .gap_1()
        .child(
            base_checkbox()
                .accessibility_label(label)
                .checked(checked)
                .on_change(on_change)
                .flex_row()
                .items_center()
                .gap_2()
                .focus_visible(|s| s.outline(px(2.0)).outline_color(move || theme().ring))
                .indicator_style(|s| {
                    s.w(px(16.0))
                        .h(px(16.0))
                        .rounded(tokens::radius_inset)
                        .border_1()
                        .border_color(move || theme().input)
                        .bg(move || theme().secondary)
                        .items_center()
                        .justify_center()
                        .transition_colors()
                        .duration(tokens::DURATION_FAST)
                        .checked(|s| {
                            s.bg(move || theme().primary)
                                .border_color(move || theme().primary)
                        })
                })
                .indicator(when(is_checked_fn, || {
                    text("✓")
                        .text_xs()
                        .font_weight(FontWeight::BOLD)
                        .text_color(move || theme().primary_foreground)
                }))
                .child(text(label).text_sm().text_color(move || theme().foreground)),
        )
        .child(description.map(|desc| {
            view().pl(px(24.0)).child(
                text(desc)
                    .text_xs()
                    .text_color(move || theme().muted_foreground),
            )
        }))
}
