//! Commit composer bar.

use nsh_base::{input, theme};
use nsh_core::prelude::*;
use std::sync::Arc;

use crate::widgets::header_button;

#[derive(Clone)]
pub(crate) struct CommitActions {
    pub(crate) commit: Arc<dyn Fn()>,
    pub(crate) no_hooks: Arc<dyn Fn()>,
    pub(crate) amend: Arc<dyn Fn()>,
}

pub(crate) fn commit_bar(
    message: Signal<String>,
    actions: CommitActions,
) -> impl IntoElement {
    view()
        .h(px(38.0))
        .flex_row()
        .items_center()
        .gap_2()
        .px_3()
        .bg(move || theme().background)
        .child(
            input()
                .value(message)
                .on_change(move |text: &str| message.set(text.to_string()))
                .placeholder("Commit message")
                .flex_1(),
        )
        .child(header_button("Commit", true, {
            let action = actions.commit.clone();
            move || action()
        }))
        .child(header_button("No hooks", false, {
            let action = actions.no_hooks.clone();
            move || action()
        }))
        .child(header_button("Amend", false, {
            let action = actions.amend.clone();
            move || action()
        }))
}
