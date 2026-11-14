//! Commit details sheet.

use crate::design;
use crate::widgets::header_button;
use gitui_git::{ChangedFile, CommitDetails};
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct CommitInfoActions {
    pub(crate) close: Arc<dyn Fn()>,
}

pub(crate) fn commit_info_panel(
    open: Signal<bool>,
    details: Signal<Option<CommitDetails>>,
    files: Signal<Vec<ChangedFile>>,
    actions: CommitInfoActions,
) -> impl IntoElement {
    view()
        .absolute()
        .inset(px(0.0))
        .w_full()
        .h_full()
        .z_index(move || if open.get() { 51 } else { -1 })
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
                .w(px(760.0))
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
                            text(move || {
                                details
                                    .get()
                                    .map(|details| format!("Commit {}", details.short))
                                    .unwrap_or_else(|| "Commit".to_string())
                            })
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
                        .flex_col()
                        .gap_2()
                        .p_3()
                        .child(
                            text(move || {
                                details
                                    .get()
                                    .map(|details| {
                                        format!(
                                            "{} <{}>  ·  {}",
                                            details.author, details.email, details.date
                                        )
                                    })
                                    .unwrap_or_default()
                            })
                            .text_size(px(10.5))
                            .text_color(move || design::text2()),
                        )
                        .child(
                            text(move || {
                                details
                                    .get()
                                    .map(|details| {
                                        if details.parents.is_empty() {
                                            "root commit".to_string()
                                        } else {
                                            format!("parents: {}", details.parents.join(" "))
                                        }
                                    })
                                    .unwrap_or_default()
                            })
                            .text_size(px(9.5))
                            .text_color(move || design::text3()),
                        )
                        .child(
                            text(move || {
                                details
                                    .get()
                                    .map(|details| details.refs)
                                    .unwrap_or_default()
                            })
                            .text_size(px(9.5))
                            .text_color(move || design::text3()),
                        )
                        .child(
                            text(move || {
                                details
                                    .get()
                                    .map(|details| details.message)
                                    .unwrap_or_default()
                            })
                            .text_size(px(11.0))
                            .text_color(move || design::text()),
                        )
                        .child(
                            view().h(px(1.0)).bg(move || design::line_soft()),
                        )
                        .child(
                            text(move || format!("Files · {}", files.get().len()))
                                .text_size(px(10.0))
                                .text_color(move || design::text3()),
                        )
                        .child(
                            view()
                                .flex_1()
                                .min_h_0()
                                .flex_col()
                                .child(each(
                                    files,
                                    |file: &ChangedFile| file.path.clone(),
                                    |file, _| {
                                        view()
                                            .h(px(24.0))
                                            .flex_row()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                text(move || file.get().status)
                                                    .w(px(36.0))
                                                    .text_size(px(9.5))
                                                    .text_color(move || design::text3()),
                                            )
                                            .child(
                                                text(move || file.get().path)
                                                    .text_size(px(10.0))
                                                    .text_color(move || design::text()),
                                            )
                                    },
                                )),
                        ),
                ),
        )
}
