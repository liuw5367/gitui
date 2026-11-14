//! 历史表：`.hist` 256 高，含工具条（32）、筛选浮层、列标题（23）与 28 高的提交行。
//!
//! 行内是设计稿的四列：图线 64、说明（含引用徽章）、作者 96、时间 58。工具条上的
//! 芯片显示当前筛选条件；点筛选图标打开浮层（设计板 05）。对比模式下点击提交是
//! 加入/移出对比集合（设计板 06）。

use crate::design::{self, metrics};
use crate::filters::When;
use crate::i18n;
use crate::model::Tab;
use crate::widgets::kbd;
use crate::Actions;
use nsh_core::prelude::*;

const AUTHOR_W: f32 = 96.0;
const DATE_W: f32 = 58.0;
const GRAPH_W: f32 = 64.0;

/// 浮层宽度（设计板 `.filter-pop`）。
const POP_W: f32 = 268.0;

#[allow(clippy::too_many_arguments)]
pub(crate) fn history(
    language: Signal<SharedString>,
    tab: Tab,
    query: Signal<String>,
    filter_reference: Signal<String>,
    filter_author: Signal<String>,
    filter_when: Signal<SharedString>,
    filter_open: Signal<bool>,
    compare_mode: Signal<bool>,
    actions: Actions,
) -> impl IntoElement {
    let commits = tab.commits.clone();
    let selected = tab.selected;
    let compare = tab.compare.clone();
    let select = actions.select_commit.clone();
    let toggle_compare = actions.toggle_compare.clone();
    let total = commits.len();
    let now = store::now_unix();
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
    let branches: Vec<String> = tab
        .refs
        .branches
        .iter()
        .map(|branch| branch.name.clone())
        .collect();
    let authors = tab.authors.clone();
    let branch_value = tab.branch.clone();
    let language_value = language;

    return view()
        .w_full()
        .h(px(metrics::HISTORY_H))
        .flex_shrink_0()
        .flex_col()
        .bg(move || design::content())
        .border_b_1()
        .border_color(move || design::line())
        .child(hist_bar(
            language,
            branch_value,
            total,
            filter_reference,
            filter_author,
            filter_when,
            filter_open,
            compare_mode,
            actions.clone(),
        ))
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
                .child(column_header(language, "history.description").flex_1())
                .child(column_header(language, "history.author").w(px(AUTHOR_W)))
                .child(column_header(language, "history.date").w(px(DATE_W))),
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
                            let select = select.clone();
                            let toggle_compare = toggle_compare.clone();
                            let compare_for_rows = compare.clone();
                            return view()
                                .w_full()
                                .flex_col()
                                .child(each(
                                    visible,
                                    |row: &(usize, domain::Commit)| row.0,
                                    move |row, _| {
                                        let (index, commit) = row.get();
                                        let select = select.clone();
                                        let toggle_compare = toggle_compare.clone();
                                        let compare_for_rows = compare_for_rows.clone();
                                        let is_selected = selected == Some(index);
                                        let in_compare = compare_for_rows.contains(&index);
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
                                                if in_compare {
                                                    design::accent_soft()
                                                } else if is_selected {
                                                    design::selected()
                                                } else {
                                                    AdaptiveColor::TRANSPARENT
                                                }
                                            })
                                            .hover(|style| style.bg(move || design::hover()))
                                            .on_press(move |event: &PressEvent| {
                                                // 对比模式下点击是加入/移出集合；
                                                // 否则普通点击选中，⌘ 点击也加入对比。
                                                if compare_mode.get_untracked()
                                                    || event.modifiers.cmd
                                                {
                                                    toggle_compare(index);
                                                } else {
                                                    select(index);
                                                }
                                            })
                                            .child(graph_cell(is_selected || in_compare, is_merge))
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
                                                                this.child(ref_chips(commit.refs.clone()))
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
                                                    let timestamp =
                                                        commit.date.parse().unwrap_or(0);
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
        )
        .child(
            view()
                .w_full()
                .h(px(0.0))
                .child(dynamic(move || -> AnyElement {
                    if !filter_open.get() {
                        return view().w_full().h(px(0.0)).into_element();
                    }
                    return filter_pop(
                        language_value,
                        branches.clone(),
                        authors.clone(),
                        filter_reference,
                        filter_author,
                        filter_when,
                        actions.clone(),
                    )
                    .into_element();
                })),
        );
}

