//! 历史改写弹窗（设计板 22 的 Edit commit message 与 Squash commits）。
//!
//! 两种形态共用一个模态：改写说明只填一段文本，合并提交额外列出将被合并的提交。
//! 工作区不干净或有进行中的操作时，主按钮禁用并写明原因——改写历史前必须先让仓库
//! 处于干净状态，否则 rebase 会带着改动一起跑。

use super::commands::CommitCommands;
use super::state::{CommitState, RewriteKind};
use crate::design;
use crate::i18n;
use nsh_core::prelude::*;

/// 入口 `rewrite_dialog` 收本功能的状态与用例；语言单独传。
pub(crate) fn rewrite_dialog(
    state: &CommitState,
    cmd: &CommitCommands,
    language: Signal<SharedString>,
) -> impl IntoElement {
    let state = state.rewrite;
    // 视图回调要 `'static`，这里把本功能的命令各包一层。
    let on_change: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |text: String| cmd.set_rewrite_message(text))
    };
    let on_submit: std::sync::Arc<dyn Fn()> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move || cmd.submit_rewrite())
    };
    let on_close: std::sync::Arc<dyn Fn()> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move || cmd.close_rewrite())
    };
    let dismiss = on_close.clone();
    return crate::ui::dialog::modal(
        move || state.get().is_some(),
        move || {
            state
                .get()
                .map(|dialog| i18n::t(language.get().as_str(), dialog.kind.title_key()))
                .unwrap_or_default()
        },
        140,
        move || {
            let state = state;
            let language = language;
            let on_close = on_close.clone();
            let on_submit = on_submit.clone();
            let on_change = on_change.clone();
            let kind = computed(move || {
                state.with(|dialog| dialog.as_ref().map(|dialog| dialog.kind.clone()))
            });
            dynamic(move || {
                let Some(kind) = kind.get() else {
                    return view().into_element();
                };
                let title_key = kind.title_key();
                let action_key = kind.action_key();
                let mut rows = Vec::new();
                match &kind {
                    RewriteKind::Squash { commits, from, to } => {
                        let range = format!(
                            "{}..{}",
                            from.chars().take(7).collect::<String>(),
                            to.chars().take(7).collect::<String>()
                        );
                        let count = commits.len().to_string();
                        rows.push(
                            text(move || {
                                i18n::with_arguments(
                                    language.get().as_str(),
                                    "rewrite.squashRange",
                                    &[("range", range.clone()), ("count", count.clone())],
                                )
                            })
                            .text_size(px(crate::ui::SECONDARY))
                            .text_color(design::text3)
                            .into_element(),
                        );
                        for (short, summary) in commits {
                            rows.push(
                                text(format!("{short} · {summary}"))
                                    .text_size(px(crate::ui::SECONDARY))
                                    .line_height(px(crate::ui::SECONDARY_LINE))
                                    .text_color(design::text2)
                                    .into_element(),
                            );
                        }
                    }
                    RewriteKind::Reword { short, summary, .. } => {
                        rows.push(
                            text(format!("{short} · {summary}"))
                                .text_size(px(crate::ui::SECONDARY))
                                .line_height(px(crate::ui::SECONDARY_LINE))
                                .text_color(design::text2)
                                .into_element(),
                        );
                    }
                }
                let edit = on_change.clone();
                let body = view()
                    .w_full()
                    .flex_col()
                    .gap(px(crate::ui::GAP))
                    .child(
                        scroll().w_full().max_h(px(160.0)).child(
                            view()
                                .w_full()
                                .flex_col()
                                .gap(px(crate::ui::GAP_XS))
                                .children(rows),
                        ),
                    )
                    .child(crate::ui::text_input::text_area_bound(
                        move || {
                            state.with(|dialog| {
                                dialog
                                    .as_ref()
                                    .map(|dialog| dialog.message.clone())
                                    .unwrap_or_default()
                            })
                        },
                        move || i18n::t(language.get().as_str(), "rewrite.placeholder"),
                        88.0,
                        move |message| edit(message),
                    ))
                    .child(when(
                        move || {
                            state.with(|dialog| {
                                dialog
                                    .as_ref()
                                    .is_some_and(|dialog| dialog.blocker.is_some())
                            })
                        },
                        move || {
                            text(move || {
                                state
                                    .get()
                                    .and_then(|dialog| dialog.blocker)
                                    .unwrap_or_default()
                            })
                            .text_size(px(crate::ui::SECONDARY))
                            .line_height(px(crate::ui::SECONDARY_LINE))
                            .text_color(design::warn)
                        },
                    ))
                    .child(when(
                        move || {
                            state.with(|dialog| {
                                dialog.as_ref().is_some_and(|dialog| dialog.error.is_some())
                            })
                        },
                        move || {
                            text(move || {
                                state
                                    .get()
                                    .and_then(|dialog| dialog.error)
                                    .unwrap_or_default()
                            })
                            .text_size(px(crate::ui::SECONDARY))
                            .line_height(px(crate::ui::SECONDARY_LINE))
                            .text_color(design::danger)
                        },
                    ));
                let cancel = on_close.clone();
                let close = on_close.clone();
                let submit = on_submit.clone();
                let can_submit = computed(move || {
                    state.with(|dialog| {
                        dialog.as_ref().is_some_and(|dialog| {
                            dialog.blocker.is_none() && !dialog.message.trim().is_empty()
                        })
                    })
                });
                let footer = vec![
                    crate::ui::button::button(
                        move || i18n::t(language.get().as_str(), "common.cancel"),
                        crate::ui::button::ButtonVariant::Secondary,
                        crate::ui::button::ButtonSize::Regular,
                        false,
                        move || cancel(),
                    )
                    .into_element(),
                    dynamic(move || {
                        let submit = submit.clone();
                        crate::ui::button::button(
                            move || i18n::t(language.get().as_str(), action_key),
                            crate::ui::button::ButtonVariant::Primary,
                            crate::ui::button::ButtonSize::Regular,
                            !can_submit.get(),
                            move || submit(),
                        )
                    })
                    .into_element(),
                ];
                crate::ui::dialog::dialog_panel(
                    480.0,
                    move || i18n::t(language.get().as_str(), title_key),
                    Some(std::rc::Rc::new(move || {
                        i18n::t(language.get().as_str(), "common.close")
                    })),
                    body,
                    footer,
                    move || close(),
                )
            })
        },
        move || dismiss(),
    );
}
