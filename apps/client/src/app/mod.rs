//! 应用组装层：引导、窗口、菜单、快捷键、会话与主窗口版式。
//!
//! 这里只放"把各功能拼起来"的代码；具体用例在 `features/*/commands.rs`，版式在
//! `features/*/view.rs`。`Screen` / `Mode` 是屏幕路由与主区模式的根类型，其余模块
//! 都经 `crate::Screen` / `crate::Mode` 引用。

pub(crate) mod bootstrap;
pub(crate) mod handles;
pub(crate) mod hotkeys;
pub(crate) mod menu;
pub(crate) mod ops;
pub(crate) mod session;
pub(crate) mod windows;
pub(crate) mod workspace;

/// 当前显示哪一屏。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Screen {
    Workspace,
    Settings,
}

/// 工作区主区的两种模式（工具栏的 History / Changes 分段）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    History,
    Changes,
    Refs,
    Agent,
    /// 单文件的 blame + 文件历史（板 19）。从检查器的 Blame 页或文件行进入，
    /// 不占工具栏的分段——画板的入口是 `.blamebar` 里的 Files 按钮。
    Blame,
}
