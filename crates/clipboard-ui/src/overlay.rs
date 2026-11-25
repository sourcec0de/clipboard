//! Floating Quick Paste Clipboard Overlay window (Screenshot 5).
//!
//! Features:
//! - Floating `WindowKind::Panel` above all windows and workspaces (`Workspaces::All`)
//! - Top bar with Settings gear button and `Clipboard ⌄` source switcher
//! - Immediate type-to-search: typing any character refocuses the search input and filters
//! - Down arrow from search navigates into the list; Up/Down move selection; Enter pastes; Escape closes
//! - Blue highlighted selected row with inline Filter & Preview buttons and italic shortcut index
//! - Strictly bounded row height for image clippings so thumbnails peek cleanly without blowing up the row

use clipboard_core::Clipping;
use clipboard_core::ClippingKind;
use clipboard_core::QuickPasteMenuStyle;
use clipboard_core::TextFilter;
use nsh_base::button as base_button;
use nsh_base::theme;
use nsh_core::desktop::WindowAttributes;
use nsh_core::desktop::WindowFrameStyle;
use nsh_core::desktop::WindowKind;
use nsh_core::desktop::WindowLevel;
use nsh_core::desktop::WindowPointerEvents;
use nsh_core::desktop::WindowPosition;
use nsh_core::desktop::Workspaces;
use nsh_core::element::ComponentBuilder;
use nsh_core::element::ImageSrc;
use nsh_core::element::Interactive;
use nsh_core::element::desktop::OpenChange;
use nsh_core::element::desktop::displays;
use nsh_core::element::desktop::main_window;
use nsh_core::element::desktop::window;
use nsh_core::geometry::Point;
use nsh_core::geometry::Points;
use nsh_core::geometry::Size;
use nsh_core::keymap::When;
use nsh_core::prelude::FontWeight;
use nsh_core::prelude::IntoElement;
use nsh_core::prelude::KeyOptions;
use nsh_core::prelude::ParentElement;
use nsh_core::prelude::Styled;
use nsh_core::prelude::computed;
use nsh_core::prelude::dynamic;
use nsh_core::prelude::effect;
use nsh_core::prelude::element_ref;
use nsh_core::prelude::image;
use nsh_core::prelude::on_key_with;
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
use crate::ui::button::svg_icon_button;
use crate::ui::icons;
use crate::ui::input::search_input;
use crate::ui::surface::BadgeTone;
use crate::ui::surface::badge;
use crate::ui::surface::divider;
use crate::ui::surface::empty_state;
use crate::ui::tokens;

const OVERLAY_WIDTH: f32 = 420.0;
const OVERLAY_HEIGHT: f32 = 460.0;
const IMAGE_ROW_MAX_HEIGHT: f32 = 54.0;
const IMAGE_THUMB_MAX_W: u32 = 160;
const IMAGE_THUMB_MAX_H: u32 = 42;

/// Declares the separate floating Quick Paste overlay window in the reactive tree.
pub(crate) fn floating_clipboard_window() -> impl IntoElement {
    let is_open = computed(|| state::store_signal().with(|s| s.overlay_open));

    window()
        .id("clipboard-quick-paste")
        .kind(WindowKind::Panel)
        .default_size(Size::new(px(OVERLAY_WIDTH), px(OVERLAY_HEIGHT)))
        .default_position(overlay_position())
        .restore_bounds(false)
        .attributes(overlay_window_attributes())
        .open(is_open)
        .on_open_change(|ev: &OpenChange| {
            if !ev.open {
                state::close_overlay();
            }
        })
        .on_close_press(|_| state::close_overlay())
        .content(overlay_surface)
}

fn overlay_window_attributes() -> WindowAttributes {
    let mut attrs = WindowAttributes::default();
    attrs.frame = Some(WindowFrameStyle::None);
    attrs.transparent = Some(true);
    attrs.shadow = Some(true);
    attrs.resizable = Some(false);
    attrs.movable = Some(true);
    attrs.minimizable = Some(false);
    attrs.maximizable = Some(false);
    attrs.closable = Some(true);
    attrs.focusable = Some(true);
    attrs.activate = Some(true);
    attrs.close_on_blur = Some(true);
    attrs.close_on_escape = Some(true);
    attrs.level = Some(WindowLevel::Floating);
    attrs.workspaces = Some(Workspaces::All);
    attrs.pointer_events = Some(WindowPointerEvents::Auto);
    attrs.taskbar = Some(false);
    attrs
}

fn overlay_position() -> WindowPosition {
    let all = displays();
    let Some(primary) = all.iter().find(|d| d.primary).or_else(|| all.first()) else {
        return WindowPosition::Center;
    };
    let bounds = primary.visible_bounds;
    let x = bounds.origin.x.0 + ((bounds.size.width.0 - OVERLAY_WIDTH) * 0.5).max(0.0);
    let y = bounds.origin.y.0 + ((bounds.size.height.0 - OVERLAY_HEIGHT) * 0.35).max(24.0);
    WindowPosition::At(Point::new(Points(x), Points(y)))
}

