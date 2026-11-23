//! 根组件：窗口属性、主题装载、语言状态与屏幕路由。
//!
//! 一个窗口、多个仓库标签页；屏幕为欢迎页、仓库管理页与工作区。当前只读：
//! 打开/克隆/初始化仓库、按条件筛选历史、多提交对比、查看提交内单文件补丁。
//! 写路径在后续阶段加入。
//!
//! P4 起本文件只留模块声明与根转发：引导在 `app::bootstrap`，主窗口版式在
//! `app::workspace`，菜单、快捷键与窗口函数分别在 `app::{menu, hotkeys, windows}`。

mod acceptance;
mod app;
mod askpass;
pub mod brand;
mod dates;
mod design;
mod features;
mod filters;
mod i18n;
mod interactions;
mod shared;
mod shell;
mod shortcuts;
#[cfg(test)]
mod test_support;
// ui 层已建立；P1–P7 逐组迁入后仍有为后续阶段准备的组件，P8 收尾时
// 全部组件都有使用者，届时删除这条 allow 并保持零警告。
#[allow(dead_code)]
mod ui;
mod widgets;

use nsh_core::prelude::*;

/// 供子模块使用的设计常量。
pub use design::metrics;

pub(crate) use app::Mode;
pub(crate) use app::windows::{persist, window_insets};

/// Run this executable as the lightweight Git/OpenSSH AskPass helper when requested.
pub fn run_askpass_helper() -> Option<i32> {
    return askpass::run_askpass_helper();
}

/// 应用根组件。
#[component]
pub fn App() -> impl IntoElement {
    app::bootstrap::app()
}

/// NSH 原生模块入口：注册本 crate 的组件。
///
/// `#[native_module]` 必须在 crate 根（NSH 的生成代码按 `crate::__nsh_native_module`
/// 调用），因此这里只转发到 `app::hotkeys::register`。
#[native_module("app/native")]
fn native(registry: &mut Registry) {
    app::hotkeys::register(registry);
}
