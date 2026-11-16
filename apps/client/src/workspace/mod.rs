//! 工作区内部视图：Agent 面板。
//!
//! 提交历史（历史表 + 检查器）已迁到 `features/history`，变更/暂存已迁到
//! `features/changes`，引用管理（侧栏 + Refs 表 + Reflog）已迁到 `features/refs`，
//! 同步面板已迁到 `features/sync`，冲突解决与提交面板已迁到 `features/merge` /
//! `features/commit`；diff 面板与显示开关已迁到 `shared/diff`。

pub(crate) mod agent;