/// Renders the Quick Paste overlay card (also mountable inline for headless testing).
pub(crate) fn overlay_surface() -> impl IntoElement {
    let search_ref = element_ref();

    let focus_seq = computed(|| state::store_signal().with(|s| s.search_focus_seq));
    effect(move || {
        let _seq = focus_seq.get();
        search_ref.focus();
    });

    // Bind navigation & confirm/dismiss keys with `When::Always` so Up/Down/Enter/Escape work
    // seamlessly even while the search `NSTextField` holds keyboard focus, while any other
    // typed character flows directly into the search input.
    bind_overlay_keys();

    view()
        .w(px(OVERLAY_WIDTH))
        .h(px(OVERLAY_HEIGHT))
        .rounded(tokens::radius_overlay)
        .bg(move || theme().popover)
        .border_1()
        .border_color(move || theme().border)
        .overflow_hidden()
        .shadow_lg()
        .child(overlay_header())
        .child(search_input(
            &search_ref,
            move || state::store_signal().with(|s| s.search_query.clone()),
            state::set_search_query,
            |_| {},
        ))
        .child(view().flex_1().min_h(px(0.0)).child(clippings_list()))
        .child(when(
            || state::store_signal().with(|s| s.pending_delete_id.is_some()),
            delete_confirm_bar,
        ))
        .child(when(
            || state::store_signal().with(|s| s.filter_picker_open),
            filter_drawer,
        ))
        .child(when(
            || {
                state::store_signal().with(|s| {
                    s.pending_delete_id.is_none()
                        && (s.preview_open
                            || s.settings.quick_paste_menu == QuickPasteMenuStyle::Expanded)
                })
            },
            preview_drawer,
        ))
}

fn bind_overlay_keys() {
    let repeat_opts = KeyOptions {
        when: When::Always,
        repeat: true,
        ..KeyOptions::default()
    };
    let once_opts = KeyOptions {
        when: When::Always,
        repeat: false,
        ..KeyOptions::default()
    };

    on_key_with("down", repeat_opts, |_| state::move_selection(1));
    on_key_with("up", repeat_opts, |_| state::move_selection(-1));
    on_key_with("enter", once_opts, |_| {
        if state::store_signal().with(|s| s.pending_delete_id.is_some()) {
            state::confirm_delete_pending();
        } else {
            state::paste_selected(false);
        }
    });
    on_key_with("shift+enter", once_opts, |_| {
        if state::store_signal().with(|s| s.pending_delete_id.is_some()) {
            state::confirm_delete_pending();
        } else {
            state::paste_selected(true);
        }
    });
    on_key_with("escape", once_opts, |_| {
        if state::store_signal().with(|s| s.pending_delete_id.is_some()) {
            state::cancel_delete_pending();
        } else {
            state::dismiss_overlay_and_restore_focus();
        }
    });
    on_key_with("delete", once_opts, |_| state::request_delete_selected());
    on_key_with("mod+backspace", once_opts, |_| {
        state::request_delete_selected();
    });
    // Only trigger delete-prompt on plain Backspace when NOT typing in the search input,
    // so Backspace and Cmd+A + Backspace work natively inside the search field.
    on_key_with("backspace", KeyOptions::default(), |_| {
        state::request_delete_selected();
    });
    on_key_with("tab", once_opts, |_| state::toggle_preview());
}

fn overlay_header() -> impl IntoElement {
    view()
        .h(px(40.0))
        .px_3()
        .flex_row()
        .items_center()
        .justify_between()
        .bg(tokens::toolbar)
        .border_b_1()
        .border_color(move || theme().border)
        .window_drag(true)
        .child(svg_icon_button(
            icons::GEAR_SVG,
            18.0,
            "Open Preferences",
            false,
            false,
            || {
                state::close_overlay();
                main_window().show();
                main_window().focus();
            },
        ))
        .child(
            base_button()
                .accessibility_label("Switch clipboard source")
                .on_press(state::cycle_overlay_source)
                .h(px(32.0))
                .px_2p5()
                .rounded(tokens::radius_control)
                .hover(|s| s.bg(move || theme().accent))
                .flex_row()
                .items_center()
                .gap_1p5()
                .child(
                    text(move || state::store_signal().with(|s| s.overlay_source.label()))
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || theme().foreground),
                )
                .child(icons::vector_icon(
                    icons::CHEVRONS_UP_DOWN_SVG,
                    14.0,
                    || theme().muted_foreground,
                )),
        )
        .child(view().min_w(px(32.0)).items_end().child(when(
            || state::store_signal().with(|s| s.sequential_mode_active),
            || badge("SEQ", || BadgeTone::Warning),
        )))
}

