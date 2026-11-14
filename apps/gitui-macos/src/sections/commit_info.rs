//! Commit details with per-file diff.

use crate::design;
use crate::widgets::header_button;
use gitui_git::{ChangedFile, CommitDetails};
use nsh_base::editor;
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct CommitInfoActions {
    pub(crate) close: Arc<dyn Fn()>,
    pub(crate) select_file: Arc<dyn Fn(String)>,
}

pub(crate) fn commit_info_panel(
    open: Signal<bool>,
    details: Signal<Option<CommitDetails>>,
    files: Signal<Vec<ChangedFile>>,
    selected_file: Signal<String>,
    file_diff: Signal<SharedString>,
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
                        .flex_col()
                        .gap_1()
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
                        ),
                )
                .child(
                    view()
                        .flex_1()
                        .min_h_0()
                        .flex_row()
                        .border_t_1()
                        .border_color(move || design::line())
                        .child(
                            view()
                                .w(px(280.0))
                                .flex_col()
                                .bg(move || design::panel())
                                .border_r_1()
                                .border_color(move || design::line())
                                .child(
                                    text(move || {
                                        let files = files.get();
                                        let additions: usize =
                                            files.iter().map(|file| file.additions).sum();
                                        let deletions: usize =
                                            files.iter().map(|file| file.deletions).sum();
                                        format!(
                                            "Files · {}  +{} −{}",
                                            files.len(),
                                            additions,
                                            deletions
                                        )
                                    })
                                        .px_2()
                                        .py_1()
                                        .text_size(px(9.5))
                                        .text_color(move || design::text3()),
                                )
                                .child(each(
                                    files,
                                    |file: &ChangedFile| file.path.clone(),
                                    move |file, _| {
                                        let path = file.get().path.clone();
                                        let select = actions.select_file.clone();
                                        view()
                                            .h(px(24.0))
                                            .flex_row()
                                            .items_center()
                                            .gap_2()
                                            .px_2()
                                            .border_b_1()
                                            .border_color(move || design::line_soft())
                                            .bg(move || {
                                                if selected_file.get() == file.get().path {
                                                    design::selected()
                                                } else {
                                                    design::panel()
                                                }
                                            })
                                            .on_press(move || select(path.clone()))
                                            .child(
                                                text(move || file.get().status)
                                                    .w(px(28.0))
                                                    .text_size(px(9.0))
                                                    .text_color(move || design::text3()),
                                            )
                                            .child(
                                                text(move || file.get().path)
                                                    .flex_1()
                                                    .text_size(px(9.5))
                                                    .text_color(move || design::text()),
                                            )
                                            .child(
                                                text(move || {
                                                    format!(
                                                        "+{} −{}",
                                                        file.get().additions,
                                                        file.get().deletions
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
                                .min_w_0()
                                .flex_col()
                                .child(
                                    text(move || selected_file.get())
                                        .px_2()
                                        .py_1()
                                        .text_size(px(9.0))
                                        .text_color(move || design::text3()),
                                )
                                .child(
                                    editor()
                                        .value(move || Some(file_diff.get()))
                                        .read_only(true)
                                        .language("diff")
                                        .font_size(10.0)
                                        .line_height(17.0)
                                        .flex_1(),
                                ),
                        ),
                ),
        )
}
