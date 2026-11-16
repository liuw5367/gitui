//! 搜索功能的状态：命令面板的开关/高亮/过滤词、工作区主区模式与表单浮层。

use nsh_core::prelude::*;

/// 命令面板、视图模式切换与表单浮层的信号集合。
///
/// 三者放在一起，因为命令面板正是这些动作的入口：`⌘P` 打开面板，面板里的命令
/// 改主区模式（History / Changes / Refs / Agent）或打开表单；Esc 的浮层退出顺序
/// 也由根组件按这一份状态统一处理。
#[derive(Clone, Copy)]
pub(crate) struct SearchState {
    /// 命令面板是否打开。
    pub(crate) palette_open: Signal<bool>,
    /// 面板当前高亮行（下标落在结果表里，超出时由渲染层夹到末行）。
    pub(crate) palette_selected: Signal<usize>,
    /// 面板的过滤输入：同时匹配仓库路径与命令文案。
    pub(crate) palette_query: Signal<String>,
    /// 工作区主区的模式；工具栏分段与命令面板的命令都改它。
    pub(crate) mode: Signal<crate::Mode>,
    /// 表单浮层（分支/标签/远程/贮藏/身份）；`None` 表示关闭。
    pub(crate) form: Signal<Option<crate::screens::FormState>>,
}
