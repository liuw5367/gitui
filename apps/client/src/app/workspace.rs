//! 工作区：标题栏 + 工具栏 + 引用侧栏 + 中栏主视图 + 右侧检查器 + 状态栏。
//!
//! Changes 启动时在中栏显示 History 提交列表，选中工作区文件后整栏切换到 Diff。
//! 打开失败时在正文区显示错误面板。
//!
//! P4 起这是主窗口版式的组装根：参数收成一个 `AppHandles`，各功能句柄按需克隆。

use crate::app::handles::AppHandles;
use crate::app::session::{Sync, Tab};
use crate::design;
use crate::features::agent::commands::AgentCommands;
use crate::features::changes::commands::ChangesCommands;
use crate::features::commit::commands::CommitCommands;
use crate::features::history::commands::HistoryCommands;
use crate::features::history::{inspector, view as history_view};
use crate::features::refs;
use crate::features::refs::commands::RefsCommands;
use crate::features::search::commands::SearchCommands;
use crate::i18n;
use crate::shared::diff;
use crate::shell;
use nsh_core::prelude::*;

/// 主窗口版式：工具条 + 侧栏 + 主体 + 状态栏。
pub(crate) fn workspace(handles: &AppHandles) -> impl IntoElement + use<> {
    let language = handles.language;
    let tabs = handles.session.tabs;
    let active = handles.session.active;
    let notice = handles.session.notice;
    let sidebar_open = handles.search.sidebar_open;
    let right_panel_open = handles.search.right_panel_open;
    let settings_state = handles.settings.state.settings_state;
    let changes = handles.changes.clone();
    let agent = handles.agent.clone();
    let refs = handles.refs.clone();
    let sync = handles.sync.clone();
    let history = handles.history.clone();
    let merge = handles.merge.clone();
    let commit = handles.commit.clone();
    let session = handles.session.clone();
    let settings = handles.settings.clone();
    let search = handles.search.clone();
    // 主区模式由搜索功能持有（工具栏分段与命令面板的命令都改它）。
    let mode = search.state.mode;
    let current = computed(move || tabs.with(|list| list.get(active.get()).cloned()));
    let change_count = move || {
        return current
            .get()
            .map(|tab| tab.status.len() as u32)
            .unwrap_or(0);
    };
    let search_for_body = search.clone();
    return view()
        .w_full()
        .h_full()
        .flex_col()
        .bg(move || design::content())
        .child(shell::titlebar(
            language,
            tabs,
            active,
            session.clone(),
            settings.clone(),
            refs.clone(),
        ))
        .child(shell::toolbar(
            language,
            change_count,
            mode,
            &search,
            &refs,
            {
                let sync = sync.clone();
                std::sync::Arc::new(move |kind: Sync| sync.open_sync(kind))
            },
            sidebar_open,
            right_panel_open,
        ))
        .child(shell::state_banner(
            language,
            current,
            active,
            session.clone(),
            settings.clone(),
            &refs,
            &sync,
            &commit,
            &search,
        ))
        .child(shell::op_banner(language, current, &merge))
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .flex_row()
                .child(match_body(
                    language,
                    current,
                    sidebar_open,
                    right_panel_open,
                    changes,
                    settings_state,
                    agent,
                    refs,
                    history,
                    commit,
                    session,
                    search_for_body,
                )),
        )
        .child(shell::statusbar(language, current, notice));
}

#[allow(clippy::too_many_arguments)]
fn match_body(
    language: Signal<SharedString>,
    current: Memo<Option<Tab>>,
    sidebar_open: Signal<bool>,
    right_panel_open: Signal<bool>,
    changes: ChangesCommands,
    settings_state: Signal<store::Settings>,
    agent: AgentCommands,
    refs: RefsCommands,
    history: HistoryCommands,
    commit: CommitCommands,
    session: crate::features::SessionHandle,
    search: SearchCommands,
) -> impl IntoElement {
    let left_current = current;
    let right_current = current;
    let show_tree = settings_state.get_untracked().file_tree;
    let tree = crate::app::session::TreeOptions::from_settings(&settings_state.get_untracked());
    return crate::ui::split::horizontal(vec![
        crate::ui::split::Pane::fixed(
            refs_sidebar_content(language, current, refs.clone()),
            crate::design::metrics::SIDEBAR_W,
        )
        .min(180.0)
        .visible(move || {
            sidebar_open.get() && left_current.get().is_some_and(|tab| tab.error.is_none())
        }),
        crate::ui::split::Pane::flex(center_content(
            language,
            current,
            changes.clone(),
            refs.clone(),
            history.clone(),
            session.clone(),
            search.clone(),
            settings_state,
        ))
        .min(320.0),
        crate::ui::split::Pane::fixed(
            right_content(
                language,
                current,
                agent,
                history,
                commit,
                changes,
                show_tree,
                tree,
                search,
                refs,
                settings_state,
            ),
            crate::features::commit::view::WIDTH,
        )
        .min(280.0)
        .visible(move || {
            right_panel_open.get() && right_current.get().is_some_and(|tab| tab.error.is_none())
        }),
    ]);
}