#[allow(clippy::too_many_arguments)]
fn hist_bar(
    language: Signal<SharedString>,
    branch: String,
    total: usize,
    filter_reference: Signal<String>,
    filter_author: Signal<String>,
    filter_when: Signal<SharedString>,
    filter_open: Signal<bool>,
    compare_mode: Signal<bool>,
    actions: Actions,
) -> impl IntoElement {
    let clear = actions.clear_filter.clone();
    let set_compare_mode = actions.set_compare_mode.clone();
    return view()
        .w_full()
        .h(px(metrics::HIST_BAR_H))
        .flex_row()
        .items_center()
        .gap(px(7.0))
        .px(px(10.0))
        .border_b_1()
        .border_color(move || design::line_soft())
        .child(
            text(move || i18n::t(language.get().as_str(), "shell.history"))
                .text_size(px(12.5))
                .font_weight(FontWeight(600))
                .text_color(move || design::text()),
        )
        .child(chip(
            move || true,
            {
                let branch = branch.clone();
                move || {
                    let reference = filter_reference.get();
                    if reference.is_empty() {
                        branch.clone()
                    } else {
                        reference
                    }
                }
            },
            design::branch_bg,
            design::branch_fg,
        ))
        .child(
            text(move || {
                i18n::with_arguments(
                    language.get().as_str(),
                    "history.commits",
                    &[("count", total.to_string())],
                )
            })
            .text_size(px(10.5))
            .text_color(move || design::text3()),
        )
        .child(chip(
            move || When::from_key(filter_when.get().as_str()) != When::All,
            move || {
                i18n::t(
                    language.get().as_str(),
                    When::from_key(filter_when.get().as_str()).key(),
                )
            },
            design::panel2,
            design::text2,
        ))
        .child(chip(
            move || !filter_author.get().is_empty(),
            move || filter_author.get(),
            design::panel2,
            design::text2,
        ))
        .child(view().flex_1())
        .child(
            view()
                .h(px(20.0))
                .px(px(6.0))
                .items_center()
                .rounded(px(4.0))
                .bg(move || {
                    if compare_mode.get() {
                        design::accent_soft()
                    } else {
                        AdaptiveColor::TRANSPARENT
                    }
                })
                .on_press({
                    let set_compare_mode = set_compare_mode.clone();
                    move || set_compare_mode(!compare_mode.get_untracked())
                })
                .child(
                    text(move || i18n::t(language.get().as_str(), "compare.mode"))
                        .text_size(px(10.5))
                        .text_color(move || {
                            if compare_mode.get() {
                                design::accent()
                            } else {
                                design::text3()
                            }
                        }),
                ),
        )
        .child(
            view()
                .h(px(20.0))
                .px(px(6.0))
                .items_center()
                .rounded(px(4.0))
                .hover(|style| style.bg(move || design::hover()))
                .on_press(move || clear())
                .child(
                    text(move || i18n::t(language.get().as_str(), "filter.clear"))
                        .text_size(px(10.5))
                        .text_color(move || design::text3()),
                ),
        )
        .child(widgets_icon(filter_open));
}

fn widgets_icon(filter_open: Signal<bool>) -> impl IntoElement {
    return crate::widgets::icon_button("filter", 12.0, design::text2, move || {
        filter_open.set(!filter_open.get_untracked())
    });
}

/// 工具条上的筛选芯片：`visible` 为假时不占位。
fn chip(
    visible: impl Fn() -> bool + 'static,
    label: impl Fn() -> String + 'static,
    background: fn() -> AdaptiveColor,
    foreground: fn() -> AdaptiveColor,
) -> impl IntoElement {
    return view()
        .h(move || if visible() { px(18.0) } else { px(0.0) })
        .px(px(6.0))
        .items_center()
        .rounded(px(4.0))
        .bg(move || background())
        .child(
            text(move || label())
                .text_size(px(9.5))
                .text_color(move || foreground()),
        );
}

fn column_header(language: Signal<SharedString>, key: &'static str) -> nsh_core::element::Text {
    return text(move || i18n::t(language.get().as_str(), key))
        .text_size(px(9.5))
        .font_weight(FontWeight(600))
        .text_color(move || design::text3());
}

