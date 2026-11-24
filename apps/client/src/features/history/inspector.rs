//! 提交检查器（设计板 08 / 18）：底部提交信息、可滚动文件列表与固定高度的 Info 区。
//!
//! - 文件列表按目录展示提交改动（numstat，含 +/−），可滚动并提供路径动作；
//! - 提交头与精简元信息集中在底部 Info，可滚动、可选择，分支与引用列在其后；
//! - 单文件 Diff 顶部工具栏提供 Blame 入口，进入统一的 board 19 主区视图。
//!
//! 宽度由工作区分栏调整，初始宽度与 Commit 面板一致。入口 `inspector` 收本功能的状态与用例；
//! 复制与编辑器打开走 `shared::os` 的自由函数。

use super::commands::HistoryCommands;
use super::state::HistoryState;
use crate::app::session::Tab;
use crate::design;
use crate::i18n;
use nsh_core::prelude::*;

/// Info 固定在检查器底部，文件列表使用剩余高度。
const INFO_PANEL_H: f32 = 232.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileListMode {
    Tree,
    Flat,
}

/// 检查器不读筛选条件，`state` 只为与 `history` 统一入口签名；`use<>` 同 `history`，
/// 返回的元素不借用入参。
pub(crate) fn inspector(
    state: &HistoryState,
    cmd: &HistoryCommands,
    tab: Tab,
    language: Signal<SharedString>,
    file_list_mode: Signal<FileListMode>,
    tree: crate::app::session::TreeOptions,
) -> impl IntoElement + use<> {
    let file_tree_expanded = state.file_tree_expanded;
    let commit_info_loading = state.commit_info_loading;
    let fast_selection = state.fast_selection;
    let multi_commit_selection = state.multi_commit_selection;
    let multi_commit_data = state.multi_commit_data;
    // 视图回调要 `Arc<dyn Fn>`，这里把本功能的命令包一层。
    let select_file: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |id: String| cmd.select_tree_path(id))
    };
    let notice = cmd.notice;
    let tab_for_info = tab.clone();
    let tab_id = tab.id;
    let repo_path = tab.path.clone();
    let file_list_files = tab.files.clone();
    let selected_path = tab
        .selected_file
        .and_then(|index| tab.files.get(index))
        .map(|file| file.path.clone());
    let retry_multi_files: std::sync::Arc<dyn Fn()> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move || cmd.retry_multi_commit_files())
    };
    let file_history: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |path| cmd.show_file_history(tab_id, path))
    };
    let files = dynamic(move || -> AnyElement {
        let selection = multi_commit_selection
            .get()
            .filter(|selection| selection.tab_id == tab_id);
        if let Some(selection) = selection {
            let data = multi_commit_data
                .get()
                .filter(|data| data.tab_id == tab_id && data.shas == selection.shas);
            let (files, loading, failed) = data
                .map(|data| (data.files, data.loading, data.failed))
                .unwrap_or_else(|| (Vec::new(), true, false));
            return files_body(
                language,
                files,
                repo_path.clone(),
                None,
                std::sync::Arc::new(|_| {}),
                file_list_mode,
                tree,
                file_tree_expanded,
                notice,
                Some(selection.shas.len()),
                loading,
                failed,
                retry_multi_files.clone(),
                Some(file_history.clone()),
            );
        }
        let files_loading = commit_info_loading
            .get()
            .iter()
            .any(|(id, _)| *id == tab_id);
        return files_body(
            language,
            file_list_files.clone(),
            repo_path.clone(),
            selected_path.clone(),
            select_file.clone(),
            file_list_mode,
            tree,
            file_tree_expanded,
            notice,
            None,
            files_loading,
            false,
            std::sync::Arc::new(|| {}),
            Some(file_history.clone()),
        );
    })
    .into_element();
    let info = dynamic(move || -> AnyElement {
        let selection = multi_commit_selection
            .get()
            .filter(|selection| selection.tab_id == tab_for_info.id);
        let data = multi_commit_data.get().filter(|data| {
            selection.as_ref().is_some_and(|selection| {
                data.tab_id == tab_for_info.id && data.shas == selection.shas
            })
        });
        let loading =
            if selection.is_some() {
                data.as_ref().is_none_or(|data| data.loading)
            } else {
                tab_for_info.selected_commit().is_some_and(|commit| {
                    fast_selection.get().as_ref().is_some_and(|(tab_id, sha)| {
                        *tab_id == tab_for_info.id && sha == &commit.sha
                    }) || commit_info_loading
                        .get()
                        .contains(&(tab_for_info.id, commit.sha.clone()))
                })
            };
        let fields = info_fields(
            language,
            tab_for_info.clone(),
            selection.clone(),
            data,
            loading,
            notice,
        );
        return info_panel(fields);
    })
    .into_element();
    return inspector_shell(files, info);
}

