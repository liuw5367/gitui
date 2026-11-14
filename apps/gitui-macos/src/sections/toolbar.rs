//! Repository toolbar: open/close and sync actions.

use crate::sections::header::HeaderActions;
use crate::design;
use crate::widgets::header_button;
use nsh_base::input;
use nsh_core::prelude::*;

pub(crate) fn toolbar(
    path_input: Signal<String>,
    branch: Signal<SharedString>,
    ahead: Signal<usize>,
    behind: Signal<usize>,
    actions: HeaderActions,
) -> impl IntoElement {
    view()
        .h(px(44.0))
        .flex_row()
        .items_center()
        .gap_2()
        .px_3()
        .bg(move || design::toolbar())
        .border_b_1()
        .border_color(move || design::line())
        .child(
            input()
                .value(path_input)
                .on_change(move |text: &str| path_input.set(text.to_string()))
                .placeholder("Repository path")
                .text_size(px(12.0))
                .w(px(240.0)),
        )
        .child(header_button("Open", true, {
            let action = actions.open.clone();
            move || action()
        }))
        .child(header_button("Close tab", false, {
            let action = actions.close.clone();
            move || action()
        }))
        .child(
            view()
                .w(px(1.0))
                .h(px(18.0))
                .bg(move || design::line()),
        )
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
        .child(view().flex_1())
        .child(
            text(move || format!("{} ↑{} ↓{}", branch.get(), ahead.get(), behind.get()))
                .text_size(px(10.5))
                .text_color(move || design::text2()),
        )
}
