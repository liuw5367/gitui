//! 变更视图（设计板 10 / 11 的左侧主区）：变更列表 + 文件 diff。
//!
//! 列表按「暂存 / 未暂存」两段呈现，段内按目录分组；行高 24、状态字母与 +/− 统计
//! 来自设计稿。changelists 通过列表上方的选择器切换（本地存储，按仓库分组）。

use crate::design::{self, metrics};
use crate::i18n;
use crate::model::Tab;
use crate::widgets;
use crate::Actions;
use domain::diff::{DiffLineKind, Patch};
use nsh_core::prelude::*;

/// 变更列表高度（设计板 `.changeslist`）。
const LIST_H: f32 = 342.0;

#[allow(clippy::too_many_arguments)]
pub(crate) fn changes_view(
    language: Signal<SharedString>,
    tab: Tab,
    changelist_open: Signal<bool>,
    actions: Actions,
) -> impl IntoElement {
    let total = tab.changes.len();
    let staged = tab.changes.iter().filter(|entry| entry.staged).count();
    let patch = Patch::parse(tab.change_diff.as_str());
    let label = tab
        .selected_change
        .and_then(|index| tab.changes.get(index))
        .map(|entry| entry.path.clone())
        .unwrap_or_default();
    let additions: u32 = patch.files.iter().map(|file| file.additions).sum();
    let deletions: u32 = patch.files.iter().map(|file| file.deletions).sum();
    let rows: Vec<AnyElement> = patch.lines.iter().map(diff_row).collect();
    return view()
        .w_full()
        .h_full()
        .flex_col()
        .child(changes_list(
            language,
            tab,
            staged,
            total,
            changelist_open,
            actions,
        ))
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .flex_col()
                .border_t_1()
                .border_color(move || design::line())
                .child(
                    view()
                        .w_full()
                        .h(px(metrics::DIFFBAR_H))
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .px(px(11.0))
                        .border_b_1()
                        .border_color(move || design::line())
                        .child(widgets::icon("file", 12.0, design::text3))
                        .child(
                            text(label)
                                .text_size(px(11.0))
                                .font_family("SF Mono")
                                .text_color(move || design::text())
                                .text_overflow(nsh_core::EllipsizeMode::Tail),
                        )
                        .child(
                            text(format!("+{} −{}", additions, deletions))
                                .text_size(px(9.5))
                                .font_family("SF Mono")
                                .text_color(move || design::text3()),
                        ),
                )
                .child(
                    view().w_full().flex_1().min_h_0().child(
                        scroll().w_full().h_full().child(
                            view().w_full().flex_col().child(if rows.is_empty() {
                                view()
                                    .w_full()
                                    .h(px(48.0))
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        text(move || {
                                            i18n::t(
                                                language.get().as_str(),
                                                "changes.selectFile",
                                            )
                                        })
                                        .text_size(px(11.0))
                                        .text_color(move || design::text3()),
                                    )
                                    .into_element()
                            } else {
                                view().w_full().flex_col().child(rows).into_element()
                            }),
                        ),
                    ),
                ),
        );
}