/// Reflog 选中状态使用的简洁占位内容。
pub(crate) fn reflog_commit_placeholder(
    language: Signal<SharedString>,
    key: &'static str,
) -> impl IntoElement {
    return view()
        .w_full()
        .h_full()
        .items_center()
        .justify_center()
        .px(px(24.0))
        .child(
            text(move || i18n::t(language.get().as_str(), key))
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        );
}

/// Reflog 与 History 共用同一套文件列表和提交信息面板。
pub(crate) fn reflog_commit_inspector(
    language: Signal<SharedString>,
    details: git::CommitDetails,
    files: Vec<domain::DiffStat>,
    root_path: String,
    file_list_mode: Signal<FileListMode>,
    tree: crate::app::session::TreeOptions,
    file_tree_expanded: Signal<Option<Vec<SharedString>>>,
    notice: Signal<Option<String>>,
    file_history: std::sync::Arc<dyn Fn(String)>,
) -> impl IntoElement + use<> {
    let commit = domain::Commit {
        sha: details.sha.clone(),
        short: details.short.clone(),
        summary: if details.subject.is_empty() {
            "—".to_string()
        } else {
            details.subject.clone()
        },
        author: details.author.clone(),
        author_email: details.author_email.clone(),
        message: if details.body.is_empty() {
            details.subject.clone()
        } else {
            format!("{}\n\n{}", details.subject, details.body)
        },
        date: details.authored_at.to_string(),
        refs: details.refs.clone(),
        parents: details.parents.clone(),
        ..domain::Commit::default()
    };
    let file_content = files_body(
        language,
        files,
        root_path,
        None,
        std::sync::Arc::new(|_| {}),
        file_list_mode,
        tree,
        file_tree_expanded,
        notice,
        None,
        false,
        false,
        std::sync::Arc::new(|| {}),
        Some(file_history),
    );
    let info = commit_info_entry(language, commit, Some(details), &[], &[], notice, false);
    return inspector_shell(file_content, info_panel(info));
}

fn inspector_shell(files: AnyElement, info: AnyElement) -> AnyElement {
    return view()
        .w_full()
        .min_w_0()
        .h_full()
        .flex_col()
        .bg(move || design::panel())
        .child(view().w_full().flex_1().min_h_0().child(files))
        .child(info)
        .into_element();
}

fn info_panel(content: AnyElement) -> AnyElement {
    return view()
        .w_full()
        .h(px(INFO_PANEL_H))
        .min_h_0()
        .flex_shrink_0()
        .flex_col()
        .border_t_1()
        .border_color(move || design::line())
        .child(scroll().w_full().flex_1().min_h_0().child(content))
        .into_element();
}

