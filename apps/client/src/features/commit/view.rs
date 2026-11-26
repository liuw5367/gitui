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
use nsh_core::element::IntoProp;
use nsh_core::prelude::*;

/// 提交面板的初始宽度（可由工作区分栏拖拽调整）。
pub(crate) const WIDTH: f32 = 360.0;

/// 工作区右侧统一使用当前 Changes 提交面板。
pub(crate) fn commit_panel(
    state: &CommitState,
    cmd: &CommitCommands,
    changes: ChangesCommands,
    language: Signal<SharedString>,
    tab: Tab,
    current: Memo<Option<Tab>>,
    show_tree: bool,
    tree: crate::app::session::TreeOptions,
) -> AnyElement {
    return changes_commit_panel(state, cmd, changes, language, tab, current, show_tree, tree)
        .into_element();
}

fn changes_commit_panel(
    state: &CommitState,
    cmd: &CommitCommands,
    changes: ChangesCommands,
    language: Signal<SharedString>,
    tab: Tab,
    current: Memo<Option<Tab>>,
    show_tree: bool,
    tree: crate::app::session::TreeOptions,
) -> impl IntoElement {
    let summary = state.summary;
    let description = state.description;
    let hook_snapshot = computed({
        let current = current;
        move || {
            current.with(|tab| {
                tab.as_ref()
                    .map(|tab| (tab.hook_log.clone(), tab.hook_ok, tab.commit_feedback_kind))
            })
        }
    });
    let hook_cmd = cmd.clone();

    return view()
        .w_full()
        .min_w_0()
        .h_full()
        .flex_col()
        .bg(move || design::panel())
        .child(crate::features::changes::view::changelists_panel(
            &changes, tab, current, show_tree, tree,
        ))
        .child(dynamic(move || -> AnyElement {
            let Some((Some(log), ok, kind)) = hook_snapshot.get() else {
                return view().w_full().h(px(0.0)).into_element();
            };
            return view()
                .w_full()
                .flex_shrink_0()
                .px(px(12.0))
                .py(px(8.0))
                .border_t_1()
                .border_color(move || design::line_soft())
                .child(hook_box(language, log, ok, kind, hook_cmd.clone()))
                .into_element();
        }))
        .child(
            view()
                .w_full()
                .flex_shrink_0()
                .p(px(12.0))
                .pb(px(6.0))
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
                        .child(checkbox_row(
                            language,
                            "commit.amendShort",
                            move || current.with(|tab| tab.as_ref().is_some_and(|tab| tab.amend)),
                            false,
                            {
                                let cmd = cmd.clone();
                                move || {
                                    let amend = current.with_untracked(|tab| {
                                        tab.as_ref().is_some_and(|tab| tab.amend)
                                    });
                                    cmd.set_commit_option("amend", !amend);
                                }
                            },
                        ))
                        .child(checkbox_row(
                            language,
                            "commit.signoffShort",
                            move || current.with(|tab| tab.as_ref().is_some_and(|tab| tab.signoff)),
                            false,
                            {
                                let cmd = cmd.clone();
                                move || {
                                    let signoff = current.with_untracked(|tab| {
                                        tab.as_ref().is_some_and(|tab| tab.signoff)
                                    });
                                    cmd.set_commit_option("signoff", !signoff);
                                }
                            },
                        ))
                        .child(checkbox_row(
                            language,
                            "commit.pushAfterShort",
                            move || {
                                current.with(|tab| {
                                    tab.as_ref().is_some_and(|tab| tab.push_after_commit)
                                })
                            },
                            false,
                            {
                                let cmd = cmd.clone();
                                move || {
                                    let push_after = current.with_untracked(|tab| {
                                        tab.as_ref().is_some_and(|tab| tab.push_after_commit)
                                    });
                                    cmd.set_commit_option("pushAfter", !push_after);
                                }
                            },
                        ))
                        .child(checkbox_row(
                            language,
                            "commit.forcePushShort",
                            move || {
                                current.with(|tab| {
                                    tab.as_ref().is_some_and(|tab| tab.force_push_after_commit)
                                })
                            },
                            move || {
                                !current.with(|tab| {
                                    tab.as_ref().is_some_and(|tab| tab.push_after_commit)
                                })
                            },
                            {
                                let cmd = cmd.clone();
                                move || {
                                    let force_push_after = current.with_untracked(|tab| {
                                        tab.as_ref().is_some_and(|tab| tab.force_push_after_commit)
                                    });
                                    cmd.set_commit_option("forcePush", !force_push_after);
                                }
                            },
                        ))
                        .child(view().flex_1())
                        .child(crate::ui::button::button(
                            move || i18n::t(language.get().as_str(), "commit.actionShort"),
                            crate::ui::button::ButtonVariant::Primary,
                            crate::ui::button::ButtonSize::Regular,
                            move || {
                                !current.with(|tab| {
                                    tab.as_ref().is_some_and(|tab| {
                                        tab.amend || tab.changes.iter().any(|entry| entry.staged)
                                    })
                                })
                            },
                            {
                                let cmd = cmd.clone();
                                move || {
                                    let can_commit = current.with_untracked(|tab| {
                                        tab.as_ref().is_some_and(|tab| {
                                            tab.amend
                                                || tab.changes.iter().any(|entry| entry.staged)
                                        })
                                    });
                                    if can_commit {
                                        cmd.commit();
                                    }
                                }
                            },
                        )),
                ),
        );
}

/// 一行复选：14px 方框加标签，与设计板的 `check` 行一致。
fn checkbox_row<MC, MD>(
    language: Signal<SharedString>,
    key: &'static str,
    checked: impl IntoProp<bool, MC> + 'static,
    disabled: impl IntoProp<bool, MD> + 'static,
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
