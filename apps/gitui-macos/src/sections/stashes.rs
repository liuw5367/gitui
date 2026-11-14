//! Stash bar.

use gitui_git::Stash;
use nsh_base::theme;
use nsh_core::prelude::*;
use std::sync::Arc;

use crate::widgets::{header_button, mini_button};

#[derive(Clone)]
pub(crate) struct StashActions {
    pub(crate) push: Arc<dyn Fn()>,
    pub(crate) apply: Arc<dyn Fn(String)>,
    pub(crate) pop: Arc<dyn Fn(String)>,
    pub(crate) drop: Arc<dyn Fn(String)>,
}

pub(crate) fn stashes_bar(
    stashes: Signal<Vec<Stash>>,
    actions: StashActions,
) -> impl IntoElement {
    view()
        .h(px(32.0))
        .flex_row()
        .items_center()
        .gap_2()
        .px_3()
        .bg(move || theme().background)
        .child(
            text("Stashes")
                .text_size(px(9.0))
                .text_color(move || theme().foreground),
        )
        .child(each(
            stashes,
            |stash: &Stash| stash.selector.clone(),
            move |stash, _| {
                let selector = stash.get().selector.clone();
                let apply_selector = selector.clone();
                let pop_selector = selector.clone();
                let drop_selector = selector;
                let apply = actions.apply.clone();
                let pop = actions.pop.clone();
                let drop = actions.drop.clone();
                view()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .child(
                        text(move || stash.get().message)
                            .text_size(px(9.0))
                            .text_color(move || theme().foreground),
                    )
                    .child(mini_button("Apply", true, move || apply(apply_selector.clone())))
                    .child(mini_button("Pop", false, move || pop(pop_selector.clone())))
                    .child(mini_button("Drop", false, move || drop(drop_selector.clone())))
            },
        ))
        .child(view().flex_1())
        .child(header_button("Stash", false, {
            let action = actions.push.clone();
            move || action()
        }))
}
