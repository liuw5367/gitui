//! 提交功能的 Signal 集合：提交说明、描述与历史改写弹窗的状态。
//!
//! 只放属于本功能、且不跨功能的信号；提交选项（amend / signoff / pushAfter）、
//! 钩子输出与钩子弹层的打开状态是每个仓库各自的数据，挂在 `Tab` 上，仍留在
//! `Tab` 里。

use nsh_core::prelude::*;

/// 这次要做的改写。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RewriteKind {
    /// 改写单个提交的说明。
    Reword {
        sha: String,
        short: String,
        summary: String,
    },
    /// 把 `from..=to` 合并成一个提交。
    Squash {
        from: String,
        to: String,
        /// 将被合并的提交（旧到新）：(短 SHA, 说明)。
        commits: Vec<(String, String)>,
    },
}

impl RewriteKind {
    pub fn title_key(&self) -> &'static str {
        return match self {
            RewriteKind::Reword { .. } => "rewrite.rewordTitle",
            RewriteKind::Squash { .. } => "rewrite.squashTitle",
        };
    }

    pub fn action_key(&self) -> &'static str {
        return match self {
            RewriteKind::Reword { .. } => "rewrite.rewordAction",
            RewriteKind::Squash { .. } => "rewrite.squashAction",
        };
    }
}

/// 改写弹窗的状态。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RewriteState {
    pub kind: RewriteKind,
    pub message: String,
    /// 提交失败时的原因。
    pub error: Option<String>,
    /// 不能改写的阻塞原因（工作区不干净等）；`None` 表示可以执行。
    pub blocker: Option<String>,
}

impl RewriteState {
    pub fn new(kind: RewriteKind, message: String, blocker: Option<String>) -> RewriteState {
        return RewriteState {
            kind,
            message,
            error: None,
            blocker,
        };
    }
}

#[derive(Clone)]
pub(crate) struct CommitState {
    /// 提交说明（Summary 输入框）。
    pub(crate) summary: Signal<String>,
    /// 提交正文（Description 输入框）。
    pub(crate) description: Signal<String>,
    /// 打开的历史改写弹窗；`None` 表示没打开。
    pub(crate) rewrite: Signal<Option<RewriteState>>,
}