fn changes_list(
    language: Signal<SharedString>,
    tab: Tab,
    staged_count: usize,
    total: usize,
    changelist_open: Signal<bool>,
    actions: Actions,
) -> impl IntoElement {
    let select_change = actions.select_change.clone();
    let toggle_stage = actions.toggle_stage.clone();
    let discard_change = actions.discard_change.clone();
    let stage_all = actions.stage_all.clone();
    let unstage_all = actions.unstage_all.clone();
    let selected = tab.selected_change;

    let mut sections: Vec<AnyElement> = Vec::new();
    for (is_staged, title_key, action_label_key, action) in [
        (true, "changes.staged", "changes.unstageAll", unstage_all),
        (false, "changes.unstaged", "changes.stageAll", stage_all),
    ] {
        let entries: Vec<_> = tab
            .changes
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.staged == is_staged)
            .collect();
        if entries.is_empty() {
            continue;
        }
        let mut rows: Vec<AnyElement> = Vec::new();
        let mut current_directory = String::new();
        for (index, entry) in entries {
            let directory = entry.directory().to_string();
            if directory != current_directory {
                current_directory = directory.clone();
                rows.push(
                    view()
                        .w_full()
                        .h(px(20.0))
                        .pl(px(8.0))
                        .flex_row()
                        .items_center()
                        .gap(px(5.0))
                        .child(widgets::icon("chev_down", 9.0, design::text3))
                        .child(
                            text(if directory.is_empty() {
                                ".".to_string()
                            } else {
                                directory
                            })
                            .text_size(px(10.5))
                            .text_color(move || design::text2()),
                        )
                        .into_element(),
                );
            }
            let select_change = select_change.clone();
            let toggle_stage = toggle_stage.clone();
            let discard_change = discard_change.clone();
            let path = entry.path.clone();
            let file_name = path.rsplit('/').next().unwrap_or(&path).to_string();
            let status = entry.status_letter();
            let is_untracked = entry.untracked;
            let additions = entry.additions;
            let deletions = entry.deletions;
            let is_selected = selected == Some(index);
            rows.push(
                view()
                    .w_full()
                    .h(px(24.0))
                    .pl(px(20.0))
                    .pr(px(8.0))
                    .flex_row()
                    .items_center()
                    .gap(px(6.0))
                    .bg(move || {
                        if is_selected {
                            design::selected()
                        } else {
                            AdaptiveColor::TRANSPARENT
                        }
                    })
                    .hover(|style| style.bg(move || design::hover()))
                    .on_press(move || select_change(index))
                    .child(
                        view()
                            .w(px(12.0))
                            .h(px(12.0))
                            .items_center()
                            .justify_center()
                            .rounded(px(3.0))
                            .border_1()
                            .border_color(move || design::line_strong())
                            .bg(move || {
                                if entry_is_staged(is_staged) {
                                    design::accent()
                                } else {
                                    AdaptiveColor::TRANSPARENT
                                }
                            })
                            .on_press(move || toggle_stage(index))
                            .child(
                                text(move || {
                                    if entry_is_staged(is_staged) {
                                        "✓".to_string()
                                    } else {
                                        String::new()
                                    }
                                })
                                .text_size(px(8.0))
                                .text_color(move || design::content()),
                            ),
                    )
                    .child(
                        text(file_name)
                            .flex_1()
                            .text_size(px(10.5))
                            .font_family("SF Mono")
                            .text_color(move || design::text())
                            .text_overflow(nsh_core::EllipsizeMode::Tail),
                    )
                    .child(
                        text(status.to_string())
                            .w(px(10.0))
                            .text_size(px(9.5))
                            .font_family("SF Mono")
                            .text_color(move || status_color(status)),
                    )
                    .child(
                        text(format!("+{}", additions))
                            .text_size(px(9.5))
                            .text_color(move || design::add_fg()),
                    )
                    .child(
                        text(format!("−{}", deletions))
                            .text_size(px(9.5))
                            .text_color(move || design::del_fg()),
                    )
                    .child(
                        // 丢弃只对未跟踪以外的未暂存文件有意义，且需要确认。
                        view()
                            .w(px(14.0))
                            .h(px(14.0))
                            .items_center()
                            .justify_center()
                            .rounded(px(3.0))
                            .when(is_selected && !is_staged && !is_untracked, |this| {
                                this.hover(|style| style.bg(move || design::hover()))
                                    .on_press(move || discard_change(index))
                                    .child(
                                        text("⌫")
                                            .text_size(px(9.0))
                                            .text_color(move || design::text3()),
                                    )
                            }),
                    )
                    .into_element(),
            );
        }
        sections.push(
            view()
                .w_full()
                .flex_col()
                .child(
                    view()
                        .w_full()
                        .h(px(24.0))
                        .px(px(8.0))
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            text(move || i18n::t(language.get().as_str(), title_key))
                                .text_size(px(10.5))
                                .font_weight(FontWeight(600))
                                .text_color(move || design::text2()),
                        )
                        .child(
                            text(entries_count(&tab, is_staged).to_string())
                                .text_size(px(9.5))
                                .text_color(move || design::text3()),
                        )
                        .child(view().flex_1())
                        .child(
                            view()
                                .h(px(16.0))
                                .px(px(4.0))
                                .items_center()
                                .rounded(px(3.0))
                                .hover(|style| style.bg(move || design::hover()))
                                .on_press({
                                    let action = action.clone();
                                    move || action()
                                })
                                .child(
                                    text(move || {
                                        i18n::t(language.get().as_str(), action_label_key)
                                    })
                                    .text_size(px(9.5))
                                    .text_color(move || design::accent()),
                                ),
                        ),
                )
                .child(rows)
                .into_element(),
        );
    }

    if !tab.conflicts.is_empty() {
        let resolve = actions.resolve_conflict.clone();
        let conflicts = tab.conflicts.clone();
        let mut rows: Vec<AnyElement> = Vec::new();
        for path in conflicts {
            let ours = resolve.clone();
            let theirs = resolve.clone();
            let ours_path = path.clone();
            let theirs_path = path.clone();
            rows.push(
                view()
                    .w_full()
                    .h(px(24.0))
                    .pl(px(20.0))
                    .pr(px(8.0))
                    .flex_row()
                    .items_center()
                    .gap(px(6.0))
                    .bg(move || design::del_bg())
                    .child(
                        text(path)
                            .flex_1()
                            .text_size(px(10.5))
                            .font_family("SF Mono")
                            .text_color(move || design::text())
                            .text_overflow(nsh_core::EllipsizeMode::Tail),
                    )
                    .child(widgets::small_button(
                        move || i18n::t(language.get().as_str(), "conflicts.ours"),
                        move || ours(ours_path.clone(), true),
                    ))
                    .child(widgets::small_button(
                        move || i18n::t(language.get().as_str(), "conflicts.theirs"),
                        move || theirs(theirs_path.clone(), false),
                    ))
                    .into_element(),
            );
        }
        sections.insert(
            0,
            view()
                .w_full()
                .flex_col()
                .child(
                    view()
                        .w_full()
                        .h(px(24.0))
                        .px(px(8.0))
                        .items_center()
                        .child(
                            text(move || {
                                i18n::t(language.get().as_str(), "conflicts.title")
                            })
                            .text_size(px(10.5))
                            .font_weight(FontWeight(600))
                            .text_color(move || design::danger()),
                        ),
                )
                .child(rows)
                .into_element(),
        );
    }

    return view()
        .w_full()
        .h(px(LIST_H))
        .flex_shrink_0()
        .flex_col()
        .bg(move || design::content())
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
                    text(move || i18n::t(language.get().as_str(), "shell.changes"))
                        .text_size(px(12.5))
                        .font_weight(FontWeight(600))
                        .text_color(move || design::text()),
                )
                .child(
                    view()
                        .h(px(18.0))
                        .px(px(6.0))
                        .flex_row()
                        .items_center()
                        .gap(px(4.0))
                        .rounded(px(4.0))
                        .border_1()
                        .border_color(move || design::line_soft())
                        .on_press(move || {
                            changelist_open.set(!changelist_open.get_untracked())
                        })
                        .child(
                            view()
                                .w(px(6.0))
                                .h(px(6.0))
                                .rounded(px(3.0))
                                .bg(move || design::accent()),
                        )
                        .child(
                            text(move || {
                                i18n::t(language.get().as_str(), "changelists.default")
                            })
                            .text_size(px(9.5))
                            .text_color(move || design::text2()),
                        )
                        .child(
                            text(total.to_string())
                                .text_size(px(9.5))
                                .text_color(move || design::text3()),
                        ),
                )
                .child(view().flex_1())
                .child(
                    text(move || {
                        i18n::with_arguments(
                            language.get().as_str(),
                            "changes.summary",
                            &[
                                ("staged", staged_count.to_string()),
                                ("unstaged", (total - staged_count).to_string()),
                            ],
                        )
                    })
                    .text_size(px(10.0))
                    .text_color(move || design::text3()),
                ),
        )
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .child(scroll().w_full().h_full().child(
                    view().w_full().flex_col().child(sections),
                )),
        );
}

