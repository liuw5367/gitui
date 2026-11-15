//! Agent 面板（设计板 12 / 13）：提议的计划、逐步确认、权限与审计。
//!
//! 默认 **Ask first**：写动作必须逐条确认；`Deny` 的动作直接跳过并记入审计。
//! Agent 只提议，执行走应用已有的 Git 动作（同步、提交、暂存），因此权限边界与
//! 手动操作完全一致（见 `docs/AGENT-THREAT-MODEL.md`）。

use crate::design;
use crate::i18n;
use crate::widgets;
use crate::Actions;
use agent::{Action as AgentAction, AuditEntry, Permission, Plan};
use nsh_core::prelude::*;

/// 面板宽度（设计板 `.insp` 同宽）。
pub(crate) const WIDTH: f32 = 320.0;

pub(crate) fn agent_panel(
    language: Signal<SharedString>,
    plan: Signal<Option<Plan>>,
    audit: Signal<Vec<AuditEntry>>,
    policies: Signal<agent::Policies>,
    actions: Actions,
) -> impl IntoElement {
    let make_plan = actions.propose_plan.clone();
    let run_step = actions.run_agent_step.clone();
    let remove_step = actions.remove_agent_step.clone();
    let set_permission = actions.set_agent_permission.clone();
    let now = store::now_unix();
    return view()
        .w(px(WIDTH))
        .h_full()
        .flex_shrink_0()
        .flex_col()
        .bg(move || design::panel())
        .border_l_1()
        .border_color(move || design::line())
        .child(
            view()
                .w_full()
                .px(px(12.0))
                .py(px(10.0))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .border_b_1()
                .border_color(move || design::line())
                .child(widgets::icon("sparkle", 13.0, design::accent))
                .child(
                    text(move || i18n::t(language.get().as_str(), "agent.name"))
                        .text_size(px(crate::ui::BODY))
                        .font_weight(FontWeight(600))
                        .line_height(px(15.0))
                        .text_color(move || design::text()),
                )
                .child(view().flex_1())
                .child(
                    text(move || i18n::t(language.get().as_str(), "agent.localOnly"))
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(move || design::text3()),
                ),
        )
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .p(px(12.0))
                .child(scroll().w_full().h_full().child(
                    view()
                        .w_full()
                        .flex_col()
                        .gap(px(10.0))
                        .child(
                            text(move || i18n::t(language.get().as_str(), "agent.intro"))
                                .text_size(px(crate::ui::SECONDARY))
                                .line_height(px(crate::ui::SECONDARY_LINE))
                                .text_color(move || design::text2()),
                        )
                        .child(widgets::primary_button(
                            move || {
                                if plan.get().is_some() {
                                    i18n::t(language.get().as_str(), "agent.replan")
                                } else {
                                    i18n::t(language.get().as_str(), "agent.plan")
                                }
                            },
                            {
                                let make_plan = make_plan.clone();
                                move || make_plan()
                            },
                        ))
                        .child(dynamic(move || -> AnyElement {
                            match plan.get() {
                                None => view().w_full().h(px(0.0)).into_element(),
                                Some(plan) => {
                                    let mut steps: Vec<AnyElement> = Vec::new();
                                    for (index, step) in plan.steps.iter().enumerate() {
                                        let run_step = run_step.clone();
                                        let permission = policies.get_untracked().permission(step.action);
                                        let denied = permission == Permission::Deny;
                                        let label = step.label.clone();
                                        steps.push(
                                            view()
                                                .w_full()
                                                .py(px(4.0))
                                                .flex_row()
                                                .items_center()
                                                .gap(px(6.0))
                                                .child(
                                                    view()
                                                        .w(px(6.0))
                                                        .h(px(6.0))
                                                        .rounded(px(3.0))
                                                        .bg(move || {
                                                            if denied {
                                                                design::danger()
                                                            } else {
                                                                design::accent()
                                                            }
                                                        }),
                                                )
                                                .child(
                                                    text(label)
                                                        .flex_1()
                                                        .text_size(px(crate::ui::SECONDARY))
                                                        .line_height(px(crate::ui::SECONDARY_LINE))
                                                        .text_color(move || {
                                                            if denied {
                                                                design::text3()
                                                            } else {
                                                                design::text()
                                                            }
                                                        })
                                                        .text_overflow(
                                                            nsh_core::EllipsizeMode::Tail,
                                                        ),
                                                )
                                                .child(
                                                    // 删除用减号：图标集里没有 minus，直接用字形。
                                                    crate::ui::button::button(
                                                        move || "−".to_string(),
                                                        crate::ui::button::ButtonVariant::Ghost,
                                                        crate::ui::button::ButtonSize::Small,
                                                        false,
                                                        {
                                                            let remove = remove_step.clone();
                                                            move || remove(index)
                                                        },
                                                    ),
                                                )
                                                .child(
                                                    crate::ui::button::button(
                                                        {
                                                            let language = language;
                                                            move || {
                                                                i18n::t(
                                                                    language.get().as_str(),
                                                                    if denied {
                                                                        "agent.denied"
                                                                    } else {
                                                                        "agent.run"
                                                                    },
                                                                )
                                                            }
                                                        },
                                                        crate::ui::button::ButtonVariant::Secondary,
                                                        crate::ui::button::ButtonSize::Small,
                                                        denied,
                                                        move || run_step(index),
                                                    )
                                                )
                                                .into_element(),
                                        );
                                    }
                                    let approve = actions.run_plan.clone();
                                    let add_step = actions.add_agent_step.clone();
                                    return view()
                                        .w_full()
                                        .flex_col()
                                        .gap(px(6.0))
                                        .child(
                                            text(move || plan.summary.clone())
                                                .text_size(px(crate::ui::SECONDARY))
                                                .line_height(px(crate::ui::SECONDARY_LINE))
                                                .text_color(move || design::text2()),
                                        )
                                        .child(steps)
                                        .child(
                                            view()
                                                .w_full()
                                                .flex_row()
                                                .items_center()
                                                .gap(px(6.0))
                                                // 菜单要开在点击处，坐标由 ui::menu 取。
                                                .child(crate::ui::menu::menu_button(
                                                    move || {
                                                        i18n::t(
                                                            language.get().as_str(),
                                                            "agent.addStep",
                                                        )
                                                    },
                                                    None,
                                                    {
                                                        let add_step = add_step.clone();
                                                        move || {
                                                            step_menu(language, add_step.clone())
                                                        }
                                                    },
                                                ))
                                                .child(widgets::primary_button(
                                                    move || {
                                                        i18n::t(
                                                            language.get().as_str(),
                                                            "agent.approveRun",
                                                        )
                                                    },
                                                    move || approve(),
                                                )),
                                        )
                                        .into_element();
                                }
                            }
                        }))
                        .child(permissions_block(language, policies, set_permission))
                        .child(audit_block(language, audit, now)),
                )),
        );
}

