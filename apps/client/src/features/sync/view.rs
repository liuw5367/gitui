//! Fetch / Pull / Push 共用的同步 sheet（设计板 15 / 28 / 29）。
//!
//! 左侧放操作目标、选项与动作，右侧只显示 Git 子进程实际返回的输出和状态。

use super::commands::SyncCommands;
use super::state::{SyncState, SyncSummary, SyncTaskState};
use crate::app::session::{Sync, SyncOptions, Tab};
use crate::design::{self, metrics};
use crate::i18n;
use crate::ui;
use nsh_core::prelude::*;

/// 三个 tab 的显示顺序。
const KINDS: [Sync; 3] = [Sync::Fetch, Sync::Pull, Sync::Push];

pub(crate) fn sync_dialog(
    state: &SyncState,
    cmd: &SyncCommands,
    language: Signal<SharedString>,
    tabs: Signal<Vec<Tab>>,
    tab: Memo<Option<Tab>>,
) -> impl IntoElement + use<> {
    let dialog = state.dialog;
    let task = state.task;
    let set_kind: std::sync::Arc<dyn Fn(Sync)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |kind| cmd.open_sync(kind))
    };
    let set_option: std::sync::Arc<dyn Fn(&'static str, bool)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |key, value| cmd.set_sync_option(key, value))
    };
    let run_commands = cmd.clone();
    let on_cancel: std::sync::Arc<dyn Fn()> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move || cmd.cancel_sync())
    };
    let on_close: std::sync::Arc<dyn Fn()> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move || cmd.close_sync())
    };
    let dismiss = on_close.clone();

    return crate::ui::dialog::modal(
        move || dialog.get().is_some(),
        move || i18n::t(language.get().as_str(), "sync.title"),
        110,
        move || {
            let dialog = dialog;
            let tabs = tabs;
            let tab = tab;
            let language = language;
            let set_kind = set_kind.clone();
            let set_option = set_option.clone();
            let run_commands = run_commands.clone();
            let new_target = signal(String::new());
            let on_cancel = on_cancel.clone();
            let on_close = on_close.clone();

            return dynamic(move || -> AnyElement {
                let Some(dialog_state) = dialog.get() else {
                    return view().w(px(0.0)).h(px(0.0)).into_element();
                };
                let current_tab = match dialog_state.target_tab_id {
                    Some(id) => {
                        tabs.with(|tabs| tabs.iter().find(|candidate| candidate.id == id).cloned())
                    }
                    None => tab.get(),
                };
                let is_running = dialog_state.running;
                let result = dialog_state.result.clone();
                let target_tab_id = dialog_state.target_tab_id;
                let target_path = dialog_state
                    .target_path
                    .clone()
                    .or_else(|| current_tab.as_ref().map(|current| current.path.clone()));
                let kind = dialog_state.kind;
                let options = dialog_state.options.clone();
                let remote = dialog_state.remote.clone().unwrap_or_else(|| {
                    current_tab
                        .as_ref()
                        .and_then(|tab| tab.refs.remotes.first().cloned())
                        .unwrap_or_else(|| "origin".to_string())
                });
                let repo_name = current_tab
                    .as_ref()
                    .map(Tab::label)
                    .or_else(|| {
                        target_path.as_deref().and_then(|path| {
                            path.rsplit(['/', '\\'])
                                .find(|segment| !segment.is_empty())
                                .map(str::to_string)
                        })
                    })
                    .unwrap_or_default();
                let branch = current_tab
                    .as_ref()
                    .map(|tab| tab.branch.clone())
                    .unwrap_or_default();
                let ahead = current_tab.as_ref().map(|tab| tab.ahead).unwrap_or(0);
                let behind = current_tab.as_ref().map(|tab| tab.behind).unwrap_or(0);
                let remote_url = current_tab
                    .as_ref()
                    .and_then(|tab| {
                        tab.remote_urls
                            .iter()
                            .find(|(name, _)| name == &remote)
                            .map(|(_, url)| url.clone())
                    })
                    .unwrap_or_default();

                let set_kind = set_kind.clone();
                let set_option = set_option.clone();
                let local_branches = current_tab
                    .as_ref()
                    .map(|tab| {
                        tab.refs
                            .branches
                            .iter()
                            .map(|item| item.name.clone())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let mut destinations = current_tab
                    .as_ref()
                    .map(|tab| {
                        tab.refs
                            .remote_branches
                            .iter()
                            .filter(|item| item.remote == remote)
                            .map(|item| item.name.clone())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let push_pair = (kind == Sync::Push && !options.push.all_branches).then(|| {
                    options.push.branches.clone().unwrap_or_else(|| {
                        let source = if local_branches.contains(&branch) {
                            branch.clone()
                        } else {
                            local_branches.first().cloned().unwrap_or_default()
                        };
                        let destination = if destinations.contains(&source) {
                            source.clone()
                        } else {
                            String::new()
                        };
                        (source, destination)
                    })
                });
                if let Some((_, destination)) = &push_pair
                    && !destination.is_empty()
                    && !destinations.contains(destination)
                {
                    destinations.push(destination.clone());
                }
                let available_remotes = current_tab
                    .as_ref()
                    .map(|tab| tab.refs.remotes.clone())
                    .unwrap_or_default();
                let remote_branches = current_tab
                    .as_ref()
                    .map(|tab| tab.refs.remote_branches.clone())
                    .unwrap_or_default();
                const PUSH_LIST_HEIGHT: f32 = 224.0;
                let remote_field = (kind == Sync::Push).then(|| {
                    let entries = available_remotes
                        .into_iter()
                        .map(|name| {
                            ui::select::SelectOption::new(name.clone(), move || name.clone())
                        })
                        .collect();
                    let source = push_pair
                        .as_ref()
                        .map(|(source, _)| source.clone())
                        .unwrap_or_else(|| branch.clone());
                    view()
                        .w_full()
                        .flex_row()
                        .items_center()
                        .gap(px(ui::GAP))
                        .child(
                            view()
                                .flex_shrink_0()
                                .child(push_field_label(language, "sync.pushRemote")),
                        )
                        .child(
                            view()
                                .flex_1()
                                .min_w_0()
                                .child(ui::select::select_with_limit(
                                    entries,
                                    remote.clone(),
                                    None,
                                    is_running,
                                    Some(PUSH_LIST_HEIGHT),
                                    move |remote: String| {
                                        new_target.set(source.clone());
                                        dialog.update(|state| {
                                            let Some(state) =
                                                state.as_mut().filter(|state| !state.running)
                                            else {
                                                return;
                                            };
                                            let destination = remote_branches
                                                .iter()
                                                .find(|branch| {
                                                    branch.remote == remote && branch.name == source
                                                })
                                                .map(|branch| branch.name.clone())
                                                .unwrap_or_default();
                                            state.remote = Some(remote);
                                            state.options.push.branches =
                                                Some((source.clone(), destination));
                                            state.result = None;
                                            state.summary = None;
                                        });
                                    },
                                )),
                        )
                        .into_element()
                });
                let branch_fields = push_pair.as_ref().map(|(source, destination)| {
                    let source = source.clone();
                    let destination = destination.clone();
                    let next_destination = destination.clone();
                    let next_source = source.clone();
                    let set_pair = move |source, destination| {
                        dialog.update(|state| {
                            if let Some(state) = state.as_mut().filter(|state| !state.running) {
                                state.options.push.branches = Some((source, destination));
                            }
                        });
                    };
                    let dest_options = destinations
                        .iter()
                        .cloned()
                        .map(|name| {
                            ui::select::SelectOption::new(name.clone(), move || name.clone())
                        })
                        .chain(std::iter::once(ui::select::SelectOption::new(
                            String::new(),
                            move || i18n::t(language.get().as_str(), "sync.createTargetBranch"),
                        )))
                        .collect();
                    let source_options = local_branches
                        .iter()
                        .cloned()
                        .map(|name| {
                            ui::select::SelectOption::new(name.clone(), move || name.clone())
                        })
                        .collect();
                    let is_new = destination.is_empty();
                    view()
                        .w_full()
                        .flex_col()
                        .gap(px(ui::GAP_XS))
                        .child(
                            view()
                                .w_full()
                                .flex_row()
                                .gap(px(ui::GAP_SM))
                                .child(
                                    view()
                                        .flex_1()
                                        .min_w_0()
                                        .flex_col()
                                        .child(push_field_label(language, "sync.sourceBranch"))
                                        .child(ui::select::select_with_limit(
                                            source_options,
                                            source.clone(),
                                            None,
                                            is_running,
                                            Some(PUSH_LIST_HEIGHT),
                                            move |name| {
                                                new_target.set(name.clone());
                                                set_pair(name, next_destination.clone());
                                            },
                                        )),
                                )
                                .child(
                                    view()
                                        .flex_1()
                                        .min_w_0()
                                        .flex_col()
                                        .child(push_field_label(language, "sync.targetBranch"))
                                        .child(ui::select::select_with_limit(
                                            dest_options,
                                            destination,
                                            None,
                                            is_running,
                                            Some(PUSH_LIST_HEIGHT),
                                            move |name: String| {
                                                if name.is_empty() {
                                                    new_target.set(next_source.clone());
                                                }
                                                set_pair(next_source.clone(), name);
                                            },
                                        )),
                                ),
                        )
                        .when(is_new && !is_running, |field| {
                            if new_target.get_untracked().is_empty() {
                                new_target.set(source);
                            }
                            field.child(ui::text_input::text_input(
                                new_target,
                                move || i18n::t(language.get().as_str(), "sync.newTargetName"),
                                false,
                                move |value| new_target.set(value.to_string()),
                            ))
                        })
                        .into_element()
                });
                let branch_fields = remote_field.map(|remote| {
                    view()
                        .w_full()
                        .flex_col()
                        .gap(px(ui::GAP_XS))
                        .child(remote)
                        .child(branch_fields)
                        .into_element()
                });
                let missing_source = (kind == Sync::Push
                    && current_tab
                        .as_ref()
                        .is_none_or(|tab| tab.refs.remotes.is_empty()))
                    || push_pair.as_ref().is_some_and(|(source, _)| {
                        source.is_empty() || !local_branches.contains(source)
                    });
                let needs_name = push_pair
                    .as_ref()
                    .is_some_and(|(_, destination)| destination.is_empty());
                let run_commands = run_commands.clone();
                let on_run: std::sync::Arc<dyn Fn(Sync)> = std::sync::Arc::new(move |kind| {
                    run_commands.run_sync_to(kind, push_pair.clone(), new_target.get_untracked());
                });
                let on_cancel = on_cancel.clone();
                let on_close = on_close.clone();
                let log = log_panel(
                    language,
                    kind,
                    is_running,
                    task,
                    target_tab_id,
                    target_path.clone(),
                    dialog_state.remote.clone(),
                    result.clone(),
                );
                let left = action_panel(
                    language,
                    kind,
                    options.clone(),
                    is_running,
                    result.is_some(),
                    repo_name,
                    branch,
                    remote,
                    remote_url,
                    ahead,
                    behind,
                    set_option,
                    branch_fields,
                    move || missing_source || (needs_name && new_target.get().trim().is_empty()),
                    on_run,
                    on_cancel,
                );
                let header = sync_header(language, kind, is_running, set_kind, on_close.clone());
                let footer = sync_footer(
                    language,
                    is_running,
                    result,
                    ahead,
                    behind,
                    task,
                    target_tab_id,
                    target_path,
                    kind,
                    dialog_state.remote.clone(),
                    on_close,
                );

                return crate::ui::dialog::modality(
                    view()
                        .w(px(metrics::SYNC_SHEET_W))
                        .h(px(metrics::SYNC_SHEET_H))
                        .flex_col()
                        .rounded(px(metrics::RADIUS_PANEL + 2.0))
                        .border_1()
                        .border_color(move || design::line_strong())
                        .bg(move || design::token("popover"))
                        .shadow_lg()
                        .overflow_hidden()
                        .child(header)
                        .child(
                            view()
                                .w_full()
                                .h(px(metrics::SYNC_SHEET_H
                                    - metrics::SYNC_HEAD_H
                                    - metrics::SYNC_FOOT_H))
                                .flex_shrink_0()
                                .flex_row()
                                .overflow_hidden()
                                .child(left)
                                .child(log),
                        )
                        .child(footer),
                )
                .into_element();
            });
        },
        move || dismiss(),
    );
}

fn sync_header(
    language: Signal<SharedString>,
    kind: Sync,
    running: bool,
    set_kind: std::sync::Arc<dyn Fn(Sync)>,
    on_close: std::sync::Arc<dyn Fn()>,
) -> impl IntoElement {
    return view()
        .w_full()
        .h(px(metrics::SYNC_HEAD_H))
        .flex_shrink_0()
        .flex_row()
        .items_center()
        .gap(px(ui::GAP_LG))
        .px(px(ui::GAP_XL))
        .border_b_1()
        .border_color(move || design::line())
        .child(
            text(move || i18n::t(language.get().as_str(), "sync.title"))
                .text_size(px(ui::BODY))
                .line_height(px(ui::BODY_LINE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || design::text()),
        )
        .child(
            view()
                .h(px(28.0))
                .flex_row()
                .items_center()
                .gap(px(1.0))
                .p(px(2.0))
                .rounded(px(6.0))
                .border_1()
                .border_color(move || design::line_soft())
                .bg(move || design::panel2())
                .child(
                    KINDS
                        .iter()
                        .map(|choice| {
                            let choice = *choice;
                            let active = choice == kind;
                            let disabled = running && !active;
                            let set_kind = set_kind.clone();
                            let language = language;
                            return crate::ui::button::button(
                                move || i18n::t(language.get().as_str(), sync_kind_key(choice)),
                                if active {
                                    crate::ui::button::ButtonVariant::Secondary
                                } else {
                                    crate::ui::button::ButtonVariant::Ghost
                                },
                                crate::ui::button::ButtonSize::Small,
                                disabled,
                                move || set_kind(choice),
                            )
                            .into_element();
                        })
                        .collect::<Vec<AnyElement>>(),
                ),
        )
        .child(view().flex_1())
        .child(crate::ui::button::icon_button(
            "x",
            move || i18n::t(language.get().as_str(), "sync.close"),
            crate::ui::button::ButtonSize::Small,
            false,
            move || on_close(),
        ));
}

#[allow(clippy::too_many_arguments)]
fn action_panel(
    language: Signal<SharedString>,
    kind: Sync,
    options: SyncOptions,
    running: bool,
    finished: bool,
    repo_name: String,
    branch: String,
    remote: String,
    remote_url: String,
    ahead: u32,
    behind: u32,
    set_option: std::sync::Arc<dyn Fn(&'static str, bool)>,
    branch_fields: Option<AnyElement>,
    invalid_target: impl Fn() -> bool + 'static,
    on_run: std::sync::Arc<dyn Fn(Sync)>,
    on_cancel: std::sync::Arc<dyn Fn()>,
) -> impl IntoElement {
    let mut form = view()
        .w(px(metrics::SYNC_SHEET_W / 2.0 - 1.0))
        .h_full()
        .flex_col()
        .gap(px(ui::GAP))
        .p(px(ui::GAP_XL))
        .overflow_hidden()
        .child(
            text(move || i18n::t(language.get().as_str(), sync_kind_key(kind)))
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::SECONDARY_LINE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || design::text2()),
        )
        .child(
            view()
                .w_full()
                .flex_col()
                .gap(px(ui::GAP_XS))
                .child(
                    text(format!("{repo_name} · {remote}"))
                        .text_size(px(ui::BODY))
                        .line_height(px(ui::BODY_LINE))
                        .text_color(move || design::text()),
                )
                .child(
                    text(if remote_url.is_empty() {
                        branch.clone()
                    } else {
                        format!("{branch} · {remote_url}")
                    })
                    .text_size(px(ui::CODE))
                    .line_height(px(ui::CODE_LINE))
                    .text_color(move || design::text3())
                    .text_overflow(nsh_core::EllipsizeMode::Head),
                ),
        )
        .child(
            text(move || {
                i18n::with_arguments(
                    language.get().as_str(),
                    "sync.tracking",
                    &[
                        ("branch", branch.clone()),
                        ("ahead", ahead.to_string()),
                        ("behind", behind.to_string()),
                    ],
                )
            })
            .text_size(px(ui::SECONDARY))
            .line_height(px(ui::SECONDARY_LINE))
            .text_color(move || design::text2()),
        );

    if let Some(fields) = branch_fields {
        form = form.child(fields);
    }
    form = form.child(sync_options(
        language,
        kind,
        options.clone(),
        running,
        set_option,
    ));
    if options.is_destructive(kind) {
        form = form.child(
            text(move || i18n::t(language.get().as_str(), "sync.forceWarning"))
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::SECONDARY_LINE))
                .text_color(move || design::danger()),
        );
    }

    form = form.child(view().flex_1());
    let action_label_key = if running {
        "sync.running"
    } else if finished {
        "sync.retry"
    } else {
        sync_kind_key(kind)
    };
    form = form.child(
        view()
            .w_full()
            .flex_row()
            .items_center()
            .gap(px(ui::GAP_SM))
            .when(running, |row| {
                row.child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "sync.cancelAction"),
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Regular,
                    false,
                    move || on_cancel(),
                ))
            })
            .child(view().flex_1().child(crate::ui::button::button(
                move || i18n::t(language.get().as_str(), action_label_key),
                crate::ui::button::ButtonVariant::Primary,
                crate::ui::button::ButtonSize::Regular,
                move || running || invalid_target(),
                move || {
                    if !running {
                        on_run(kind);
                    }
                },
            ))),
    );
    form = form.child(
        text(move || i18n::t(language.get().as_str(), sync_hint_key(kind)))
            .text_size(px(ui::SECONDARY))
            .line_height(px(ui::SECONDARY_LINE))
            .text_color(move || design::text3()),
    );
    return form;
}