fn graph_cell(highlight: bool, is_merge: bool) -> impl IntoElement {
    return view()
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
                    if highlight {
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
        );
}

/// 筛选浮层（设计板 05）：分支、作者、时间三组，底部应用与清除。
fn filter_pop(
    language: Signal<SharedString>,
    branches: Vec<String>,
    authors: Vec<String>,
    filter_reference: Signal<String>,
    filter_author: Signal<String>,
    filter_when: Signal<SharedString>,
    actions: Actions,
) -> impl IntoElement {
    let apply = actions.apply_filter.clone();
    let clear = actions.clear_filter.clone();
    let mut branch_rows: Vec<AnyElement> = Vec::new();
    branch_rows.push(choice_row(
        language,
        "filter.allBranches".to_string(),
        move || filter_reference.get().is_empty(),
        {
            let filter_reference = filter_reference;
            move || filter_reference.set(String::new())
        },
    ));
    for branch in branches.iter().take(8) {
        let name = branch.clone();
        let value = branch.clone();
        branch_rows.push(choice_row(
            language,
            name,
            move || filter_reference.get() == value,
            {
                let filter_reference = filter_reference;
                let value = branch.clone();
                move || filter_reference.set(value.clone())
            },
        ));
    }

    let mut author_rows: Vec<AnyElement> = Vec::new();
    author_rows.push(choice_row(
        language,
        i18n::t(language.get_untracked().as_str(), "filter.allAuthors"),
        move || filter_author.get().is_empty(),
        {
            let filter_author = filter_author;
            move || filter_author.set(String::new())
        },
    ));
    for author in authors.iter().take(8) {
        let label = author.clone();
        let value = author.clone();
        author_rows.push(choice_row(
            language,
            label,
            move || filter_author.get() == value,
            {
                let filter_author = filter_author;
                let value = author.clone();
                move || filter_author.set(value.clone())
            },
        ));
    }

    let when_rows: Vec<AnyElement> = When::ALL
        .iter()
        .map(|when| {
            let when = *when;
            choice_row(
                language,
                i18n::t(language.get_untracked().as_str(), when.key()),
                move || When::from_key(filter_when.get().as_str()) == when,
                {
                    let filter_when = filter_when;
                    move || filter_when.set(SharedString::from(when.id()))
                },
            )
        })
        .collect();

    return view()
        .w(px(POP_W))
        .p(px(6.0))
        .flex_col()
        .gap(px(2.0))
        .rounded(px(metrics::RADIUS_PANEL))
        .border_1()
        .border_color(move || design::line())
        .bg(move || design::token("popover"))
        .child(section_label(language, "filter.branch"))
        .child(branch_rows)
        .child(section_label(language, "filter.author"))
        .child(author_rows)
        .child(section_label(language, "filter.when"))
        .child(when_rows)
        .child(
            view()
                .w_full()
                .mt(px(4.0))
                .pt(px(6.0))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .border_t_1()
                .border_color(move || design::line())
                .child(crate::widgets::primary_button(
                    move || i18n::t(language.get().as_str(), "filter.apply"),
                    {
                        let apply = apply.clone();
                        move || apply()
                    },
                ))
                .child(crate::widgets::small_button(
                    move || i18n::t(language.get().as_str(), "filter.clear"),
                    {
                        let clear = clear.clone();
                        move || clear()
                    },
                ))
                .child(view().flex_1())
                .child(kbd("esc")),
        );
}

fn section_label(language: Signal<SharedString>, key: &'static str) -> impl IntoElement {
    return view().w_full().h(px(20.0)).items_center().child(
        text(move || i18n::t(language.get().as_str(), key))
            .text_size(px(9.5))
            .font_weight(FontWeight(600))
            .text_color(move || design::text3()),
    );
}

fn choice_row(
    language: Signal<SharedString>,
    label: String,
    selected: impl Fn() -> bool + 'static,
    on_press: impl Fn() + 'static,
) -> AnyElement {
    let _ = language;
    return view()
        .w_full()
        .h(px(22.0))
        .px(px(6.0))
        .flex_row()
        .items_center()
        .rounded(px(4.0))
        .bg(move || {
            if selected() {
                design::selected()
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .hover(|style| style.bg(move || design::hover()))
        .on_press(on_press)
        .child(
            text(label)
                .text_size(px(11.0))
                .text_color(move || design::text())
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        )
        .into_element();
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
