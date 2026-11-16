//! Agent 的用例：提议计划、逐步/整单执行、编辑计划、权限与运行设置。
//!
//! 方法体由原 `Actions` 里的闭包原样搬来，只把捕获的局部变量换成 `self.*`；
//! 这里只调 `app::ops` / `commit::ops` / `agent` 并更新状态，不写布局。

use super::state::AgentState;
use crate::app::ops::{stage_all_or_none, sync};
use crate::app::session::Sync;
use crate::features::SessionHandle;
use crate::features::commit::ops::commit_staged;
use crate::i18n;
use crate::interactions;
use nsh_core::prelude::*;
use std::path::PathBuf;

#[derive(Clone)]
pub(crate) struct AgentCommands {
    pub(crate) state: AgentState,
    pub(crate) session: SessionHandle,
    /// 计划步骤与提示文案要按当前语言取。
    pub(crate) language: Signal<SharedString>,
    /// 状态栏短提示（没有可执行步骤时说明原因）。
    pub(crate) notice: Signal<Option<String>>,
    /// 权限、运行设置与审计的落盘目录。
    pub(crate) store_dir: PathBuf,
    /// 提交说明（提交功能的状态）：提议计划与执行 Commit 步骤时用它兜底。
    pub(crate) commit_summary: Signal<String>,
}

impl AgentCommands {
    /// 审计写盘（原 `persist_audit`）：一行一条，字段用 `\t` 分隔。
    fn persist_audit(&self) {
        let lines: Vec<String> = self
            .state
            .audit
            .get_untracked()
            .iter()
            .map(|entry| {
                format!(
                    "{}\t{}\t{}\t{}",
                    entry.timestamp,
                    entry.action,
                    if entry.ok { "ok" } else { "fail" },
                    entry.detail
                )
            })
            .collect();
        store::save_agent_audit(&self.store_dir, &lines);
    }

    /// 运行设置写盘（原 `persist_agent`）：只覆盖这三项，权限条目原样保留。
    fn persist_settings(&self, settings: &agent::Settings) {
        let pairs = settings.write_pairs();
        let mut lines: Vec<(String, String)> = store::load_agent_policies(&self.store_dir);
        for (key, value) in pairs {
            lines.retain(|(existing, _)| existing != &key);
            lines.push((key, value));
        }
        store::save_agent_policies(&self.store_dir, &lines);
    }

