//! 同步面板（设计板 15 / 28 / 29）：Fetch / Pull / Push 三个 tab，各带选项。
//!
//! 顶部三 tab 切换动作，中部是动作专属选项，底部执行并就地展示 Git 输出。
//! 选项随动作切换保留在同一个状态里，不会因为来回切 tab 丢失。

use super::commands::SyncCommands;
use super::state::SyncState;
use crate::app::session::{Sync, SyncOptions, Tab};
use crate::design::{self, metrics};
use crate::i18n;
use nsh_core::prelude::*;

/// 三个 tab 的显示顺序。
const KINDS: [Sync; 3] = [Sync::Fetch, Sync::Pull, Sync::Push];

/// 入口 `sync_dialog` 收本功能的状态与用例；语言与当前仓库单独传。
pub(crate) fn sync_dialog(
    state: &SyncState,
    cmd: &SyncCommands,
    language: Signal<SharedString>,
    tab: Memo<Option<Tab>>,
) -> impl IntoElement + use<> {
    let state = state.dialog;
    // 视图回调要 `Arc<dyn Fn>`，这里把本功能的命令包一层。
    let set_kind: std::sync::Arc<dyn Fn(Sync)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |kind: Sync| cmd.open_sync(kind))
    };
    let set_option: std::sync::Arc<dyn Fn(&'static str, bool)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |key: &'static str, value: bool| cmd.set_sync_option(key, value))
    };
    let on_run: std::sync::Arc<dyn Fn(Sync)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |kind: Sync| cmd.run_sync(kind))
    };
    let on_close: std::sync::Arc<dyn Fn()> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move || cmd.close_sync())
    };
    let dismiss = on_close.clone();
    return crate::ui::dialog::scrim(
        move || state.get().is_some(),
        110,
        dynamic(move || -> AnyElement {
            let Some(dialog) = state.get() else {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            };
            let current = tab.get();
            let repo_name = current.as_ref().map(|tab| tab.label()).unwrap_or_default();
            let branch = current
                .as_ref()
                .map(|tab| tab.branch.clone())
                .unwrap_or_default();
            let ahead = current.as_ref().map(|tab| tab.ahead).unwrap_or(0);
            let behind = current.as_ref().map(|tab| tab.behind).unwrap_or(0);
            let remote = current
                .as_ref()
                .and_then(|tab| tab.refs.remotes.first().cloned())
                .unwrap_or_else(|| "origin".to_string());
            let kind = dialog.kind;
            let running = dialog.running;
            let has_result = dialog.result.is_some();
            let result = dialog.result.clone();
            let options = dialog.options.clone();
            let run = on_run.clone();
            let close = on_close.clone();
            let set_kind = set_kind.clone();
            let set_option = set_option.clone();

            return crate::ui::dialog::modality(
                view()
                    .w(px(metrics::SYNC_SHEET_W))
                    .max_w(px(760.0))
                    .h(px(metrics::SYNC_SHEET_H))
                    .flex_col()
                    .rounded(px(12.0))
                    .border_1()
                    .border_color(move || design::line())
                    .bg(move || design::content())
                    .overflow_hidden()
                    .child(
                        view()
                            .w_full()
                            .h(px(metrics::SYNC_HEAD_H))
                            .flex_shrink_0()
                            .px(px(14.0))
                            .flex_row()
                            .items_center()
                            .gap(px(10.0))
                            .bg(move || design::panel())
                            .border_b_1()
                            .border_color(move || design::line())
                            .child(
                                text(move || {
                                    i18n::t(
                                        language.get().as_str(),
                                        match kind {
                                            Sync::Fetch => "sync.fetchTitle",
                                            Sync::Pull => "sync.pullTitle",
                                            Sync::Push => "sync.pushTitle",
                                        },
                                    )
                                })
                                .text_size(px(crate::ui::BODY))
                                .font_weight(FontWeight(600))
                                .line_height(px(17.0))
                                .text_color(move || design::text()),
                            ),
                    )
                    // 三 tab：切换动作，不丢已选项。
                    .child(
                        view()
                            .h(px(26.0))
                            .flex_shrink_0()
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
                                    .map(|which| {
                                        let which = *which;
                                        let active = which == kind;
                                        let set_kind = set_kind.clone();
                                        let language = language;
                                        let variant = if active {
                                            crate::ui::button::ButtonVariant::Secondary
                                        } else {
                                            crate::ui::button::ButtonVariant::Ghost
                                        };
                                        return crate::ui::button::button(
                                            {
                                                let language = language;
                                                move || {
                                                    i18n::t(
                                                        language.get().as_str(),
                                                        match which {
                                                            Sync::Fetch => "sync.fetch",
                                                            Sync::Pull => "sync.pull",
                                                            Sync::Push => "sync.push",
                                                        },
                                                    )
                                                }
                                            },
                                            variant,
                                            crate::ui::button::ButtonSize::Small,
                                            false,
                                            move || set_kind(which),
                                        )
                                        .into_element();
                                    })
                                    .collect::<Vec<AnyElement>>(),
                            ),
                    )
                    .child(
                        text(format!("{} · {} · {}", repo_name, remote, branch))
                            .text_size(px(crate::ui::CODE))
                            .line_height(px(14.0))
                            .text_color(move || design::text3()),
                    )
                    .child(
                        text(move || {
                            i18n::with_arguments(
                                language.get().as_str(),
                                "sync.tracking",
                                &[("ahead", ahead.to_string()), ("behind", behind.to_string())],
                            )
                        })
                        .text_size(px(crate::ui::BODY))
                        .line_height(px(15.0))
                        .text_color(move || design::text2()),
                    )
                    // 动作专属选项。
                    .child(match kind {
                        Sync::Fetch => view()
                            .w_full()
                            .flex_col()
                            .gap(px(2.0))
                            .child(option_row(
                                language,
                                "sync.optPrune",
                                options.fetch.prune,
                                {
                                    let set_option = set_option.clone();
                                    move || set_option("prune", !options.fetch.prune)
                                },
                            ))
                            .child(option_row(language, "sync.optTags", options.fetch.tags, {
                                let set_option = set_option.clone();
                                move || set_option("fetchTags", !options.fetch.tags)
                            }))
                            .child(option_row(
                                language,
                                "sync.optAllRemotes",
                                options.fetch.all_remotes,
                                {
                                    let set_option = set_option.clone();
                                    move || set_option("allRemotes", !options.fetch.all_remotes)
                                },
                            ))
                            .into_element(),
                        Sync::Pull => view()
                            .w_full()
                            .flex_col()
                            .gap(px(2.0))
                            .child(strategy_row(language, options.clone(), set_option.clone()))
                            .child(option_row(
                                language,
                                "sync.optAutostash",
                                options.pull.autostash,
                                {
                                    let set_option = set_option.clone();
                                    move || set_option("autostash", !options.pull.autostash)
                                },
                            ))
                            .child(option_row(
                                language,
                                "sync.optRebaseLocal",
                                options.pull.rebase_local,
                                {
                                    let set_option = set_option.clone();
                                    move || set_option("rebaseLocal", !options.pull.rebase_local)
                                },
                            ))
                            .into_element(),
                        Sync::Push => view()
                            .w_full()
                            .flex_col()
                            .gap(px(2.0))
                            .child(option_row(
                                language,
                                "sync.optPushTags",
                                options.push.tags,
                                {
                                    let set_option = set_option.clone();
                                    move || set_option("pushTags", !options.push.tags)
                                },
                            ))
                            .child(option_row(
                                language,
                                "sync.optForceLease",
                                options.push.force_with_lease,
                                {
                                    let set_option = set_option.clone();
                                    move || set_option("forceLease", !options.push.force_with_lease)
                                },
                            ))
                            .child(option_row(
                                language,
                                "sync.optAllBranches",
                                options.push.all_branches,
                                {
                                    let set_option = set_option.clone();
                                    move || set_option("allBranches", !options.push.all_branches)
                                },
                            ))
                            .into_element(),
                    })
                    .child(
                        text(move || {
                            i18n::t(
                                language.get().as_str(),
                                match kind {
                                    Sync::Fetch => "sync.fetchHint",
                                    Sync::Pull => "sync.pullHint",
                                    Sync::Push => "sync.pushHint",
                                },
                            )
                        })
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(crate::ui::SECONDARY_LINE))
                        .text_color(move || design::text3()),
                    )
                    .child(
                        view()
                            .w_full()
                            .h(move || if has_result { px(72.0) } else { px(0.0) })
                            .overflow_hidden()
                            .child(dynamic(move || -> AnyElement {
                                let Some(result) = result.clone() else {
                                    return view().w_full().h(px(0.0)).into_element();
                                };
                                let (ok, output) = match result {
                                    Ok(message) => (true, message),
                                    Err(detail) => (false, detail),
                                };
                                let output = if output.trim().is_empty() {
                                    i18n::t(language.get_untracked().as_str(), "sync.done")
                                } else {
                                    output
                                };
                                return view()
                                    .w_full()
                                    .flex_col()
                                    .gap(px(3.0))
                                    .child(
                                        text(move || {
                                            i18n::t(
                                                language.get().as_str(),
                                                if ok { "sync.ok" } else { "sync.failed" },
                                            )
                                        })
                                        .text_size(px(crate::ui::SECONDARY))
                                        .font_weight(FontWeight(600))
                                        .line_height(px(13.0))
                                        .text_color(
                                            move || {
                                                if ok { design::ok() } else { design::danger() }
                                            },
                                        ),
                                    )
                                    .child(
                                        text(output)
                                            .text_size(px(crate::ui::CODE))
                                            .line_height(px(13.0))
                                            .text_color(move || design::text2())
                                            .text_overflow(nsh_core::EllipsizeMode::Tail),
                                    )
                                    .into_element();
                            })),
                    )
                    // 强推会覆盖远端：在按钮上方单独用红色说明，不藏进选项里。
                    .child(view().w_full().when(options.is_destructive(kind), |this| {
                        this.child(
                            text(move || i18n::t(language.get().as_str(), "sync.forceWarning"))
                                .text_size(px(crate::ui::SECONDARY))
                                .line_height(px(crate::ui::SECONDARY_LINE))
                                .text_color(move || design::danger()),
                        )
                    }))
                    .child(
                        view()
                            .w_full()
                            .flex_row()
                            .items_center()
                            .gap(px(8.0))
                            .child(view().flex_1())
                            .child(crate::ui::button::button(
                                move || {
                                    i18n::t(
                                        language.get().as_str(),
                                        if has_result {
                                            "sync.close"
                                        } else {
                                            "common.cancel"
                                        },
                                    )
                                },
                                crate::ui::button::ButtonVariant::Secondary,
                                crate::ui::button::ButtonSize::Regular,
                                false,
                                {
                                    let close = close.clone();
                                    move || close()
                                },
                            ))
                            .child(dynamic(move || -> AnyElement {
                                if has_result {
                                    return view().w(px(0.0)).into_element();
                                }
                                return crate::ui::button::button(
                                    move || {
                                        i18n::t(
                                            language.get().as_str(),
                                            if running {
                                                "sync.running"
                                            } else {
                                                match kind {
                                                    Sync::Fetch => "sync.fetch",
                                                    Sync::Pull => "sync.pull",
                                                    Sync::Push => "sync.push",
                                                }
                                            },
                                        )
                                    },
                                    crate::ui::button::ButtonVariant::Primary,
                                    crate::ui::button::ButtonSize::Regular,
                                    false,
                                    {
                                        let run = run.clone();
                                        move || {
                                            if !running {
                                                run(kind)
                                            }
                                        }
                                    },
                                )
                                .into_element();
                            })),
                    )
                    .into_element(),
            )
            .into_element();
        }),
        move || dismiss(),
    );
}

