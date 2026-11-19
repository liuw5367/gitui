//! 冲突解决视图（设计板 14）。
//!
//! 三方对照：左侧 ours、右侧 theirs、下方可编辑的结果区。顶部可切换冲突文件、
//! 显示 base，底部是接受一侧、标记已解决、继续与放弃。
//!
//! 结果区是一个真实的编辑框：接受某一侧只是把它填进去，仍可继续手改；标记已解决
//! 会把内容写回文件并 `git add`。

use super::commands::MergeCommands;
use super::state::MergeState;
use crate::app::session::Tab;
use crate::design::{self, metrics};
use crate::i18n;
use domain::conflict::Side;
use nsh_core::prelude::*;

/// A read-only stage pane. Missing stages and binary blobs get an explicit state.
fn pane(
    language: Signal<SharedString>,
    title_key: &'static str,
    path: String,
    bytes: Option<&[u8]>,
) -> AnyElement {
    let body = match bytes {
        None => crate::ui::text_input::code_view(
            path,
            String::new(),
            i18n::t(language.get_untracked().as_str(), "conflicts.sideDeleted"),
        )
        .into_element(),
        Some(bytes) => match conflict_text(bytes) {
            Some(value) => {
                crate::ui::text_input::code_view(path, value, String::new()).into_element()
            }
            None => view()
                .w_full()
                .flex_1()
                .items_center()
                .justify_center()
                .child(
                    text(move || i18n::t(language.get().as_str(), "conflicts.binaryPane"))
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(move || design::text3()),
                )
                .into_element(),
        },
    };
    return view()
        .flex_1()
        .min_w_0()
        .h_full()
        .flex_col()
        .rounded(px(metrics::RADIUS_CONTROL))
        .border_1()
        .border_color(move || design::line_soft())
        .bg(move || design::content())
        .child(
            view()
                .w_full()
                .h(px(22.0))
                .px(px(8.0))
                .flex_row()
                .items_center()
                .border_b_1()
                .border_color(move || design::line_soft())
                .child(
                    text(move || i18n::t(language.get().as_str(), title_key))
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(crate::ui::SECONDARY_LINE))
                        .text_color(move || design::text3()),
                ),
        )
        .child(view().w_full().flex_1().min_h_0().child(body))
        .into_element();
}

fn conflict_text(bytes: &[u8]) -> Option<String> {
    if bytes.contains(&0) {
        return None;
    }
    return String::from_utf8(bytes.to_vec()).ok();
}

