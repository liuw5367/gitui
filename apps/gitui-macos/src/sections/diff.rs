//! Selected-commit diff pane.

use crate::design;
use crate::widgets::header_button;
use nsh_base::editor;
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct DiffActions {
    pub(crate) cherry_pick: Arc<dyn Fn(String)>,
    pub(crate) revert: Arc<dyn Fn(String)>,
}

pub(crate) fn diff_view(
    selected: Signal<SharedString>,
    diff: Signal<SharedString>,
    actions: DiffActions,
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
                    text(move || format!("Diff · {}", selected.get()))
                        .text_size(px(10.5))
                        .text_color(move || design::text2()),
                )
                .child(view().flex_1())
                .child(header_button("Cherry-pick", false, {
                    let action = actions.cherry_pick.clone();
                    move || action(selected.get().to_string())
                }))
                .child(header_button("Revert", false, {
                    let action = actions.revert.clone();
                    move || action(selected.get().to_string())
                })),
        )
        .child(
            editor()
                .value(move || Some(diff.get()))
                .read_only(true)
                .language("diff")
                .font_size(11.0)
                .line_height(19.0)
                .flex_1(),
        )
}
