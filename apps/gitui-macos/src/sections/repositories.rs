//! Repositories / recent repos sheet.

use crate::design;
use crate::widgets::{header_button, mini_button};
use nsh_base::input;
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct RepositoriesActions {
    pub(crate) close: Arc<dyn Fn()>,
    pub(crate) open: Arc<dyn Fn()>,
    pub(crate) open_recent: Arc<dyn Fn(String)>,
    pub(crate) remove_recent: Arc<dyn Fn(String)>,
}

pub(crate) fn repositories_panel(
    open: Signal<bool>,
    recent: Signal<Vec<SharedString>>,
    path_input: Signal<String>,
    actions: RepositoriesActions,
) -> impl IntoElement {
    view()
        .absolute()
        .inset(px(0.0))
        .w_full()
        .h_full()
        .z_index(move || if open.get() { 53 } else { -1 })
        .items_center()
        .justify_center()
        .p(px(24.0))
        .bg(move || {
            if open.get() {
                AdaptiveColor::from(rgb(0x000000)).opacity(0.28)
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .child(
            view()
                .w(px(720.0))
                .h(px(520.0))
                .flex_col()
                .rounded(px(12.0))
                .bg(move || design::content())
                .border_1()
                .border_color(move || design::line())
                .overflow_hidden()
                .child(
                    view()
                        .h(px(44.0))
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .bg(move || design::panel())
                        .border_b_1()
                        .border_color(move || design::line())
                        .child(
                            text("Repositories")
                                .flex_1()
                                .text_size(px(13.0))
                                .text_color(move || design::text()),
                        )
                        .child(header_button("Close", false, {
                            let action = actions.close.clone();
                            move || action()
                        })),
                )
                .child(
                    view()
                        .flex_1()
                        .min_h_0()
                        .flex_col()
                        .gap_1()
                        .p_3()
                        .child(
                            text("Recent")
                                .text_size(px(10.0))
                                .text_color(move || design::text3()),
                        )
                        .child(each(
                            recent,
                            |path: &SharedString| path.clone(),
                            move |path, _| {
                                let open_recent = actions.open_recent.clone();
                                let remove_recent = actions.remove_recent.clone();
                                let remove_path = path.get().to_string();
                                view()
                                    .h(px(32.0))
                                    .flex_row()
                                    .items_center()
                                    .gap_2()
                                    .px_1()
                                    .rounded(px(5.0))
                                    .border_b_1()
                                    .border_color(move || design::line_soft())
                                    .on_press(move || open_recent(path.get().to_string()))
                                    .child(
                                        text(move || path.get())
                                            .flex_1()
                                            .text_size(px(10.5))
                                            .text_color(move || design::text()),
                                    )
                                    .child(mini_button("Remove", false, move || {
                                        remove_recent(remove_path.clone())
                                    }))
                            },
                        ))
                        .child(view().flex_1())
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .child(
                                    input()
                                        .value(path_input)
                                        .on_change(move |text: &str| {
                                            path_input.set(text.to_string())
                                        })
                                        .placeholder("Repository path")
                                        .text_size(px(11.0))
                                        .flex_1(),
                                )
                                .child(header_button("Open", true, {
                                    let action = actions.open.clone();
                                    move || action()
                                })),
                        ),
                ),
        )
}