fn files_body(
    language: Signal<SharedString>,
    files: Vec<domain::DiffStat>,
    root_path: String,
    selected_path: Option<String>,
    select_file: std::sync::Arc<dyn Fn(String)>,
    file_list_mode: Signal<FileListMode>,
    tree: crate::app::session::TreeOptions,
    file_tree_expanded: Signal<Option<Vec<SharedString>>>,
    notice: Signal<Option<String>>,
    multi_commit_count: Option<usize>,
    loading: bool,
    failed: bool,
    retry: std::sync::Arc<dyn Fn()>,
    file_history: Option<std::sync::Arc<dyn Fn(String)>>,
) -> AnyElement {
    let selected_id = selected_path
        .as_ref()
        .map(|path| SharedString::from(format!("file:{path}")));
    let (items, expanded) = crate::ui::tree::file_tree(
        files.iter().map(|file| file.path.clone()),
        tree.merge_empty_dirs,
        tree.compact_single_child,
    );
    let additions = files.iter().map(|file| file.additions).sum::<u32>();
    let deletions = files.iter().map(|file| file.deletions).sum::<u32>();
    let count = files.len();
    let all_directories = crate::ui::tree::folder_ids(&items);
    let initial_expanded = expanded.clone();
    let files_for_list = files.clone();
    return view()
        .w_full()
        .h_full()
        .min_h_0()
        .flex_col()
        .child(
            view()
                .w_full()
                .h(px(crate::ui::ROW))
                .pl(px(10.0))
                .pr(px(4.0))
                .flex_row()
                .items_center()
                .gap(px(crate::ui::GAP_SM))
                .border_b_1()
                .border_color(move || design::line_soft())
                .child(
                    text(move || {
                        if let Some(commits) = multi_commit_count {
                            return i18n::with_arguments(
                                language.get().as_str(),
                                "files.multiChangedCount",
                                &[
                                    ("commits", commits.to_string()),
                                    ("count", count.to_string()),
                                ],
                            );
                        }
                        i18n::with_arguments(
                            language.get().as_str(),
                            "files.changedCount",
                            &[("count", count.to_string())],
                        )
                    })
                    .flex_1()
                    .text_size(px(crate::ui::SECONDARY))
                    .text_color(move || design::text2()),
                )
                .child(
                    text(format!("+{additions}"))
                        .text_size(px(crate::ui::META))
                        .text_color(move || design::add_fg()),
                )
                .child(
                    text(format!("−{deletions}"))
                        .text_size(px(crate::ui::META))
                        .text_color(move || design::del_fg()),
                )
                .child(
                    view()
                        .ml(px(crate::ui::GAP_SM))
                        .flex_row()
                        .items_center()
                        .gap(px(0.0))
                        .child(dynamic(move || {
                            let all = all_directories.clone();
                            let current = file_tree_expanded
                                .get()
                                .unwrap_or_else(|| initial_expanded.clone());
                            let fully_expanded =
                                !all.is_empty() && all.iter().all(|id| current.contains(id));
                            let key = if fully_expanded {
                                "files.collapseAll"
                            } else {
                                "files.expandAll"
                            };
                            let name = if fully_expanded {
                                "collapse-all"
                            } else {
                                "expand-all"
                            };
                            crate::ui::button::icon_button_tip(
                                name,
                                {
                                    let language = language;
                                    move || i18n::t(language.get().as_str(), key)
                                },
                                crate::ui::button::ButtonSize::Small,
                                all.is_empty() || file_list_mode.get() != FileListMode::Tree,
                                {
                                    let language = language;
                                    move || i18n::t(language.get().as_str(), key)
                                },
                                move || {
                                    file_tree_expanded.set(Some(if fully_expanded {
                                        vec![]
                                    } else {
                                        all.clone()
                                    }))
                                },
                            )
                        }))
                        .child(dynamic(move || -> AnyElement {
                            let mode = file_list_mode.get();
                            crate::ui::button::file_list_toggle(
                                language,
                                mode == FileListMode::Tree,
                                move || {
                                    file_list_mode.set(if mode == FileListMode::Tree {
                                        FileListMode::Flat
                                    } else {
                                        FileListMode::Tree
                                    });
                                },
                            )
                            .into_element()
                        })),
                ),
        )
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .child(dynamic(move || -> AnyElement {
                    if loading || failed {
                        let message_key = if failed {
                            "files.multiFilesFailed"
                        } else {
                            "common.loading"
                        };
                        let mut message = view()
                            .w_full()
                            .h_full()
                            .items_center()
                            .justify_center()
                            .flex_col()
                            .gap(px(crate::ui::GAP_SM))
                            .child(
                                text(move || i18n::t(language.get().as_str(), message_key))
                                    .text_size(px(crate::ui::SECONDARY))
                                    .text_color(move || design::text3()),
                            );
                        if failed {
                            let retry = retry.clone();
                            message = message.child(crate::ui::button::button(
                                move || i18n::t(language.get().as_str(), "files.retry"),
                                crate::ui::button::ButtonVariant::Secondary,
                                crate::ui::button::ButtonSize::Small,
                                false,
                                move || retry(),
                            ));
                        }
                        return message.into_element();
                    }
                    if files_for_list.is_empty() {
                        return view()
                            .w_full()
                            .h_full()
                            .items_center()
                            .justify_center()
                            .child(
                                text(move || i18n::t(language.get().as_str(), "files.empty"))
                                    .text_size(px(crate::ui::SECONDARY))
                                    .text_color(move || design::text3()),
                            )
                            .into_element();
                    }
                    match file_list_mode.get() {
                        FileListMode::Tree => {
                            let files_for_render = files_for_list.clone();
                            crate::ui::tree::tree_with_renderer(
                                items.clone(),
                                expanded.clone(),
                                move || file_tree_expanded.get(),
                                selected_id.clone(),
                                {
                                    let select_file = select_file.clone();
                                    move |id| select_file(id.to_string())
                                },
                                move |expanded| file_tree_expanded.set(Some(expanded)),
                                {
                                    let root = root_path.clone();
                                    let history = file_history.clone();
                                    move |entry| {
                                        tree_file_row(
                                            entry,
                                            &files_for_render,
                                            &root,
                                            notice,
                                            language,
                                            history.clone(),
                                        )
                                    }
                                },
                            )
                            .into_element()
                        }
                        FileListMode::Flat => flat_file_list(
                            &files_for_list,
                            selected_id.clone(),
                            select_file.clone(),
                            root_path.clone(),
                            notice,
                            language,
                            file_history.clone(),
                        ),
                    }
                })),
        )
        .into_element();
}

