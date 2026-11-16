//! 工作区内部视图：侧栏、diff 与提交面板。
//!
//! 提交历史（历史表 + 检查器）已迁到 `features/history`，diff 面板与显示开关已迁到
//! `shared/diff`。

pub(crate) mod agent;
pub(crate) mod commit;
pub(crate) mod sidebar;
