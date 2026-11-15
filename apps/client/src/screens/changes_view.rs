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
use nsh_core::element::desktop::MenuItem;
use nsh_core::prelude::*;

/// 变更列表高度（设计板 `.changeslist`）。
const LIST_H: f32 = 342.0;

#[allow(clippy::too_many_arguments)]
pub(crate) fn changes_view(
    language: Signal<SharedString>,
    tab: Tab,
    changelist_open: Signal<bool>,
    group_by_directory: bool,
    tree: crate::model::TreeOptions,
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
    // 逐行走一遍，记住当前在第几个 hunk、hunk 内第几行，供行级操作定位。
    let staged_selection = tab
        .selected_change
        .and_then(|index| tab.changes.get(index))
        .map(|entry| entry.staged)
        .unwrap_or(false);
    let piece_actions = actions.clone();
    let mut rows: Vec<AnyElement> = Vec::new();
    let mut hunk_index = 0usize;
    let mut line_index = 0usize;
    for line in patch.lines.iter() {
        match line.kind {
            DiffLineKind::Hunk => {
                if hunk_index > 0 || !rows.is_empty() {
                    // 第一个 hunk 之前可能已有 Meta 行，不算 hunk。
                }
                let apply = piece_actions.apply_piece.clone();
                let language = language;
                rows.push(hunk_header_row(
                    language,
                    line.text.clone(),
                    hunk_index,
                    staged_selection,
                    apply,
                ));
                hunk_index += 1;
                line_index = 0;
            }
            DiffLineKind::Add | DiffLineKind::Remove => {
                let apply = piece_actions.apply_piece.clone();
                let language = language;
                rows.push(line_row_with_piece(
                    line,
                    hunk_index.saturating_sub(1),
                    line_index,
                    staged_selection,
                    language,
                    apply,
                ));
                line_index += 1;
            }
            _ => {
                rows.push(diff_row(line));
            }
        }
    }
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
            group_by_directory,
            tree,
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
                                .text_size(px(crate::ui::CODE))
                                .text_color(move || design::text())
                                .text_overflow(nsh_core::EllipsizeMode::Tail),
                        )
                        .child(
                            text(format!("+{} −{}", additions, deletions))
                                .text_size(px(crate::ui::CODE))
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
                                        .text_size(px(crate::ui::BODY))
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
    group_by_directory: bool,
    tree: crate::model::TreeOptions,
    actions: Actions,
) -> impl IntoElement {
    let select_change = actions.select_change.clone();
    let toggle_stage = actions.toggle_stage.clone();
    let discard_change = actions.discard_change.clone();
    let tab_for_pop = tab.clone();
    let actions_pop = actions.clone();
    let selected_path = tab
        .selected_change
        .and_then(|index| tab.changes.get(index))
        .map(|entry| entry.path.clone());
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
        // 目录链压缩需要知道所有出现过的目录。
        let all_dirs: Vec<String> = tab
            .changes
            .iter()
            .map(|entry| entry.directory().to_string())
            .filter(|directory| !directory.is_empty())
            .collect();
        for (index, entry) in entries {
            let raw_directory = entry.directory().to_string();
            let directory = tree.compact_label(&all_dirs, raw_directory.as_str());
            // 该目录自己有没有直接文件；没有且开了「合并空目录」就不画这一行。
            let direct_files = tab
                .changes
                .iter()
                .filter(|candidate| candidate.directory() == raw_directory)
                .count();
            let show_row = tree.shows_directory(direct_files);
            if group_by_directory && directory != current_directory && show_row {
                current_directory = directory.clone();
                rows.push(
                    view()
                        .w_full()
                        .h(px(20.0))
                        .pl(px(8.0))
                        .flex_row()
                        .items_center()
                        .gap(px(5.0))
                        .child(widgets::icon("chev-down", 9.0, design::text3))
                        .child(
                            text(if directory.is_empty() {
                                ".".to_string()
                            } else {
                                directory
                            })
                            .text_size(px(crate::ui::SECONDARY))
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
                    // 右键：暂存、撤销修改、外部打开、复制路径、移入清单。
                    .on_context_menu({
                        let toggle_stage = toggle_stage.clone();
                        let discard_change = discard_change.clone();
                        let copy_text = actions.copy_text.clone();
                        let open_editor = actions.open_editor.clone();
                        let reveal_path = actions.reveal_path.clone();
                        let move_to_changelist = actions.move_to_changelist.clone();
                        let repo_root = tab.path.clone();
                        let changelists = tab.changelists.clone();
                        let path_for_menu = entry.path.clone();
                        move |event: &nsh_core::ContextMenuEvent| {
                            let items = file_menu(
                                language,
                                repo_root.as_str(),
                                path_for_menu.as_str(),
                                changelists.clone(),
                                is_staged,
                                is_untracked,
                                index,
                                toggle_stage.clone(),
                                discard_change.clone(),
                                copy_text.clone(),
                                open_editor.clone(),
                                reveal_path.clone(),
                                move_to_changelist.clone(),
                            );
                            let _ = nsh_core::element::desktop::popup_menu_at(
                                items,
                                event.page_position,
                            );
                        }
                    })
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
                                .text_size(px(crate::ui::SECONDARY))
                                .text_color(move || design::content()),
                            ),
                    )
                    .child(
                        text(file_name)
                            .flex_1()
                            .text_size(px(crate::ui::CODE))
                            .text_color(move || design::text())
                            .text_overflow(nsh_core::EllipsizeMode::Tail),
                    )
                    .child(
                        text(status.to_string())
                            .w(px(10.0))
                            .text_size(px(crate::ui::CODE))
                            .text_color(move || status_color(status)),
                    )
                    .child(
                        text(format!("+{}", additions))
                            .text_size(px(crate::ui::SECONDARY))
                            .text_color(move || design::add_fg()),
                    )
                    .child(
                        text(format!("−{}", deletions))
                            .text_size(px(crate::ui::SECONDARY))
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
                                            .text_size(px(crate::ui::SECONDARY))
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
                                .text_size(px(crate::ui::SECONDARY))
                                .font_weight(FontWeight(600))
                                .text_color(move || design::text2()),
                        )
                        .child(
                            text(entries_count(&tab, is_staged).to_string())
                                .text_size(px(crate::ui::SECONDARY))
                                .text_color(move || design::text3()),
                        )
                        .child(view().flex_1())
                        .child(crate::ui::button::button(
                            move || i18n::t(language.get().as_str(), action_label_key),
                            crate::ui::button::ButtonVariant::Ghost,
                            crate::ui::button::ButtonSize::Small,
                            false,
                            {
                                let action = action.clone();
                                move || action()
                            },
                        )),
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
                            .text_size(px(crate::ui::CODE))
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
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            text(move || {
                                i18n::t(language.get().as_str(), "conflicts.title")
                            })
                            .text_size(px(crate::ui::SECONDARY))
                            .font_weight(FontWeight(600))
                            .text_color(move || design::danger()),
                        )
                        .child(view().flex_1())
                        .child(widgets::small_button(
                            move || i18n::t(language.get().as_str(), "conflicts.resolve"),
                            {
                                let open_conflict = actions.open_conflict.clone();
                                move || open_conflict()
                            },
                        )),
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
                        .text_size(px(crate::ui::BODY))
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
                                tab.active_changelist
                                    .clone()
                                    .unwrap_or_else(|| {
                                        i18n::t(
                                            language.get().as_str(),
                                            "changelists.default",
                                        )
                                    })
                            })
                            .text_size(px(crate::ui::SECONDARY))
                            .text_color(move || design::text2()),
                        )
                        .child(
                            text(total.to_string())
                                .text_size(px(crate::ui::SECONDARY))
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
                    .text_size(px(crate::ui::SECONDARY))
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
        )
        .child(
            view()
                .absolute()
                .top(px(metrics::HIST_BAR_H))
                .left(px(10.0))
                .child(dynamic(move || -> AnyElement {
                    if !changelist_open.get() {
                        return view().w(px(0.0)).h(px(0.0)).into_element();
                    }
                    return changelist_pop(
                        language,
                        tab_for_pop.clone(),
                        changelist_open,
                        actions_pop.clone(),
                        selected_path.clone(),
                    )
                    .into_element();
                })),
        );
}

