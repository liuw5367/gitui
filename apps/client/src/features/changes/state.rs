//! 变更/暂存功能的 Signal 集合：变更清单浮层的开关。
//!
//! 只放属于本功能、且不跨功能的信号；变更列表、选中项、选中变更的补丁都在 `Tab`
//! 上（每个仓库各自的数据），仍留在 `Tab` 里。

use nsh_core::prelude::*;

#[derive(Clone)]
pub(crate) struct ChangesState {
    /// 变更清单浮层（设计板 11）是否打开。
    pub(crate) changelist_open: Signal<bool>,
}
