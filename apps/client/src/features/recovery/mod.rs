//! 恢复操作功能：reflog 撤销、重置、拣选与回滚。
//!
//! 本功能没有独立的大视图：入口是 refs 的 reflog 表与 history 的提交右键菜单，
//! 因此目录里只有用例；视图经 `features::refs` / `features::history` 调用。

pub(crate) mod commands;
