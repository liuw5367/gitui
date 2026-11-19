//! 变更视图（设计板 10 / 11 的左侧主区）：变更列表 + 文件 diff。
//!
//! 列表按「暂存 / 未暂存」两段呈现，段内按目录分组；行高 24、状态字母与 +/− 统计
//! 来自设计稿。changelists 通过列表上方的选择器切换（本地存储，按仓库分组）。
//!
//! 入口 `changes` 收本功能的状态与用例；外部打开与复制走 `shared::os` 的自由函数
//! （提示与文案取自 `cmd`），冲突文件的解决走 `cmd.merge`。

use super::commands::ChangesCommands;
use crate::app::session::Tab;
use crate::design::{self, metrics};
use crate::i18n;
use crate::widgets;
use domain::diff::DiffLineKind;
use nsh_core::element::desktop::MenuItem;
use nsh_core::prelude::*;

pub(crate) fn change_diff(cmd: &ChangesCommands, tab: Tab) -> AnyElement {
    let language = cmd.language;
    if let Some((label, patch)) = tab.stash_diff.clone() {
        return stash_diff_view(language, cmd, tab.id, label, patch);
    }
    let patch = tab.change_diff.clone();
    let label = tab
        .selected_change
        .and_then(|index| tab.changes.get(index))
        .map(|entry| entry.path.clone())
        .unwrap_or_default();
    let blame_cmd = cmd.clone();
    let blame_path = label.clone();
    let additions: u32 = patch.files.iter().map(|file| file.additions).sum();
    let deletions: u32 = patch.files.iter().map(|file| file.deletions).sum();
    // 逐行走一遍，记住当前在第几个 hunk、hunk 内第几行，供行级操作定位。
    let staged_selection = tab
        .selected_change
        .and_then(|index| tab.changes.get(index))
        .map(|entry| entry.staged)
        .unwrap_or(false);
    // 视图回调要 `Arc<dyn Fn>`，这里把本功能的命令包一层。
    let apply_piece: std::sync::Arc<dyn Fn(usize, Option<usize>, bool)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |hunk: usize, line: Option<usize>, reverse: bool| {
            cmd.apply_piece(hunk, line, reverse)
        })
    };
    let mut rows: Vec<AnyElement> = Vec::new();
    let mut hunk_index = 0usize;
    let mut line_index = 0usize;
    for line in patch.lines.iter() {
        match line.kind {
            DiffLineKind::Hunk => {
                if hunk_index > 0 || !rows.is_empty() {
                    // 第一个 hunk 之前可能已有 Meta 行，不算 hunk。
                }
                let apply = apply_piece.clone();
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
                let apply = apply_piece.clone();
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
    let clear = cmd.clone();
    return view()
        .w_full()
        .h_full()
        .min_h_0()
        .flex_col()
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
                .child(crate::ui::button::button_icon(
                    move || i18n::t(language.get().as_str(), "history.backToList"),
                    "chev-left",
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || clear.clear_selected_change(),
                ))
                .child(crate::ui::icon::icon_view(
                    "file",
                    crate::ui::icon::IconSize::Button,
                    design::text3,
                ))
                .child(
                    text(label)
                        .flex_1()
                        .text_size(px(crate::ui::CODE))
                        .text_color(move || design::text())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "files.blame"),
                    crate::ui::button::ButtonVariant::Ghost,
                    crate::ui::button::ButtonSize::Small,
                    blame_path.is_empty(),
                    move || blame_cmd.open_blame(blame_path.clone()),
                ))
                .child(
                    text(format!("+{} −{}", additions, deletions))
                        .text_size(px(crate::ui::CODE))
                        .text_color(move || design::text3()),
                ),
        )
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .child(
                    scroll()
                        .w_full()
                        .h_full()
                        .child(view().w_full().flex_col().child(if rows.is_empty() {
                            view()
                                .w_full()
                                .h(px(48.0))
                                .items_center()
                                .justify_center()
                                .child(
                                    text(move || {
                                        i18n::t(language.get().as_str(), "changes.selectFile")
                                    })
                                    .text_size(px(crate::ui::BODY))
                                    .text_color(move || design::text3()),
                                )
                                .into_element()
                        } else {
                            view().w_full().flex_col().child(rows).into_element()
                        })),
                ),
        )
        .into_element();
}

