//! 提交面板（设计板 10 右侧 `insp`）：Summary / Description、暂存清单、操作反馈与
//! 提交按钮。
//!
//! 提交、钩子或提交后推送的结果显示在可关闭状态块中。提交按钮在说明为空或没有暂存内容时禁用。

use super::commands::CommitCommands;
use super::state::CommitState;
use crate::app::session::{CommitFeedbackKind, Tab};
use crate::design::{self, metrics};
use crate::features::changes::commands::ChangesCommands;
use crate::i18n;
use nsh_core::prelude::*;

/// 提交面板的初始宽度（可由工作区分栏拖拽调整）。
pub(crate) const WIDTH: f32 = 360.0;

/// Changes 使用独立的紧凑提交区；Refs 保留原提交检查器布局。
pub(crate) fn commit_panel(
    state: &CommitState,
    cmd: &CommitCommands,
    changes: Option<ChangesCommands>,
    language: Signal<SharedString>,
    tab: Tab,
    show_tree: bool,
    tree: crate::app::session::TreeOptions,
    ask_agent: Option<std::sync::Arc<dyn Fn()>>,
) -> AnyElement {
    if let Some(changes) = changes {
        return changes_commit_panel(state, cmd, changes, language, tab, show_tree, tree)
            .into_element();
    }
    return refs_commit_panel(
        state,
        cmd,
        None,
        language,
        tab,
        show_tree,
        tree,
        ask_agent.unwrap_or_else(|| std::sync::Arc::new(|| {})),
    )
    .into_element();
}