#[allow(clippy::too_many_arguments)]
fn refs_sidebar_content(
    language: Signal<SharedString>,
    current: Memo<Option<Tab>>,
    refs: RefsCommands,
) -> impl IntoElement {
    // 引用侧栏不依赖提交选择或详情，避免浏览提交时重建无关侧栏。
    let sidebar = computed(move || {
        current.with(|current| {
            current.as_ref().map(|tab| Tab {
                id: tab.id,
                refs: tab.refs.clone(),
                branch: tab.branch.clone(),
                head: tab.head.clone(),
                sidebar_collapsed: tab.sidebar_collapsed.clone(),
                operation: tab.operation.clone(),
                status: tab.status.clone(),
                changes: tab.changes.clone(),
                remote_urls: tab.remote_urls.clone(),
                error: tab.error.clone(),
                loaded: tab.loaded,
                ..Default::default()
            })
        })
    });
    return view()
        .w_full()
        .h_full()
        .min_w_0()
        .child(dynamic(move || -> AnyElement {
            let Some(tab) = sidebar.get().filter(|tab| tab.error.is_none()) else {
                return view().w_full().h_full().into_element();
            };
            if !tab.loaded {
                return repository_sidebar_loading().into_element();
            }
            return refs::view::refs_sidebar(&refs.state, &refs, language, tab).into_element();
        }));
}