fn push_field_label(language: Signal<SharedString>, key: &'static str) -> impl IntoElement {
    text(move || i18n::t(language.get().as_str(), key))
        .text_size(px(ui::SECONDARY))
        .text_color(move || design::text2())
}

fn sync_options(
    language: Signal<SharedString>,
    kind: Sync,
    options: SyncOptions,
    disabled: bool,
    set_option: std::sync::Arc<dyn Fn(&'static str, bool)>,
) -> impl IntoElement {
    let mut rows = view()
        .w_full()
        .flex_col()
        .gap(px(ui::GAP_XS))
        .py(px(ui::GAP_SM))
        .border_t_1()
        .border_color(move || design::line_soft());
    if kind == Sync::Pull {
        rows = rows.child(strategy_row(
            language,
            options.clone(),
            disabled,
            set_option.clone(),
        ));
    }
    let choices: Vec<(&'static str, bool)> = match kind {
        Sync::Fetch => vec![
            ("sync.optPrune", options.fetch.prune),
            ("sync.optTags", options.fetch.tags),
            ("sync.optAllRemotes", options.fetch.all_remotes),
        ],
        Sync::Pull => vec![
            ("sync.optAutostash", options.pull.autostash),
            (
                "sync.optAutostashPop",
                options.pull.autostash && options.pull.autostash_pop,
            ),
            ("sync.optRebaseLocal", options.pull.rebase_local),
        ],
        Sync::Push => vec![
            ("sync.optPushTags", options.push.tags),
            ("sync.optForceLease", options.push.force_with_lease),
            ("sync.optAllBranches", options.push.all_branches),
        ],
    };
    for (key, checked) in choices {
        if key == "sync.optRebaseLocal" && options.pull.strategy != git::PullStrategy::Rebase {
            continue;
        }
        let option_id = match key {
            "sync.optPrune" => "prune",
            "sync.optTags" => "fetchTags",
            "sync.optAllRemotes" => "allRemotes",
            "sync.optAutostash" => "autostash",
            "sync.optAutostashPop" => "autostashPop",
            "sync.optRebaseLocal" => "rebaseLocal",
            "sync.optPushTags" => "pushTags",
            "sync.optForceLease" => "forceLease",
            "sync.optAllBranches" => "allBranches",
            _ => continue,
        };
        let set_option = set_option.clone();
        let row_disabled = disabled || (key == "sync.optAutostashPop" && !options.pull.autostash);
        rows = rows.child(option_row(
            language,
            key,
            checked,
            row_disabled,
            move |next| set_option(option_id, next),
        ));
    }
    return rows;
}

fn option_row(
    language: Signal<SharedString>,
    key: &'static str,
    checked: bool,
    disabled: bool,
    on_change: impl Fn(bool) + 'static,
) -> impl IntoElement {
    return crate::ui::check::check_row(
        move || i18n::t(language.get().as_str(), key),
        checked,
        disabled,
        move |next| on_change(next),
    );
}

fn strategy_row(
    language: Signal<SharedString>,
    options: SyncOptions,
    disabled: bool,
    set_option: std::sync::Arc<dyn Fn(&'static str, bool)>,
) -> impl IntoElement {
    let current = options.pull.strategy;
    return view()
        .w_full()
        .h(px(ui::ROW))
        .flex_row()
        .items_center()
        .gap(px(ui::GAP_XS))
        .child(
            text(move || i18n::t(language.get().as_str(), "sync.strategy"))
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::SECONDARY_LINE))
                .text_color(move || design::text2()),
        )
        .child(view().flex_1())
        .child(
            [PullChoice::Merge, PullChoice::Rebase, PullChoice::FfOnly]
                .iter()
                .map(|choice| {
                    let choice = *choice;
                    let active = choice.matches(current);
                    let set_option = set_option.clone();
                    return crate::ui::button::button(
                        {
                            let language = language;
                            move || i18n::t(language.get().as_str(), choice.key())
                        },
                        if active {
                            crate::ui::button::ButtonVariant::Secondary
                        } else {
                            crate::ui::button::ButtonVariant::Ghost
                        },
                        crate::ui::button::ButtonSize::Small,
                        disabled,
                        move || set_option(choice.option_id(), true),
                    )
                    .into_element();
                })
                .collect::<Vec<AnyElement>>(),
        );
}