/// 一行复选：由 ui::check_row 提供方框、选中态与键盘操作。
fn option_row(
    language: Signal<SharedString>,
    key: &'static str,
    checked: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    return crate::ui::check::check_row(
        move || i18n::t(language.get().as_str(), key),
        checked,
        false,
        move |_next: bool| on_press(),
    );
}

/// 拉取策略三选一（设计板 29）。
fn strategy_row(
    language: Signal<SharedString>,
    options: SyncOptions,
    set_option: std::sync::Arc<dyn Fn(&'static str, bool)>,
) -> impl IntoElement {
    let current = options.pull.strategy;
    return view()
        .w_full()
        .h(px(24.0))
        .flex_row()
        .items_center()
        .gap(px(4.0))
        .child(
            text(move || i18n::t(language.get().as_str(), "sync.strategy"))
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
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
                    let language = language;
                    return view()
                        .child(crate::ui::button::button(
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
                            false,
                            {
                                let set_option = set_option.clone();
                                move || set_option(choice.option_id(), true)
                            },
                        ))
                        .into_element();
                })
                .collect::<Vec<AnyElement>>(),
        );
}

/// 三选一的枚举包装：界面只关心 id 与文案。
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

    /// 复用同一个「设置选项」入口：值为真表示选中这个策略。
    fn option_id(self) -> &'static str {
        return match self {
            PullChoice::Merge => "strategyMerge",
            PullChoice::Rebase => "strategyRebase",
            PullChoice::FfOnly => "strategyFfOnly",
        };
    }
}