#[allow(clippy::too_many_arguments)]
fn center_content(
    language: Signal<SharedString>,
    current: Memo<Option<Tab>>,
    changes: ChangesCommands,
    refs: RefsCommands,
    history: HistoryCommands,
    session: crate::features::SessionHandle,
    search: SearchCommands,
    settings_state: Signal<store::Settings>,
) -> impl IntoElement {
    let mode = search.state.mode;
    let refresh = history.clone();
    // commit 详情和文件变更只更新检查器；列表 snapshot 不包含这些高频变化，
    // 因此选中 commit 时保留现有 History 表和 graph 布局。
    let history_tab = computed({
        let current = current;
        move || current.with(|tab| tab.as_ref().map(history_list_snapshot))
    });
    let current_selection = computed({
        let current = current;
        move || {
            current.get().and_then(|tab| {
                tab.selected_commit()
                    .map(|commit| (tab.id, commit.sha.clone()))
            })
        }
    });
    let current_error = computed({
        let current = current;
        move || current.get().and_then(|tab| tab.error.clone())
    });
    let current_unloaded = computed({
        let current = current;
        move || current.get().is_some_and(|tab| !tab.loaded)
    });
    let current_has_history_diff = computed({
        let current = current;
        move || {
            current.get().is_some_and(|tab| {
                tab.show_commit_diff
                    || (tab.compare_base.is_some() && tab.compare_target.is_some())
                    || (tab.compare_worktree && tab.compare_target.is_some())
                    || tab.compare_range().is_some()
                    || tab.file_patch.is_some()
            })
        }
    });
    let current_changes_diff = computed({
        let current = current;
        move || {
            current.with(|tab| {
                tab.as_ref()
                    .and_then(crate::features::changes::view::diff_snapshot)
            })
        }
    });
    let current_blame = computed(move || {
        current.with(|tab| tab.as_ref().map(crate::features::history::blame::snapshot))
    });
    let blame_selected_line =
        computed(move || current.with(|tab| tab.as_ref().and_then(|tab| tab.blame_selected_line)));
    let current_exists = computed({
        let current = current;
        move || current.get().is_some()
    });
    return view()
        .w_full()
        .h_full()
        .min_w_0()
        .min_h_0()
        .child(dynamic(move || -> AnyElement {
            let current_mode = mode.get();
            let reflog_visible = current_mode == crate::Mode::Refs
                && refs.state.selected_tab.get() == refs::state::RefsTab::Reflog
                && current_error.get().is_none()
                && !current_unloaded.get();
            if !reflog_visible {
                refs.clear_reflog_cache();
            }
            if let Some(detail) = current_error.get() {
                let session = session.clone();
                let refresh = refresh.clone();
                let retry: std::sync::Arc<dyn Fn()> =
                    std::sync::Arc::new(move || refresh.refresh());
                return error_panel(language, detail, retry, session).into_element();
            }
            if current_unloaded.get() {
                return repository_loading().into_element();
            }
            return match current_mode {
                crate::Mode::History => {
                    if !current_has_history_diff.get() {
                        let Some(tab) = history_tab.get() else {
                            return empty_workspace(language).into_element();
                        };
                        return history_view::history(
                            &history.state,
                            &history,
                            tab,
                            &search,
                            &refs,
                            current_selection,
                            settings_state,
                        )
                        .into_element();
                    }
                    let Some(tab) = current.get() else {
                        return empty_workspace(language).into_element();
                    };
                    let reference_compare =
                        tab.compare_base.is_some() && tab.compare_target.is_some();
                    let worktree_compare = tab.compare_worktree && tab.compare_target.is_some();
                    let compare_range = tab.compare_range();
                    let compare_active =
                        reference_compare || worktree_compare || compare_range.is_some();
                    let (label, raw_patch, compare_feedback) = if compare_active {
                        let lang = language.get_untracked();
                        let (from, to) = if let Some((newer, older)) = compare_range {
                            (
                                tab.commits
                                    .get(older)
                                    .map(|commit| commit.short.clone())
                                    .unwrap_or_default(),
                                tab.commits
                                    .get(newer)
                                    .map(|commit| commit.short.clone())
                                    .unwrap_or_default(),
                            )
                        } else {
                            (
                                tab.compare_base
                                    .as_deref()
                                    .map(|reference| {
                                        comparison_endpoint_label(&tab, lang.as_str(), reference)
                                    })
                                    .unwrap_or_default(),
                                tab.compare_target
                                    .as_deref()
                                    .map(|reference| {
                                        comparison_endpoint_label(&tab, lang.as_str(), reference)
                                    })
                                    .unwrap_or_default(),
                            )
                        };
                        let label = if worktree_compare {
                            i18n::with_arguments(
                                lang.as_str(),
                                "compare.worktreeRange",
                                &[("target", to)],
                            )
                        } else {
                            i18n::with_arguments(
                                lang.as_str(),
                                "compare.range",
                                &[("from", from), ("to", to)],
                            )
                        };
                        let (patch, feedback) =
                            match crate::features::history::ops::comparison_patch(&tab, false) {
                                Ok(patch) if patch.raw.trim().is_empty() => {
                                    (patch, Some("compare.noChanges"))
                                }
                                Ok(patch) => (patch, None),
                                Err(
                                    crate::features::history::ops::CompareFailure::IdenticalEndpoints,
                                ) => (domain::diff::Patch::default(), Some("compare.sameEndpoint")),
                                Err(
                                    crate::features::history::ops::CompareFailure::UnrelatedHistories,
                                ) => (domain::diff::Patch::default(), Some("compare.unrelated")),
                                Err(crate::features::history::ops::CompareFailure::Git) => {
                                    (domain::diff::Patch::default(), Some("compare.failed"))
                                }
                            };
                        (label, patch, feedback)
                    } else {
                        match tab.file_patch.clone() {
                            Some((path, patch)) => (path, patch, None),
                            None => (
                                tab.selected_commit()
                                    .map(|commit| commit.short.clone())
                                    .unwrap_or_default(),
                                tab.patch.clone(),
                                None,
                            ),
                        }
                    };
                    let ignore_whitespace = history.diff.state.ignore_whitespace.get();
                    let patch = if ignore_whitespace {
                        crate::features::history::ops::diff_patch_for_tab(&tab, true)
                    } else {
                        raw_patch.clone()
                    };
                    let return_title = if compare_active {
                        label.clone()
                    } else {
                        tab.selected_commit()
                            .map(|commit| commit.summary.clone())
                            .unwrap_or_else(|| label.clone())
                    };
                    let return_to_history: std::sync::Arc<dyn Fn()> = {
                        let history = history.clone();
                        std::sync::Arc::new(move || history.return_to_history())
                    };
                    let open_blame = tab.file_patch.as_ref().map(|_| {
                        let history = history.clone();
                        std::sync::Arc::new(move || history.open_blame())
                            as std::sync::Arc<dyn Fn()>
                    });
                    let file_name = if compare_active {
                        None
                    } else {
                        tab.file_patch.as_ref().map(|(path, _)| path.clone())
                    };
                    let file_navigation = if !compare_active && tab.file_patch.is_some() {
                        let tab_id = tab.id;
                        let previous_history = history.clone();
                        let next_history = history.clone();
                        Some(diff::FileNavigation {
                            current: tab.selected_file.unwrap_or(0),
                            total: tab.files.len(),
                            previous: std::sync::Arc::new(move || {
                                previous_history.select_file_relative(tab_id, -1)
                            }),
                            next: std::sync::Arc::new(move || {
                                next_history.select_file_relative(tab_id, 1)
                            }),
                        })
                    } else {
                        None
                    };
                    return diff::diff(
                        &history.diff.state,
                        &history.diff,
                        language,
                        patch,
                        raw_patch,
                        compare_feedback,
                        return_title,
                        return_to_history,
                        open_blame,
                        file_name,
                        file_navigation,
                    )
                    .into_element();
                }
                crate::Mode::Agent => {
                    if !current_exists.get() {
                        return empty_workspace(language).into_element();
                    }
                    view().w_full().flex_1().min_h_0().into_element()
                }
                crate::Mode::Blame => {
                    let Some(tab) = current_blame.get() else {
                        return empty_workspace(language).into_element();
                    };
                    crate::features::history::blame_view(
                        language,
                        tab,
                        blame_selected_line,
                        history.clone(),
                    )
                    .into_element()
                }
                crate::Mode::Refs => {
                    let Some(tab) = current.get() else {
                        return empty_workspace(language).into_element();
                    };
                    // 差异状态自身带 Reflog 行身份；这里不订阅行选中信号，避免重建整个中栏。
                    if refs.state.selected_tab.get() == refs::state::RefsTab::Reflog
                        && let Some(file_diff) = refs
                            .state
                            .reflog_file_diff
                            .get()
                            .filter(|diff| {
                                diff.selection.tab_id == tab.id && diff.selection.path == tab.path
                            })
                    {
                        let selection = file_diff.selection.clone();
                        match file_diff.status.clone() {
                            refs::state::ReflogFileDiffStatus::Loading => {
                                return inspector::reflog_commit_placeholder(
                                    language,
                                    "reflog.diffLoading",
                                )
                                .into_element();
                            }
                            refs::state::ReflogFileDiffStatus::Failed => {
                                return inspector::reflog_commit_placeholder(
                                    language,
                                    "reflog.diffFailed",
                                )
                                .into_element();
                            }
                            refs::state::ReflogFileDiffStatus::Loaded { patch, raw_patch } => {
                                let title = match refs.state.reflog_detail.get() {
                                    Some(refs::state::ReflogDetailState::Loaded {
                                        selection: loaded,
                                        details,
                                        ..
                                    }) if loaded == selection => details.subject,
                                    _ => selection.summary.clone(),
                                };
                                return reflog_file_diff_view(
                                    language,
                                    &history.diff.state,
                                    &history.diff,
                                    refs.clone(),
                                    file_diff,
                                    title,
                                    patch,
                                    raw_patch,
                                )
                                .into_element();
                            }
                        }
                    }
                    refs::view::refs_view(
                        &refs.state,
                        &refs,
                        language,
                        tab,
                        &search,
                        settings_state,
                    )
                    .into_element()
                }
                crate::Mode::Changes => {
                    if let Some(snapshot) = current_changes_diff.get() {
                        crate::features::changes::view::change_diff(
                            &changes,
                            &history.diff.state,
                            &history.diff,
                            current,
                            snapshot,
                        )
                    } else {
                        let Some(tab) = history_tab.get() else {
                            return empty_workspace(language).into_element();
                        };
                        history_view::history(
                            &history.state,
                            &history,
                            tab,
                            &search,
                            &refs,
                            current_selection,
                            settings_state,
                        )
                        .into_element()
                    }
                }
            };
        }));
}

