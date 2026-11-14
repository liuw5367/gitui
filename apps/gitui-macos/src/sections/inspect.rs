//! Compare refs and per-file history sheet.

use crate::design;
use crate::widgets::header_button;
use crate::InspectTab;
use gitui_git::Commit;
use nsh_base::{editor, input};
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct InspectActions {
    pub(crate) close: Arc<dyn Fn()>,
    pub(crate) compare: Arc<dyn Fn()>,
    pub(crate) history: Arc<dyn Fn()>,
    pub(crate) select_commit: Arc<dyn Fn(String)>,
}

fn tab(
    label: &'static str,
    target: InspectTab,
    current: Signal<InspectTab>,
    on_tab: Arc<dyn Fn(InspectTab)>,
) -> impl IntoElement {
    view()
        .h(px(22.0))
        .px_2()
        .rounded(px(4.0))
        .bg(move || {
            if current.get() == target {
                design::content()
            } else {
                design::panel2()
            }
        })
        .on_press(move || on_tab(target))
        .child(
            text(label)
                .text_size(px(10.5))
                .text_color(move || {
                    if current.get() == target {
                        design::text()
                    } else {
                        design::text2()
                    }
                }),
        )
}

pub(crate) fn inspect_panel(
    open: Signal<bool>,
    current: Signal<InspectTab>,
    from: Signal<String>,
    to: Signal<String>,
    path: Signal<String>,
    diff: Signal<SharedString>,
    commits: Signal<Vec<Commit>>,
    selected: Signal<SharedString>,
    error: Signal<SharedString>,
    actions: InspectActions,
) -> impl IntoElement {
    view()
        .absolute()
        .inset(px(0.0))
        .w_full()
        .h_full()
        .z_index(move || if open.get() { 46 } else { -1 })
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
                .w(px(900.0))
                .h(px(620.0))
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
                            text("Inspect")
                                .text_size(px(13.0))
                                .text_color(move || design::text()),
                        )
                        .child(
                            view()
                                .h(px(26.0))
                                .flex_row()
                                .items_center()
                                .gap_1()
                                .p_1()
                                .rounded(px(6.0))
                                .bg(move || design::panel2())
                                .child(tab(
                                    "Compare",
                                    InspectTab::Compare,
                                    current,
                                    Arc::new(move |next| current.set(next)),
                                ))
                                .child(tab(
                                    "File history",
                                    InspectTab::FileHistory,
                                    current,
                                    Arc::new(move |next| current.set(next)),
                                )),
                        )
                        .child(view().flex_1())
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
                        .child(
                            view()
                                .h(move || {
                                    if current.get() == InspectTab::Compare {
                                        px(44.0)
                                    } else {
                                        px(0.0)
                                    }
                                })
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .px_3()
                                .border_b_1()
                                .border_color(move || design::line())
                                .child(
                                    input()
                                        .value(from)
                                        .on_change(move |text: &str| from.set(text.to_string()))
                                        .placeholder("from ref")
                                        .text_size(px(11.0))
                                        .w(px(180.0)),
                                )
                                .child(
                                    text("→")
                                        .text_size(px(11.0))
                                        .text_color(move || design::text3()),
                                )
                                .child(
                                    input()
                                        .value(to)
                                        .on_change(move |text: &str| to.set(text.to_string()))
                                        .placeholder("to ref")
                                        .text_size(px(11.0))
                                        .w(px(180.0)),
                                )
                                .child(header_button("Compare", true, {
                                    let action = actions.compare.clone();
                                    move || action()
                                })),
                        )
                        .child(
                            view()
                                .h(move || {
                                    if current.get() == InspectTab::FileHistory {
                                        px(44.0)
                                    } else {
                                        px(0.0)
                                    }
                                })
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .px_3()
                                .border_b_1()
                                .border_color(move || design::line())
                                .child(
                                    input()
                                        .value(path)
                                        .on_change(move |text: &str| path.set(text.to_string()))
                                        .placeholder("path, e.g. src/main.rs")
                                        .text_size(px(11.0))
                                        .flex_1(),
                                )
                                .child(header_button("Load history", true, {
                                    let action = actions.history.clone();
                                    move || action()
                                })),
                        )
                        .child(
                            view()
                                .h(move || {
                                    if current.get() == InspectTab::FileHistory {
                                        px(220.0)
                                    } else {
                                        px(0.0)
                                    }
                                })
                                .flex_col()
                                .border_b_1()
                                .border_color(move || design::line())
                                .child(each(
                                    commits,
                                    |commit: &Commit| commit.sha.clone(),
                                    move |commit, _| {
                                        let sha = commit.get().sha.clone();
                                        let select = actions.select_commit.clone();
                                        view()
                                            .h(px(32.0))
                                            .flex_col()
                                            .justify_center()
                                            .px_2()
                                            .border_b_1()
                                            .border_color(move || design::line_soft())
                                            .bg(move || {
                                                if selected.get().to_string() == commit.get().sha {
                                                    design::selected()
                                                } else {
                                                    design::content()
                                                }
                                            })
                                            .on_press(move || select(sha.clone()))
                                            .child(
                                                text(move || commit.get().summary.clone())
                                                    .text_size(px(10.5))
                                                    .text_color(move || design::text()),
                                            )
                                            .child(
                                                text(move || {
                                                    format!(
                                                        "{}  {}  {}",
                                                        commit.get().short,
                                                        commit.get().author,
                                                        commit.get().date
                                                    )
                                                })
                                                .text_size(px(9.0))
                                                .text_color(move || design::text3()),
                                            )
                                    },
                                )),
                        )
                        .child(
                            text(move || error.get())
                                .px_3()
                                .text_size(px(9.5))
                                .text_color(move || design::danger()),
                        )
                        .child(
                            editor()
                                .value(move || Some(diff.get()))
                                .read_only(true)
                                .language("diff")
                                .font_size(11.0)
                                .line_height(19.0)
                                .flex_1(),
                        ),
                ),
        )
}
