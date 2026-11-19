//! 冲突解决视图（设计板 14）：操作说明、快捷工具栏、Base 与三方结果编辑区。

mod files;

use super::commands::MergeCommands;
use super::state::MergeState;
use crate::app::session::{ConflictDraft, Tab};
use crate::design::{self, metrics};
use crate::i18n;
use domain::conflict::Side;
use nsh_core::prelude::*;

fn conflict_text(bytes: &[u8]) -> Option<String> {
    if bytes.contains(&0) {
        return None;
    }
    return String::from_utf8(bytes.to_vec()).ok();
}

fn code_pane(
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
                .h(px(30.0))
                .flex_shrink_0()
                .px(px(9.0))
                .flex_row()
                .items_center()
                .border_b_1()
                .border_color(move || design::line_soft())
                .child(
                    text(move || i18n::t(language.get().as_str(), title_key))
                        .text_size(px(crate::ui::SECONDARY))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(move || design::text2()),
                ),
        )
        .child(view().w_full().flex_1().min_h_0().child(body))
        .into_element();
}

fn small_separator() -> impl IntoElement {
    return view()
        .w(px(1.0))
        .h(px(18.0))
        .mx(px(2.0))
        .bg(move || design::line());
}

/// `conflict_view` 收本功能状态与用例；语言与当前仓库单独传入。
pub(crate) fn conflict_view(
    state: &MergeState,
    cmd: &MergeCommands,
    language: Signal<SharedString>,
    tab: Memo<Option<Tab>>,
) -> impl IntoElement {
    let base_open = state.base_open;
    let files_open = state.files_open;
    let files_mode = state.files_mode;
    let close = {
        let cmd = cmd.clone();
        move || cmd.close_conflict()
    };
    let files_command = cmd.clone();
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
        .pointer_events(move || {
            if tab.get().is_some_and(|tab| tab.conflict_open) {
                PointerEvents::Auto
            } else {
                PointerEvents::None
            }
        })
        .items_center()
        .justify_center()
        .p(px(metrics::MERGE_LAYER_PAD.1))
        .pt(px(metrics::MERGE_LAYER_PAD.0))
        .bg(move || {
            if tab.get().is_some_and(|tab| tab.conflict_open) {
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

            let paths = current.conflict_paths.clone();
            let unresolved_paths = current.conflicts.clone();
            let unresolved_count = unresolved_paths.len();
            let total = paths.len();
            let index = current.conflict_index.min(total.saturating_sub(1));
            let selected_path = paths.get(index).cloned();
            let selected_is_unresolved = selected_path
                .as_ref()
                .is_some_and(|path| unresolved_paths.contains(path));
            let selected_draft = selected_path
                .as_ref()
                .and_then(|path| current.conflict_drafts.get(path))
                .cloned();
            let selected_resolved = selected_path
                .as_ref()
                .and_then(|path| current.resolved_conflicts.get(path))
                .cloned();
            let conflict_sides = current.conflict_sides.clone();
            let result_draft = selected_resolved
                .as_ref()
                .map(|resolved| resolved.result.clone())
                .or_else(|| selected_draft.clone());
            let result_is_delete = matches!(result_draft.as_ref(), Some(ConflictDraft::Delete));
            let result_is_binary = matches!(result_draft.as_ref(), Some(ConflictDraft::Binary(_)))
                || (result_draft.is_none() && current.conflict_binary);
            let result_text = super::ops::conflict_result(&current);
            let has_markers = domain::conflict::has_markers(result_text.as_str());
            let operation = current.operation;
            let can_continue = unresolved_count == 0
                && current.conflict_drafts.is_empty()
                && operation != git::Operation::None;
            let can_mark = selected_is_unresolved
                && !has_markers
                && (!result_is_binary
                    || matches!(result_draft.as_ref(), Some(ConflictDraft::Binary(_))));

            let detail: AnyElement = if let Some(path) = selected_path.clone() {
                let sides = conflict_sides.clone();
                let ours = code_pane(
                    language,
                    "conflicts.ours",
                    path.clone(),
                    sides.as_ref().and_then(|sides| sides.ours.as_deref()),
                );
                let sides = conflict_sides.clone();
                let theirs = code_pane(
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
                                if matches!(result_draft.as_ref(), Some(ConflictDraft::Binary(_))) {
                                    i18n::t(language.get().as_str(), "conflicts.binarySelected")
                                } else {
                                    i18n::t(language.get().as_str(), "conflicts.binaryChooseSide")
                                }
                            })
                            .text_size(px(crate::ui::SECONDARY))
                            .text_color(move || design::text3()),
                        )
                        .into_element()
                } else if !selected_is_unresolved {
                    crate::ui::text_input::code_view(
                        path.clone(),
                        result_text.clone(),
                        if result_is_delete {
                            i18n::t(
                                language.get_untracked().as_str(),
                                "conflicts.resultWillDelete",
                            )
                        } else {
                            String::new()
                        },
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
                        i18n::t(language.get_untracked().as_str(), placeholder_key),
                        move |value| update_result(value.to_string()),
                    )
                    .into_element()
                };
                let result_header_key = if selected_is_unresolved {
                    "conflicts.resultEditable"
                } else {
                    "conflicts.fileResolved"
                };
                let result_column = view()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex_col()
                    .rounded(px(metrics::RADIUS_CONTROL))
                    .border_1()
                    .border_color(move || design::accent())
                    .bg(move || design::field())
                    .child(
                        view()
                            .w_full()
                            .h(px(30.0))
                            .flex_shrink_0()
                            .px(px(9.0))
                            .flex_row()
                            .items_center()
                            .border_b_1()
                            .border_color(move || design::line_soft())
                            .child(
                                text(move || i18n::t(language.get().as_str(), "conflicts.result"))
                                    .text_size(px(crate::ui::SECONDARY))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(move || design::text2()),
                            )
                            .child(view().flex_1())
                            .child(
                                text(move || i18n::t(language.get().as_str(), result_header_key))
                                    .text_size(px(crate::ui::META))
                                    .text_color(move || {
                                        if selected_is_unresolved {
                                            design::text3()
                                        } else {
                                            design::ok()
                                        }
                                    }),
                            ),
                    )
                    .child(view().w_full().flex_1().min_h_0().child(result_body));
                view()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .p(px(12.0))
                    .child(crate::ui::split::horizontal(vec![
                        crate::ui::split::Pane::flex(ours),
                        crate::ui::split::Pane::flex(result_column.into_element()),
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
                        text(move || i18n::t(language.get().as_str(), "conflicts.allResolved"))
                            .text_size(px(crate::ui::BODY))
                            .text_color(move || design::text2()),
                    )
                    .into_element()
            };

            let base_strip = if base_open.get() && selected_path.is_some() {
                let base = conflict_sides
                    .as_ref()
                    .and_then(|sides| sides.base.as_deref())
                    .and_then(conflict_text)
                    .unwrap_or_default();
                view()
                    .w_full()
                    .h(px(metrics::MERGE_BASE_H))
                    .flex_shrink_0()
                    .px(px(10.0))
                    .flex_row()
                    .items_center()
                    .gap(px(10.0))
                    .bg(move || design::panel())
                    .border_b_1()
                    .border_color(move || design::line())
                    .child(
                        view()
                            .w(px(78.0))
                            .h_full()
                            .flex_shrink_0()
                            .flex_col()
                            .justify_center()
                            .gap(px(3.0))
                            .border_r_1()
                            .border_color(move || design::line_soft())
                            .child(
                                text(move || i18n::t(language.get().as_str(), "conflicts.base"))
                                    .text_size(px(crate::ui::META))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(move || design::text2()),
                            )
                            .child(
                                text("BASE")
                                    .text_size(px(crate::ui::CODE))
                                    .text_color(move || design::text3()),
                            ),
                    )
                    .child(view().flex_1().min_w_0().h_full().overflow_hidden().child(
                        crate::ui::text_input::code_view(
                            selected_path.clone().unwrap_or_default(),
                            base,
                            String::new(),
                        ),
                    ))
                    .child(crate::ui::button::button(
                        move || i18n::t(language.get().as_str(), "conflicts.hideBase"),
                        crate::ui::button::ButtonVariant::Ghost,
                        crate::ui::button::ButtonSize::Small,
                        false,
                        move || base_open.set(false),
                    ))
                    .into_element()
            } else {
                view().h(px(0.0)).into_element()
            };

            let operation_name = super::ops::operation_label(language.get().as_str(), operation);
            let header_summary = i18n::with_arguments(
                language.get().as_str(),
                "conflicts.operationSummary",
                &[
                    ("operation", operation_name.clone()),
                    ("branch", current.branch.clone()),
                ],
            );
            let count_summary = i18n::with_arguments(
                language.get().as_str(),
                "conflicts.fileCounts",
                &[
                    ("files", total.to_string()),
                    ("unresolved", unresolved_count.to_string()),
                ],
            );
            let badge_operation_name = operation_name.clone();
            let footer_abort = abort.clone();
            let footer_continue = continue_op.clone();

            let panel = view()
                .relative()
                .w_full()
                .h_full()
                .max_w(px(metrics::MERGE_SHEET_MAX_W))
                .max_h(px(metrics::MERGE_SHEET_H))
                .flex_col()
                .rounded(px(12.0))
                .border_1()
                .border_color(move || design::line())
                .bg(move || design::content())
                .overflow_hidden()
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
                        .child(crate::ui::badge::badge(
                            move || badge_operation_name.clone(),
                            crate::ui::badge::Tone::Accent,
                        ))
                        .child(
                            view()
                                .flex_1()
                                .min_w_0()
                                .flex_row()
                                .items_center()
                                .gap(px(8.0))
                                .child(
                                    text(move || {
                                        i18n::t(language.get().as_str(), "conflicts.resolveTitle")
                                    })
                                    .text_size(px(crate::ui::BODY))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(move || design::text()),
                                )
                                .child(
                                    text(header_summary.clone())
                                        .text_size(px(crate::ui::SECONDARY))
                                        .text_color(move || design::text3()),
                                )
                                .child(view().flex_1())
                                .child(
                                    text(count_summary.clone())
                                        .text_size(px(crate::ui::SECONDARY))
                                        .text_color(move || design::text2()),
                                ),
                        )
                        .child(crate::ui::button::icon_button(
                            "x",
                            move || i18n::t(language.get().as_str(), "conflicts.closeEditor"),
                            crate::ui::button::ButtonSize::Small,
                            false,
                            {
                                let close = close.clone();
                                move || close()
                            },
                        )),
                )
                .child(
                    view()
                        .w_full()
                        .h(px(metrics::MERGE_TOOLBAR_H))
                        .flex_shrink_0()
                        .px(px(12.0))
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .bg(move || design::content())
                        .border_b_1()
                        .border_color(move || design::line())
                        .child(crate::ui::button::button(
                            move || i18n::t(language.get().as_str(), "conflicts.base"),
                            if base_open.get() {
                                crate::ui::button::ButtonVariant::Secondary
                            } else {
                                crate::ui::button::ButtonVariant::Ghost
                            },
                            crate::ui::button::ButtonSize::Small,
                            selected_path.is_none(),
                            move || base_open.set(!base_open.get_untracked()),
                        ))
                        .child(small_separator())
                        .child(crate::ui::button::button(
                            move || i18n::t(language.get().as_str(), "conflicts.acceptOurs"),
                            crate::ui::button::ButtonVariant::Secondary,
                            crate::ui::button::ButtonSize::Small,
                            !selected_is_unresolved
                                || conflict_sides
                                    .as_ref()
                                    .is_none_or(|sides| sides.ours.is_none()),
                            {
                                let apply_side = apply_side.clone();
                                move || apply_side(Side::Ours)
                            },
                        ))
                        .child(crate::ui::button::button(
                            move || i18n::t(language.get().as_str(), "conflicts.acceptTheirs"),
                            crate::ui::button::ButtonVariant::Secondary,
                            crate::ui::button::ButtonSize::Small,
                            !selected_is_unresolved
                                || conflict_sides
                                    .as_ref()
                                    .is_none_or(|sides| sides.theirs.is_none()),
                            {
                                let apply_side = apply_side.clone();
                                move || apply_side(Side::Theirs)
                            },
                        ))
                        .child(crate::ui::button::button(
                            move || i18n::t(language.get().as_str(), "conflicts.acceptBoth"),
                            crate::ui::button::ButtonVariant::Secondary,
                            crate::ui::button::ButtonSize::Small,
                            !selected_is_unresolved
                                || result_is_binary
                                || !domain::conflict::has_markers(current.conflict_text.as_str()),
                            {
                                let apply_side = apply_side.clone();
                                move || apply_side(Side::Both)
                            },
                        ))
                        .child(small_separator())
                        .child(crate::ui::button::button(
                            move || i18n::t(language.get().as_str(), "conflicts.markResolved"),
                            crate::ui::button::ButtonVariant::Secondary,
                            crate::ui::button::ButtonSize::Small,
                            !can_mark,
                            {
                                let mark = mark.clone();
                                move || mark()
                            },
                        ))
                        .child(view().flex_1())
                        .child(crate::ui::button::icon_button(
                            "chev-left",
                            move || i18n::t(language.get().as_str(), "conflicts.previousFile"),
                            crate::ui::button::ButtonSize::Small,
                            total == 0 || index == 0,
                            {
                                let step = step.clone();
                                move || step(false)
                            },
                        ))
                        .child(
                            text(if total == 0 {
                                "0 / 0".to_string()
                            } else {
                                format!("{} / {}", index + 1, total)
                            })
                            .text_size(px(crate::ui::CODE))
                            .text_color(move || design::text3()),
                        )
                        .child(crate::ui::button::icon_button(
                            "chev-right",
                            move || i18n::t(language.get().as_str(), "conflicts.nextFile"),
                            crate::ui::button::ButtonSize::Small,
                            total == 0 || index + 1 >= total,
                            {
                                let step = step.clone();
                                move || step(true)
                            },
                        ))
                        .child(crate::ui::button::button_icon(
                            move || i18n::t(language.get().as_str(), "conflicts.files"),
                            "folder",
                            if files_open.get() {
                                crate::ui::button::ButtonVariant::Secondary
                            } else {
                                crate::ui::button::ButtonVariant::Ghost
                            },
                            crate::ui::button::ButtonSize::Small,
                            false,
                            move || files_open.set(!files_open.get_untracked()),
                        )),
                )
                .child(base_strip)
                .child(view().w_full().flex_1().min_h_0().child(detail))
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
                        .child(view().w(px(6.0)).h(px(6.0)).rounded(px(3.0)).bg(move || {
                            if unresolved_count == 0 {
                                design::ok()
                            } else {
                                design::danger()
                            }
                        }))
                        .child(
                            text(i18n::with_arguments(
                                language.get().as_str(),
                                "conflicts.unresolved",
                                &[("count", unresolved_count.to_string())],
                            ))
                            .text_size(px(crate::ui::SECONDARY))
                            .text_color(move || design::text2()),
                        )
                        .child(
                            text(move || i18n::t(language.get().as_str(), "conflicts.resultHint"))
                                .text_size(px(crate::ui::SECONDARY))
                                .text_color(move || design::text3()),
                        )
                        .child(view().flex_1())
                        .child(crate::ui::button::button(
                            {
                                let operation_name = operation_name.clone();
                                move || {
                                    i18n::with_arguments(
                                        language.get().as_str(),
                                        "conflicts.abort",
                                        &[("operation", operation_name.clone())],
                                    )
                                }
                            },
                            crate::ui::button::ButtonVariant::Secondary,
                            crate::ui::button::ButtonSize::Small,
                            operation == git::Operation::None,
                            move || footer_abort(),
                        ))
                        .child(crate::ui::button::button(
                            {
                                let operation_name = operation_name.clone();
                                move || {
                                    i18n::with_arguments(
                                        language.get().as_str(),
                                        "conflicts.applyContinue",
                                        &[("operation", operation_name.clone())],
                                    )
                                }
                            },
                            crate::ui::button::ButtonVariant::Primary,
                            crate::ui::button::ButtonSize::Regular,
                            !can_continue,
                            move || footer_continue(),
                        )),
                )
                .into_element();
            return panel;
        }))
        .child(dynamic(move || -> AnyElement {
            if !files_open.get() {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            }
            let Some(current) = tab.get() else {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            };
            if !current.conflict_open {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            }
            let paths = current.conflict_paths.clone();
            let unresolved = current.conflicts.clone();
            let selected = paths.get(current.conflict_index).cloned();
            let choose = files_command.clone();
            let close_popover = files_open;
            let popup = files::picker(
                language,
                paths.clone(),
                unresolved,
                selected,
                files_mode,
                move |path| {
                    if let Some(index) = paths.iter().position(|candidate| candidate == &path) {
                        choose.select_conflict(index);
                    }
                    close_popover.set(false);
                },
            );
            return crate::ui::dialog::popover_overlay(
                move || i18n::t(language.get().as_str(), "conflicts.files"),
                140,
                view().w_full().h_full().relative().child(
                    view()
                        .absolute()
                        .right(px(metrics::MERGE_LAYER_PAD.1 + 12.0))
                        .top(px(metrics::MERGE_LAYER_PAD.0
                            + metrics::MERGE_HEAD_H
                            + metrics::MERGE_TOOLBAR_H))
                        .z_index(1)
                        .child(crate::ui::dialog::modality(popup)),
                ),
                move || files_open.set(false),
            );
        }));
}