fn reflog_file_diff_view(
    language: Signal<SharedString>,
    diff_state: &crate::shared::diff::DiffState,
    diff_cmd: &crate::shared::diff::DiffCommands,
    refs: RefsCommands,
    file_diff: refs::state::ReflogFileDiffState,
    title: String,
    patch: domain::diff::Patch,
    raw_patch: domain::diff::Patch,
) -> AnyElement {
    let index = file_diff
        .files
        .iter()
        .position(|file| file.path == file_diff.path)
        .unwrap_or(0);
    let file_count = file_diff.files.len();
    let previous_refs = refs.clone();
    let next_refs = refs.clone();
    let file_navigation = crate::shared::diff::FileNavigation {
        current: index,
        total: file_count,
        previous: std::sync::Arc::new(move || previous_refs.select_reflog_file_relative(-1)),
        next: std::sync::Arc::new(move || next_refs.select_reflog_file_relative(1)),
    };
    let return_refs = refs.clone();
    let back: std::sync::Arc<dyn Fn()> =
        std::sync::Arc::new(move || return_refs.return_to_reflog());
    let header = crate::shared::diff::diff_title_bar(
        language,
        "reflog.backToList",
        back,
        title,
        Some(file_diff.path.clone()),
        Some(file_navigation),
    );
    let reload_refs = refs.clone();
    let on_ignore_whitespace: std::sync::Arc<dyn Fn(bool)> =
        std::sync::Arc::new(move |ignore| reload_refs.reload_reflog_file_diff(ignore));
    return view()
        .w_full()
        .h_full()
        .min_h_0()
        .flex_col()
        .bg(move || design::content())
        .child(header)
        .child(crate::shared::diff::diff_panel(
            diff_state,
            diff_cmd,
            language,
            patch.clone(),
            raw_patch,
            Some("reflog.diffEmpty"),
            None,
            Some(on_ignore_whitespace),
            None,
        ))
        .into_element();
}

