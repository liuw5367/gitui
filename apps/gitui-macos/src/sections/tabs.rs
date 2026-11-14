//! Repository tab bar.

use crate::model::{short_name, Loaded};
use nsh_base::theme;
use nsh_core::prelude::*;
use std::sync::Arc;

pub(crate) fn tabs(
    tabs: Signal<Vec<Loaded>>,
    active: Signal<usize>,
    activate: Arc<dyn Fn(usize)>,
) -> impl IntoElement {
    view()
        .h(px(30.0))
        .flex_row()
        .items_center()
        .gap_1()
        .px_3()
        .bg(move || theme().background)
        .child(each(
            tabs,
            |tab: &Loaded| tab.path.clone(),
            move |tab, _| {
                let index = tab.get().path.clone();
                let activate = activate.clone();
                view()
                    .h(px(24.0))
                    .px_2()
                    .rounded(px(4.0))
                    .bg(move || {
                        let path = tab.get().path;
                        if active.get()
                            == tabs
                                .get_untracked()
                                .iter()
                                .position(|item| item.path == path)
                                .unwrap_or(usize::MAX)
                        {
                            theme().primary
                        } else {
                            theme().background
                        }
                    })
                    .on_press(move || {
                        let index = tabs
                            .get_untracked()
                            .iter()
                            .position(|item| item.path == index)
                            .unwrap_or(0);
                        activate(index);
                    })
                    .child(
                        text(move || short_name(&tab.get().path))
                            .text_size(px(10.0))
                            .text_color(move || {
                                let index = tabs
                                    .get_untracked()
                                    .iter()
                                    .position(|item| item.path == tab.get().path)
                                    .unwrap_or(usize::MAX);
                                if active.get() == index {
                                    theme().primary_foreground
                                } else {
                                    theme().foreground
                                }
                            }),
                    )
            },
        ))
}