fn permissions_block(
    language: Signal<SharedString>,
    policies: Signal<agent::Policies>,
    set_permission: std::sync::Arc<dyn Fn(AgentAction, Permission)>,
) -> impl IntoElement {
    let mut rows: Vec<AnyElement> = Vec::new();
    for action in [
        AgentAction::StageAll,
        AgentAction::Commit,
        AgentAction::UnstageAll,
        AgentAction::Fetch,
        AgentAction::Pull,
        AgentAction::Push,
        AgentAction::Discard,
        AgentAction::ReadFiles,
        AgentAction::ForcePush,
    ] {
        let set_permission = set_permission.clone();
        rows.push(
            view()
                .w_full()
                .h(px(24.0))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(move || i18n::t(language.get().as_str(), action_key(action)))
                        .flex_1()
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(crate::ui::SECONDARY_LINE))
                        .text_color(move || design::text2()),
                )
                .child(crate::ui::button::button(
                    {
                        let language = language;
                        move || {
                            i18n::t(
                                language.get().as_str(),
                                match policies.get().permission(action) {
                                    Permission::Ask => "agent.ask",
                                    Permission::Allow => "agent.allow",
                                    Permission::Deny => "agent.deny",
                                },
                            )
                        }
                    },
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    {
                        let set_permission = set_permission.clone();
                        move || {
                            // 点一下循环：Ask → Allow → Deny → Ask。
                            let next = match policies.get_untracked().permission(action) {
                                Permission::Ask => Permission::Allow,
                                Permission::Allow => Permission::Deny,
                                Permission::Deny => Permission::Ask,
                            };
                            set_permission(action, next);
                        }
                    },
                ))
                .into_element(),
        );
    }
    return view()
        .w_full()
        .flex_col()
        .gap(px(2.0))
        .child(
            text(move || i18n::t(language.get().as_str(), "agent.permissions"))
                .text_size(px(crate::ui::SECONDARY))
                .font_weight(FontWeight(600))
                .line_height(px(14.0))
                .text_color(move || design::text3()),
        )
        .child(rows);
}