fn clippings_list() -> impl IntoElement {
    let list_revision = computed(|| {
        state::store_signal().with(|s| {
            (
                s.visible_clippings(),
                s.selected_index,
                s.settings.quick_paste_menu,
            )
        })
    });

    scroll().flex_1().child(dynamic(move || {
        let (items, selected_index, density) = list_revision.get();
        untracked(|| {
            if items.is_empty() {
                return empty_state(
                    "No clippings found",
                    "Copy text or images anywhere, or clear your search filter.",
                )
                .into_element();
            }

            view()
                .children(items.iter().enumerate().map(|(idx, item)| {
                    let is_selected = idx == selected_index;
                    clipping_row(item, idx, is_selected, density)
                }))
                .into_element()
        })
    }))
}

fn clipping_row(
    item: &Clipping,
    index: usize,
    is_selected: bool,
    density: QuickPasteMenuStyle,
) -> impl IntoElement {
    let max_chars = match density {
        QuickPasteMenuStyle::Compact => 42,
        _ => 76,
    };
    let is_image = item.kind == ClippingKind::Image;

    let mut row = view()
        .group()
        .px_3()
        .py_2()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_2p5()
        .border_b_1()
        .border_color(move || {
            if is_selected {
                theme().primary
            } else {
                theme().border.opacity(0.6)
            }
        })
        .bg(move || {
            if is_selected {
                theme().primary
            } else {
                theme().popover
            }
        })
        .hover(move |s| {
            if is_selected {
                s.bg(move || theme().primary)
            } else {
                s.bg(move || theme().accent)
            }
        })
        .transition_colors()
        .duration(tokens::DURATION_INSTANT)
        .on_press(move || {
            state::select_index(index);
            state::paste_selected(false);
        });

    row = if is_image {
        // Strictly bound image row height so thumbnails peek cleanly without expanding the list.
        row.h(px(IMAGE_ROW_MAX_HEIGHT)).overflow_hidden()
    } else if density == QuickPasteMenuStyle::Compact {
        row.h(px(36.0)).overflow_hidden()
    } else {
        row.h(px(56.0)).overflow_hidden()
    };

    let content_el = if is_image {
        image_row_preview(item, is_selected).into_element()
    } else {
        view()
            .max_h(px(38.0))
            .overflow_hidden()
            .justify_center()
            .child(
                text(item.preview_text(max_chars))
                    .text_sm()
                    .text_color(move || {
                        if is_selected {
                            theme().primary_foreground
                        } else {
                            theme().popover_foreground
                        }
                    }),
            )
            .into_element()
    };

    row.child(
        view()
            .flex_1()
            .min_w(px(0.0))
            .overflow_hidden()
            .justify_center()
            .child(content_el),
    )
    .child(clipping_row_trailing(index, is_selected))
}

fn clipping_row_trailing(index: usize, is_selected: bool) -> impl IntoElement {
    let index_label = (index + 1).to_string();
    view()
        .flex_row()
        .items_center()
        .gap_1()
        // Single fixed-width hover-only icon slot: invisible during keyboard navigation
        // and causes zero layout shift on selection or hover.
        .child(
            view()
                .w(px(32.0))
                .h(px(32.0))
                .items_center()
                .justify_center()
                .opacity(0.0)
                .group_hover(|s| s.opacity(1.0))
                .transition_opacity()
                .duration(tokens::DURATION_INSTANT)
                .child(svg_icon_button(
                    icons::SHARE_PREVIEW_SVG,
                    16.0,
                    "Preview clipping",
                    is_selected,
                    false,
                    move || {
                        state::select_index(index);
                        state::toggle_preview();
                    },
                )),
        )
        .child(
            view().w(px(20.0)).items_end().child(
                text(index_label)
                    .text_xs()
                    .italic()
                    .tabular_nums()
                    .text_color(move || {
                        if is_selected {
                            theme().primary_foreground.opacity(0.9)
                        } else {
                            theme().muted_foreground
                        }
                    }),
            ),
        )
}

fn delete_confirm_bar() -> impl IntoElement {
    let snippet = computed(|| {
        state::store_signal().with(|s| {
            s.pending_delete_id
                .and_then(|id| s.history.items.iter().find(|c| c.id == id))
                .map(|c| c.preview_text(42))
                .unwrap_or_default()
        })
    });

    view()
        .bg(tokens::toolbar)
        .border_t_1()
        .border_color(move || theme().border)
        .px_3()
        .py_2p5()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            view()
                .flex_1()
                .min_w(px(0.0))
                .gap_0p5()
                .child(
                    text("Remove this clipping?")
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || theme().foreground),
                )
                .child(
                    text(move || snippet.get())
                        .text_xs()
                        .text_color(move || theme().muted_foreground)
                        .truncate(),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_1p5()
                .child(action_button(
                    "Cancel",
                    ButtonVariant::Ghost,
                    ControlSize::Sm,
                    false,
                    state::cancel_delete_pending,
                ))
                .child(action_button(
                    "Remove",
                    ButtonVariant::Destructive,
                    ControlSize::Sm,
                    false,
                    state::confirm_delete_pending,
                )),
        )
}

