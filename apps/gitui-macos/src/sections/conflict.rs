//! Three-way conflict panel.

use gitui_git::ConflictStages;
use nsh_base::{editor, theme};
use nsh_core::prelude::*;
use std::sync::Arc;

use crate::widgets::{conflict_pane, header_button};

#[derive(Clone)]
pub(crate) struct ConflictActions {
    pub(crate) accept_ours: Arc<dyn Fn()>,
    pub(crate) accept_theirs: Arc<dyn Fn()>,
    pub(crate) accept_both: Arc<dyn Fn()>,
    pub(crate) mark_resolved: Arc<dyn Fn()>,
}

pub(crate) fn conflict_panel(
    conflicts: Signal<Vec<String>>,
    conflict_path: Signal<String>,
    conflict_stages: Signal<Option<ConflictStages>>,
    conflict_result: Signal<String>,
    actions: ConflictActions,
) -> impl IntoElement {
    view()
        .h(move || {
            if conflict_stages.get().is_some() {
                px(250.0)
            } else {
                px(0.0)
            }
        })
        .flex_col()
        .gap_2()
        .p_2()
        .bg(move || theme().background)
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    text(move || {
                        let total = conflicts.get().len();
                        let current = conflicts
                            .get()
                            .iter()
                            .position(|path| path == &conflict_path.get())
                            .map(|index| index + 1)
                            .unwrap_or(0);
                        format!("Conflict · {} · {} of {}", conflict_path.get(), current, total)
                    })
                    .text_size(px(10.0))
                    .text_color(|| rgb(0xff3b30)),
                )
                .child(view().flex_1())
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
                .child(header_button("Mark Resolved", true, {
                    let action = actions.mark_resolved.clone();
                    move || action()
                })),
        )
        .child(
            view()
                .flex_1()
                .flex_row()
                .gap_2()
                .child(conflict_pane("Base", move || {
                    conflict_stages
                        .get()
                        .as_ref()
                        .map(|stages| stages.base.clone())
                        .unwrap_or_default()
                }))
                .child(conflict_pane("Ours", move || {
                    conflict_stages
                        .get()
                        .as_ref()
                        .map(|stages| stages.ours.clone())
                        .unwrap_or_default()
                }))
                .child(conflict_pane("Theirs", move || {
                    conflict_stages
                        .get()
                        .as_ref()
                        .map(|stages| stages.theirs.clone())
                        .unwrap_or_default()
                }))
                .child(
                    view()
                        .flex_1()
                        .flex_col()
                        .gap_1()
                        .child(
                            text("Result")
                                .text_size(px(9.0))
                                .text_color(move || theme().primary),
                        )
                        .child(
                            editor()
                                .value(move || Some(SharedString::from(conflict_result.get())))
                                .on_change(move |value: &SharedString| {
                                    conflict_result.set(value.to_string())
                                })
                                .font_size(10.0)
                                .line_height(15.0)
                                .flex_1(),
                        ),
                ),
        )
}
