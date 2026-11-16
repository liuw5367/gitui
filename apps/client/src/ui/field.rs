//! 表单字段：标签、控件、说明与错误的固定排布。
//!
//! 设计稿里 `.field` 高度在不同板里是 24–26，这里统一到原生常规控件高度 28，
//! 标签 13、说明与错误 11；错误行用危险色，且不再加图标（颜色之外还有文案）。

use crate::design;
use crate::ui;
use nsh_core::prelude::*;

/// 字段标签。
pub(crate) fn label(caption: impl Fn() -> String + 'static) -> impl IntoElement {
    return text(move || caption())
        .text_size(px(ui::BODY))
        .line_height(px(ui::BODY_LINE))
        .font_weight(FontWeight::MEDIUM)
        .text_color(move || design::text());
}

/// 帮助文字。
pub(crate) fn help(hint: impl Fn() -> String + 'static) -> impl IntoElement {
    return text(move || hint())
        .text_size(px(ui::SECONDARY))
        .line_height(px(ui::SECONDARY_LINE))
        .text_color(move || design::text2());
}

/// 错误文字。
pub(crate) fn error(message: impl Fn() -> String + 'static) -> impl IntoElement {
    return text(move || message())
        .text_size(px(ui::SECONDARY))
        .line_height(px(ui::SECONDARY_LINE))
        .text_color(move || design::danger());
}

/// 一个字段：标签在上，控件居中，帮助或错误在下。
///
/// `error` 与 `help` 互斥：有错误时不显示帮助，避免两行小字互相挤压。
pub(crate) fn field(
    caption: impl Fn() -> String + 'static,
    control: impl IntoElement,
    help_text: Option<Box<dyn Fn() -> String>>,
    error_text: Option<Box<dyn Fn() -> String>>,
) -> impl IntoElement {
    let mut container = view()
        .w_full()
        .flex_col()
        .gap(px(ui::GAP_XS))
        .child(label(caption))
        .child(control);
    if let Some(message) = error_text {
        return container.child(error(move || message()));
    }
    if let Some(hint) = help_text {
        container = container.child(help(move || hint()));
    }
    return container;
}