    /// 按当前改动提议一份计划（原 `propose_plan`）。
    pub(crate) fn propose_plan(&self) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let agent_plan = self.state.plan;
        let commit_summary = self.commit_summary;
        let current = active.get_untracked();
        let list = tabs.get_untracked();
        let Some(tab) = list.get(current) else {
            return;
        };
        let staged = tab.changes.iter().filter(|entry| entry.staged).count();
        let unstaged = tab.changes.len() - staged;
        let message = commit_summary.get_untracked();
        let plan = agent::propose(
            "本地计划：整理当前仓库",
            staged,
            unstaged,
            tab.ahead,
            tab.behind,
            message.as_str(),
        );
        agent_plan.set(Some(plan));
    }

    /// 执行计划里的第 `index` 步（原 `run_agent_step`）：受保护分支与 Deny 先拦下，
    /// 其余落到已有的 Git 动作，结果记入审计。
    pub(crate) fn run_agent_step(&self, index: usize) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let agent_plan = self.state.plan;
        let agent_audit = self.state.audit;
        let agent_policies = self.state.policies;
        let agent_settings = self.state.settings;
        let commit_summary = self.commit_summary;
        let Some(plan) = agent_plan.get_untracked() else {
            return;
        };
        let Some(step) = plan.steps.get(index).cloned() else {
            return;
        };
        let now = store::now_unix();
        // 受保护分支：写动作一律不执行，记审计并说明原因。
        let branch_now = {
            let list = tabs.get_untracked();
            list.get(active.get_untracked())
                .map(|tab| tab.branch.clone())
                .unwrap_or_default()
        };
        if let Some(reason) = agent_settings
            .get_untracked()
            .blocks(step.action, branch_now.as_str())
        {
            let mut audit = agent_audit.get_untracked();
            audit.insert(
                0,
                agent::AuditEntry::new(now, step.action, false, reason.as_str()),
            );
            audit.truncate(agent::Audit::LIMIT);
            agent_audit.set(audit);
            self.persist_audit();
            return;
        }
        // 权限：Deny 直接记审计并跳过；Ask 在界面上就是用户按了这一行。
        if agent_policies.get_untracked().is_denied(step.action) {
            let mut audit = agent_audit.get_untracked();
            audit.insert(
                0,
                agent::AuditEntry::new(now, step.action, false, "denied by policy"),
            );
            audit.truncate(agent::Audit::LIMIT);
            agent_audit.set(audit);
            self.persist_audit();
            return;
        }
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        let Some(tab) = list.get_mut(current) else {
            return;
        };
        let detail = match step.action {
            agent::Action::StageAll => {
                stage_all_or_none(tab, true);
                "staged all".to_string()
            }
            agent::Action::UnstageAll => {
                stage_all_or_none(tab, false);
                "unstaged all".to_string()
            }
            // 只读动作与强推不是 plan 的步骤：前者不产生副作用，后者必须由人
            // 在同步面板里勾选确认，不交给 Agent 自动执行。
            agent::Action::ReadFiles | agent::Action::ForcePush => "skipped".to_string(),
            agent::Action::Commit => {
                tab.summary = step
                    .argument
                    .clone()
                    .unwrap_or_else(|| commit_summary.get_untracked());
                commit_staged(tab);
                tab.hook_log.clone().unwrap_or_default()
            }
            agent::Action::Fetch => {
                sync(tab, Sync::Fetch);
                tab.sync_message.clone().unwrap_or_default()
            }
            agent::Action::Pull => {
                sync(tab, Sync::Pull);
                tab.sync_message.clone().unwrap_or_default()
            }
            agent::Action::Push => {
                sync(tab, Sync::Push);
                tab.sync_message.clone().unwrap_or_default()
            }
            agent::Action::Discard => "skipped: destructive".to_string(),
        };
        let ok = match step.action {
            agent::Action::Commit => tab.hook_ok,
            agent::Action::Fetch | agent::Action::Pull | agent::Action::Push => !tab.sync_failed,
            agent::Action::Discard => false,
            _ => true,
        };
        let mut audit = agent_audit.get_untracked();
        audit.insert(0, agent::AuditEntry::new(now, step.action, ok, detail));
        audit.truncate(agent::Audit::LIMIT);
        agent_audit.set(audit);
        tabs.set(list);
        self.persist_audit();
    }

    /// 删掉计划里的一步（原 `remove_agent_step`）。
    pub(crate) fn remove_agent_step(&self, index: usize) {
        let agent_plan = self.state.plan;
        let Some(mut plan) = agent_plan.get_untracked() else {
            return;
        };
        if plan.remove_step(index) {
            agent_plan.set(Some(plan));
        }
    }

    /// 往计划末尾补一步（原 `add_agent_step`）：参数是动作 id。
    pub(crate) fn add_agent_step(&self, id: &'static str) {
        let agent_plan = self.state.plan;
        let language = self.language;
        let Some(action) = agent::Action::ALL
            .into_iter()
            .find(|candidate| candidate.id() == id)
        else {
            return;
        };
        let lang = language.get_untracked();
        let label = i18n::t(lang.as_str(), action_key(action));
        let mut plan = agent_plan.get_untracked().unwrap_or_default();
        if plan.summary.trim().is_empty() {
            plan.summary = i18n::t(lang.as_str(), "agent.manualSummary");
        }
        plan.push_step(agent::Step::new(action, label));
        agent_plan.set(Some(plan));
    }

    /// Approve & Run（原 `run_plan`）：先按策略算出可执行范围，列出来让用户确认，
    /// 再按顺序执行。
    pub(crate) fn run_plan(&self) {
        let agent_plan = self.state.plan;
        let agent_policies = self.state.policies;
        let agent_settings = self.state.settings;
        let tabs = self.session.tabs;
        let active = self.session.active;
        let language = self.language;
        let notice = self.notice;
        let Some(plan) = agent_plan.get_untracked() else {
            return;
        };
        let branch = tabs
            .get_untracked()
            .get(active.get_untracked())
            .map(|tab| tab.branch.clone())
            .unwrap_or_default();
        let runnable = plan.runnable(
            &agent_policies.get_untracked(),
            &agent_settings.get_untracked(),
            branch.as_str(),
        );
        let lang = language.get_untracked();
        if runnable.is_empty() {
            interactions::notify(notice, i18n::t(lang.as_str(), "agent.nothingRunnable"));
            return;
        }
        let listed: Vec<String> = runnable
            .iter()
            .filter_map(|index| plan.steps.get(*index))
            .map(|step| format!("• {}", step.label))
            .collect();
        let message = format!(
            "{}\n\n{}",
            i18n::t(lang.as_str(), "agent.approveMessage"),
            listed.join("\n")
        );
        let action_label = i18n::t(lang.as_str(), "agent.approveRun");
        let title = i18n::t(lang.as_str(), "agent.approveTitle");
        // 确认框返回后要逐条执行，先把用例克隆一份带进异步块。
        let run_step = self.clone();
        spawn(async move {
            let ok = interactions::confirm(title, message, "runPlan", action_label, false).await;
            if !ok {
                return;
            }
            for index in runnable {
                run_step.run_agent_step(index);
            }
        });
    }

    /// 设置某个动作的权限（原 `set_agent_permission`）：改完立刻落盘。
    pub(crate) fn set_agent_permission(
        &self,
        action: agent::Action,
        permission: agent::Permission,
    ) {
        let agent_policies = self.state.policies;
        let store_dir = self.store_dir.clone();
        let mut policies = agent_policies.get_untracked();
        policies.set(action, permission);
        agent_policies.set(policies.clone());
        let entries: Vec<(String, String)> = agent::Action::ALL
            .into_iter()
            .map(|candidate| {
                (
                    candidate.id().to_string(),
                    policies.permission(candidate).id().to_string(),
                )
            })
            .collect();
        store::save_agent_policies(&store_dir, &entries);
    }

    /// 设置自动化程度（原 `set_automation`）：改动即持久化。
    pub(crate) fn set_automation(&self, mode: agent::Automation) {
        let agent_settings = self.state.settings;
        let mut next = agent_settings.get_untracked();
        next.automation = mode;
        agent_settings.set(next.clone());
        self.persist_settings(&next);
    }

    /// 设置自动执行前的预览时长（原 `set_review_millis`）。
    pub(crate) fn set_review_millis(&self, millis: u32) {
        let agent_settings = self.state.settings;
        let mut next = agent_settings.get_untracked();
        next.review_millis = millis;
        agent_settings.set(next.clone());
        self.persist_settings(&next);
    }

    /// 切换某个分支的受保护状态（原 `toggle_protected_branch`）。
    pub(crate) fn toggle_protected_branch(&self, branch: String) {
        let agent_settings = self.state.settings;
        let mut next = agent_settings.get_untracked();
        if next.is_protected(branch.as_str()) {
            next.protected.retain(|name| name != &branch);
        } else {
            next.protected.push(branch);
        }
        agent_settings.set(next.clone());
        self.persist_settings(&next);
    }
}

/// 动作的文案键：面板、设置页与「补一步」菜单共用。
pub(crate) fn action_key(action: agent::Action) -> &'static str {
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
