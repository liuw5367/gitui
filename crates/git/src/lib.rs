//! Git 执行层：只在这里拼装并运行 Git 命令，并把输出解析为 `domain` 的类型。
//!
//! 读路径与写路径按功能模块实现，涵盖仓库状态、历史、引用、同步、子模块与工作树。
//!
//! 模块划分：`exec` 起进程，`error` 认失败类别，`parse` 把文本变成类型，其余按
//! 功能分（`query` / `history` / `diff` / `stage` / `commit` / `branch` /
//! `remote` / `merge` / `submodule` / `worktree`）。`impl Repo` 分散在这些模块里，公开名字由本文件原样再
//! 导出，`git::Xxx` 的路径不变；测试跟着各自的模块走。

mod branch;
mod commit;
mod diff;
mod error;
mod exec;
mod history;
mod merge;
mod parse;
mod query;
mod remote;
mod stage;
mod submodule;
mod worktree;

pub use commit::*;
pub use error::*;
pub use exec::*;
pub use history::*;
pub use merge::*;
pub use query::*;
pub use remote::*;
