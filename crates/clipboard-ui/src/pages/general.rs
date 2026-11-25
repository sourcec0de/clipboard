//! General preferences page (Screenshot 4), strictly local-only with zero network sync.

use clipboard_core::DockMenubarMode;
use clipboard_core::QuickPasteMenuStyle;
use nsh_base::SelectItem;
use nsh_base::SliderValue;
use nsh_base::theme;
use nsh_core::prelude::IntoElement;
use nsh_core::prelude::ParentElement;
use nsh_core::prelude::SharedString;
use nsh_core::prelude::Styled;
use nsh_core::prelude::px;
use nsh_core::prelude::scroll;
use nsh_core::prelude::text;
use nsh_core::prelude::view;

use crate::state;
use crate::ui::checkbox::checkbox_row;
use crate::ui::input::compact_number_field;
use crate::ui::select::dropdown_select;
use crate::ui::slider::history_slider;
use crate::ui::surface::divider;

const LABEL_COL_W: f32 = 186.0;

/// Renders the General preferences tab.
pub(crate) fn page() -> impl IntoElement {
    scroll().flex_1().child(
        view()
            .p_5()
            .gap_4()
            .child(history_section())
            .child(divider())
            .child(quick_paste_section())
            .child(divider())
            .child(system_section()),
    )
}

fn history_section() -> impl IntoElement {
    view()
        .gap_3()
        .child(form_row(
            "Ignore clippings larger than:",
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(compact_number_field(
                    || state::store_signal().with(|s| s.settings.max_clipping_mb.to_string()),
                    "Ignore clippings larger than megabytes",
                    state::set_max_clipping_mb_str,
                ))
                .child(text("MB").text_sm().text_color(move || theme().foreground)),
        ))
        .child(form_row_dynamic(
            || {
                state::store_signal()
                    .with(|s| format!("Keep the last {} clippings:", s.settings.history_limit))
            },
            history_slider(
                || {
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "history limit is 100..=1000 and exact in f64"
                    )]
                    let v = state::store_signal().with(|s| s.settings.history_limit as f64);
                    Some(SliderValue::Single(v))
                },
                |val: &SliderValue| {
                    #[expect(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "slider range is clamped to 100..=1000"
                    )]
                    let count = val.start().round().clamp(100.0, 1000.0) as usize;
                    state::set_history_limit(count);
                },
            ),
        ))
        .child(indented_control(checkbox_row(
            "Always Paste Plain Text",
            None,
            || state::store_signal().with(|s| s.settings.always_paste_plain_text),
            || state::store_signal().with(|s| s.settings.always_paste_plain_text),
            |checked: &bool| state::set_always_paste_plain_text(*checked),
        )))
}

fn quick_paste_section() -> impl IntoElement {
    let quick_paste_items: Vec<SelectItem> = QuickPasteMenuStyle::ALL
        .iter()
        .copied()
        .map(|m| SelectItem::new(m.as_id(), m.label()))
        .collect();

    view()
        .gap_3()
        .child(form_row(
            "Quick Paste Menu:",
            dropdown_select(
                "Quick Paste Menu",
                quick_paste_items,
                || {
                    Some(SharedString::from(
                        state::store_signal().with(|s| s.settings.quick_paste_menu.as_id()),
                    ))
                },
                || {
                    SharedString::from(
                        state::store_signal().with(|s| s.settings.quick_paste_menu.label()),
                    )
                },
                |next: &Option<SharedString>| {
                    if let Some(id) = next {
                        state::set_quick_paste_menu(QuickPasteMenuStyle::from_id(id.as_ref()));
                    }
                },
            ),
        ))
        .child(indented_control(checkbox_row(
            "Appear Behind Mouse Cursor",
            None,
            || state::store_signal().with(|s| s.settings.appear_behind_mouse_cursor),
            || state::store_signal().with(|s| s.settings.appear_behind_mouse_cursor),
            |checked: &bool| state::set_appear_behind_mouse_cursor(*checked),
        )))
        .child(indented_control(checkbox_row(
            "Release to Paste",
            Some("The selected clipping gets pasted when you release the modifier keys."),
            || state::store_signal().with(|s| s.settings.release_to_paste),
            || state::store_signal().with(|s| s.settings.release_to_paste),
            |checked: &bool| state::set_release_to_paste(*checked),
        )))
}

fn system_section() -> impl IntoElement {
    let dock_items: Vec<SelectItem> = DockMenubarMode::ALL
        .iter()
        .copied()
        .map(|m| SelectItem::new(m.as_id(), m.label()))
        .collect();

    view()
        .gap_3()
        .child(indented_control(checkbox_row(
            "Enable Interaction Sounds",
            None,
            || state::store_signal().with(|s| s.settings.enable_interaction_sounds),
            || state::store_signal().with(|s| s.settings.enable_interaction_sounds),
            |checked: &bool| state::set_enable_interaction_sounds(*checked),
        )))
        .child(form_row(
            "Dock and Menubar Icon:",
            view()
                .flex_1()
                .gap_1p5()
                .child(dropdown_select(
                    "Dock and Menubar Icon",
                    dock_items,
                    || {
                        Some(SharedString::from(
                            state::store_signal()
                                .with(|s| s.settings.dock_menubar_mode.as_id()),
                        ))
                    },
                    || {
                        SharedString::from(
                            state::store_signal()
                                .with(|s| s.settings.dock_menubar_mode.label()),
                        )
                    },
                    |next: &Option<SharedString>| {
                        if let Some(id) = next {
                            state::set_dock_menubar_mode(DockMenubarMode::from_id(id.as_ref()));
                        }
                    },
                ))
                .child(
                    text("The menubar icon provides mouse access to the quick paste menu. The dock icon provides easy access to the main window.")
                        .text_xs()
                        .text_color(move || theme().muted_foreground),
                ),
        ))
        .child(indented_control(checkbox_row(
            "Open automatically at login",
            None,
            || state::store_signal().with(|s| s.settings.open_at_login),
            || state::store_signal().with(|s| s.settings.open_at_login),
            |checked: &bool| state::set_open_at_login(*checked),
        )))
}

fn form_row(label: &'static str, control: impl IntoElement) -> impl IntoElement {
    view()
        .flex_row()
        .items_start()
        .gap_3()
        .child(
            view()
                .w(px(LABEL_COL_W))
                .items_end()
                .pt_1()
                .child(text(label).text_sm().text_color(move || theme().foreground)),
        )
        .child(view().flex_1().min_w(px(0.0)).child(control))
}

fn form_row_dynamic(
    label: impl Fn() -> String + 'static,
    control: impl IntoElement,
) -> impl IntoElement {
    view()
        .flex_row()
        .items_start()
        .gap_3()
        .child(
            view().w(px(LABEL_COL_W)).items_end().pt_1().child(
                text(label)
                    .text_sm()
                    .tabular_nums()
                    .text_color(move || theme().foreground),
            ),
        )
        .child(view().flex_1().min_w(px(0.0)).child(control))
}

fn indented_control(control: impl IntoElement) -> impl IntoElement {
    view()
        .flex_row()
        .items_start()
        .gap_3()
        .child(view().w(px(LABEL_COL_W)))
        .child(view().flex_1().min_w(px(0.0)).child(control))
}
