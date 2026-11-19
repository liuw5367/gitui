//! 提交检查器（设计板 08 / 18）：Files / Info 两个页签；Blame 是独立主区视图。
//!
//! - Files：按目录展示提交改动的文件（numstat，含 +/−），选中文件以启用路径动作；
//! - Info：提交元信息（SHA、tree、父提交、作者/提交者与时间、引用）与动作入口
//!   （当前只有复制 SHA 之外的入口标注未实现）；
//! Blame 入口切换到统一的 board 19 主区视图，不在检查器内嵌一个静态预览。
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

/// 检查器的页签（设计板 08 / 18）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InspectorTab {
    Files,
    Info,
}

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
    selected_tab: Signal<InspectorTab>,
    file_list_mode: Signal<FileListMode>,
) -> impl IntoElement + use<> {
    let file_tree_expanded = state.file_tree_expanded;
    // 视图回调要 `Arc<dyn Fn>`，这里把本功能的命令包一层。
    let select_file: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |id: String| cmd.select_tree_path(id))
    };
    let commit = tab.selected_commit().cloned();
    let now = store::now_unix();
    let summary = commit
        .as_ref()
        .map(|commit| commit.summary.clone())
        .unwrap_or_default();
    let author = commit
        .as_ref()
        .map(|commit| commit.author.clone())
        .unwrap_or_default();
    let short = commit
        .as_ref()
        .map(|commit| commit.short.clone())
        .unwrap_or_default();
    let timestamp = commit
        .as_ref()
        .and_then(|commit| commit.date.parse::<i64>().ok())
        .unwrap_or(0);
    let tab_for_body = tab.clone();
    // 复制与外部打开的提示走本功能的用例（`shared::os` 的自由函数）。
    let notice = cmd.notice;
    // 拣选/回滚的用例先克隆一份：`dynamic` 的闭包不能借用 `cmd`。
    let recovery_for_body = cmd.recovery.clone();
    // 页脚两个按钮作用于当前选中的文件（相对仓库根）。
    let selected_path = tab
        .selected_file
        .and_then(|index| tab.files.get(index))
        .map(|file| file.path.clone());
    let sha = commit
        .as_ref()
        .map(|commit| commit.sha.clone())
        .unwrap_or_default();
    let has_commit = commit.is_some();
    let initials = author
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase();
    let additions = tab.files.iter().map(|file| file.additions).sum::<u32>();
    let deletions = tab.files.iter().map(|file| file.deletions).sum::<u32>();
    let file_count = tab.files.len();
    let open_blame = cmd.clone();
    return view()
        .w_full()
        .min_w_0()
        .h_full()
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
                                    text(author)
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
                                            now,
                                        )
                                    })
                                    .text_size(px(crate::ui::META))
                                    .line_height(px(crate::ui::SECONDARY_LINE))
                                    .text_color(move || design::text3()),
                                ),
                        )
                        .child(crate::ui::button::icon_button(
                            "copy",
                            {
                                let language = language;
                                move || i18n::t(language.get().as_str(), "files.copySha")
                            },
                            crate::ui::button::ButtonSize::Small,
                            !has_commit,
                            {
                                let sha = sha.clone();
                                move || crate::shared::os::copy_text(notice, language, sha.clone())
                            },
                        )),
                )
                .child(
                    view()
                        .w_full()
                        .h(px(crate::ui::ROW_COMPACT))
                        .px(px(crate::ui::GAP_SM))
                        .flex_row()
                        .items_center()
                        .gap(px(crate::ui::GAP_SM))
                        .rounded(px(5.0))
                        .bg(move || design::panel2())
                        .child(
                            text(move || {
                                i18n::with_arguments(
                                    language.get().as_str(),
                                    "files.atCommit",
                                    &[("short", short.clone())],
                                )
                            })
                            .text_size(px(crate::ui::CODE))
                            .line_height(px(crate::ui::ROW_COMPACT))
                            .text_color(move || design::text2()),
                        ),
                )
                .child(
                    view()
                        .w_full()
                        .flex_row()
                        .items_center()
                        .gap(px(crate::ui::GAP_SM))
                        .child(
                            text(move || {
                                i18n::with_arguments(
                                    language.get().as_str(),
                                    "files.changedCount",
                                    &[("count", file_count.to_string())],
                                )
                            })
                            .flex_1()
                            .text_size(px(crate::ui::SECONDARY))
                            .line_height(px(crate::ui::SECONDARY_LINE))
                            .font_weight(FontWeight::MEDIUM)
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
                ),
        )
        .child(dynamic(move || -> AnyElement {
            let open_blame = open_blame.clone();
            view()
                .w_full()
                .h(px(crate::ui::ROW))
                .px(px(crate::ui::GAP_LG))
                .child(crate::ui::tabs::tabs(
                    vec![
                        crate::ui::tabs::TabItem {
                            id: "files",
                            label: Box::new({
                                let language = language;
                                move || i18n::t(language.get().as_str(), "files.title")
                            }),
                        },
                        crate::ui::tabs::TabItem {
                            id: "info",
                            label: Box::new({
                                let language = language;
                                move || i18n::t(language.get().as_str(), "files.info")
                            }),
                        },
                        crate::ui::tabs::TabItem {
                            id: "blame",
                            label: Box::new({
                                let language = language;
                                move || i18n::t(language.get().as_str(), "files.blame")
                            }),
                        },
                    ],
                    match selected_tab.get() {
                        InspectorTab::Files => "files",
                        InspectorTab::Info => "info",
                    },
                    move |id| match id {
                        "info" => selected_tab.set(InspectorTab::Info),
                        "blame" => open_blame.open_blame(),
                        _ => selected_tab.set(InspectorTab::Files),
                    },
                ))
                .into_element()
        }))
        .child(dynamic(move || -> AnyElement {
            match selected_tab.get() {
                InspectorTab::Files => files_body(
                    language,
                    tab_for_body.clone(),
                    select_file.clone(),
                    file_list_mode,
                    file_tree_expanded,
                )
                .into_element(),
                InspectorTab::Info => info_body(
                    language,
                    tab_for_body.clone(),
                    notice,
                    recovery_for_body.clone(),
                )
                .into_element(),
            }
        }))
        .child(
            view()
                .w_full()
                .h(px(34.0))
                .px(px(12.0))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .border_t_1()
                .border_color(move || design::line())
                .child(crate::ui::button::button(
                    {
                        let language = language;
                        move || i18n::t(language.get().as_str(), "files.copyPath")
                    },
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Small,
                    selected_path.is_none(),
                    {
                        let root = tab.path.clone();
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
                .child(view().flex_1())
                .child(crate::ui::button::button(
                    {
                        let language = language;
                        move || i18n::t(language.get().as_str(), "files.openInEditor")
                    },
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Small,
                    selected_path.is_none(),
                    {
                        let root = tab.path.clone();
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
                )),
        );
}

fn files_body(
    language: Signal<SharedString>,
    tab: Tab,
    select_file: std::sync::Arc<dyn Fn(String)>,
    file_list_mode: Signal<FileListMode>,
    file_tree_expanded: Signal<Option<Vec<SharedString>>>,
) -> AnyElement {
    let files = tab.files.clone();
    let selected_id = tab
        .selected_file
        .and_then(|index| files.get(index))
        .map(|file| SharedString::from(format!("file:{}", file.path)));
    let (items, expanded) = file_tree(&files);
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
                .child(dynamic(move || -> AnyElement {
                    crate::ui::segmented::segmented(
                        vec![
                            crate::ui::segmented::Segment {
                                id: "tree",
                                label: Box::new({
                                    let language = language;
                                    move || i18n::t(language.get().as_str(), "changelists.tree")
                                }),
                            },
                            crate::ui::segmented::Segment {
                                id: "flat",
                                label: Box::new({
                                    let language = language;
                                    move || i18n::t(language.get().as_str(), "changelists.flat")
                                }),
                            },
                        ],
                        match file_list_mode.get() {
                            FileListMode::Tree => "tree",
                            FileListMode::Flat => "flat",
                        },
                        move |id| {
                            file_list_mode.set(if id == "flat" {
                                FileListMode::Flat
                            } else {
                                FileListMode::Tree
                            })
                        },
                    )
                    .into_element()
                })),
        )
        .child(dynamic(move || -> AnyElement {
            if files_for_list.is_empty() {
                return view()
                    .w_full()
                    .flex_1()
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
                                .unwrap_or_default()
                                .iter()
                                .any(|id| id.as_ref() == entry.id.as_ref());
                            tree_file_row(entry, &files_for_render, expanded)
                        },
                    )
                    .into_element()
                }
                FileListMode::Flat => {
                    flat_file_list(&files_for_list, selected_id.clone(), select_file.clone())
                }
            }
        }))
        .into_element();
}