fn history_list_snapshot(tab: &Tab) -> Tab {
    let started = std::time::Instant::now();
    let snapshot = Tab {
        id: tab.id,
        branch: tab.branch.clone(),
        commits: tab.commits.clone(),
        graph_commits: tab.graph_commits.clone(),
        refs: tab.refs.clone(),
        authors: tab.authors.clone(),
        filter: tab.filter.clone(),
        compare: tab.compare.clone(),
        compare_base: tab.compare_base.clone(),
        compare_target: tab.compare_target.clone(),
        compare_three_dot: tab.compare_three_dot,
        compare_worktree: tab.compare_worktree,
        head: tab.head.clone(),
        loading: tab.loading,
        ..Tab::default()
    };
    log::debug!(
        target: "gitui::history",
        "event=workspace.history_snapshot commits={} graph_commits={} refs={} elapsed_ms={}",
        snapshot.commits.len(),
        snapshot.graph_commits.len(),
        snapshot.refs.branches.len()
            + snapshot.refs.remote_branches.len()
            + snapshot.refs.tags.len()
            + snapshot.refs.stashes.len(),
        started.elapsed().as_millis(),
    );
    return snapshot;
}

fn skeleton_block(width: f32, height: f32) -> AnyElement {
    return view()
        .w(px(width))
        .h(px(height))
        .rounded(px(height / 2.0))
        .bg(move || design::line_soft())
        .into_element();
}

fn repository_sidebar_loading() -> AnyElement {
    let widths = [118.0, 78.0, 104.0, 64.0, 92.0, 74.0, 112.0, 68.0, 96.0];
    let rows = widths
        .into_iter()
        .enumerate()
        .map(|(index, width)| {
            view()
                .w_full()
                .h(px(crate::ui::ROW))
                .flex_row()
                .items_center()
                .gap(px(crate::ui::GAP_SM))
                .px(px(if index % 3 == 0 { 8.0 } else { 20.0 }))
                .child(skeleton_block(10.0, 10.0))
                .child(skeleton_block(width, 8.0))
                .into_element()
        })
        .collect::<Vec<_>>();
    return view()
        .w_full()
        .h_full()
        .min_h_0()
        .flex_col()
        .gap(px(2.0))
        .pt(px(8.0))
        .bg(move || design::sidebar())
        .border_r_1()
        .border_color(move || design::line())
        .child(rows)
        .into_element();
}

