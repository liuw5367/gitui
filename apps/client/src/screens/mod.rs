//! 屏幕：欢迎页、仓库管理页与工作区。
//!
//! `lib.rs` 只负责状态与动作，屏幕函数只负责布局与呈现。

mod repo_manager;
mod welcome;
mod workspace;

pub(crate) use repo_manager::repo_manager;
pub(crate) use welcome::welcome;
pub(crate) use workspace::workspace;
