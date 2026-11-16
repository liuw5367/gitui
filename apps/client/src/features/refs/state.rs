//! 引用管理功能的 Signal 集合：reflog 的过滤关键字。
//!
//! 只放属于本功能、且不跨功能的信号；分支、远程、标签、贮藏、工作树与 reflog
//! 条目都是每个仓库各自的数据，挂在 `Tab` 上，仍留在 `Tab` 里。
//!
//! Refs / Reflog 的页签选择没有放进来：它原本由工作区在每次重建正文时创建，
//! 本阶段只搬代码、不动这个重建时机，因此仍留在视图侧（见 `refs_view`）。

use nsh_core::prelude::*;

/// Refs 主区的两个页签。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RefsTab {
    Refs,
    Reflog,
}

impl RefsTab {
    /// 页签文案的 i18n 键。
    pub(crate) fn key(self) -> &'static str {
        return match self {
            RefsTab::Refs => "refs.title",
            RefsTab::Reflog => "reflog.title",
        };
    }
}

#[derive(Clone)]
pub(crate) struct RefsState {
    /// Reflog 的过滤关键字；同时匹配动作、说明与短 sha，空串不过滤。
    pub(crate) reflog_query: Signal<String>,
}