/// 文件行的右键菜单（设计板 10 的 `file-menu`）。
///
/// 顺序：暂存/取消暂存 → 外部打开与复制 → 移入清单 → 撤销修改（破坏性，靠后且
/// 需确认）。未跟踪文件没有「撤销修改」：那等于删文件，不属于撤销。
#[allow(clippy::too_many_arguments)]
fn file_menu(
    language: Signal<SharedString>,
    repo_root: &str,
    path: &str,
    changelists: Vec<store::Changelist>,
    is_staged: bool,
    is_untracked: bool,
    index: usize,
    toggle_stage: std::sync::Arc<dyn Fn(usize)>,
    discard_change: std::sync::Arc<dyn Fn(usize)>,
    copy_text: std::sync::Arc<dyn Fn(String)>,
    open_editor: std::sync::Arc<dyn Fn(String)>,
    reveal_path: std::sync::Arc<dyn Fn(String)>,
    move_to_changelist: std::sync::Arc<dyn Fn(String, Option<String>)>,
) -> Vec<MenuItem> {
    let language_now = language.get_untracked();
    let lang = language_now.as_str();
    let absolute = format!("{}/{}", repo_root.trim_end_matches('/'), path);
    let mut items = vec![
        MenuItem::new(i18n::t(
            lang,
            if is_staged {
                "changes.unstage"
            } else {
                "changes.stage"
            },
        ))
        .on_select({
            let toggle_stage = toggle_stage.clone();
            move || toggle_stage(index)
        }),
        MenuItem::separator(),
        MenuItem::new(i18n::t(lang, "files.openInEditor")).on_select({
            let open_editor = open_editor.clone();
            let absolute = absolute.clone();
            move || open_editor(absolute.clone())
        }),
        MenuItem::new(i18n::t(lang, "files.revealInFinder")).on_select({
            let reveal_path = reveal_path.clone();
            let absolute = absolute.clone();
            move || reveal_path(absolute.clone())
        }),
        MenuItem::new(i18n::t(lang, "files.copyPath")).on_select({
            let copy_text = copy_text.clone();
            let absolute = absolute.clone();
            move || copy_text(absolute.clone())
        }),
    ];

    // 移入清单：不改暂存状态，所以不受 staged / untracked 限制。
    let mut targets: Vec<MenuItem> = changelists
        .iter()
        .map(|list| {
            let move_to_changelist = move_to_changelist.clone();
            let name = list.name.clone();
            let path = path.to_string();
            let target = name.clone();
            MenuItem::new(name).on_select(move || {
                move_to_changelist(path.clone(), Some(target.clone()));
            })
        })
        .collect();
    if !targets.is_empty() {
        targets.push(MenuItem::separator());
    }
    targets.push(MenuItem::new(i18n::t(lang, "changelists.default")).on_select({
        let move_to_changelist = move_to_changelist.clone();
        let path = path.to_string();
        move || move_to_changelist(path.clone(), None)
    }));
    items.push(MenuItem::separator());
    items.push(MenuItem::submenu(
        i18n::t(lang, "changelists.moveTo"),
        targets,
    ));

    items.push(MenuItem::separator());
    items.push(
        MenuItem::new(i18n::t(lang, "changes.revert"))
            .disabled(is_staged || is_untracked)
            .on_select({
                let discard_change = discard_change.clone();
                move || discard_change(index)
            }),
    );
    return items;
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
                .text_size(px(crate::ui::CODE))
                .line_height(px(crate::ui::CODE_LINE))
                .text_color(move || design::text3()),
        )
        .child(
            text(new)
                .w(px(34.0))
                .flex_shrink_0()
                .pl(px(6.0))
                .text_size(px(crate::ui::CODE))
                .line_height(px(crate::ui::CODE_LINE))
                .text_color(move || design::text3()),
        )
        .child(
            text(content)
                .flex_1()
                .text_size(px(crate::ui::CODE))
                .line_height(px(crate::ui::CODE_LINE))
                .text_color(move || foreground)
                .text_overflow(nsh_core::EllipsizeMode::Clip),
        )
        .into_element();
}

