//! Right inspector: commit composer and agent policy.

use crate::design;
use crate::sections::CommitActions;
use crate::widgets::header_button;
use crate::{InspectorTab, Mode};
use gitui_agent::{AgentPermissions, AgentPlan, PlanStep};
use gitui_git::StatusEntry;
use nsh_base::input;
use nsh_core::prelude::*;
use std::sync::Arc;

fn tab(
    label: &'static str,
    target: InspectorTab,
    current: Signal<InspectorTab>,
    on_tab: Arc<dyn Fn(InspectorTab)>,
) -> impl IntoElement {
    view()
        .h(px(24.0))
        .px_2()
        .rounded(px(4.0))
        .bg(move || {
            if current.get() == target {
                design::content()
            } else {
                design::panel()
            }
        })
        .on_press(move || on_tab(target))
        .child(
            text(label)
                .text_size(px(11.5))
                .text_color(move || {
                    if current.get() == target {
                        design::text()
                    } else {
                        design::text3()
                    }
                }),
        )
}

pub(crate) fn inspector(
    mode: Signal<Mode>,
    focus: Signal<bool>,
    tab_state: Signal<InspectorTab>,
    permissions: Signal<AgentPermissions>,
    branch: Signal<SharedString>,
    staged: Signal<usize>,
    unstaged: Signal<usize>,
    staged_entries: Signal<Vec<StatusEntry>>,
    message: Signal<String>,
    commit_log: Signal<SharedString>,
    actions: CommitActions,
    on_open_agent: Arc<dyn Fn()>,
) -> impl IntoElement {
    view()
        .w(move || {
            if !focus.get() && mode.get() == Mode::Changes {
                px(360.0)
            } else {
                px(0.0)
            }
        })
        .flex_col()
        .min_h_0()
        .bg(move || design::panel())
        .border_l_1()
        .border_color(move || design::line())
        .child(
            view()
                .h(px(33.0))
                .flex_row()
                .items_center()
                .gap_3()
                .px_3()
                .border_b_1()
                .border_color(move || design::line())
                .child(tab(
                    "Commit",
                    InspectorTab::Commit,
                    tab_state,
                    Arc::new(move |next| tab_state.set(next)),
                ))
                .child(tab(
                    "Agent",
                    InspectorTab::Agent,
                    tab_state,
                    Arc::new(move |next| tab_state.set(next)),
                )),
        )
        .child(
            view()
                .flex_1()
                .min_h_0()
                .flex_row()
                .child(
                    view()
                        .w(move || {
                            if tab_state.get() == InspectorTab::Commit {
                                px(360.0)
                            } else {
                                px(0.0)
                            }
                        })
                        .h_full()
                        .flex_col()
                        .gap_2()
                        .p_2()
                        .child(
                            text(move || format!("Commit to {}", branch.get()))
                                .text_size(px(11.0))
                                .text_color(move || design::text2()),
                        )
                        .child(
                            text(move || {
                                format!("{} staged · {} unstaged", staged.get(), unstaged.get())
                            })
                            .text_size(px(10.0))
                            .text_color(move || design::text3()),
                        )
                        .child(
                            text("Summary")
                                .text_size(px(10.5))
                                .text_color(move || design::text2()),
                        )
                        .child(
                            input()
                                .value(message)
                                .on_change(move |text: &str| message.set(text.to_string()))
                                .placeholder("Commit summary")
                                .text_size(px(11.0)),
                        )
                        .child(
                            text(move || format!("Staged files · {}", staged_entries.get().len()))
                                .text_size(px(10.0))
                                .text_color(move || design::text3()),
                        )
                        .child(each(
                            staged_entries,
                            |entry: &StatusEntry| entry.path.clone(),
                            |entry, _| {
                                view().h(px(22.0)).child(
                                    text(move || entry.get().path)
                                        .text_size(px(10.0))
                                        .text_color(move || design::text()),
                                )
                            },
                        ))
                        .child(view().flex_1())
                        .child(
                            text(move || {
                                if commit_log.get().is_empty() {
                                    String::new()
                                } else {
                                    commit_log.get().to_string()
                                }
                            })
                            .text_size(px(9.0))
                            .text_color(move || design::text3()),
                        )
                        .child(
                            view()
                                .flex_row()
                                .gap_1()
                                .child(header_button("Commit", true, {
                                    let action = actions.commit.clone();
                                    move || action()
                                }))
                                .child(header_button("No hooks", false, {
                                    let action = actions.no_hooks.clone();
                                    move || action()
                                }))
                                .child(header_button("Amend", false, {
                                    let action = actions.amend.clone();
                                    move || action()
                                })),
                        ),
                )
                .child(
                    view()
                        .w(move || {
                            if tab_state.get() == InspectorTab::Agent {
                                px(360.0)
                            } else {
                                px(0.0)
                            }
                        })
                        .h_full()
                        .flex_col()
                        .gap_2()
                        .p_2()
                        .child(
                            text("gitui Agent")
                                .text_size(px(12.0))
                                .text_color(move || design::text()),
                        )
                        .child(
                            text("Provider not connected · manual plan approval available.")
                                .text_size(px(9.5))
                                .text_color(move || design::text3()),
                        )
                        .child(
                            text(move || {
                                format!(
                                    "Read {} · Stage {} · Commit {} · Push {}",
                                    if permissions.get().read { "on" } else { "off" },
                                    if permissions.get().stage { "on" } else { "off" },
                                    if permissions.get().commit { "on" } else { "off" },
                                    if permissions.get().push { "on" } else { "off" },
                                )
                            })
                            .text_size(px(9.5))
                            .text_color(move || design::text2()),
                        )
                        .child(
                            text("Configure in Settings → Agent.")
                                .text_size(px(9.5))
                                .text_color(move || design::text3()),
                        )
                        .child(
                            text("Proposed plan")
                                .text_size(px(10.5))
                                .text_color(move || design::text2()),
                        )
                        .child(
                            text(move || {
                                format!("1. Commit {} staged file(s)", staged.get())
                            })
                            .text_size(px(9.5))
                            .text_color(move || design::text()),
                        )
                        .child(
                            text(move || {
                                if permissions.get().push {
                                    format!("2. Push {}", branch.get())
                                } else {
                                    "2. Push · blocked by policy".to_string()
                                }
                            })
                            .text_size(px(9.5))
                            .text_color(move || design::text()),
                        )
                        .child(
                            text(move || {
                                let plan = AgentPlan {
                                    steps: vec![
                                        PlanStep::Commit {
                                            message: message.get(),
                                        },
                                        PlanStep::Push,
                                    ],
                                };
                                match plan.validate(&permissions.get()) {
                                    Ok(()) => "Plan ready · approval required".to_string(),
                                    Err(err) => err.to_string(),
                                }
                            })
                            .text_size(px(9.0))
                            .text_color(move || design::text3()),
                        )
                        .child(header_button("Review plan", true, move || on_open_agent()))
                        .child(view().flex_1())
                        .child(
                            text("The agent never uses --no-verify or reads Keychain.")
                                .text_size(px(9.0))
                                .text_color(move || design::text3()),
                        ),
                ),
        )
}
