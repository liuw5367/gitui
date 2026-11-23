//! 提交检查器（设计板 08 / 18）：底部提交信息、可滚动文件列表与固定高度的 Info 区。
//!
//! - 文件列表按目录展示提交改动（numstat，含 +/−），可滚动并提供路径动作；
//! - 提交头与精简元信息集中在底部 Info，可滚动、可选择，分支与引用列在其后；
//! - 单文件 Diff 顶部工具栏提供 Blame 入口，进入统一的 board 19 主区视图。
//!
//! 宽度由工作区分栏调整，初始宽度与 Commit 面板一致。入口 `inspector` 收本功能的状态与用例；
//! 拣选/回滚走 `cmd.recovery`，复制与编辑器打开走 `shared::os` 的自由函数。

use super::commands::HistoryCommands;
use super::state::HistoryState;
use crate::app::session::Tab;
use crate::design;
use crate::features::recovery::commands::RecoveryCommands;
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
    let commit = tab.selected_commit().cloned();
    let notice = cmd.notice;
    let sha = commit
        .as_ref()
        .map(|commit| commit.sha.clone())
        .unwrap_or_default();
    let has_commit = commit.is_some();
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
    let recovery = cmd.recovery.clone();
    return view()
        .w_full()
        .min_w_0()
        .h_full()
        .flex_col()
        .bg(move || design::panel())
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .child(dynamic(move || -> AnyElement {
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
                            String::new(),
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
                        );
                    }
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
                        false,
                        false,
                        std::sync::Arc::new(|| {}),
                    );
                })),
        )
        .child(dynamic(move || -> AnyElement {
            let selection = multi_commit_selection
                .get()
                .filter(|selection| selection.tab_id == tab_for_info.id);
            let data = multi_commit_data.get().filter(|data| {
                selection.as_ref().is_some_and(|selection| {
                    data.tab_id == tab_for_info.id && data.shas == selection.shas
                })
            });
            let loading = if selection.is_some() {
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
            let panel = view()
                .w_full()
                .h(px(INFO_PANEL_H))
                .min_h_0()
                .flex_shrink_0()
                .flex_col()
                .border_t_1()
                .border_color(move || design::line())
                .child(scroll().w_full().flex_1().min_h_0().child(fields));
            if selection.is_some() {
                return panel.into_element();
            }
            return panel
                .child(info_actions(
                    language,
                    recovery.clone(),
                    sha.clone(),
                    !has_commit,
                ))
                .into_element();
        }))
        .into_element();
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

/// 展示直接从 Reflog SHA 读取的只读提交快照，不依赖 History 当前选中项。
pub(crate) fn reflog_commit_inspector(
    language: Signal<SharedString>,
    details: git::CommitDetails,
    files: Vec<domain::DiffStat>,
    notice: Signal<Option<String>>,
) -> impl IntoElement + use<> {
    let author_identity = if details.author_email.is_empty() {
        details.author.clone()
    } else {
        format!("{} <{}>", details.author, details.author_email)
    };
    let short = details.short.clone();
    let sha = details.sha.clone();
    let summary = if details.subject.is_empty() {
        "—".to_string()
    } else {
        details.subject.clone()
    };
    let timestamp = details.authored_at;
    let now = store::now_unix();
    let count = files.len();
    let additions = files.iter().map(|file| file.additions).sum::<u32>();
    let deletions = files.iter().map(|file| file.deletions).sum::<u32>();
    let info_refs = ordered_commit_refs(&details.refs, &[], &[], &[]);
    let info_refs = if info_refs.is_empty() {
        "—".to_string()
    } else {
        info_refs.join(", ")
    };
    let info = info_values(language, details, info_refs);
    let mut file_rows = Vec::with_capacity(files.len());
    for file in files {
        let status = file.status;
        file_rows.push(
            view()
                .w_full()
                .h(px(crate::ui::ROW))
                .px(px(12.0))
                .flex_row()
                .items_center()
                .gap(px(crate::ui::GAP_SM))
                .border_b_1()
                .border_color(move || design::line_soft())
                .child(
                    text(status.to_string())
                        .w(px(18.0))
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(move || status_color(status)),
                )
                .child(
                    text(file.path)
                        .flex_1()
                        .min_w_0()
                        .select_text()
                        .text_size(px(crate::ui::CODE))
                        .text_color(move || design::text2())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .child(
                    text(format!("+{}", file.additions))
                        .text_size(px(crate::ui::META))
                        .text_color(move || design::add_fg()),
                )
                .child(
                    text(format!("−{}", file.deletions))
                        .text_size(px(crate::ui::META))
                        .text_color(move || design::del_fg()),
                )
                .into_element(),
        );
    }
    let files_body = if file_rows.is_empty() {
        view()
            .w_full()
            .h_full()
            .items_center()
            .justify_center()
            .child(
                text(move || i18n::t(language.get().as_str(), "files.empty"))
                    .text_size(px(crate::ui::SECONDARY))
                    .text_color(move || design::text3()),
            )
            .into_element()
    } else {
        view()
            .w_full()
            .flex_col()
            .children(file_rows)
            .into_element()
    };
    return view()
        .w_full()
        .h_full()
        .min_w_0()
        .flex_col()
        .bg(move || design::panel())
        .child(
            view()
                .w_full()
                .px(px(12.0))
                .py(px(12.0))
                .flex_col()
                .gap(px(8.0))
                .border_b_1()
                .border_color(move || design::line())
                .child(
                    text(summary)
                        .text_size(px(crate::ui::BODY))
                        .font_weight(FontWeight(600))
                        .line_height(px(crate::ui::BODY_LINE))
                        .text_color(move || design::text())
                        .number_of_lines(2)
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .child(
                    view()
                        .w_full()
                        .flex_row()
                        .items_center()
                        .gap(px(crate::ui::GAP_SM))
                        .child(
                            text(author_identity)
                                .flex_1()
                                .min_w_0()
                                .text_size(px(crate::ui::SECONDARY))
                                .line_height(px(crate::ui::SECONDARY_LINE))
                                .text_color(move || design::text2())
                                .text_overflow(nsh_core::EllipsizeMode::Tail),
                        )
                        .child(
                            text(move || {
                                crate::dates::format(language.get().as_str(), timestamp, now)
                            })
                            .text_size(px(crate::ui::META))
                            .line_height(px(crate::ui::SECONDARY_LINE))
                            .text_color(move || design::text3()),
                        )
                        .child(
                            text(short)
                                .text_size(px(crate::ui::CODE))
                                .text_color(move || design::text2()),
                        )
                        .child(crate::ui::button::icon_button(
                            "copy",
                            {
                                let language = language;
                                move || i18n::t(language.get().as_str(), "files.copySha")
                            },
                            crate::ui::button::ButtonSize::Small,
                            false,
                            {
                                let sha = sha.clone();
                                move || crate::shared::os::copy_text(notice, language, sha.clone())
                            },
                        )),
                ),
        )
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .flex_col()
                .child(
                    view()
                        .w_full()
                        .h(px(crate::ui::ROW))
                        .px(px(12.0))
                        .flex_row()
                        .items_center()
                        .gap(px(crate::ui::GAP_SM))
                        .border_b_1()
                        .border_color(move || design::line_soft())
                        .child(
                            text(move || {
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
                        ),
                )
                .child(scroll().w_full().flex_1().min_h_0().child(files_body)),
        )
        .child(
            view()
                .w_full()
                .h(px(INFO_PANEL_H))
                .min_h_0()
                .flex_shrink_0()
                .flex_col()
                .border_t_1()
                .border_color(move || design::line())
                .child(scroll().w_full().flex_1().min_h_0().child(info)),
        );
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
) -> AnyElement {
    let selected_id = selected_path
        .as_ref()
        .map(|path| SharedString::from(format!("file:{path}")));
    let (items, expanded) = file_tree(&files, tree);
    let additions = files.iter().map(|file| file.additions).sum::<u32>();
    let deletions = files.iter().map(|file| file.deletions).sum::<u32>();
    let count = files.len();
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
                .px(px(10.0))
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
                .child(crate::ui::button::icon_button_tip(
                    "copy",
                    {
                        let language = language;
                        move || i18n::t(language.get().as_str(), "files.copyPath")
                    },
                    crate::ui::button::ButtonSize::Small,
                    selected_path.is_none(),
                    {
                        let language = language;
                        move || i18n::t(language.get().as_str(), "files.copyPath")
                    },
                    {
                        let root = root_path.clone();
                        let relative = selected_path.clone();
                        move || {
                            if let Some(relative) = relative.clone() {
                                crate::shared::os::copy_text(
                                    notice,
                                    language,
                                    format!("{}/{}", root.trim_end_matches('/'), relative),
                                );
                            }
                        }
                    },
                ))
                .child(crate::ui::button::icon_button_tip(
                    "external",
                    {
                        let language = language;
                        move || i18n::t(language.get().as_str(), "files.openInEditor")
                    },
                    crate::ui::button::ButtonSize::Small,
                    selected_path.is_none(),
                    {
                        let language = language;
                        move || i18n::t(language.get().as_str(), "files.openInEditor")
                    },
                    {
                        let root = root_path.clone();
                        let relative = selected_path.clone();
                        move || {
                            if let Some(relative) = relative.clone() {
                                crate::shared::os::open_editor(
                                    notice,
                                    language,
                                    format!("{}/{}", root.trim_end_matches('/'), relative),
                                );
                            }
                        }
                    },
                ))
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
                            "files.multiFilesLoading"
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
                            let expanded_state = file_tree_expanded;
                            let default_expanded = expanded.clone();
                            crate::ui::tree::tree_with_renderer(
                                items.clone(),
                                expanded.clone(),
                                file_tree_expanded.get(),
                                selected_id.clone(),
                                {
                                    let select_file = select_file.clone();
                                    move |id| select_file(id.to_string())
                                },
                                move |expanded| file_tree_expanded.set(Some(expanded)),
                                move |entry| {
                                    let expanded = expanded_state
                                        .get()
                                        .unwrap_or_else(|| default_expanded.clone())
                                        .iter()
                                        .any(|id| id.as_ref() == entry.id.as_ref());
                                    tree_file_row(entry, &files_for_render, expanded)
                                },
                            )
                            .into_element()
                        }
                        FileListMode::Flat => flat_file_list(
                            &files_for_list,
                            selected_id.clone(),
                            select_file.clone(),
                        ),
                    }
                })),
        )
        .into_element();
}

