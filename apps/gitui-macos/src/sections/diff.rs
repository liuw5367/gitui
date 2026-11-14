//! Diff pane for History commits and Changes files.

use crate::design;
use crate::model::short_name;
use crate::sections::HunkActions;
use crate::widgets::{header_button, mini_button};
use crate::Mode;
use nsh_base::editor;
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct DiffActions {
    pub(crate) cherry_pick: Arc<dyn Fn(String)>,
    pub(crate) revert: Arc<dyn Fn(String)>,
}

pub(crate) fn diff_view(
    mode: Signal<Mode>,
    selected: Signal<SharedString>,
    diff: Signal<SharedString>,
    selected_file: Signal<String>,
    file_patch: Signal<SharedString>,
    staged_patch: Signal<SharedString>,
    hunk_indices: Signal<Vec<usize>>,
    staged_hunk_indices: Signal<Vec<usize>>,
    actions: DiffActions,
    hunk_actions: HunkActions,
) -> impl IntoElement {
    view()
        .flex_1()
        .min_h_0()
        .flex_col()
        .bg(move || design::content())
        .child(
            view()
                .h(px(38.0))
                .flex_row()
                .items_center()
                .gap_2()
                .px_3()
                .border_b_1()
                .border_color(move || design::line())
                .child(
                    text(move || match mode.get() {
                        Mode::History => format!("Diff · {}", selected.get()),
                        Mode::Changes => {
                            if selected_file.get().is_empty() {
                                "Select a changed file".to_string()
                            } else {
                                format!(
                                    "{} · {}",
                                    if file_patch.get().is_empty() {
                                        "Staged"
                                    } else {
                                        "Unstaged"
                                    },
                                    short_name(&selected_file.get())
                                )
                            }
                        }
                    })
                    .text_size(px(10.5))
                    .text_color(move || design::text2()),
                )
                .child(view().flex_1())
                .child(
                    view()
                        .flex_row()
                        .gap_1()
                        .w(move || {
                            if mode.get() == Mode::History {
                                px(170.0)
                            } else {
                                px(0.0)
                            }
                        })
                        .overflow_hidden()
                        .child(header_button("Cherry-pick", false, {
                            let action = actions.cherry_pick.clone();
                            move || action(selected.get().to_string())
                        }))
                        .child(header_button("Revert", false, {
                            let action = actions.revert.clone();
                            move || action(selected.get().to_string())
                        })),
                ),
        )
        .child(
            view()
                .h(move || {
                    if mode.get() == Mode::Changes
                        && (!file_patch.get().is_empty() || !staged_patch.get().is_empty())
                    {
                        px(30.0)
                    } else {
                        px(0.0)
                    }
                })
                .flex_row()
                .items_center()
                .gap_1()
                .px_2()
                .border_b_1()
                .border_color(move || design::line_soft())
                .child(
                    text("Hunks")
                        .text_size(px(9.5))
                        .text_color(move || design::text3()),
                )
                .child(each(
                    hunk_indices,
                    |index: &usize| index.to_string(),
                    move |index, _| {
                        let stage = hunk_actions.stage.clone();
                        let number = index.get();
                        view()
                            .flex_row()
                            .items_center()
                            .gap_1()
                            .child(
                                text(move || format!("#{}", number + 1))
                                    .text_size(px(9.0))
                                    .text_color(move || design::text3()),
                            )
                            .child(mini_button("Stage", true, move || stage(number)))
                    },
                ))
                .child(each(
                    staged_hunk_indices,
                    |index: &usize| index.to_string(),
                    move |index, _| {
                        let unstage = hunk_actions.unstage.clone();
                        let number = index.get();
                        view()
                            .flex_row()
                            .items_center()
                            .gap_1()
                            .child(
                                text(move || format!("#{}", number + 1))
                                    .text_size(px(9.0))
                                    .text_color(move || design::text3()),
                            )
                            .child(mini_button("Unstage", false, move || unstage(number)))
                    },
                )),
        )
        .child(
            editor()
                .value(move || {
                    if mode.get() == Mode::History {
                        Some(diff.get())
                    } else if !file_patch.get().is_empty() {
                        Some(file_patch.get())
                    } else if !staged_patch.get().is_empty() {
                        Some(staged_patch.get())
                    } else {
                        Some(SharedString::from("Select a changed file to see its patch."))
                    }
                })
                .read_only(true)
                .language("diff")
                .font_size(11.0)
                .line_height(19.0)
                .flex_1(),
        )
}
