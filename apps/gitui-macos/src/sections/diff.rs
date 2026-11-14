//! Read-only selected-commit diff with commit actions.

use nsh_base::{editor, theme};
use nsh_core::prelude::*;
use std::sync::Arc;

use crate::widgets::header_button;

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
        .flex_col()
        .gap_2()
        .p_2()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    text(move || format!("Diff · {}", selected.get()))
                        .text_size(px(10.0))
                        .text_color(move || theme().foreground),
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
                .line_height(17.0)
                .flex_1(),
        )
}
