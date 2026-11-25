//! 引用管理（设计板 09 / 30 / 17）：左侧侧栏 + 右侧 Refs 表与 Reflog 列表。
//!
//! 侧栏是 Branches / Remotes / Tags / Stashes / Submodules / Worktrees 六组，行高 25、
//! 分组标题 27；Refs 表是同一批数据的完整列表（名称、上游、追踪、说明），行高 28，
//! 与设计稿的 `.refs` 表一致；Reflog 是最近的引用移动，提供「撤销到此处」
//! （`reset --hard`，需确认）。
//!
//! 入口 `refs_sidebar` 画侧栏、`refs_view` 画主区，两者都收本功能的状态与用例；
//! 新建分支/标签的表单归 `features::search`，复制与在 Finder 中显示走 `shared::os`。

use super::commands::RefsCommands;
use super::state::{RefsState, RefsTab};
use crate::app::session::Tab;
use crate::design::{self, metrics};
use crate::features::search::commands::SearchCommands;
use crate::i18n;
use crate::ui;
use nsh_core::element::desktop::MenuItem;
use nsh_core::prelude::*;

pub(crate) fn refs_sidebar(
    _state: &RefsState,
    cmd: &RefsCommands,
    language: Signal<SharedString>,
    tab: Tab,
) -> impl IntoElement + use<> {
    let refs = tab.refs.clone();
    let tab_id = tab.id;
    let collapsed = tab.sidebar_collapsed.clone();
    let mut groups: Vec<AnyElement> = Vec::new();
    let branch_tree = build_branch_tree(&refs.branches);
    let can_rebase = matches!(&tab.head, git::HeadState::Branch(_))
        && tab.operation == git::Operation::None
        && tab.status.is_empty();
    let has_origin = tab.remote_urls.iter().any(|(remote, _)| remote == "origin");
    let branch_rows = branch_tree_rows(
        &branch_tree,
        language,
        cmd,
        tab_id,
        tab.branch.as_str(),
        can_rebase,
        has_origin,
        &collapsed,
        0,
    );
    let new_branch_from_current: std::sync::Arc<dyn Fn()> = {
        let search = cmd.search.clone();
        let from = tab.branch.clone();
        std::sync::Arc::new(move || {
            search.open_form_from_tab(
                tab_id,
                crate::features::search::FormKind::NewBranch,
                from.clone(),
            )
        })
    };
    groups.push(group(
        language,
        cmd,
        tab_id,
        "group:branches",
        "sidebar.branches",
        refs.branches.len(),
        collapsed.contains("group:branches"),
        branch_rows,
        vec![("form.newBranch", true, new_branch_from_current)],
    ));

    let remote_infos = if refs.remote_info.is_empty() {
        refs.remotes
            .iter()
            .map(|name| {
                let url = tab
                    .remote_urls
                    .iter()
                    .find(|(remote, _)| remote == name)
                    .map(|(_, url)| url.clone())
                    .into_iter()
                    .collect::<Vec<_>>();
                git::RemoteInfo {
                    name: name.clone(),
                    fetch_urls: url.clone(),
                    push_urls: url,
                }
            })
            .collect::<Vec<_>>()
    } else {
        refs.remote_info.clone()
    };
    let remote_rows = remote_node_rows(
        &remote_infos,
        &refs.remote_branches,
        language,
        cmd,
        tab_id,
        tab.branch.as_str(),
        can_rebase,
        &collapsed,
    );
    let add_remote: std::sync::Arc<dyn Fn()> = {
        let search = cmd.search.clone();
        std::sync::Arc::new(move || {
            search.open_form_from_tab(
                tab_id,
                crate::features::search::FormKind::AddRemote,
                String::new(),
            )
        })
    };
    groups.push(group(
        language,
        cmd,
        tab_id,
        "group:remotes",
        "sidebar.remotes",
        remote_infos.len(),
        collapsed.contains("group:remotes"),
        remote_rows,
        vec![("remote.addRemote", true, add_remote)],
    ));

    let mut tags = refs.tags.clone();
    tags.sort_by(|left, right| {
        left.to_lowercase()
            .cmp(&right.to_lowercase())
            .then_with(|| left.cmp(right))
    });
    let mut configured_remotes = remote_infos
        .iter()
        .map(|remote| remote.name.clone())
        .collect::<Vec<_>>();
    configured_remotes.sort_by(|left, right| {
        left.to_lowercase()
            .cmp(&right.to_lowercase())
            .then_with(|| left.cmp(right))
    });
    let tag_rows: Vec<AnyElement> = tags
        .into_iter()
        .map(|tag| tag_row(language, cmd, tab_id, tag, configured_remotes.clone()))
        .collect();
    let new_tag: std::sync::Arc<dyn Fn()> = {
        let search = cmd.search.clone();
        std::sync::Arc::new(move || {
            search.open_form_from_tab(
                tab_id,
                crate::features::search::FormKind::NewTag,
                String::new(),
            )
        })
    };
    groups.push(group(
        language,
        cmd,
        tab_id,
        "group:tags",
        "sidebar.tags",
        refs.tags.len(),
        collapsed.contains("group:tags"),
        tag_rows,
        vec![("tag.newTag", true, new_tag)],
    ));

    let mut stashes = refs.stashes.clone();
    stashes.sort_by(|left, right| {
        left.summary
            .to_lowercase()
            .cmp(&right.summary.to_lowercase())
            .then_with(|| left.summary.cmp(&right.summary))
            .then_with(|| left.oid.cmp(&right.oid))
    });
    let stash_rows: Vec<AnyElement> = stashes
        .iter()
        .cloned()
        .map(|stash| stash_row(language, cmd, tab_id, stash))
        .collect();
    let stash_changes: std::sync::Arc<dyn Fn()> = {
        let search = cmd.search.clone();
        std::sync::Arc::new(move || {
            search.open_form_from_tab(
                tab_id,
                crate::features::search::FormKind::NewStash,
                String::new(),
            )
        })
    };
    let stash_selected_files: std::sync::Arc<dyn Fn()> = {
        let search = cmd.search.clone();
        let paths = tab
            .changes
            .iter()
            .map(|change| (change.path.clone(), change.untracked))
            .collect::<Vec<_>>();
        std::sync::Arc::new(move || search.open_stash_selected_from_tab(tab_id, paths.clone()))
    };
    groups.push(group(
        language,
        cmd,
        tab_id,
        "group:stashes",
        "sidebar.stashes",
        refs.stashes.len(),
        collapsed.contains("group:stashes"),
        stash_rows,
        vec![
            ("stash.stashChanges", !tab.changes.is_empty(), stash_changes),
            (
                "stash.stashSelectedFiles",
                !tab.changes.is_empty(),
                stash_selected_files,
            ),
        ],
    ));

    let mut submodules = refs.submodules.clone();
    submodules.sort_by(|left, right| {
        left.path
            .to_lowercase()
            .cmp(&right.path.to_lowercase())
            .then_with(|| left.path.cmp(&right.path))
    });
    let submodule_paths = submodules
        .iter()
        .map(|module| module.path.clone())
        .collect::<Vec<_>>();
    let submodule_rows: Vec<AnyElement> = submodules
        .into_iter()
        .map(|module| submodule_row(language, cmd, tab_id, module))
        .collect();
    let update_all_submodules: std::sync::Arc<dyn Fn()> = {
        let refs = cmd.clone();
        std::sync::Arc::new(move || refs.update_all_submodules_at(tab_id, submodule_paths.clone()))
    };
    groups.push(group(
        language,
        cmd,
        tab_id,
        "group:submodules",
        "sidebar.submodules",
        refs.submodules.len(),
        collapsed.contains("group:submodules"),
        submodule_rows,
        vec![(
            "submodule.updateAll",
            !refs.submodules.is_empty(),
            update_all_submodules,
        )],
    ));

    let mut worktrees = refs.worktrees.clone();
    worktrees.sort_by(|left, right| {
        short_path(&left.path)
            .to_lowercase()
            .cmp(&short_path(&right.path).to_lowercase())
            .then_with(|| short_path(&left.path).cmp(&short_path(&right.path)))
            .then_with(|| left.path.cmp(&right.path))
    });
    let prune_available = worktrees
        .iter()
        .any(|worktree| worktree.prunable && !worktree.locked);
    let worktree_rows: Vec<AnyElement> = worktrees
        .iter()
        .cloned()
        .map(|worktree| worktree_row(language, cmd, tab_id, worktree))
        .collect();
    let new_worktree: std::sync::Arc<dyn Fn()> = {
        let refs = cmd.clone();
        std::sync::Arc::new(move || refs.new_worktree_form_at(tab_id))
    };
    let prune_worktrees: std::sync::Arc<dyn Fn()> = {
        let refs = cmd.clone();
        std::sync::Arc::new(move || refs.prune_worktrees_at(tab_id))
    };
    groups.push(group(
        language,
        cmd,
        tab_id,
        "group:worktrees",
        "sidebar.worktrees",
        refs.worktrees.len(),
        collapsed.contains("group:worktrees"),
        worktree_rows,
        vec![
            ("worktree.new", true, new_worktree),
            ("worktree.prune", prune_available, prune_worktrees),
        ],
    ));

    return view()
        .w_full()
        .min_w_0()
        .h_full()
        .flex_col()
        .bg(move || design::sidebar())
        .child(
            scroll().w_full().h_full().child(
                view()
                    .w_full()
                    .pt(px(metrics::SIDE_SCROLL_PAD.0))
                    .pb(px(metrics::SIDE_SCROLL_PAD.2))
                    .px(px(metrics::SIDE_SCROLL_PAD.1))
                    .flex_col()
                    .child(groups),
            ),
        );
}

