//! 文本输入：`nsh_core` 的原生输入框（NSTextField / NSTextView），这里统一外观。
//!
//! 高度、内边距、圆角与描边按原生刻度；失焦与聚焦不改尺寸，错误态换描边颜色，
//! 并在调用方补一行 11pt 的错误文案（不只靠颜色表达）。

use crate::design::{self, metrics};
use crate::ui;
use nsh_core::prelude::*;

/// 单行输入框，值受控于 `value`。
pub(crate) fn text_input(
    value: Signal<String>,
    placeholder: impl Fn() -> String + 'static,
    invalid: bool,
    on_change: impl Fn(&str) + 'static,
) -> impl IntoElement {
    return style_input(
        nsh_base::input()
            .value(value)
            .placeholder(placeholder)
            .on_change(move |text: &str| on_change(text)),
        invalid,
        ui::CONTROL,
    );
}

/// 多行输入框（提交说明、附注、冲突结果）。
pub(crate) fn text_area(
    value: Signal<String>,
    placeholder: impl Fn() -> String + 'static,
    height: f32,
    invalid: bool,
    on_change: impl Fn(&str) + 'static,
) -> impl IntoElement {
    return style_input(
        nsh_base::input()
            .multiline(true)
            .value(value)
            .placeholder(placeholder)
            .on_change(move |text: &str| on_change(text)),
        invalid,
        height,
    );
}

/// 共用外观：焦点环、描边、底色与 13pt 文字。
fn style_input(input: nsh_core::element::Input, invalid: bool, height: f32) -> impl IntoElement {
    return input
        .w_full()
        .h(px(height))
        .min_h(px(height))
        .px(px(ui::GAP))
        .py(px(ui::GAP_XS))
        .rounded(px(metrics::RADIUS_CONTROL))
        .border_1()
        .border_color(move || {
            if invalid {
                design::danger()
            } else {
                design::field_border()
            }
        })
        .bg(move || design::field())
        .text_size(px(ui::BODY))
        .text_color(move || design::text())
        .focus_visible(move |style| {
            style
                .outline(px(ui::FOCUS_WIDTH))
                .outline_color(move || ui::focus_color())
        });
}
