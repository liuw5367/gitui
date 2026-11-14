//! 提交面板（设计板 10 右侧 `insp`）：Summary / Description、暂存清单、钩子输出与
//! 提交按钮。
//!
//! 钩子失败时显示 `hook-box`：状态、耗时占位（0.5.0 没有钩子级的计时）、命令输出，
//! 以及重试提示。提交按钮在说明为空或没有暂存内容时禁用。

use crate::design::{self, metrics};
use crate::i18n;
use crate::model::Tab;
use crate::widgets;
use crate::Actions;
use nsh_core::prelude::*;

/// 面板宽度（与提交文件面板一致）。
pub(crate) const WIDTH: f32 = 320.0;

pub(crate) fn commit_panel(
    language: Signal<SharedString>,
    tab: Tab,
    summary: Signal<String>,
    description: Signal<String>,
    actions: Actions,
) -> impl IntoElement {
    let staged: Vec<_> = tab
        .changes
        .iter()
        .filter(|entry| entry.staged)
        .cloned()
        .collect();
    let staged_count = staged.len();
    let unstaged_count = tab.changes.len() - staged_count;
    let hook_log = tab.hook_log.clone();
    let hook_ok = tab.hook_ok;
    let commit = actions.commit.clone();
    let can_commit = staged_count > 0;
    let mut staged_rows: Vec<AnyElement> = Vec::new();
    for entry in &staged {
        let name = entry.path.rsplit('/').next().unwrap_or(&entry.path).to_string();
        let status = entry.status_letter();
        let additions = entry.additions;
        let deletions = entry.deletions;
        staged_rows.push(
            view()
                .w_full()
                .h(px(20.0))
                .px(px(6.0))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(name)
                        .flex_1()
                        .text_size(px(10.0))
                        .font_family("SF Mono")
                        .text_color(move || design::text())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .child(
                    text(status.to_string())
                        .text_size(px(9.0))
                        .font_family("SF Mono")
                        .text_color(move || design::warn()),
                )
                .child(
                    text(format!("+{}", additions))
                        .text_size(px(9.0))
                        .text_color(move || design::add_fg()),
                )
                .child(
                    text(format!("−{}", deletions))
                        .text_size(px(9.0))
                        .text_color(move || design::del_fg()),
                )
                .into_element(),
        );
    }
    return view()
        .w(px(WIDTH))
        .h_full()
        .flex_shrink_0()
        .flex_col()
        .bg(move || design::panel())
        .border_l_1()
        .border_color(move || design::line())
        .child(
            view()
                .w_full()
                .h(px(26.0))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .px(px(12.0))
                .border_b_1()
                .border_color(move || design::line_soft())
                .child(
                    text(move || i18n::t(language.get().as_str(), "commit.tab"))
                        .text_size(px(10.5))
                        .font_weight(FontWeight(600))
                        .text_color(move || design::text()),
                )
                .child(
                    text(tab.changes.len().to_string())
                        .text_size(px(9.5))
                        .text_color(move || design::text3()),
                ),
        )
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .p(px(12.0))
                .flex_col()
                .gap(px(8.0))
                .child(scroll().w_full().flex_1().child(
                    view()
                        .w_full()
                        .flex_col()
                        .gap(px(8.0))
                        .child(
                            text(move || {
                                i18n::with_arguments(
                                    language.get().as_str(),
                                    "changes.summary",
                                    &[
                                        ("staged", staged_count.to_string()),
                                        ("unstaged", unstaged_count.to_string()),
                                    ],
                                )
                            })
                            .text_size(px(9.5))
                            .text_color(move || design::text3()),
                        )
                        .child(field_label(language, "commit.summary", "commit.required"))
                        .child(
                            view()
                                .w_full()
                                .h(px(29.0))
                                .flex_row()
                                .items_center()
                                .px(px(8.0))
                                .rounded(px(metrics::RADIUS_CONTROL))
                                .border_1()
                                .border_color(move || design::field_border())
                                .bg(move || design::field())
                                .child(
                                    nsh_base::input()
                                        .value(summary)
                                        .on_change(move |text: &str| {
                                            summary.set(text.to_string())
                                        })
                                        .placeholder(move || {
                                            i18n::t(
                                                language.get().as_str(),
                                                "commit.summaryPlaceholder",
                                            )
                                        })
                                        .text_size(px(12.0))
                                        .flex_1(),
                                ),
                        )
                        .child(field_label(language, "commit.description", "commit.markdown"))
                        .child(
                            view()
                                .w_full()
                                .min_h(px(76.0))
                                .p(px(6.0))
                                .rounded(px(metrics::RADIUS_CONTROL))
                                .border_1()
                                .border_color(move || design::field_border())
                                .bg(move || design::field())
                                .child(
                                    nsh_base::input()
                                        .value(description)
                                        .on_change(move |text: &str| {
                                            description.set(text.to_string())
                                        })
                                        .placeholder(move || {
                                            i18n::t(
                                                language.get().as_str(),
                                                "commit.descriptionPlaceholder",
                                            )
                                        })
                                        .text_size(px(11.0))
                                        .flex_1(),
                                ),
                        )
                        .child(
                            view()
                                .w_full()
                                .flex_col()
                                .rounded(px(metrics::RADIUS_CONTROL))
                                .border_1()
                                .border_color(move || design::line())
                                .bg(move || design::content())
                                .child(
                                    view()
                                        .w_full()
                                        .h(px(20.0))
                                        .px(px(6.0))
                                        .items_center()
                                        .child(
                                            text(move || {
                                                i18n::with_arguments(
                                                    language.get().as_str(),
                                                    "commit.stagedFiles",
                                                    &[("count", staged_count.to_string())],
                                                )
                                            })
                                            .text_size(px(9.5))
                                            .text_color(move || design::text3()),
                                        ),
                                )
                                .child(staged_rows),
                        )
                        .child(dynamic(move || -> AnyElement {
                            let Some(log) = hook_log.clone() else {
                                return view().w_full().h(px(0.0)).into_element();
                            };
                            return hook_box(language, log, hook_ok).into_element();
                        })),
                )),
        )
        .child(
            view()
                .w_full()
                .p(px(12.0))
                .flex_col()
                .gap(px(4.0))
                .border_t_1()
                .border_color(move || design::line())
                .child(widgets::primary_button(
                    move || {
                        i18n::with_arguments(
                            language.get().as_str(),
                            "commit.action",
                            &[("count", staged_count.to_string())],
                        )
                    },
                    {
                        let commit = commit.clone();
                        move || {
                            if can_commit {
                                commit();
                            }
                        }
                    },
                ))
                .child(
                    text(move || {
                        if can_commit {
                            i18n::t(language.get().as_str(), "commit.hooksHint")
                        } else {
                            i18n::t(language.get().as_str(), "commit.nothingStaged")
                        }
                    })
                    .text_size(px(9.5))
                    .text_color(move || design::text3()),
                ),
        );
}

