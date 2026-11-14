//! Agent 的核心：计划模型、权限策略与审计日志。
//!
//! v0.2 的 Agent 只在**本机**工作：它把当前仓库状态整理成一份可审阅的计划
//! （`Plan`），每一步是受权限约束的动作（`Action`）。应用负责真正执行——本 crate
//! 不碰 `std::process`，也不读凭据（见 `docs/AGENT-THREAT-MODEL.md`）。
//!
//! 默认策略是 **Ask first**：每个动作都要用户确认；`Permission::Deny` 的动作永远
//! 不执行，`Permission::Allow` 只用于只读动作。

use std::collections::BTreeMap;

/// Agent 可以请求的动作。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action {
    /// 暂存全部改动（`git add -A`）。
    StageAll,
    /// 用建议的说明提交（`git commit`）。
    Commit,
    /// 取消暂存全部（`git restore --staged .`）。
    UnstageAll,
    /// 取回远程引用（`git fetch`）。
    Fetch,
    /// 拉取（`git pull --ff-only`）。
    Pull,
    /// 推送（`git push`）。
    Push,
    /// 丢弃工作区改动（`git restore --worktree`）。
    Discard,
}

impl Action {
    /// 稳定标识，用于存储与审计。
    pub fn id(self) -> &'static str {
        return match self {
            Action::StageAll => "stage-all",
            Action::Commit => "commit",
            Action::UnstageAll => "unstage-all",
            Action::Fetch => "fetch",
            Action::Pull => "pull",
            Action::Push => "push",
            Action::Discard => "discard",
        };
    }

    /// 默认权限：只读动作可以自动执行，写动作必须询问。
    pub fn default_permission(self) -> Permission {
        return match self {
            Action::Fetch => Permission::Allow,
            _ => Permission::Ask,
        };
    }

    /// 是否会改变仓库（用于在 UI 上区分危险动作）。
    pub fn is_destructive(self) -> bool {
        return matches!(self, Action::Discard | Action::Push | Action::Commit);
    }
}

/// 权限策略。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Permission {
    /// 每次都询问（默认）。
    #[default]
    Ask,
    /// 直接允许。
    Allow,
    /// 永远不执行。
    Deny,
}

impl Permission {
    pub fn id(self) -> &'static str {
        return match self {
            Permission::Ask => "ask",
            Permission::Allow => "allow",
            Permission::Deny => "deny",
        };
    }

    pub fn from_id(id: &str) -> Permission {
        return match id {
            "allow" => Permission::Allow,
            "deny" => Permission::Deny,
            _ => Permission::Ask,
        };
    }
}

/// 计划中的一步。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub action: Action,
    /// 面向用户的说明（已完成本地化）。
    pub label: String,
    /// 附加参数，例如提交说明。
    pub argument: Option<String>,
}

impl Step {
    pub fn new(action: Action, label: impl Into<String>) -> Step {
        return Step {
            action,
            label: label.into(),
            argument: None,
        };
    }

    pub fn with_argument(mut self, argument: impl Into<String>) -> Step {
        self.argument = Some(argument.into());
        return self;
    }
}

/// 一份计划：有序的步骤。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub summary: String,
    pub steps: Vec<Step>,
}

impl Plan {
    pub fn new(summary: impl Into<String>, steps: Vec<Step>) -> Plan {
        return Plan {
            summary: summary.into(),
            steps,
        };
    }

    /// 计划里是否有会改动仓库的步骤。
    pub fn touches_worktree(&self) -> bool {
        return self.steps.iter().any(|step| match step.action {
            Action::Fetch => false,
            _ => true,
        });
    }
}

/// 权限表：按动作保存，未出现的动作用默认值。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Policies {
    entries: BTreeMap<Action, Permission>,
}

impl Policies {
    /// 读取某个动作的权限。
    pub fn permission(&self, action: Action) -> Permission {
        return self
            .entries
            .get(&action)
            .copied()
            .unwrap_or_else(|| action.default_permission());
    }

    /// 设置某个动作的权限。
    pub fn set(&mut self, action: Action, permission: Permission) {
        self.entries.insert(action, permission);
    }

    /// 该动作是否可以直接执行（无需询问）。
    pub fn may_run(&self, action: Action) -> bool {
        return self.permission(action) == Permission::Allow;
    }

    /// 该动作是否被禁止。
    pub fn is_denied(&self, action: Action) -> bool {
        return self.permission(action) == Permission::Deny;
    }
}

/// 审计记录里的一条。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditEntry {
    /// UTC 秒。
    pub timestamp: i64,
    /// 动作标识。
    pub action: String,
    /// 是否执行成功。
    pub ok: bool,
    /// 结果或拒绝原因的简述。
    pub detail: String,
}

