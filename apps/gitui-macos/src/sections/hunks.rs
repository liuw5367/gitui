//! Hunk staging bar for the selected file.

use nsh_base::{editor, theme};
use nsh_core::prelude::*;
use std::sync::Arc;

use crate::widgets::mini_button;

#[derive(Clone)]
pub(crate) struct HunkActions {
    pub(crate) stage: Arc<dyn Fn(usize)>,
    pub(crate) unstage: Arc<dyn Fn(usize)>,
}

pub(crate) fn hunks_bar(
    selected_file: Signal<String>,
    hunk_indices: Signal<Vec<usize>>,
    staged_hunk_indices: Signal<Vec<usize>>,
    patch: Signal<SharedString>,
    actions: HunkActions,
) -> impl IntoElement {
    view()
        .h(move || {
            if patch.get().is_empty() {
                px(30.0)
            } else {
                px(150.0)
            }
        })
        .flex_col()
        .gap_1()
        .px_3()
        .py_1()
        .bg(move || theme().background)
        .child(
            view()
                .h(px(28.0))
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    text(move || {
                        if selected_file.get().is_empty() {
                            "Hunks · select a changed file".to_string()
                        } else {
                            format!("Hunks · {}", selected_file.get())
                        }
                    })
                    .text_size(px(9.0))
                    .text_color(move || theme().foreground),
                )
                .child(each(
                    hunk_indices,
                    |index: &usize| index.to_string(),
                    move |index, _| {
                        let stage = actions.stage.clone();
                        let number = index.get();
                        view()
                            .flex_row()
                            .items_center()
                            .gap_1()
                            .child(
                                text(move || format!("#{}", number + 1))
                                    .text_size(px(8.5))
                                    .text_color(move || theme().foreground),
                            )
                            .child(mini_button("Stage", true, move || stage(number)))
                    },
                )),
        )
        .child(
            view()
                .h(px(24.0))
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    text("Staged")
                        .text_size(px(8.5))
                        .text_color(move || theme().foreground),
                )
                .child(each(
                    staged_hunk_indices,
                    |index: &usize| index.to_string(),
                    move |index, _| {
                        let unstage = actions.unstage.clone();
                        let number = index.get();
                        view()
                            .flex_row()
                            .items_center()
                            .gap_1()
                            .child(
                                text(move || format!("#{}", number + 1))
                                    .text_size(px(8.5))
                                    .text_color(move || theme().foreground),
                            )
                            .child(mini_button("Unstage", false, move || unstage(number)))
                    },
                )),
        )
        .child(
            editor()
                .value(move || Some(patch.get()))
                .read_only(true)
                .language("diff")
                .font_size(9.0)
                .line_height(14.0)
                .flex_1(),
        )
}
