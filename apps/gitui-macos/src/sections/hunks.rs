//! Hunk staging bar for the selected file.

use nsh_base::theme;
use nsh_core::prelude::*;
use std::sync::Arc;

use crate::widgets::mini_button;

#[derive(Clone)]
pub(crate) struct HunkActions {
    pub(crate) stage: Arc<dyn Fn(usize)>,
}

pub(crate) fn hunks_bar(
    selected_file: Signal<String>,
    hunk_indices: Signal<Vec<usize>>,
    actions: HunkActions,
) -> impl IntoElement {
    view()
        .h(px(30.0))
        .flex_row()
        .items_center()
        .gap_2()
        .px_3()
        .bg(move || theme().background)
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
        ))
}
