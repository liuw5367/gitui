//! Repository-local commit identity bar.

use nsh_base::{input, theme};
use nsh_core::prelude::*;
use std::sync::Arc;

use crate::widgets::header_button;

#[derive(Clone)]
pub(crate) struct IdentityActions {
    pub(crate) save: Arc<dyn Fn()>,
}

pub(crate) fn identity_bar(
    name: Signal<String>,
    email: Signal<String>,
    actions: IdentityActions,
) -> impl IntoElement {
    view()
        .h(px(32.0))
        .flex_row()
        .items_center()
        .gap_2()
        .px_3()
        .bg(move || theme().background)
        .child(
            text("Identity")
                .text_size(px(9.0))
                .text_color(move || theme().foreground),
        )
        .child(
            input()
                .value(name)
                .on_change(move |text: &str| name.set(text.to_string()))
                .placeholder("user.name")
                .text_size(px(9.0))
                .w(px(180.0)),
        )
        .child(
            input()
                .value(email)
                .on_change(move |text: &str| email.set(text.to_string()))
                .placeholder("user.email")
                .text_size(px(9.0))
                .w(px(240.0)),
        )
        .child(view().flex_1())
        .child(header_button("Save identity", false, {
            let action = actions.save.clone();
            move || action()
        }))
}
