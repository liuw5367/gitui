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
    let current = computed(move || tabs.get().get(active.get()).cloned());
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
        move || current.get().map(history_list_snapshot)
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
            if let Some(detail) = current_error.get() {
                let session = session.clone();
                let refresh = refresh.clone();
                let retry: std::sync::Arc<dyn Fn()> =
                    std::sync::Arc::new(move || refresh.refresh());
                return error_panel(language, detail, retry, session).into_element();
            }
            return match mode.get() {
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
                    let file_count = if compare_active {
                        raw_patch.files.len()
                    } else {
                        tab.files.len()
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
                    return diff::diff(
                        &history.diff.state,
                        &history.diff,
                        language,
                        patch,
                        raw_patch,
                        compare_feedback,
                        return_title,
                        file_count,
                        return_to_history,
                        open_blame,
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
                    let Some(tab) = current.get() else {
                        return empty_workspace(language).into_element();
                    };
                    crate::features::history::blame_view(
                        language,
                        tab,
                        history.clone(),
                        settings_state,
                    )
                    .into_element()
                }
                crate::Mode::Refs => {
                    let Some(tab) = current.get() else {
                        return empty_workspace(language).into_element();
                    };
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
                    let Some(tab) = current.get() else {
                        return empty_workspace(language).into_element();
                    };
                    if tab.stash_diff.is_some() || tab.selected_change.is_some() {
                        crate::features::changes::view::change_diff(&changes, tab)
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

fn history_list_snapshot(mut tab: Tab) -> Tab {
    tab.selected = None;
    tab.show_commit_diff = false;
    tab.patch = domain::diff::Patch::default();
    tab.files.clear();
    tab.selected_file = None;
    tab.file_patch = None;
    tab.details = None;
    tab.error = None;
    tab.blame.clear();
    tab.blame_path.clear();
    tab.blame_revision.clear();
    tab.blame_return_mode = None;
    tab.blame_focus_sha = None;
    tab.blame_selected_line = None;
    tab.blame_show_diff = false;
    tab.blame_patch = domain::diff::Patch::default();
    tab.blame_error = None;
    tab.file_history.clear();
    return tab;
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
    let inspector_file_list_mode = signal(inspector::FileListMode::Tree);
    let reflog_file_tree_expanded: Signal<Option<Vec<SharedString>>> = signal(None);
    return view()
        .w_full()
        .h_full()
        .min_w_0()
        .child(dynamic(move || -> AnyElement {
            let Some(tab) = current.get().filter(|tab| tab.error.is_none()) else {
                return view().w_full().h_full().into_element();
            };
            match mode.get() {
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
                    Some(changes.clone()),
                    language,
                    tab,
                    show_tree,
                    tree,
                    None,
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
                        if selected.is_none() {
                            return inspector::reflog_commit_placeholder(
                                language,
                                if tab.reflog.is_empty() {
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
                    let ask_agent: std::sync::Arc<dyn Fn()> = {
                        let agent = agent.clone();
                        let mode = mode;
                        std::sync::Arc::new(move || {
                            mode.set(crate::Mode::Agent);
                            agent.propose_plan();
                        })
                    };
                    crate::features::commit::view::commit_panel(
                        &commit.state,
                        &commit,
                        None,
                        language,
                        tab,
                        show_tree,
                        tree,
                        Some(ask_agent),
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