fn refs_commit_panel(
    state: &CommitState,
    cmd: &CommitCommands,
    changes: Option<ChangesCommands>,
    language: Signal<SharedString>,
    tab: Tab,
    show_tree: bool,
    tree: crate::app::session::TreeOptions,
    ask_agent: std::sync::Arc<dyn Fn()>,
) -> impl IntoElement {
    let summary = state.summary;
    let description = state.description;
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
    let hook_kind = tab.commit_feedback_kind;
    let commit = {
        let cmd = cmd.clone();
        move || cmd.commit()
    };
    let amend = tab.amend;
    let signoff = tab.signoff;
    let push_after = tab.push_after_commit;
    let force_push_after = tab.force_push_after_commit;
    let hook_cmd = cmd.clone();
    let force_cmd = cmd.clone();
    let ask_agent_header = ask_agent.clone();
    // 「改写上一条」时不需要暂存内容：只改提交说明也是合法操作。
    let can_commit = staged_count > 0 || amend;
    return view()
        .w_full()
        .min_w_0()
        .h_full()
        .flex_col()
        .bg(move || design::panel())
        .child(
            view()
                .w_full()
                .h(px(33.0))
                .flex_row()
                .items_center()
                .gap(px(7.0))
                .px(px(14.0))
                .border_b_1()
                .border_color(move || design::line_soft())
                .child(
                    text(move || i18n::t(language.get().as_str(), "commit.tab"))
                        .text_size(px(crate::ui::SECONDARY))
                        .font_weight(FontWeight(600))
                        .text_color(move || design::text()),
                )
                .child(
                    view()
                        .px(px(4.0))
                        .rounded(px(4.0))
                        .bg(move || design::panel2())
                        .child(
                            text(tab.changes.len().to_string())
                                .text_size(px(crate::ui::META))
                                .text_color(move || design::text2()),
                        ),
                )
                .child(view().flex_1())
                .child(crate::ui::button::button_icon(
                    || "Agent".to_string(),
                    "sparkle",
                    crate::ui::button::ButtonVariant::Ghost,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || ask_agent_header(),
                )),
        )
        .child(dynamic(move || -> AnyElement {
            let Some(changes) = changes.clone() else {
                return view().w_full().h(px(0.0)).into_element();
            };
            return crate::features::changes::view::changelists_panel(
                &changes,
                tab.clone(),
                show_tree,
                tree,
            );
        }))
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .p(px(12.0))
                .flex_col()
                .gap(px(8.0))
                .child(
                    scroll().w_full().flex_1().child(
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
                                .text_size(px(crate::ui::SECONDARY))
                                .text_color(move || design::text3()),
                            )
                            .child(field_label(language, "commit.summary", "commit.required"))
                            .child(crate::ui::text_input::text_input(
                                summary,
                                move || {
                                    i18n::t(language.get().as_str(), "commit.summaryPlaceholder")
                                },
                                false,
                                move |text| summary.set(text.to_string()),
                            ))
                            .child(field_label(
                                language,
                                "commit.description",
                                "commit.markdown",
                            ))
                            .child(crate::ui::text_input::text_area(
                                description,
                                move || {
                                    i18n::t(
                                        language.get().as_str(),
                                        "commit.descriptionPlaceholder",
                                    )
                                },
                                Some(76.0),
                                false,
                                move |text| description.set(text.to_string()),
                            ))
                            .child(crate::ui::button::button(
                                move || i18n::t(language.get().as_str(), "commit.askAgent"),
                                crate::ui::button::ButtonVariant::Ghost,
                                crate::ui::button::ButtonSize::Small,
                                false,
                                {
                                    let ask_agent = ask_agent.clone();
                                    move || ask_agent()
                                },
                            ))
                            .child(dynamic(move || -> AnyElement {
                                let Some(log) = hook_log.clone() else {
                                    return view().w_full().h(px(0.0)).into_element();
                                };
                                return hook_box(
                                    language,
                                    log,
                                    hook_ok,
                                    hook_kind,
                                    hook_cmd.clone(),
                                )
                                .into_element();
                            })),
                    ),
                ),
        )
        .child(
            view()
                .w_full()
                .p(px(12.0))
                .flex_col()
                .gap(px(4.0))
                .border_t_1()
                .border_color(move || design::line())
                .child(
                    view()
                        .w_full()
                        .flex_col()
                        .gap(px(3.0))
                        .child(checkbox_row(language, "commit.amend", amend, false, {
                            let cmd = cmd.clone();
                            move || cmd.set_commit_option("amend", !amend)
                        }))
                        .child(checkbox_row(language, "commit.signoff", signoff, false, {
                            let cmd = cmd.clone();
                            move || cmd.set_commit_option("signoff", !signoff)
                        }))
                        .child(
                            view()
                                .w_full()
                                .flex_row()
                                .items_center()
                                .gap(px(8.0))
                                .child(checkbox_row(
                                    language,
                                    "commit.pushAfter",
                                    push_after,
                                    false,
                                    {
                                        let cmd = cmd.clone();
                                        move || cmd.set_commit_option("pushAfter", !push_after)
                                    },
                                ))
                                .child(checkbox_row(
                                    language,
                                    "commit.forcePushShort",
                                    force_push_after,
                                    !push_after,
                                    {
                                        let cmd = force_cmd.clone();
                                        move || {
                                            cmd.set_commit_option("forcePush", !force_push_after)
                                        }
                                    },
                                )),
                        ),
                )
                .child(crate::ui::button::button(
                    move || {
                        if amend {
                            return i18n::t(language.get().as_str(), "commit.amendAction");
                        }
                        i18n::with_arguments(
                            language.get().as_str(),
                            "commit.action",
                            &[("count", staged_count.to_string())],
                        )
                    },
                    crate::ui::button::ButtonVariant::Primary,
                    crate::ui::button::ButtonSize::Regular,
                    false,
                    {
                        let commit = commit.clone();
                        move || {
                            if can_commit {
                                commit();
                            }
                        }
                    },
                ))
                .child(dynamic(move || -> AnyElement {
                    if !can_commit {
                        return view().w_full().h(px(0.0)).into_element();
                    }
                    return text(i18n::t(language.get().as_str(), "commit.hooksHint"))
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(move || design::text3())
                        .into_element();
                })),
        );
}

