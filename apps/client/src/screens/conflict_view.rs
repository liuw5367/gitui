//! 冲突解决视图（设计板 14）。
//!
//! 三方对照：左侧 ours、右侧 theirs、下方可编辑的结果区。顶部可切换冲突文件、
//! 显示 base，底部是接受一侧、标记已解决、继续与放弃。
//!
//! 结果区是一个真实的编辑框：接受某一侧只是把它填进去，仍可继续手改；标记已解决
//! 会把内容写回文件并 `git add`。

use crate::design::{self, metrics};
use crate::i18n;
use crate::model::Tab;
use crate::widgets;
use crate::Actions;
use domain::conflict::{self, Side};
use nsh_core::prelude::*;

/// 受控的只读文本块：带底色和等宽字体，长内容可滚动。
fn pane(language: Signal<SharedString>, title_key: &'static str, body: String) -> AnyElement {
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
        .child(
            view().w_full().flex_1().min_h_0().child(
                scroll().w_full().h_full().child(
                    view().w_full().p(px(7.0)).child(
                        text(body)
                            .text_size(px(crate::ui::CODE))
                            .line_height(px(crate::ui::CODE_LINE))
                            .text_color(move || design::text2()),
                    ),
                ),
            ),
        )
        .into_element();
}

/// 从冲突文件文本里取某一侧的展示内容。
fn side_text(text: &str, side: Side) -> String {
    let segments = conflict::parse(text);
    let mut out: Vec<String> = Vec::new();
    for segment in segments {
        match segment {
            conflict::Segment::Common(lines) => out.extend(lines),
            conflict::Segment::Conflict { ours, theirs, .. } => match side {
                Side::Ours => out.extend(ours),
                Side::Theirs => out.extend(theirs),
                Side::Both => {
                    out.extend(ours);
                    out.extend(theirs);
                }
            },
        }
    }
    return out.join("\n");
}

/// base 侧内容（diff3 风格才有）。
fn base_text(text: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for segment in conflict::parse(text) {
        match segment {
            conflict::Segment::Common(lines) => out.extend(lines),
            conflict::Segment::Conflict { base, ours, .. } => {
                out.extend(base.unwrap_or(ours));
            }
        }
    }
    return out.join("\n");
}