fn stash_diff_view(
    language: Signal<SharedString>,
    cmd: &ChangesCommands,
    tab_id: u64,
    label: String,
    patch: domain::diff::Patch,
) -> AnyElement {
    let rows = patch.lines.iter().map(diff_row).collect::<Vec<_>>();
    let additions: u32 = patch.files.iter().map(|file| file.additions).sum();
    let deletions: u32 = patch.files.iter().map(|file| file.deletions).sum();
    let clear = cmd.clone();
    return view()
        .w_full()
        .h_full()
        .flex_col()
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
                .child(crate::ui::icon::icon_view(
                    "archive",
                    crate::ui::icon::IconSize::Button,
                    design::text3,
                ))
                .child(
                    text(label)
                        .flex_1()
                        .text_size(px(crate::ui::CODE))
                        .text_color(move || design::text())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .child(
                    text(format!("+{} −{}", additions, deletions))
                        .text_size(px(crate::ui::CODE))
                        .text_color(move || design::text3()),
                )
                .child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "stash.backToChanges"),
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || clear.clear_stash_diff_at(tab_id),
                )),
        )
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .child(scroll().w_full().h_full().child(if rows.is_empty() {
                    view()
                        .w_full()
                        .h(px(48.0))
                        .items_center()
                        .justify_center()
                        .child(
                            text(move || i18n::t(language.get().as_str(), "stash.diffEmpty"))
                                .text_size(px(crate::ui::BODY))
                                .text_color(move || design::text3()),
                        )
                        .into_element()
                } else {
                    view().w_full().flex_col().child(rows).into_element()
                })),
        )
        .into_element();
}