fn audit_block(
    language: Signal<SharedString>,
    audit: Signal<Vec<AuditEntry>>,
    now: i64,
) -> impl IntoElement {
    return view()
        .w_full()
        .flex_col()
        .gap(px(2.0))
        .child(
            text(move || i18n::t(language.get().as_str(), "agent.audit"))
                .text_size(px(crate::ui::SECONDARY))
                .font_weight(FontWeight(600))
                .line_height(px(14.0))
                .text_color(move || design::text3()),
        )
        .child(dynamic(move || -> AnyElement {
            let entries = audit.get();
            if entries.is_empty() {
                return view()
                    .w_full()
                    .h(px(20.0))
                    .items_center()
                    .child(
                        text(move || i18n::t(language.get().as_str(), "agent.auditEmpty"))
                            .text_size(px(crate::ui::SECONDARY))
                            .text_color(move || design::text3()),
                    )
                    .into_element();
            }
            let mut rows: Vec<AnyElement> = Vec::new();
            for entry in entries.iter().take(8) {
                let action = entry.action.clone();
                let detail = entry.detail.clone();
                let ok = entry.ok;
                let timestamp = entry.timestamp;
                rows.push(
                    view()
                        .w_full()
                        .py(px(2.0))
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            view()
                                .w(px(5.0))
                                .h(px(5.0))
                                .rounded(px(3.0))
                                .bg(move || {
                                    if ok {
                                        design::ok()
                                    } else {
                                        design::danger()
                                    }
                                }),
                        )
                        .child(
                            text(action)
                                .text_size(px(crate::ui::CODE))
                                
                                .line_height(px(12.0))
                                .text_color(move || design::text2()),
                        )
                        .child(
                            text(detail)
                                .flex_1()
                                .text_size(px(crate::ui::SECONDARY))
                                .line_height(px(crate::ui::SECONDARY_LINE))
                                .text_color(move || design::text3())
                                .text_overflow(nsh_core::EllipsizeMode::Tail),
                        )
                        .child(
                            text(move || {
                                crate::dates::format(
                                    language.get().as_str(),
                                    timestamp,
                                    now,
                                )
                            })
                            .text_size(px(crate::ui::SECONDARY))
                            .text_color(move || design::text3()),
                        )
                        .into_element(),
                );
            }
            return view().w_full().flex_col().child(rows).into_element();
        }));
}

/// 「补一步」菜单：每个可执行动作一项，选中即往计划末尾追加。
fn step_menu(
    language: Signal<SharedString>,
    add_step: std::sync::Arc<dyn Fn(&'static str)>,
) -> Vec<nsh_core::element::desktop::MenuItem> {
    let lang = language.get_untracked();
    return AgentAction::ALL
        .into_iter()
        .map(|action| {
            let add_step = add_step.clone();
            let id = action.id();
            nsh_core::element::desktop::MenuItem::new(i18n::t(lang.as_str(), action_key(action)))
                .on_select(move || add_step(id))
        })
        .collect();
}

pub(crate) fn action_key(action: AgentAction) -> &'static str {
    return match action {
        AgentAction::StageAll => "agent.action.stageAll",
        AgentAction::Commit => "agent.action.commit",
        AgentAction::UnstageAll => "agent.action.unstageAll",
        AgentAction::Fetch => "agent.action.fetch",
        AgentAction::Pull => "agent.action.pull",
        AgentAction::Push => "agent.action.push",
        AgentAction::Discard => "agent.action.discard",
        AgentAction::ReadFiles => "agent.action.readFiles",
        AgentAction::ForcePush => "agent.action.forcePush",
    };
}
