//! 同步功能的 Signal 集合：同步面板的开关与它在面板内编辑的内容。
//!
//! 只放属于本功能、且不跨功能的信号；同步结果（`sync_message` / `sync_failed` /
//! `sync_fault`）是每个仓库各自的数据，挂在 `Tab` 上，仍留在 `Tab` 里。

use crate::app::session::{Sync, SyncOptions, WorkflowProgress};
use nsh_core::prelude::*;

/// 远端操作完成后供面板本地化呈现的事实摘要。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SyncSummary {
    Fetch {
        target: String,
        updated_refs: usize,
        deleted_refs: usize,
        commits: Option<usize>,
    },
    Pull {
        target: String,
        commits: Option<usize>,
    },
    Push {
        target: String,
        updated_refs: usize,
        deleted_refs: usize,
        commits: Option<usize>,
        rejected_refs: usize,
    },
    Unknown {
        kind: Sync,
        target: String,
    },
}

/// 对话框状态：种类 + 选项 + 结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SyncDialogState {
    pub kind: Sync,
    pub target_tab_id: Option<u64>,
    /// 仓库路径快照；tab 关闭后仍能标识后台任务的目标。
    pub target_path: Option<String>,
    pub remote: Option<String>,
    pub options: SyncOptions,
    /// 执行结果；`None` 表示还没执行。
    pub result: Option<Result<String, String>>,
    /// Fetch/Pull/Push 完成后可确认的操作摘要。
    pub summary: Option<SyncSummary>,
    /// 正在执行。
    pub running: bool,
}

impl SyncDialogState {
    pub fn new(kind: Sync, target_tab_id: Option<u64>, remote: Option<String>) -> SyncDialogState {
        return SyncDialogState {
            kind,
            target_tab_id,
            target_path: None,
            remote,
            options: SyncOptions::default(),
            result: None,
            summary: None,
            running: false,
        };
    }
}

#[derive(Clone)]
pub(crate) struct SyncTaskState {
    pub kind: Sync,
    pub target_tab_id: u64,
    pub path: String,
    pub remote: Option<String>,
    pub options: SyncOptions,
    pub progress: String,
    pub result: Option<Result<String, String>>,
    pub summary: Option<SyncSummary>,
    pub cancelled: bool,
    /// 是否由同步面板启动；提交后自动推送走提交反馈块，不弹同步面板。
    pub show_dialog: bool,
    /// 串行同步操作链进度；普通单步 Fetch/Pull/Push 为空。
    pub workflow_progress: Option<WorkflowProgress>,
    pub follow_up_push: Option<SyncOptions>,
    pub retry_push: bool,
    pub cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl PartialEq for SyncTaskState {
    fn eq(&self, other: &Self) -> bool {
        return self.kind == other.kind
            && self.target_tab_id == other.target_tab_id
            && self.path == other.path
            && self.remote == other.remote
            && self.options == other.options
            && self.progress == other.progress
            && self.result == other.result
            && self.summary == other.summary
            && self.cancelled == other.cancelled
            && self.show_dialog == other.show_dialog
            && self.workflow_progress == other.workflow_progress
            && self.follow_up_push == other.follow_up_push
            && self.retry_push == other.retry_push;
    }
}

impl Eq for SyncTaskState {}

#[derive(Clone)]
pub(crate) struct SyncState {
    /// 打开的同步面板；`None` 表示没打开。
    pub(crate) dialog: Signal<Option<SyncDialogState>>,
    /// 最近一次同步任务，关闭面板后继续保存进度与结果。
    pub(crate) task: Signal<Option<SyncTaskState>>,
}
