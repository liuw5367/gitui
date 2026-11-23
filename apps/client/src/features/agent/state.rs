//! Agent 功能的 Signal 集合：计划、权限策略、运行设置与审计记录。
//!
//! 这些都是本功能自己的状态，不挂在 `Tab` 上；设置页的 Agent 分区读同一份状态，
//! 所以放在这里而不是 `features/settings`。默认 **Ask first**：写动作必须逐条
//! 确认；`Deny` 的动作直接跳过并记入审计（见 `docs/AGENT-THREAT-MODEL.md`）。

use nsh_core::prelude::*;

#[derive(Clone)]
pub(crate) struct AgentState {
    /// 当前提议的计划；`None` 表示还没提议（面板显示「Plan」入口）。
    pub(crate) plan: Signal<Option<agent::Plan>>,
    /// 单步或整份计划正在执行时，阻止重复启动。
    pub(crate) running: Signal<bool>,
    /// 逐动作的 Ask / Allow / Deny。
    pub(crate) policies: Signal<agent::Policies>,
    /// 自动化程度、自动执行前的预览时长与受保护分支。
    pub(crate) settings: Signal<agent::Settings>,
    /// 最近的执行记录：面板显示前 8 条，落盘保留 `agent::Audit::LIMIT` 条。
    pub(crate) audit: Signal<Vec<agent::AuditEntry>>,
}
