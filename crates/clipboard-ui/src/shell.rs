//! Top-level Preferences shell, system tray integration, global shortcut sync, and floating overlay mount.

use clipboard_core::DockMenubarMode;
use clipboard_core::ShortcutAction;
use nsh_base::theme;
use nsh_core::desktop::Shortcut as NshShortcut;
use nsh_core::element::ImageSrc;
use nsh_core::element::desktop::MenuItem;
use nsh_core::element::desktop::main_window;
use nsh_core::element::desktop::quit;
use nsh_core::element::desktop::register_global_shortcut;
use nsh_core::element::desktop::set_tray;
use nsh_core::element::desktop::tray;
use nsh_core::element::desktop::unregister_all_global_shortcuts;
use nsh_core::platform::Os;
use nsh_core::prelude::FontWeight;
use nsh_core::prelude::IntoElement;
use nsh_core::prelude::ParentElement;
use nsh_core::prelude::Styled;
use nsh_core::prelude::computed;
use nsh_core::prelude::dynamic;
use nsh_core::prelude::effect;
use nsh_core::prelude::on_key;
use nsh_core::prelude::os;
use nsh_core::prelude::px;
use nsh_core::prelude::text;
use nsh_core::prelude::untracked;
use nsh_core::prelude::view;

use crate::overlay;
use crate::pages;
use crate::state;
use crate::state::PrefTab;
use crate::ui::button::ButtonVariant;
use crate::ui::button::ControlSize;
use crate::ui::button::action_button;
use crate::ui::icons;
use crate::ui::surface::BadgeTone;
use crate::ui::surface::badge;
use crate::ui::surface::divider;
use crate::ui::surface::toolbar_tab;
use crate::ui::tokens;

/// Renders the Preferences shell, binds global hotkeys & system tray, and mounts the floating overlay.
pub(crate) fn shell() -> impl IntoElement {
    register_local_shortcuts();
    sync_global_shortcuts();
    sync_tray_icon();

    // Keep the Preferences window hidden at launch in production; open only from the tray or overlay gear.
    if state::is_background_polling_enabled() {
        main_window().hide();
    }

    let tab = computed(|| state::store_signal().with(|s| s.active_tab));

    view()
        .flex_1()
        .bg(move || theme().background)
        .child(preferences_toolbar())
        .child(divider())
        .child(view().flex_1().min_h(px(0.0)).child(dynamic(move || {
            let active = tab.get();
            untracked(|| match active {
                PrefTab::General => pages::general::page().into_element(),
                PrefTab::Shortcuts => pages::shortcuts::page().into_element(),
                PrefTab::Blacklist => pages::blacklist::page().into_element(),
            })
        })))
        .child(divider())
        .child(bottom_status_bar())
        .child(overlay::floating_clipboard_window())
}

fn preferences_toolbar() -> impl IntoElement {
    view()
        .bg(tokens::toolbar)
        .pt_2()
        .pb_2()
        .px_4()
        .gap_1()
        .items_center()
        .child(
            text(move || state::store_signal().with(|s| s.active_tab.title()))
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().foreground),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_center()
                .gap_2()
                .child(toolbar_tab(
                    icons::TAB_GENERAL_SVG,
                    "General",
                    || state::store_signal().with(|s| s.active_tab == PrefTab::General),
                    || state::set_tab(PrefTab::General),
                ))
                .child(toolbar_tab(
                    icons::TAB_SHORTCUTS_SVG,
                    "Shortcuts",
                    || state::store_signal().with(|s| s.active_tab == PrefTab::Shortcuts),
                    || state::set_tab(PrefTab::Shortcuts),
                ))
                .child(toolbar_tab(
                    icons::TAB_BLACKLIST_SVG,
                    "Blacklist",
                    || state::store_signal().with(|s| s.active_tab == PrefTab::Blacklist),
                    || state::set_tab(PrefTab::Blacklist),
                )),
        )
}

