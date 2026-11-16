//! 搜索功能：命令面板（设计板 20）、视图模式切换与表单浮层的开关。
//!
//! `state.rs` 放面板与表单的信号，`commands.rs` 放命令分发与表单提交的用例，
//! `view.rs` 放面板渲染，`form_dialog.rs` 放表单浮层；历史筛选浮层仍归
//! `features/history`。

pub(crate) mod commands;
pub(crate) mod form_dialog;
pub(crate) mod ops;
pub(crate) mod state;
pub(crate) mod view;

pub(crate) use form_dialog::{form_dialog, FormKind, FormState};