fn log_panel(
    language: Signal<SharedString>,
    kind: Sync,
    running: bool,
    task: Signal<Option<SyncTaskState>>,
    target_tab_id: Option<u64>,
    target_path: Option<String>,
    target_remote: Option<String>,
    result: Option<Result<String, String>>,
) -> impl IntoElement {
    let status_result = result.clone();
    let status_task = task;
    let status_path = target_path.clone();
    let status_remote = target_remote.clone();
    let color_result = result.clone();
    let color_task = task;
    let color_path = target_path.clone();
    let color_remote = target_remote.clone();
    let log_result = result.clone();
    let log_task = task;
    let log_path = target_path.clone();
    let log_remote = target_remote.clone();
    let title_task = task;
    let title_path = target_path;
    let title_remote = target_remote;
    let title_language = language;
    let log_scroll = element_ref();
    let mut content = view()
        .w(px(metrics::SYNC_SHEET_W / 2.0 - 1.0))
        .h_full()
        .flex_col()
        .gap(px(ui::GAP))
        .p(px(ui::GAP_XL))
        .border_l_1()
        .border_color(move || design::line())
        .bg(move || design::panel())
        .child(dynamic(move || -> AnyElement {
            let task = title_task.get().filter(|task| {
                sync_task_matches(
                    task,
                    kind,
                    target_tab_id,
                    title_path.as_deref(),
                    title_remote.as_deref(),
                )
            });
            let progress = task
                .as_ref()
                .and_then(|task| task.workflow_progress)
                .filter(|progress| progress.label().is_some());
            let title = text(i18n::t(
                title_language.get().as_str(),
                sync_log_title_key(kind),
            ))
            .text_size(px(ui::SECONDARY))
            .line_height(px(ui::SECONDARY_LINE))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(move || design::text2());
            let Some(progress) = progress else {
                return title.into_element();
            };
            let color = if progress.running {
                design::warn()
            } else if task
                .as_ref()
                .is_some_and(|task| task.result.as_ref().is_some_and(Result::is_ok))
            {
                design::ok()
            } else {
                design::danger()
            };
            return view()
                .w_full()
                .flex_row()
                .items_center()
                .gap(px(ui::GAP_SM))
                .child(view().w(px(6.0)).h(px(6.0)).rounded(px(3.0)).bg(color))
                .child(title.flex_1().min_w_0())
                .child(
                    text(progress.label().unwrap_or_default())
                        .text_size(px(ui::SECONDARY))
                        .font_family(ui::MONO)
                        .text_color(move || design::text3()),
                )
                .into_element();
        }))
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .gap(px(ui::GAP_SM))
                .child(dynamic(move || -> AnyElement {
                    if running {
                        return crate::ui::icon::icon_view(
                            "refresh",
                            crate::ui::icon::IconSize::Button,
                            design::accent,
                        )
                        .into_element();
                    }
                    return view().w(px(0.0)).h(px(0.0)).into_element();
                }))
                .child(
                    text(move || {
                        let cancelled = status_task.get().is_some_and(|task| {
                            sync_task_matches(
                                &task,
                                kind,
                                target_tab_id,
                                status_path.as_deref(),
                                status_remote.as_deref(),
                            ) && task.cancelled
                        });
                        let lang = language.get();
                        if let Some(summary) = status_task
                            .get()
                            .filter(|task| {
                                sync_task_matches(
                                    task,
                                    kind,
                                    target_tab_id,
                                    status_path.as_deref(),
                                    status_remote.as_deref(),
                                )
                            })
                            .and_then(|task| task.summary)
                        {
                            return sync_summary_text(
                                lang.as_str(),
                                &summary,
                                status_result.as_ref().is_some_and(Result::is_err),
                            );
                        }
                        let key = if cancelled {
                            "sync.cancelled"
                        } else if running {
                            "sync.runningDetail"
                        } else if status_result.as_ref().is_some_and(Result::is_ok) {
                            "sync.ok"
                        } else if status_result.as_ref().is_some_and(Result::is_err) {
                            "sync.failed"
                        } else {
                            "sync.ready"
                        };
                        i18n::t(lang.as_str(), key)
                    })
                    .flex_1()
                    .min_w_0()
                    .text_size(px(ui::BODY))
                    .line_height(px(ui::BODY_LINE))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(move || {
                        let cancelled = color_task.get().is_some_and(|task| {
                            sync_task_matches(
                                &task,
                                kind,
                                target_tab_id,
                                color_path.as_deref(),
                                color_remote.as_deref(),
                            ) && task.cancelled
                        });
                        if color_result.as_ref().is_some_and(Result::is_err) && !cancelled {
                            design::danger()
                        } else if color_result.is_some() && !cancelled {
                            design::ok()
                        } else {
                            design::text()
                        }
                    }),
                ),
        )
        .when(running, |panel| {
            panel.child(crate::ui::feedback::progress(None))
        })
        .child(
            scroll()
                .element_ref(&log_scroll)
                .w_full()
                .flex_1()
                .min_h_0()
                .rounded(px(6.0))
                .border_1()
                .border_color(move || design::line_soft())
                .bg(move || design::content())
                .p(px(ui::GAP))
                .child(dynamic(move || -> AnyElement {
                    let output = log_task
                        .get()
                        .filter(|task| {
                            sync_task_matches(
                                task,
                                kind,
                                target_tab_id,
                                log_path.as_deref(),
                                log_remote.as_deref(),
                            )
                        })
                        .map(|task| task.progress)
                        .unwrap_or_default();
                    let Some(output) = sync_log_output(&output, log_result.as_ref()) else {
                        let key = if log_result.is_some() {
                            "sync.emptyOutput"
                        } else {
                            "sync.noOutput"
                        };
                        return text(move || i18n::t(language.get().as_str(), key))
                            .text_size(px(ui::SECONDARY))
                            .line_height(px(ui::SECONDARY_LINE))
                            .text_color(move || design::text3())
                            .into_element();
                    };
                    log_scroll.scroll_to_end(false);
                    return text(output)
                        .text_size(px(ui::CODE))
                        .line_height(px(ui::CODE_LINE))
                        .font_family(ui::MONO)
                        .text_color(move || design::text2())
                        .into_element();
                })),
        );
    content = content.child(
        text(move || {
            i18n::t(
                language.get().as_str(),
                if running {
                    "sync.background"
                } else {
                    "sync.logHint"
                },
            )
        })
        .text_size(px(ui::SECONDARY))
        .line_height(px(ui::SECONDARY_LINE))
        .text_color(move || design::text3()),
    );
    return content;
}