/// hunk 头：右侧一个「暂存/取消暂存此段」按钮。
fn hunk_header_row(
    language: Signal<SharedString>,
    header: String,
    hunk_index: usize,
    staged: bool,
    apply: std::sync::Arc<dyn Fn(usize, Option<usize>, bool)>,
) -> AnyElement {
    let label_key = if staged {
        "changes.unstageHunk"
    } else {
        "changes.stageHunk"
    };
    return view()
        .w_full()
        .h(px(metrics::DIFF_LINE_H))
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .px(px(6.0))
        .bg(move || design::panel2())
        .child(
            text(header)
                .flex_1()
                .text_size(px(crate::ui::CODE))
                .line_height(px(crate::ui::CODE_LINE))
                .text_color(move || design::text2())
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        )
        .child(crate::widgets::small_button(
            {
                let language = language;
                move || i18n::t(language.get().as_str(), label_key)
            },
            move || apply(hunk_index, None, staged),
        ))
        .into_element();
}

/// 增删行：行尾一个「暂存/取消暂存此行」的小按钮。
fn line_row_with_piece(
    line: &domain::diff::DiffLine,
    hunk_index: usize,
    line_index: usize,
    staged: bool,
    language: Signal<SharedString>,
    apply: std::sync::Arc<dyn Fn(usize, Option<usize>, bool)>,
) -> AnyElement {
    let (background, foreground) = match line.kind {
        DiffLineKind::Add => (design::add_bg(), design::add_fg()),
        _ => (design::del_bg(), design::del_fg()),
    };
    let old = line.old.map(|value| value.to_string()).unwrap_or_default();
    let new = line.new.map(|value| value.to_string()).unwrap_or_default();
    let content = line.text.clone();
    let label_key = if staged {
        "changes.unstageLine"
    } else {
        "changes.stageLine"
    };
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
                .text_size(px(crate::ui::CODE))
                .line_height(px(crate::ui::CODE_LINE))
                .text_color(move || design::text3()),
        )
        .child(
            text(new)
                .w(px(34.0))
                .flex_shrink_0()
                .pl(px(6.0))
                .text_size(px(crate::ui::CODE))
                .line_height(px(crate::ui::CODE_LINE))
                .text_color(move || design::text3()),
        )
        .child(
            text(content)
                .flex_1()
                .text_size(px(crate::ui::CODE))
                .line_height(px(crate::ui::CODE_LINE))
                .text_color(move || foreground)
                .text_overflow(nsh_core::EllipsizeMode::Clip),
        )
        .child(crate::widgets::small_button(
            {
                let language = language;
                move || i18n::t(language.get().as_str(), label_key)
            },
            move || apply(hunk_index, Some(line_index), staged),
        ))
        .into_element();
}