fn entries_count(tab: &Tab, staged: bool) -> usize {
    return tab
        .changes
        .iter()
        .filter(|entry| entry.staged == staged)
        .count();
}

fn entry_is_staged(staged: bool) -> bool {
    return staged;
}

fn status_color(status: char) -> AdaptiveColor {
    return match status {
        'A' => design::accent(),
        'D' => design::del_fg(),
        '?' => design::text3(),
        _ => design::warn(),
    };
}

fn diff_row(line: &domain::diff::DiffLine) -> AnyElement {
    let (background, foreground) = match line.kind {
        DiffLineKind::Add => (design::add_bg(), design::add_fg()),
        DiffLineKind::Remove => (design::del_bg(), design::del_fg()),
        DiffLineKind::Hunk => (design::panel2(), design::text2()),
        DiffLineKind::Meta => (design::content(), design::text3()),
        DiffLineKind::Context => (design::content(), design::text()),
    };
    let old = line.old.map(|value| value.to_string()).unwrap_or_default();
    let new = line.new.map(|value| value.to_string()).unwrap_or_default();
    let content = line.text.clone();
    return view()
        .w_full()
        .h(px(metrics::DIFF_LINE_H))
        .flex_row()
        .items_center()
        .bg(move || background)
        .child(
            text(old)
                .w(px(34.0))
                .flex_shrink_0()
                .pl(px(6.0))
                .text_size(px(metrics::DIFF_FONT))
                .line_height(px(metrics::DIFF_LINE_H))
                .font_family("SF Mono")
                .text_color(move || design::text3()),
        )
        .child(
            text(new)
                .w(px(34.0))
                .flex_shrink_0()
                .pl(px(6.0))
                .text_size(px(metrics::DIFF_FONT))
                .line_height(px(metrics::DIFF_LINE_H))
                .font_family("SF Mono")
                .text_color(move || design::text3()),
        )
        .child(
            text(content)
                .flex_1()
                .text_size(px(metrics::DIFF_FONT))
                .line_height(px(metrics::DIFF_LINE_H))
                .font_family("SF Mono")
                .text_color(move || foreground)
                .text_overflow(nsh_core::EllipsizeMode::Clip),
        )
        .into_element();
}
