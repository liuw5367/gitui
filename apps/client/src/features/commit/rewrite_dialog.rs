//! 历史改写弹窗（设计板 22 的 Edit commit message 与 Squash commits）。
//!
//! 两种形态共用一个模态：改写说明只填一段文本，合并提交额外列出将被合并的提交。
//! 工作区不干净或有进行中的操作时，主按钮禁用并写明原因——改写历史前必须先让仓库
//! 处于干净状态，否则 rebase 会带着改动一起跑。

use super::commands::CommitCommands;
use super::state::{CommitState, RewriteKind};
use crate::design::{self, metrics};
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
            dynamic(move || -> AnyElement {
                let Some(dialog) = state.get() else {
                    return view().w(px(0.0)).h(px(0.0)).into_element();
                };
                let kind = dialog.kind.clone();
                // 提前取成 `&'static str`，闭包里就不用再借用 `kind`。
                let title_key = kind.title_key();
                let action_key = kind.action_key();
                let blocker = dialog.blocker.clone();
                let error = dialog.error.clone();
                let message = dialog.message.clone();
                let can_submit = blocker.is_none() && !message.trim().is_empty();
                let language_value = language;
                let on_close = on_close.clone();
                let on_submit = on_submit.clone();
                let on_change = on_change.clone();

                let mut body: Vec<AnyElement> = Vec::new();
                // 合并时先列出被合并的提交，让用户确认范围。
                if let RewriteKind::Squash { commits, from, to } = &kind {
                    let mut rows: Vec<AnyElement> = Vec::new();
                    for (short, summary) in commits.iter() {
                        let short = short.clone();
                        let summary = summary.clone();
                        rows.push(
                            view()
                                .w_full()
                                .h(px(20.0))
                                .flex_row()
                                .items_center()
                                .gap(px(8.0))
                                .child(
                                    text(short)
                                        .w(px(64.0))
                                        .text_size(px(crate::ui::CODE))
                                        .line_height(px(13.0))
                                        .text_color(move || design::text3()),
                                )
                                .child(
                                    text(summary)
                                        .flex_1()
                                        .text_size(px(crate::ui::SECONDARY))
                                        .line_height(px(crate::ui::SECONDARY_LINE))
                                        .text_color(move || design::text2())
                                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                                )
                                .into_element(),
                        );
                    }
                    // 先把范围拷成自有字符串：闭包需要 'static，不能借 `kind`。
                    let from_short: String = from.chars().take(7).collect();
                    let to_short: String = to.chars().take(7).collect();
                    let range = format!("{}..{}", from_short, to_short);
                    let count = commits.len().to_string();
                    body.push(
                        view()
                            .w_full()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                text(move || {
                                    i18n::with_arguments(
                                        language_value.get().as_str(),
                                        "rewrite.squashRange",
                                        &[("range", range.clone()), ("count", count.clone())],
                                    )
                                })
                                .text_size(px(crate::ui::SECONDARY))
                                .line_height(px(crate::ui::SECONDARY_LINE))
                                .text_color(move || design::text3()),
                            )
                            .child(rows)
                            .into_element(),
                    );
                }
                // 改写说明时显示目标提交，避免改错。
                if let RewriteKind::Reword { short, summary, .. } = &kind {
                    let short = short.clone();
                    let summary = summary.clone();
                    body.push(
                        view()
                            .w_full()
                            .flex_row()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                text(short)
                                    .text_size(px(crate::ui::CODE))
                                    .line_height(px(13.0))
                                    .text_color(move || design::text3()),
                            )
                            .child(
                                text(summary)
                                    .flex_1()
                                    .text_size(px(crate::ui::SECONDARY))
                                    .line_height(px(crate::ui::SECONDARY_LINE))
                                    .text_color(move || design::text2())
                                    .text_overflow(nsh_core::EllipsizeMode::Tail),
                            )
                            .into_element(),
                    );
                }

                return crate::ui::dialog::modality(
                    view()
                        .w(px(480.0))
                        .flex_col()
                        .rounded(px(12.0))
                        .border_1()
                        .border_color(move || design::line())
                        .bg(move || design::content())
                        .overflow_hidden()
                        .child(
                            view()
                                .w_full()
                                .h(px(metrics::DIALOG_HEAD_H))
                                .flex_shrink_0()
                                .px(px(14.0))
                                .flex_row()
                                .items_center()
                                .bg(move || design::panel())
                                .border_b_1()
                                .border_color(move || design::line())
                                .child(
                                    text(move || i18n::t(language_value.get().as_str(), title_key))
                                        .text_size(px(crate::ui::TITLE))
                                        .font_weight(FontWeight(600))
                                        .line_height(px(17.0))
                                        .text_color(move || design::text()),
                                ),
                        )
                        // `.dialog-body`：padding 11 14。
                        .child(
                            view()
                                .w_full()
                                .flex_col()
                                .gap(px(10.0))
                                .px(px(14.0))
                                .py(px(11.0))
                                .child(body)
                                .child(
                                    view()
                                        .w_full()
                                        .min_h(px(72.0))
                                        .p(px(7.0))
                                        .rounded(px(metrics::RADIUS_CONTROL))
                                        .border_1()
                                        .border_color(move || design::field_border())
                                        .bg(move || design::field())
                                        .child(crate::ui::text_input::text_input_bound(
                                            {
                                                let message = message.clone();
                                                move || message.clone()
                                            },
                                            move || {
                                                i18n::t(
                                                    language_value.get().as_str(),
                                                    "rewrite.placeholder",
                                                )
                                            },
                                            false,
                                            {
                                                let on_change = on_change.clone();
                                                move |text: String| on_change(text)
                                            },
                                        )),
                                )
                                // 阻塞原因与错误分开显示：前者是前置条件，后者是执行结果。
                                .child(view().w_full().when(blocker.is_some(), |this| {
                                    let blocker = blocker.clone().unwrap_or_default();
                                    this.child(
                                        text(blocker)
                                            .text_size(px(crate::ui::SECONDARY))
                                            .line_height(px(crate::ui::SECONDARY_LINE))
                                            .text_color(move || design::warn()),
                                    )
                                }))
                                .child(view().w_full().when(error.is_some(), |this| {
                                    let error = error.clone().unwrap_or_default();
                                    this.child(
                                        text(error)
                                            .text_size(px(crate::ui::SECONDARY))
                                            .line_height(px(crate::ui::SECONDARY_LINE))
                                            .text_color(move || design::danger()),
                                    )
                                })),
                        )
                        // `.dialog-foot`：42 高、panel 底色。
                        .child(
                            view()
                                .w_full()
                                .h(px(metrics::DIALOG_FOOT_H))
                                .flex_shrink_0()
                                .px(px(14.0))
                                .flex_row()
                                .items_center()
                                .gap(px(7.0))
                                .bg(move || design::panel())
                                .border_t_1()
                                .border_color(move || design::line())
                                .child(view().flex_1())
                                .child(crate::ui::button::button(
                                    move || i18n::t(language_value.get().as_str(), "common.cancel"),
                                    crate::ui::button::ButtonVariant::Secondary,
                                    crate::ui::button::ButtonSize::Regular,
                                    false,
                                    {
                                        let on_close = on_close.clone();
                                        move || on_close()
                                    },
                                ))
                                .child(crate::ui::button::button(
                                    move || i18n::t(language_value.get().as_str(), action_key),
                                    crate::ui::button::ButtonVariant::Primary,
                                    crate::ui::button::ButtonSize::Regular,
                                    false,
                                    {
                                        let on_submit = on_submit.clone();
                                        move || {
                                            if can_submit {
                                                on_submit()
                                            }
                                        }
                                    },
                                )),
                        )
                        .into_element(),
                )
                .into_element();
            })
        },
        move || dismiss(),
    );
}