#[derive(Default)]
struct FileDirectory {
    directories: std::collections::BTreeMap<String, FileDirectory>,
    files: Vec<domain::DiffStat>,
}

fn file_tree(files: &[domain::DiffStat]) -> (Vec<crate::ui::tree::TreeItem>, Vec<SharedString>) {
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
    let expanded = root
        .directories
        .keys()
        .map(|name| SharedString::from(format!("directory:{name}")))
        .collect();
    return (directory_items(&root, ""), expanded);
}

fn directory_items(directory: &FileDirectory, parent: &str) -> Vec<crate::ui::tree::TreeItem> {
    let mut items = Vec::new();
    for (name, child) in &directory.directories {
        let path = if parent.is_empty() {
            name.clone()
        } else {
            format!("{parent}/{name}")
        };
        items.push(crate::ui::tree::node(
            format!("directory:{path}"),
            name.clone(),
            directory_items(child, path.as_str()),
        ));
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
    let label_view = if directory {
        crate::ui::secondary(move || label.clone()).into_element()
    } else {
        crate::ui::code(move || label.clone()).into_element()
    };
    row = row
        .child(crate::ui::icon::icon_view(
            if directory { "folder" } else { "file" },
            crate::ui::icon::IconSize::Button,
            design::text3,
        ))
        .child(view().flex_1().min_w_0().child(label_view));
    if let Some(file) = file {
        if file.additions > 0 {
            row = row.child(
                text(format!("+{}", file.additions))
                    .text_size(px(crate::ui::META))
                    .text_color(move || design::add_fg()),
            );
        }
        if file.deletions > 0 {
            row = row.child(
                text(format!("−{}", file.deletions))
                    .text_size(px(crate::ui::META))
                    .text_color(move || design::del_fg()),
            );
        }
    }
    return row.into_element();
}

fn flat_file_list(
    files: &[domain::DiffStat],
    selected: Option<SharedString>,
    select_file: std::sync::Arc<dyn Fn(String)>,
) -> AnyElement {
    let mut rows: Vec<AnyElement> = Vec::new();
    for file in files {
        let path = file.path.clone();
        let selected_path = format!("file:{path}");
        let selected = selected.as_deref() == Some(selected_path.as_str());
        let mut trailing = Vec::new();
        if file.additions > 0 {
            trailing.push(
                text(format!("+{}", file.additions))
                    .text_size(px(crate::ui::META))
                    .text_color(move || design::add_fg())
                    .into_element(),
            );
        }
        if file.deletions > 0 {
            trailing.push(
                text(format!("−{}", file.deletions))
                    .text_size(px(crate::ui::META))
                    .text_color(move || design::del_fg())
                    .into_element(),
            );
        }
        let on_select = select_file.clone();
        rows.push(
            crate::ui::row::row(
                {
                    let path = path.clone();
                    move || path.clone()
                },
                None,
                Some(
                    crate::ui::icon::icon_view(
                        "file",
                        crate::ui::icon::IconSize::Button,
                        design::text3,
                    )
                    .into_element(),
                ),
                trailing,
                selected,
                false,
                move || on_select(format!("file:{path}")),
            )
            .into_element(),
        );
    }
    return scroll()
        .w_full()
        .h_full()
        .child(view().w_full().flex_col().px(px(6.0)).child(rows))
        .into_element();
}

fn info_body(
    language: Signal<SharedString>,
    tab: Tab,
    notice: Signal<Option<String>>,
    recovery: RecoveryCommands,
) -> AnyElement {
    let sha_for_actions = tab
        .selected_commit()
        .map(|commit| commit.sha.clone())
        .unwrap_or_default();
    let Some(details) = tab.details.clone() else {
        return view()
            .w_full()
            .h(px(40.0))
            .items_center()
            .justify_center()
            .child(
                text(move || i18n::t(language.get().as_str(), "files.empty"))
                    .text_size(px(crate::ui::SECONDARY))
                    .text_color(move || design::text3()),
            )
            .into_element();
    };
    let now = store::now_unix();
    let mut rows: Vec<AnyElement> = Vec::new();
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
    let refs = if details.refs.is_empty() {
        "—".to_string()
    } else {
        details.refs.join(", ")
    };
    for (key, value) in [
        ("files.info.sha", details.sha.clone()),
        ("files.info.tree", details.tree.chars().take(7).collect()),
        ("files.info.parents", parents),
        (
            "files.info.author",
            format!("{} <{}>", details.author, details.author_email),
        ),
        (
            "files.info.authored",
            crate::dates::format(language.get_untracked().as_str(), details.authored_at, now),
        ),
        (
            "files.info.committer",
            format!("{} <{}>", details.committer, details.committer_email),
        ),
        (
            "files.info.committed",
            crate::dates::format(language.get_untracked().as_str(), details.committed_at, now),
        ),
        ("files.info.refs", refs),
    ] {
        rows.push(
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
                        .text_size(px(crate::ui::CODE))
                        .line_height(px(14.0))
                        .text_color(move || design::text())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .into_element(),
        );
    }
    return scroll()
        .w_full()
        .h_full()
        .child(
            view()
                .w_full()
                .px(px(12.0))
                .py(px(8.0))
                .flex_col()
                .child(rows)
                .child(
                    view()
                        .w_full()
                        .mt(px(6.0))
                        .pt(px(6.0))
                        .flex_wrap()
                        .gap(px(6.0))
                        .border_t_1()
                        .border_color(move || design::line())
                        .child(crate::ui::button::button(
                            {
                                let language = language;
                                move || i18n::t(language.get().as_str(), "files.copySha")
                            },
                            crate::ui::button::ButtonVariant::Secondary,
                            crate::ui::button::ButtonSize::Small,
                            false,
                            {
                                let sha = sha_for_actions.clone();
                                move || crate::shared::os::copy_text(notice, language, sha.clone())
                            },
                        ))
                        .child(crate::ui::button::button(
                            {
                                let language = language;
                                move || i18n::t(language.get().as_str(), "files.cherryPick")
                            },
                            crate::ui::button::ButtonVariant::Secondary,
                            crate::ui::button::ButtonSize::Small,
                            false,
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
                            false,
                            {
                                let revert_commit = recovery.clone();
                                let sha = sha_for_actions.clone();
                                move || revert_commit.revert_commit(sha.clone())
                            },
                        )),
                ),
        )
        .into_element();
}
