//! Styled buttons and shortcut recorder capsules built on [`nsh_base::button`].

use nsh_base::button as base_button;
use nsh_base::theme;
use nsh_core::element::ComponentBuilder;
use nsh_core::element::IntoProp;
use nsh_core::prelude::FontWeight;
use nsh_core::prelude::IntoElement;
use nsh_core::prelude::IntoReactive;
use nsh_core::prelude::ParentElement;
use nsh_core::prelude::SharedString;
use nsh_core::prelude::Styled;
use nsh_core::prelude::px;
use nsh_core::prelude::reduce_motion;
use nsh_core::prelude::text;
use nsh_core::prelude::view;

use crate::ui::tokens;

/// Semantic visual variants for [`action_button`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(dead_code, reason = "complete design-system button variant scale")]
pub(crate) enum ButtonVariant {
    /// Single committed primary action.
    Primary,
    /// Quiet filled secondary action.
    Secondary,
    /// Outlined control.
    Outline,
    /// Borderless toolbar/inline command.
    Ghost,
    /// Destructive action.
    Destructive,
}

/// Control height scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(dead_code, reason = "complete design-system button size scale")]
pub(crate) enum ControlSize {
    /// Compact desktop control (26 pt).
    Sm,
    /// Standard desktop control (30 pt).
    Md,
}

/// Styled semantic button with hover wash, active press scale, and focus-visible ring.
pub(crate) fn action_button<M>(
    label: impl IntoReactive<SharedString>,
    variant: ButtonVariant,
    size: ControlSize,
    disabled: impl IntoProp<bool, M>,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let reduced = reduce_motion();
    let (min_h, px_pad) = match size {
        ControlSize::Sm => (px(26.0), px(10.0)),
        ControlSize::Md => (px(30.0), px(14.0)),
    };

    let mut btn = base_button()
        .disabled(disabled)
        .on_press(on_press)
        .min_h(min_h)
        .px(px_pad)
        .py_1()
        .flex_row()
        .items_center()
        .justify_center()
        .gap_1p5()
        .rounded(tokens::radius_control)
        .transition_colors()
        .duration(tokens::DURATION_INSTANT)
        .focus_visible(|s| s.outline(px(2.0)).outline_color(move || theme().ring))
        .disabled_style(|s| s.opacity(0.45));

    if reduced {
        btn = btn.active(|s| s.opacity(0.85));
    } else {
        btn = btn
            .transition_transform()
            .duration(tokens::DURATION_FAST)
            .active(|s| s.scale(tokens::PRESS_SCALE).opacity(0.9));
    }

    btn = match variant {
        ButtonVariant::Primary => btn
            .bg(move || theme().primary)
            .hover(|s| s.bg(move || theme().primary.opacity(0.9))),
        ButtonVariant::Secondary => btn
            .bg(move || theme().secondary)
            .hover(|s| s.bg(move || theme().accent)),
        ButtonVariant::Outline => btn
            .bg(move || theme().background)
            .border_1()
            .border_color(move || theme().input)
            .hover(|s| s.bg(move || theme().accent)),
        ButtonVariant::Ghost => btn.hover(|s| s.bg(move || theme().accent)),
        ButtonVariant::Destructive => btn
            .bg(tokens::destructive_surface)
            .border_1()
            .border_color(move || theme().destructive.opacity(0.4))
            .hover(|s| s.bg(move || theme().destructive.opacity(0.18))),
    };

    let label_el = text(label)
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(move || match variant {
            ButtonVariant::Primary => theme().primary_foreground,
            ButtonVariant::Secondary | ButtonVariant::Outline | ButtonVariant::Ghost => {
                theme().foreground
            }
            ButtonVariant::Destructive => theme().destructive,
        });

    btn.child(label_el)
}

/// Square vector-icon button with a generous `32×32` hit target and explicit accessible name.
pub(crate) fn svg_icon_button<M>(
    svg_markup: &'static str,
    icon_size: f32,
    accessibility_name: &'static str,
    on_selected_surface: bool,
    disabled: impl IntoProp<bool, M>,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let reduced = reduce_motion();
    let mut btn = base_button()
        .disabled(disabled)
        .accessibility_label(accessibility_name)
        .on_press(on_press)
        .w(px(32.0))
        .h(px(32.0))
        .items_center()
        .justify_center()
        .rounded(tokens::radius_control)
        .transition_colors()
        .duration(tokens::DURATION_INSTANT)
        .focus_visible(|s| s.outline(px(2.0)).outline_color(move || theme().ring))
        .disabled_style(|s| s.opacity(0.4));

    if !reduced {
        btn = btn
            .transition_transform()
            .duration(tokens::DURATION_FAST)
            .active(|s| s.scale(tokens::ICON_PRESS_SCALE).opacity(0.85));
    }

    btn = if on_selected_surface {
        btn.hover(|s| s.bg(move || theme().primary_foreground.opacity(0.18)))
    } else {
        btn.hover(|s| s.bg(move || theme().accent))
    };

    btn.child(crate::ui::icons::vector_icon(
        svg_markup,
        icon_size,
        move || {
            if on_selected_surface {
                theme().primary_foreground
            } else {
                theme().muted_foreground
            }
        },
    ))
}

/// Dark/light capsule shortcut recorder control matching the Shortcuts screenshots:
/// displays a centered blue `+` circle when unset, or the shortcut glyphs + trailing blue `×` clear circle.
pub(crate) fn shortcut_capsule(
    shortcut_label: impl Fn() -> String + Copy + 'static,
    is_recording: impl Fn() -> bool + Copy + 'static,
    record_accessibility_name: SharedString,
    clear_accessibility_name: SharedString,
    on_record_press: impl Fn() + 'static,
    on_clear_press: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    view()
        .w(px(148.0))
        .h(px(26.0))
        .rounded_full()
        .bg(move || {
            if is_recording() {
                theme().primary.opacity(0.22)
            } else {
                tokens::pill_surface()
            }
        })
        .border_1()
        .border_color(move || {
            if is_recording() {
                theme().ring
            } else {
                theme().border.opacity(0.4)
            }
        })
        .flex_row()
        .items_center()
        .px_2()
        .transition_colors()
        .duration(tokens::DURATION_INSTANT)
        .child(
            base_button()
                .accessibility_label(record_accessibility_name)
                .on_press(on_record_press)
                .flex_1()
                .h(px(24.0))
                .items_center()
                .justify_center()
                .child(nsh_core::prelude::dynamic(move || {
                    let recording = is_recording();
                    let label = shortcut_label();
                    if recording {
                        text("Type shortcut…")
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(move || theme().foreground)
                            .into_element()
                    } else if label.is_empty() {
                        view()
                            .w(px(18.0))
                            .h(px(18.0))
                            .rounded_full()
                            .bg(tokens::pill_accent)
                            .items_center()
                            .justify_center()
                            .child(
                                text("+")
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(move || theme().primary_foreground),
                            )
                            .into_element()
                    } else {
                        text(label)
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .font_family(tokens::mono_font())
                            .text_color(move || theme().foreground)
                            .into_element()
                    }
                })),
        )
        .child(nsh_core::prelude::when(
            move || !is_recording() && !shortcut_label().is_empty(),
            move || {
                base_button()
                    .accessibility_label(clear_accessibility_name.clone())
                    .on_press(on_clear_press)
                    .w(px(18.0))
                    .h(px(18.0))
                    .rounded_full()
                    .bg(tokens::pill_accent)
                    .items_center()
                    .justify_center()
                    .hover(|s| s.opacity(0.85))
                    .child(
                        text("×")
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(move || theme().primary_foreground),
                    )
            },
        ))
}
