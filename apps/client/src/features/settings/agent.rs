//! 设置页的 Agent 分区（设计板 13）：逐动作的 Ask / Allow / Deny、自动化程度、
//! 预览时长与受保护分支。

use super::view::{option_chip, section_block};
use crate::design;
use crate::features::agent::commands::AgentCommands;
use crate::features::agent::state::AgentState;
use crate::i18n;
use nsh_core::prelude::*;

/// Agent 权限分区（设计板 13）：逐动作的 Ask / Allow / Deny，并说明边界。
///
/// 状态与用例都来自 `features::agent`（面板用同一份）：设置页只读状态、调用其
/// 用例，agent 不反向依赖设置页。
pub(super) fn agent_page(
    language: Signal<SharedString>,
    state: &AgentState,
    cmd: &AgentCommands,
    branches: Vec<String>,
) -> AnyElement {
    let policies = state.policies;
    let settings = state.settings;
    // `dynamic` 与按钮回调都是 `'static`，用例先克隆一份。
    let page_cmd = cmd.clone();
    let mut rows: Vec<AnyElement> = Vec::new();
    for action in [
        agent::Action::StageAll,
        agent::Action::Commit,
        agent::Action::UnstageAll,
        agent::Action::Fetch,
        agent::Action::Pull,
        agent::Action::Push,
        agent::Action::Discard,
        agent::Action::ReadFiles,
        agent::Action::ForcePush,
    ] {
        let set_permission = page_cmd.clone();
        rows.push(
            view()
                .w_full()
                .h(px(28.0))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    text(move || i18n::t(language.get().as_str(), agent_action_key(action)))
                        .flex_1()
                        .text_size(px(crate::ui::BODY))
                        .line_height(px(15.0))
                        .text_color(move || design::text()),
                )
                .child(
                    text(move || {
                        if action.is_destructive() {
                            i18n::t(language.get().as_str(), "agentPage.destructive")
                        } else {
                            String::new()
                        }
                    })
                    .text_size(px(crate::ui::META))
                    .text_color(move || design::danger()),
                )
                .child(crate::ui::button::button(
                    move || {
                        i18n::t(
                            language.get().as_str(),
                            match policies.get().permission(action) {
                                agent::Permission::Ask => "agent.ask",
                                agent::Permission::Allow => "agent.allow",
                                agent::Permission::Deny => "agent.deny",
                            },
                        )
                    },
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    {
                        let set_permission = set_permission.clone();
                        move || {
                            let next = match policies.get_untracked().permission(action) {
                                agent::Permission::Ask => agent::Permission::Allow,
                                agent::Permission::Allow => agent::Permission::Deny,
                                agent::Permission::Deny => agent::Permission::Ask,
                            };
                            set_permission.set_agent_permission(action, next);
                        }
                    },
                ))
                .into_element(),
        );
    }
    // 自动化程度：Off / Commit / Commit & Push。
    let automation_rows: Vec<AnyElement> = [
        agent::Automation::Off,
        agent::Automation::Commit,
        agent::Automation::CommitAndPush,
    ]
    .iter()
    .map(|mode| {
        let mode = *mode;
        let active = settings.get_untracked().automation == mode;
        let set_automation = page_cmd.clone();
        let language = language;
        return option_chip(
            move || i18n::t(language.get().as_str(), automation_key(mode)),
            active,
            move || set_automation.set_automation(mode),
        )
        .into_element();
    })
    .collect();

    // 预览时长：0 / 3 / 5 / 8 秒。
    let review_rows: Vec<AnyElement> = agent::Settings::REVIEW_CHOICES
        .iter()
        .map(|millis| {
            let millis = *millis;
            let active = settings.get_untracked().review_millis == millis;
            let set_review = page_cmd.clone();
            let language = language;
            let label = if millis == 0 {
                i18n::t(language.get_untracked().as_str(), "agentPage.reviewNone")
            } else {
                format!("{}s", millis / 1000)
            };
            return option_chip(move || label.clone(), active, move || {
                set_review.set_review_millis(millis)
            })
            .into_element();
        })
        .collect();

    // 受保护分支：列出本地分支，点一下切换保护状态。
    let protected_rows: Vec<AnyElement> = branches
        .iter()
        .map(|branch| {
            let name = branch.clone();
            let checked = settings.get_untracked().is_protected(name.as_str());
            let toggle = page_cmd.clone();
            let language = language;
            let label = name.clone();
            let _ = language;
            return crate::ui::check::check_row(move || label.clone(), checked, false, {
                let name = name.clone();
                move |_next: bool| toggle.toggle_protected_branch(name.clone())
            })
            .into_element();
        })
        .collect();

    let mut block = view()
        .w_full()
        .flex_col()
        .gap(px(10.0))
        .child(
            text(move || i18n::t(language.get().as_str(), "agentPage.hint"))
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        )
        .child(rows)
        .child(section_block(
            language,
            "agentPage.automation",
            "agentPage.automationHint",
            view()
                .w_full()
                .flex_row()
                .gap(px(2.0))
                .child(automation_rows)
                .into_element(),
        ))
        .child(section_block(
            language,
            "agentPage.review",
            "agentPage.reviewHint",
            view()
                .w_full()
                .flex_row()
                .gap(px(2.0))
                .child(review_rows)
                .into_element(),
        ));
    if !protected_rows.is_empty() {
        block = block.child(section_block(
            language,
            "agentPage.protected",
            "agentPage.protectedHint",
            view()
                .w_full()
                .flex_col()
                .gap(px(2.0))
                .child(protected_rows)
                .into_element(),
        ));
    }
    return block
        .child(
            text(move || i18n::t(language.get().as_str(), "agentPage.boundary"))
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        )
        .into_element();
}

/// 自动化程度的文案键。
fn automation_key(mode: agent::Automation) -> &'static str {
    return match mode {
        agent::Automation::Off => "agentPage.autoOff",
        agent::Automation::Commit => "agentPage.autoCommit",
        agent::Automation::CommitAndPush => "agentPage.autoCommitPush",
    };
}

fn agent_action_key(action: agent::Action) -> &'static str {
    return match action {
        agent::Action::StageAll => "agent.action.stageAll",
        agent::Action::Commit => "agent.action.commit",
        agent::Action::UnstageAll => "agent.action.unstageAll",
        agent::Action::Fetch => "agent.action.fetch",
        agent::Action::Pull => "agent.action.pull",
        agent::Action::Push => "agent.action.push",
        agent::Action::Discard => "agent.action.discard",
        agent::Action::ReadFiles => "agent.action.readFiles",
        agent::Action::ForcePush => "agent.action.forcePush",
    };
}