/// 变更清单弹层（设计板 11）：清单列表、设为当前、把选中文件移入、新建清单。
fn changelist_pop(
    language: Signal<SharedString>,
    tab: Tab,
    open: Signal<bool>,
    actions: Actions,
    selected_path: Option<String>,
) -> impl IntoElement {
    let set_active = actions.set_active_changelist.clone();
    let move_to = actions.move_to_changelist.clone();
    let create = actions.create_changelist.clone();
    let new_name = signal(String::new());
    let mut rows: Vec<AnyElement> = Vec::new();

    // 「默认清单」+ 已有清单。
    let mut entries: Vec<(Option<String>, usize)> = vec![(None, 0)];
    for list in &tab.changelists {
        entries.push((Some(list.name.clone()), list.paths.len()));
    }
    for (name, count) in entries {
        let set_active = set_active.clone();
        let move_to = move_to.clone();
        let selected = selected_path.clone();
        let name_for_active = name.clone();
        let name_for_move = name.clone();
        let is_active = tab.active_changelist == name;
        let label = name.clone().unwrap_or_else(|| {
            i18n::t(language.get_untracked().as_str(), "changelists.default")
        });
        rows.push(
            view()
                .w_full()
                .h(px(24.0))
                .px(px(6.0))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .rounded(px(4.0))
                .bg(move || {
                    if is_active {
                        design::selected()
                    } else {
                        AdaptiveColor::TRANSPARENT
                    }
                })
                .hover(|style| style.bg(move || design::hover()))
                .on_press(move || set_active(name_for_active.clone()))
                .child(
                    view()
                        .w(px(6.0))
                        .h(px(6.0))
                        .rounded(px(3.0))
                        .bg(move || {
                            if is_active {
                                design::accent()
                            } else {
                                design::line_strong()
                            }
                        }),
                )
                .child(
                    text(label)
                        .flex_1()
                        .text_size(px(crate::ui::BODY))
                        .line_height(px(14.0))
                        .text_color(move || design::text())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .child(
                    text(count.to_string())
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(move || design::text3()),
                )
                .child(
                    view()
                        .h(px(16.0))
                        .px(px(4.0))
                        .items_center()
                        .rounded(px(3.0))
                        .opacity({
                            let selected = selected.clone();
                            move || if selected.is_some() { 1.0_f32 } else { 0.4_f32 }
                        })
                        .when(selected.is_some(), |this| {
                            let move_to = move_to.clone();
                            let selected = selected.clone();
                            let name_for_move = name_for_move.clone();
                            this.on_press(move || {
                                if let Some(path) = selected.clone() {
                                    move_to(path, name_for_move.clone());
                                }
                            })
                        })
                        .child(
                            text(move || {
                                i18n::t(language.get().as_str(), "changelists.moveHere")
                            })
                            .text_size(px(crate::ui::SECONDARY))
                            .line_height(px(11.0))
                            .text_color(move || design::accent()),
                        ),
                )
                .into_element(),
        );
    }

    return view()
        .w(px(300.0))
        .p(px(6.0))
        .flex_col()
        .gap(px(4.0))
        .rounded(px(metrics::RADIUS_PANEL))
        .border_1()
        .border_color(move || design::line())
        .bg(move || design::token("popover"))
        .child(
            text(move || i18n::t(language.get().as_str(), "changelists.title"))
                .text_size(px(crate::ui::SECONDARY))
                .font_weight(FontWeight(600))
                .line_height(px(14.0))
                .text_color(move || design::text3()),
        )
        .child(rows)
        .child(
            view()
                .w_full()
                .mt(px(2.0))
                .pt(px(4.0))
                .flex_row()
                .items_center()
                .gap(px(4.0))
                .border_t_1()
                .border_color(move || design::line())
                .child(
                    view()
                        .flex_1()
                        .h(px(22.0))
                        .px(px(6.0))
                        .items_center()
                        .rounded(px(4.0))
                        .border_1()
                        .border_color(move || design::field_border())
                        .bg(move || design::field())
                        .child(
                            nsh_base::input()
                                .value(new_name)
                                .on_change(move |text: &str| new_name.set(text.to_string()))
                                .placeholder(move || {
                                    i18n::t(language.get().as_str(), "changelists.newName")
                                })
                                .text_size(px(crate::ui::BODY))
                                .flex_1(),
                        ),
                )
                .child(crate::widgets::small_button(
                    move || i18n::t(language.get().as_str(), "changelists.create"),
                    {
                        let create = create.clone();
                        let open = open;
                        move || {
                            let name = new_name.get_untracked();
                            if !name.trim().is_empty() {
                                create(name.trim().to_string());
                                new_name.set(String::new());
                            }
                            open.set(false);
                        }
                    },
                )),
        )
        .child(
            text(move || i18n::t(language.get().as_str(), "changelists.footHint"))
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        );
}
