//! Commit history table.

use crate::design;
use crate::widgets::header_button;
use crate::Mode;
use gitui_git::Commit;
use nsh_core::prelude::*;
use std::sync::Arc;

pub(crate) fn history(
    branch: Signal<SharedString>,
    commits: Signal<Vec<Commit>>,
    selected: Signal<SharedString>,
    on_select: Arc<dyn Fn(String)>,
    on_reflog: Arc<dyn Fn()>,
    on_compare: Arc<dyn Fn()>,
    on_file_history: Arc<dyn Fn()>,
    on_info: Arc<dyn Fn()>,
    mode: Signal<Mode>,
) -> impl IntoElement {
    view()
        .h(move || {
            if mode.get() == Mode::History {
                px(256.0)
            } else {
                px(0.0)
            }
        })
        .w_full()
        .flex_col()
        .bg(move || design::content())
        .border_b_1()
        .border_color(move || design::line())
        .child(
            view()
                .h(px(38.0))
                .flex_row()
                .items_center()
                .gap_2()
                .px_3()
                .border_b_1()
                .border_color(move || design::line_soft())
                .child(
                    text("History")
                        .text_size(px(13.0))
                        .text_color(move || design::text()),
                )
                .child(
                    view()
                        .h(px(20.0))
                        .px_2()
                        .rounded(px(5.0))
                        .bg(move || design::selected())
                        .child(
                            text(move || branch.get())
                                .text_size(px(10.5))
                                .text_color(move || design::primary()),
                        ),
                )
                .child(
                    text(move || format!("{} commits", commits.get().len()))
                        .text_size(px(10.5))
                        .text_color(move || design::text3()),
                )
                .child(view().flex_1())
                .child(
                    text("All authors")
                        .text_size(px(10.5))
                        .text_color(move || design::text3()),
                )
                .child(header_button("Reflog", false, move || on_reflog()))
                .child(header_button("Compare", false, move || on_compare()))
                .child(header_button("Files", false, move || on_file_history()))
                .child(header_button("Info", false, move || on_info())),
        )
        .child(
            view()
                .h(px(23.0))
                .flex_row()
                .items_center()
                .gap_2()
                .px_3()
                .border_b_1()
                .border_color(move || design::line_soft())
                .child(view().w(px(56.0)))
                .child(
                    text("Description")
                        .flex_1()
                        .text_size(px(10.5))
                        .text_color(move || design::text3()),
                )
                .child(
                    text("Author")
                        .w(px(96.0))
                        .text_size(px(10.5))
                        .text_color(move || design::text3()),
                )
                .child(
                    text("Date")
                        .w(px(52.0))
                        .text_size(px(10.5))
                        .text_color(move || design::text3()),
                ),
        )
        .child(
            view()
                .flex_1()
                .min_h_0()
                .flex_col()
                .child(each(
                    commits,
                    |commit: &Commit| commit.sha.clone(),
                    move |commit, _| {
                        let sha = commit.get().sha.clone();
                        let on_select = on_select.clone();
                        view()
                            .h(px(38.0))
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .px_3()
                            .border_b_1()
                            .border_l(move || {
                                if selected.get().to_string() == commit.get().sha {
                                    px(2.0)
                                } else {
                                    px(0.0)
                                }
                            })
                            .border_color(move || design::primary())
                            .bg(move || {
                                if selected.get().to_string() == commit.get().sha {
                                    design::selected()
                                } else {
                                    design::content()
                                }
                            })
                            .on_press(move || on_select(sha.clone()))
                            .child(
                                view()
                                    .w(px(56.0))
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        view()
                                            .w(px(8.0))
                                            .h(px(8.0))
                                            .rounded(px(4.0))
                                            .bg(move || design::primary()),
                                    ),
                            )
                            .child(
                                view()
                                    .flex_1()
                                    .min_w_0()
                                    .flex_col()
                                    .child(
                                        text(move || commit.get().summary.clone())
                                            .text_size(px(12.0))
                                            .text_color(move || design::text()),
                                    )
                                    .child(
                                        text(move || commit.get().short.clone())
                                            .text_size(px(10.0))
                                            .text_color(move || design::text3()),
                                    ),
                            )
                            .child(
                                text(move || commit.get().author.clone())
                                    .w(px(96.0))
                                    .text_size(px(10.5))
                                    .text_color(move || design::text2()),
                            )
                            .child(
                                text(move || commit.get().date.clone())
                                    .w(px(52.0))
                                    .text_size(px(10.5))
                                    .text_color(move || design::text3()),
                            )
                    },
                )),
        )
}