#[derive(Default)]
struct FileDirectory {
    directories: std::collections::BTreeMap<String, FileDirectory>,
    files: Vec<domain::DiffStat>,
}

fn file_tree(
    files: &[domain::DiffStat],
    tree: crate::app::session::TreeOptions,
) -> (Vec<crate::ui::tree::TreeItem>, Vec<SharedString>) {
    let mut root = FileDirectory::default();
    for file in files {
        let mut segments = file.path.split('/').collect::<Vec<_>>();
        if segments.is_empty() {
            continue;
        }
        segments.pop();
        let mut directory = &mut root;
        for segment in segments {
            directory = directory
                .directories
                .entry(segment.to_string())
                .or_default();
        }
        directory.files.push(file.clone());
    }
    let items = directory_items(&root, "", tree);
    let expanded = items
        .iter()
        .filter(|item| !item.children.is_empty())
        .map(|item| item.id.clone())
        .collect();
    return (items, expanded);
}

fn directory_items(
    directory: &FileDirectory,
    parent: &str,
    tree: crate::app::session::TreeOptions,
) -> Vec<crate::ui::tree::TreeItem> {
    let mut items = Vec::new();
    for (name, child) in &directory.directories {
        let mut path = if parent.is_empty() {
            name.clone()
        } else {
            format!("{parent}/{name}")
        };
        let mut label = name.clone();
        let mut child = child;
        while tree.compact_single_child && child.files.is_empty() && child.directories.len() == 1 {
            let (name, next) = child.directories.iter().next().unwrap();
            path = format!("{path}/{name}");
            label = format!("{label}/{name}");
            child = next;
        }
        let children = directory_items(child, &path, tree);
        if tree.shows_directory(child.files.len()) {
            items.push(crate::ui::tree::node(
                format!("directory:{path}"),
                label,
                children,
            ));
        } else {
            items.extend(children);
        }
    }
    for file in &directory.files {
        let name = file.path.rsplit('/').next().unwrap_or(file.path.as_str());
        items.push(crate::ui::tree::node(
            format!("file:{}", file.path),
            name.to_string(),
            Vec::new(),
        ));
    }
    return items;
}

