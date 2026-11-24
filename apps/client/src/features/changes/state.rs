//! 变更/暂存功能的 Signal 集合：变更清单浮层的开关。
//!
//! 只放属于本功能、且不跨功能的信号；变更列表、选中项、选中变更的补丁都在 `Tab`
//! 上（每个仓库各自的数据），仍留在 `Tab` 里。

use std::collections::{BTreeMap, BTreeSet};

use nsh_core::prelude::*;

#[derive(Clone)]
pub(crate) struct ChangesState {
    /// 变更清单浮层（设计板 11）是否打开。
    pub(crate) changelist_open: Signal<bool>,
    /// 当前重命名对话框对应的清单；关闭时为 `None`。
    pub(crate) rename_changelist: Signal<Option<String>>,
    /// 重命名对话框中的编辑值。
    pub(crate) rename_changelist_value: Signal<String>,
    /// 按仓库标签保存已折叠的变更清单，避免面板重建时丢失折叠状态。
    pub(crate) collapsed_changelists: Signal<BTreeMap<u64, BTreeSet<Option<String>>>>,
    /// 按仓库标签和清单保存文件树展开状态。
    pub(crate) changelist_tree_expanded: Signal<BTreeMap<(u64, Option<String>), Vec<SharedString>>>,
}