/// 入口 `conflict_view` 收本功能的状态与用例；语言与当前仓库单独传。
pub(crate) fn conflict_view(
    state: &MergeState,
    cmd: &MergeCommands,
    language: Signal<SharedString>,
    tab: Memo<Option<Tab>>,
) -> impl IntoElement {
    let base_open = state.base_open;
    // 视图回调要 `'static`，这里把本功能的命令各包一层。
    let close = {
        let cmd = cmd.clone();
        move || cmd.close_conflict()
    };
    let select = {
        let cmd = cmd.clone();
        move |index: usize| cmd.select_conflict(index)
    };
    let apply_side = {
        let cmd = cmd.clone();
        move |side: Side| cmd.apply_side(side)
    };
    let update_result: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |value| cmd.update_result(value))
    };
    let mark = {
        let cmd = cmd.clone();
        move || cmd.mark_resolved()
    };
    let step = {
        let cmd = cmd.clone();
        move |forward: bool| cmd.step_conflict(forward)
    };
    let continue_op = {
        let cmd = cmd.clone();
        move || cmd.continue_op()
    };
    let abort = {
        let cmd = cmd.clone();
        move || cmd.abort_operation()
    };
    return view()
        .absolute()
        .inset(px(0.0))
        .z_index(130)
        // 空状态仍保留全屏根节点；透明背景不会穿透点击，关闭时要退出命中测试。
        .pointer_events(move || {
            if tab.get().is_some_and(|tab| tab.conflict_open) {
                PointerEvents::Auto
            } else {
                PointerEvents::None
            }
        })
        // 设计稿 `.merge-layer`：顶部对齐、`padding:22px 34px`。
        .items_center()
        .justify_center()
        .p(px(metrics::MERGE_LAYER_PAD.1))
        .pt(px(metrics::MERGE_LAYER_PAD.0))
        .bg(move || {
            let open = tab.get().map(|tab| tab.conflict_open).unwrap_or(false);
            if open {
                design::token("overlay")
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .child(dynamic(move || -> AnyElement {
            let Some(current) = tab.get() else {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            };
            if !current.conflict_open {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            }
            let paths = current.conflicts.clone();
            let index = current.conflict_index;
            let total = paths.len();
            let unresolved = total;
            let selected_path = paths.get(index).cloned();
            let selected_draft = selected_path
                .as_ref()
                .and_then(|path| current.conflict_drafts.get(path))
                .cloned();
            let conflict_sides = current.conflict_sides.clone();
            let result_is_delete = matches!(
                selected_draft.as_ref(),
                Some(crate::app::session::ConflictDraft::Delete)
            );
            let result_is_binary = match selected_draft.as_ref() {
                Some(crate::app::session::ConflictDraft::Binary(_)) => true,
                Some(
                    crate::app::session::ConflictDraft::Text(_)
                    | crate::app::session::ConflictDraft::Delete,
                ) => false,
                None => current.conflict_binary,
            };
            let result_text = super::ops::conflict_result(&current);
            let has_markers = domain::conflict::has_markers(result_text.as_str());
            let operation = current.operation;
            let can_continue = unresolved == 0
                && current.conflict_drafts.is_empty()
                && current.operation != git::Operation::None;
            let can_mark = selected_path.is_some()
                && !has_markers
                && (!result_is_binary
                    || matches!(
                        selected_draft.as_ref(),
                        Some(crate::app::session::ConflictDraft::Binary(_))
                    ));
            let language_value = language;
            let close = close.clone();
            let select = select.clone();
            let apply_side = apply_side.clone();
            let update_result = update_result.clone();
            let mark = mark.clone();
            let step = step.clone();
            let continue_op = continue_op.clone();
            let abort = abort.clone();
            // 冲突文件的标签条。
            let chips: Vec<AnyElement> = paths
                .iter()
                .enumerate()
                .map(|(chip_index, path)| {
                    let select = select.clone();
                    let active = chip_index == index;
                    let name = path.clone();
                    let variant = if active {
                        crate::ui::button::ButtonVariant::Secondary
                    } else {
                        crate::ui::button::ButtonVariant::Ghost
                    };
                    return crate::ui::button::button(
                        move || name.clone(),
                        variant,
                        crate::ui::button::ButtonSize::Small,
                        false,
                        move || select(chip_index),
                    )
                    .into_element();
                })
                .collect();

            let detail: AnyElement = if let Some(path) = selected_path.clone() {
                let sides = conflict_sides.clone();
                let ours = pane(
                    language,
                    "conflicts.ours",
                    path.clone(),
                    sides.as_ref().and_then(|sides| sides.ours.as_deref()),
                );
                let sides = conflict_sides.clone();
                let theirs = pane(
                    language,
                    "conflicts.theirs",
                    path.clone(),
                    sides.as_ref().and_then(|sides| sides.theirs.as_deref()),
                );
                let result_body: AnyElement = if result_is_binary {
                    view()
                        .w_full()
                        .flex_1()
                        .items_center()
                        .justify_center()
                        .child(
                            text(move || {
                                let key = if matches!(
                                    selected_draft.as_ref(),
                                    Some(crate::app::session::ConflictDraft::Binary(_))
                                ) {
                                    "conflicts.binarySelected"
                                } else {
                                    "conflicts.binaryChooseSide"
                                };
                                i18n::t(language_value.get().as_str(), key)
                            })
                            .text_size(px(crate::ui::SECONDARY))
                            .text_color(move || design::text3()),
                        )
                        .into_element()
                } else {
                    let placeholder_key = if result_is_delete {
                        "conflicts.resultWillDelete"
                    } else {
                        "conflicts.resultPlaceholder"
                    };
                    let result_tab = tab.clone();
                    let update_result = update_result.clone();
                    crate::ui::text_input::code_editor(
                        path.clone(),
                        move || {
                            result_tab
                                .get()
                                .map(|tab| super::ops::conflict_result(&tab))
                                .unwrap_or_default()
                        },
                        i18n::t(language_value.get_untracked().as_str(), placeholder_key),
                        move |value| update_result(value.to_string()),
                    )
                    .into_element()
                };
                view()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .p(px(12.0))
                    .child(crate::ui::split::horizontal(vec![
                        crate::ui::split::Pane::flex(ours),
                        crate::ui::split::Pane::flex(
                            view()
                                .flex_1()
                                .min_w_0()
                                .h_full()
                                .flex_col()
                                .rounded(px(metrics::RADIUS_CONTROL))
                                .border_1()
                                .border_color(move || design::line())
                                .bg(move || design::field())
                                .child(
                                    view()
                                        .w_full()
                                        .h(px(22.0))
                                        .px(px(8.0))
                                        .flex_row()
                                        .items_center()
                                        .border_b_1()
                                        .border_color(move || design::line_soft())
                                        .child(
                                            text(move || {
                                                i18n::t(
                                                    language_value.get().as_str(),
                                                    "conflicts.result",
                                                )
                                            })
                                            .text_size(px(crate::ui::SECONDARY))
                                            .line_height(px(crate::ui::SECONDARY_LINE))
                                            .text_color(move || design::text2()),
                                        ),
                                )
                                .child(view().w_full().flex_1().min_h_0().child(result_body)),
                        ),
                        crate::ui::split::Pane::flex(theirs),
                    ]))
                    .into_element()
            } else {
                view()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .items_center()
                    .justify_center()
                    .child(
                        text(move || {
                            i18n::t(language_value.get().as_str(), "conflicts.allResolved")
                        })
                        .text_size(px(crate::ui::BODY))
                        .text_color(move || design::text2()),
                    )
                    .into_element()
            };
            let base_pane = if base_open.get() && selected_path.is_some() {
                let path = selected_path.clone().unwrap_or_default();
                let base = pane(
                    language,
                    "conflicts.basePane",
                    path,
                    conflict_sides
                        .as_ref()
                        .and_then(|sides| sides.base.as_deref()),
                );
                view()
                    .w_full()
                    .h(px(metrics::MERGE_BASE_H))
                    .px(px(12.0))
                    .pb(px(8.0))
                    .child(base)
                    .into_element()
            } else {
                view().h(px(0.0)).into_element()
            };

            return view()
                .w_full()
                .max_w(px(metrics::MERGE_SHEET_MAX_W))
                .h(px(metrics::MERGE_SHEET_H))
                .flex_col()
                .rounded(px(12.0))
                .border_1()
                .border_color(move || design::line())
                .bg(move || design::content())
                .overflow_hidden()
                // 顶部：标题、文件 chip、计数、关闭。
                .child(
                    view()
                        .w_full()
                        .h(px(metrics::MERGE_HEAD_H))
                        .flex_shrink_0()
                        .px(px(14.0))
                        .flex_row()
                        .items_center()
                        .gap(px(9.0))
                        .bg(move || design::panel())
                        .border_b_1()
                        .border_color(move || design::line())
                        .child(
                            view()
                                .w_full()
                                .flex_row()
                                .items_center()
                                .gap(px(8.0))
                                .child(
                                    text(move || {
                                        i18n::t(language_value.get().as_str(), "conflicts.title")
                                    })
                                    .text_size(px(crate::ui::BODY))
                                    .font_weight(FontWeight(600))
                                    .line_height(px(17.0))
                                    .text_color(move || design::text()),
                                )
                                .child(
                                    text(move || {
                                        i18n::with_arguments(
                                            language_value.get().as_str(),
                                            "conflicts.unresolved",
                                            &[("count", unresolved.to_string())],
                                        )
                                    })
                                    .text_size(px(crate::ui::SECONDARY))
                                    .line_height(px(crate::ui::SECONDARY_LINE))
                                    .text_color(move || design::danger()),
                                )
                                .child(view().flex_1())
                                .child(crate::ui::button::button(
                                    move || {
                                        i18n::t(language_value.get().as_str(), "conflicts.base")
                                    },
                                    crate::ui::button::ButtonVariant::Secondary,
                                    crate::ui::button::ButtonSize::Small,
                                    selected_path.is_none(),
                                    move || base_open.set(!base_open.get_untracked()),
                                ))
                                .child(crate::ui::button::button(
                                    move || "‹".to_string(),
                                    crate::ui::button::ButtonVariant::Secondary,
                                    crate::ui::button::ButtonSize::Small,
                                    index == 0,
                                    {
                                        let step = step.clone();
                                        move || step(false)
                                    },
                                ))
                                .child(
                                    text(move || {
                                        if total == 0 {
                                            "0 / 0".to_string()
                                        } else {
                                            format!("{} / {}", index + 1, total)
                                        }
                                    })
                                    .text_size(px(crate::ui::CODE))
                                    .line_height(px(crate::ui::CODE_LINE))
                                    .text_color(move || design::text3()),
                                )
                                .child(crate::ui::button::button(
                                    move || "›".to_string(),
                                    crate::ui::button::ButtonVariant::Secondary,
                                    crate::ui::button::ButtonSize::Small,
                                    total == 0 || index + 1 >= total,
                                    {
                                        let step = step.clone();
                                        move || step(true)
                                    },
                                ))
                                .child(crate::ui::button::button(
                                    move || i18n::t(language_value.get().as_str(), "common.close"),
                                    crate::ui::button::ButtonVariant::Secondary,
                                    crate::ui::button::ButtonSize::Small,
                                    false,
                                    {
                                        let close = close.clone();
                                        move || close()
                                    },
                                )),
                        ),
                )
                // `.merge-body`：左侧 224 文件列表 + 三栏。
                .child(
                    view()
                        .w_full()
                        .flex_1()
                        .min_h_0()
                        .flex_row()
                        .child(
                            view()
                                .w(px(metrics::MERGE_LIST_W))
                                .h_full()
                                .flex_shrink_0()
                                .flex_col()
                                .bg(move || design::panel())
                                .border_r_1()
                                .border_color(move || design::line())
                                .child(
                                    view()
                                        .w_full()
                                        .h(px(metrics::MERGE_LIST_HEAD_H))
                                        .flex_shrink_0()
                                        .px(px(10.0))
                                        .flex_row()
                                        .items_center()
                                        .border_b_1()
                                        .border_color(move || design::line_soft())
                                        .child(
                                            text(move || {
                                                i18n::t(
                                                    language_value.get().as_str(),
                                                    "conflicts.files",
                                                )
                                            })
                                            .text_size(px(crate::ui::SECONDARY))
                                            .font_weight(FontWeight(600))
                                            .line_height(px(crate::ui::SECONDARY_LINE))
                                            .text_color(move || design::text2()),
                                        ),
                                )
                                .child(view().w_full().flex_1().min_h_0().p(px(6.0)).child(
                                    scroll().w_full().h_full().child(
                                        view().w_full().flex_col().gap(px(1.0)).child(chips),
                                    ),
                                )),
                        )
                        .child(view().flex_1().min_w_0().h_full().p(px(12.0)).child(detail)),
                )
                .child(base_pane)
                // 底部：接受一侧、标记、继续、放弃。
                .child(
                    view()
                        .w_full()
                        .h(px(metrics::MERGE_FOOT_H))
                        .flex_shrink_0()
                        .px(px(14.0))
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .bg(move || design::panel())
                        .border_t_1()
                        .border_color(move || design::line())
                        .child(crate::ui::button::button(
                            move || i18n::t(language_value.get().as_str(), "conflicts.acceptOurs"),
                            crate::ui::button::ButtonVariant::Secondary,
                            crate::ui::button::ButtonSize::Small,
                            selected_path.is_none(),
                            {
                                let apply_side = apply_side.clone();
                                move || apply_side(Side::Ours)
                            },
                        ))
                        .child(crate::ui::button::button(
                            move || {
                                i18n::t(language_value.get().as_str(), "conflicts.acceptTheirs")
                            },
                            crate::ui::button::ButtonVariant::Secondary,
                            crate::ui::button::ButtonSize::Small,
                            selected_path.is_none(),
                            {
                                let apply_side = apply_side.clone();
                                move || apply_side(Side::Theirs)
                            },
                        ))
                        .child(crate::ui::button::button(
                            move || i18n::t(language_value.get().as_str(), "conflicts.acceptBoth"),
                            crate::ui::button::ButtonVariant::Secondary,
                            crate::ui::button::ButtonSize::Small,
                            selected_path.is_none()
                                || result_is_binary
                                || !domain::conflict::has_markers(current.conflict_text.as_str()),
                            {
                                let apply_side = apply_side.clone();
                                move || apply_side(Side::Both)
                            },
                        ))
                        .child(view().flex_1())
                        .child(crate::ui::button::button(
                            move || {
                                i18n::t(language_value.get().as_str(), "conflicts.markResolved")
                            },
                            crate::ui::button::ButtonVariant::Secondary,
                            crate::ui::button::ButtonSize::Small,
                            !can_mark,
                            {
                                let mark = mark.clone();
                                move || mark()
                            },
                        ))
                        .child(crate::ui::button::button(
                            {
                                let operation = operation;
                                move || {
                                    let language = language_value.get();
                                    let operation =
                                        super::ops::operation_label(language.as_str(), operation);
                                    i18n::with_arguments(
                                        language.as_str(),
                                        "conflicts.applyContinue",
                                        &[("operation", operation)],
                                    )
                                }
                            },
                            crate::ui::button::ButtonVariant::Primary,
                            crate::ui::button::ButtonSize::Regular,
                            !can_continue,
                            {
                                let continue_op = continue_op.clone();
                                move || continue_op()
                            },
                        ))
                        .child(crate::ui::button::button(
                            {
                                let operation = operation;
                                move || {
                                    let language = language_value.get();
                                    let operation =
                                        super::ops::operation_label(language.as_str(), operation);
                                    i18n::with_arguments(
                                        language.as_str(),
                                        "conflicts.abort",
                                        &[("operation", operation)],
                                    )
                                }
                            },
                            crate::ui::button::ButtonVariant::Secondary,
                            crate::ui::button::ButtonSize::Small,
                            false,
                            move || abort(),
                        )),
                )
                .into_element();
        }));
}
