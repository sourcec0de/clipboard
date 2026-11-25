//! Styled history-count slider (`100..=1000`) built on [`nsh_base::slider`].

use nsh_base::SliderValue;
use nsh_base::slider as base_slider;
use nsh_base::theme;
use nsh_core::prelude::IntoElement;
use nsh_core::prelude::ParentElement;
use nsh_core::prelude::Styled;
use nsh_core::prelude::px;
use nsh_core::prelude::text;
use nsh_core::prelude::view;

use crate::ui::tokens;

/// Stepped slider for configuring clipboard history retention (`100` to `1000` clippings).
pub(crate) fn history_slider(
    value: impl Fn() -> Option<SliderValue> + 'static,
    on_change: impl Fn(&SliderValue) + 'static,
) -> impl IntoElement {
    view()
        .flex_1()
        .gap_1()
        .child(
            base_slider()
                .accessibility_label("History clipping limit")
                .min(100.0)
                .max(1000.0)
                .step(100.0)
                .value(value)
                .on_change(on_change)
                .track_style(|s| s.h(px(22.0)).justify_center())
                .indicator_style(|s| s.h(px(4.0)).rounded_full().bg(move || theme().input))
                .range_style(|s| s.rounded_full().bg(move || theme().primary))
                .thumb_style(|s| {
                    s.w(px(12.0))
                        .h(px(18.0))
                        .rounded_full()
                        .bg(move || theme().foreground)
                        .border_1()
                        .border_color(move || theme().border)
                        .shadow_sm()
                        .transition_colors()
                        .duration(tokens::DURATION_FAST)
                }),
        )
        .child(
            view()
                .flex_row()
                .justify_between()
                .child(
                    text("100")
                        .text_xs()
                        .tabular_nums()
                        .text_color(move || theme().muted_foreground),
                )
                .child(
                    text("500")
                        .text_xs()
                        .tabular_nums()
                        .text_color(move || theme().muted_foreground),
                )
                .child(
                    text("1000")
                        .text_xs()
                        .tabular_nums()
                        .text_color(move || theme().muted_foreground),
                ),
        )
}