/// 分组标题与折叠控制。右键提供组级新增入口（如有）和展开/收起动作。
fn group(
    language: Signal<SharedString>,
    cmd: &RefsCommands,
    tab_id: u64,
    fold_key: &'static str,
    title_key: &'static str,
    count: usize,
    collapsed: bool,
    rows: Vec<AnyElement>,
    extra: Vec<(&'static str, bool, std::sync::Arc<dyn Fn()>)>,
) -> AnyElement {
    let click_cmd = cmd.clone();
    let click_key = fold_key.to_string();
    let header = crate::ui::row::section_header(
        move || i18n::t(language.get().as_str(), title_key),
        crate::ui::icon::icon_view(
            if collapsed { "chev-right" } else { "chev-down" },
            crate::ui::icon::IconSize::Compact,
            design::text3,
        )
        .into_element(),
        vec![
            text(count.to_string())
                .text_size(px(crate::ui::SECONDARY))
                .text_color(move || design::text3())
                .into_element(),
        ],
        metrics::SIDE_HEAD_H,
        move || click_cmd.toggle_sidebar_fold(tab_id, click_key.clone()),
    );
    let menu_cmd = cmd.clone();
    let menu_key = fold_key.to_string();
    let menu_extra = extra.clone();
    let mut content = view()
        .w_full()
        .flex_col()
        .mt(px(8.0))
        .on_context_menu(move |event: &nsh_core::ContextMenuEvent| {
            let mut items = Vec::new();
            for (label_key, enabled, action) in menu_extra.iter() {
                let action = action.clone();
                items.push(ui::menu::action(
                    i18n::t(language.get().as_str(), label_key),
                    *enabled,
                    move || action(),
                ));
            }
            if !menu_extra.is_empty() {
                items.push(ui::menu::divider());
            }
            let action_cmd = menu_cmd.clone();
            let action_key = menu_key.clone();
            let group_name = i18n::t(language.get().as_str(), title_key);
            let label_key = if collapsed {
                "sidebar.expandGroup"
            } else {
                "sidebar.collapseGroup"
            };
            items.push(ui::menu::action(
                i18n::with_arguments(language.get().as_str(), label_key, &[("group", group_name)]),
                true,
                move || action_cmd.toggle_sidebar_fold(tab_id, action_key.clone()),
            ));
            ui::menu::popup_at(items, event.page_position);
        })
        .child(header);
    if !collapsed {
        content = content.child(rows);
    }
    return content.into_element();
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BranchNode {
    segment: String,
    prefix: String,
    branch: Option<git::Branch>,
    children: Vec<BranchNode>,
}

fn build_branch_tree(branches: &[git::Branch]) -> Vec<BranchNode> {
    let mut roots = Vec::new();
    for branch in branches {
        let segments = branch.name.split('/').collect::<Vec<_>>();
        insert_branch(&mut roots, &segments, branch, "");
    }
    sort_branch_nodes(&mut roots);
    return roots;
}

fn insert_branch(
    nodes: &mut Vec<BranchNode>,
    segments: &[&str],
    branch: &git::Branch,
    parent: &str,
) {
    let Some(segment) = segments.first() else {
        return;
    };
    let prefix = if parent.is_empty() {
        (*segment).to_string()
    } else {
        format!("{parent}/{segment}")
    };
    let index = nodes
        .iter()
        .position(|node| node.segment == *segment)
        .unwrap_or_else(|| {
            nodes.push(BranchNode {
                segment: (*segment).to_string(),
                prefix: prefix.clone(),
                branch: None,
                children: Vec::new(),
            });
            nodes.len() - 1
        });
    let node = &mut nodes[index];
    if segments.len() == 1 {
        node.branch = Some(branch.clone());
    } else {
        insert_branch(&mut node.children, &segments[1..], branch, prefix.as_str());
    }
}

fn sort_branch_nodes(nodes: &mut [BranchNode]) {
    nodes.sort_by(|left, right| {
        left.branch
            .is_some()
            .cmp(&right.branch.is_some())
            .then_with(|| {
                left.segment
                    .to_lowercase()
                    .cmp(&right.segment.to_lowercase())
            })
            .then_with(|| left.segment.cmp(&right.segment))
    });
    for node in nodes {
        sort_branch_nodes(&mut node.children);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RemoteBranchNode {
    segment: String,
    prefix: String,
    branch: Option<git::RemoteBranch>,
    children: Vec<RemoteBranchNode>,
}

fn remote_node_rows(
    remotes: &[git::RemoteInfo],
    branches: &[git::RemoteBranch],
    language: Signal<SharedString>,
    cmd: &RefsCommands,
    tab_id: u64,
    current_branch: &str,
    can_rebase: bool,
    collapsed: &std::collections::BTreeSet<String>,
) -> Vec<AnyElement> {
    let mut remotes = remotes.to_vec();
    remotes.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.name.cmp(&right.name))
    });
    let mut rows = Vec::new();
    for remote in remotes {
        let remote_branches = branches
            .iter()
            .filter(|branch| branch.remote == remote.name)
            .cloned()
            .collect::<Vec<_>>();
        let key = format!("remote:{}", remote.name);
        let is_collapsed = collapsed.contains(key.as_str());
        let mut children = Vec::new();
        let tree = build_remote_branch_tree(&remote_branches);
        children.extend(remote_branch_tree_rows(
            &tree,
            &remote.name,
            language,
            cmd,
            tab_id,
            current_branch,
            can_rebase,
            collapsed,
            1,
        ));
        rows.push(remote_row(
            language,
            cmd,
            tab_id,
            &remote,
            remote_branches.len(),
            is_collapsed,
            children,
        ));
    }
    return rows;
}

fn remote_row(
    language: Signal<SharedString>,
    cmd: &RefsCommands,
    tab_id: u64,
    remote: &git::RemoteInfo,
    branch_count: usize,
    collapsed: bool,
    children: Vec<AnyElement>,
) -> AnyElement {
    let name = remote.name.clone();
    let fetch_url = remote.fetch_urls.first().cloned().unwrap_or_default();
    let fold_key = format!("remote:{name}");
    let click_cmd = cmd.clone();
    let click_key = fold_key.clone();
    let menu_cmd = cmd.clone();
    let menu_name = name.clone();
    let menu_url = fetch_url.clone();
    let menu_row = view()
        .w_full()
        .pl(px(7.0))
        .on_context_menu(move |event: &nsh_core::ContextMenuEvent| {
            let mut items = Vec::new();
            let sync = menu_cmd.sync.clone();
            let sync_name = menu_name.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "remote.fetch"),
                true,
                move || {
                    sync.open_sync_for_remote(
                        tab_id,
                        crate::app::session::Sync::Fetch,
                        sync_name.clone(),
                    )
                },
            ));
            let search = menu_cmd.search.clone();
            let edit_name = menu_name.clone();
            let edit_url = menu_url.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "remote.editUrl"),
                true,
                move || {
                    search.open_form_prefilled_from_tab(
                        tab_id,
                        crate::features::search::FormKind::EditRemote,
                        edit_url.clone(),
                        edit_name.clone(),
                    )
                },
            ));
            items.push(ui::menu::divider());
            let copy_notice = menu_cmd.notice;
            let copy_name = menu_name.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "remote.copyName"),
                true,
                move || crate::shared::os::copy_text(copy_notice, language, copy_name.clone()),
            ));
            let copy_notice = menu_cmd.notice;
            let copy_url = menu_url.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "remote.copyUrl"),
                !copy_url.is_empty(),
                move || crate::shared::os::copy_text(copy_notice, language, copy_url.clone()),
            ));
            items.push(ui::menu::divider());
            let drop_cmd = menu_cmd.clone();
            let drop_name = menu_name.clone();
            items.push(ui::menu::danger(
                i18n::t(language.get().as_str(), "remote.remove"),
                true,
                move || drop_cmd.drop_remote_at(tab_id, drop_name.clone()),
            ));
            ui::menu::popup_at(items, event.page_position);
        })
        .child(crate::ui::row::row_sized(
            move || name.clone(),
            None,
            Some(
                crate::ui::icon::icon_view(
                    if collapsed { "chev-right" } else { "chev-down" },
                    crate::ui::icon::IconSize::Compact,
                    design::text3,
                )
                .into_element(),
            ),
            vec![
                crate::ui::icon::icon_view(
                    "globe",
                    crate::ui::icon::IconSize::Compact,
                    design::text3,
                )
                .into_element(),
                text(branch_count.to_string())
                    .text_size(px(crate::ui::SECONDARY))
                    .text_color(move || design::text3())
                    .into_element(),
            ],
            false,
            false,
            metrics::SIDE_ROW_H,
            move || click_cmd.toggle_sidebar_fold(tab_id, click_key.clone()),
        ));
    let mut row = view().w_full().flex_col().child(menu_row);
    if !collapsed {
        row = row.child(children);
    }
    return row.into_element();
}

fn build_remote_branch_tree(branches: &[git::RemoteBranch]) -> Vec<RemoteBranchNode> {
    let mut roots = Vec::new();
    for branch in branches {
        let segments = branch.name.split('/').collect::<Vec<_>>();
        insert_remote_branch(&mut roots, &segments, branch, "");
    }
    sort_remote_branch_nodes(&mut roots);
    return roots;
}

fn insert_remote_branch(
    nodes: &mut Vec<RemoteBranchNode>,
    segments: &[&str],
    branch: &git::RemoteBranch,
    parent: &str,
) {
    let Some(segment) = segments.first() else {
        return;
    };
    let prefix = if parent.is_empty() {
        (*segment).to_string()
    } else {
        format!("{parent}/{segment}")
    };
    let index = nodes
        .iter()
        .position(|node| node.segment == *segment)
        .unwrap_or_else(|| {
            nodes.push(RemoteBranchNode {
                segment: (*segment).to_string(),
                prefix: prefix.clone(),
                branch: None,
                children: Vec::new(),
            });
            nodes.len() - 1
        });
    let node = &mut nodes[index];
    if segments.len() == 1 {
        node.branch = Some(branch.clone());
    } else {
        insert_remote_branch(&mut node.children, &segments[1..], branch, prefix.as_str());
    }
}

fn sort_remote_branch_nodes(nodes: &mut [RemoteBranchNode]) {
    nodes.sort_by(|left, right| {
        left.branch
            .is_some()
            .cmp(&right.branch.is_some())
            .then_with(|| {
                left.segment
                    .to_lowercase()
                    .cmp(&right.segment.to_lowercase())
            })
            .then_with(|| left.segment.cmp(&right.segment))
            .then_with(|| left.prefix.cmp(&right.prefix))
    });
    for node in nodes {
        sort_remote_branch_nodes(&mut node.children);
    }
}

