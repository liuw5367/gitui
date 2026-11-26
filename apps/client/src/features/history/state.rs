//! 提交历史功能的 Signal 集合：关键字、分支/作者/时间过滤、对比模式与筛选浮层。
//!
//! 只放属于本功能、且不跨功能的信号；选中提交、选中文件、补丁这些挂在 `Tab` 上的
//! 状态留在 `Tab` 里。

use crate::filters::FilterDraft;
use domain::DiffStat;
use git::{GitExecutable, LogFilter};
use nsh_core::prelude::*;
use std::sync::Arc;

pub(crate) const DEFAULT_AUTHOR_COLUMN_WIDTH: f32 = 80.0;
pub(crate) const DEFAULT_DATE_COLUMN_WIDTH: f32 = 112.0;

pub(crate) fn resized_column_width(start: f32, offset: f32, min: f32, max: f32) -> f32 {
    return (start + offset).clamp(min, max);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FilterMenu {
    Branches,
    Authors,
}

#[derive(Clone, Default, PartialEq, Eq)]
pub(crate) struct CompareRefsDraft {
    pub(crate) base: Option<String>,
    pub(crate) target: Option<String>,
    pub(crate) three_dot: bool,
    pub(crate) worktree: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MultiCommitSelection {
    pub(crate) tab_id: u64,
    pub(crate) shas: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MultiCommitData {
    pub(crate) tab_id: u64,
    pub(crate) shas: Vec<String>,
    pub(crate) files: Vec<DiffStat>,
    /// 与 `shas` 顺序对应；单个提交详情读取失败时该项为 `None`。
    pub(crate) details: Vec<Option<git::CommitDetails>>,
    pub(crate) loading: bool,
    pub(crate) failed: bool,
}

#[derive(Clone)]
pub(crate) struct CommitLoadRequest {
    pub(crate) tab_id: u64,
    pub(crate) path: String,
    pub(crate) git_executable: GitExecutable,
    pub(crate) sha: String,
    pub(crate) short: String,
}

#[derive(Default)]
pub(crate) struct CommitLoadQueue {
    running: bool,
    pending: std::collections::VecDeque<CommitLoadRequest>,
}

impl CommitLoadQueue {
    pub(crate) fn enqueue(&mut self, request: CommitLoadRequest) -> Option<CommitLoadRequest> {
        if self.running {
            if let Some(pending) = self
                .pending
                .iter_mut()
                .find(|pending| pending.tab_id == request.tab_id)
            {
                *pending = request;
            } else {
                self.pending.push_back(request);
            }
            return None;
        }
        self.running = true;
        return Some(request);
    }

    pub(crate) fn finish(&mut self) -> Option<CommitLoadRequest> {
        if let Some(request) = self.pending.pop_front() {
            return Some(request);
        }
        self.running = false;
        return None;
    }
}

#[derive(Clone)]
pub(crate) struct HistoryState {
    /// 工具栏搜索框里的关键字。
    pub(crate) query: Signal<String>,
    /// 筛选浮层中尚未应用的条件。
    pub(crate) draft: Signal<FilterDraft>,
    /// 是否展开筛选浮层（设计板 05）。
    pub(crate) filter_open: Signal<bool>,
    /// `Some` 表示按筛选按钮定位；`None` 表示快捷键打开时居中显示。
    pub(crate) filter_position: Signal<Option<(f32, f32)>>,
    /// 顶栏快捷筛选的锚定浮层。
    pub(crate) filter_menu: Signal<Option<FilterMenu>>,
    /// 筛选草稿所属的仓库标签页，避免切仓后把草稿应用到别的仓库。
    pub(crate) filter_tab_id: Signal<Option<u64>>,
    /// 快捷筛选浮层打开时的草稿快照，用于判断用户是否实际改过选择。
    pub(crate) quick_filter_baseline: Signal<Option<(Vec<String>, Vec<String>)>>,
    /// 提交文件树的展开路径，切换提交时继续显示同一目录层级。
    pub(crate) file_tree_expanded: Signal<Option<Vec<SharedString>>>,
    /// 对比模式（设计板 06）。
    pub(crate) compare_mode: Signal<bool>,
    /// Compare refs 窗口与搜索框状态。
    pub(crate) compare_refs_open: Signal<bool>,
    pub(crate) compare_refs_query: Signal<String>,
    /// Compare refs 窗口中尚未应用的选项。
    pub(crate) compare_refs_draft: Signal<CompareRefsDraft>,
    /// 提交行先用此信号即时反馈选中，再异步更新仓库标签页数据。
    pub(crate) fast_selection: Signal<Option<(u64, String)>>,
    /// ⌘ 点击选中的提交，按仓库标签页隔离。
    pub(crate) multi_commit_selection: Signal<Option<MultiCommitSelection>>,
    /// 多选提交的合并文件统计与逐项详情，异步结果需与 `multi_commit_selection` 匹配。
    pub(crate) multi_commit_data: Signal<Option<MultiCommitData>>,
    /// 当前正在后台读取信息的提交（标签页 ID 与完整 SHA）。
    pub(crate) commit_info_loading: Signal<Vec<(u64, String)>>,
    /// 串行读取提交详情；每个仓库标签页最多保留一个最新等待项。
    pub(crate) commit_load_queue: std::sync::Arc<std::sync::Mutex<CommitLoadQueue>>,
    /// 虚拟化历史表的引用，用于引用点击后的行级滚动。
    pub(crate) table_ref: ComponentRef<nsh_base::DataTable>,
    /// 下次历史表布局完成后要滚动到的可见行。
    pub(crate) scroll_request: Signal<Option<usize>>,
    /// 当前活动标签页已应用的条件；`open_path` 也通过它初始化新标签页。
    pub(crate) filter: Arc<dyn Fn() -> LogFilter>,
    /// History 提交表格中作者列与日期列的可拖拽宽度。
    pub(crate) author_column_width: Signal<f32>,
    pub(crate) date_column_width: Signal<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(tab_id: u64, sha: &str) -> CommitLoadRequest {
        CommitLoadRequest {
            tab_id,
            path: String::from("/repo"),
            git_executable: GitExecutable::system(),
            sha: sha.to_string(),
            short: sha.to_string(),
        }
    }

    #[test]
    fn commit_load_queue_keeps_the_latest_pending_selection_per_tab() {
        let mut queue = CommitLoadQueue::default();
        assert_eq!(queue.enqueue(request(1, "active")).unwrap().sha, "active");
        assert!(queue.enqueue(request(2, "second")).is_none());
        assert!(queue.enqueue(request(3, "third")).is_none());
        assert!(queue.enqueue(request(2, "latest-second")).is_none());
        assert_eq!(queue.finish().unwrap().sha, "latest-second");
        assert_eq!(queue.finish().unwrap().sha, "third");
        assert!(queue.finish().is_none());
        assert_eq!(
            queue.enqueue(request(1, "after-finish")).unwrap().sha,
            "after-finish"
        );
    }

    #[test]
    fn resized_column_width_stays_within_column_limits() {
        assert_eq!(resized_column_width(152.0, -100.0, 80.0, 320.0), 80.0);
        assert_eq!(resized_column_width(152.0, 300.0, 80.0, 320.0), 320.0);
        assert_eq!(resized_column_width(152.0, 24.0, 80.0, 320.0), 176.0);
    }
}