fn sync_log_output(progress: &str, result: Option<&Result<String, String>>) -> Option<String> {
    if !progress.trim().is_empty() {
        return Some(progress.to_string());
    }
    return result.and_then(|result| {
        let message = match result {
            Ok(message) | Err(message) => message,
        };
        (!message.trim().is_empty()).then(|| message.clone())
    });
}

pub(crate) fn sync_summary_text(language: &str, summary: &SyncSummary, failed: bool) -> String {
    let count = |value: usize| value.to_string();
    match summary {
        SyncSummary::Fetch {
            target,
            updated_refs,
            deleted_refs,
            commits,
        } => {
            let all_remotes = target == "*";
            let target = if all_remotes {
                i18n::t(language, "sync.allRemotes")
            } else {
                target.clone()
            };
            let args = [
                ("target", target),
                ("updated", count(*updated_refs)),
                ("deleted", count(*deleted_refs)),
                (
                    "commits",
                    commits
                        .map(count)
                        .unwrap_or_else(|| i18n::t(language, "sync.unknownCount")),
                ),
            ];
            let key = if failed {
                if commits.is_some() {
                    "sync.fetchPartialFailure"
                } else {
                    "sync.fetchPartialFailureUnknown"
                }
            } else if *updated_refs == 0 && *deleted_refs == 0 {
                if all_remotes {
                    "sync.fetchAllUpToDate"
                } else {
                    "sync.fetchUpToDate"
                }
            } else if commits.is_some() {
                "sync.fetchUpdated"
            } else {
                "sync.fetchUpdatedUnknown"
            };
            return i18n::with_arguments(language, key, &args);
        }
        SyncSummary::Pull { target, commits } => {
            let args = [
                ("target", target.clone()),
                (
                    "commits",
                    commits
                        .map(count)
                        .unwrap_or_else(|| i18n::t(language, "sync.unknownCount")),
                ),
            ];
            let key = match commits {
                Some(0) => "sync.pullUpToDate",
                Some(_) => "sync.pullUpdated",
                None => "sync.pullUpdatedUnknown",
            };
            return i18n::with_arguments(language, key, &args);
        }
        SyncSummary::Push {
            target,
            updated_refs,
            deleted_refs,
            commits,
            rejected_refs,
        } => {
            let args = [
                ("target", target.clone()),
                ("updated", count(*updated_refs)),
                ("deleted", count(*deleted_refs)),
                (
                    "commits",
                    commits
                        .map(count)
                        .unwrap_or_else(|| i18n::t(language, "sync.unknownCount")),
                ),
                ("rejected", count(*rejected_refs)),
            ];
            let key = if failed {
                if commits.is_some() {
                    "sync.pushPartialFailure"
                } else {
                    "sync.pushPartialFailureUnknown"
                }
            } else if *updated_refs == 0 && *deleted_refs == 0 && *rejected_refs == 0 {
                "sync.pushUpToDate"
            } else if commits.is_some() {
                "sync.pushUpdated"
            } else {
                "sync.pushUpdatedUnknown"
            };
            return i18n::with_arguments(language, key, &args);
        }
        SyncSummary::Unknown { kind, target } => {
            let target = if target == "*" {
                i18n::t(language, "sync.allRemotes")
            } else {
                target.clone()
            };
            let args = [("kind", i18n::t(language, kind.id())), ("target", target)];
            return i18n::with_arguments(language, "sync.completedUnknown", &args);
        }
    }
}

