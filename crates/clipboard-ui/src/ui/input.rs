//! Styled text inputs built on [`nsh_base::input`].

use nsh_base::input as base_input;
use nsh_base::theme;
use nsh_core::element::Element;
use nsh_core::element::ElementRef;
use nsh_core::element::Interactive;
use nsh_core::keyboard::KeyEvent;
use nsh_core::prelude::Accessible;
use nsh_core::prelude::IntoElement;
use nsh_core::prelude::ParentElement;
use nsh_core::prelude::Styled;
use nsh_core::prelude::color;
use nsh_core::prelude::px;
use nsh_core::prelude::view;

use crate::ui::tokens;

/// Overlay search field with leading magnifying glass icon and keyboard interception.
pub(crate) fn search_input(
    input_ref: &ElementRef,
    value: impl Fn() -> String + 'static,
    on_change: impl Fn(&str) + 'static,
    on_key_down: impl Fn(&KeyEvent) + 'static,
) -> impl IntoElement {
    view()
        .h(px(40.0))
        .px_3()
        .flex_row()
        .items_center()
        .gap_2()
        .bg(move || theme().popover)
        .border_b_1()
        .border_color(move || theme().border)
        .child(crate::ui::icons::vector_icon(
            crate::ui::icons::SEARCH_SVG,
            16.0,
            || theme().muted_foreground,
        ))
        .child(
            base_input()
                .element_ref(input_ref)
                .value(value)
                .on_change(on_change)
                .on_key_down(on_key_down)
                .placeholder("Type to Search...")
                .placeholder_color(|| color(theme().muted_foreground))
                .caret_color(|| color(theme().ring))
                .auto_focus(true)
                .flex_1()
                .h(px(28.0))
                .text_sm()
                .text_color(move || theme().popover_foreground),
        )
}

/// Compact numeric text field used for `Ignore clippings larger than: [ 25 ] MB`.
pub(crate) fn compact_number_field(
    value: impl Fn() -> String + 'static,
    accessibility_label: &'static str,
    on_change: impl Fn(&str) + 'static,
) -> impl IntoElement {
    base_input()
        .label(accessibility_label)
        .value(value)
        .on_change(on_change)
        .w(px(56.0))
        .h(px(26.0))
        .px_2()
        .text_sm()
        .text_color(move || theme().foreground)
        .bg(move || theme().secondary)
        .rounded(tokens::radius_inset)
        .border_1()
        .border_color(move || theme().input)
        .focus(|s| s.border_color(move || theme().ring))
}

/// Plain text input for adding custom blacklist app names.
pub(crate) fn plain_text_field(
    value: impl Fn() -> String + 'static,
    placeholder: &'static str,
    on_change: impl Fn(&str) + 'static,
) -> impl IntoElement {
    base_input()
        .value(value)
        .on_change(on_change)
        .placeholder(placeholder)
        .placeholder_color(|| color(theme().muted_foreground))
        .caret_color(|| color(theme().ring))
        .flex_1()
        .h(px(28.0))
        .px_2p5()
        .text_xs()
        .text_color(move || theme().foreground)
        .bg(move || theme().background)
        .rounded(tokens::radius_control)
        .border_1()
        .border_color(move || theme().input)
        .focus(|s| s.border_color(move || theme().ring))
}
