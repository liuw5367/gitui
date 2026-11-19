//! 应用组装层：引导、窗口、菜单、快捷键、会话与主窗口版式。
//!
//! 这里只放"把各功能拼起来"的代码；具体用例在 `features/*/commands.rs`，版式在
//! `features/*/view.rs`。`Mode` 是主区模式的根类型，其余模块都经 `crate::Mode` 引用。

pub(crate) mod bootstrap;
pub(crate) mod handles;
pub(crate) mod hotkeys;
pub(crate) mod menu;
pub(crate) mod ops;
pub(crate) mod session;
pub(crate) mod windows;
pub(crate) mod workspace;

/// 工作区主区模式；Blame 是 History / Changes 之外的单文件追溯页。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    History,
    Changes,
    Refs,
    Agent,
    /// 单文件的 blame + 文件历史（板 19），从提交文件或工作区 diff 进入。
    Blame,
}
