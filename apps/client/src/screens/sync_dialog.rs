//! 同步对话框（设计板 28 / 29）：Fetch / Pull / Push 的确认与结果。
//!
//! 显示仓库与远程、当前的 ahead/behind、即将执行的动作与说明；执行后留在对话框里
//! 展示 Git 的输出（成功或失败），可关闭。拉取只做快进，会在说明里写清楚。

use crate::design::{self, metrics};
use crate::i18n;
use crate::model::{Sync, Tab};
use nsh_core::prelude::*;

/// 对话框状态：种类 + 结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SyncDialogState {
    pub kind: Sync,
    /// 执行结果；`None` 表示还没执行。
    pub result: Option<Result<String, String>>,
    /// 正在执行。
    pub running: bool,
}

impl SyncDialogState {
    pub fn new(kind: Sync) -> SyncDialogState {
        return SyncDialogState {
            kind,
            result: None,
            running: false,
        };
    }
}

pub(crate) fn sync_dialog(
    language: Signal<SharedString>,
    tab: Memo<Option<Tab>>,
    state: Signal<Option<SyncDialogState>>,
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
            let run = on_run.clone();
            let close = on_close.clone();
            let close_again = on_close.clone();

            return view()
                .w(px(440.0))
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
                .child(
                    text(format!("{} · {}", repo_name, remote))
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
                                ("branch", branch.clone()),
                                ("ahead", ahead.to_string()),
                                ("behind", behind.to_string()),
                            ],
                        )
                    })
                    .text_size(px(11.0))
                    .line_height(px(15.0))
                    .text_color(move || design::text2()),
                )
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
                        .h(move || if has_result { px(60.0) } else { px(0.0) })
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
                                        .text_color(move || design::text2()),
                                )
                                .into_element();
                        })),
                )
                .child(
                    view()
                        .w_full()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(view().flex_1())
                        .child(crate::widgets::button(
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
                        }))
                        .child(dynamic(move || -> AnyElement {
                            if !has_result {
                                return view().w(px(0.0)).into_element();
                            }
                            return view()
                                .h(px(metrics::BUTTON_SMALL_H))
                                .px(px(8.0))
                                .items_center()
                                .rounded(px(5.0))
                                .border_1()
                                .border_color(move || design::btn_border())
                                .bg(move || design::btn())
                                .on_press({
                                    let close_again = close_again.clone();
                                    move || close_again()
                                })
                                .child(
                                    text(move || {
                                        i18n::t(language.get().as_str(), "sync.close")
                                    })
                                    .text_size(px(11.0))
                                    .line_height(px(14.0))
                                    .text_color(move || design::text()),
                                )
                                .into_element();
                        })),
                )
                .into_element();
        }));
}