fn remote_branch_tree_rows(
    nodes: &[RemoteBranchNode],
    remote: &str,
    language: Signal<SharedString>,
    cmd: &RefsCommands,
    tab_id: u64,
    current_branch: &str,
    can_rebase: bool,
    collapsed: &std::collections::BTreeSet<String>,
    depth: usize,
) -> Vec<AnyElement> {
    let mut rows = Vec::new();
    for node in nodes {
        let key = format!("remote-prefix:{remote}:{}", node.prefix);
        let is_collapsed = collapsed.contains(key.as_str());
        if let Some(branch) = node.branch.as_ref() {
            rows.push(remote_branch_row(
                language,
                cmd,
                tab_id,
                branch,
                current_branch,
                can_rebase,
                !node.children.is_empty(),
                key.clone(),
                is_collapsed,
                remote_branch_descendant_fold_keys(remote, node),
                depth,
            ));
        } else {
            rows.push(remote_branch_folder_row(
                language, cmd, tab_id, remote, node, depth, collapsed,
            ));
        }
        if !node.children.is_empty() && !is_collapsed {
            rows.extend(remote_branch_tree_rows(
                &node.children,
                remote,
                language,
                cmd,
                tab_id,
                current_branch,
                can_rebase,
                collapsed,
                depth + 1,
            ));
        }
    }
    return rows;
}

fn remote_branch_folder_row(
    language: Signal<SharedString>,
    cmd: &RefsCommands,
    tab_id: u64,
    remote: &str,
    node: &RemoteBranchNode,
    depth: usize,
    collapsed: &std::collections::BTreeSet<String>,
) -> AnyElement {
    let key = format!("remote-prefix:{remote}:{}", node.prefix);
    let is_collapsed = collapsed.contains(key.as_str());
    let click_cmd = cmd.clone();
    let click_key = key.clone();
    let menu_cmd = cmd.clone();
    let menu_key = key;
    let descendants = remote_branch_descendant_fold_keys(remote, node);
    let menu_descendants = descendants.clone();
    let label = node.segment.clone();
    let menu_label = label.clone();
    let row = view()
        .w_full()
        .pl(px(19.0 + depth as f32 * 12.0))
        .on_context_menu(move |event: &nsh_core::ContextMenuEvent| {
            let action_cmd = menu_cmd.clone();
            let action_key = menu_key.clone();
            let expand_label = if is_collapsed {
                "sidebar.expandGroup"
            } else {
                "sidebar.collapseGroup"
            };
            let mut items = vec![ui::menu::action(
                i18n::with_arguments(
                    language.get().as_str(),
                    expand_label,
                    &[("group", menu_label.clone())],
                ),
                true,
                move || {
                    action_cmd.set_sidebar_folds(tab_id, vec![action_key.clone()], !is_collapsed)
                },
            )];
            if !menu_descendants.is_empty() {
                items.push(ui::menu::divider());
                let expand_cmd = menu_cmd.clone();
                let expand_keys = menu_descendants.clone();
                items.push(ui::menu::action(
                    i18n::t(language.get().as_str(), "sidebar.expandAllDescendants"),
                    true,
                    move || expand_cmd.set_sidebar_folds(tab_id, expand_keys.clone(), false),
                ));
                let collapse_cmd = menu_cmd.clone();
                let collapse_keys = menu_descendants.clone();
                items.push(ui::menu::action(
                    i18n::t(language.get().as_str(), "sidebar.collapseAllDescendants"),
                    true,
                    move || collapse_cmd.set_sidebar_folds(tab_id, collapse_keys.clone(), true),
                ));
            }
            ui::menu::popup_at(items, event.page_position);
        })
        .child(crate::ui::row::row_sized(
            move || label.clone(),
            None,
            Some(
                crate::ui::icon::icon_view(
                    if is_collapsed {
                        "chev-right"
                    } else {
                        "chev-down"
                    },
                    crate::ui::icon::IconSize::Compact,
                    design::text3,
                )
                .into_element(),
            ),
            Vec::new(),
            false,
            false,
            metrics::SIDE_ROW_H,
            move || click_cmd.toggle_sidebar_fold(tab_id, click_key.clone()),
        ));
    return row.into_element();
}

fn remote_branch_descendant_fold_keys(remote: &str, node: &RemoteBranchNode) -> Vec<String> {
    let mut keys = Vec::new();
    for child in &node.children {
        if !child.children.is_empty() {
            keys.push(format!("remote-prefix:{remote}:{}", child.prefix));
        }
        keys.extend(remote_branch_descendant_fold_keys(remote, child));
    }
    return keys;
}

fn remote_branch_row(
    language: Signal<SharedString>,
    cmd: &RefsCommands,
    tab_id: u64,
    branch: &git::RemoteBranch,
    current_branch: &str,
    can_rebase: bool,
    has_children: bool,
    prefix_key: String,
    prefix_collapsed: bool,
    prefix_descendants: Vec<String>,
    depth: usize,
) -> AnyElement {
    let full_name = branch.full_name();
    let remote = branch.remote.clone();
    let branch_name = branch.name.clone();
    let menu_cmd = cmd.clone();
    let menu_full_name = full_name.clone();
    let menu_remote = remote.clone();
    let menu_branch = branch_name.clone();
    let menu_current = current_branch.to_string();
    let prefix_cmd = cmd.clone();
    let prefix_menu_key = prefix_key.clone();
    let prefix_menu_descendants = prefix_descendants.clone();
    let history_cmd = cmd.history.clone();
    let history_ref = format!("refs/remotes/{full_name}");
    let compare_ref = history_ref.clone();
    let label = branch_item_label(branch_name.as_str()).to_string();
    let row = view()
        .w_full()
        .pl(px(19.0 + depth as f32 * 12.0))
        .on_context_menu(move |event: &nsh_core::ContextMenuEvent| {
            let mut items = Vec::new();
            let search = menu_cmd.search.clone();
            let local_name = menu_branch.clone();
            let remote_name = menu_full_name.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "remote.checkoutAsNew"),
                true,
                move || {
                    search.open_tracking_branch_form(
                        tab_id,
                        local_name.clone(),
                        remote_name.clone(),
                    )
                },
            ));
            items.push(ui::menu::divider());
            let compare_history = menu_cmd.history.clone();
            let compare_base = format!("refs/heads/{}", menu_current);
            let compare_target = compare_ref.clone();
            let can_compare = !menu_current.is_empty();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "remote.compareCurrent"),
                can_compare,
                move || {
                    compare_history.set_compare_refs_at(
                        tab_id,
                        Some(compare_base.clone()),
                        Some(compare_target.clone()),
                    )
                },
            ));
            let merge_cmd = menu_cmd.clone();
            let merge_ref = menu_full_name.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "remote.mergeInto"),
                can_rebase,
                move || merge_cmd.merge_reference_at(tab_id, merge_ref.clone()),
            ));
            let rebase_cmd = menu_cmd.clone();
            let rebase_target = menu_full_name.clone();
            let rebase_current = menu_current.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "remote.rebaseCurrent"),
                can_rebase && !rebase_current.is_empty(),
                move || {
                    rebase_cmd.rebase_current_on(
                        tab_id,
                        rebase_current.clone(),
                        rebase_target.clone(),
                    )
                },
            ));
            let pull_cmd = menu_cmd.clone();
            let pull_remote = menu_remote.clone();
            let pull_branch = menu_branch.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "remote.pullIntoCurrent"),
                can_rebase,
                move || {
                    pull_cmd.pull_remote_branch_at(tab_id, pull_remote.clone(), pull_branch.clone())
                },
            ));
            items.push(ui::menu::divider());
            let delete_cmd = menu_cmd.clone();
            let delete_remote = menu_remote.clone();
            let delete_branch = menu_branch.clone();
            items.push(ui::menu::danger(
                i18n::t(language.get().as_str(), "remote.deleteBranch"),
                true,
                move || {
                    delete_cmd.delete_remote_branch_at(
                        tab_id,
                        delete_remote.clone(),
                        delete_branch.clone(),
                    )
                },
            ));
            let copy_notice = menu_cmd.notice;
            let copy_name = menu_full_name.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "remote.copyName"),
                true,
                move || crate::shared::os::copy_text(copy_notice, language, copy_name.clone()),
            ));
            if has_children {
                items.push(ui::menu::divider());
                let fold_cmd = prefix_cmd.clone();
                let fold_key = prefix_menu_key.clone();
                let label_key = if prefix_collapsed {
                    "sidebar.expandGroup"
                } else {
                    "sidebar.collapseGroup"
                };
                items.push(ui::menu::action(
                    i18n::with_arguments(
                        language.get().as_str(),
                        label_key,
                        &[("group", menu_branch.clone())],
                    ),
                    true,
                    move || {
                        fold_cmd.set_sidebar_folds(
                            tab_id,
                            vec![fold_key.clone()],
                            !prefix_collapsed,
                        )
                    },
                ));
                if !prefix_menu_descendants.is_empty() {
                    let expand_cmd = prefix_cmd.clone();
                    let expand_keys = prefix_menu_descendants.clone();
                    items.push(ui::menu::action(
                        i18n::t(language.get().as_str(), "sidebar.expandAllDescendants"),
                        true,
                        move || expand_cmd.set_sidebar_folds(tab_id, expand_keys.clone(), false),
                    ));
                    let collapse_cmd = prefix_cmd.clone();
                    let collapse_keys = prefix_menu_descendants.clone();
                    items.push(ui::menu::action(
                        i18n::t(language.get().as_str(), "sidebar.collapseAllDescendants"),
                        true,
                        move || collapse_cmd.set_sidebar_folds(tab_id, collapse_keys.clone(), true),
                    ));
                }
            }
            ui::menu::popup_at(items, event.page_position);
        })
        .child(crate::ui::row::row_sized(
            move || label.clone(),
            None,
            if has_children {
                Some(
                    crate::ui::icon::icon_view(
                        if prefix_collapsed {
                            "chev-right"
                        } else {
                            "chev-down"
                        },
                        crate::ui::icon::IconSize::Compact,
                        design::text3,
                    )
                    .into_element(),
                )
            } else {
                None
            },
            Vec::new(),
            false,
            false,
            metrics::SIDE_ROW_H,
            move || history_cmd.show_reference_at(tab_id, history_ref.clone()),
        ));
    return row.into_element();
}

