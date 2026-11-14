//! Local agent plan approval and execution.

use crate::design;
use crate::widgets::header_button;
use gitui_agent::{AgentPermissions, PlanStep};
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct AgentActions {
    pub(crate) close: Arc<dyn Fn()>,
    pub(crate) add_stage: Arc<dyn Fn()>,
    pub(crate) add_commit: Arc<dyn Fn()>,
    pub(crate) add_push: Arc<dyn Fn()>,
    pub(crate) clear: Arc<dyn Fn()>,
    pub(crate) approve: Arc<dyn Fn()>,
}

fn step_label(step: &PlanStep) -> String {
    match step {
        PlanStep::StageAll => "Stage all changes".to_string(),
        PlanStep::Commit { message } => format!("Commit: {message}"),
        PlanStep::Push => "Push to upstream".to_string(),
    }
}

pub(crate) fn agent_panel(
    open: Signal<bool>,
    permissions: Signal<AgentPermissions>,
    branch: Signal<SharedString>,
    staged: Signal<usize>,
    steps: Signal<Vec<PlanStep>>,
    log: Signal<SharedString>,
    error: Signal<SharedString>,
    actions: AgentActions,
) -> impl IntoElement {
    view()
        .absolute()
        .inset(px(0.0))
        .w_full()
        .h_full()
        .z_index(move || if open.get() { 48 } else { -1 })
        .items_center()
        .justify_center()
        .p(px(24.0))
        .bg(move || {
            if open.get() {
                AdaptiveColor::from(rgb(0x000000)).opacity(0.28)
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .child(
            view()
                .w(px(760.0))
                .h(px(520.0))
                .flex_col()
                .rounded(px(12.0))
                .bg(move || design::content())
                .border_1()
                .border_color(move || design::line())
                .overflow_hidden()
                .child(
                    view()
                        .h(px(44.0))
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .bg(move || design::panel())
                        .border_b_1()
                        .border_color(move || design::line())
                        .child(
                            text("Agent plan")
                                .text_size(px(13.0))
                                .text_color(move || design::text()),
                        )
                        .child(
                            text(move || format!("{} · {} staged · {}", branch.get(), staged.get(), if permissions.get().push { "push allowed" } else { "push denied" }))
                                .flex_1()
                                .text_size(px(9.5))
                                .text_color(move || design::text3()),
                        )
                        .child(header_button("Close", false, {
                            let action = actions.close.clone();
                            move || action()
                        })),
                )
                .child(
                    view()
                        .flex_1()
                        .min_h_0()
                        .flex_col()
                        .gap_2()
                        .p_3()
                        .child(
                            view()
                                .flex_row()
                                .gap_1()
                                .child(header_button("Add stage", false, {
                                    let action = actions.add_stage.clone();
                                    move || action()
                                }))
                                .child(header_button("Add commit", false, {
                                    let action = actions.add_commit.clone();
                                    move || action()
                                }))
                                .child(header_button("Add push", false, {
                                    let action = actions.add_push.clone();
                                    move || action()
                                }))
                                .child(view().flex_1())
                                .child(header_button("Clear", false, {
                                    let action = actions.clear.clone();
                                    move || action()
                                })),
                        )
                        .child(
                            view()
                                .flex_1()
                                .min_h_0()
                                .flex_col()
                                .gap_1()
                                .p_2()
                                .rounded(px(6.0))
                                .bg(move || design::panel())
                                .child(
                                    text("Proposed plan")
                                        .text_size(px(10.0))
                                        .text_color(move || design::text3()),
                                )
                                .child(each(
                                    steps,
                                    |step: &PlanStep| step_label(step),
                                    |step, _| {
                                        view().h(px(26.0)).child(
                                            text(move || step_label(&step.get()))
                                                .text_size(px(11.0))
                                                .text_color(move || design::text()),
                                        )
                                    },
                                )),
                        )
                        .child(
                            text(move || {
                                if log.get().is_empty() {
                                    String::new()
                                } else {
                                    log.get().to_string()
                                }
                            })
                            .text_size(px(9.5))
                            .text_color(move || design::text3()),
                        )
                        .child(
                            text(move || error.get())
                                .text_size(px(9.5))
                                .text_color(move || design::danger()),
                        )
                        .child(
                            view()
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .child(
                                    text("Hooks always run; --no-verify is unavailable to the agent.")
                                        .flex_1()
                                        .text_size(px(9.0))
                                        .text_color(move || design::text3()),
                                )
                                .child(header_button("Approve & Run", true, {
                                    let action = actions.approve.clone();
                                    move || action()
                                })),
                        ),
                ),
        )
}
