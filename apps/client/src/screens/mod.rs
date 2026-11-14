//! 屏幕：欢迎页、仓库管理页与工作区。
//!
//! `lib.rs` 只负责状态与动作，屏幕函数只负责布局与呈现。

mod changes_view;
mod palette;
mod repo_manager;
mod welcome;
mod workspace;

pub(crate) use changes_view::changes_view;
pub(crate) use palette::palette;

/// 命令面板的命令列表（供根组件与测试共用）。
pub(crate) fn palette_commands() -> Vec<palette::Command> {
    return palette::commands();
}
pub(crate) use repo_manager::repo_manager;
pub(crate) use welcome::welcome;
pub(crate) use workspace::workspace;