fn branch_tree_rows(
    nodes: &[BranchNode],
    language: Signal<SharedString>,
    cmd: &RefsCommands,
    tab_id: u64,
    current_branch: &str,
    can_rebase: bool,
    has_origin: bool,
    collapsed: &std::collections::BTreeSet<String>,
    depth: usize,
) -> Vec<AnyElement> {
    let mut rows = Vec::new();
    for node in nodes {
        if let Some(branch) = node.branch.as_ref() {
            rows.push(branch_row(
                language,
                cmd,
                tab_id,
                branch,
                current_branch,
                can_rebase,
                has_origin,
                depth,
            ));
        } else {
            rows.push(branch_folder_row(
                language, cmd, tab_id, node, depth, collapsed,
            ));
        }
        let key = format!("branch-prefix:{}", node.prefix);
        if !node.children.is_empty() && !collapsed.contains(key.as_str()) {
            rows.extend(branch_tree_rows(
                &node.children,
                language,
                cmd,
                tab_id,
                current_branch,
                can_rebase,
                has_origin,
                collapsed,
                depth + 1,
            ));
        }
    }
    return rows;
}

fn branch_folder_row(
    language: Signal<SharedString>,
    cmd: &RefsCommands,
    tab_id: u64,
    node: &BranchNode,
    depth: usize,
    collapsed: &std::collections::BTreeSet<String>,
) -> AnyElement {
    let key = format!("branch-prefix:{}", node.prefix);
    let is_collapsed = collapsed.contains(key.as_str());
    let click_cmd = cmd.clone();
    let click_key = key.clone();
    let menu_cmd = cmd.clone();
    let menu_key = key;
    let descendants = branch_descendant_fold_keys(node);
    let menu_descendants = descendants.clone();
    let label = node.segment.clone();
    let menu_label = label.clone();
    let row = view()
        .w_full()
        .pl(px(7.0 + depth as f32 * 12.0))
        .on_context_menu(move |event: &nsh_core::ContextMenuEvent| {
            let action_cmd = menu_cmd.clone();
            let action_key = menu_key.clone();
            let expand_label = if is_collapsed {
                "sidebar.expandGroup"
            } else {
                "sidebar.collapseGroup"
            };
            let mut items = vec![ui::menu::action(
                i18n::with_arguments(
                    language.get().as_str(),
                    expand_label,
                    &[("group", menu_label.clone())],
                ),
                true,
                move || {
                    action_cmd.set_sidebar_folds(tab_id, vec![action_key.clone()], !is_collapsed)
                },
            )];
            if !menu_descendants.is_empty() {
                items.push(ui::menu::divider());
                let expand_cmd = menu_cmd.clone();
                let expand_keys = menu_descendants.clone();
                items.push(ui::menu::action(
                    i18n::t(language.get().as_str(), "sidebar.expandAllDescendants"),
                    true,
                    move || expand_cmd.set_sidebar_folds(tab_id, expand_keys.clone(), false),
                ));
                let collapse_cmd = menu_cmd.clone();
                let collapse_keys = menu_descendants.clone();
                items.push(ui::menu::action(
                    i18n::t(language.get().as_str(), "sidebar.collapseAllDescendants"),
                    true,
                    move || collapse_cmd.set_sidebar_folds(tab_id, collapse_keys.clone(), true),
                ));
            }
            ui::menu::popup_at(items, event.page_position);
        })
        .child(crate::ui::row::row_sized(
            move || label.clone(),
            None,
            Some(
                crate::ui::icon::icon_view(
                    if is_collapsed {
                        "chev-right"
                    } else {
                        "chev-down"
                    },
                    crate::ui::icon::IconSize::Compact,
                    design::text3,
                )
                .into_element(),
            ),
            Vec::new(),
            false,
            false,
            metrics::SIDE_ROW_H,
            move || click_cmd.toggle_sidebar_fold(tab_id, click_key.clone()),
        ));
    return row.into_element();
}

fn branch_descendant_fold_keys(node: &BranchNode) -> Vec<String> {
    let mut keys = Vec::new();
    for child in &node.children {
        if !child.children.is_empty() {
            keys.push(format!("branch-prefix:{}", child.prefix));
        }
        keys.extend(branch_descendant_fold_keys(child));
    }
    return keys;
}

fn branch_row(
    language: Signal<SharedString>,
    cmd: &RefsCommands,
    tab_id: u64,
    branch: &git::Branch,
    current_branch: &str,
    can_rebase: bool,
    has_origin: bool,
    depth: usize,
) -> AnyElement {
    let label = branch_item_label(branch.name.as_str()).to_string();
    let current = branch.current;
    let branch_checkout = branch.name.clone();
    let branch_new = branch.name.clone();
    let branch_rebase = branch.name.clone();
    let branch_merge = branch.name.clone();
    let branch_new_tag = branch.name.clone();
    let branch_rename = branch.name.clone();
    let branch_delete = branch.name.clone();
    let branch_copy = branch.name.clone();
    let branch_push = branch.name.clone();
    let history_cmd = cmd.history.clone();
    let history_ref = format!("refs/heads/{}", branch.name);
    let compare_ref = history_ref.clone();
    let can_compare = !current && !current_branch.is_empty();
    let menu_cmd = cmd.clone();
    let menu_current_branch = current_branch.to_string();
    let row = view()
        .w_full()
        .pl(px(7.0 + depth as f32 * 12.0))
        .on_context_menu(move |event: &nsh_core::ContextMenuEvent| {
            let mut items = Vec::with_capacity(12);
            let checkout_cmd = menu_cmd.clone();
            let checkout_name = branch_checkout.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "branch.menuCheckout"),
                !current,
                move || checkout_cmd.checkout_branch_at(tab_id, checkout_name.clone()),
            ));
            let new_search = menu_cmd.search.clone();
            let new_name = branch_new.clone();
            items.push(ui::menu::action(
                i18n::with_arguments(
                    language.get().as_str(),
                    "branch.newFrom",
                    &[("name", new_name.clone())],
                ),
                true,
                move || {
                    new_search.open_form_from_tab(
                        tab_id,
                        crate::features::search::FormKind::NewBranch,
                        new_name.clone(),
                    )
                },
            ));
            items.push(ui::menu::divider());
            let compare_history = menu_cmd.history.clone();
            let current_for_compare = format!("refs/heads/{}", menu_current_branch);
            let compare_name = compare_ref.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "branch.compareCurrent"),
                can_compare,
                move || {
                    compare_history.set_compare_refs_at(
                        tab_id,
                        Some(current_for_compare.clone()),
                        Some(compare_name.clone()),
                    )
                },
            ));
            let rebase_cmd = menu_cmd.clone();
            let current_for_rebase = menu_current_branch.clone();
            let rebase_name = branch_rebase.clone();
            items.push(ui::menu::action(
                i18n::with_arguments(
                    language.get().as_str(),
                    "branch.rebaseCurrentOnto",
                    &[("name", rebase_name.clone())],
                ),
                !current && can_rebase,
                move || {
                    rebase_cmd.rebase_current_on(
                        tab_id,
                        current_for_rebase.clone(),
                        rebase_name.clone(),
                    )
                },
            ));
            let merge_cmd = menu_cmd.clone();
            let merge_name = branch_merge.clone();
            items.push(ui::menu::action(
                i18n::with_arguments(
                    language.get().as_str(),
                    "branch.mergeNamedIntoCurrent",
                    &[("name", merge_name.clone())],
                ),
                !current,
                move || merge_cmd.merge_reference_at(tab_id, merge_name.clone()),
            ));
            items.push(ui::menu::divider());
            let new_tag_search = menu_cmd.search.clone();
            let new_tag_name = branch_new_tag.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "branch.newTag"),
                true,
                move || {
                    new_tag_search.open_form_from_tab(
                        tab_id,
                        crate::features::search::FormKind::NewTag,
                        new_tag_name.clone(),
                    )
                },
            ));
            let rename_search = menu_cmd.search.clone();
            let rename_name = branch_rename.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "branch.rename"),
                true,
                move || {
                    rename_search.open_form_from_tab(
                        tab_id,
                        crate::features::search::FormKind::RenameBranch,
                        rename_name.clone(),
                    )
                },
            ));
            let delete_cmd = menu_cmd.clone();
            let delete_name = branch_delete.clone();
            items.push(ui::menu::danger(
                i18n::t(language.get().as_str(), "branch.deleteMenu"),
                !current,
                move || delete_cmd.delete_branch_at(tab_id, delete_name.clone()),
            ));
            let copy_notice = menu_cmd.notice;
            let copy_name = branch_copy.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "branch.copyName"),
                true,
                move || crate::shared::os::copy_text(copy_notice, language, copy_name.clone()),
            ));
            let push_cmd = menu_cmd.clone();
            let push_name = branch_push.clone();
            items.push(ui::menu::action(
                i18n::with_arguments(
                    language.get().as_str(),
                    "branch.pushOrigin",
                    &[("name", push_name.clone())],
                ),
                has_origin,
                move || push_cmd.push_branch_at(tab_id, push_name.clone()),
            ));
            ui::menu::popup_at(items, event.page_position);
        })
        .child(crate::ui::row::row_sized(
            move || label.clone(),
            None,
            None,
            Vec::new(),
            current,
            false,
            metrics::SIDE_ROW_H,
            move || history_cmd.show_reference_at(tab_id, history_ref.clone()),
        ));
    return row.into_element();
}

/// 斜杠路径已由树节点表示，行内只显示当前层级的分支名称。
fn branch_item_label(name: &str) -> &str {
    return name.rsplit('/').next().unwrap_or(name);
}

