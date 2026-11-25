//! Blacklist preferences page (Screenshot 1): manage applications ignored by Clipboard.

use clipboard_core::BlacklistEntry;
use nsh_base::button as base_button;
use nsh_base::theme;
use nsh_core::element::ComponentBuilder;
use nsh_core::element::Interactive;
use nsh_core::prelude::FontWeight;
use nsh_core::prelude::IntoElement;
use nsh_core::prelude::ParentElement;
use nsh_core::prelude::Styled;
use nsh_core::prelude::computed;
use nsh_core::prelude::dynamic;
use nsh_core::prelude::px;
use nsh_core::prelude::scroll;
use nsh_core::prelude::text;
use nsh_core::prelude::untracked;
use nsh_core::prelude::view;
use nsh_core::prelude::when;

use crate::state;
use crate::ui::button::ButtonVariant;
use crate::ui::button::ControlSize;
use crate::ui::button::action_button;
use crate::ui::input::plain_text_field;
use crate::ui::tokens;

const MIN_VISIBLE_ROWS: usize = 8;

/// Renders the Blacklist preferences tab.
pub(crate) fn page() -> impl IntoElement {
    let entries = computed(|| {
        state::store_signal().with(|s| (s.settings.blacklist.clone(), s.selected_blacklist_index))
    });

    view()
        .flex_1()
        .p_4()
        .gap_3()
        .child(
            view()
                .gap_1()
                .child(
                    text("Applications added to the blacklist are ignored by Clipboard.")
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || theme().foreground),
                )
                .child(
                    text("Apps that mark sensitive data as transient (like 1Password) are already ignored.")
                        .text_xs()
                        .text_color(move || theme().muted_foreground),
                ),
        )
        .child(
            view()
                .flex_1()
                .min_h(px(0.0))
                .rounded(tokens::radius_control)
                .border_1()
                .border_color(move || theme().border)
                .overflow_hidden()
                .child(scroll().flex_1().child(dynamic(move || {
                    let (list, selected) = entries.get();
                    untracked(|| {
                        let filler_count = MIN_VISIBLE_ROWS.saturating_sub(list.len());
                        let mut container = view();
                        for (idx, entry) in list.into_iter().enumerate() {
                            let is_sel = selected == Some(idx);
                            container = container.child(blacklist_row(entry, idx, is_sel));
                        }
                        for pad in 0..filler_count {
                            let row_idx = MIN_VISIBLE_ROWS - filler_count + pad;
                            container = container.child(
                                view().h(px(36.0)).bg(move || {
                                    if row_idx % 2 == 1 {
                                        tokens::row_alt()
                                    } else {
                                        theme().background
                                    }
                                }),
                            );
                        }
                        container.into_element()
                    })
                }))),
        )
        .child(when(
            || state::store_signal().with(|s| s.add_blacklist_open),
            add_application_panel,
        ))
        .child(
            view()
                .flex_row()
                .items_center()
                .child(segmented_add_remove()),
        )
}

fn blacklist_row(entry: BlacklistEntry, index: usize, is_selected: bool) -> impl IntoElement {
    let badge_glyph = if entry.name.to_lowercase().contains("keychain")
        || entry.name.to_lowercase().contains("password")
    {
        "🔑"
    } else {
        "▣"
    };

    view()
        .h(px(36.0))
        .px_3()
        .flex_row()
        .items_center()
        .gap_2p5()
        .bg(move || {
            if is_selected {
                theme().primary
            } else if index % 2 == 1 {
                tokens::row_alt()
            } else {
                theme().background
            }
        })
        .on_press(move || state::select_blacklist_index(index))
        .child(
            view()
                .w(px(22.0))
                .h(px(22.0))
                .rounded(tokens::radius_inset)
                .bg(move || {
                    if is_selected {
                        theme().primary_foreground.opacity(0.2)
                    } else {
                        theme().secondary
                    }
                })
                .items_center()
                .justify_center()
                .child(text(badge_glyph).text_xs()),
        )
        .child(text(entry.name).text_sm().text_color(move || {
            if is_selected {
                theme().primary_foreground
            } else {
                theme().foreground
            }
        }))
}

fn segmented_add_remove() -> impl IntoElement {
    let can_remove = move || {
        state::store_signal()
            .with(|s| s.selected_blacklist_index.is_some() && !s.settings.blacklist.is_empty())
    };

    view()
        .flex_row()
        .rounded(tokens::radius_inset)
        .border_1()
        .border_color(move || theme().border)
        .overflow_hidden()
        .child(
            base_button()
                .accessibility_label("Add application to blacklist")
                .on_press(state::toggle_add_blacklist_picker)
                .w(px(28.0))
                .h(px(22.0))
                .bg(move || theme().secondary)
                .hover(|s| s.bg(move || theme().accent))
                .items_center()
                .justify_center()
                .child(
                    text("+")
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(move || theme().foreground),
                ),
        )
        .child(view().w(px(1.0)).bg(move || theme().border))
        .child(
            base_button()
                .accessibility_label("Remove selected application from blacklist")
                .disabled(move || !can_remove())
                .on_press(state::remove_selected_blacklist_entry)
                .w(px(28.0))
                .h(px(22.0))
                .bg(move || theme().secondary)
                .hover(|s| s.bg(move || theme().accent))
                .disabled_style(|s| s.opacity(0.4))
                .items_center()
                .justify_center()
                .child(
                    text("−")
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(move || theme().foreground),
                ),
        )
}

fn add_application_panel() -> impl IntoElement {
    let installed = state::store_signal().with(|s| s.installed_apps.clone());

    view()
        .p_3()
        .gap_2()
        .rounded(tokens::radius_control)
        .bg(move || theme().card)
        .border_1()
        .border_color(move || theme().border)
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(plain_text_field(
                    || state::store_signal().with(|s| s.custom_blacklist_draft.clone()),
                    "Application name (e.g. 1Password.app)",
                    state::set_custom_blacklist_draft,
                ))
                .child(action_button(
                    "Add",
                    ButtonVariant::Primary,
                    ControlSize::Sm,
                    || state::store_signal().with(|s| s.custom_blacklist_draft.trim().is_empty()),
                    || {
                        let name = state::store_signal()
                            .with(|s| s.custom_blacklist_draft.trim().to_owned());
                        if !name.is_empty() {
                            state::add_blacklist_entry(BlacklistEntry::new(name.clone(), name));
                        }
                    },
                )),
        )
        .child(
            scroll()
                .max_h(px(108.0))
                .child(view().flex_row().flex_wrap().gap_1p5().children(
                    installed.into_iter().take(18).map(|app| {
                        let name = app.name.clone();
                        let ident = app.identifier;
                        action_button(
                            name.clone(),
                            ButtonVariant::Secondary,
                            ControlSize::Sm,
                            false,
                            move || {
                                state::add_blacklist_entry(BlacklistEntry::new(
                                    name.clone(),
                                    ident.clone(),
                                ));
                            },
                        )
                    }),
                )),
        )
}
