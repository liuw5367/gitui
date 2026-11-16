//! 冲突解决功能的 Signal 集合：可编辑的结果区与 base 条的显隐。
//!
//! 只放属于本功能、且不跨功能的信号；视图开关、冲突文件列表、选中下标与冲突
//! 文件文本都是每个仓库各自的数据，挂在 `Tab` 上，仍留在 `Tab` 里。

use nsh_core::prelude::*;

#[derive(Clone)]
pub(crate) struct MergeState {
    /// 结果区的可编辑文本：接受某一侧只是把它填进来，仍可继续手改。
    pub(crate) result: Signal<String>,
    /// base 条是否展开（只有 diff3 风格写出的冲突块才有内容）。
    pub(crate) base_open: Signal<bool>,
}
