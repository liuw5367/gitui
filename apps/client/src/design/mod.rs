//! 设计层：颜色 token、几何常量与图标。
//!
//! 颜色来自 `theme.json`（与设计稿 CSS 的 `:root` / `.theme-dark` 同值），几何来自
//! 共享样式表与各设计板的内联尺寸。界面只通过本模块取这两类值，避免散落的字面量。

pub mod icons;
pub mod metrics;

use nsh_base::theme;
use nsh_core::prelude::*;

/// 取一个设计 token；未定义时退回前景色，便于早发现拼写错误。
pub(crate) fn token(name: &'static str) -> AdaptiveColor {
    let theme = theme();
    return theme.color(name).unwrap_or(theme.foreground);
}

// 表面
pub(crate) fn content() -> AdaptiveColor {
    return token("content");
}

pub(crate) fn panel() -> AdaptiveColor {
    return token("panel");
}

pub(crate) fn panel2() -> AdaptiveColor {
    return token("panel2");
}

pub(crate) fn sidebar() -> AdaptiveColor {
    return token("sidebar");
}

pub(crate) fn bar() -> AdaptiveColor {
    return token("bar");
}

pub(crate) fn titlebar() -> AdaptiveColor {
    return token("titlebar");
}

pub(crate) fn status() -> AdaptiveColor {
    return token("status");
}

pub(crate) fn field() -> AdaptiveColor {
    return token("field");
}

// 文本
pub(crate) fn text() -> AdaptiveColor {
    return token("text");
}

pub(crate) fn text2() -> AdaptiveColor {
    return token("text2");
}

pub(crate) fn text3() -> AdaptiveColor {
    return token("text3");
}

// 线条
pub(crate) fn line() -> AdaptiveColor {
    return token("line");
}

pub(crate) fn line_soft() -> AdaptiveColor {
    return token("line-soft");
}

pub(crate) fn line_strong() -> AdaptiveColor {
    return token("line-strong");
}

// 强调与语义色
/// 设计稿里的 `--accent`（链接与图标蓝），与 `primary`（主按钮底色）不同。
pub(crate) fn accent() -> AdaptiveColor {
    return token("blue");
}

pub(crate) fn accent2() -> AdaptiveColor {
    return token("accent-2");
}

pub(crate) fn accent_soft() -> AdaptiveColor {
    return token("accent-soft");
}

pub(crate) fn selected() -> AdaptiveColor {
    return token("sel");
}

pub(crate) fn hover() -> AdaptiveColor {
    return token("hover");
}

pub(crate) fn branch_bg() -> AdaptiveColor {
    return token("branch-bg");
}

pub(crate) fn branch_fg() -> AdaptiveColor {
    return token("branch-fg");
}

pub(crate) fn tag_bg() -> AdaptiveColor {
    return token("tag-bg");
}

pub(crate) fn tag_fg() -> AdaptiveColor {
    return token("tag-fg");
}

pub(crate) fn remote_fg() -> AdaptiveColor {
    return token("remote-fg");
}

pub(crate) fn add_bg() -> AdaptiveColor {
    return token("add-bg");
}

pub(crate) fn add_fg() -> AdaptiveColor {
    return token("add-fg");
}

pub(crate) fn del_bg() -> AdaptiveColor {
    return token("del-bg");
}

pub(crate) fn del_fg() -> AdaptiveColor {
    return token("del-fg");
}

pub(crate) fn warn() -> AdaptiveColor {
    return token("warn");
}

pub(crate) fn danger() -> AdaptiveColor {
    return token("danger");
}

pub(crate) fn ok() -> AdaptiveColor {
    return token("ok");
}

pub(crate) fn field_border() -> AdaptiveColor {
    return token("field-border");
}

pub(crate) fn btn() -> AdaptiveColor {
    return token("btn");
}

pub(crate) fn btn_border() -> AdaptiveColor {
    return token("btn-border");
}
