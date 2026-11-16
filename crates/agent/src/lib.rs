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
    /// 读取仓库文件内容（只读）。
    ReadFiles,
    /// 强制推送（`--force-with-lease`）。
    ForcePush,
}

impl Action {
    /// 全部动作，顺序即界面显示顺序（策略持久化与设置页共用）。
    pub const ALL: [Action; 9] = [
        Action::StageAll,
        Action::Commit,
        Action::UnstageAll,
        Action::Fetch,
        Action::Pull,
        Action::Push,
        Action::Discard,
        Action::ReadFiles,
        Action::ForcePush,
    ];

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
            Action::ReadFiles => "read-files",
            Action::ForcePush => "force-push",
        };
    }

    /// 默认权限：只读动作可以自动执行，写动作必须询问。
    pub fn default_permission(self) -> Permission {
        return match self {
            Action::Fetch | Action::ReadFiles => Permission::Allow,
            _ => Permission::Ask,
        };
    }

    /// 是否为只读动作（读文件、取回）。
    pub fn is_read_only(self) -> bool {
        return matches!(self, Action::ReadFiles | Action::Fetch);
    }

    /// 是否会改变仓库（用于在 UI 上区分危险动作）。
    pub fn is_destructive(self) -> bool {
        return matches!(
            self,
            Action::Discard | Action::Push | Action::Commit | Action::ForcePush
        );
    }
}

/// 自动化程度（设计板 13 的 Automation）：Agent 能自己走多远。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Automation {
    /// 每步都等确认。
    #[default]
    Off,
    /// 可以自动提交，推送仍需确认。
    Commit,
    /// 提交并推送。
    CommitAndPush,
}

impl Automation {
    pub fn id(self) -> &'static str {
        return match self {
            Automation::Off => "off",
            Automation::Commit => "commit",
            Automation::CommitAndPush => "commit-and-push",
        };
    }

    pub fn from_id(id: &str) -> Automation {
        return match id {
            "commit" => Automation::Commit,
            "commit-and-push" => Automation::CommitAndPush,
            _ => Automation::Off,
        };
    }

    /// 该自动化程度是否允许直接推送。只有「提交并推送」允许。
    pub fn may_push(self) -> bool {
        return self == Automation::CommitAndPush;
    }
}

/// Agent 的运行设置：自动化程度、执行前预览时长、受保护分支。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub automation: Automation,
    /// 自动执行前的预览毫秒数（0 表示不预览）。
    pub review_millis: u32,
    /// 受保护分支：Agent 不直接在这些分支上提交或推送。
    pub protected: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        return Settings {
            automation: Automation::Off,
            review_millis: 8000,
            protected: vec!["main".to_string()],
        };
    }
}

impl Settings {
    /// 预览时长的可选项（设计板 13）。
    pub const REVIEW_CHOICES: [u32; 4] = [0, 3000, 5000, 8000];

    /// 分支是否受保护；空列表表示不保护任何分支。
    pub fn is_protected(&self, branch: &str) -> bool {
        return self.protected.iter().any(|name| name == branch);
    }

    /// 这个动作在当前设置下是否被挡住（受保护分支 + 写动作）。
    ///
    /// 返回 `Some(原因)` 表示拒绝执行，由界面提示而不是静默跳过。
    pub fn blocks(&self, action: Action, branch: &str) -> Option<String> {
        if action.is_read_only() || !self.is_protected(branch) {
            return None;
        }
        return Some(format!("{branch} 是受保护分支，Agent 不会在上面执行 {}", action.id()));
    }

    /// 从 `key=value` 行读取（与策略共用同一个文件）。
    pub fn read_pair(&mut self, key: &str, value: &str) -> bool {
        return match key {
            "automation" => {
                self.automation = Automation::from_id(value);
                true
            }
            "review-millis" => {
                self.review_millis = value.parse().unwrap_or(self.review_millis);
                true
            }
            "protected" => {
                self.protected = value
                    .split(',')
                    .map(|name| name.trim().to_string())
                    .filter(|name| !name.is_empty())
                    .collect();
                true
            }
            _ => false,
        };
    }