pub(crate) fn conflict_view(
    language: Signal<SharedString>,
    tab: Memo<Option<Tab>>,
    result: Signal<String>,
    base_open: Signal<bool>,
    actions: Actions,
) -> impl IntoElement {
    let close = actions.close_conflict.clone();
    let select = actions.select_conflict.clone();
    let apply_side = actions.apply_side.clone();
    let mark = actions.mark_resolved.clone();
    let step = actions.step_conflict.clone();
    let continue_op = actions.continue_op.clone();
    let abort = actions.abort_operation.clone();
    return view()
        .absolute()
        .inset(px(0.0))
        .z_index(130)
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
            let conflict_text = current.conflict_text.clone();
            let unresolved = total;
            let operation = current.operation.id().to_string();
            let can_continue = unresolved == 0;
            let language_value = language;
            let close = close.clone();
            let select = select.clone();
            let apply_side = apply_side.clone();
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
                    let name = path.rsplit('/').next().unwrap_or(path).to_string();
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
                                           false,
                                           move || base_open.set(!base_open.get_untracked()),
                                       ))
                                .child(crate::ui::button::button(
                                           
                                    move || "‹".to_string(),
                                           crate::ui::button::ButtonVariant::Secondary,
                                           crate::ui::button::ButtonSize::Small,
                                           false,
                                           {
                                        let step = step.clone();
                                        move || step(false)
                                    },
                                       ))
                                .child(
                                    text(move || format!("{} / {}", index + 1, total.max(1)))
                                        .text_size(px(crate::ui::CODE))
                                        .line_height(px(crate::ui::CODE_LINE))
                                        .text_color(move || design::text3()),
                                )
                                .child(crate::ui::button::button(
                                           
                                    move || "›".to_string(),
                                           crate::ui::button::ButtonVariant::Secondary,
                                           crate::ui::button::ButtonSize::Small,
                                           false,
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
                                            .line_height(px(12.0))
                                            .text_color(move || design::text2()),
                                        ),
                                )
                                .child(
                                    view().w_full().flex_1().min_h_0().p(px(6.0)).child(
                                        scroll().w_full().h_full().child(
                                            view().w_full().flex_col().gap(px(1.0)).child(chips),
                                        ),
                                    ),
                                ),
                        )
                        .child(
                            view()
                                .flex_1()
                                .min_w_0()
                                .h_full()
                                .p(px(12.0))
                                .child(crate::ui::split::horizontal(vec![
                                    crate::ui::split::Pane::flex(pane(
                                        language,
                                        "conflicts.ours",
                                        side_text(conflict_text.as_str(), Side::Ours),
                                    )),
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
                                .child(
                                    view().w_full().flex_1().min_h_0().p(px(7.0)).child(
                                        scroll().w_full().h_full().child(
                                            nsh_base::input()
                                                .value(result)
                                                .on_change(move |value: &str| {
                                                    result.set(value.to_string())
                                                })
                                                .text_size(px(crate::ui::CODE))
                                                .line_height(px(crate::ui::CODE_LINE))
                                                .flex_1(),
                                        ),
                                    ),
                                ),
                            ),
                                    crate::ui::split::Pane::flex(pane(
                                        language,
                                        "conflicts.theirs",
                                        side_text(conflict_text.as_str(), Side::Theirs),
                                    )),
                                ])),
                        ),
                )
                // base 条：只在 diff3 风格有内容，且需要用户展开。
                .child(
                    view()
                        .w_full()
                        .px(px(12.0))
                        .when(base_open.get(), |this| {
                            let base = base_text(conflict_text.as_str());
                            this.child(
                                view()
                                    .w_full()
                                    .h(px(metrics::MERGE_BASE_H))
                                    .flex_col()
                                    .rounded(px(metrics::RADIUS_CONTROL))
                                    .border_1()
                                    .border_color(move || design::line_soft())
                                    .bg(move || design::content())
                                    .child(
                                        text(move || {
                                            i18n::t(
                                                language_value.get().as_str(),
                                                "conflicts.basePane",
                                            )
                                        })
                                        .text_size(px(crate::ui::SECONDARY))
                                        .px(px(8.0))
                                        .py(px(4.0))
                                        .text_color(move || design::text3()),
                                    )
                                    .child(
                                        view().w_full().p(px(7.0)).child(
                                            text(base)
                                                .text_size(px(crate::ui::CODE))
                                                .line_height(px(crate::ui::CODE_LINE))
                                                .text_color(move || design::text2()),
                                        ),
                                    ),
                            )
                        }),
                )
                // 底部：接受一侧、标记、继续、放弃。
                .child(
                    view()
                        .w_full()
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
                                   false,
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
                                   false,
                                   {
                                let apply_side = apply_side.clone();
                                move || apply_side(Side::Theirs)
                            },
                               ))
                        .child(crate::ui::button::button(
                                   
                            move || i18n::t(language_value.get().as_str(), "conflicts.acceptBoth"),
                                   crate::ui::button::ButtonVariant::Secondary,
                                   crate::ui::button::ButtonSize::Small,
                                   false,
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
                                   false,
                                   {
                                let mark = mark.clone();
                                move || mark()
                            },
                               ))
                        .child(widgets::primary_button(
                            {
                                let operation = operation.clone();
                                move || {
                                    i18n::with_arguments(
                                        language_value.get().as_str(),
                                        "conflicts.applyContinue",
                                        &[("operation", operation.clone())],
                                    )
                                }
                            },
                            {
                                let continue_op = continue_op.clone();
                                move || {
                                    if can_continue {
                                        continue_op();
                                    }
                                }
                            },
                        ))
                        .child(crate::ui::button::button(
                                   
                            {
                                let operation = operation.clone();
                                move || {
                                    i18n::with_arguments(
                                        language_value.get().as_str(),
                                        "conflicts.abort",
                                        &[("operation", operation.clone())],
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
        }))
}