pub(crate) fn changelists_panel(
    cmd: &ChangesCommands,
    tab: Tab,
    show_tree: bool,
    tree: crate::app::session::TreeOptions,
) -> AnyElement {
    let language = cmd.language;
    let tree_mode = signal(show_tree);
    let filter_open = signal(false);
    let filter_query = signal(String::new());
    let dragged_path = signal(Option::<String>::None);
    let drop_target = signal(Option::<Option<String>>::None);
    let manage_open = cmd.state.changelist_open;
    let total = tab.changes.len();
    let notice = cmd.notice;
    let copy_text: std::sync::Arc<dyn Fn(String)> = {
        let language = language;
        std::sync::Arc::new(move |value| crate::shared::os::copy_text(notice, language, value))
    };
    let open_editor: std::sync::Arc<dyn Fn(String)> = {
        let language = language;
        std::sync::Arc::new(move |path| crate::shared::os::open_editor(notice, language, path))
    };
    let reveal_path: std::sync::Arc<dyn Fn(String)> = {
        let language = language;
        std::sync::Arc::new(move |path| crate::shared::os::reveal_path(notice, language, path))
    };
    let toggle_stage: std::sync::Arc<dyn Fn(usize)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |index| cmd.toggle_stage(index))
    };
    let discard_change: std::sync::Arc<dyn Fn(usize)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |index| cmd.discard_change(index))
    };
    let move_to_changelist: std::sync::Arc<dyn Fn(String, Option<String>)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |path, target| cmd.move_to_changelist(path, target))
    };
    let mut groups: Vec<(Option<String>, String, Vec<usize>)> = vec![(
        None,
        i18n::t(language.get_untracked().as_str(), "changelists.default"),
        Vec::new(),
    )];
    for list in &tab.changelists {
        groups.push((Some(list.name.clone()), list.name.clone(), Vec::new()));
    }
    for (index, entry) in tab.changes.iter().enumerate() {
        if let Some((_, _, indices)) = groups
            .iter_mut()
            .find(|(name, _, _)| *name == entry.changelist)
        {
            indices.push(index);
        }
    }
    groups.sort_by_key(|(name, _, _)| {
        if name == &tab.active_changelist {
            0
        } else if name.is_none() {
            1
        } else {
            2
        }
    });
    let group_count = groups.len();
    let list_height = (total as f32 * 28.0 + group_count as f32 * 23.0).clamp(36.0, 252.0);

    let mut group_views: Vec<AnyElement> = Vec::new();
    if !tab.conflicts.is_empty() {
        let mut conflict_rows: Vec<AnyElement> = Vec::new();
        for path in tab.conflicts.clone() {
            let ours_path = path.clone();
            let theirs_path = path.clone();
            let ours = cmd.merge.clone();
            let theirs = cmd.merge.clone();
            conflict_rows.push(
                view()
                    .w_full()
                    .h(px(28.0))
                    .px(px(7.0))
                    .flex_row()
                    .items_center()
                    .gap(px(5.0))
                    .bg(move || design::del_bg())
                    .child(
                        text(path)
                            .flex_1()
                            .text_size(px(crate::ui::CODE))
                            .text_color(move || design::text())
                            .text_overflow(nsh_core::EllipsizeMode::Tail),
                    )
                    .child(crate::ui::button::button(
                        move || i18n::t(language.get().as_str(), "conflicts.ours"),
                        crate::ui::button::ButtonVariant::Secondary,
                        crate::ui::button::ButtonSize::Small,
                        false,
                        move || ours.resolve_conflict(ours_path.clone(), true),
                    ))
                    .child(crate::ui::button::button(
                        move || i18n::t(language.get().as_str(), "conflicts.theirs"),
                        crate::ui::button::ButtonVariant::Secondary,
                        crate::ui::button::ButtonSize::Small,
                        false,
                        move || theirs.resolve_conflict(theirs_path.clone(), false),
                    ))
                    .into_element(),
            );
        }
        group_views.push(
            view()
                .w_full()
                .flex_col()
                .child(
                    view()
                        .w_full()
                        .h(px(23.0))
                        .px(px(7.0))
                        .flex_row()
                        .items_center()
                        .child(
                            text(move || i18n::t(language.get().as_str(), "conflicts.title"))
                                .text_size(px(crate::ui::SECONDARY))
                                .font_weight(FontWeight(600))
                                .text_color(move || design::danger()),
                        ),
                )
                .child(conflict_rows)
                .into_element(),
        );
    }
    for (name, _label, mut indices) in groups {
        indices.sort_by_key(|index| tab.changes[*index].path.clone());
        let group_target = name.clone();
        let leave_target = name.clone();
        let active = tab.active_changelist == name;
        let count = indices.len();
        let mut group = view()
            .w_full()
            .flex_col()
            .on_mouse_enter(move |_: &MouseEvent| {
                if dragged_path.get_untracked().is_some() {
                    drop_target.set(Some(group_target.clone()));
                }
            })
            .on_mouse_leave(move |_: &MouseEvent| {
                if drop_target.get_untracked() == Some(leave_target.clone()) {
                    drop_target.set(None);
                }
            });
        let active_name = name.clone();
        let display_name = name.clone();
        let set_active = cmd.clone();
        group = group.child(crate::ui::row::row_sized(
            move || {
                display_name
                    .clone()
                    .unwrap_or_else(|| i18n::t(language.get().as_str(), "changelists.default"))
            },
            None,
            Some(
                view()
                    .flex_row()
                    .items_center()
                    .gap(px(4.0))
                    .child(widgets::icon("chev-down", 9.0, design::text3))
                    .child(view().w(px(7.0)).h(px(7.0)).rounded(px(4.0)).bg(move || {
                        if active {
                            design::accent()
                        } else {
                            design::text3()
                        }
                    }))
                    .into_element(),
            ),
            vec![
                text(count.to_string())
                    .text_size(px(crate::ui::META))
                    .text_color(move || design::text2())
                    .into_element(),
            ],
            active,
            false,
            23.0,
            move || set_active.set_active_changelist(active_name.clone()),
        ));

        let entries = tab.changes.clone();
        let command_for_rows = cmd.clone();
        let toggle_for_rows = toggle_stage.clone();
        let discard_for_rows = discard_change.clone();
        let copy_for_rows = copy_text.clone();
        let open_for_rows = open_editor.clone();
        let reveal_for_rows = reveal_path.clone();
        let move_for_rows = move_to_changelist.clone();
        let lists_for_rows = tab.changelists.clone();
        let root_for_rows = tab.path.clone();
        group = group.child(dynamic(move || -> AnyElement {
            let needle = filter_query.get().trim().to_lowercase();
            let tree_view = tree_mode.get();
            let mut rows: Vec<AnyElement> = Vec::new();
            let mut last_directory = String::new();
            for index in indices.iter().copied() {
                let Some(entry) = entries.get(index).cloned() else {
                    continue;
                };
                if !needle.is_empty() && !entry.path.to_lowercase().contains(&needle) {
                    continue;
                }
                if tree_view {
                    let directory = entry.directory().to_string();
                    let direct_files = entries
                        .iter()
                        .filter(|candidate| candidate.directory() == directory)
                        .count();
                    if !directory.is_empty()
                        && directory != last_directory
                        && tree.shows_directory(direct_files)
                    {
                        let directories = entries
                            .iter()
                            .map(|candidate| candidate.directory().to_string())
                            .filter(|candidate| !candidate.is_empty())
                            .collect::<Vec<_>>();
                        rows.push(
                            view()
                                .w_full()
                                .h(px(19.0))
                                .pl(px(17.0))
                                .flex_row()
                                .items_center()
                                .gap(px(5.0))
                                .child(widgets::icon("folder", 10.0, design::text3))
                                .child(
                                    text(tree.compact_label(&directories, &directory))
                                        .text_size(px(crate::ui::META))
                                        .text_color(move || design::text3()),
                                )
                                .into_element(),
                        );
                        last_directory = directory;
                    }
                }

                let path = entry.path.clone();
                let filename = path.rsplit('/').next().unwrap_or(&path).to_string();
                let source_list = entry.changelist.clone();
                let status = entry.status_letter();
                let select = command_for_rows.clone();
                let toggle = command_for_rows.clone();
                let drop_cmd = command_for_rows.clone();
                let toggle_for_menu = toggle_for_rows.clone();
                let discard_for_menu = discard_for_rows.clone();
                let copy_for_menu = copy_for_rows.clone();
                let open_for_menu = open_for_rows.clone();
                let reveal_for_menu = reveal_for_rows.clone();
                let move_for_menu = move_for_rows.clone();
                let lists_for_menu = lists_for_rows.clone();
                let root_for_menu = root_for_rows.clone();
                let start_path = path.clone();
                let end_path = path.clone();
                let path_for_menu = path.clone();
                let staged_for_menu = entry.staged;
                let untracked_for_menu = entry.untracked;
                let filename_for_row = filename.clone();
                let selected_file = tab.selected_change == Some(index);
                let source_for_style = source_list.clone();
                let drag_start = dragged_path;
                let drag_end = dragged_path;
                let target_for_row = drop_target;
                rows.push(
                    view()
                        .w_full()
                        .h(px(28.0))
                        .pl(px(if tree_view { 19.0 } else { 7.0 }))
                        .pr(px(7.0))
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .rounded(px(5.0))
                        .bg(move || {
                            if target_for_row.get() == Some(source_for_style.clone()) {
                                design::accent_soft()
                            } else if selected_file {
                                design::selected()
                            } else {
                                design::content()
                            }
                        })
                        .child(crate::ui::check::indicator_action(
                            entry.staged,
                            filename.clone(),
                            move || toggle.toggle_stage(index),
                        ))
                        .child(view().flex_1().min_w_0().child(crate::ui::row::row_sized(
                            move || filename_for_row.clone(),
                            None,
                            None,
                            vec![
                                    text(status.to_string())
                                        .w(px(10.0))
                                        .text_size(px(crate::ui::META))
                                        .font_weight(FontWeight(700))
                                        .text_color(move || status_color(status))
                                        .into_element(),
                                    text(format!("+{}", entry.additions))
                                        .text_size(px(crate::ui::META))
                                        .text_color(move || design::add_fg())
                                        .into_element(),
                                    text(format!("−{}", entry.deletions))
                                        .text_size(px(crate::ui::META))
                                        .text_color(move || design::del_fg())
                                        .into_element(),
                                ],
                            selected_file,
                            false,
                            28.0,
                            move || select.select_change(index),
                        )))
                        .on_pan_start(move |_: &PanEvent| {
                            drag_start.set(Some(start_path.clone()));
                            target_for_row.set(None);
                        })
                        .on_pan_end(move |event: &PanEvent| {
                            if !event.cancelled
                                && let Some(Some(target)) = target_for_row.get_untracked()
                                && Some(target.clone()) != source_list
                            {
                                drop_cmd.move_to_changelist(end_path.clone(), Some(target));
                            } else if !event.cancelled
                                && let Some(None) = target_for_row.get_untracked()
                                && source_list.is_some()
                            {
                                drop_cmd.move_to_changelist(end_path.clone(), None);
                            }
                            drag_end.set(None);
                            target_for_row.set(None);
                        })
                        .on_context_menu(move |event: &ContextMenuEvent| {
                            let items = file_menu(
                                language,
                                root_for_menu.as_str(),
                                path_for_menu.as_str(),
                                lists_for_menu.clone(),
                                staged_for_menu,
                                untracked_for_menu,
                                index,
                                toggle_for_menu.clone(),
                                discard_for_menu.clone(),
                                copy_for_menu.clone(),
                                open_for_menu.clone(),
                                reveal_for_menu.clone(),
                                move_for_menu.clone(),
                            );
                            crate::ui::menu::popup_at(items, event.page_position);
                        })
                        .into_element(),
                );
            }
            return view().w_full().flex_col().child(rows).into_element();
        }));
        group_views.push(group.into_element());
    }

    let manage_cmd = cmd.clone();
    let tab_for_manage = tab.clone();
    return view()
        .w_full()
        .flex_shrink_0()
        .flex_col()
        .rounded(px(7.0))
        .border_1()
        .border_color(move || design::line())
        .bg(move || design::content())
        .child(
            view()
                .w_full()
                .h(px(26.0))
                .px(px(8.0))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .rounded_t(px(7.0))
                .bg(move || design::panel2())
                .border_b_1()
                .border_color(move || design::line_soft())
                .child(crate::ui::icon::icon_view(
                    "branch",
                    crate::ui::icon::IconSize::Compact,
                    design::text2,
                ))
                .child(
                    text(move || i18n::t(language.get().as_str(), "changelists.title"))
                        .text_size(px(crate::ui::SECONDARY))
                        .font_weight(FontWeight(600))
                        .text_color(move || design::text2()),
                )
                .child(
                    text(move || {
                        i18n::with_arguments(
                            language.get().as_str(),
                            "changelists.files",
                            &[("count", total.to_string())],
                        )
                    })
                    .text_size(px(crate::ui::META))
                    .text_color(move || design::text3()),
                )
                .child(view().flex_1())
                .child(tree_mode_controls(language, tree_mode))
                .child(crate::ui::button::icon_button(
                    "filter",
                    move || i18n::t(language.get().as_str(), "changelists.filter"),
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || filter_open.set(!filter_open.get_untracked()),
                )),
        )
        .child(dynamic(move || -> AnyElement {
            if !filter_open.get() {
                return view().w_full().h(px(0.0)).into_element();
            }
            return view()
                .w_full()
                .h(px(30.0))
                .px(px(7.0))
                .items_center()
                .border_b_1()
                .border_color(move || design::line_soft())
                .child(crate::ui::text_input::text_input(
                    filter_query,
                    move || i18n::t(language.get().as_str(), "changelists.filter"),
                    false,
                    move |value| filter_query.set(value.to_string()),
                ))
                .into_element();
        }))
        .child(
            scroll()
                .w_full()
                .h(px(list_height))
                .child(view().w_full().flex_col().child(group_views)),
        )
        .child(
            view()
                .w_full()
                .h(px(9.0))
                .flex_row()
                .items_center()
                .justify_center()
                .border_t_1()
                .border_color(move || design::line_soft())
                .bg(move || design::panel2())
                .child(
                    text("⠿")
                        .text_size(px(crate::ui::META))
                        .text_color(move || design::text3()),
                ),
        )
        .child(
            view()
                .w_full()
                .h(px(24.0))
                .px(px(8.0))
                .flex_row()
                .items_center()
                .gap(px(5.0))
                .child(
                    text("⋯")
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(move || design::text3()),
                )
                .child(
                    text(move || i18n::t(language.get().as_str(), "changelists.dragHint"))
                        .flex_1()
                        .text_size(px(crate::ui::META))
                        .text_color(move || design::text3())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "changelists.manage"),
                    crate::ui::button::ButtonVariant::Ghost,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || manage_open.set(!manage_open.get_untracked()),
                )),
        )
        .child(
            view()
                .absolute()
                .top(px(30.0))
                .right(px(8.0))
                .child(dynamic(move || -> AnyElement {
                    if !manage_open.get() {
                        return view().w(px(0.0)).h(px(0.0)).into_element();
                    }
                    let selected_path = tab_for_manage
                        .selected_change
                        .and_then(|index| tab_for_manage.changes.get(index))
                        .map(|entry| entry.path.clone());
                    return changelist_pop(
                        language,
                        tab_for_manage.clone(),
                        manage_open,
                        &manage_cmd,
                        selected_path,
                    )
                    .into_element();
                })),
        )
        .into_element();
}