fn repository_loading() -> AnyElement {
    let toolbar = view()
        .w_full()
        .h(px(crate::ui::ROW))
        .flex_row()
        .items_center()
        .gap(px(crate::ui::GAP))
        .px(px(10.0))
        .child(skeleton_block(56.0, 10.0))
        .child(skeleton_block(72.0, 10.0))
        .child(skeleton_block(48.0, 10.0))
        .child(view().flex_1())
        .child(skeleton_block(80.0, 10.0));
    let header = view()
        .w_full()
        .h(px(crate::design::metrics::COMMIT_COLS_H))
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .px(px(8.0))
        .border_b_1()
        .border_color(move || design::line_soft())
        .child(skeleton_block(28.0, 8.0))
        .child(view().flex_1().child(skeleton_block(76.0, 8.0)))
        .child(skeleton_block(64.0, 8.0))
        .child(skeleton_block(48.0, 8.0));
    let rows = (0..12)
        .map(|index| {
            let summary_width = [84.0, 128.0, 112.0, 72.0, 136.0, 96.0][index % 6];
            view()
                .w_full()
                .h(px(crate::design::metrics::COMMIT_ROW_H))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .px(px(8.0))
                .child(skeleton_block(20.0, 8.0))
                .child(skeleton_block(summary_width, 10.0))
                .child(view().flex_1())
                .child(skeleton_block(60.0, 8.0))
                .child(skeleton_block(48.0, 8.0))
                .into_element()
        })
        .collect::<Vec<_>>();
    return view()
        .w_full()
        .h_full()
        .min_h_0()
        .flex_col()
        .bg(move || design::content())
        .child(toolbar)
        .child(header)
        .child(view().w_full().flex_1().min_h_0().flex_col().child(rows))
        .into_element();
}

fn repository_inspector_loading() -> AnyElement {
    let mut file_rows = Vec::new();
    for index in 0..8 {
        let indent = if index % 3 == 0 { 8.0 } else { 24.0 };
        let width = [108.0, 76.0, 132.0, 92.0][index % 4];
        file_rows.push(
            view()
                .w_full()
                .h(px(crate::ui::ROW))
                .flex_row()
                .items_center()
                .gap(px(crate::ui::GAP_SM))
                .px(px(indent))
                .child(skeleton_block(10.0, 10.0))
                .child(skeleton_block(width, 8.0))
                .into_element(),
        );
    }
    let file_header = view()
        .w_full()
        .h(px(crate::ui::ROW))
        .flex_row()
        .items_center()
        .gap(px(crate::ui::GAP_SM))
        .px(px(10.0))
        .border_b_1()
        .border_color(move || design::line_soft())
        .child(skeleton_block(112.0, 8.0))
        .child(view().flex_1())
        .child(skeleton_block(24.0, 8.0))
        .child(skeleton_block(24.0, 8.0));
    let info_rows = view()
        .w_full()
        .h(px(inspector::INFO_PANEL_H))
        .flex_shrink_0()
        .flex_col()
        .gap(px(12.0))
        .p(px(12.0))
        .border_t_1()
        .border_color(move || design::line())
        .child(skeleton_block(136.0, 10.0))
        .child(skeleton_block(196.0, 8.0))
        .child(skeleton_block(112.0, 8.0))
        .child(skeleton_block(164.0, 8.0));
    return view()
        .w_full()
        .h_full()
        .min_h_0()
        .flex_col()
        .bg(move || design::panel())
        .child(file_header)
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .flex_col()
                .gap(px(2.0))
                .pt(px(8.0))
                .child(file_rows),
        )
        .child(info_rows)
        .into_element();
}