fn bottom_status_bar() -> impl IntoElement {
    let is_macos = os() == Os::MacOs;
    view()
        .h(px(36.0))
        .px_4()
        .flex_row()
        .items_center()
        .justify_between()
        .bg(tokens::toolbar)
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(badge("Zero Network", || BadgeTone::Success))
                .child(
                    text(move || state::store_signal().with(|s| s.status_message.clone()))
                        .text_xs()
                        .text_color(move || theme().muted_foreground),
                ),
        )
        .child(action_button(
            move || {
                let sc = state::store_signal().with(|s| {
                    s.settings
                        .shortcut_for(ShortcutAction::ShowClipboard)
                        .display_string(is_macos)
                });
                if sc.is_empty() {
                    "Show Clipboard".to_owned()
                } else {
                    format!("Show Clipboard ({sc})")
                }
            },
            ButtonVariant::Secondary,
            ControlSize::Sm,
            false,
            state::toggle_overlay,
        ))
}

fn register_local_shortcuts() {
    on_key("mod+1", |_| state::set_tab(PrefTab::General));
    on_key("mod+2", |_| state::set_tab(PrefTab::Shortcuts));
    on_key("mod+3", |_| state::set_tab(PrefTab::Blacklist));
    on_key("escape", |_| {
        let is_recording = state::store_signal().with(|s| s.recording_shortcut.is_some());
        if is_recording {
            state::cancel_recording_shortcut();
        } else {
            state::close_overlay();
        }
    });
}

fn sync_global_shortcuts() {
    let rev = computed(|| {
        state::store_signal().with(|s| (s.shortcuts_revision, s.recording_shortcut.is_some()))
    });

    effect(move || {
        let (_revision, is_recording) = rev.get();
        unregister_all_global_shortcuts();
        if is_recording {
            return;
        }
        let is_macos = os() == Os::MacOs;
        let bindings = state::store_signal().with(|s| s.settings.shortcuts.clone());

        for (action, shortcut) in bindings {
            if let Some(stroke) = shortcut.to_nsh_keystroke()
                && NshShortcut::parse(&stroke, is_macos).is_ok()
            {
                drop(register_global_shortcut(&stroke, move || {
                    dispatch_global_action(action);
                }));
            }
        }
    });
}

fn show_preferences_window() {
    main_window().show();
    main_window().focus();
}

fn dispatch_global_action(action: ShortcutAction) {
    match action {
        ShortcutAction::ShowClipboard => state::toggle_overlay(),
        ShortcutAction::ShowMainWindow => show_preferences_window(),
        ShortcutAction::ToggleSequentialPaste => state::toggle_sequential_mode(),
        ShortcutAction::PasteFromSequentialQueue => state::paste_from_sequential_queue(),
        ShortcutAction::OpenFilterWindow => state::open_filter_window(),
        ShortcutAction::PasteWithLastFilter => state::paste_with_last_filter(),
        ShortcutAction::PasteClipping(slot) => state::paste_recent_slot(slot),
        _ => {}
    }
}

fn sync_tray_icon() {
    let snap = computed(|| {
        state::store_signal().with(|s| {
            (
                s.settings.dock_menubar_mode,
                s.sequential_mode_active,
                s.history.items.len(),
            )
        })
    });

    effect(move || {
        let (mode, seq_active, count) = snap.get();
        if mode == DockMenubarMode::DockOnly {
            return;
        }
        let items = vec![
            MenuItem::new("Show Clipboard").on_select(state::open_overlay),
            MenuItem::new("Sequential Paste Mode")
                .checked(seq_active)
                .on_select(state::toggle_sequential_mode),
            MenuItem::new("Paste from Sequential Queue")
                .on_select(state::paste_from_sequential_queue),
            MenuItem::separator(),
            MenuItem::new("General Preferences…").on_select(|| {
                state::set_tab(PrefTab::General);
                show_preferences_window();
            }),
            MenuItem::new("Shortcuts…").on_select(|| {
                state::set_tab(PrefTab::Shortcuts);
                show_preferences_window();
            }),
            MenuItem::new("Blacklist…").on_select(|| {
                state::set_tab(PrefTab::Blacklist);
                show_preferences_window();
            }),
            MenuItem::separator(),
            MenuItem::new(format!("Clear History ({count})")).on_select(state::clear_history),
            MenuItem::separator(),
            MenuItem::new("Quit Clipboard").on_select(quit),
        ];

        set_tray(
            tray()
                .icon(ImageSrc::svg_markup(icons::TRAY_CLIPBOARD_SVG))
                .tooltip(format!("Clipboard — {count} local clippings"))
                .menu(items),
        );
    });
}