    /// 写回成 `key=value` 行。
    pub fn write_pairs(&self) -> Vec<(String, String)> {
        return vec![
            ("automation".to_string(), self.automation.id().to_string()),
            ("review-millis".to_string(), self.review_millis.to_string()),
            ("protected".to_string(), self.protected.join(",")),
        ];
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

    /// 追加一步（计划编辑：用户手动补一步）。
    pub fn push_step(&mut self, step: Step) {
        self.steps.push(step);
    }

    /// 删掉某一步；返回是否真的删了（下标越界时为 `false`）。
    ///
    /// 删除只影响这份建议，不改变仓库；执行过的步骤不会回滚。
    pub fn remove_step(&mut self, index: usize) -> bool {
        if index >= self.steps.len() {
            return false;
        }
        self.steps.remove(index);
        return true;
    }

    /// 按当前策略，哪些步骤可以执行（`Deny` 与受保护分支挡住的除外）。
    ///
    /// 返回下标，保持计划顺序；界面用它做「Approve & Run」的范围预览。
    pub fn runnable(&self, policies: &Policies, settings: &Settings, branch: &str) -> Vec<usize> {
        return self
            .steps
            .iter()
            .enumerate()
            .filter(|(_, step)| {
                !policies.is_denied(step.action)
                    && settings.blocks(step.action, branch).is_none()
            })
            .map(|(index, _)| index)
            .collect();
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
    fn automation_gates_push_and_round_trips_through_ids() {
        assert!(!Automation::Off.may_push());
        assert!(!Automation::Commit.may_push());
        assert!(Automation::CommitAndPush.may_push());
        for automation in [Automation::Off, Automation::Commit, Automation::CommitAndPush] {
            assert_eq!(Automation::from_id(automation.id()), automation);
        }
        assert_eq!(Automation::from_id("nonsense"), Automation::Off);
    }

    #[test]
    fn protected_branches_block_writes_but_not_reads() {
        let settings = Settings::default();
        assert!(settings.is_protected("main"), "main 默认受保护");
        assert!(!settings.is_protected("feature/x"));
        // 写动作被挡住，并带可读原因。
        let blocked = settings.blocks(Action::Commit, "main").expect("应被挡");
        assert!(blocked.contains("main"), "{blocked}");
        assert!(settings.blocks(Action::ForcePush, "main").is_some());
        // 只读动作不受保护影响。
        assert!(settings.blocks(Action::ReadFiles, "main").is_none());
        assert!(settings.blocks(Action::Fetch, "main").is_none());
        // 非保护分支不挡。
        assert!(settings.blocks(Action::Commit, "feature/x").is_none());
    }

    #[test]
    fn plan_steps_can_be_added_and_removed() {
        let mut plan = propose("s", 0, 2, 0, 0, "fix: thing");
        let before = plan.steps.len();
        plan.push_step(Step::new(Action::Fetch, "取回远程"));
        assert_eq!(plan.steps.len(), before + 1);
        assert_eq!(plan.steps.last().map(|step| step.action), Some(Action::Fetch));
        assert!(plan.remove_step(before));
        assert_eq!(plan.steps.len(), before);
        // 越界删除返回 false，不改计划。
        assert!(!plan.remove_step(99));
        assert_eq!(plan.steps.len(), before);
    }

    #[test]
    fn runnable_skips_denied_and_protected() {
        let mut plan = Plan::default();
        plan.push_step(Step::new(Action::Fetch, "取回"));
        plan.push_step(Step::new(Action::StageAll, "暂存"));
        plan.push_step(Step::new(Action::Discard, "丢弃"));
        plan.push_step(Step::new(Action::Commit, "提交"));
        let mut policies = Policies::default();
        policies.set(Action::Discard, Permission::Deny);
        let mut settings = Settings::default();
        settings.protected = vec!["main".to_string()];
        // 受保护分支挡住写动作，只读动作照常。
        assert_eq!(plan.runnable(&policies, &settings, "main"), vec![0]);
        // 非保护分支：除了被 Deny 的丢弃，其余都可执行。
        assert_eq!(plan.runnable(&policies, &settings, "feature/x"), vec![0, 1, 3]);
    }

    #[test]
    fn settings_pairs_round_trip() {
        let settings = Settings::default();
        let pairs = settings.write_pairs();
        assert_eq!(pairs.len(), 3);

        let mut parsed = Settings {
            automation: Automation::Off,
            review_millis: 0,
            protected: Vec::new(),
        };
        for (key, value) in &pairs {
            assert!(parsed.read_pair(key, value), "应认识 {key}");
        }
        assert_eq!(parsed.automation, settings.automation);
        assert_eq!(parsed.review_millis, settings.review_millis);
        assert_eq!(parsed.protected, settings.protected);
        assert!(!parsed.read_pair("unknown-key", "1"), "未知键应返回 false");
    }

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
