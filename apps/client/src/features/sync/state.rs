//! 同步功能的 Signal 集合：同步面板的开关与它在面板内编辑的内容。
//!
//! 只放属于本功能、且不跨功能的信号；同步结果（`sync_message` / `sync_failed` /
//! `sync_fault`）是每个仓库各自的数据，挂在 `Tab` 上，仍留在 `Tab` 里。

use crate::model::{Sync, SyncOptions};
use nsh_core::prelude::*;

/// 对话框状态：种类 + 选项 + 结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SyncDialogState {
    pub kind: Sync,
    pub options: SyncOptions,
    /// 执行结果；`None` 表示还没执行。
    pub result: Option<Result<String, String>>,
    /// 正在执行。
    pub running: bool,
}

impl SyncDialogState {
    pub fn new(kind: Sync) -> SyncDialogState {
        return SyncDialogState {
            kind,
            options: SyncOptions::default(),
            result: None,
            running: false,
        };
    }
}

#[derive(Clone)]
pub(crate) struct SyncState {
    /// 打开的同步面板；`None` 表示没打开。
    pub(crate) dialog: Signal<Option<SyncDialogState>>,
}