fn tree_file_row(
    entry: &crate::ui::tree::TreeEntry,
    files: &[domain::DiffStat],
    root: &str,
    notice: Signal<Option<String>>,
    language: Signal<SharedString>,
    file_history: Option<std::sync::Arc<dyn Fn(String)>>,
) -> AnyElement {
    let directory = entry.folder;
    let id = entry.id.as_ref();
    let file = id
        .strip_prefix("file:")
        .and_then(|path| files.iter().find(|file| file.path == path));
    let depth = entry.depth as f32;
    let mut row = view()
        .test_id(&format!("commit-path:{id}"))
        .w_full()
        .h(px(crate::ui::ROW))
        .flex_row()
        .items_center()
        .gap(px(crate::ui::GAP_XS))
        .pl(px(4.0 + depth * 12.0))
        .pr(px(4.0));
    if directory {
        row = row.child(
            view()
                .app_class(Some(SharedString::from("group-aria-expanded:rotate-90")))
                .child(crate::ui::icon::icon_view(
                    "chev-right",
                    crate::ui::icon::IconSize::Compact,
                    design::text3,
                )),
        );
    } else {
        row = row.child(view().w(px(11.0)).h(px(11.0)));
    }
    let label = entry.label.to_string();
    let color = file
        .map(|file| status_color(file.status))
        .unwrap_or_else(design::text2);
    let label_view = text(label)
        .text_size(px(crate::ui::CODE))
        .text_color(move || color)
        .number_of_lines(1)
        .text_overflow(nsh_core::EllipsizeMode::Tail);
    row = row
        .child(crate::ui::icon::icon_view(
            if directory { "folder" } else { "file" },
            crate::ui::icon::IconSize::Button,
            design::text3,
        ))
        .child(
            view()
                .flex_1()
                .min_w_0()
                .child(label_view)
                .tooltip(crate::ui::button::text_tooltip(
                    id.strip_prefix("file:")
                        .unwrap_or(entry.label.as_ref())
                        .to_string(),
                )),
        );
    if let Some(file) = file {
        let color = status_color(file.status);
        row = row
            .child(
                text(format!("+{}", file.additions))
                    .flex_shrink_0()
                    .text_size(px(crate::ui::META))
                    .text_color(design::add_fg),
            )
            .child(
                text(format!("−{}", file.deletions))
                    .flex_shrink_0()
                    .text_size(px(crate::ui::META))
                    .text_color(design::del_fg),
            )
            .child(
                text(file.status.to_string())
                    .ml(px(crate::ui::GAP))
                    .w(px(14.0))
                    .flex_shrink_0()
                    .text_size(px(crate::ui::META))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(move || color),
            );
    }
    let relative = id
        .strip_prefix("file:")
        .or_else(|| id.strip_prefix("directory:"))
        .unwrap_or(id);
    let history_path = relative.to_string();
    let absolute = std::path::Path::new(root).join(relative);
    return row
        .on_context_menu(move |event: &nsh_core::ContextMenuEvent| {
            let path = absolute.to_string_lossy().into_owned();
            let reveal = path.clone();
            let history = file_history.clone();
            let history_path = history_path.clone();
            let copy = path.clone();
            crate::ui::menu::popup_at(
                vec![
                    crate::ui::menu::action(
                        i18n::t(language.get_untracked().as_str(), "files.revealInFinder"),
                        true,
                        move || {
                            crate::shared::os::reveal_path(notice, language, reveal.clone());
                        },
                    ),
                    crate::ui::menu::action(
                        i18n::t(language.get_untracked().as_str(), "files.copyPath"),
                        true,
                        move || crate::shared::os::copy_text(notice, language, copy.clone()),
                    ),
                    crate::ui::menu::action(
                        i18n::t(language.get_untracked().as_str(), "files.openInEditor"),
                        true,
                        move || crate::shared::os::open_editor(notice, language, path.clone()),
                    ),
                    crate::ui::menu::action(
                        i18n::t(language.get_untracked().as_str(), "shell.history"),
                        !directory && history.is_some(),
                        move || {
                            if let Some(history) = &history {
                                history(history_path.clone());
                            }
                        },
                    ),
                ],
                event.page_position,
            );
        })
        .into_element();
}

fn flat_file_list(
    files: &[domain::DiffStat],
    selected: Option<SharedString>,
    select_file: std::sync::Arc<dyn Fn(String)>,
    root: String,
    notice: Signal<Option<String>>,
    language: Signal<SharedString>,
    file_history: Option<std::sync::Arc<dyn Fn(String)>>,
) -> AnyElement {
    let items = files
        .iter()
        .map(|file| {
            crate::ui::tree::node(format!("file:{}", file.path), file.path.clone(), Vec::new())
        })
        .collect();
    let files = files.to_vec();
    crate::ui::tree::tree_with_renderer(
        items,
        vec![],
        || None,
        selected,
        move |id| select_file(id.to_string()),
        |_| {},
        move |entry| tree_file_row(entry, &files, &root, notice, language, file_history.clone()),
    )
    .into_element()
}

