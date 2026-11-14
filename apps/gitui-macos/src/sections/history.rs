//! Commit history table.

use crate::design;
use crate::widgets::header_button;
use crate::Mode;
use gitui_git::Commit;
use nsh_base::input;
use nsh_core::prelude::*;
use std::sync::Arc;

fn commit_matches(commit: &Commit, query: &str) -> bool {
    let query = query.to_lowercase();
    query.is_empty()
        || commit.summary.to_lowercase().contains(&query)
        || commit.author.to_lowercase().contains(&query)
        || commit.short.contains(&query)
        || commit
            .refs
            .iter()
            .any(|reference| reference.to_lowercase().contains(&query))
}

fn ref_chip(label: String) -> impl IntoElement {
    let is_tag = label.starts_with("tag: ");
    let is_remote = label.contains('/');
    view()
        .h(px(16.0))
        .px_1()
        .rounded(px(3.0))
        .bg(move || {
            if is_tag {
                design::tag_bg()
            } else if is_remote {
                design::panel2()
            } else {
                design::branch_bg()
            }
        })
        .child(
            text(label)
                .text_size(px(8.5))
                .text_color(move || {
                    if is_tag {
                        design::tag_fg()
                    } else if is_remote {
                        design::text2()
                    } else {
                        design::branch_fg()
                    }
                }),
        )
}

pub(crate) fn history(
    branch: Signal<SharedString>,
    commits: Signal<Vec<Commit>>,
    selected: Signal<SharedString>,
    on_select: Arc<dyn Fn(String)>,
    on_compare: Arc<dyn Fn()>,
    mode: Signal<Mode>,
    focus: Signal<bool>,
) -> impl IntoElement {
    let filter = signal(String::new());
    view()
        .h(move || {
            if !focus.get() && mode.get() == Mode::History {
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
                    text(move || {
                        let query = filter.get();
                        let count = commits
                            .get()
                            .iter()
                            .filter(|commit| commit_matches(commit, &query))
                            .count();
                        format!("{} commits", count)
                    })
                        .text_size(px(10.5))
                        .text_color(move || design::text3()),
                )
                .child(view().flex_1())
                .child(
                    input()
                        .value(filter)
                        .on_change(move |text: &str| filter.set(text.to_string()))
                        .placeholder("Filter commits")
                        .text_size(px(10.5))
                        .w(px(150.0)),
                )
                .child(header_button("Compare", false, move || on_compare())),
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
                .child(
                    view()
                        .h(move || {
                            if commits
                                .get()
                                .iter()
                                .any(|commit| commit_matches(commit, &filter.get()))
                            {
                                px(0.0)
                            } else {
                                px(40.0)
                            }
                        })
                        .items_center()
                        .justify_center()
                        .child(
                            text("No commits match")
                                .text_size(px(10.5))
                                .text_color(move || design::text3()),
                        ),
                )
                .child(each(
                    commits,
                    |commit: &Commit| commit.sha.clone(),
                    move |commit, _| {
                        let sha = commit.get().sha.clone();
                        let on_select = on_select.clone();
                        view()
                            .h(move || {
                                if commit_matches(&commit.get(), &filter.get()) {
                                    px(38.0)
                                } else {
                                    px(0.0)
                                }
                            })
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
                                        view()
                                            .flex_row()
                                            .items_center()
                                            .gap_1()
                                            .child(
                                                text(move || commit.get().short.clone())
                                                    .text_size(px(9.5))
                                                    .text_color(move || design::text3()),
                                            )
                                            .child({
                                                let mut chips = view().flex_row().items_center().gap_1();
                                                for label in commit.get_untracked().refs {
                                                    chips = chips.child(ref_chip(label));
                                                }
                                                chips
                                            }),
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