fn tree_mode_controls(language: Signal<SharedString>, tree_mode: Signal<bool>) -> AnyElement {
    return dynamic(move || -> AnyElement {
        let selected_tree = tree_mode.get();
        let tree_variant = if selected_tree {
            crate::ui::button::ButtonVariant::Secondary
        } else {
            crate::ui::button::ButtonVariant::Ghost
        };
        let flat_variant = if selected_tree {
            crate::ui::button::ButtonVariant::Ghost
        } else {
            crate::ui::button::ButtonVariant::Secondary
        };
        return view()
            .h(px(22.0))
            .flex_row()
            .items_center()
            .gap(px(1.0))
            .rounded(px(5.0))
            .border_1()
            .border_color(move || design::line_soft())
            .bg(move || design::panel())
            .child(crate::ui::button::button(
                move || i18n::t(language.get().as_str(), "changelists.tree"),
                tree_variant,
                crate::ui::button::ButtonSize::Small,
                false,
                move || tree_mode.set(true),
            ))
            .child(crate::ui::button::button(
                move || i18n::t(language.get().as_str(), "changelists.flat"),
                flat_variant,
                crate::ui::button::ButtonSize::Small,
                false,
                move || tree_mode.set(false),
            ))
            .into_element();
    });
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
    targets.push(
        MenuItem::new(i18n::t(lang, "changelists.default")).on_select({
            let move_to_changelist = move_to_changelist.clone();
            let path = path.to_string();
            move || move_to_changelist(path.clone(), None)
        }),
    );
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
        .child(crate::ui::button::button(
            {
                let language = language;
                move || i18n::t(language.get().as_str(), label_key)
            },
            crate::ui::button::ButtonVariant::Secondary,
            crate::ui::button::ButtonSize::Small,
            false,
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
        .child(crate::ui::button::button(
            {
                let language = language;
                move || i18n::t(language.get().as_str(), label_key)
            },
            crate::ui::button::ButtonVariant::Secondary,
            crate::ui::button::ButtonSize::Small,
            false,
            move || apply(hunk_index, Some(line_index), staged),
        ))
        .into_element();
}

/// 变更清单弹层（设计板 11）：清单列表、设为当前、把选中文件移入、新建清单。
fn changelist_pop(
    language: Signal<SharedString>,
    tab: Tab,
    open: Signal<bool>,
    cmd: &ChangesCommands,
    selected_path: Option<String>,
) -> impl IntoElement + use<> {
    // 视图回调要 `Arc<dyn Fn>`，这里把本功能的命令包一层。
    let set_active: std::sync::Arc<dyn Fn(Option<String>)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |name: Option<String>| cmd.set_active_changelist(name))
    };
    let move_to: std::sync::Arc<dyn Fn(String, Option<String>)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |path: String, name: Option<String>| {
            cmd.move_to_changelist(path, name)
        })
    };
    let create: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |name: String| cmd.create_changelist(name))
    };
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
        let label = name
            .clone()
            .unwrap_or_else(|| i18n::t(language.get_untracked().as_str(), "changelists.default"));
        rows.push(
            crate::ui::row::row(
                move || label.clone(),
                None,
                Some(
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
                        })
                        .into_element(),
                ),
                vec![
                    text(count.to_string())
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(move || design::text3())
                        .into_element(),
                    crate::ui::button::button(
                        move || i18n::t(language.get().as_str(), "changelists.moveHere"),
                        crate::ui::button::ButtonVariant::Ghost,
                        crate::ui::button::ButtonSize::Small,
                        selected.is_none(),
                        {
                            let move_to = move_to.clone();
                            let selected = selected.clone();
                            let name_for_move = name_for_move.clone();
                            move || {
                                if let Some(path) = selected.clone() {
                                    move_to(path, name_for_move.clone());
                                }
                            }
                        },
                    )
                    .into_element(),
                ],
                is_active,
                false,
                move || set_active(name_for_active.clone()),
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
                .flex_row()
                .gap(px(4.0))
                .child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "changes.stageAll"),
                    crate::ui::button::ButtonVariant::Ghost,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    {
                        let cmd = cmd.clone();
                        move || cmd.stage_all()
                    },
                ))
                .child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "changes.unstageAll"),
                    crate::ui::button::ButtonVariant::Ghost,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    {
                        let cmd = cmd.clone();
                        move || cmd.unstage_all()
                    },
                )),
        )
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
                        .child(crate::ui::text_input::text_input(
                            new_name,
                            move || i18n::t(language.get().as_str(), "changelists.newName"),
                            false,
                            move |text: &str| new_name.set(text.to_string()),
                        )),
                )
                .child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "changelists.create"),
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Small,
                    false,
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
