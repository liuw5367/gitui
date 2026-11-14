//! Top header: repository input, branch/ahead/behind, sync actions.

use crate::widgets::header_button;
use nsh_base::{input, theme};
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct HeaderActions {
    pub(crate) open: Arc<dyn Fn()>,
    pub(crate) close: Arc<dyn Fn()>,
    pub(crate) fetch: Arc<dyn Fn()>,
    pub(crate) pull: Arc<dyn Fn()>,
    pub(crate) push: Arc<dyn Fn()>,
}

pub(crate) fn header(
    path_input: Signal<String>,
    branch: Signal<SharedString>,
    ahead: Signal<usize>,
    behind: Signal<usize>,
    actions: HeaderActions,
) -> impl IntoElement {
    view()
        .h(px(46.0))
        .flex_row()
        .items_center()
        .gap_3()
        .px_3()
        .bg(move || theme().background)
        .child(
            text("gitui · local")
                .text_size(px(13.0))
                .text_color(move || theme().foreground),
        )
        .child(
            input()
                .value(path_input)
                .on_change(move |text: &str| path_input.set(text.to_string()))
                .placeholder("Repository path")
                .text_size(px(11.0))
                .w(px(420.0)),
        )
        .child(
            text(move || format!("{} ↑{} ↓{}", branch.get(), ahead.get(), behind.get()))
                .text_size(px(10.0))
                .text_color(move || theme().primary),
        )
        .child(view().flex_1())
        .child(header_button("Open", true, {
            let action = actions.open.clone();
            move || action()
        }))
        .child(header_button("Close tab", false, {
            let action = actions.close.clone();
            move || action()
        }))
        .child(header_button("Fetch", false, {
            let action = actions.fetch.clone();
            move || action()
        }))
        .child(header_button("Pull", false, {
            let action = actions.pull.clone();
            move || action()
        }))
        .child(header_button("Push", false, {
            let action = actions.push.clone();
            move || action()
        }))
}
