//! 冲突解决功能的窗口级 Signal 集合：base 条的显隐。
//!
//! 只放窗口级信号；冲突 Result 草稿是每仓库、每路径的会话数据，唯一存放在 `Tab`。

use nsh_core::prelude::*;

#[derive(Clone)]
pub(crate) struct MergeState {
    /// Base 区域是否展开。
    pub(crate) base_open: Signal<bool>,
}
