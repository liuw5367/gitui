//! 工作区：标题栏 + 工具栏 + 侧栏 + History/diff（+ 提交文件面板）+ 状态栏。
//!
//! diff 区显示三种内容之一：选中提交的补丁、提交内单个文件的补丁、两个提交的区间
//! 补丁。打开失败时在正文区显示错误面板。
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
    let inspector_open = handles.search.inspector_open;
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
            inspector_open,
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
                    inspector_open,
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
    inspector_open: Signal<bool>,
    changes: ChangesCommands,
    settings_state: Signal<store::Settings>,
    agent: AgentCommands,
    refs: RefsCommands,
    history: HistoryCommands,
    commit: CommitCommands,
    session: crate::features::SessionHandle,
    search: SearchCommands,
) -> impl IntoElement {
    return view()
        .w_full()
        .h_full()
        .flex_row()
        .child(dynamic(move || -> AnyElement {
            let tab = current.get();
            let has_error = tab.as_ref().map(|tab| tab.error.is_some()).unwrap_or(false);
            if has_error {
                let detail = tab
                    .and_then(|tab| tab.error)
                    .unwrap_or_else(|| i18n::t(language.get().as_str(), "error.title"));
                let session = session.clone();
                // 「重试」就是历史功能的刷新。
                let refresh = history.clone();
                let retry: std::sync::Arc<dyn Fn()> =
                    std::sync::Arc::new(move || refresh.refresh());
                return error_panel(language, detail, retry, session).into_element();
            }
            let Some(tab) = tab else {
                return view()
                    .w_full()
                    .h_full()
                    .items_center()
                    .justify_center()
                    .child(
                        text(move || i18n::t(language.get().as_str(), "history.empty"))
                            .text_size(px(crate::ui::BODY))
                            .text_color(move || design::text3()),
                    )
                    .into_element();
            };
            return body(
                language,
                tab,
                inspector_open,
                changes.clone(),
                settings_state,
                agent.clone(),
                refs.clone(),
                history.clone(),
                commit.clone(),
                search.clone(),
            )
            .into_element();
        }));
}

#[allow(clippy::too_many_arguments)]
fn body(
    language: Signal<SharedString>,
    tab: Tab,
    inspector_open: Signal<bool>,
    changes: ChangesCommands,
    settings_state: Signal<store::Settings>,
    agent: AgentCommands,
    refs: RefsCommands,
    history: HistoryCommands,
    commit: CommitCommands,
    search: SearchCommands,
) -> impl IntoElement {
    let mode = search.state.mode;
    let tab_for_sidebar = tab.clone();
    let tab_for_history = tab.clone();
    let tab_for_inspector = tab.clone();

    // diff 区展示的内容：区间对比 > 单文件补丁 > 提交补丁。
    let (label, patch) = match tab.compare_range() {
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
            let label = i18n::with_arguments(
                language.get_untracked().as_str(),
                "compare.range",
                &[("from", from_short), ("to", to_short)],
            );
            (
                label,
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
    let inspector_visible = inspector_open.get_untracked();
    let tab_for_changes = tab.clone();
    let inspector_tab = signal(inspector::InspectorTab::Files);
    // Refs / Reflog 的页签选择仍随正文重建（与迁入前一致）。
    let refs_tab = signal(refs::state::RefsTab::Refs);
    let group_by_directory = settings_state.get_untracked().file_tree;
    let tree_options =
        crate::app::session::TreeOptions::from_settings(&settings_state.get_untracked());
    let commit_for_panel = commit.clone();
    let history_for_inspector = history.clone();
    let search_for_body = search.clone();
    let notice = history.notice;
    return view()
        .w_full()
        .h_full()
        .flex_row()
        .child(refs::view::refs_sidebar(
            &refs.state,
            &refs,
            language,
            tab_for_sidebar,
        ))
        .child(view().flex_1().min_w_0().h_full().flex_col().child(dynamic(
            move || -> AnyElement {
                match mode.get() {
                    crate::Mode::History => view()
                        .w_full()
                        .flex_1()
                        .min_h_0()
                        .flex_col()
                        .child(history_view::history(
                            &history.state,
                            &history,
                            tab_for_history.clone(),
                            &search_for_body,
                            &refs,
                        ))
                        .child(diff::diff(
                            &history.diff.state,
                            &history.diff,
                            language,
                            label.clone(),
                            patch.clone(),
                        ))
                        .into_element(),
                    crate::Mode::Agent => view().w_full().flex_1().min_h_0().into_element(),
                    crate::Mode::Blame => crate::features::history::blame_view(
                        language,
                        tab_for_history.clone(),
                        notice,
                    )
                    .into_element(),
                    crate::Mode::Refs => refs::view::refs_view(
                        &refs.state,
                        &refs,
                        language,
                        tab_for_changes.clone(),
                        &search,
                        refs_tab,
                    )
                    .into_element(),
                    crate::Mode::Changes => crate::features::changes::view::changes(
                        &changes.state,
                        &changes,
                        tab_for_changes.clone(),
                        group_by_directory,
                        tree_options,
                    )
                    .into_element(),
                }
            },
        )))
        .child(dynamic(move || -> AnyElement {
            match mode.get() {
                crate::Mode::History if inspector_visible => inspector::inspector(
                    &history_for_inspector.state,
                    &history_for_inspector,
                    tab_for_inspector.clone(),
                    language,
                    inspector_tab,
                )
                .into_element(),
                crate::Mode::Agent => {
                    crate::features::agent::view::agent_panel(&agent.state, &agent, language)
                        .into_element()
                }
                crate::Mode::Refs => view().w(px(0.0)).h_full().into_element(),
                crate::Mode::Changes => {
                    // 「请 Agent 起草」= 切到 Agent 主视图并提议一份计划。
                    let ask_agent: std::sync::Arc<dyn Fn()> = {
                        let agent = agent.clone();
                        std::sync::Arc::new(move || {
                            mode.set(crate::Mode::Agent);
                            agent.propose_plan();
                        })
                    };
                    crate::features::commit::view::commit_panel(
                        &commit_for_panel.state,
                        &commit_for_panel,
                        language,
                        tab_for_inspector.clone(),
                        ask_agent,
                    )
                    .into_element()
                }
                _ => view().w(px(0.0)).h_full().into_element(),
            }
        }));
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
