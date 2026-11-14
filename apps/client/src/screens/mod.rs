//! 屏幕：欢迎页、工作区与设置。
//!
//! `lib.rs` 只负责状态与动作，屏幕函数只负责布局与呈现。欢迎页是**独立窗口**
//! （`id = "welcome"`），标签页工作区与设置在同一个窗口的 `main` 里。

mod changes_view;
mod palette;
mod settings;
mod welcome;
mod workspace;

pub(crate) use changes_view::changes_view;
pub(crate) use palette::palette;
pub(crate) use settings::{settings, Section as SettingsSection};
pub(crate) use welcome::welcome;
pub(crate) use workspace::workspace;

/// 命令面板的命令列表（供根组件与测试共用）。
pub(crate) fn palette_commands() -> Vec<palette::Command> {
    return palette::commands();
}
