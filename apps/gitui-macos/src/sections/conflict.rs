//! Merge conflict sheet, based on board 14.

use crate::design;
use crate::model::short_name;
use crate::widgets::header_button;
use gitui_git::ConflictStages;
use nsh_base::editor;
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct ConflictActions {
    pub(crate) accept_ours: Arc<dyn Fn()>,
    pub(crate) accept_theirs: Arc<dyn Fn()>,
    pub(crate) accept_both: Arc<dyn Fn()>,
    pub(crate) mark_resolved: Arc<dyn Fn()>,
    pub(crate) select: Arc<dyn Fn(String)>,
    pub(crate) continue_op: Arc<dyn Fn()>,
    pub(crate) abort: Arc<dyn Fn()>,
}

fn pane_head(name: &'static str, meta: &'static str) -> impl IntoElement {
    view()
        .h(px(30.0))
        .flex_row()
        .items_center()
        .gap_1()
        .px_2()
        .bg(move || design::panel())
        .border_b_1()
        .border_color(move || design::line_soft())
        .child(
            text(name)
                .text_size(px(10.5))
                .text_color(move || design::text()),
        )
        .child(
            text(meta)
                .flex_1()
                .text_size(px(9.0))
                .text_color(move || design::text3()),
        )
}

pub(crate) fn conflict_panel(
    conflicts: Signal<Vec<String>>,
    conflict_path: Signal<String>,
    conflict_stages: Signal<Option<ConflictStages>>,
    conflict_result: Signal<String>,
    actions: ConflictActions,
) -> impl IntoElement {
    let open = move || conflict_stages.get().is_some();
    view()
        .absolute()
        .inset(px(0.0))
        .w_full()
        .h_full()
        .z_index(move || if open() { 20 } else { -1 })
        .items_center()
        .justify_center()
        .p(px(24.0))
        .bg(move || {
            if open() {
                AdaptiveColor::from(rgb(0x000000)).opacity(0.34)
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .child(
            view()
                .w(px(1320.0))
                .h(px(700.0))
                .flex_col()
                .rounded(px(12.0))
                .bg(move || design::content())
                .border_1()
                .border_color(move || design::line())
                .overflow_hidden()
                .child(
                    view()
                        .h(px(46.0))
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .bg(move || design::panel())
                        .border_b_1()
                        .border_color(move || design::line())
                        .child(
                            view()
                                .h(px(18.0))
                                .px_1()
                                .rounded(px(4.0))
                                .bg(move || design::selected())
                                .child(
                                    text("MERGE")
                                        .text_size(px(8.5))
                                        .text_color(move || design::primary()),
                                ),
                        )
                        .child(
                            view()
                                .flex_col()
                                .child(
                                    text("Resolve conflicts")
                                        .text_size(px(13.0))
                                        .text_color(move || design::text()),
                                )
                                .child(
                                    text(move || conflict_path.get())
                                        .text_size(px(9.5))
                                        .text_color(move || design::text3()),
                                ),
                        )
                        .child(view().flex_1())
                        .child(header_button("Abort", false, {
                            let action = actions.abort.clone();
                            move || action()
                        })),
                )
                .child(
                    view()
                        .h(px(38.0))
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .px_3()
                        .border_b_1()
                        .border_color(move || design::line())
                        .child(header_button("Accept Ours", false, {
                            let action = actions.accept_ours.clone();
                            move || action()
                        }))
                        .child(header_button("Accept Theirs", false, {
                            let action = actions.accept_theirs.clone();
                            move || action()
                        }))
                        .child(header_button("Accept Both", false, {
                            let action = actions.accept_both.clone();
                            move || action()
                        }))
                        .child(view().flex_1())
                        .child(header_button("Mark Resolved", true, {
                            let action = actions.mark_resolved.clone();
                            move || action()
                        })),
                )
                .child(
                    view()
                        .flex_1()
                        .min_h_0()
                        .flex_row()
                        .child(
                            view()
                                .w(px(224.0))
                                .flex_col()
                                .bg(move || design::panel())
                                .border_r_1()
                                .border_color(move || design::line())
                                .child(
                                    view()
                                        .h(px(30.0))
                                        .flex_row()
                                        .items_center()
                                        .gap_1()
                                        .px_2()
                                        .border_b_1()
                                        .border_color(move || design::line_soft())
                                        .child(
                                            text("Conflicts")
                                                .text_size(px(10.0))
                                                .text_color(move || design::text2()),
                                        )
                                        .child(
                                            text(move || conflicts.get().len().to_string())
                                                .flex_1()
                                                .text_size(px(10.0))
                                                .text_color(move || design::text3()),
                                        ),
                                )
                                .child(
                                    view().flex_1().p_1().child(each(
                                        conflicts,
                                        |path: &String| path.clone(),
                                        move |path, _| {
                                            let selected = path.get();
                                            let select = actions.select.clone();
                                            view()
                                                .h(px(27.0))
                                                .flex_row()
                                                .items_center()
                                                .gap_1()
                                                .px_1()
                                                .rounded(px(5.0))
                                                .bg(move || {
                                                    if conflict_path.get() == path.get() {
                                                        design::selected()
                                                    } else {
                                                        design::panel()
                                                    }
                                                })
                                                .on_press(move || select(selected.clone()))
                                                .child(
                                                    view()
                                                        .w(px(6.0))
                                                        .h(px(6.0))
                                                        .rounded(px(3.0))
                                                        .bg(move || design::danger()),
                                                )
                                                .child(
                                                    text(move || short_name(&path.get()))
                                                        .text_size(px(10.0))
                                                        .text_color(move || design::text()),
                                                )
                                        },
                                    )),
                                ),
                        )
                        .child(
                            view()
                                .flex_1()
                                .min_w_0()
                                .flex_col()
                                .child(
                                    view()
                                        .h(px(62.0))
                                        .flex_row()
                                        .bg(move || design::panel2())
                                        .border_b_1()
                                        .border_color(move || design::line())
                                        .child(
                                            view()
                                                .w(px(86.0))
                                                .flex_col()
                                                .gap_1()
                                                .p_2()
                                                .border_r_1()
                                                .border_color(move || design::line())
                                                .child(
                                                    text("BASE")
                                                        .text_size(px(9.0))
                                                        .text_color(move || design::text3()),
                                                )
                                                .child(
                                                    text("common")
                                                        .text_size(px(9.0))
                                                        .text_color(move || design::text2()),
                                                ),
                                        )
                                        .child(
                                            editor()
                                                .value(move || {
                                                    Some(SharedString::from(
                                                        conflict_stages
                                                            .get()
                                                            .as_ref()
                                                            .map(|stages| stages.base.clone())
                                                            .unwrap_or_default(),
                                                    ))
                                                })
                                                .read_only(true)
                                                .font_size(9.0)
                                                .line_height(13.0)
                                                .flex_1(),
                                        ),
                                )
                                .child(
                                    view()
                                        .flex_1()
                                        .min_h_0()
                                        .flex_row()
                                        .child(
                                            view()
                                                .flex_1()
                                                .min_w_0()
                                                .flex_col()
                                                .border_r_1()
                                                .border_color(move || design::line())
                                                .child(pane_head("Ours", "HEAD"))
                                                .child(
                                                    editor()
                                                        .value(move || {
                                                            Some(SharedString::from(
                                                                conflict_stages
                                                                    .get()
                                                                    .as_ref()
                                                                    .map(|stages| stages.ours.clone())
                                                                    .unwrap_or_default(),
                                                            ))
                                                        })
                                                        .read_only(true)
                                                        .font_size(10.0)
                                                        .line_height(18.0)
                                                        .flex_1(),
                                                ),
                                        )
                                        .child(
                                            view()
                                                .flex_1()
                                                .min_w_0()
                                                .flex_col()
                                                .border_r_1()
                                                .border_color(move || design::line())
                                                .child(pane_head("Result", "editable"))
                                                .child(
                                                    editor()
                                                        .value(move || {
                                                            Some(SharedString::from(
                                                                conflict_result.get(),
                                                            ))
                                                        })
                                                        .on_change(move |value: &SharedString| {
                                                            conflict_result.set(value.to_string())
                                                        })
                                                        .font_size(10.0)
                                                        .line_height(18.0)
                                                        .flex_1(),
                                                ),
                                        )
                                        .child(
                                            view()
                                                .flex_1()
                                                .min_w_0()
                                                .flex_col()
                                                .child(pane_head("Theirs", "incoming"))
                                                .child(
                                                    editor()
                                                        .value(move || {
                                                            Some(SharedString::from(
                                                                conflict_stages
                                                                    .get()
                                                                    .as_ref()
                                                                    .map(|stages| stages.theirs.clone())
                                                                    .unwrap_or_default(),
                                                            ))
                                                        })
                                                        .read_only(true)
                                                        .font_size(10.0)
                                                        .line_height(18.0)
                                                        .flex_1(),
                                                ),
                                        ),
                                ),
                        ),
                )
                .child(
                    view()
                        .h(px(46.0))
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .bg(move || design::panel())
                        .border_t_1()
                        .border_color(move || design::line())
                        .child(
                            text(move || {
                                format!("{} unresolved", conflicts.get().len())
                            })
                            .text_size(px(10.0))
                            .text_color(move || design::text2()),
                        )
                        .child(view().flex_1())
                        .child(header_button("Abort", false, {
                            let action = actions.abort.clone();
                            move || action()
                        }))
                        .child(header_button("Continue", true, {
                            let action = actions.continue_op.clone();
                            move || action()
                        })),
                ),
        )
}
