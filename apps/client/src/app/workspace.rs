//! 工作区：标题栏 + 工具栏 + 引用侧栏 + History（+ 右侧提交检查器）+ 状态栏。
//!
//! History 默认显示提交列表与 branch graph；提交菜单或 Compare 显式打开时才显示补丁。
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
            history.state.query,
            change_count,
            mode,
            &search,
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
        .child(shell::sync_banner(current))
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
            changes,
            settings_state,
            refs,
            history.clone(),
            session.clone(),
            search.clone(),
        ))
        .min(320.0),
        crate::ui::split::Pane::fixed(
            right_content(language, current, agent, history, commit, search),
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
    return view()
        .w_full()
        .h_full()
        .min_w_0()
        .child(dynamic(move || -> AnyElement {
            let Some(tab) = current.get().filter(|tab| tab.error.is_none()) else {
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
    settings_state: Signal<store::Settings>,
    refs: RefsCommands,
    history: HistoryCommands,
    session: crate::features::SessionHandle,
    search: SearchCommands,
) -> impl IntoElement {
    let mode = search.state.mode;
    // Refs / Reflog 的页签选择仍随正文重建（与迁入前一致）。
    let refs_tab = signal(refs::state::RefsTab::Refs);
    let group_by_directory = settings_state.get_untracked().file_tree;
    let tree_options =
        crate::app::session::TreeOptions::from_settings(&settings_state.get_untracked());
    let notice = history.notice;
    let refresh = history.clone();
    return view()
        .w_full()
        .h_full()
        .min_w_0()
        .min_h_0()
        .child(dynamic(move || -> AnyElement {
            let Some(tab) = current.get() else {
                return empty_workspace(language).into_element();
            };
            if let Some(detail) = tab.error.clone() {
                let session = session.clone();
                let refresh = refresh.clone();
                let retry: std::sync::Arc<dyn Fn()> =
                    std::sync::Arc::new(move || refresh.refresh());
                return error_panel(language, detail, retry, session).into_element();
            }

            // diff 区展示的内容：区间对比 > 单文件补丁 > 提交补丁。
            let (label, raw_patch) = match tab.compare_range() {
                Some((from, to)) => {
                    let from_short = tab
                        .commits
                        .get(from)
                        .map(|commit| commit.short.clone())
                        .unwrap_or_default();
                    let to_short = tab
                        .commits
                        .get(to)
                        .map(|commit| commit.short.clone())
                        .unwrap_or_default();
                    (
                        i18n::with_arguments(
                            language.get_untracked().as_str(),
                            "compare.range",
                            &[("from", from_short), ("to", to_short)],
                        ),
                        crate::features::history::ops::compare_patch(&tab).unwrap_or_default(),
                    )
                }
                None => match tab.file_patch.clone() {
                    Some((path, patch)) => (path, patch),
                    None => (
                        tab.selected_commit()
                            .map(|commit| commit.short.clone())
                            .unwrap_or_default(),
                        tab.patch.clone(),
                    ),
                },
            };
            // 忽略空白用 Git 重新生成展示补丁；原始补丁仍供统计、复制与导出使用。
            let ignore_whitespace = history.diff.state.ignore_whitespace.get();
            let patch = if ignore_whitespace {
                crate::features::history::ops::diff_patch_for_tab(&tab, true)
            } else {
                raw_patch.clone()
            };
            let tab_for_history = tab.clone();
            let tab_for_changes = tab.clone();
            let search_for_body = search.clone();
            return match mode.get() {
                crate::Mode::History => {
                    let show_diff = tab.show_commit_diff
                        || tab.compare_range().is_some()
                        || (tab.compare_base.is_some() && tab.compare_target.is_some())
                        || tab.file_patch.is_some();
                    let mut center =
                        view()
                            .w_full()
                            .flex_1()
                            .min_h_0()
                            .flex_col()
                            .child(history_view::history(
                                &history.state,
                                &history,
                                tab_for_history,
                                &search_for_body,
                                &refs,
                            ));
                    if show_diff {
                        center = center.child(diff::diff(
                            &history.diff.state,
                            &history.diff,
                            language,
                            label,
                            patch,
                            raw_patch,
                        ));
                    }
                    center.into_element()
                }
                crate::Mode::Agent => view().w_full().flex_1().min_h_0().into_element(),
                crate::Mode::Blame => {
                    crate::features::history::blame_view(language, tab_for_history, notice)
                        .into_element()
                }
                crate::Mode::Refs => refs::view::refs_view(
                    &refs.state,
                    &refs,
                    language,
                    tab_for_changes,
                    &search,
                    refs_tab,
                )
                .into_element(),
                crate::Mode::Changes => crate::features::changes::view::changes(
                    &changes.state,
                    &changes,
                    tab_for_changes,
                    group_by_directory,
                    tree_options,
                )
                .into_element(),
            };
        }));
}

#[allow(clippy::too_many_arguments)]
fn right_content(
    language: Signal<SharedString>,
    current: Memo<Option<Tab>>,
    agent: AgentCommands,
    history: HistoryCommands,
    commit: CommitCommands,
    search: SearchCommands,
) -> impl IntoElement {
    let mode = search.state.mode;
    let inspector_tab = signal(inspector::InspectorTab::Files);
    return view()
        .w_full()
        .h_full()
        .min_w_0()
        .child(dynamic(move || -> AnyElement {
            let Some(tab) = current.get().filter(|tab| tab.error.is_none()) else {
                return view().w_full().h_full().into_element();
            };
            match mode.get() {
                crate::Mode::History => {
                    inspector::inspector(&history.state, &history, tab, language, inspector_tab)
                        .into_element()
                }
                crate::Mode::Agent => {
                    crate::features::agent::view::agent_panel(&agent.state, &agent, language)
                        .into_element()
                }
                crate::Mode::Changes | crate::Mode::Refs | crate::Mode::Blame => {
                    // 「请 Agent 起草」= 切到 Agent 主视图并提议一份计划。
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
                        language,
                        tab,
                        ask_agent,
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
