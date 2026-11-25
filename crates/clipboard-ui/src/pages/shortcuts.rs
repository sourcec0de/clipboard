//! Shortcuts preferences page (Screenshots 2 & 3): global keyboard shortcut recorder table.

use clipboard_core::KeyboardShortcut;
use clipboard_core::Modifiers;
use clipboard_core::ShortcutAction;
use nsh_base::theme;
use nsh_core::element::Element;
use nsh_core::element::Interactive;
use nsh_core::keyboard::KeyCapture;
use nsh_core::keyboard::KeyEvent;
use nsh_core::platform::Os;
use nsh_core::prelude::FontWeight;
use nsh_core::prelude::IntoElement;
use nsh_core::prelude::ParentElement;
use nsh_core::prelude::SharedString;
use nsh_core::prelude::Styled;
use nsh_core::prelude::effect;
use nsh_core::prelude::element_ref;
use nsh_core::prelude::os;
use nsh_core::prelude::px;
use nsh_core::prelude::scroll;
use nsh_core::prelude::text;
use nsh_core::prelude::view;
use nsh_core::prelude::when;

use crate::state;
use crate::ui::button::shortcut_capsule;
use crate::ui::tokens;

/// Renders the Shortcuts preferences tab.
pub(crate) fn page() -> impl IntoElement {
    view()
        .flex_1()
        .p_4()
        .gap_3()
        .child(
            text("Edit global keyboard shortcuts.")
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().foreground),
        )
        .child(when(
            || state::store_signal().with(|s| s.recording_shortcut.is_some()),
            live_capture_banner,
        ))
        .child(
            view()
                .flex_1()
                .min_h(px(0.0))
                .rounded(tokens::radius_control)
                .border_1()
                .border_color(move || theme().border)
                .overflow_hidden()
                .child(
                    scroll()
                        .flex_1()
                        .child(section_header_row("Actions"))
                        .children(
                            ShortcutAction::ACTIONS
                                .iter()
                                .copied()
                                .enumerate()
                                .map(|(idx, action)| shortcut_table_row(action, idx % 2 == 1)),
                        )
                        .child(section_header_row("Paste Recent Clippings"))
                        .children(
                            ShortcutAction::RECENT_CLIPPINGS
                                .iter()
                                .copied()
                                .enumerate()
                                .map(|(idx, action)| shortcut_table_row(action, idx % 2 == 1)),
                        ),
                ),
        )
}

fn section_header_row(title: &'static str) -> impl IntoElement {
    view()
        .h(px(30.0))
        .px_3()
        .justify_center()
        .bg(tokens::section_header_bg)
        .border_b_1()
        .border_color(move || theme().border)
        .child(
            text(title)
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().foreground),
        )
}

fn shortcut_table_row(action: ShortcutAction, alternate: bool) -> impl IntoElement {
    let is_macos = os() == Os::MacOs;
    let display_name = action.display_name();
    let record_name: SharedString = format!("Record shortcut for {display_name}").into();
    let clear_name: SharedString = format!("Clear shortcut for {display_name}").into();

    let is_recording = move || state::store_signal().with(|s| s.recording_shortcut == Some(action));
    let current_label = move || {
        state::store_signal().with(|s| s.settings.shortcut_for(action).display_string(is_macos))
    };

    view()
        .h(px(36.0))
        .px_3()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .bg(move || {
            if alternate {
                tokens::row_alt()
            } else {
                theme().background
            }
        })
        .child(
            text(display_name)
                .text_sm()
                .text_color(move || theme().foreground),
        )
        .child(shortcut_capsule(
            current_label,
            is_recording,
            record_name,
            clear_name,
            move || {
                if is_recording() {
                    state::cancel_recording_shortcut();
                } else {
                    state::start_recording_shortcut(action);
                }
            },
            move || state::clear_shortcut(action),
        ))
}

fn live_capture_banner() -> impl IntoElement {
    let capture_ref = element_ref();
    effect(move || {
        capture_ref.focus();
    });

    view()
        .element_ref(&capture_ref)
        .focusable(true)
        .capture_keys(KeyCapture::All)
        .on_key_down(|event: &KeyEvent| {
            if event.repeat {
                return;
            }
            let Some(action) = state::store_signal().with(|s| s.recording_shortcut) else {
                return;
            };
            let mods = Modifiers::new(
                event.modifiers.ctrl,
                event.modifiers.alt,
                event.modifiers.shift,
                event.modifiers.cmd,
            );
            if event.key.as_ref() == "Escape" && mods.is_empty() {
                state::cancel_recording_shortcut();
                return;
            }
            if let Some(captured) =
                KeyboardShortcut::from_dom_key(event.key.as_ref(), event.code.as_ref(), mods)
            {
                state::apply_shortcut(action, captured);
            }
        })
        .px_3()
        .py_2()
        .rounded(tokens::radius_control)
        .bg(tokens::pill_surface)
        .border_1()
        .border_color(move || theme().ring)
        .flex_row()
        .items_center()
        .justify_between()
        .child(
            text(move || {
                let name = state::store_signal()
                    .with(|s| s.recording_shortcut.map(ShortcutAction::display_name))
                    .unwrap_or_default();
                format!("Press key combination for {name}…")
            })
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .text_color(move || theme().foreground),
        )
        .child(
            text("Esc to cancel")
                .text_xs()
                .text_color(move || theme().muted_foreground),
        )
}