fn worktree_row(
    language: Signal<SharedString>,
    cmd: &RefsCommands,
    tab_id: u64,
    worktree: git::WorktreeInfo,
) -> AnyElement {
    let path = worktree.path.clone();
    let name = short_path(path.as_str());
    let branch = worktree
        .branch
        .clone()
        .unwrap_or_else(|| i18n::t(language.get_untracked().as_str(), "worktree.detached"));
    let group_exists = worktree.exists && !worktree.prunable;
    let can_remove = worktree.exists
        && !worktree.main
        && !worktree.prunable
        && !worktree.locked
        && !worktree.dirty;
    let can_reveal = worktree.exists;
    let group_prune = cmd
        .session
        .tabs
        .get_untracked()
        .iter()
        .find(|tab| tab.id == tab_id)
        .is_some_and(|tab| {
            tab.refs
                .worktrees
                .iter()
                .any(|item| item.prunable && !item.locked)
        });
    let menu_cmd = cmd.clone();
    let menu_path = path.clone();
    let menu_new = cmd.clone();
    let menu_open = cmd.clone();
    let menu_remove = cmd.clone();
    let menu_prune = cmd.clone();
    let menu_notice = cmd.notice;
    let menu = move |event: &nsh_core::ContextMenuEvent| {
        let mut items = vec![ui::menu::action(
            i18n::t(language.get().as_str(), "worktree.openNewWindow"),
            group_exists,
            {
                let cmd = menu_open.clone();
                let path = menu_path.clone();
                move || cmd.open_worktree_window_at(tab_id, path.clone())
            },
        )];
        items.push(ui::menu::action(
            i18n::t(language.get().as_str(), "worktree.new"),
            true,
            {
                let cmd = menu_new.clone();
                move || cmd.new_worktree_form_at(tab_id)
            },
        ));
        items.push(ui::menu::divider());
        let is_open = menu_cmd.worktree_is_open(menu_path.as_str());
        items.push(ui::menu::danger(
            i18n::t(language.get().as_str(), "worktree.remove"),
            can_remove && !is_open,
            {
                let cmd = menu_remove.clone();
                let path = menu_path.clone();
                move || cmd.remove_worktree_at(tab_id, path.clone())
            },
        ));
        items.push(ui::menu::action(
            i18n::t(language.get().as_str(), "worktree.prune"),
            group_prune,
            {
                let cmd = menu_prune.clone();
                move || cmd.prune_worktrees_at(tab_id)
            },
        ));
        items.push(ui::menu::divider());
        items.push(ui::menu::action(
            i18n::t(language.get().as_str(), "menu.reveal"),
            can_reveal,
            {
                let notice = menu_notice;
                let path = menu_path.clone();
                move || crate::shared::os::reveal_path(notice, language, path.clone())
            },
        ));
        items.push(ui::menu::action(
            i18n::t(language.get().as_str(), "menu.copyPath"),
            true,
            {
                let notice = menu_notice;
                let path = menu_path.clone();
                move || crate::shared::os::copy_text(notice, language, path.clone())
            },
        ));
        ui::menu::popup_at(items, event.page_position);
    };
    let mut row = view()
        .w_full()
        .h(px(metrics::SIDE_ROW_H))
        .px(px(7.0))
        .flex_row()
        .items_center()
        .gap(px(7.0))
        .rounded(px(metrics::RADIUS_CONTROL))
        .hover(|style| style.bg(move || design::hover()))
        .on_context_menu(menu)
        .child(crate::ui::icon::icon_view(
            "folder",
            crate::ui::icon::IconSize::Compact,
            design::accent,
        ))
        .child(
            text(name)
                .text_size(px(crate::ui::BODY))
                .text_color(move || design::text2())
                .min_w_0()
                .flex_1()
                .number_of_lines(1)
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        );
    if !branch.is_empty() {
        row = row.child(
            text(branch)
                .text_size(px(crate::ui::SECONDARY))
                .text_color(move || design::text3())
                .max_w(px(86.0))
                .number_of_lines(1)
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        );
    }
    return row.into_element();
}

fn submodule_row(
    language: Signal<SharedString>,
    cmd: &RefsCommands,
    tab_id: u64,
    module: git::SubmoduleInfo,
) -> AnyElement {
    let path = module.path.clone();
    let row_path = path.clone();
    let status_key = if !module.initialized {
        "submodule.uninitialized"
    } else if module.dirty {
        "submodule.dirty"
    } else if module.current_oid == module.index_oid {
        "submodule.clean"
    } else {
        "submodule.changed"
    };
    let status_color = if !module.initialized {
        design::text3
    } else if module.dirty || module.current_oid != module.index_oid {
        design::warn
    } else {
        design::ok
    };
    let menu_cmd = cmd.clone();
    let menu_path = path.clone();
    let branch_configured = module.branch.is_some();
    let can_open = module.initialized;
    let can_commit = module.initialized && module.current_oid != module.index_oid;
    return view()
        .w_full()
        .h(px(metrics::SIDE_ROW_H))
        .px(px(7.0))
        .flex_row()
        .items_center()
        .gap(px(7.0))
        .rounded(px(metrics::RADIUS_CONTROL))
        .hover(|style| style.bg(move || design::hover()))
        .on_context_menu(move |event: &nsh_core::ContextMenuEvent| {
            let update_cmd = menu_cmd.clone();
            let update_path = menu_path.clone();
            let mut items = vec![ui::menu::action(
                i18n::t(language.get().as_str(), "submodule.update"),
                true,
                move || update_cmd.update_submodule_at(tab_id, update_path.clone(), false),
            )];
            let latest_cmd = menu_cmd.clone();
            let latest_path = menu_path.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "submodule.updateLatest"),
                branch_configured,
                move || latest_cmd.update_submodule_at(tab_id, latest_path.clone(), true),
            ));
            let sync_cmd = menu_cmd.clone();
            let sync_path = menu_path.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "submodule.syncUrl"),
                true,
                move || sync_cmd.sync_submodule_at(tab_id, sync_path.clone()),
            ));
            items.push(ui::menu::divider());
            let open_cmd = menu_cmd.clone();
            let open_path = menu_path.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "submodule.open"),
                can_open,
                move || open_cmd.open_submodule_at(tab_id, open_path.clone()),
            ));
            let commit_cmd = menu_cmd.clone();
            let commit_path = menu_path.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "submodule.commitChange"),
                can_commit,
                move || commit_cmd.stage_submodule_change_at(tab_id, commit_path.clone()),
            ));
            items.push(ui::menu::divider());
            let remove_cmd = menu_cmd.clone();
            let remove_path = menu_path.clone();
            items.push(ui::menu::danger(
                i18n::t(language.get().as_str(), "submodule.remove"),
                true,
                move || remove_cmd.remove_submodule_at(tab_id, remove_path.clone()),
            ));
            let copy_notice = menu_cmd.notice;
            let copy_path = menu_path.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "submodule.copyPath"),
                true,
                move || crate::shared::os::copy_text(copy_notice, language, copy_path.clone()),
            ));
            ui::menu::popup_at(items, event.page_position);
        })
        .child(crate::ui::icon::icon_view(
            "folder",
            crate::ui::icon::IconSize::Compact,
            design::text2,
        ))
        .child(
            text(row_path)
                .text_size(px(crate::ui::BODY))
                .text_color(move || design::text2())
                .min_w_0()
                .flex_1()
                .number_of_lines(1)
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        )
        .child(
            text(move || i18n::t(language.get().as_str(), status_key))
                .text_size(px(crate::ui::SECONDARY))
                .text_color(move || status_color())
                .number_of_lines(1)
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        )
        .into_element();
}

fn tag_row(
    language: Signal<SharedString>,
    cmd: &RefsCommands,
    tab_id: u64,
    name: String,
    remotes: Vec<String>,
) -> AnyElement {
    let menu_cmd = cmd.clone();
    let menu_name = name.clone();
    let menu_remotes = remotes.clone();
    let history_cmd = cmd.history.clone();
    let history_ref = format!("refs/tags/{name}");
    return view()
        .w_full()
        .on_context_menu(move |event: &nsh_core::ContextMenuEvent| {
            let push_items = menu_remotes.iter().map(|remote| {
                let push_cmd = menu_cmd.clone();
                let push_name = menu_name.clone();
                let push_remote = remote.clone();
                ui::menu::action(remote.clone(), true, move || {
                    push_cmd.push_tag_to_remote_at(tab_id, push_name.clone(), push_remote.clone())
                })
            });
            let mut items = vec![
                MenuItem::submenu(i18n::t(language.get().as_str(), "tag.pushTag"), push_items)
                    .disabled(menu_remotes.is_empty()),
            ];
            let copy_notice = menu_cmd.notice;
            let copy_name = menu_name.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "tag.copyName"),
                true,
                move || crate::shared::os::copy_text(copy_notice, language, copy_name.clone()),
            ));
            items.push(ui::menu::divider());
            let delete_cmd = menu_cmd.clone();
            let delete_name = menu_name.clone();
            items.push(ui::menu::danger(
                i18n::t(language.get().as_str(), "tag.deleteTag"),
                true,
                move || delete_cmd.drop_tag_at(tab_id, delete_name.clone()),
            ));
            ui::menu::popup_at(items, event.page_position);
        })
        .child(crate::ui::row::row_sized(
            move || name.clone(),
            None,
            None,
            vec![
                crate::ui::icon::icon_view(
                    "tag",
                    crate::ui::icon::IconSize::Compact,
                    design::tag_fg,
                )
                .into_element(),
            ],
            false,
            false,
            metrics::SIDE_ROW_H,
            move || history_cmd.show_reference_at(tab_id, history_ref.clone()),
        ))
        .into_element();
}

fn stash_row(
    language: Signal<SharedString>,
    cmd: &RefsCommands,
    tab_id: u64,
    stash: git::Stash,
) -> AnyElement {
    let reference = stash.reference.clone();
    let oid = stash.oid.clone();
    let name = stash.summary.clone();
    let row_name = name.clone();
    let menu_cmd = cmd.clone();
    let menu_oid = oid.clone();
    let menu_reference = reference.clone();
    let changed_paths = cmd
        .session
        .tabs
        .get_untracked()
        .iter()
        .find(|tab| tab.id == tab_id)
        .map(|tab| {
            tab.changes
                .iter()
                .map(|change| (change.path.clone(), change.untracked))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let can_stash = !changed_paths.is_empty();
    let menu_stash = cmd.search.clone();
    let menu_stash_selected = cmd.search.clone();
    let selected_paths = changed_paths;
    let history_cmd = cmd.history.clone();
    let history_sha = oid.clone();
    return view()
        .w_full()
        .on_context_menu(move |event: &nsh_core::ContextMenuEvent| {
            let stash_cmd = menu_stash.clone();
            let mut items = vec![ui::menu::action(
                i18n::t(language.get().as_str(), "stash.stashChanges"),
                can_stash,
                move || {
                    stash_cmd.open_form_from_tab(
                        tab_id,
                        crate::features::search::FormKind::NewStash,
                        String::new(),
                    )
                },
            )];
            let selected_cmd = menu_stash_selected.clone();
            let selected_paths = selected_paths.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "stash.stashSelectedFiles"),
                can_stash,
                move || selected_cmd.open_stash_selected_from_tab(tab_id, selected_paths.clone()),
            ));
            items.push(ui::menu::divider());
            let apply_cmd = menu_cmd.clone();
            let apply_oid = menu_oid.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "stash.apply"),
                true,
                move || apply_cmd.apply_stash_at(tab_id, apply_oid.clone()),
            ));
            let pop_cmd = menu_cmd.clone();
            let pop_oid = menu_oid.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "stash.pop"),
                true,
                move || pop_cmd.pop_stash_at(tab_id, pop_oid.clone()),
            ));
            items.push(ui::menu::divider());
            let drop_cmd = menu_cmd.clone();
            let drop_oid = menu_oid.clone();
            let drop_reference = menu_reference.clone();
            let drop_summary = name.clone();
            items.push(ui::menu::danger(
                i18n::t(language.get().as_str(), "stash.drop"),
                true,
                move || {
                    drop_cmd.drop_stash_at(
                        tab_id,
                        drop_oid.clone(),
                        drop_reference.clone(),
                        drop_summary.clone(),
                    )
                },
            ));
            items.push(ui::menu::divider());
            let diff_cmd = menu_cmd.clone();
            let diff_oid = menu_oid.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "stash.showDiff"),
                true,
                move || diff_cmd.show_stash_diff_at(tab_id, diff_oid.clone()),
            ));
            let copy_notice = menu_cmd.notice;
            let copy_reference = menu_reference.clone();
            items.push(ui::menu::action(
                i18n::t(language.get().as_str(), "stash.copyName"),
                true,
                move || crate::shared::os::copy_text(copy_notice, language, copy_reference.clone()),
            ));
            ui::menu::popup_at(items, event.page_position);
        })
        .child(crate::ui::row::row_sized(
            move || row_name.clone(),
            None,
            None,
            vec![
                crate::ui::icon::icon_view(
                    "archive",
                    crate::ui::icon::IconSize::Compact,
                    design::text2,
                )
                .into_element(),
            ],
            false,
            false,
            metrics::SIDE_ROW_H,
            move || history_cmd.show_commit_at(tab_id, history_sha.clone()),
        ))
        .into_element();
}

