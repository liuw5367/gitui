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
use std::sync::mpsc;
use std::time::Duration;

struct AgentSyncResult {
    tab: crate::app::session::Tab,
    cancelled: bool,
}

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
    /// Agent 的网络步骤与手动同步共用按需 AskPass 通道。
    pub(crate) askpass: crate::askpass::AskPassClient,
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
        if self.state.running.get_untracked() {
            return;
        }
        let Some(step) = self
            .state
            .plan
            .get_untracked()
            .and_then(|plan| plan.steps.get(index).cloned())
        else {
            return;
        };
        let Some(tab_id) = self
            .session
            .tabs
            .get_untracked()
            .get(self.session.active.get_untracked())
            .map(|tab| tab.id)
        else {
            return;
        };
        self.state.running.set(true);
        interactions::notify(
            self.notice,
            i18n::t(self.language.get_untracked().as_str(), "agent.running"),
        );
        let commands = self.clone();
        spawn(async move {
            commands.run_agent_step_inner(tab_id, step).await;
            commands.state.running.set(false);
        });
    }

    async fn run_agent_step_inner(&self, tab_id: u64, step: agent::Step) {
        let tabs = self.session.tabs;
        let agent_policies = self.state.policies;
        let agent_settings = self.state.settings;
        let commit_summary = self.commit_summary;
        let now = store::now_unix();
        // 受保护分支：写动作一律不执行，记审计并说明原因。
        let branch_now = {
            let list = tabs.get_untracked();
            list.iter()
                .find(|tab| tab.id == tab_id)
                .map(|tab| tab.branch.clone())
                .unwrap_or_default()
        };
        if let Some(reason) = agent_settings
            .get_untracked()
            .blocks(step.action, branch_now.as_str())
        {
            self.record_step(now, step.action, false, reason);
            return;
        }
        // 权限：Deny 直接记审计并跳过；Ask 在界面上就是用户按了这一行。
        if agent_policies.get_untracked().is_denied(step.action) {
            self.record_step(now, step.action, false, "denied by policy".to_string());
            return;
        }

        if let Some(kind) = sync_kind(step.action) {
            let Some(tab) = tabs
                .get_untracked()
                .into_iter()
                .find(|tab| tab.id == tab_id)
            else {
                return;
            };
            let (ok, detail) = self.run_sync_step(tab, kind).await;
            self.record_step(now, step.action, ok, detail);
            return;
        }

        let mut list = tabs.get_untracked();
        let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) else {
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
            agent::Action::Fetch | agent::Action::Pull | agent::Action::Push => {
                unreachable!("network actions are handled asynchronously")
            }
            agent::Action::Discard => "skipped: destructive".to_string(),
        };
        let ok = match step.action {
            agent::Action::Commit => tab.hook_ok,
            agent::Action::Fetch | agent::Action::Pull | agent::Action::Push => !tab.sync_failed,
            agent::Action::Discard => false,
            _ => true,
        };
        tabs.set(list);
        self.record_step(now, step.action, ok, detail);
    }

    async fn run_sync_step(&self, mut tab: crate::app::session::Tab, kind: Sync) -> (bool, String) {
        let tab_id = tab.id;
        let path = tab.path.clone();
        let executable = tab.git_executable.clone();
        let askpass = self.askpass.clone();
        let language = self.language.get_untracked().to_string();
        let cancelled_text = i18n::t(
            language.as_str(),
            match kind {
                Sync::Fetch => "sync.cancelledFetch",
                Sync::Pull => "sync.cancelledPull",
                Sync::Push => "sync.cancelledPush",
            },
        );
        let (sender, receiver) = mpsc::channel();

        std::thread::spawn(move || {
            match crate::askpass::AskPassSession::new(executable.clone(), askpass) {
                Ok(session) => {
                    tab.git_executable = session.executable().clone();
                    sync(&mut tab, kind);
                    let cancelled = session.was_cancelled();
                    tab.git_executable = executable;
                    if cancelled {
                        crate::features::sync::ops::finish_sync_cancelled(&mut tab, cancelled_text);
                    }
                    let _ = sender.send(AgentSyncResult { tab, cancelled });
                }
                Err(error) => {
                    tab.sync_failed = true;
                    tab.sync_fault = None;
                    tab.sync_message = Some(
                        domain::GitError::Spawn {
                            program: "AskPass".to_string(),
                            reason: error.to_string(),
                        }
                        .detail(),
                    );
                    let _ = sender.send(AgentSyncResult {
                        tab,
                        cancelled: false,
                    });
                }
            }
        });

        let result = loop {
            match receiver.try_recv() {
                Ok(result) => break result,
                Err(mpsc::TryRecvError::Disconnected) => {
                    return (false, "Git sync worker stopped unexpectedly".to_string());
                }
                Err(mpsc::TryRecvError::Empty) => timer(Duration::from_millis(50)).await,
            }
        };

        let mut tabs = self.session.tabs.get_untracked();
        if let Some(current) = tabs
            .iter_mut()
            .find(|current| current.id == tab_id && current.path == path)
        {
            apply_sync_updates(current, &result.tab);
            self.session.tabs.set(tabs);
        }
        let detail = result.tab.sync_message.clone().unwrap_or_default();
        let ok = !result.cancelled && !result.tab.sync_failed;
        return (ok, detail);
    }

    fn record_step(&self, now: i64, action: agent::Action, ok: bool, detail: String) {
        let mut audit = self.state.audit.get_untracked();
        audit.insert(0, agent::AuditEntry::new(now, action, ok, detail));
        audit.truncate(agent::Audit::LIMIT);
        self.state.audit.set(audit);
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
        if self.state.running.get_untracked() {
            return;
        }
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
        let Some(target_tab_id) = tabs
            .get_untracked()
            .get(active.get_untracked())
            .map(|tab| tab.id)
        else {
            return;
        };
        let lang = language.get_untracked();
        if runnable.is_empty() {
            interactions::notify(notice, i18n::t(lang.as_str(), "agent.nothingRunnable"));
            return;
        }
        let runnable_steps: Vec<agent::Step> = runnable
            .iter()
            .filter_map(|index| plan.steps.get(*index))
            .cloned()
            .collect();
        let listed = runnable_steps
            .iter()
            .map(|step| format!("• {}", step.label))
            .collect::<Vec<_>>();
        let message = format!(
            "{}\n\n{}",
            i18n::t(lang.as_str(), "agent.approveMessage"),
            listed.join("\n")
        );
        let action_label = i18n::t(lang.as_str(), "agent.approveRun");
        let title = i18n::t(lang.as_str(), "agent.approveTitle");
        // 确认框返回后要逐条执行，先把用例克隆一份带进异步块。
        let run_step = self.clone();
        self.state.running.set(true);
        spawn(async move {
            let ok = interactions::confirm(
                title,
                message,
                "runPlan",
                action_label,
                i18n::t(lang.as_str(), "common.cancel"),
                false,
            )
            .await;
            if !ok {
                run_step.state.running.set(false);
                return;
            }
            interactions::notify(run_step.notice, i18n::t(lang.as_str(), "agent.running"));
            for step in runnable_steps {
                run_step.run_agent_step_inner(target_tab_id, step).await;
            }
            run_step.state.running.set(false);
        });
    }

    /// 设置某个动作的权限（原 `set_agent_permission`）：改完立刻落盘。
    pub(crate) fn set_agent_permission(
        &self,
        action: agent::Action,
        permission: agent::Permission,
    ) {
        let agent_policies = self.state.policies;
        let mut policies = agent_policies.get_untracked();
        policies.set(action, permission);
        agent_policies.set(policies);
        store::update_agent_preferences(&self.store_dir, |preferences| {
            preferences
                .permissions
                .insert(action.id().to_string(), permission.id().to_string());
        });
    }

    /// 设置自动化程度（原 `set_automation`）：改动即持久化。
    pub(crate) fn set_automation(&self, mode: agent::Automation) {
        let agent_settings = self.state.settings;
        let mut next = agent_settings.get_untracked();
        next.automation = mode;
        agent_settings.set(next);
        store::update_agent_preferences(&self.store_dir, |preferences| {
            preferences.automation = mode.id().to_string();
        });
    }

    /// 设置自动执行前的预览时长（原 `set_review_millis`）。
    pub(crate) fn set_review_millis(&self, millis: u32) {
        let agent_settings = self.state.settings;
        let mut next = agent_settings.get_untracked();
        next.review_millis = millis;
        agent_settings.set(next);
        store::update_agent_preferences(&self.store_dir, |preferences| {
            preferences.review_millis = millis;
        });
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
        let protected = next.protected.clone();
        agent_settings.set(next);
        store::update_agent_preferences(&self.store_dir, |preferences| {
            preferences.protected_branches = protected;
        });
    }
}

fn sync_kind(action: agent::Action) -> Option<Sync> {
    return match action {
        agent::Action::Fetch => Some(Sync::Fetch),
        agent::Action::Pull => Some(Sync::Pull),
        agent::Action::Push => Some(Sync::Push),
        _ => None,
    };
}

fn apply_sync_updates(target: &mut crate::app::session::Tab, updated: &crate::app::session::Tab) {
    target.sync_message = updated.sync_message.clone();
    target.sync_failed = updated.sync_failed;
    target.sync_fault = updated.sync_fault;
    target.status = updated.status.clone();
    target.changes = updated.changes.clone();
    target.branch = updated.branch.clone();
    target.ahead = updated.ahead;
    target.behind = updated.behind;
    target.commits = updated.commits.clone();
    target.refs = updated.refs.clone();
    target.remote_urls = updated.remote_urls.clone();
    target.changelists = updated.changelists.clone();
    target.head = updated.head.clone();
    target.operation = updated.operation;
    target.conflicts = updated.conflicts.clone();
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