fn info_fields(
    language: Signal<SharedString>,
    tab: Tab,
    selection: Option<super::state::MultiCommitSelection>,
    data: Option<super::state::MultiCommitData>,
    loading: bool,
    notice: Signal<Option<String>>,
) -> AnyElement {
    let mut entries = Vec::new();
    let local_branches = &tab.refs.branches;
    let remote_branches = &tab.refs.remote_branches;
    if let Some(selection) = selection {
        let mut missing_details = false;
        for (index, sha) in selection.shas.iter().enumerate() {
            let Some(commit) = tab
                .commits
                .iter()
                .find(|commit| &commit.sha == sha)
                .cloned()
            else {
                continue;
            };
            let details = data
                .as_ref()
                .and_then(|data| data.shas.iter().position(|selected| selected == sha))
                .and_then(|index| data.as_ref()?.details.get(index).cloned().flatten());
            missing_details |= details.is_none();
            entries.push(commit_info_entry(
                language,
                commit,
                details,
                local_branches,
                remote_branches,
                notice,
                index + 1 < selection.shas.len(),
            ));
        }
        if loading {
            entries.push(info_status(language, "files.infoLoading"));
        } else if missing_details {
            entries.push(info_status(language, "files.infoUnavailable"));
        }
    } else if let Some(commit) = tab.selected_commit().cloned() {
        let details = tab
            .details
            .clone()
            .filter(|details| details.sha == commit.sha);
        let missing_details = details.is_none();
        entries.push(commit_info_entry(
            language,
            commit,
            details,
            local_branches,
            remote_branches,
            notice,
            false,
        ));
        if missing_details {
            let key = if loading {
                "files.infoLoading"
            } else {
                "files.infoUnavailable"
            };
            entries.push(info_status(language, key));
        }
    } else {
        return view()
            .w_full()
            .h_full()
            .items_center()
            .justify_center()
            .child(
                text(move || i18n::t(language.get().as_str(), "files.empty"))
                    .text_size(px(crate::ui::SECONDARY))
                    .text_color(move || design::text3()),
            )
            .into_element();
    }
    return view()
        .w_full()
        .flex_col()
        .selectable(true)
        .child(entries)
        .into_element();
}

fn commit_info_entry(
    language: Signal<SharedString>,
    commit: domain::Commit,
    details: Option<git::CommitDetails>,
    local_branches: &[git::Branch],
    remote_branches: &[git::RemoteBranch],
    notice: Signal<Option<String>>,
    show_separator: bool,
) -> AnyElement {
    let refs = details
        .as_ref()
        .map(|details| details.refs.as_slice())
        .unwrap_or(commit.refs.as_slice());
    let references = ordered_commit_refs(refs, &commit.branches, local_branches, remote_branches);
    let references = if references.is_empty() {
        "—".to_string()
    } else {
        references.join(", ")
    };
    let mut entry = view().w_full().flex_col().selectable(true);
    if show_separator {
        entry = entry.border_b_1().border_color(move || design::line());
    }
    entry = entry.child(commit_header(
        language,
        notice,
        commit,
        details
            .as_ref()
            .map(|details| details.body.clone())
            .unwrap_or_default(),
    ));
    if let Some(details) = details {
        if details.author != details.committer
            || details.author_email != details.committer_email
            || details.authored_at != details.committed_at
        {
            let same_identity = details.author == details.committer
                && details.author_email == details.committer_email;
            let identity = commit_identity(&details.committer, &details.committer_email);
            let committed_at = crate::dates::format(
                language.get_untracked().as_str(),
                details.committed_at,
                store::now_unix(),
            );
            let key = if same_identity {
                "files.info.committedOn"
            } else {
                "files.info.committedBy"
            };
            entry = entry.child(
                text(move || {
                    let arguments = [
                        ("identity", identity.clone()),
                        ("value", committed_at.clone()),
                    ];
                    i18n::with_arguments(language.get_untracked().as_str(), key, &arguments)
                })
                .w_full()
                .px(px(12.0))
                .py(px(3.0))
                .select_text()
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
            );
        }
    }
    entry = entry.child(info_ref_value(language, references));
    return entry.into_element();
}