fn short_path(path: &str) -> String {
    return path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_string();
}

pub(crate) fn refs_view(
    state: &RefsState,
    cmd: &RefsCommands,
    language: Signal<SharedString>,
    tab: Tab,
    search: &SearchCommands,
    settings_state: Signal<store::Settings>,
) -> impl IntoElement + use<> {
    let now = store::now_unix();
    let reflog_query = state.reflog_query;
    let selected_tab = state.selected_tab;
    let reflog_selection = state.reflog_selection;
    // 表格在 `dynamic` 里重建，先克隆一份用例，闭包才拿得到 `'static` 数据。
    let table_cmd = cmd.clone();
    let table_search = search.clone();
    // 新建表单归搜索功能：这里只把入口包成按钮要的回调。
    let open_form: std::sync::Arc<dyn Fn(crate::features::search::FormKind)> = {
        let search = search.clone();
        std::sync::Arc::new(move |kind: crate::features::search::FormKind| search.open_form(kind))
    };
    return view()
        .w_full()
        .h_full()
        .flex_col()
        .bg(move || design::content())
        .child(
            view()
                .w_full()
                .h(px(metrics::HIST_BAR_H))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .px(px(12.0))
                .border_b_1()
                .border_color(move || design::line())
                .child(
                    text(move || i18n::t(language.get().as_str(), "refs.title"))
                        .text_size(px(crate::ui::TITLE))
                        .font_weight(FontWeight(600))
                        .line_height(px(16.0))
                        .text_color(move || design::text()),
                )
                .child(view().flex_1())
                .child(when(
                    move || selected_tab.get() == RefsTab::Refs,
                    move || {
                        let open_form = open_form.clone();
                        view()
                            .flex_row()
                            .items_center()
                            .gap(px(crate::ui::GAP))
                            .child(create_button(language, "form.newBranch", {
                                let open_form = open_form.clone();
                                move || open_form(crate::features::search::FormKind::NewBranch)
                            }))
                            .child(create_button(language, "form.newTag", {
                                let open_form = open_form.clone();
                                move || open_form(crate::features::search::FormKind::NewTag)
                            }))
                            .child(create_button(language, "form.addRemote", {
                                let open_form = open_form.clone();
                                move || open_form(crate::features::search::FormKind::AddRemote)
                            }))
                            .child(create_button(language, "form.newStash", {
                                let open_form = open_form.clone();
                                move || open_form(crate::features::search::FormKind::NewStash)
                            }))
                    },
                ))
                .child(tab_button(language, RefsTab::Refs, selected_tab))
                .child(tab_button(language, RefsTab::Reflog, selected_tab)),
        )
        .child(dynamic(move || -> AnyElement {
            match selected_tab.get() {
                RefsTab::Refs => {
                    refs_table(language, tab.clone(), &table_search, &table_cmd).into_element()
                }
                RefsTab::Reflog => {
                    // 「撤销」归恢复功能（`cmd.recovery`），「在历史中显示」归提交
                    // 历史（`cmd.history`）：refs 只是这两个入口的宿主。
                    reflog_list(
                        language,
                        tab.clone(),
                        now,
                        reflog_query,
                        reflog_selection,
                        &table_cmd,
                        &table_search,
                        settings_state,
                    )
                    .into_element()
                }
            }
        }));
}
fn tab_button(
    language: Signal<SharedString>,
    which: RefsTab,
    selected: Signal<RefsTab>,
) -> impl IntoElement {
    return dynamic(move || -> AnyElement {
        // 每个按钮单独订阅页签状态，切换时更新选中外观。
        let variant = if selected.get() == which {
            crate::ui::button::ButtonVariant::Secondary
        } else {
            crate::ui::button::ButtonVariant::Ghost
        };
        crate::ui::button::button(
            move || i18n::t(language.get().as_str(), which.key()),
            variant,
            crate::ui::button::ButtonSize::Small,
            false,
            move || selected.set(which),
        )
        .into_element()
    });
}
fn refs_table(
    language: Signal<SharedString>,
    tab: Tab,
    search: &SearchCommands,
    cmd: &RefsCommands,
) -> impl IntoElement {
    // 复制与在 Finder 中显示走 `shared::os`，表单入口走搜索功能；都包成回调供
    // 右键菜单使用。
    let notice = cmd.notice;
    let copy_text: std::sync::Arc<dyn Fn(String)> = {
        let language = language;
        std::sync::Arc::new(move |text: String| {
            crate::shared::os::copy_text(notice, language, text)
        })
    };
    let reveal_path: std::sync::Arc<dyn Fn(String)> = {
        let language = language;
        std::sync::Arc::new(move |path: String| {
            crate::shared::os::reveal_path(notice, language, path)
        })
    };
    let open_form_from: std::sync::Arc<dyn Fn(crate::features::search::FormKind, String)> = {
        let search = search.clone();
        std::sync::Arc::new(
            move |kind: crate::features::search::FormKind, from: String| {
                search.open_form_from(kind, from)
            },
        )
    };
    // 「比较」与各段引用行的写动作要 `Arc<dyn Fn>`，这里把用例包成回调。
    let compare_refs: std::sync::Arc<dyn Fn(Option<String>, Option<String>)> = {
        let history = cmd.history.clone();
        std::sync::Arc::new(move |base: Option<String>, target: Option<String>| {
            history.set_compare_refs(base, target)
        })
    };
    let current_compare_base = match &tab.head {
        git::HeadState::Branch(name) => Some(format!("refs/heads/{name}")),
        git::HeadState::Detached { .. } | git::HeadState::Unborn { .. } => None,
    };
    let checkout: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |name: String| cmd.checkout_branch(name))
    };
    let checkout_new: std::sync::Arc<dyn Fn(String, String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |name: String, from: String| cmd.checkout_new_branch(name, from))
    };
    let merge: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |name: String| cmd.merge_reference(name))
    };
    let delete: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |name: String| cmd.delete_branch(name))
    };
    let drop_remote: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |name: String| cmd.drop_remote(name))
    };
    let push_tag: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |name: String| cmd.push_tag(name))
    };
    let drop_tag: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |name: String| cmd.drop_tag(name))
    };
    let apply_stash: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |reference: String| cmd.apply_stash(reference))
    };
    let pop_stash: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |reference: String| cmd.pop_stash(reference))
    };
    let drop_stash: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |reference: String| cmd.drop_stash(reference))
    };
    // 远程行的「取回」打开同步面板：那是同步功能的入口，走它的用例。
    let fetch: std::sync::Arc<dyn Fn(crate::app::session::Sync)> = {
        let sync = cmd.sync.clone();
        std::sync::Arc::new(move |kind: crate::app::session::Sync| sync.open_sync(kind))
    };
    let mut sections: Vec<AnyElement> = Vec::new();
    // 分支：检出、检出为新本地分支、比较、合并、重命名、删除、复制。
    let mut branch_rows: Vec<AnyElement> = Vec::new();
    for (index, branch) in tab.refs.branches.iter().enumerate() {
        let name = branch.name.clone();
        let track = if branch.ahead == 0 && branch.behind == 0 {
            String::new()
        } else {
            format!("↑{} ↓{}", branch.ahead, branch.behind)
        };
        let note = if branch.current {
            i18n::t(language.get_untracked().as_str(), "shell.branch.current")
        } else {
            String::new()
        };
        let is_current = branch.current;
        let checkout = checkout.clone();
        let checkout_new = checkout_new.clone();
        let merge = merge.clone();
        let compare = compare_refs.clone();
        let compare_base = current_compare_base.clone();
        let open_form = open_form_from.clone();
        let delete = delete.clone();
        let copy_text = copy_text.clone();
        let name_for_menu = name.clone();
        let compare_target = format!("refs/heads/{name}");
        let can_open_compare = compare_base
            .as_deref()
            .is_some_and(|base| base != compare_target.as_str());
        let items = vec![
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "branch.checkout"),
                !is_current,
                {
                    let checkout = checkout.clone();
                    let name = name_for_menu.clone();
                    move || checkout(name.clone())
                },
            ),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "branch.checkoutAsNew"),
                true,
                {
                    let checkout_new = checkout_new.clone();
                    let name = name_for_menu.clone();
                    move || checkout_new(name.clone(), name.clone())
                },
            ),
            ui::menu::divider(),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.openCompare"),
                can_open_compare,
                {
                    let compare = compare.clone();
                    let target = compare_target;
                    let base = compare_base.clone();
                    move || compare(base.clone(), Some(target.clone()))
                },
            ),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "branch.mergeInto"),
                !is_current,
                {
                    let merge = merge.clone();
                    let name = name_for_menu.clone();
                    move || {
                        let _ = merge(name.clone());
                    }
                },
            ),
            ui::menu::divider(),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "branch.rename"),
                true,
                {
                    let open_form = open_form.clone();
                    let name = name_for_menu.clone();
                    move || {
                        open_form(
                            crate::features::search::FormKind::RenameBranch,
                            name.clone(),
                        )
                    }
                },
            ),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.copyName"),
                true,
                {
                    let copy_text = copy_text.clone();
                    let name = name_for_menu.clone();
                    move || copy_text(name.clone())
                },
            ),
            ui::menu::divider(),
            ui::menu::danger(
                i18n::t(language.get_untracked().as_str(), "branch.delete"),
                !is_current,
                {
                    let delete = delete.clone();
                    let name = name_for_menu;
                    move || delete(name.clone())
                },
            ),
        ];
        branch_rows.push(menu_row(
            language,
            name,
            note,
            track,
            design::branch_fg,
            items,
            index,
        ));
    }
    // 远程：编辑 URL、取回、移除、复制。
    let mut remote_rows: Vec<AnyElement> = Vec::new();
    for (index, remote) in tab.refs.remotes.iter().enumerate() {
        let name = remote.clone();
        let drop_remote = drop_remote.clone();
        let fetch = fetch.clone();
        let copy_text = copy_text.clone();
        let edit_form = open_form_from.clone();
        let url = tab
            .remote_urls
            .iter()
            .find(|(remote_name, _)| remote_name == &name)
            .map(|(_, url)| url.clone())
            .unwrap_or_default();
        let items = vec![
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.editRemoteUrl"),
                true,
                {
                    let edit_form = edit_form.clone();
                    let name = name.clone();
                    move || {
                        edit_form(crate::features::search::FormKind::EditRemote, name.clone());
                    }
                },
            ),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "sync.fetch"),
                true,
                {
                    let fetch = fetch.clone();
                    move || fetch(crate::app::session::Sync::Fetch)
                },
            ),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.copyUrl"),
                true,
                {
                    let copy_text = copy_text.clone();
                    let url = url.clone();
                    move || copy_text(url.clone())
                },
            ),
            ui::menu::divider(),
            ui::menu::danger(
                i18n::t(language.get_untracked().as_str(), "form.dropRemote"),
                true,
                {
                    let drop_remote = drop_remote.clone();
                    let name = name.clone();
                    move || drop_remote(name.clone())
                },
            ),
        ];
        remote_rows.push(menu_row(
            language,
            name,
            url,
            String::new(),
            design::text2,
            items,
            index,
        ));
    }
    // 标签：推送、删除、复制。
    let mut tag_rows: Vec<AnyElement> = Vec::new();
    for (index, tag) in tab.refs.tags.iter().enumerate() {
        let name = tag.clone();
        let push = push_tag.clone();
        let drop_tag = drop_tag.clone();
        let copy_text = copy_text.clone();
        let items = vec![
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.pushTag"),
                true,
                {
                    let push = push.clone();
                    let name = name.clone();
                    move || push(name.clone())
                },
            ),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.copyName"),
                true,
                {
                    let copy_text = copy_text.clone();
                    let name = name.clone();
                    move || copy_text(name.clone())
                },
            ),
            ui::menu::divider(),
            ui::menu::danger(
                i18n::t(language.get_untracked().as_str(), "form.dropTag"),
                true,
                {
                    let drop_tag = drop_tag.clone();
                    let name = name.clone();
                    move || drop_tag(name.clone())
                },
            ),
        ];
        tag_rows.push(menu_row(
            language,
            name,
            String::new(),
            String::new(),
            design::tag_fg,
            items,
            index,
        ));
    }
    // 贮藏：应用、弹出、丢弃、复制引用。
    let mut stash_rows: Vec<AnyElement> = Vec::new();
    for (index, stash) in tab.refs.stashes.iter().enumerate() {
        let reference = stash.reference.clone();
        let summary = stash.summary.clone();
        let apply = apply_stash.clone();
        let pop = pop_stash.clone();
        let drop_stash = drop_stash.clone();
        let copy_text = copy_text.clone();
        let items = vec![
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.stashApply"),
                true,
                {
                    let apply = apply.clone();
                    let reference = reference.clone();
                    move || apply(reference.clone())
                },
            ),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.stashPop"),
                true,
                {
                    let pop = pop.clone();
                    let reference = reference.clone();
                    move || pop(reference.clone())
                },
            ),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.copyName"),
                true,
                {
                    let copy_text = copy_text.clone();
                    let reference = reference.clone();
                    move || copy_text(reference.clone())
                },
            ),
            ui::menu::divider(),
            ui::menu::danger(
                i18n::t(language.get_untracked().as_str(), "form.dropStash"),
                true,
                {
                    let drop_stash = drop_stash.clone();
                    let reference = reference.clone();
                    move || drop_stash(reference.clone())
                },
            ),
        ];
        stash_rows.push(menu_row(
            language,
            reference,
            summary,
            String::new(),
            design::text2,
            items,
            index,
        ));
    }
    // 工作树：在 Finder 中显示、复制路径。
    let mut worktree_rows: Vec<AnyElement> = Vec::new();
    for (index, worktree) in tab.refs.worktrees.iter().enumerate() {
        let reveal = reveal_path.clone();
        let copy_text = copy_text.clone();
        let path_for_menu = worktree.path.clone();
        let items = vec![
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.reveal"),
                true,
                {
                    let reveal = reveal.clone();
                    let path = path_for_menu.clone();
                    move || reveal(path.clone())
                },
            ),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.copyPath"),
                true,
                {
                    let copy_text = copy_text.clone();
                    let path = path_for_menu.clone();
                    move || copy_text(path.clone())
                },
            ),
        ];
        worktree_rows.push(menu_row(
            language,
            short_path(worktree.path.as_str()),
            worktree.branch.clone().unwrap_or_default(),
            String::new(),
            design::text2,
            items,
            index,
        ));
    }
    sections.push(section(
        language,
        "sidebar.branches",
        tab.refs.branches.len(),
        branch_rows,
    ));
    sections.push(section(
        language,
        "sidebar.remotes",
        tab.refs.remotes.len(),
        remote_rows,
    ));
    sections.push(section(
        language,
        "sidebar.tags",
        tab.refs.tags.len(),
        tag_rows,
    ));
    sections.push(section(
        language,
        "sidebar.stashes",
        tab.refs.stashes.len(),
        stash_rows,
    ));
    sections.push(section(
        language,
        "sidebar.worktrees",
        tab.refs.worktrees.len(),
        worktree_rows,
    ));
    return view().w_full().h_full().child(
        scroll().w_full().h_full().child(
            view()
                .w_full()
                .py(px(8.0))
                .px(px(12.0))
                .flex_col()
                .child(
                    text(move || i18n::t(language.get().as_str(), "refs.writeHint"))
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(crate::ui::SECONDARY_LINE))
                        .text_color(move || design::text3()),
                )
                .child(sections),
        ),
    );
}
fn section(
    language: Signal<SharedString>,
    key: &'static str,
    count: usize,
    rows: Vec<AnyElement>,
) -> AnyElement {
    return view()
        .w_full()
        .mt(px(10.0))
        .flex_col()
        .child(
            view()
                .w_full()
                .h(px(24.0))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(move || i18n::t(language.get().as_str(), key))
                        .text_size(px(crate::ui::SECONDARY))
                        .font_weight(FontWeight(600))
                        .line_height(px(14.0))
                        .text_color(move || design::text2()),
                )
                .child(
                    text(count.to_string())
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(move || design::text3()),
                ),
        )
        .child(rows)
        .into_element();
}
fn reflog_list(
    language: Signal<SharedString>,
    tab: Tab,
    now: i64,
    query: Signal<String>,
    selected: Signal<Option<super::state::ReflogEntryKey>>,
    cmd: &RefsCommands,
    search: &SearchCommands,
    settings_state: Signal<store::Settings>,
) -> impl IntoElement {
    let recovery = cmd.recovery.clone();
    let show_in_history = cmd.history.clone();
    let notice = cmd.notice;
    let open_form_from: std::sync::Arc<dyn Fn(crate::features::search::FormKind, String)> = {
        let search = search.clone();
        std::sync::Arc::new(
            move |kind: crate::features::search::FormKind, from: String| {
                search.open_form_from(kind, from)
            },
        )
    };
    let copy_text_fn: std::sync::Arc<dyn Fn(String)> = {
        let language = language;
        std::sync::Arc::new(move |text: String| {
            crate::shared::os::copy_text(notice, language, text)
        })
    };
    let entries = tab.reflog.clone();
    let tab_id = tab.id;
    let path = tab.path.clone();
    let commands = cmd.clone();
    let mut body = view()
        .w_full()
        .h_full()
        .flex_col()
        .child(
            view()
                .w_full()
                .py(px(8.0))
                .px(px(12.0))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    text(move || i18n::t(language.get().as_str(), "reflog.hint"))
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(crate::ui::SECONDARY_LINE))
                        .text_color(move || design::text3()),
                )
                .child(view().flex_1())
                .child(
                    view()
                        .w(px(180.0))
                        .child(crate::ui::text_input::search_field(
                            query,
                            move || i18n::t(language.get().as_str(), "reflog.filterHint"),
                            None,
                            move |text| query.set(text.to_string()),
                        )),
                ),
        )
        .child(view().w_full().h(px(1.0)).bg(move || design::line()));
    body = body.child(dynamic(move || -> AnyElement {
        let needle = query.get().to_lowercase();
        let current_selection = selected.get();
        let visible = entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                needle.is_empty()
                    || entry.action.to_lowercase().contains(&needle)
                    || entry.summary.to_lowercase().contains(&needle)
                    || entry.short.to_lowercase().contains(&needle)
            })
            .map(|(index, entry)| (index, entry.clone()))
            .collect::<Vec<_>>();
        if visible.is_empty() {
            let key = if entries.is_empty() {
                "reflog.empty"
            } else {
                "reflog.noMatches"
            };
            return view()
                .w_full()
                .flex_1()
                .min_h_0()
                .items_center()
                .justify_center()
                .child(
                    text(move || i18n::t(language.get().as_str(), key))
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(move || design::text3()),
                )
                .into_element();
        }

        let selected_index = visible.iter().position(|(index, entry)| {
            current_selection
                .as_ref()
                .is_some_and(|key| key.matches_entry(tab_id, path.as_str(), *index, entry))
        });
        let rows = visible
            .iter()
            .map(|(_, entry)| vec![entry.sha.clone()])
            .collect::<Vec<_>>();
        let render_entries = visible.clone();
        let render_language = language;
        let render_recovery = recovery.clone();
        let render: Box<dyn Fn(u32, &str, String) -> AnyElement> = Box::new(move |row, _, _| {
            let Some((index, entry)) = render_entries.get(row as usize) else {
                return view().into_element();
            };
            let index = *index;
            let short = entry.short.clone();
            let action = entry.action.clone();
            let summary = entry.summary.clone();
            let timestamp = entry.timestamp;
            let undo = render_recovery.clone();
            let action_tooltip = ui::button::text_tooltip(action.clone());
            view()
                .w_full()
                .h(px(36.0))
                .px(px(8.0))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    text(short)
                        .w(px(64.0))
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(14.0))
                        .text_color(move || design::text3()),
                )
                .child(
                    view().w(px(128.0)).min_w_0().tooltip(action_tooltip).child(
                        text(action)
                            .w_full()
                            .text_size(px(crate::ui::SECONDARY))
                            .line_height(px(crate::ui::SECONDARY_LINE))
                            .text_color(move || design::accent())
                            .number_of_lines(2)
                            .text_overflow(nsh_core::EllipsizeMode::Tail),
                    ),
                )
                .child(
                    text(summary)
                        .flex_1()
                        .min_w_0()
                        .text_size(px(crate::ui::BODY))
                        .line_height(px(14.0))
                        .text_color(move || design::text())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .child(
                    text(move || {
                        crate::dates::format_reactive(
                            settings_state,
                            render_language.get().as_str(),
                            timestamp,
                            now,
                        )
                    })
                    .text_size(px(crate::ui::SECONDARY))
                    .text_color(move || design::text3()),
                )
                .child(crate::ui::button::button(
                    move || i18n::t(render_language.get().as_str(), "reflog.undo"),
                    crate::ui::button::ButtonVariant::Ghost,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || undo.undo_reflog(index),
                ))
                .into_element()
        });
        let select_entries = visible.clone();
        let select_commands = commands.clone();
        let select_path = path.clone();
        let on_select: Box<dyn Fn(usize)> = Box::new(move |row| {
            if let Some((index, entry)) = select_entries.get(row) {
                select_commands.select_reflog_entry(super::state::ReflogEntryKey::new(
                    tab_id,
                    select_path.clone(),
                    *index,
                    entry,
                ));
            }
        });
        // RowPress also fires for an already selected row, so a failed detail read can be retried.
        let press_entries = visible.clone();
        let press_commands = commands.clone();
        let press_path = path.clone();
        let on_row_press: Box<dyn Fn(u32, bool)> = Box::new(move |row, _| {
            if let Some((index, entry)) = press_entries.get(row as usize) {
                press_commands.select_reflog_entry(super::state::ReflogEntryKey::new(
                    tab_id,
                    press_path.clone(),
                    *index,
                    entry,
                ));
            }
        });
        let menu_entries = visible.clone();
        let menu_undo = recovery.clone();
        let menu_open_form = open_form_from.clone();
        let menu_show_history = show_in_history.clone();
        let menu_copy = copy_text_fn.clone();
        let menu_language = language;
        let on_context_menu: Box<dyn Fn(u32, f32, f32)> = Box::new(move |row, x, y| {
            let Some((index, entry)) = menu_entries.get(row as usize) else {
                return;
            };
            let index = *index;
            let lang = menu_language.get_untracked();
            let lang = lang.as_str();
            let sha = entry.sha.clone();
            let short = entry.short.clone();
            let items = vec![
                ui::menu::action(i18n::t(lang, "reflog.undo"), true, {
                    let undo = menu_undo.clone();
                    move || undo.undo_reflog(index)
                }),
                ui::menu::action(i18n::t(lang, "menu.newBranchFromHere"), true, {
                    let open_form = menu_open_form.clone();
                    let sha = sha.clone();
                    move || open_form(crate::features::search::FormKind::NewBranch, sha.clone())
                }),
                ui::menu::action(i18n::t(lang, "menu.showInHistory"), true, {
                    let show = menu_show_history.clone();
                    let sha = sha.clone();
                    move || show.show_in_history(sha.clone())
                }),
                ui::menu::divider(),
                ui::menu::action(i18n::t(lang, "menu.copySha"), true, {
                    let copy = menu_copy.clone();
                    let sha = sha.clone();
                    move || copy(sha.clone())
                }),
                ui::menu::action(i18n::t(lang, "menu.copyName"), true, {
                    let copy = menu_copy.clone();
                    move || copy(short.clone())
                }),
            ];
            crate::ui::menu::popup_at(items, nsh_core::geometry::Point::new(Points(x), Points(y)));
        });
        crate::ui::table::table(crate::ui::table::TableSpec {
            columns: vec![crate::ui::table::Column {
                key: "entry",
                title: None,
                width: None,
                fixed: true,
            }],
            rows,
            selected: selected_index,
            row_height: Some(28.0),
            on_select,
            render: Some(render),
            on_row_press: Some(on_row_press),
            on_context_menu: Some(on_context_menu),
        })
        .into_element()
    }));
    return body;
}
/// 创建入口按钮（工具条右侧）。
fn create_button(
    language: Signal<SharedString>,
    key: &'static str,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    return crate::ui::button::button(
        move || i18n::t(language.get().as_str(), key),
        crate::ui::button::ButtonVariant::Ghost,
        crate::ui::button::ButtonSize::Small,
        false,
        on_press,
    );
}
/// 行内的删除动作。
/// 一行带右键菜单的引用。`items` 由各分段自己组装，顺序即菜单顺序。
#[allow(clippy::too_many_arguments)]
fn menu_row(
    language: Signal<SharedString>,
    name: String,
    note: String,
    track: String,
    color: fn() -> AdaptiveColor,
    items: Vec<MenuItem>,
    _index: usize,
) -> AnyElement {
    return view()
        .w_full()
        .h(px(28.0))
        .px(px(6.0))
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .rounded(px(4.0))
        .hover(|style| style.bg(move || design::hover()))
        .on_context_menu(move |event: &nsh_core::ContextMenuEvent| {
            crate::ui::menu::popup_at(items.clone(), event.page_position);
        })
        .child(
            text(name)
                .w(px(220.0))
                .text_size(px(crate::ui::BODY))
                .line_height(px(15.0))
                .text_color(move || color())
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        )
        .child(
            text(note)
                .flex_1()
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3())
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        )
        .child(
            text(track)
                .text_size(px(crate::ui::SECONDARY))
                .text_color(move || design::text3()),
        )
        .child(
            text(move || i18n::t(language.get().as_str(), "common.menuHint"))
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        )
        .into_element();
}

