//! 功能切片：每个功能一个目录，固定 `state` / `commands` / `view` 三个文件。
//!
//! `state.rs` 只放本功能的 `Signal` 集合，`commands.rs` 只放本功能的用例（调
//! `git::Repo` / `model` 并更新 state），`view.rs` 只放本功能的渲染。

use nsh_core::prelude::*;

pub(crate) mod changes;
pub(crate) mod commit;
pub(crate) mod history;
pub(crate) mod merge;
pub(crate) mod refs;
pub(crate) mod sync;

/// 标签页集合与活动页签的句柄。用例要读写当前标签页时用它，避免逐个传信号。
///
/// P4 会把 `Tab` 的壳收进 `app/session.rs`，届时这里只剩类型别名。
#[derive(Clone, Copy)]
pub(crate) struct SessionHandle {
    pub(crate) tabs: Signal<Vec<crate::model::Tab>>,
    pub(crate) active: Signal<usize>,
}