fn comparison_endpoint_label(tab: &Tab, language: &str, endpoint: &str) -> String {
    if let Some(name) = endpoint.strip_prefix("refs/heads/") {
        return i18n::with_arguments(
            language,
            "compare.localBranch",
            &[("name", name.to_string())],
        );
    }
    if let Some(name) = endpoint.strip_prefix("refs/remotes/") {
        return i18n::with_arguments(
            language,
            "compare.remoteBranch",
            &[("name", name.to_string())],
        );
    }
    if let Some(name) = endpoint.strip_prefix("refs/tags/") {
        return i18n::with_arguments(language, "compare.tagRef", &[("name", name.to_string())]);
    }
    if let Some(stash) = tab.refs.stashes.iter().find(|stash| stash.oid == endpoint) {
        return i18n::with_arguments(
            language,
            "compare.stashRef",
            &[("name", stash.summary.clone())],
        );
    }
    return endpoint.chars().take(10).collect();
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RightPanelStructure {
    tab_id: u64,
    path: String,
    loaded: bool,
    error: Option<String>,
    changes: Vec<(String, Option<String>)>,
    changelists: Vec<String>,
    active_changelist: Option<String>,
    conflicts: Vec<String>,
}

impl RightPanelStructure {
    fn from_tab(tab: &Tab) -> Self {
        let mut changes = tab
            .changes
            .iter()
            .map(|entry| (entry.path.clone(), entry.changelist.clone()))
            .collect::<Vec<_>>();
        // 暂存会让 Git 按 staged/unstaged 重排列表；右侧树只关心稳定的路径和清单归属。
        changes.sort();
        return Self {
            tab_id: tab.id,
            path: tab.path.clone(),
            loaded: tab.loaded,
            error: tab.error.clone(),
            changes,
            changelists: tab
                .changelists
                .iter()
                .map(|list| list.name.clone())
                .collect(),
            active_changelist: tab.active_changelist.clone(),
            conflicts: tab.conflicts.clone(),
        };
    }
}

#[allow(clippy::too_many_arguments)]
fn right_content(
    language: Signal<SharedString>,
    current: Memo<Option<Tab>>,
    agent: AgentCommands,
    history: HistoryCommands,
    commit: CommitCommands,
    changes: ChangesCommands,
    show_tree: bool,
    tree: crate::app::session::TreeOptions,
    search: SearchCommands,
    refs: RefsCommands,
    settings_state: Signal<store::Settings>,
) -> impl IntoElement {
    let mode = search.state.mode;
    let current_structure = computed({
        let current = current;
        move || current.with(|tab| tab.as_ref().map(RightPanelStructure::from_tab))
    });
    let inspector_file_list_mode = signal(inspector::FileListMode::Tree);
    let reflog_file_tree_expanded: Signal<Option<Vec<SharedString>>> = signal(None);
    let reflog_tree_selection: Signal<Option<refs::state::ReflogEntryKey>> = signal(None);
    return view()
        .w_full()
        .h_full()
        .min_w_0()
        .child(dynamic(move || -> AnyElement {
            let current_mode = mode.get();
            // Changes / Refs 的提交面板只在文件树结构变化时重建；staged、行数统计和
            // 当前选择继续由面板内部的绑定单独响应。
            let structure = matches!(current_mode, crate::Mode::Changes | crate::Mode::Refs)
                .then(|| current_structure.get());
            let tab = if structure.is_some() {
                current.get_untracked()
            } else {
                current.get()
            };
            let Some(tab) = tab.filter(|tab| tab.error.is_none()) else {
                return view().w_full().h_full().into_element();
            };
            let loaded = match structure {
                Some(Some(structure)) => structure.loaded,
                Some(None) => return view().w_full().h_full().into_element(),
                None => tab.loaded,
            };
            if !loaded {
                return repository_inspector_loading();
            }
            if current_mode != crate::Mode::Refs && reflog_tree_selection.get().is_some() {
                reflog_tree_selection.set(None);
                reflog_file_tree_expanded.set(None);
            }
            match current_mode {
                crate::Mode::History => inspector::inspector(
                    &history.state,
                    &history,
                    tab,
                    language,
                    inspector_file_list_mode,
                    crate::app::session::TreeOptions::from_settings(&settings_state.get()),
                )
                .into_element(),
                crate::Mode::Agent => crate::features::agent::view::agent_panel(
                    &agent.state,
                    &agent,
                    language,
                    settings_state,
                )
                .into_element(),
                crate::Mode::Changes => crate::features::commit::view::commit_panel(
                    &commit.state,
                    &commit,
                    changes.clone(),
                    language,
                    tab,
                    current,
                    show_tree,
                    tree,
                )
                .into_element(),
                crate::Mode::Blame => crate::features::history::blame::blame_history_panel(
                    language,
                    tab,
                    history.clone(),
                    settings_state,
                )
                .into_element(),
                crate::Mode::Refs => {
                    if refs.state.selected_tab.get() == refs::state::RefsTab::Reflog {
                        let selected = refs.state.reflog_selection.get().filter(|selection| {
                            selection.tab_id == tab.id && selection.path == tab.path
                        });
                        if reflog_tree_selection.get() != selected {
                            reflog_tree_selection.set(selected.clone());
                            reflog_file_tree_expanded.set(None);
                        }
                        if selected.is_none() {
                            return inspector::reflog_commit_placeholder(
                                language,
                                if refs.state.reflog_page.get().loading {
                                    "reflog.detailsLoading"
                                } else if refs.state.reflog_entries.get().is_empty() {
                                    "reflog.empty"
                                } else {
                                    "reflog.selectEntry"
                                },
                            )
                            .into_element();
                        }
                        return match refs.state.reflog_detail.get() {
                            Some(refs::state::ReflogDetailState::Loading(selection))
                                if selected.as_ref() == Some(&selection) =>
                            {
                                inspector::reflog_commit_placeholder(
                                    language,
                                    "reflog.detailsLoading",
                                )
                                .into_element()
                            }
                            Some(refs::state::ReflogDetailState::Loaded {
                                selection,
                                details,
                                files,
                            }) if selected.as_ref() == Some(&selection) => {
                                let history = history.clone();
                                let tab_id = tab.id;
                                let sha = details.sha.clone();
                                let file_history: std::sync::Arc<dyn Fn(String)> =
                                    std::sync::Arc::new(move |path| {
                                        history.show_file_history_at(tab_id, path, sha.clone())
                                    });
                                let file_selection = selected.clone().expect("selected reflog row");
                                let select_file: std::sync::Arc<dyn Fn(String)> = {
                                    let refs = refs.clone();
                                    std::sync::Arc::new(move |path| {
                                        let Some(path) = path.strip_prefix("file:") else {
                                            return;
                                        };
                                        refs.select_reflog_file(
                                            file_selection.clone(),
                                            path.to_string(),
                                        )
                                    })
                                };
                                inspector::reflog_commit_inspector(
                                    language,
                                    details,
                                    files,
                                    tab.path.clone(),
                                    inspector_file_list_mode,
                                    crate::app::session::TreeOptions::from_settings(
                                        &settings_state.get(),
                                    ),
                                    reflog_file_tree_expanded,
                                    refs.notice,
                                    file_history,
                                    select_file,
                                )
                                .into_element()
                            }
                            Some(refs::state::ReflogDetailState::Failed(selection))
                                if selected.as_ref() == Some(&selection) =>
                            {
                                inspector::reflog_commit_placeholder(
                                    language,
                                    "reflog.detailsFailed",
                                )
                                .into_element()
                            }
                            _ => inspector::reflog_commit_placeholder(
                                language,
                                "reflog.detailsLoading",
                            )
                            .into_element(),
                        };
                    }
                    if reflog_tree_selection.get().is_some() {
                        reflog_tree_selection.set(None);
                        reflog_file_tree_expanded.set(None);
                    }
                    crate::features::commit::view::commit_panel(
                        &commit.state,
                        &commit,
                        changes.clone(),
                        language,
                        tab,
                        current,
                        show_tree,
                        tree,
                    )
                    .into_element()
                }
            }
        }));
}

fn empty_workspace(language: Signal<SharedString>) -> impl IntoElement {
    return view()
        .w_full()
        .h_full()
        .items_center()
        .justify_center()
        .child(
            text(move || i18n::t(language.get().as_str(), "history.empty"))
                .text_size(px(crate::ui::BODY))
                .text_color(move || design::text3()),
        );
}

fn error_panel(
    language: Signal<SharedString>,
    detail: String,
    retry: std::sync::Arc<dyn Fn()>,
    session: crate::features::SessionHandle,
) -> impl IntoElement {
    let detail = std::sync::Arc::new(detail);
    let welcome = session;
    return view()
        .w_full()
        .h_full()
        .items_center()
        .justify_center()
        .flex_col()
        .gap(px(10.0))
        .child(crate::widgets::icon("info", 20.0, design::danger))
        .child(
            text(move || i18n::t(language.get().as_str(), "error.title"))
                .text_size(px(crate::ui::BODY))
                .font_weight(FontWeight(600))
                .text_color(move || design::text()),
        )
        .child(
            text({
                let detail = detail.clone();
                move || detail.as_str().to_string()
            })
            .text_size(px(crate::ui::SECONDARY))
            .text_color(move || design::text2()),
        )
        .child(
            view()
                .flex_row()
                .gap(px(8.0))
                .child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "error.retry"),
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Regular,
                    false,
                    {
                        let retry = retry.clone();
                        move || retry()
                    },
                ))
                .child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "error.close"),
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Regular,
                    false,
                    {
                        let welcome = welcome.clone();
                        move || welcome.go_welcome()
                    },
                )),
        );
}