#[cfg(test)]
mod sidebar_tests {
    use super::*;

    fn branch(name: &str) -> git::Branch {
        return git::Branch {
            name: name.to_string(),
            ..git::Branch::default()
        };
    }

    #[test]
    fn branch_tree_groups_slashes_and_sorts_each_level() {
        let tree = build_branch_tree(&[
            branch("zeta/fix"),
            branch("feature/z"),
            branch("Release/2.0"),
            branch("develop"),
            branch("alpha"),
            branch("feature/a"),
            branch("feature/deep/one"),
        ]);

        assert_eq!(
            tree.iter()
                .map(|node| node.segment.as_str())
                .collect::<Vec<_>>(),
            vec!["feature", "Release", "zeta", "alpha", "develop"]
        );
        let feature = tree.iter().find(|node| node.segment == "feature").unwrap();
        assert!(feature.branch.is_none());
        assert_eq!(
            feature
                .children
                .iter()
                .map(|node| node.segment.as_str())
                .collect::<Vec<_>>(),
            vec!["deep", "a", "z"]
        );
        let deep = feature
            .children
            .iter()
            .find(|node| node.segment == "deep")
            .unwrap();
        assert_eq!(deep.prefix, "feature/deep");
        assert!(deep.branch.is_none());
        assert_eq!(deep.children[0].prefix, "feature/deep/one");
    }

