//! 工作区：标题栏 + 工具栏 + 侧栏 + History/diff（+ 提交文件面板）+ 状态栏。
//!
//! diff 区显示三种内容之一：选中提交的补丁、提交内单个文件的补丁、两个提交的区间
//! 补丁。打开失败时在正文区显示错误面板。

use crate::design;
use crate::i18n;
use crate::model;
use crate::model::Tab;
use crate::shell;
use crate::workspace::{commit, diff, history, inspector, sidebar};
use crate::Actions;
use nsh_core::prelude::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn workspace(
    language: Signal<SharedString>,
    tabs: Signal<Vec<Tab>>,
    active: Signal<usize>,
    history_query: Signal<String>,
    filter_reference: Signal<String>,
    filter_author: Signal<String>,
    filter_when: Signal<SharedString>,
    filter_open: Signal<bool>,
    inspector_open: Signal<bool>,
    compare_mode: Signal<bool>,
    mode: Signal<crate::Mode>,
    commit_summary: Signal<String>,
    commit_description: Signal<String>,
    changelist_open: Signal<bool>,
    settings_state: Signal<store::Settings>,
    agent_plan: Signal<Option<agent::Plan>>,
    agent_audit: Signal<Vec<agent::AuditEntry>>,
    agent_policies: Signal<agent::Policies>,
    reset_request: Signal<Option<(String, git::ResetMode)>>,
    notice: Signal<Option<String>>,
    reflog_query: Signal<String>,
    diff_options: Signal<crate::workspace::diff::DiffOptions>,
    diff_split: Signal<bool>,
    actions: Actions,
) -> impl IntoElement {
    let current = computed(move || tabs.get().get(active.get()).cloned());
    let change_count = move || {
        return current
            .get()
            .map(|tab| tab.status.len() as u32)
            .unwrap_or(0);
    };
    let actions_for_body = actions.clone();
    return view()
        .w_full()
        .h_full()
        .flex_col()
        .bg(move || design::content())
        .child(shell::titlebar(
            language,
            tabs,
            active,
            actions.clone(),
        ))
        .child(shell::toolbar(
            language,
            history_query,
            change_count,
            mode,
            actions.set_mode.clone(),
            actions.open_sync.clone(),
            actions.set_mode_refs.clone(),
            actions.set_mode_agent.clone(),
            inspector_open,
        ))
        .child(shell::state_banner(
            language,
            current,
            actions.clone(),
        ))
        .child(shell::op_banner(
            language,
            current,
            actions.clone(),
        ))
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
                    history_query,
                    filter_reference,
                    filter_author,
                    filter_when,
                    filter_open,
                    inspector_open,
                    compare_mode,
                    mode,
                    commit_summary,
                    commit_description,
                    changelist_open,
                    settings_state,
                    agent_plan,
                    agent_audit,
                    agent_policies,
                    reset_request,
                    reflog_query,
                    diff_options,
                    diff_split,
                    actions_for_body,
                )),
        )
        .child(shell::statusbar(language, current, notice));
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
fn match_body(
    language: Signal<SharedString>,
    current: Memo<Option<Tab>>,
    history_query: Signal<String>,
    filter_reference: Signal<String>,
    filter_author: Signal<String>,
    filter_when: Signal<SharedString>,
    filter_open: Signal<bool>,
    inspector_open: Signal<bool>,
    compare_mode: Signal<bool>,
    mode: Signal<crate::Mode>,
    commit_summary: Signal<String>,
    commit_description: Signal<String>,
    changelist_open: Signal<bool>,
    settings_state: Signal<store::Settings>,
    agent_plan: Signal<Option<agent::Plan>>,
    agent_audit: Signal<Vec<agent::AuditEntry>>,
    agent_policies: Signal<agent::Policies>,
    reset_request: Signal<Option<(String, git::ResetMode)>>,
    reflog_query: Signal<String>,
    diff_options: Signal<crate::workspace::diff::DiffOptions>,
    diff_split: Signal<bool>,
    actions: Actions,
) -> impl IntoElement {
    return view().w_full().h_full().flex_row().child(dynamic(
        move || -> AnyElement {
            let tab = current.get();
            let has_error = tab
                .as_ref()
                .map(|tab| tab.error.is_some())
                .unwrap_or(false);
            if has_error {
                let detail = tab
                    .and_then(|tab| tab.error)
                    .unwrap_or_else(|| i18n::t(language.get().as_str(), "error.title"));
                let actions = actions.clone();
                return error_panel(language, detail, actions).into_element();
            }
            let Some(tab) = tab else {
                return view()
                    .w_full()
                    .h_full()
                    .items_center()
                    .justify_center()
                    .child(
                        text(move || i18n::t(language.get().as_str(), "history.empty"))
                            .text_size(px(12.0))
                            .text_color(move || design::text3()),
                    )
                    .into_element();
            };
            let actions = actions.clone();
            return body(
                language,
                tab,
                history_query,
                filter_reference,
                filter_author,
                filter_when,
                filter_open,
                inspector_open,
                compare_mode,
                mode,
                commit_summary,
                commit_description,
                changelist_open,
                settings_state,
                agent_plan,
                agent_audit,
                agent_policies,
                reset_request,
                reflog_query,
                diff_options,
                diff_split,
                actions,
            )
            .into_element();
        },
    ));
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
fn body(
    language: Signal<SharedString>,
    tab: Tab,
    history_query: Signal<String>,
    filter_reference: Signal<String>,
    filter_author: Signal<String>,
    filter_when: Signal<SharedString>,
    filter_open: Signal<bool>,
    inspector_open: Signal<bool>,
    compare_mode: Signal<bool>,
    mode: Signal<crate::Mode>,
    commit_summary: Signal<String>,
    commit_description: Signal<String>,
    changelist_open: Signal<bool>,
    settings_state: Signal<store::Settings>,
    agent_plan: Signal<Option<agent::Plan>>,
    agent_audit: Signal<Vec<agent::AuditEntry>>,
    agent_policies: Signal<agent::Policies>,
    reset_request: Signal<Option<(String, git::ResetMode)>>,
    reflog_query: Signal<String>,
    diff_options: Signal<crate::workspace::diff::DiffOptions>,
    diff_split: Signal<bool>,
    actions: Actions,
) -> impl IntoElement {
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
            (
                i18n::with_arguments(
                    language.get_untracked().as_str(),
                    "compare.range",
                    &[("from", from_short), ("to", to_short)],
                ),
                model::compare_patch(&tab).unwrap_or_default(),
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
    let set_inspector_tab = {
        let inspector_tab = inspector_tab;
        std::sync::Arc::new(move |next: inspector::InspectorTab| inspector_tab.set(next))
    };
    let refs_tab = signal(crate::screens::RefsTab::Refs);
    let set_refs_tab = std::sync::Arc::new(move |next: crate::screens::RefsTab| refs_tab.set(next));
    let group_by_directory = settings_state.get_untracked().file_tree;
    let changes_actions = actions.clone();
    let history_actions = actions.clone();
    let inspector_actions = actions.clone();
    let commit_actions = actions.clone();
    return view()
        .w_full()
        .h_full()
        .flex_row()
        .child(sidebar::sidebar(language, tab_for_sidebar, actions.clone()))
        .child(
            view()
                .flex_1()
                .min_w_0()
                .h_full()
                .flex_col()
                .child(dynamic(move || -> AnyElement {
                    match mode.get() {
                        crate::Mode::History => view()
                            .w_full()
                            .flex_1()
                            .min_h_0()
                            .flex_col()
                            .child(history::history(
                                language,
                                tab_for_history.clone(),
                                history_query,
                                filter_reference,
                                filter_author,
                                filter_when,
                                filter_open,
                                compare_mode,
                                reset_request,
                                history_actions.clone(),
                            ))
                            .child(diff::diff(
                                language,
                                label.clone(),
                                patch.clone(),
                                history_actions.copy_text.clone(),
                                diff_options.get(),
                                history_actions.toggle_diff_option.clone(),
                                history_actions.export_patch.clone(),
                                diff_split.get(),
                                history_actions.set_diff_split.clone(),
                            ))
                            .into_element(),
                        crate::Mode::Agent => view()
                            .w_full()
                            .flex_1()
                            .min_h_0()
                            .into_element(),
                        crate::Mode::Refs => crate::screens::refs_view(
                            language,
                            tab_for_changes.clone(),
                            reflog_query,
                            refs_tab,
                            set_refs_tab.clone(),
                            changes_actions.clone(),
                        )
                        .into_element(),
                        crate::Mode::Changes => crate::screens::changes_view(
                            language,
                            tab_for_changes.clone(),
                            changelist_open,
                            group_by_directory,
                            changes_actions.clone(),
                        )
                        .into_element(),
                    }
                })),
        )
        .child(dynamic(move || -> AnyElement {
            match mode.get() {
                crate::Mode::History if inspector_visible => inspector::inspector(
                    language,
                    tab_for_inspector.clone(),
                    inspector_tab,
                    set_inspector_tab.clone(),
                    inspector_actions.clone(),
                )
                .into_element(),
                crate::Mode::Agent => crate::workspace::agent::agent_panel(
                    language,
                    agent_plan,
                    agent_audit,
                    agent_policies,
                    actions.clone(),
                )
                .into_element(),
                crate::Mode::Refs => view().w(px(0.0)).h_full().into_element(),
                crate::Mode::Changes => commit::commit_panel(
                    language,
                    tab_for_inspector.clone(),
                    commit_summary,
                    commit_description,
                    commit_actions.clone(),
                )
                .into_element(),
                _ => view().w(px(0.0)).h_full().into_element(),
            }
        }));
}

fn error_panel(
    language: Signal<SharedString>,
    detail: String,
    actions: Actions,
) -> impl IntoElement {
    let detail = std::sync::Arc::new(detail);
    let retry = actions.refresh.clone();
    let welcome = actions.go_welcome.clone();
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
                .text_size(px(13.0))
                .font_weight(FontWeight(600))
                .text_color(move || design::text()),
        )
        .child(
            text({
                let detail = detail.clone();
                move || detail.as_str().to_string()
            })
            .text_size(px(11.0))
            .text_color(move || design::text2()),
        )
        .child(
            view()
                .flex_row()
                .gap(px(8.0))
                .child(crate::widgets::button(
                    move || i18n::t(language.get().as_str(), "error.retry"),
                    {
                        let retry = retry.clone();
                        move || retry()
                    },
                ))
                .child(crate::widgets::button(
                    move || i18n::t(language.get().as_str(), "error.close"),
                    {
                        let welcome = welcome.clone();
                        move || welcome()
                    },
                )),
        );
}
