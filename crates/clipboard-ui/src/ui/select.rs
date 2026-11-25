//! Styled dropdown select control built on [`nsh_base::select`].

use nsh_base::SelectEntry;
use nsh_base::SelectItem;
use nsh_base::select as base_select;
use nsh_base::theme;
use nsh_core::prelude::FontWeight;
use nsh_core::prelude::IntoElement;
use nsh_core::prelude::ParentElement;
use nsh_core::prelude::SharedString;
use nsh_core::prelude::Styled;
use nsh_core::prelude::px;
use nsh_core::prelude::text;
use nsh_core::prelude::view;

use crate::ui::tokens;

/// macOS-styled dropdown select with trailing blue chevron cap.
pub(crate) fn dropdown_select(
    accessibility_label: &'static str,
    items: Vec<SelectItem>,
    selected_id: impl Fn() -> Option<SharedString> + 'static,
    selected_label: impl Fn() -> SharedString + 'static,
    on_change: impl Fn(&Option<SharedString>) + 'static,
) -> impl IntoElement {
    base_select()
        .accessibility_label(accessibility_label)
        .items(items)
        .value(selected_id)
        .on_change(on_change)
        .offset(px(4.0))
        .popup_style(|s| {
            s.p_1()
                .rounded(tokens::radius_control)
                .border_1()
                .border_color(move || theme().border)
                .bg(move || theme().popover)
                .shadow_md()
        })
        .item_class("rounded-md focus:bg-accent selected:font-medium disabled:opacity-50")
        .render_item(|entry: &SelectEntry| {
            view().px_2().py_1p5().child(
                text(entry.title.clone())
                    .text_xs()
                    .text_color(move || theme().popover_foreground),
            )
        })
        .trigger(
            view()
                .h(px(26.0))
                .pl_2p5()
                .pr_1()
                .flex_row()
                .items_center()
                .justify_between()
                .gap_3()
                .rounded(tokens::radius_control)
                .bg(move || theme().secondary)
                .border_1()
                .border_color(move || theme().input)
                .child(
                    text(selected_label)
                        .text_xs()
                        .text_color(move || theme().foreground),
                )
                .child(
                    view()
                        .w(px(18.0))
                        .h(px(18.0))
                        .rounded(tokens::radius_inset)
                        .bg(move || theme().primary)
                        .items_center()
                        .justify_center()
                        .child(
                            text("⌄")
                                .text_xs()
                                .font_weight(FontWeight::BOLD)
                                .text_color(move || theme().primary_foreground),
                        ),
                ),
        )
}