fn tree_file_row(
    entry: &crate::ui::tree::TreeEntry,
    files: &[domain::DiffStat],
    expanded: bool,
) -> AnyElement {
    let directory = entry.folder;
    let id = entry.id.as_ref();
    let file = id
        .strip_prefix("file:")
        .and_then(|path| files.iter().find(|file| file.path == path));
    let depth = entry.depth as f32;
    let mut row = view()
        .w_full()
        .h(px(crate::ui::ROW))
        .flex_row()
        .items_center()
        .gap(px(crate::ui::GAP_XS))
        .pl(px(6.0 + depth * 12.0))
        .pr(px(8.0));
    if directory {
        row = row.child(crate::ui::icon::icon_view(
            if expanded { "chev-down" } else { "chev-right" },
            crate::ui::icon::IconSize::Compact,
            design::text3,
        ));
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
                    .w(px(14.0))
                    .flex_shrink_0()
                    .text_size(px(crate::ui::META))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(move || color),
            );
    }
    return row.into_element();
}

fn flat_file_list(
    files: &[domain::DiffStat],
    selected: Option<SharedString>,
    select_file: std::sync::Arc<dyn Fn(String)>,
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
        None,
        selected,
        move |id| select_file(id.to_string()),
        |_| {},
        move |entry| tree_file_row(entry, &files, false),
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
        for sha in &selection.shas {
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
        .px(px(12.0))
        .py(px(8.0))
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
    let mut entry = view()
        .w_full()
        .flex_col()
        .selectable(true)
        .child(commit_header(language, notice, commit))
        .child(info_ref_value(language, references));
    if let Some(details) = details {
        if details.author != details.committer
            || details.author_email != details.committer_email
            || details.authored_at != details.committed_at
        {
            let identity = commit_identity(&details.committer, &details.committer_email);
            let date = crate::dates::format(
                language.get_untracked().as_str(),
                details.committed_at,
                store::now_unix(),
            );
            entry = entry.child(
                text(move || {
                    i18n::with_arguments(
                        language.get_untracked().as_str(),
                        "files.info.committedBy",
                        &[("identity", identity.clone()), ("date", date.clone())],
                    )
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
    return entry.into_element();
}

fn commit_header(
    language: Signal<SharedString>,
    notice: Signal<Option<String>>,
    commit: domain::Commit,
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
        .py(px(12.0))
        .flex_col()
        .gap(px(8.0))
        .border_b_1()
        .border_color(move || design::line())
        .child(
            text(commit.summary)
                .text_size(px(crate::ui::BODY))
                .font_weight(FontWeight(600))
                .line_height(px(crate::ui::BODY_LINE))
                .text_color(move || design::text())
                .number_of_lines(2)
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        )
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
                            text(move || {
                                crate::dates::format(
                                    language.get().as_str(),
                                    timestamp,
                                    store::now_unix(),
                                )
                            })
                            .text_size(px(crate::ui::META))
                            .line_height(px(crate::ui::SECONDARY_LINE))
                            .text_color(move || design::text3()),
                        ),
                )
                .child(
                    text(short)
                        .text_size(px(crate::ui::CODE))
                        .text_color(design::text2),
                )
                .child(crate::ui::button::icon_button(
                    "copy",
                    {
                        let language = language;
                        move || i18n::t(language.get().as_str(), "files.copySha")
                    },
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || crate::shared::os::copy_text(notice, language, sha.clone()),
                )),
        )
        .into_element();
}

fn info_ref_value(language: Signal<SharedString>, refs: String) -> AnyElement {
    return view()
        .w_full()
        .px(px(12.0))
        .py(px(3.0))
        .flex_col()
        .gap(px(1.0))
        .child(
            text(move || i18n::t(language.get_untracked().as_str(), "files.info.refs"))
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        )
        .child(
            text(refs)
                .w_full()
                .min_w_0()
                .select_text()
                .text_size(px(crate::ui::CODE))
                .line_height(px(14.0))
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

/// Reflog 检查器仍展示完整的只读提交快照；History 提交检查器使用精简字段。
fn info_values(
    language: Signal<SharedString>,
    details: git::CommitDetails,
    refs: String,
) -> AnyElement {
    let now = store::now_unix();
    let parents = if details.parents.is_empty() {
        "—".to_string()
    } else {
        details
            .parents
            .iter()
            .map(|parent| parent.chars().take(7).collect::<String>())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let fields = [
        ("files.info.refs", refs),
        ("files.info.sha", details.sha.clone()),
        ("files.info.tree", details.tree.chars().take(7).collect()),
        ("files.info.parents", parents),
        (
            "files.info.author",
            commit_identity(&details.author, &details.author_email),
        ),
        (
            "files.info.authored",
            crate::dates::format(language.get_untracked().as_str(), details.authored_at, now),
        ),
        (
            "files.info.committer",
            commit_identity(&details.committer, &details.committer_email),
        ),
        (
            "files.info.committed",
            crate::dates::format(language.get_untracked().as_str(), details.committed_at, now),
        ),
    ];
    let rows = fields
        .into_iter()
        .map(|(key, value)| {
            view()
                .w_full()
                .py(px(3.0))
                .flex_col()
                .gap(px(1.0))
                .child(
                    text(move || i18n::t(language.get_untracked().as_str(), key))
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(crate::ui::SECONDARY_LINE))
                        .text_color(move || design::text3()),
                )
                .child(
                    text(value)
                        .w_full()
                        .min_w_0()
                        .select_text()
                        .text_size(px(crate::ui::CODE))
                        .line_height(px(14.0))
                        .text_color(move || design::text()),
                )
                .into_element()
        })
        .collect::<Vec<_>>();
    return view()
        .w_full()
        .px(px(12.0))
        .py(px(8.0))
        .flex_col()
        .selectable(true)
        .child(rows)
        .into_element();
}

fn info_actions(
    language: Signal<SharedString>,
    recovery: RecoveryCommands,
    sha_for_actions: String,
    disabled: bool,
) -> AnyElement {
    return view()
        .w_full()
        .px(px(12.0))
        .py(px(6.0))
        .flex_row()
        .flex_shrink_0()
        .items_center()
        .gap(px(crate::ui::GAP_SM))
        .border_t_1()
        .border_color(move || design::line())
        .child(crate::ui::button::button(
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "files.cherryPick")
            },
            crate::ui::button::ButtonVariant::Secondary,
            crate::ui::button::ButtonSize::Small,
            disabled,
            {
                let cherry_pick = recovery.clone();
                let sha = sha_for_actions.clone();
                move || cherry_pick.cherry_pick(sha.clone())
            },
        ))
        .child(crate::ui::button::button(
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "files.revert")
            },
            crate::ui::button::ButtonVariant::Secondary,
            crate::ui::button::ButtonSize::Small,
            disabled,
            {
                let revert_commit = recovery.clone();
                let sha = sha_for_actions.clone();
                move || revert_commit.revert_commit(sha.clone())
            },
        ))
        .into_element();
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
        let (plain, _) = file_tree(&files, crate::app::session::TreeOptions::default());
        assert_eq!(plain[0].label.as_ref(), "src");
        let (compact, expanded) = file_tree(
            &files,
            crate::app::session::TreeOptions {
                compact_single_child: true,
                merge_empty_dirs: false,
            },
        );
        assert_eq!(compact[0].label.as_ref(), "src/git/deep");
        assert_eq!(expanded[0].as_ref(), "directory:src/git/deep");
        let (merged, _) = file_tree(
            &files,
            crate::app::session::TreeOptions {
                compact_single_child: false,
                merge_empty_dirs: true,
            },
        );
        assert_eq!(merged[0].id.as_ref(), "directory:src/git/deep");
        assert_eq!(
            merged[0].children[0].id.as_ref(),
            "file:src/git/deep/file.rs"
        );
    }
}