impl AuditEntry {
    pub fn new(timestamp: i64, action: Action, ok: bool, detail: impl Into<String>) -> AuditEntry {
        return AuditEntry {
            timestamp,
            action: action.id().to_string(),
            ok,
            detail: detail.into(),
        };
    }
}

/// 审计日志：最新的在前，最多保留 `LIMIT` 条。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Audit {
    entries: Vec<AuditEntry>,
}

impl Audit {
    /// 保留的记录条数上限。
    pub const LIMIT: usize = 200;

    pub fn record(&mut self, entry: AuditEntry) {
        self.entries.insert(0, entry);
        self.entries.truncate(Self::LIMIT);
    }

    pub fn entries(&self) -> &[AuditEntry] {
        return &self.entries;
    }

    pub fn is_empty(&self) -> bool {
        return self.entries.is_empty();
    }
}

/// 根据仓库状态提出一份计划。
///
/// 规则是确定的，便于测试与解释：
/// - 有未暂存改动 → 先暂存全部；
/// - 有已暂存改动 → 用给定说明提交；
/// - 落后远程 → 拉取；领先远程 → 推送。
pub fn propose(
    summary: &str,
    staged: usize,
    unstaged: usize,
    ahead: u32,
    behind: u32,
    commit_message: &str,
) -> Plan {
    let mut steps: Vec<Step> = Vec::new();
    if unstaged > 0 {
        steps.push(Step::new(Action::StageAll, "暂存全部改动"));
    }
    if staged > 0 || (unstaged > 0 && !commit_message.trim().is_empty()) {
        let label = format!("提交：{}", commit_message.trim());
        steps.push(Step::new(Action::Commit, label).with_argument(commit_message.trim().to_string()));
    }
    if behind > 0 {
        steps.push(Step::new(Action::Pull, "拉取远程改动"));
    }
    if ahead > 0 {
        steps.push(Step::new(Action::Push, "推送到远程"));
    }
    let summary = if steps.is_empty() {
        "仓库是干净的，没有需要执行的动作".to_string()
    } else {
        summary.to_string()
    };
    return Plan::new(summary, steps);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_actions_ask_by_default_and_fetch_is_allowed() {
        let policies = Policies::default();
        assert_eq!(policies.permission(Action::Fetch), Permission::Allow);
        assert_eq!(policies.permission(Action::Commit), Permission::Ask);
        assert!(policies.may_run(Action::Fetch));
        assert!(!policies.may_run(Action::Commit));
    }

    #[test]
    fn policies_can_deny_and_allow_per_action() {
        let mut policies = Policies::default();
        policies.set(Action::Push, Permission::Deny);
        policies.set(Action::Commit, Permission::Allow);
        assert!(policies.is_denied(Action::Push));
        assert!(policies.may_run(Action::Commit));
        assert!(!policies.may_run(Action::StageAll));
        assert_eq!(
            Permission::from_id(Permission::Deny.id()),
            Permission::Deny
        );
    }

    #[test]
    fn plan_orders_stage_commit_pull_push() {
        let plan = propose("整理提交", 1, 2, 1, 1, "fix composer");
        let actions: Vec<Action> = plan.steps.iter().map(|step| step.action).collect();
        assert_eq!(
            actions,
            vec![
                Action::StageAll,
                Action::Commit,
                Action::Pull,
                Action::Push
            ]
        );
        assert_eq!(plan.steps[1].argument.as_deref(), Some("fix composer"));
        assert!(plan.touches_worktree());
    }

    #[test]
    fn clean_repository_proposes_nothing() {
        let plan = propose("", 0, 0, 0, 0, "");
        assert!(plan.steps.is_empty());
        assert!(plan.summary.contains("干净"));
        assert!(!plan.touches_worktree());
    }

    #[test]
    fn fetch_only_plan_does_not_touch_the_worktree() {
        let plan = propose("取回", 0, 0, 0, 2, "");
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].action, Action::Pull);
        assert!(plan.touches_worktree());
    }

    #[test]
    fn audit_keeps_the_latest_first_and_caps_entries() {
        let mut audit = Audit::default();
        assert!(audit.is_empty());
        audit.record(AuditEntry::new(1, Action::Fetch, true, "ok"));
        audit.record(AuditEntry::new(2, Action::Push, false, "rejected"));
        assert_eq!(audit.entries()[0].action, "push");
        assert!(!audit.entries()[0].ok);

        for index in 0..Audit::LIMIT + 10 {
            audit.record(AuditEntry::new(index as i64, Action::Fetch, true, "ok"));
        }
        assert_eq!(audit.entries().len(), Audit::LIMIT);
    }

    #[test]
    fn destructive_actions_are_flagged() {
        assert!(Action::Discard.is_destructive());
        assert!(Action::Push.is_destructive());
        assert!(!Action::Fetch.is_destructive());
        assert!(!Action::StageAll.is_destructive());
    }
}