fn sync_task_matches(
    task: &SyncTaskState,
    kind: Sync,
    target_tab_id: Option<u64>,
    target_path: Option<&str>,
    target_remote: Option<&str>,
) -> bool {
    return task.kind == kind
        && target_tab_id == Some(task.target_tab_id)
        && target_path == Some(task.path.as_str())
        && target_remote == task.remote.as_deref();
}

fn sync_log_title_key(kind: Sync) -> &'static str {
    return match kind {
        Sync::Fetch => "sync.remoteRefs",
        Sync::Pull => "sync.incoming",
        Sync::Push => "sync.activity",
    };
}

fn sync_footer(
    language: Signal<SharedString>,
    running: bool,
    result: Option<Result<String, String>>,
    ahead: u32,
    behind: u32,
    task: Signal<Option<SyncTaskState>>,
    target_tab_id: Option<u64>,
    target_path: Option<String>,
    kind: Sync,
    target_remote: Option<String>,
    on_close: std::sync::Arc<dyn Fn()>,
) -> impl IntoElement {
    let status_task = task;
    let status_path = target_path.clone();
    let status_remote = target_remote.clone();
    let color_task = task;
    let color_path = target_path;
    let color_remote = target_remote;
    let color_result = result.clone();
    return view()
        .w_full()
        .h(px(metrics::SYNC_FOOT_H))
        .flex_shrink_0()
        .flex_row()
        .items_center()
        .gap(px(ui::GAP))
        .px(px(ui::GAP_XL))
        .border_t_1()
        .border_color(move || design::line())
        .child(
            text(move || {
                let cancelled = status_task.get().is_some_and(|task| {
                    sync_task_matches(
                        &task,
                        kind,
                        target_tab_id,
                        status_path.as_deref(),
                        status_remote.as_deref(),
                    ) && task.cancelled
                });
                if cancelled {
                    return i18n::t(language.get().as_str(), "sync.cancelled");
                }
                if running {
                    i18n::t(language.get().as_str(), "sync.background")
                } else {
                    i18n::with_arguments(
                        language.get().as_str(),
                        "sync.trackingSummary",
                        &[("ahead", ahead.to_string()), ("behind", behind.to_string())],
                    )
                }
            })
            .flex_1()
            .min_w_0()
            .text_size(px(ui::SECONDARY))
            .line_height(px(ui::SECONDARY_LINE))
            .text_color(move || {
                let cancelled = color_task.get().is_some_and(|task| {
                    sync_task_matches(
                        &task,
                        kind,
                        target_tab_id,
                        color_path.as_deref(),
                        color_remote.as_deref(),
                    ) && task.cancelled
                });
                if color_result.as_ref().is_some_and(Result::is_err) && !cancelled {
                    design::danger()
                } else {
                    design::text3()
                }
            })
            .text_overflow(nsh_core::EllipsizeMode::Tail),
        )
        .child(crate::ui::button::button(
            move || i18n::t(language.get().as_str(), "sync.close"),
            crate::ui::button::ButtonVariant::Secondary,
            crate::ui::button::ButtonSize::Regular,
            false,
            move || on_close(),
        ));
}