fn field_label(
    language: Signal<SharedString>,
    key: &'static str,
    hint_key: &'static str,
) -> impl IntoElement {
    return view()
        .w_full()
        .flex_row()
        .items_center()
        .gap(px(5.0))
        .child(
            text(move || i18n::t(language.get().as_str(), key))
                .text_size(px(10.5))
                .font_weight(FontWeight(600))
                .text_color(move || design::text2()),
        )
        .child(
            text(move || i18n::t(language.get().as_str(), hint_key))
                .text_size(px(9.5))
                .text_color(move || design::text3()),
        );
}

/// 钩子输出块：成功绿点、失败红点，命令输出等宽显示。
fn hook_box(language: Signal<SharedString>, log: String, ok: bool) -> impl IntoElement {
    let has_log = !log.trim().is_empty();
    return view()
        .w_full()
        .flex_col()
        .gap(px(4.0))
        .p(px(6.0))
        .rounded(px(metrics::RADIUS_CONTROL))
        .border_1()
        .border_color(move || {
            if ok {
                design::line()
            } else {
                design::danger()
            }
        })
        .bg(move || design::content())
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .gap(px(5.0))
                .child(
                    view()
                        .w(px(6.0))
                        .h(px(6.0))
                        .rounded(px(3.0))
                        .bg(move || {
                            if ok {
                                design::ok()
                            } else {
                                design::danger()
                            }
                        }),
                )
                .child(
                    text(move || {
                        i18n::t(
                            language.get().as_str(),
                            if ok {
                                "commit.hooksPassed"
                            } else {
                                "commit.hooksFailed"
                            },
                        )
                    })
                    .text_size(px(10.0))
                    .text_color(move || design::text2()),
                ),
        )
        .child(
            view().w_full().when(has_log, |this| {
                this.child(
                    text(log)
                        .text_size(px(9.5))
                        .line_height(px(14.0))
                        .font_family("SF Mono")
                        .text_color(move || design::text2()),
                )
            }),
        );
}
