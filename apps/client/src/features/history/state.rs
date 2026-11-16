//! 提交历史功能的 Signal 集合：关键字、分支/作者/时间过滤、对比模式与筛选浮层。
//!
//! 只放属于本功能、且不跨功能的信号；选中提交、选中文件、补丁这些挂在 `Tab` 上的
//! 状态留在 `Tab` 里。

use git::LogFilter;
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct HistoryState {
    /// 工具栏搜索框里的关键字。
    pub(crate) query: Signal<String>,
    /// 只在该分支/标签上读历史；空串表示当前 HEAD。
    pub(crate) reference: Signal<String>,
    /// 作者过滤；空串表示不限。
    pub(crate) author: Signal<String>,
    /// 时间预设（`When` 的 id）。
    pub(crate) when: Signal<SharedString>,
    /// 是否展开筛选浮层（设计板 05）。
    pub(crate) filter_open: Signal<bool>,
    /// 对比模式（设计板 06）。
    pub(crate) compare_mode: Signal<bool>,
    /// 当前生效的筛选条件（原 `initial_filter` / `current_filter` 用的那份）：
    /// 读上面四个信号现算，不在别处缓存；`open_path` 也要用它，所以随状态一起持有。
    pub(crate) filter: Arc<dyn Fn() -> LogFilter>,
}