fn sync_kind_key(kind: Sync) -> &'static str {
    return match kind {
        Sync::Fetch => "sync.fetch",
        Sync::Pull => "sync.pull",
        Sync::Push => "sync.push",
    };
}

#[cfg(test)]
mod log_panel_tests {
    use super::{SyncTaskState, sync_log_output, sync_summary_text, sync_task_matches};
    use crate::app::session::{Sync, SyncOptions};
    use crate::features::sync::state::SyncSummary;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    fn task(kind: Sync, remote: Option<&str>) -> SyncTaskState {
        return SyncTaskState {
            kind,
            target_tab_id: 7,
            path: "/repo".to_string(),
            remote: remote.map(str::to_string),
            options: SyncOptions::default(),
            progress: "previous operation output".to_string(),
            result: None,
            summary: None,
            cancelled: false,
            show_dialog: true,
            workflow_progress: None,
            follow_up_push: None,
            retry_push: false,
            cancel: Arc::new(AtomicBool::new(false)),
        };
    }

    #[test]
    fn summary_shows_fetch_ref_and_commit_counts() {
        let text = sync_summary_text(
            "zh-Hans",
            &SyncSummary::Fetch {
                target: "origin".to_string(),
                updated_refs: 2,
                deleted_refs: 1,
                commits: Some(5),
            },
            false,
        );

        assert!(text.contains("origin"));
        assert!(text.contains("2"));
        assert!(text.contains("1"));
        assert!(text.contains("5"));
    }