    #[test]
    fn branch_prefix_menu_collects_nested_folds() {
        let tree = build_branch_tree(&[
            branch("feature/deep/one"),
            branch("feature/nested/second/leaf"),
        ]);
        let feature = tree.iter().find(|node| node.segment == "feature").unwrap();
        assert_eq!(
            branch_descendant_fold_keys(feature),
            vec![
                "branch-prefix:feature/deep".to_string(),
                "branch-prefix:feature/nested".to_string(),
                "branch-prefix:feature/nested/second".to_string(),
            ]
        );
    }

    fn remote_branch(name: &str) -> git::RemoteBranch {
        return git::RemoteBranch {
            remote: "origin".to_string(),
            name: name.to_string(),
        };
    }

    #[test]
    fn remote_branch_tree_groups_slashes_and_sorts_each_level() {
        let tree = build_remote_branch_tree(&[
            remote_branch("zeta/fix"),
            remote_branch("feature/z"),
            remote_branch("Release/2.0"),
            remote_branch("develop"),
            remote_branch("alpha"),
            remote_branch("feature/a"),
            remote_branch("feature/deep/one"),
        ]);

        assert_eq!(
            tree.iter()
                .map(|node| node.segment.as_str())
                .collect::<Vec<_>>(),
            vec!["feature", "Release", "zeta", "alpha", "develop"]
        );
        let feature = tree.iter().find(|node| node.segment == "feature").unwrap();
        assert_eq!(
            feature
                .children
                .iter()
                .map(|node| node.segment.as_str())
                .collect::<Vec<_>>(),
            vec!["deep", "a", "z"]
        );
        assert_eq!(
            remote_branch_descendant_fold_keys("origin", feature),
            vec!["remote-prefix:origin:feature/deep".to_string()]
        );
    }

    #[test]
    fn remote_branch_that_is_also_a_prefix_keeps_its_children_foldable() {
        let tree =
            build_remote_branch_tree(&[remote_branch("feature"), remote_branch("feature/child")]);
        let feature = tree.first().expect("feature prefix");
        assert_eq!(
            feature.branch.as_ref().map(|branch| branch.full_name()),
            Some("origin/feature".to_string())
        );
        assert_eq!(feature.children[0].prefix, "feature/child");
    }
}
