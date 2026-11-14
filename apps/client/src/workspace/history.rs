//! 历史表：`.hist` 256 高，含工具条（32）、列标题（23）与 28 高的提交行。
//!
//! 行内是设计稿的四列：图线 64、说明（含引用徽章）、作者 96、时间 58。P1 的图线
//! 只画当前单条主线与合并标记；多泳道图形在 P2 补。

use crate::design::{self, metrics};
use crate::i18n;
use crate::model::Tab;
use crate::widgets;
use crate::Actions;
use nsh_core::prelude::*;

/// 提交行的时间列宽度；作者列宽度固定，说明列占满剩余空间。
const AUTHOR_W: f32 = 96.0;
const DATE_W: f32 = 58.0;
const GRAPH_W: f32 = 64.0;

pub(crate) fn history(
    language: Signal<SharedString>,
    tab: Tab,
    query: Signal<String>,
    actions: Actions,
) -> impl IntoElement {
    let commits = tab.commits.clone();
    let selected = tab.selected;
    let select = actions.select_commit.clone();
    let visible = {
        let commits = commits.clone();
        computed(move || {
            let needle = query.get().to_lowercase();
            let mut rows: Vec<(usize, domain::Commit)> = Vec::new();
            for (index, commit) in commits.iter().enumerate() {
                if needle.is_empty()
                    || commit.summary.to_lowercase().contains(&needle)
                    || commit.author.to_lowercase().contains(&needle)
                    || commit.short.contains(&needle)
                    || commit
                        .refs
                        .iter()
                        .any(|reference| reference.to_lowercase().contains(&needle))
                {
                    rows.push((index, commit.clone()));
                }
            }
            return rows;
        })
    };
    let total = commits.len();
    let now = store::now_unix();
    let language_value = language;
    return view()
        .w_full()
        .h(px(metrics::HISTORY_H))
        .flex_shrink_0()
        .flex_col()
        .bg(move || design::content())
        .border_b_1()
        .border_color(move || design::line())
        .child(
            view()
                .w_full()
                .h(px(metrics::HIST_BAR_H))
                .flex_row()
                .items_center()
                .gap(px(7.0))
                .px(px(10.0))
                .border_b_1()
                .border_color(move || design::line_soft())
                .child(
                    text(move || i18n::t(language_value.get().as_str(), "shell.history"))
                        .text_size(px(12.5))
                        .font_weight(FontWeight(600))
                        .text_color(move || design::text()),
                )
                .child(
                    view()
                        .h(px(18.0))
                        .px(px(6.0))
                        .items_center()
                        .rounded(px(4.0))
                        .bg(move || design::branch_bg())
                        .child(
                            text(move || tab_branch(&tab))
                                .text_size(px(9.5))
                                .text_color(move || design::branch_fg()),
                        ),
                )
                .child(
                    text(move || {
                        i18n::with_arguments(
                            language_value.get().as_str(),
                            "history.commits",
                            &[("count", total.to_string())],
                        )
                    })
                    .text_size(px(10.5))
                    .text_color(move || design::text3()),
                )
                .child(view().flex_1())
                .child(
                    text(move || {
                        let visible = visible.get().len();
                        if visible == total {
                            String::new()
                        } else {
                            i18n::with_arguments(
                                language_value.get().as_str(),
                                "history.commits",
                                &[("count", visible.to_string())],
                            )
                        }
                    })
                    .text_size(px(10.5))
                    .text_color(move || design::text3()),
                )
                .child(widgets::icon_button(
                    "filter",
                    12.0,
                    design::text2,
                    || {},
                )),
        )
        .child(
            view()
                .w_full()
                .h(px(metrics::COMMIT_COLS_H))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .px(px(8.0))
                .border_b_1()
                .border_color(move || design::line_soft())
                .child(view().w(px(GRAPH_W - 8.0)))
                .child(
                    text(move || {
                        i18n::t(language_value.get().as_str(), "history.description")
                    })
                    .flex_1()
                    .text_size(px(9.5))
                    .font_weight(FontWeight(600))
                    .text_color(move || design::text3()),
                )
                .child(
                    text(move || i18n::t(language_value.get().as_str(), "history.author"))
                        .w(px(AUTHOR_W))
                        .text_size(px(9.5))
                        .font_weight(FontWeight(600))
                        .text_color(move || design::text3()),
                )
                .child(
                    text(move || i18n::t(language_value.get().as_str(), "history.date"))
                        .w(px(DATE_W))
                        .text_size(px(9.5))
                        .font_weight(FontWeight(600))
                        .text_color(move || design::text3()),
                ),
        )
        .child(
            view().w_full().flex_1().min_h_0().child(
                scroll().w_full().h_full().child(
                    view()
                        .w_full()
                        .flex_col()
                        .child(dynamic(move || -> AnyElement {
                            if visible.get().is_empty() {
                                return view()
                                    .w_full()
                                    .h(px(72.0))
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        text(move || {
                                            let key = if total == 0 {
                                                "history.empty"
                                            } else {
                                                "history.noMatch"
                                            };
                                            i18n::t(language_value.get().as_str(), key)
                                        })
                                        .text_size(px(11.0))
                                        .text_color(move || design::text3()),
                                    )
                                    .into_element();
                            }
                            let select_for_rows = select.clone();
                            return view()
                                .w_full()
                                .flex_col()
                                .child(each(
                                    visible,
                                    |row: &(usize, domain::Commit)| row.0,
                                    move |row, _| {
                                        let (index, commit) = row.get();
                                        let select = select_for_rows.clone();
                                        let is_selected = selected == Some(index);
                                        let is_merge = commit.parents.len() > 1;
                                        let has_refs = !commit.refs.is_empty();
                                        view()
                                            .w_full()
                                            .h(px(metrics::COMMIT_ROW_H))
                                            .flex_row()
                                            .items_center()
                                            .gap(px(8.0))
                                            .px(px(8.0))
                                            .border_b_1()
                                            .border_color(move || design::line_soft())
                                            .bg(move || {
                                                if is_selected {
                                                    design::selected()
                                                } else {
                                                    AdaptiveColor::TRANSPARENT
                                                }
                                            })
                                            .hover(|style| style.bg(move || design::hover()))
                                            .on_press(move || select(index))
                                            .child(
                                                view()
                                                    .w(px(GRAPH_W - 8.0))
                                                    .h_full()
                                                    .flex_row()
                                                    .items_center()
                                                    .gap(px(4.0))
                                                    .child(
                                                        view()
                                                            .w(px(1.0))
                                                            .h_full()
                                                            .bg(move || design::line_strong()),
                                                    )
                                                    .child(
                                                        view()
                                                            .w(px(7.0))
                                                            .h(px(7.0))
                                                            .rounded(px(4.0))
                                                            .bg(move || {
                                                                if is_selected {
                                                                    design::accent()
                                                                } else {
                                                                    design::line_strong()
                                                                }
                                                            }),
                                                    )
                                                    .child(
                                                        view()
                                                            .w(px(10.0))
                                                            .h(px(1.0))
                                                            .bg(move || {
                                                                if is_merge {
                                                                    design::line_strong()
                                                                } else {
                                                                    AdaptiveColor::TRANSPARENT
                                                                }
                                                            }),
                                                    ),
                                            )
                                            .child(
                                                view()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .flex_row()
                                                    .items_center()
                                                    .gap(px(6.0))
                                                    .child(
                                                        text(move || commit.summary.clone())
                                                            .text_size(px(12.5))
                                                            .text_color(move || design::text())
                                                            .text_overflow(nsh_core::EllipsizeMode::Tail),
                                                    )
                                                    .child(
                                                        view()
                                                            .h(px(16.0))
                                                            .flex_row()
                                                            .items_center()
                                                            .gap(px(4.0))
                                                            .when(has_refs, |this| {
                                                                this.child(ref_chips(
                                                                    commit.refs.clone(),
                                                                ))
                                                            }),
                                                    ),
                                            )
                                            .child(
                                                text(move || commit.author.clone())
                                                    .w(px(AUTHOR_W))
                                                    .text_size(px(10.5))
                                                    .text_color(move || design::text2())
                                                    .text_overflow(nsh_core::EllipsizeMode::Tail),
                                            )
                                            .child(
                                                text(move || {
                                                    let timestamp = commit.date.parse().unwrap_or(0);
                                                    i18n::relative_time(
                                                        language_value.get().as_str(),
                                                        timestamp,
                                                        now,
                                                    )
                                                })
                                                .w(px(DATE_W))
                                                .text_size(px(9.5))
                                                .text_color(move || design::text3()),
                                            )
                                    },
                                ))
                                .into_element();
                        })),
                ),
            ),
        );
}

/// 引用徽章：本地分支、远程、标签各用一组配色。
fn ref_chips(refs: Vec<String>) -> impl IntoElement {
    let mut chips: Vec<AnyElement> = Vec::new();
    for reference in refs.into_iter().take(2) {
        let is_tag = reference.starts_with("tag:");
        let is_remote = reference.contains('/') && !is_tag;
        let label = reference
            .trim_start_matches("HEAD -> ")
            .trim_start_matches("tag: ")
            .to_string();
        chips.push(
            view()
                .h(px(16.0))
                .px(px(4.0))
                .items_center()
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
                        .text_size(px(9.5))
                        .text_color(move || {
                            if is_tag {
                                design::tag_fg()
                            } else if is_remote {
                                design::remote_fg()
                            } else {
                                design::branch_fg()
                            }
                        }),
                )
                .into_element(),
        );
    }
    return view().flex_row().items_center().gap(px(4.0)).child(chips);
}

fn tab_branch(tab: &Tab) -> String {
    if tab.branch.is_empty() {
        return "HEAD".to_string();
    }
    return tab.branch.clone();
}