    #[test]
    fn partial_push_summary_keeps_rejected_count_and_unknown_commit_count() {
        let text = sync_summary_text(
            "zh-Hans",
            &SyncSummary::Push {
                target: "origin/main".to_string(),
                updated_refs: 1,
                deleted_refs: 0,
                commits: None,
                rejected_refs: 2,
            },
            true,
        );

        assert!(text.contains("origin/main"));
        assert!(text.contains("1"));
        assert!(text.contains("2"));
        assert!(text.contains("未确认"));
    }

    #[test]
    fn progress_output_takes_precedence_over_completion_summary() {
        let result = Ok("Fetch completed".to_string());

        assert_eq!(
            sync_log_output("Receiving objects: 100%", Some(&result)),
            Some("Receiving objects: 100%".to_string())
        );
    }

    #[test]
    fn completion_summary_fills_an_empty_progress_panel() {
        let result = Ok("Fetch completed".to_string());

        assert_eq!(
            sync_log_output("", Some(&result)),
            Some("Fetch completed".to_string())
        );
    }

    #[test]
    fn empty_result_has_no_log_text() {
        let result = Ok(String::new());

        assert_eq!(sync_log_output("", Some(&result)), None);
    }

    #[test]
    fn task_output_is_hidden_after_switching_sync_kind() {
        let task = task(Sync::Fetch, Some("origin"));

        assert!(!sync_task_matches(
            &task,
            Sync::Pull,
            Some(7),
            Some("/repo"),
            Some("origin"),
        ));
    }

