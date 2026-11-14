//! Reflog & undo sheet.

use crate::design;
use crate::widgets::header_button;
use gitui_git::ReflogEntry;
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct ReflogActions {
    pub(crate) close: Arc<dyn Fn()>,
    pub(crate) reset: Arc<dyn Fn(String)>,
}

pub(crate) fn reflog_panel(
    open: Signal<bool>,
    entries: Signal<Vec<ReflogEntry>>,
    selected: Signal<SharedString>,
    error: Signal<SharedString>,
    actions: ReflogActions,
) -> impl IntoElement {
    let reset_armed = signal(false);
    view()
        .absolute()
        .inset(px(0.0))
        .w_full()
        .h_full()
        .z_index(move || if open.get() { 45 } else { -1 })
        .items_center()
        .justify_center()
        .p(px(24.0))
        .bg(move || {
            if open.get() {
                AdaptiveColor::from(rgb(0x000000)).opacity(0.28)
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .child(
            view()
                .w(px(800.0))
                .h(px(560.0))
                .flex_col()
                .rounded(px(12.0))
                .bg(move || design::content())
                .border_1()
                .border_color(move || design::line())
                .overflow_hidden()
                .child(
                    view()
                        .h(px(44.0))
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .bg(move || design::panel())
                        .border_b_1()
                        .border_color(move || design::line())
                        .child(
                            text("Reflog & undo")
                                .flex_1()
                                .text_size(px(13.0))
                                .text_color(move || design::text()),
                        )
                        .child(header_button("Close", false, {
                            let action = actions.close.clone();
                            move || action()
                        })),
                )
                .child(
                    view()
                        .flex_1()
                        .min_h_0()
                        .flex_row()
                        .child(
                            view()
                                .w(px(340.0))
                                .flex_col()
                                .bg(move || design::panel())
                                .border_r_1()
                                .border_color(move || design::line())
                                .child(each(
                                    entries,
                                    |entry: &ReflogEntry| entry.selector.clone(),
                                    move |entry, _| {
                                        let sha = entry.get().sha.clone();
                                        view()
                                            .h(px(30.0))
                                            .flex_col()
                                            .justify_center()
                                            .px_2()
                                            .border_b_1()
                                            .border_color(move || design::line_soft())
                                            .bg(move || {
                                                if selected.get().to_string() == entry.get().sha {
                                                    design::selected()
                                                } else {
                                                    design::panel()
                                                }
                                            })
                                            .on_press(move || selected.set(SharedString::from(sha.clone())))
                                            .child(
                                                text(move || {
                                                    format!(
                                                        "{} · {}",
                                                        entry.get().selector,
                                                        entry.get().action
                                                    )
                                                })
                                                .text_size(px(10.5))
                                                .text_color(move || design::text()),
                                            )
                                            .child(
                                                text(move || {
                                                    format!(
                                                        "{}  {}",
                                                        entry.get().short,
                                                        entry.get().date
                                                    )
                                                })
                                                .text_size(px(9.0))
                                                .text_color(move || design::text3()),
                                            )
                                    },
                                )),
                        )
                        .child(
                            view()
                                .flex_1()
                                .flex_col()
                                .gap_2()
                                .p_3()
                                .child(
                                    text(move || {
                                        entries
                                            .get()
                                            .iter()
                                            .find(|entry| entry.sha == selected.get().to_string())
                                            .map(|entry| format!("{} · {}", entry.selector, entry.sha))
                                            .unwrap_or_else(|| "Select a reflog entry".to_string())
                                    })
                                    .text_size(px(11.0))
                                    .text_color(move || design::text()),
                                )
                                .child(
                                    text(move || {
                                        entries
                                            .get()
                                            .iter()
                                            .find(|entry| entry.sha == selected.get().to_string())
                                            .map(|entry| entry.message.clone())
                                            .unwrap_or_default()
                                    })
                                    .text_size(px(10.0))
                                    .text_color(move || design::text2()),
                                )
                                .child(view().flex_1())
                                .child(
                                    text("Reset --hard discards uncommitted changes and moves the branch.")
                                        .text_size(px(9.5))
                                        .text_color(move || design::danger()),
                                )
                                .child(
                                    text(move || error.get())
                                        .text_size(px(9.5))
                                        .text_color(move || design::danger()),
                                )
                                .child(
                                    view()
                                        .h(px(26.0))
                                        .px_3()
                                        .rounded(px(6.0))
                                        .bg(move || {
                                            if reset_armed.get() {
                                                design::danger()
                                            } else {
                                                design::panel2()
                                            }
                                        })
                                        .on_press({
                                            let reset = actions.reset.clone();
                                            move || {
                                                let sha = selected.get_untracked().to_string();
                                                if reset_armed.get_untracked() {
                                                    reset_armed.set(false);
                                                    reset(sha);
                                                } else {
                                                    reset_armed.set(true);
                                                }
                                            }
                                        })
                                        .child(
                                            text(move || {
                                                if reset_armed.get() {
                                                    "Confirm reset --hard"
                                                } else {
                                                    "Reset --hard"
                                                }
                                            })
                                            .text_size(px(11.0))
                                            .text_color(move || design::text()),
                                        ),
                                ),
                        ),
                ),
        )
}
