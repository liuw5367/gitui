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
            view().w_full().flex_1().min_h_0().child(
                if patch.raw.trim().is_empty()
                    || !patch
                        .lines
                        .iter()
                        .any(|line| line.kind == domain::diff::DiffLineKind::Hunk)
                {
                    view()
                        .w_full()
                        .h_full()
                        .items_center()
                        .justify_center()
                        .child(
                            text(move || {
                                i18n::t(
                                    language.get().as_str(),
                                    if patch.raw.trim().is_empty() {
                                        "changes.selectFile"
                                    } else {
                                        "diff.noTextChanges"
                                    },
                                )
                            })
                            .text_size(px(crate::ui::BODY))
                            .text_color(design::text3),
                        )
                        .into_element()
                } else {
                    crate::ui::diff_surface::staging_surface(
                        SharedString::from(patch.raw),
                        staged_selection,
                        language,
                        apply_piece,
                    )
                    .into_element()
                },
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
            view().w_full().flex_1().min_h_0().child(
                if patch.raw.trim().is_empty()
                    || !patch
                        .lines
                        .iter()
                        .any(|line| line.kind == domain::diff::DiffLineKind::Hunk)
                {
                    view()
                        .w_full()
                        .h_full()
                        .items_center()
                        .justify_center()
                        .child(
                            text(move || {
                                i18n::t(
                                    language.get().as_str(),
                                    if patch.raw.trim().is_empty() {
                                        "stash.diffEmpty"
                                    } else {
                                        "diff.noTextChanges"
                                    },
                                )
                            })
                            .text_size(px(crate::ui::BODY))
                            .text_color(design::text3),
                        )
                        .into_element()
                } else {
                    let handle = crate::ui::diff_surface::DiffHandle::new();
                    let copy =
                        crate::shared::diff::copy_hunk_handler(&patch.raw, cmd.notice, language);
                    crate::ui::diff_surface::diff_surface(
                        SharedString::from(patch.raw),
                        crate::ui::diff_surface::Layout::Unified,
                        false,
                        true,
                        &handle,
                        language,
                        copy,
                    )
                    .into_element()
                },
            ),
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
    let collapsed_changelists = cmd.state.collapsed_changelists;
    let changelist_tree_expanded = cmd.state.changelist_tree_expanded;
    let total = tab.changes.len();
    let staged_count = tab.changes.iter().filter(|entry| entry.staged).count();
    let unstaged_count = total - staged_count;
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
    let all_group_names = groups
        .iter()
        .map(|(name, _, _)| name.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let repo_id = tab.id;
    groups.sort_by_key(|(name, _, _)| {
        if name == &tab.active_changelist {
            0
        } else if name.is_none() {
            1
        } else {
            2
        }
    });
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
        let background_target = name.clone();
        let background_drop_target = drop_target;
        let active = tab.active_changelist == name;
        let count = indices.len();
        let mut group = view()
            .w_full()
            .flex_col()
            .bg(move || {
                if background_drop_target.get() == Some(background_target.clone()) {
                    design::accent_soft()
                } else {
                    design::content()
                }
            })
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
        let display_name = name.clone();
        let group_name = name.clone();
        let collapsed_for_rows = collapsed_changelists;
        let group_name_for_rows = group_name.clone();
        let repo_id_for_rows = repo_id;
        let collapsed_for_toggle = collapsed_changelists;
        let group_name_for_toggle = group_name.clone();
        let repo_id_for_toggle = repo_id;
        let collapsed_for_header = collapsed_changelists;
        let group_name_for_header = group_name.clone();
        let repo_id_for_header = repo_id;
        let row_name = name.clone();
        let row_cmd = cmd.clone();
        let row_stage_cmd = cmd.clone();
        let row_language = language;
        let row_menu_name = name.clone();
        group = group.child(
            view()
                .w_full()
                .h(px(23.0))
                .child(dynamic(move || -> AnyElement {
                    let collapsed = collapsed_for_header
                        .get()
                        .get(&repo_id_for_header)
                        .is_some_and(|groups| groups.contains(&group_name_for_header));
                    let icon = if collapsed { "chev-right" } else { "chev-down" };
                    let arrow = crate::ui::icon::icon_view(
                        icon,
                        crate::ui::icon::IconSize::Compact,
                        design::text3,
                    );
                    let leading = view()
                        .flex_row()
                        .items_center()
                        .gap(px(crate::ui::GAP_XS))
                        .child(arrow)
                        .child(view().w(px(7.0)).h(px(7.0)).rounded(px(4.0)).bg(move || {
                            if active {
                                design::accent()
                            } else {
                                design::text3()
                            }
                        }))
                        .into_element();
                    let group_name = group_name_for_toggle.clone();
                    let toggle = collapsed_for_toggle;
                    let row_label = display_name.clone();
                    let row = crate::ui::row::row_sized_flat(
                        move || {
                            row_label.clone().unwrap_or_else(|| {
                                i18n::t(language.get().as_str(), "changelists.default")
                            })
                        },
                        None,
                        Some(leading),
                        vec![
                            text(count.to_string())
                                .mr(px(crate::ui::GAP_SM))
                                .text_size(px(crate::ui::META))
                                .text_color(move || design::text2())
                                .into_element(),
                        ],
                        active,
                        false,
                        23.0,
                        move || {
                            toggle.update(|by_repo| {
                                let groups = by_repo.entry(repo_id_for_toggle).or_default();
                                if groups.contains(&group_name) {
                                    groups.remove(&group_name);
                                } else {
                                    groups.insert(group_name.clone());
                                }
                            });
                        },
                    );
                    row.into_element()
                }))
                .on_context_menu(move |event: &ContextMenuEvent| {
                    let lang = row_language.get_untracked();
                    let mut items = Vec::new();
                    if let Some(name) = row_menu_name.clone() {
                        let set_cmd = row_cmd.clone();
                        let set_name = name.clone();
                        items.push(
                            MenuItem::new(i18n::t(lang.as_str(), "changelists.setDefault"))
                                .on_select(move || {
                                    set_cmd.set_active_changelist(Some(set_name.clone()));
                                }),
                        );
                        items.push(MenuItem::separator());

                        let rename_cmd = row_cmd.clone();
                        let rename_name = name.clone();
                        items.push(
                            MenuItem::new(i18n::t(lang.as_str(), "changelists.renameAction"))
                                .on_select(move || {
                                    rename_cmd.open_rename_changelist(rename_name.clone());
                                }),
                        );
                        let delete_cmd = row_cmd.clone();
                        let delete_name = name;
                        items.push(
                            MenuItem::new(i18n::t(lang.as_str(), "changelists.delete")).on_select(
                                move || delete_cmd.delete_changelist(delete_name.clone()),
                            ),
                        );
                        items.push(MenuItem::separator());
                    }
                    let stage_cmd = row_stage_cmd.clone();
                    let stage_name = row_name.clone();
                    items.push(
                        MenuItem::new(i18n::t(lang.as_str(), "changes.stageAll")).on_select(
                            move || stage_cmd.stage_changelist(stage_name.clone(), true),
                        ),
                    );
                    let unstage_cmd = row_stage_cmd.clone();
                    let unstage_name = row_name.clone();
                    items.push(
                        MenuItem::new(i18n::t(lang.as_str(), "changes.unstageAll")).on_select(
                            move || unstage_cmd.stage_changelist(unstage_name.clone(), false),
                        ),
                    );
                    crate::ui::menu::popup_at(items, event.page_position);
                })
                .into_element(),
        );

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
        let tree_expanded_for_rows = changelist_tree_expanded;
        group = group.child(dynamic(move || -> AnyElement {
            if collapsed_for_rows
                .get()
                .get(&repo_id_for_rows)
                .is_some_and(|groups| groups.contains(&group_name_for_rows))
            {
                return view().w_full().h(px(0.0)).into_element();
            }
            let needle = filter_query.get().trim().to_lowercase();
            let tree_view = tree_mode.get();
            let visible_indices = indices
                .iter()
                .copied()
                .filter(|index| {
                    entries.get(*index).is_some_and(|entry| {
                        needle.is_empty() || entry.path.to_lowercase().contains(&needle)
                    })
                })
                .collect::<Vec<_>>();
            if tree_view {
                let paths = visible_indices
                    .iter()
                    .filter_map(|index| entries.get(*index).map(|entry| entry.path.clone()))
                    .collect::<Vec<_>>();
                let (items, default_expanded) = crate::ui::tree::file_tree(
                    paths,
                    tree.merge_empty_dirs,
                    tree.compact_single_child,
                );
                let path_indices = visible_indices
                    .iter()
                    .filter_map(|index| {
                        entries
                            .get(*index)
                            .map(|entry| (entry.path.clone(), *index))
                    })
                    .collect::<std::collections::BTreeMap<_, _>>();
                let tree_key = (repo_id_for_rows, group_name_for_rows.clone());
                let selected_id = tab
                    .selected_change
                    .and_then(|index| entries.get(index))
                    .filter(|entry| path_indices.contains_key(&entry.path))
                    .map(|entry| SharedString::from(format!("file:{}", entry.path)));
                let tree_select = command_for_rows.clone();
                let tree_expanded = tree_expanded_for_rows;
                let expanded_key = tree_key.clone();
                let render_entries = entries.clone();
                let render_indices = path_indices.clone();
                let render_lists = lists_for_rows.clone();
                let render_root = root_for_rows.clone();
                let render_toggle = toggle_for_rows.clone();
                let render_discard = discard_for_rows.clone();
                let render_copy = copy_for_rows.clone();
                let render_open = open_for_rows.clone();
                let render_reveal = reveal_for_rows.clone();
                let render_move = move_for_rows.clone();
                let render_dragged = dragged_path;
                let render_target = drop_target;
                return crate::ui::tree::tree_with_renderer(
                    items,
                    default_expanded,
                    move || tree_expanded.get().get(&expanded_key).cloned(),
                    selected_id,
                    move |id| {
                        if let Some(index) = id
                            .strip_prefix("file:")
                            .and_then(|path| path_indices.get(path))
                        {
                            tree_select.select_change(*index);
                        }
                    },
                    move |expanded| {
                        tree_expanded.update(|by_group| {
                            by_group.insert(tree_key.clone(), expanded);
                        });
                    },
                    move |tree_entry| {
                        let entry_id = tree_entry.id.to_string();
                        let depth = tree_entry.depth as f32;
                        if tree_entry.folder {
                            return view()
                                .w_full()
                                .h(px(crate::ui::ROW))
                                .flex_row()
                                .items_center()
                                .gap(px(crate::ui::GAP_XS))
                                .pl(px(4.0 + depth * 12.0))
                                .pr(px(4.0))
                                .child(
                                    view()
                                        .app_class(Some(SharedString::from(
                                            "group-aria-expanded:rotate-90",
                                        )))
                                        .child(crate::ui::icon::icon_view(
                                            "chev-right",
                                            crate::ui::icon::IconSize::Compact,
                                            design::text3,
                                        )),
                                )
                                .child(crate::ui::icon::icon_view(
                                    "folder",
                                    crate::ui::icon::IconSize::Button,
                                    design::text3,
                                ))
                                .child(
                                    text(tree_entry.label.to_string())
                                        .flex_1()
                                        .min_w_0()
                                        .text_size(px(crate::ui::CODE))
                                        .text_color(move || design::text2())
                                        .number_of_lines(1)
                                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                                )
                                .into_element();
                        }
                        let Some(index) = entry_id
                            .strip_prefix("file:")
                            .and_then(|path| render_indices.get(path).copied())
                        else {
                            return view().w_full().h(px(crate::ui::ROW)).into_element();
                        };
                        let Some(entry) = render_entries.get(index).cloned() else {
                            return view().w_full().h(px(crate::ui::ROW)).into_element();
                        };
                        let path = entry.path.clone();
                        let filename = tree_entry.label.to_string();
                        let status = entry.status_letter();
                        let source_list = entry.changelist.clone();
                        let start_path = path.clone();
                        let path_for_menu = path.clone();
                        let staged = entry.staged;
                        let untracked = entry.untracked;
                        let move_for_drag = render_move.clone();
                        let root_for_menu = render_root.clone();
                        let lists_for_menu = render_lists.clone();
                        let toggle_for_menu = render_toggle.clone();
                        let discard_for_menu = render_discard.clone();
                        let copy_for_menu = render_copy.clone();
                        let open_for_menu = render_open.clone();
                        let reveal_for_menu = render_reveal.clone();
                        let move_for_menu = render_move.clone();
                        let target_for_row = render_target;
                        let drag_start = render_dragged;
                        let drag_end = render_dragged;
                        view()
                            .w_full()
                            .h(px(crate::ui::ROW))
                            .flex_row()
                            .items_center()
                            .gap(px(crate::ui::GAP_XS))
                            .pl(px(4.0 + depth * 12.0))
                            .pr(px(4.0))
                            .child(crate::ui::check::indicator_action(
                                entry.staged,
                                filename.clone(),
                                {
                                    let command = render_toggle.clone();
                                    move || command(index)
                                },
                            ))
                            .child(crate::ui::icon::icon_view(
                                "file",
                                crate::ui::icon::IconSize::Button,
                                design::text3,
                            ))
                            .child(
                                text(filename)
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(crate::ui::CODE))
                                    .text_color(move || design::text())
                                    .number_of_lines(1)
                                    .text_overflow(nsh_core::EllipsizeMode::Tail),
                            )
                            .child(
                                text(format!("+{}", entry.additions))
                                    .flex_shrink_0()
                                    .text_size(px(crate::ui::META))
                                    .text_color(move || design::add_fg()),
                            )
                            .child(
                                text(format!("−{}", entry.deletions))
                                    .flex_shrink_0()
                                    .text_size(px(crate::ui::META))
                                    .text_color(move || design::del_fg()),
                            )
                            .child(
                                text(status.to_string())
                                    .w(px(14.0))
                                    .flex_shrink_0()
                                    .text_size(px(crate::ui::META))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(move || status_color(status)),
                            )
                            .on_pan_start(move |_: &PanEvent| {
                                drag_start.set(Some(start_path.clone()));
                                target_for_row.set(None);
                            })
                            .on_pan_end(move |event: &PanEvent| {
                                if !event.cancelled
                                    && let Some(Some(target)) = target_for_row.get_untracked()
                                    && Some(target.clone()) != source_list
                                {
                                    move_for_drag(path.clone(), Some(target));
                                } else if !event.cancelled
                                    && let Some(None) = target_for_row.get_untracked()
                                    && source_list.is_some()
                                {
                                    move_for_drag(path.clone(), None);
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
                                    staged,
                                    untracked,
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
                            .into_element()
                    },
                )
                .into_element();
            }
            let mut rows: Vec<AnyElement> = Vec::new();
            for index in visible_indices {
                let Some(entry) = entries.get(index).cloned() else {
                    continue;
                };

                let path = entry.path.clone();
                let filename = path.clone();
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
                let tooltip_path = path.clone();
                let staged_for_menu = entry.staged;
                let untracked_for_menu = entry.untracked;
                let filename_for_row = filename.clone();
                let selected_file = tab.selected_change == Some(index);
                let drag_start = dragged_path;
                let drag_end = dragged_path;
                let target_for_row = drop_target;
                rows.push(
                    view()
                        .w_full()
                        .h(px(28.0))
                        .pl(px(7.0))
                        .pr(px(7.0))
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child(crate::ui::check::indicator_action(
                            entry.staged,
                            filename.clone(),
                            move || toggle.toggle_stage(index),
                        ))
                        .child(
                            view()
                                .flex_1()
                                .min_w_0()
                                .child(crate::ui::row::row_sized(
                                    move || filename_for_row.clone(),
                                    None,
                                    None,
                                    vec![
                                        text(format!("+{}", entry.additions))
                                            .text_size(px(crate::ui::META))
                                            .text_color(move || design::add_fg())
                                            .into_element(),
                                        text(format!("−{}", entry.deletions))
                                            .text_size(px(crate::ui::META))
                                            .text_color(move || design::del_fg())
                                            .into_element(),
                                        text(status.to_string())
                                            .w(px(14.0))
                                            .text_size(px(crate::ui::META))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(move || status_color(status))
                                            .into_element(),
                                    ],
                                    selected_file,
                                    false,
                                    28.0,
                                    move || select.select_change(index),
                                ))
                                .tooltip(crate::ui::button::text_tooltip(tooltip_path)),
                        )
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
        .flex_1()
        .min_h_0()
        .flex_col()
        .bg(move || design::content())
        .child(
            view()
                .w_full()
                .h(px(26.0))
                .px(px(8.0))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .bg(move || design::panel2())
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
                .child(
                    text(move || {
                        i18n::with_arguments(
                            language.get().as_str(),
                            "changes.summary",
                            &[
                                ("staged", staged_count.to_string()),
                                ("unstaged", unstaged_count.to_string()),
                            ],
                        )
                    })
                    .text_size(px(crate::ui::META))
                    .text_color(move || design::text3()),
                )
                .child(view().flex_1())
                .child(tree_mode_controls(language, tree_mode))
                .child(dynamic(move || -> AnyElement {
                    let collapsed_now = collapsed_changelists.get();
                    let all_collapsed = all_group_names.iter().all(|name| {
                        collapsed_now
                            .get(&repo_id)
                            .is_some_and(|groups| groups.contains(name))
                    });
                    let icon = if all_collapsed {
                        "expand-all"
                    } else {
                        "collapse-all"
                    };
                    let label_key = if all_collapsed {
                        "changelists.expandAll"
                    } else {
                        "changelists.collapseAll"
                    };
                    let names = all_group_names.clone();
                    crate::ui::button::icon_button(
                        icon,
                        move || i18n::t(language.get().as_str(), label_key),
                        crate::ui::button::ButtonSize::Small,
                        false,
                        move || {
                            collapsed_changelists.update(|by_repo| {
                                let collapsed = by_repo.entry(repo_id).or_default();
                                if all_collapsed {
                                    for name in &names {
                                        collapsed.remove(name);
                                    }
                                } else {
                                    collapsed.extend(names.iter().cloned());
                                }
                            });
                        },
                    )
                    .into_element()
                }))
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
                .h(px(34.0))
                .px(px(7.0))
                .py(px(4.0))
                .items_center()
                .border_b_1()
                .border_color(move || design::line_soft())
                .child(
                    crate::ui::text_input::search_field_clearable(
                        filter_query,
                        move || i18n::t(language.get().as_str(), "changelists.filter"),
                        move || i18n::t(language.get().as_str(), "history.clear"),
                        move |value| filter_query.set(value.to_string()),
                    )
                    .h(px(24.0)),
                )
                .into_element();
        }))
        .child(
            scroll()
                .w_full()
                .flex_1()
                .min_h_0()
                .child(view().w_full().min_h_0().flex_col().child(group_views)),
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
                .bottom(px(24.0))
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
        .child(rename_changelist_dialog(language, cmd.clone()))
        .into_element();
}

fn tree_mode_controls(language: Signal<SharedString>, tree_mode: Signal<bool>) -> AnyElement {
    return dynamic(move || -> AnyElement {
        let selected_tree = tree_mode.get();
        return crate::ui::button::file_list_toggle(language, selected_tree, move || {
            tree_mode.set(!selected_tree)
        })
        .into_element();
    });
}

fn rename_changelist_dialog(
    language: Signal<SharedString>,
    cmd: ChangesCommands,
) -> impl IntoElement {
    let target = cmd.state.rename_changelist;
    let value = cmd.state.rename_changelist_value;
    let tabs = cmd.session.tabs;
    let active = cmd.session.active;
    let check_target = target;
    let check_value = value;
    let can_submit = computed(move || {
        let Some(old_name) = check_target.get() else {
            return false;
        };
        let new_name = check_value.get();
        let new_name = new_name.trim();
        if new_name.is_empty() {
            return false;
        }
        let current = active.get();
        tabs.get().get(current).is_some_and(|tab| {
            !tab.changelists
                .iter()
                .any(|list| list.name == new_name && list.name != old_name)
        })
    });
    let dismiss_cmd = cmd.clone();
    return crate::ui::dialog::modal(
        move || target.get().is_some(),
        move || i18n::t(language.get().as_str(), "changelists.rename"),
        150,
        move || {
            let cancel_cmd = cmd.clone();
            let close_cmd = cmd.clone();
            let submit_cmd = cmd.clone();
            let input_value = value;
            let body = view()
                .w_full()
                .child(crate::ui::text_input::text_input_bound(
                    move || input_value.get(),
                    move || i18n::t(language.get().as_str(), "changelists.newName"),
                    false,
                    move |text| value.set(text),
                ));
            let cancel = crate::ui::button::button(
                move || i18n::t(language.get().as_str(), "common.cancel"),
                crate::ui::button::ButtonVariant::Secondary,
                crate::ui::button::ButtonSize::Regular,
                false,
                move || cancel_cmd.close_rename_changelist(),
            )
            .into_element();
            let close_label: std::rc::Rc<dyn Fn() -> String> =
                std::rc::Rc::new(move || i18n::t(language.get().as_str(), "common.close"));
            let footer = vec![
                cancel,
                dynamic(move || {
                    let disabled = !can_submit.get();
                    let submit = submit_cmd.clone();
                    crate::ui::button::button(
                        move || i18n::t(language.get().as_str(), "changelists.renameAction"),
                        crate::ui::button::ButtonVariant::Primary,
                        crate::ui::button::ButtonSize::Regular,
                        disabled,
                        move || {
                            if submit.rename_changelist() {
                                submit.close_rename_changelist();
                            }
                        },
                    )
                })
                .into_element(),
            ];
            crate::ui::dialog::dialog_panel(
                380.0,
                move || i18n::t(language.get().as_str(), "changelists.rename"),
                Some(close_label),
                body,
                footer,
                move || close_cmd.close_rename_changelist(),
            )
        },
        move || dismiss_cmd.close_rename_changelist(),
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
    let delete: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |name: String| cmd.delete_changelist(name))
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
        let move_here = crate::ui::button::button(
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
        );
        let delete_action = name.clone();
        let delete_label = label.clone();
        let is_default = name.is_none();
        let delete = delete.clone();
        let delete_button = crate::ui::button::icon_button_tuned(
            "x",
            move || {
                if is_default {
                    i18n::t(language.get().as_str(), "changelists.deleteDefault")
                } else {
                    i18n::with_arguments(
                        language.get().as_str(),
                        "changelists.deleteNamed",
                        &[("name", delete_label.clone())],
                    )
                }
            },
            crate::ui::button::ButtonSize::Small,
            None,
            Some(std::rc::Rc::new(design::danger)),
            is_default,
            move || {
                if let Some(name) = delete_action.clone() {
                    delete(name);
                }
            },
        );
        rows.push(
            view()
                .w_full()
                .h(px(30.0))
                .flex_row()
                .items_center()
                .gap(px(4.0))
                .child(
                    view()
                        .flex_1()
                        .min_w_0()
                        .child(crate::ui::row::row_sized_flat(
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
                            Vec::new(),
                            is_active,
                            false,
                            30.0,
                            move || set_active(name_for_active.clone()),
                        )),
                )
                .child(
                    text(count.to_string())
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(move || design::text3()),
                )
                .child(move_here)
                .child(delete_button)
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
            view()
                .w_full()
                .flex_row()
                .items_center()
                .child(
                    text(move || i18n::t(language.get().as_str(), "changelists.title"))
                        .flex_1()
                        .text_size(px(crate::ui::SECONDARY))
                        .font_weight(FontWeight(600))
                        .line_height(px(crate::ui::SECONDARY_LINE))
                        .text_color(move || design::text2()),
                )
                .child(crate::ui::button::icon_button(
                    "x",
                    move || i18n::t(language.get().as_str(), "common.close"),
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || open.set(false),
                )),
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
                        .min_w_0()
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
        );
}
