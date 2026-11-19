//! 提交历史功能的 Signal 集合：关键字、分支/作者/时间过滤、对比模式与筛选浮层。
//!
//! 只放属于本功能、且不跨功能的信号；选中提交、选中文件、补丁这些挂在 `Tab` 上的
//! 状态留在 `Tab` 里。

use crate::filters::FilterDraft;
use git::LogFilter;
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FilterMenu {
    Branches,
    Authors,
}

#[derive(Clone)]
pub(crate) struct HistoryState {
    /// 工具栏搜索框里的关键字。
    pub(crate) query: Signal<String>,
    /// 筛选浮层中尚未应用的条件。
    pub(crate) draft: Signal<FilterDraft>,
    /// 是否展开筛选浮层（设计板 05）。
    pub(crate) filter_open: Signal<bool>,
    /// 筛选浮层在主窗口根视图中的锚点位置。
    pub(crate) filter_position: Signal<(f32, f32)>,
    /// 顶栏快捷筛选的锚定浮层。
    pub(crate) filter_menu: Signal<Option<FilterMenu>>,
    /// 提交文件树的展开路径，切换提交时继续显示同一目录层级。
    pub(crate) file_tree_expanded: Signal<Option<Vec<SharedString>>>,
    /// 对比模式（设计板 06）。
    pub(crate) compare_mode: Signal<bool>,
    /// 虚拟化历史表的引用，用于引用点击后的行级滚动。
    pub(crate) table_ref: ComponentRef<nsh_base::DataTable>,
    /// 下次历史表布局完成后要滚动到的可见行。
    pub(crate) scroll_request: Signal<Option<usize>>,
    /// 当前活动标签页已应用的条件；`open_path` 也通过它初始化新标签页。
    pub(crate) filter: Arc<dyn Fn() -> LogFilter>,
}