    #[test]
    fn task_output_is_hidden_after_switching_remote() {
        let task = task(Sync::Fetch, Some("origin"));

        assert!(!sync_task_matches(
            &task,
            Sync::Fetch,
            Some(7),
            Some("/repo"),
            Some("upstream"),
        ));
    }

    #[test]
    fn task_output_matches_its_kind_and_target() {
        let task = task(Sync::Fetch, Some("origin"));

        assert!(sync_task_matches(
            &task,
            Sync::Fetch,
            Some(7),
            Some("/repo"),
            Some("origin"),
        ));
    }
}

fn sync_hint_key(kind: Sync) -> &'static str {
    return match kind {
        Sync::Fetch => "sync.fetchHint",
        Sync::Pull => "sync.pullHint",
        Sync::Push => "sync.pushHint",
    };
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PullChoice {
    Merge,
    Rebase,
    FfOnly,
}

impl PullChoice {
    fn matches(self, strategy: git::PullStrategy) -> bool {
        return match self {
            PullChoice::Merge => strategy == git::PullStrategy::Merge,
            PullChoice::Rebase => strategy == git::PullStrategy::Rebase,
            PullChoice::FfOnly => strategy == git::PullStrategy::FfOnly,
        };
    }

    fn key(self) -> &'static str {
        return match self {
            PullChoice::Merge => "sync.merge",
            PullChoice::Rebase => "sync.rebase",
            PullChoice::FfOnly => "sync.ffOnly",
        };
    }

    fn option_id(self) -> &'static str {
        return match self {
            PullChoice::Merge => "strategyMerge",
            PullChoice::Rebase => "strategyRebase",
            PullChoice::FfOnly => "strategyFfOnly",
        };
    }
}
