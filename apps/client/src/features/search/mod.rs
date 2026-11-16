//! 搜索功能：命令面板（设计板 20）、视图模式切换与表单浮层的开关。
//!
//! `state.rs` 放面板与表单的信号，`commands.rs` 放命令分发与表单提交的用例，
//! `view.rs` 放面板渲染，`form_dialog.rs` 放表单浮层；历史筛选浮层仍归
//! `features/history`。

use nsh_core::prelude::*;

pub(crate) mod commands;
pub(crate) mod form_dialog;
pub(crate) mod ops;
pub(crate) mod state;
pub(crate) mod view;

pub(crate) use form_dialog::{form_dialog, FormKind, FormState};

/// 表单浮层的现成接线：提交与取消都交给搜索用例。
///
/// 主窗口与设置窗口各画一份，调用方按"哪扇窗在前面"决定由谁渲染（见
/// `app::bootstrap` 与 `app::windows`），避免两扇窗各弹一个同样的表单。
pub(crate) fn form_overlay(
    commands: &commands::SearchCommands,
    language: Signal<SharedString>,
) -> impl IntoElement {
    let submit = commands.clone();
    let cancel = commands.clone();
    return form_dialog(
        language,
        commands.state.form,
        std::sync::Arc::new(move |form: FormState| submit.submit_form(form)),
        std::sync::Arc::new(move || cancel.cancel_form()),
    );
}