fn changes_commit_panel(
    state: &CommitState,
    cmd: &CommitCommands,
    changes: ChangesCommands,
    language: Signal<SharedString>,
    tab: Tab,
    show_tree: bool,
    tree: crate::app::session::TreeOptions,
) -> impl IntoElement {
    let summary = state.summary;
    let description = state.description;
    let staged_count = tab.changes.iter().filter(|entry| entry.staged).count();
    let amend = tab.amend;
    let signoff = tab.signoff;
    let push_after = tab.push_after_commit;
    let force_push_after = tab.force_push_after_commit;
    let can_commit = staged_count > 0 || amend;
    let hook_log = tab.hook_log.clone();
    let hook_ok = tab.hook_ok;
    let hook_kind = tab.commit_feedback_kind;
    let hook_cmd = cmd.clone();
    let force_cmd = cmd.clone();

    return view()
        .w_full()
        .min_w_0()
        .h_full()
        .flex_col()
        .bg(move || design::panel())
        .child(crate::features::changes::view::changelists_panel(
            &changes, tab, show_tree, tree,
        ))
        .child(dynamic(move || -> AnyElement {
            let Some(log) = hook_log.clone() else {
                return view().w_full().h(px(0.0)).into_element();
            };
            return view()
                .w_full()
                .flex_shrink_0()
                .px(px(12.0))
                .py(px(8.0))
                .border_t_1()
                .border_color(move || design::line_soft())
                .child(hook_box(
                    language,
                    log,
                    hook_ok,
                    hook_kind,
                    hook_cmd.clone(),
                ))
                .into_element();
        }))
        .child(
            view()
                .w_full()
                .flex_shrink_0()
                .p(px(12.0))
                .flex_col()
                .gap(px(6.0))
                .border_t_1()
                .border_color(move || design::line())
                .child(crate::ui::text_input::text_input(
                    summary,
                    move || i18n::t(language.get().as_str(), "commit.summaryCompactPlaceholder"),
                    false,
                    move |text| summary.set(text.to_string()),
                ))
                .child(crate::ui::text_input::text_area_auto_grow(
                    description,
                    move || {
                        i18n::t(
                            language.get().as_str(),
                            "commit.descriptionCompactPlaceholder",
                        )
                    },
                    crate::ui::CONTROL + crate::ui::BODY_LINE * 2.0,
                    move |text| description.set(text.to_string()),
                ))
                .child(
                    view()
                        .w_full()
                        .flex_row()
                        .items_center()
                        .gap(px(4.0))
                        .child(checkbox_row(language, "commit.amendShort", amend, false, {
                            let cmd = cmd.clone();
                            move || cmd.set_commit_option("amend", !amend)
                        }))
                        .child(checkbox_row(
                            language,
                            "commit.signoffShort",
                            signoff,
                            false,
                            {
                                let cmd = cmd.clone();
                                move || cmd.set_commit_option("signoff", !signoff)
                            },
                        ))
                        .child(checkbox_row(
                            language,
                            "commit.pushAfterShort",
                            push_after,
                            false,
                            {
                                let cmd = cmd.clone();
                                move || cmd.set_commit_option("pushAfter", !push_after)
                            },
                        ))
                        .child(checkbox_row(
                            language,
                            "commit.forcePushShort",
                            force_push_after,
                            !push_after,
                            {
                                let cmd = force_cmd.clone();
                                move || cmd.set_commit_option("forcePush", !force_push_after)
                            },
                        ))
                        .child(view().flex_1())
                        .child(crate::ui::button::button(
                            move || i18n::t(language.get().as_str(), "commit.actionShort"),
                            crate::ui::button::ButtonVariant::Primary,
                            crate::ui::button::ButtonSize::Regular,
                            !can_commit,
                            {
                                let cmd = cmd.clone();
                                move || {
                                    if can_commit {
                                        cmd.commit();
                                    }
                                }
                            },
                        )),
                )
                .child(dynamic(move || -> AnyElement {
                    if !can_commit {
                        return view().w_full().h(px(0.0)).into_element();
                    }
                    return text(i18n::t(language.get().as_str(), "commit.hooksHint"))
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(move || design::text3())
                        .into_element();
                })),
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
                .text_size(px(crate::ui::SECONDARY))
                .font_weight(FontWeight(600))
                .text_color(move || design::text2()),
        )
        .child(
            text(move || i18n::t(language.get().as_str(), hint_key))
                .text_size(px(crate::ui::SECONDARY))
                .text_color(move || design::text3()),
        );
}