fn commit_header(
    language: Signal<SharedString>,
    notice: Signal<Option<String>>,
    commit: domain::Commit,
    description: String,
) -> AnyElement {
    let initials = commit
        .author
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();
    let author_identity = commit_identity(&commit.author, &commit.author_email);
    let timestamp = commit.date.parse::<i64>().unwrap_or(0);
    let sha = commit.sha.clone();
    let short = commit.short.clone();
    return view()
        .w_full()
        .px(px(12.0))
        .pt(px(12.0))
        .pb(px(4.0))
        .flex_col()
        .gap(px(8.0))
        .child(
            text(commit.summary)
                .text_size(px(crate::ui::BODY))
                .font_weight(FontWeight(600))
                .line_height(px(crate::ui::BODY_LINE))
                .text_color(move || design::text())
                .number_of_lines(2)
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        )
        .child((!description.trim().is_empty()).then(|| {
            text(description)
                .w_full()
                .select_text()
                .text_size(px(crate::ui::BODY))
                .line_height(px(crate::ui::BODY_LINE))
                .text_color(design::text2)
                .into_element()
        }))
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .gap(px(crate::ui::GAP_SM))
                .child(
                    view()
                        .w(px(24.0))
                        .h(px(24.0))
                        .flex_shrink_0()
                        .items_center()
                        .justify_center()
                        .rounded(px(12.0))
                        .bg(move || design::token("avatar-bg"))
                        .child(
                            text(initials)
                                .text_size(px(crate::ui::META))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(move || design::token("avatar-fg")),
                        ),
                )
                .child(
                    view()
                        .flex_1()
                        .min_w_0()
                        .flex_col()
                        .child(
                            text(author_identity)
                                .text_size(px(crate::ui::SECONDARY))
                                .line_height(px(crate::ui::SECONDARY_LINE))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(move || design::text2())
                                .text_overflow(nsh_core::EllipsizeMode::Tail),
                        )
                        .child(
                            view()
                                .w_full()
                                .flex_row()
                                .items_center()
                                .gap(px(crate::ui::GAP_XS))
                                .child(
                                    text(move || {
                                        crate::dates::format(
                                            language.get().as_str(),
                                            timestamp,
                                            store::now_unix(),
                                        )
                                    })
                                    .flex_1()
                                    .min_w_0()
                                    .text_size(px(crate::ui::META))
                                    .line_height(px(crate::ui::SECONDARY_LINE))
                                    .text_color(move || design::text3()),
                                )
                                .child(crate::ui::button::button_icon_with_accessibility_label(
                                    move || short.clone(),
                                    {
                                        let language = language;
                                        move || i18n::t(language.get().as_str(), "files.copySha")
                                    },
                                    "copy",
                                    crate::ui::button::ButtonVariant::Ghost,
                                    crate::ui::button::ButtonSize::Small,
                                    false,
                                    move || {
                                        crate::shared::os::copy_text(notice, language, sha.clone())
                                    },
                                )),
                        ),
                ),
        )
        .into_element();
}

fn info_ref_value(language: Signal<SharedString>, refs: String) -> AnyElement {
    return view()
        .w_full()
        .px(px(12.0))
        .py(px(3.0))
        .child(
            text(move || {
                format!(
                    "{} {}",
                    i18n::t(language.get_untracked().as_str(), "files.info.refsInline"),
                    refs
                )
            })
            .w_full()
            .min_w_0()
            .select_text()
            .text_size(px(crate::ui::SECONDARY))
            .line_height(px(crate::ui::SECONDARY_LINE))
            .text_color(move || design::text()),
        )
        .into_element();
}

fn info_status(language: Signal<SharedString>, key: &'static str) -> AnyElement {
    return view()
        .w_full()
        .px(px(12.0))
        .py(px(4.0))
        .child(
            text(move || i18n::t(language.get().as_str(), key))
                .text_size(px(crate::ui::SECONDARY))
                .text_color(move || design::text3()),
        )
        .into_element();
}

fn commit_identity(name: &str, email: &str) -> String {
    if email.is_empty() {
        return name.to_string();
    }
    return format!("{name} <{email}>");
}

fn ordered_commit_refs(
    refs: &[String],
    branches_at_commit: &[String],
    local_branches: &[git::Branch],
    remote_branches: &[git::RemoteBranch],
) -> Vec<String> {
    let mut known_branches = local_branches
        .iter()
        .map(|branch| branch.name.clone())
        .collect::<std::collections::HashSet<_>>();
    known_branches.extend(remote_branches.iter().map(git::RemoteBranch::full_name));

    let current_branch = local_branches
        .iter()
        .find(|branch| branch.current)
        .map(|branch| branch.name.clone());
    let mut branches = std::collections::HashSet::<String>::new();
    let mut other_refs = Vec::new();
    let mut seen_other_refs = std::collections::HashSet::new();
    for reference in refs {
        let reference = reference.trim();
        if reference == "HEAD" {
            continue;
        }
        let normalized = reference
            .strip_prefix("HEAD -> ")
            .or_else(|| reference.strip_prefix("refs/heads/"))
            .or_else(|| reference.strip_prefix("refs/remotes/"))
            .unwrap_or(reference);
        if known_branches.contains(normalized) {
            branches.insert(normalized.to_string());
        } else if normalized != "HEAD" && seen_other_refs.insert(reference.to_string()) {
            other_refs.push(reference.to_string());
        }
    }
    branches.extend(
        branches_at_commit
            .iter()
            .filter(|branch| known_branches.contains(branch.as_str()))
            .cloned(),
    );

    let mut branches = branches.into_iter().collect::<Vec<_>>();
    branches.sort_by(|left, right| {
        let left_current = current_branch.as_deref() == Some(left.as_str());
        let right_current = current_branch.as_deref() == Some(right.as_str());
        right_current
            .cmp(&left_current)
            .then_with(|| left.cmp(right))
    });
    branches.extend(other_refs);
    branches.dedup();
    return branches;
}