fn image_row_preview(item: &Clipping, is_selected: bool) -> impl IntoElement {
    let thumb = item
        .decode_bounded_rgba(IMAGE_THUMB_MAX_W, IMAGE_THUMB_MAX_H)
        .map(|(w, h, rgba)| (w, h, ImageSrc::rgba(w, h, rgba)));

    let meta = format!("{} · {}", item.text, item.formatted_size());

    view()
        .flex_row()
        .items_center()
        .gap_2p5()
        .overflow_hidden()
        .child(thumb.map(|(w, h, src)| {
            #[expect(
                clippy::cast_precision_loss,
                reason = "thumbnail pixel dimensions are <= 160 and exact in f32"
            )]
            let (wf, hf) = (w as f32, h as f32);
            image(src)
                .w(px(wf.min(120.0)))
                .h(px(hf.min(38.0)))
                .rounded(tokens::radius_inset)
                .border_1()
                .border_color(move || theme().border)
        }))
        .child(
            text(meta)
                .text_xs()
                .text_color(move || {
                    if is_selected {
                        theme().primary_foreground
                    } else {
                        theme().muted_foreground
                    }
                })
                .truncate(),
        )
}

fn filter_drawer() -> impl IntoElement {
    view()
        .bg(tokens::toolbar)
        .border_t_1()
        .border_color(move || theme().border)
        .p_2p5()
        .gap_2()
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_between()
                .child(
                    text("Paste with Filter")
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || theme().foreground),
                )
                .child(action_button(
                    "Close",
                    ButtonVariant::Ghost,
                    ControlSize::Sm,
                    false,
                    state::toggle_filter_picker,
                )),
        )
        .child(view().flex_row().flex_wrap().gap_1p5().children(
            TextFilter::ALL.iter().copied().map(|filter| {
                action_button(
                    filter.display_name(),
                    ButtonVariant::Secondary,
                    ControlSize::Sm,
                    false,
                    move || state::paste_selected_with_filter(filter),
                )
            }),
        ))
}

fn preview_drawer() -> impl IntoElement {
    let selected = computed(|| state::store_signal().with(state::AppState::selected_clipping));

    view()
        .bg(tokens::toolbar)
        .border_t_1()
        .border_color(move || theme().border)
        .p_3()
        .gap_2()
        .max_h(px(156.0))
        .child(dynamic(move || {
            let Some(item) = selected.get() else {
                return text("No clipping selected")
                    .text_xs()
                    .text_color(move || theme().muted_foreground)
                    .into_element();
            };

            let app_label = item
                .source_app
                .clone()
                .unwrap_or_else(|| "Unknown app".to_owned());
            let meta = format!("{} · {}", app_label, item.formatted_size());
            let id = item.id;
            let pinned = item.pinned;

            let body = if item.kind == ClippingKind::Image {
                let preview_img = item
                    .decode_bounded_rgba(360, 84)
                    .map(|(w, h, rgba)| (w, h, ImageSrc::rgba(w, h, rgba)));
                view()
                    .items_center()
                    .child(preview_img.map(|(w, h, src)| {
                        #[expect(
                            clippy::cast_precision_loss,
                            reason = "preview dimensions are <= 360 and exact in f32"
                        )]
                        let (wf, hf) = (w as f32, h as f32);
                        image(src).w(px(wf)).h(px(hf)).rounded(tokens::radius_inset)
                    }))
                    .into_element()
            } else {
                scroll()
                    .max_h(px(80.0))
                    .child(
                        text(item.text)
                            .text_xs()
                            .font_family(tokens::mono_font())
                            .text_color(move || theme().foreground),
                    )
                    .into_element()
            };

            view()
                .gap_2()
                .child(body)
                .child(divider())
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(
                            text(meta)
                                .text_xs()
                                .text_color(move || theme().muted_foreground),
                        )
                        .child(
                            view()
                                .flex_row()
                                .gap_1()
                                .child(action_button(
                                    if pinned { "Unpin" } else { "Pin" },
                                    ButtonVariant::Ghost,
                                    ControlSize::Sm,
                                    false,
                                    move || state::toggle_pin(id),
                                ))
                                .child(action_button(
                                    "Delete",
                                    ButtonVariant::Destructive,
                                    ControlSize::Sm,
                                    false,
                                    move || state::remove_clipping(id),
                                )),
                        ),
                )
                .into_element()
        }))
}