/// 一行复选：14px 方框加标签，与设计板的 `check` 行一致。
fn checkbox_row(
    language: Signal<SharedString>,
    key: &'static str,
    checked: bool,
    disabled: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    // 原生 Checkbox：按压、键盘激活与无障碍状态由组件负责，外观取自 ui 层。
    let toggle = move |_next: bool| on_press();
    return crate::ui::check::check_inline(
        move || i18n::t(language.get().as_str(), key),
        checked,
        22.0,
        12.0,
        disabled,
        toggle,
    );
}

/// 提交操作反馈块：成功绿点、失败红点，命令输出等宽显示。
///
/// 钩子失败时提供重试、跳过钩子提交；完整日志在弹层查看。
fn hook_box(
    language: Signal<SharedString>,
    log: String,
    ok: bool,
    kind: CommitFeedbackKind,
    cmd: CommitCommands,
) -> impl IntoElement {
    let has_log = !log.trim().is_empty();
    // 面板里只显示末尾若干行，完整内容在 `View full log` 弹层。
    let preview: String = {
        let lines: Vec<&str> = log.lines().collect();
        let start = lines.len().saturating_sub(6);
        lines[start..].join("\n")
    };
    let retry = {
        let cmd = cmd.clone();
        move || cmd.commit()
    };
    let without_hooks = {
        let cmd = cmd.clone();
        move || cmd.commit_without_hooks()
    };
    let open_log = {
        let cmd = cmd.clone();
        move |open: bool| cmd.toggle_hook_log(open)
    };
    let dismiss = cmd.clone();
    return view()
        .w_full()
        .flex_col()
        .gap(px(4.0))
        .p(px(6.0))
        .rounded(px(metrics::RADIUS_CONTROL))
        .border_1()
        .border_color(move || if ok { design::line() } else { design::danger() })
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
                        .bg(move || if ok { design::ok() } else { design::danger() }),
                )
                .child(
                    text(move || {
                        let key = match kind {
                            CommitFeedbackKind::Hooks if ok => "commit.hooksPassed",
                            CommitFeedbackKind::Hooks => "commit.hooksFailed",
                            CommitFeedbackKind::Commit => "commit.commitFailed",
                            CommitFeedbackKind::Push if ok => "commit.pushPassed",
                            CommitFeedbackKind::Push => "commit.pushFailed",
                        };
                        i18n::t(language.get().as_str(), key)
                    })
                    .text_size(px(crate::ui::SECONDARY))
                    .text_color(move || design::text2()),
                )
                .child(view().flex_1())
                .child(crate::ui::button::icon_button(
                    "x",
                    move || i18n::t(language.get().as_str(), "common.close"),
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || dismiss.dismiss_hook_status(),
                )),
        )
        .child(view().w_full().when(has_log, |this| {
            this.child(
                text(preview)
                    .text_size(px(crate::ui::CODE))
                    .line_height(px(crate::ui::CODE_LINE))
                    .text_color(move || design::text2())
                    .text_overflow(nsh_core::EllipsizeMode::Tail),
            )
        }))
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .when(!ok && kind == CommitFeedbackKind::Hooks, |this| {
                    this.child(crate::ui::button::button(
                        {
                            let language = language;
                            move || i18n::t(language.get().as_str(), "commit.retryHooks")
                        },
                        crate::ui::button::ButtonVariant::Secondary,
                        crate::ui::button::ButtonSize::Small,
                        false,
                        {
                            let retry = retry.clone();
                            move || retry()
                        },
                    ))
                    .child(crate::ui::button::button(
                        {
                            let language = language;
                            move || i18n::t(language.get().as_str(), "commit.withoutHooks")
                        },
                        crate::ui::button::ButtonVariant::Secondary,
                        crate::ui::button::ButtonSize::Small,
                        false,
                        {
                            let without_hooks = without_hooks.clone();
                            move || without_hooks()
                        },
                    ))
                })
                .child(crate::ui::button::button(
                    {
                        let language = language;
                        move || i18n::t(language.get().as_str(), "commit.viewLog")
                    },
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || open_log(true),
                )),
        );
}