fn status_color(status: char) -> AdaptiveColor {
    match status {
        'A' => design::add_fg(),
        'D' => design::del_fg(),
        _ => design::warn(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[component]
    fn FileTreeFixture() -> impl IntoElement {
        file_list_fixture(FileListMode::Tree)
    }

    #[component]
    fn FlatFileFixture() -> impl IntoElement {
        file_list_fixture(FileListMode::Flat)
    }

    fn file_list_fixture(mode: FileListMode) -> AnyElement {
        let files = (0..40)
            .map(|index| domain::DiffStat {
                path: format!("dir{index:02}/file{index:02}.rs"),
                status: 'M',
                ..Default::default()
            })
            .collect();
        let language = signal(SharedString::from("en"));
        let notice = signal(None);
        files_body(
            language,
            files,
            "/repo".into(),
            None,
            std::sync::Arc::new(|_| {}),
            signal(mode),
            crate::app::session::TreeOptions::default(),
            signal(None),
            notice,
            None,
            false,
            false,
            std::sync::Arc::new(|| {}),
            Some(std::sync::Arc::new(move |path| {
                crate::shared::os::copy_text(notice, language, path)
            })),
        )
    }

    #[test]
    fn expanding_directory_keeps_file_tree_scroll_container() {
        let mut app = nsh_testing::Headless::start(nsh_engine::EngineConfig {
            viewport: size(px(400.0), px(350.0)),
            ..nsh_engine::EngineConfig::new(nsh_core::element::Root::of::<FileTreeFixture>())
        })
        .unwrap();
        app.first_render().unwrap();
        let scroll_id = app
            .screen()
            .layout_nodes()
            .into_iter()
            .find(|node| node.scroll.is_some())
            .unwrap()
            .id;
        app.scroll("dir00", px(300.0)).unwrap();
        let anchor_y = |app: &nsh_testing::Headless| {
            app.screen()
                .layout_nodes()
                .into_iter()
                .find(|node| {
                    node.text
                        .as_ref()
                        .is_some_and(|text| text.text.text == "dir07")
                })
                .unwrap()
                .bounds
                .origin
                .y
                .0
        };
        let before = anchor_y(&app);
        app.press("dir06").unwrap();
        assert!(
            app.screen()
                .layout_nodes()
                .iter()
                .any(|node| node.id == scroll_id),
            "展开目录不得重建文件树的滚动容器"
        );
        assert!(
            (anchor_y(&app) - (before - crate::ui::ROW)).abs() < 1.0,
            "折叠目录只移除对应子行，不重置滚动位置"
        );
        app.press("dir06").unwrap();
        assert!(
            (anchor_y(&app) - before).abs() < 1.0,
            "再次展开目录应恢复原有可见行位置"
        );
    }

    #[test]
    fn context_menu_copies_the_clicked_file_and_directory_paths() {
        let mut app = nsh_testing::Headless::start(nsh_engine::EngineConfig {
            viewport: size(px(400.0), px(350.0)),
            ..nsh_engine::EngineConfig::new(nsh_core::element::Root::of::<FileTreeFixture>())
        })
        .unwrap();
        app.first_render().unwrap();
        app.set_environment("macOS", |env| env.os = nsh_core::platform::Os::MacOs)
            .unwrap();
        app.right_click("id:commit-path:file:dir00/file00.rs")
            .unwrap();
        let labels = app
            .menu()
            .unwrap()
            .into_iter()
            .map(|item| item.label)
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            ["Reveal in Finder", "Copy Path", "Open in Editor", "History"]
        );
        app.choose_menu_item("Copy Path").unwrap();
        assert_eq!(app.screen().clipboard(), Some("/repo/dir00/file00.rs"));
        app.right_click("id:commit-path:file:dir00/file00.rs")
            .unwrap();
        app.choose_menu_item("History").unwrap();
        assert_eq!(
            app.screen().clipboard(),
            Some("dir00/file00.rs"),
            "History 接收右键文件的仓库相对路径"
        );
        app.right_click("id:commit-path:directory:dir01").unwrap();
        app.choose_menu_item("Copy Path").unwrap();
        assert_eq!(app.screen().clipboard(), Some("/repo/dir01"));
    }

    #[test]
    fn flat_file_context_menu_copies_the_full_path() {
        let mut config = nsh_engine::EngineConfig {
            viewport: size(px(400.0), px(350.0)),
            ..nsh_engine::EngineConfig::new(nsh_core::element::Root::of::<FlatFileFixture>())
        };
        config.environment.os = nsh_core::platform::Os::MacOs;
        let mut app = nsh_testing::Headless::start(config).unwrap();
        app.first_render().unwrap();
        app.right_click("id:commit-path:file:dir00/file00.rs")
            .unwrap();
        app.choose_menu_item("Copy Path").unwrap();
        assert_eq!(app.screen().clipboard(), Some("/repo/dir00/file00.rs"));
    }

    #[component]
    fn CommitDescriptionFixture() -> impl IntoElement {
        commit_info_entry(
            signal(SharedString::from("en")),
            domain::Commit {
                sha: "abc".into(),
                summary: "Commit message".into(),
                author: "Author".into(),
                ..Default::default()
            },
            Some(git::CommitDetails {
                sha: "abc".into(),
                body: "First description line\nSecond description line".into(),
                ..Default::default()
            }),
            &[],
            &[],
            signal(None),
            false,
        )
    }

    #[test]
    fn commit_description_is_visible_below_message() {
        let mut app = nsh_testing::Headless::start(nsh_engine::EngineConfig::new(
            nsh_core::element::Root::of::<CommitDescriptionFixture>(),
        ))
        .unwrap();
        app.first_render().unwrap();
        app.wait_for("First description line\nSecond description line")
            .unwrap();
        let nodes = app.screen().layout_nodes();
        let position = |value: &str| {
            nodes
                .iter()
                .find(|node| {
                    node.text
                        .as_ref()
                        .is_some_and(|text| text.text.text.contains(value))
                })
                .unwrap()
                .bounds
                .origin
                .y
        };
        assert!(position("First description line") > position("Commit message"));
        assert!(
            app.screen()
                .texts()
                .iter()
                .any(|text| text.contains("Second description line"))
        );
    }

    #[test]
    fn all_folders_toggle_keeps_the_tree_and_toggles_files() {
        let mut app = nsh_testing::Headless::start(nsh_engine::EngineConfig {
            viewport: size(px(400.0), px(350.0)),
            ..nsh_engine::EngineConfig::new(nsh_core::element::Root::of::<FileTreeFixture>())
        })
        .unwrap();
        app.first_render().unwrap();
        let scroll_id = app
            .screen()
            .layout_nodes()
            .into_iter()
            .find(|node| node.scroll.is_some())
            .unwrap()
            .id;
        app.wait_for("file00.rs").unwrap();
        app.press("Collapse All Folders").unwrap();
        app.wait_for_gone("file00.rs", std::time::Duration::from_secs(1))
            .unwrap();
        app.press("Expand All Folders").unwrap();
        app.wait_for("file00.rs").unwrap();
        assert!(
            app.screen()
                .layout_nodes()
                .iter()
                .any(|node| node.id == scroll_id)
        );
    }

    #[test]
    fn info_refs_show_all_branches_with_current_first() {
        let refs = vec![String::from("HEAD -> main"), String::from("tag: v2.0")];
        let branches_at_commit = vec![
            String::from("origin/main"),
            String::from("feature/history"),
            String::from("main"),
        ];
        let local = vec![
            git::Branch {
                name: String::from("feature/history"),
                current: false,
                ..Default::default()
            },
            git::Branch {
                name: String::from("main"),
                current: true,
                ..Default::default()
            },
        ];
        let remote = vec![git::RemoteBranch {
            remote: String::from("origin"),
            name: String::from("main"),
        }];

        assert_eq!(
            ordered_commit_refs(&refs, &branches_at_commit, &local, &remote),
            vec![
                String::from("main"),
                String::from("feature/history"),
                String::from("origin/main"),
                String::from("tag: v2.0"),
            ]
        );
    }

    #[test]
    fn info_refs_do_not_add_current_branch_when_it_does_not_point_to_commit() {
        let refs = vec![String::from("origin/release"), String::from("tag: v1.0")];
        let local = vec![git::Branch {
            name: String::from("main"),
            current: true,
            ..Default::default()
        }];
        let remote = vec![git::RemoteBranch {
            remote: String::from("origin"),
            name: String::from("release"),
        }];

        assert_eq!(
            ordered_commit_refs(&refs, &[], &local, &remote),
            vec![String::from("origin/release"), String::from("tag: v1.0")]
        );
    }

    #[test]
    fn commit_tree_obeys_directory_preferences() {
        let files = vec![domain::DiffStat {
            path: "src/git/deep/file.rs".into(),
            status: 'M',
            ..Default::default()
        }];
        let default = crate::app::session::TreeOptions::default();
        let (plain, _) = crate::ui::tree::file_tree(
            files.iter().map(|file| file.path.clone()),
            default.merge_empty_dirs,
            default.compact_single_child,
        );
        assert_eq!(plain[0].label.as_ref(), "src");
        let (compact, expanded) =
            crate::ui::tree::file_tree(files.iter().map(|file| file.path.clone()), false, true);
        assert_eq!(compact[0].label.as_ref(), "src/git/deep");
        assert_eq!(expanded[0].as_ref(), "directory:src/git/deep");
        let (merged, _) =
            crate::ui::tree::file_tree(files.iter().map(|file| file.path.clone()), true, false);
        assert_eq!(merged[0].id.as_ref(), "directory:src/git/deep");
        assert_eq!(
            merged[0].children[0].id.as_ref(),
            "file:src/git/deep/file.rs"
        );
    }
}
