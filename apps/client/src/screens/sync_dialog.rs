//! 同步面板（设计板 15 / 28 / 29）：Fetch / Pull / Push 三个 tab，各带选项。
//!
//! 顶部三 tab 切换动作，中部是动作专属选项，底部执行并就地展示 Git 输出。
//! 选项随动作切换保留在同一个状态里，不会因为来回切 tab 丢失。

use crate::design::{self, metrics};
use crate::i18n;
use crate::model::{Sync, SyncOptions, Tab};
use crate::widgets;
use nsh_core::prelude::*;

/// 对话框状态：种类 + 选项 + 结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SyncDialogState {
    pub kind: Sync,
    pub options: SyncOptions,
    /// 执行结果；`None` 表示还没执行。
    pub result: Option<Result<String, String>>,
    /// 正在执行。
    pub running: bool,
}

impl SyncDialogState {
    pub fn new(kind: Sync) -> SyncDialogState {
        return SyncDialogState {
            kind,
            options: SyncOptions::default(),
            result: None,
            running: false,
        };
    }
}

/// 三个 tab 的显示顺序。
const KINDS: [Sync; 3] = [Sync::Fetch, Sync::Pull, Sync::Push];

pub(crate) fn sync_dialog(
    language: Signal<SharedString>,
    tab: Memo<Option<Tab>>,
    state: Signal<Option<SyncDialogState>>,
    set_kind: std::sync::Arc<dyn Fn(Sync)>,
    set_option: std::sync::Arc<dyn Fn(&'static str, bool)>,
    on_run: std::sync::Arc<dyn Fn(Sync)>,
    on_close: std::sync::Arc<dyn Fn()>,
) -> impl IntoElement {
    return view()
        .absolute()
        .inset(px(0.0))
        .z_index(110)
        .items_center()
        .justify_center()
        .bg(move || {
            if state.get().is_some() {
                design::token("overlay")
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .on_press({
            let on_close = on_close.clone();
            move || on_close()
        })
        .child(dynamic(move || -> AnyElement {
            let Some(dialog) = state.get() else {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            };
            let current = tab.get();
            let repo_name = current
                .as_ref()
                .map(|tab| tab.label())
                .unwrap_or_default();
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

            return view()
                .w(px(460.0))
                .p(px(16.0))
                .flex_col()
                .gap(px(10.0))
                .rounded(px(metrics::RADIUS_PANEL))
                .border_1()
                .border_color(move || design::line())
                .bg(move || design::token("popover"))
                .on_press(|| {})
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
                    .text_size(px(13.5))
                    .font_weight(FontWeight(600))
                    .line_height(px(18.0))
                    .text_color(move || design::text()),
                )
                // 三 tab：切换动作，不丢已选项。
                .child(
                    view()
                        .h(px(26.0))
                        .flex_row()
                        .items_center()
                        .gap(px(2.0))
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
                                    return view()
                                        .h(px(20.0))
                                        .px(px(9.0))
                                        .items_center()
                                        .rounded(px(5.0))
                                        .bg(move || {
                                            if active {
                                                design::content()
                                            } else {
                                                AdaptiveColor::TRANSPARENT
                                            }
                                        })
                                        .on_press(move || set_kind(which))
                                        .child(
                                            text(move || {
                                                i18n::t(
                                                    language.get().as_str(),
                                                    match which {
                                                        Sync::Fetch => "sync.fetch",
                                                        Sync::Pull => "sync.pull",
                                                        Sync::Push => "sync.push",
                                                    },
                                                )
                                            })
                                            .text_size(px(11.0))
                                            .line_height(px(14.0))
                                            .text_color(move || {
                                                if active {
                                                    design::text()
                                                } else {
                                                    design::text2()
                                                }
                                            }),
                                        )
                                        .into_element();
                                })
                                .collect::<Vec<AnyElement>>(),
                        ),
                )
                .child(
                    text(format!("{} · {} · {}", repo_name, remote, branch))
                        .text_size(px(10.5))
                        .font_family("SF Mono")
                        .line_height(px(14.0))
                        .text_color(move || design::text3()),
                )
                .child(
                    text(move || {
                        i18n::with_arguments(
                            language.get().as_str(),
                            "sync.tracking",
                            &[
                                ("ahead", ahead.to_string()),
                                ("behind", behind.to_string()),
                            ],
                        )
                    })
                    .text_size(px(11.0))
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
                        .child(option_row(
                            language,
                            "sync.optTags",
                            options.fetch.tags,
                            {
                                let set_option = set_option.clone();
                                move || set_option("fetchTags", !options.fetch.tags)
                            },
                        ))
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
                                move || {
                                    set_option("forceLease", !options.push.force_with_lease)
                                }
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
                    .text_size(px(10.0))
                    .line_height(px(15.0))
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
                                    .text_size(px(10.0))
                                    .font_weight(FontWeight(600))
                                    .line_height(px(13.0))
                                    .text_color(move || {
                                        if ok {
                                            design::ok()
                                        } else {
                                            design::danger()
                                        }
                                    }),
                                )
                                .child(
                                    text(output)
                                        .text_size(px(9.5))
                                        .font_family("SF Mono")
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
                        text(move || {
                            i18n::t(language.get().as_str(), "sync.forceWarning")
                        })
                        .text_size(px(10.0))
                        .line_height(px(14.0))
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
                        .child(widgets::button(
                            move || {
                                i18n::t(
                                    language.get().as_str(),
                                    if has_result { "sync.close" } else { "common.cancel" },
                                )
                            },
                            {
                                let close = close.clone();
                                move || close()
                            },
                        ))
                        .child(dynamic(move || -> AnyElement {
                            if has_result {
                                return view().w(px(0.0)).into_element();
                            }
                            return crate::widgets::primary_button(
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
                .into_element();
        }));
}

/// 一行复选：12px 方框加标签。
fn option_row(
    language: Signal<SharedString>,
    key: &'static str,
    checked: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    return view()
        .w_full()
        .h(px(20.0))
        .flex_row()
        .items_center()
        .gap(px(7.0))
        .on_press(on_press)
        .child(
            view()
                .w(px(12.0))
                .h(px(12.0))
                .items_center()
                .justify_center()
                .rounded(px(3.0))
                .border_1()
                .border_color(move || design::line_strong())
                .bg(move || {
                    if checked {
                        design::accent()
                    } else {
                        AdaptiveColor::TRANSPARENT
                    }
                })
                .child(
                    text(move || {
                        if checked {
                            "✓".to_string()
                        } else {
                            String::new()
                        }
                    })
                    .text_size(px(8.0))
                    .text_color(move || design::content()),
                ),
        )
        .child(
            text(move || i18n::t(language.get().as_str(), key))
                .text_size(px(10.5))
                .line_height(px(14.0))
                .text_color(move || design::text2()),
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
                .text_size(px(10.5))
                .line_height(px(14.0))
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
                        .h(px(18.0))
                        .px(px(7.0))
                        .items_center()
                        .rounded(px(4.0))
                        .bg(move || {
                            if active {
                                design::content()
                            } else {
                                AdaptiveColor::TRANSPARENT
                            }
                        })
                        .border_1()
                        .border_color(move || {
                            if active {
                                design::line_soft()
                            } else {
                                AdaptiveColor::TRANSPARENT
                            }
                        })
                        .on_press(move || set_option(choice.option_id(), true))
                        .child(
                            text(move || i18n::t(language.get().as_str(), choice.key()))
                                .text_size(px(10.0))
                                .line_height(px(13.0))
                                .text_color(move || {
                                    if active {
                                        design::text()
                                    } else {
                                        design::text3()
                                    }
                                }),
                        )
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
