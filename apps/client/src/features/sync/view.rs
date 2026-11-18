//! Fetch / Pull / Push 共用的同步 sheet（设计板 15 / 28 / 29）。
//!
//! 左侧放操作目标、选项与动作，右侧只显示 Git 子进程实际返回的输出和状态。

use super::commands::SyncCommands;
use super::state::{SyncState, SyncTaskState};
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
    let on_run: std::sync::Arc<dyn Fn(Sync)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |kind| cmd.run_sync(kind))
    };
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
            let on_run = on_run.clone();
            let on_cancel = on_cancel.clone();
            let on_close = on_close.clone();

            return dynamic(move || -> AnyElement {
                let Some(dialog_state) = dialog.get() else {
                    return view().w(px(0.0)).h(px(0.0)).into_element();
                };
                let current_tab = match dialog_state.target_tab_id {
                    Some(id) => tabs.get().into_iter().find(|candidate| candidate.id == id),
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
                let on_run = on_run.clone();
                let on_cancel = on_cancel.clone();
                let on_close = on_close.clone();
                let log = log_panel(
                    language,
                    kind,
                    is_running,
                    task,
                    target_tab_id,
                    target_path.clone(),
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
                    on_close,
                );

                return crate::ui::dialog::modality(
                    view()
                        .w(px(metrics::SYNC_SHEET_W))
                        .h(px(metrics::SYNC_SHEET_H))
                        .flex_col()
                        .rounded(px(12.0))
                        .border_1()
                        .border_color(move || design::line())
                        .bg(move || design::content())
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
        .px(px(14.0))
        .bg(move || design::panel())
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
    on_run: std::sync::Arc<dyn Fn(Sync)>,
    on_cancel: std::sync::Arc<dyn Fn()>,
) -> impl IntoElement {
    let mut form = view()
        .w(px(metrics::SYNC_SHEET_W / 2.0 - 1.0))
        .h_full()
        .flex_col()
        .gap(px(ui::GAP))
        .p(px(14.0))
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
                running,
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
            ("sync.optRebaseLocal", options.pull.rebase_local),
        ],
        Sync::Push => vec![
            ("sync.optPushTags", options.push.tags),
            ("sync.optForceLease", options.push.force_with_lease),
            ("sync.optAllBranches", options.push.all_branches),
        ],
    };
    for (key, checked) in choices {
        let option_id = match key {
            "sync.optPrune" => "prune",
            "sync.optTags" => "fetchTags",
            "sync.optAllRemotes" => "allRemotes",
            "sync.optAutostash" => "autostash",
            "sync.optRebaseLocal" => "rebaseLocal",
            "sync.optPushTags" => "pushTags",
            "sync.optForceLease" => "forceLease",
            "sync.optAllBranches" => "allBranches",
            _ => continue,
        };
        let set_option = set_option.clone();
        let row_disabled = disabled
            || (key == "sync.optRebaseLocal" && options.pull.strategy != git::PullStrategy::Rebase);
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
    result: Option<Result<String, String>>,
) -> impl IntoElement {
    let status_result = result.clone();
    let status_task = task;
    let status_path = target_path.clone();
    let color_result = result.clone();
    let color_task = task;
    let color_path = target_path.clone();
    let log_task = task;
    let log_path = target_path;
    let mut content = view()
        .w(px(metrics::SYNC_SHEET_W / 2.0 - 1.0))
        .h_full()
        .flex_col()
        .gap(px(ui::GAP))
        .p(px(14.0))
        .border_l_1()
        .border_color(move || design::line())
        .bg(move || design::panel())
        .child(
            text(move || i18n::t(language.get().as_str(), sync_log_title_key(kind)))
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::SECONDARY_LINE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || design::text2()),
        )
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
                            target_tab_id == Some(task.target_tab_id)
                                && status_path.as_deref() == Some(task.path.as_str())
                                && task.cancelled
                        });
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
                        i18n::t(language.get().as_str(), key)
                    })
                    .text_size(px(ui::BODY))
                    .line_height(px(ui::BODY_LINE))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(move || {
                        let cancelled = color_task.get().is_some_and(|task| {
                            target_tab_id == Some(task.target_tab_id)
                                && color_path.as_deref() == Some(task.path.as_str())
                                && task.cancelled
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
                            target_tab_id == Some(task.target_tab_id)
                                && log_path.as_deref() == Some(task.path.as_str())
                        })
                        .map(|task| task.progress)
                        .unwrap_or_default();
                    if output.trim().is_empty() {
                        return text(move || i18n::t(language.get().as_str(), "sync.noOutput"))
                            .text_size(px(ui::SECONDARY))
                            .line_height(px(ui::SECONDARY_LINE))
                            .text_color(move || design::text3())
                            .into_element();
                    }
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
    on_close: std::sync::Arc<dyn Fn()>,
) -> impl IntoElement {
    let status_task = task;
    let status_path = target_path.clone();
    let color_task = task;
    let color_path = target_path;
    let color_result = result.clone();
    return view()
        .w_full()
        .h(px(metrics::SYNC_FOOT_H))
        .flex_shrink_0()
        .flex_row()
        .items_center()
        .gap(px(ui::GAP))
        .px(px(14.0))
        .border_t_1()
        .border_color(move || design::line())
        .bg(move || design::panel())
        .child(
            text(move || {
                let cancelled = status_task.get().is_some_and(|task| {
                    target_tab_id == Some(task.target_tab_id)
                        && status_path.as_deref() == Some(task.path.as_str())
                        && task.cancelled
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
                    target_tab_id == Some(task.target_tab_id)
                        && color_path.as_deref() == Some(task.path.as_str())
                        && task.cancelled
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
            crate::ui::button::ButtonSize::Small,
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
