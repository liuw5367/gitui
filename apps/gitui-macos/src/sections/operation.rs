//! In-progress merge/rebase banner.

use gitui_git::Operation;
use nsh_base::theme;
use nsh_core::prelude::*;
use std::sync::Arc;

use crate::widgets::header_button;

pub(crate) fn operation_banner(
    operation: Signal<Option<Operation>>,
    on_continue: Arc<dyn Fn()>,
    on_abort: Arc<dyn Fn()>,
) -> impl IntoElement {
    view()
        .h(move || {
            if operation.get().is_some() {
                px(28.0)
            } else {
                px(0.0)
            }
        })
        .flex_row()
        .items_center()
        .gap_2()
        .px_3()
        .bg(move || theme().primary)
        .child(
            text(move || {
                operation
                    .get()
                    .map(|op| format!("{} in progress", op.label()))
                    .unwrap_or_default()
            })
            .text_size(px(10.0))
            .text_color(move || theme().primary_foreground),
        )
        .child(view().flex_1())
        .child(header_button("Continue", true, {
            let action = on_continue.clone();
            move || action()
        }))
        .child(header_button("Abort", false, {
            let action = on_abort.clone();
            move || action()
        }))
}
