//! 基础控件：按钮、图标按钮、键帽、分隔线。
//!
//! 尺寸与配色来自 `design::metrics` 与 `design` 的 token；这里不放业务文案，
//! 文案由调用方传入（已翻译）。

use crate::design::{self, icons, metrics};
use nsh_core::prelude::*;

/// 普通按钮：`.btn`（高 24、字号 12、圆角 6、1px 边框）。
///
/// 文案用闭包而不是字符串，是为了让按钮跟随语言切换。
pub(crate) fn button(
    label: impl Fn() -> String + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    return view()
        .h(px(metrics::BUTTON_H))
        .px(px(10.0))
        .flex_row()
        .items_center()
        .justify_center()
        .gap(px(5.0))
        .rounded(px(metrics::RADIUS_CONTROL))
        .border_1()
        .border_color(move || design::btn_border())
        .bg(move || design::btn())
        .hover(|style| style.bg(move || design::hover()))
        .active(|style| style.bg(move || design::line_soft()))
        .on_press(on_press)
        .child(
            text(move || label())
                .text_size(px(12.0))
                .text_color(move || design::text()),
        );
}

/// 小号按钮：`.btn.small`（高 22、字号 11）。
pub(crate) fn small_button(
    label: impl Fn() -> String + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    return view()
        .h(px(metrics::BUTTON_SMALL_H))
        .px(px(8.0))
        .flex_row()
        .items_center()
        .justify_center()
        .rounded(px(5.0))
        .border_1()
        .border_color(move || design::btn_border())
        .bg(move || design::btn())
        .hover(|style| style.bg(move || design::hover()))
        .on_press(on_press)
        .child(
            text(move || label())
                .text_size(px(11.0))
                .text_color(move || design::text()),
        );
}

/// 图标按钮：`.tb-icon`（26×24、圆角 6）；图标颜色跟随传入的 token。
pub(crate) fn icon_button(
    name: &'static str,
    size: f32,
    color: impl Fn() -> AdaptiveColor + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let mut button = view()
        .w(px(26.0))
        .h(px(24.0))
        .items_center()
        .justify_center()
        .rounded(px(metrics::RADIUS_CONTROL))
        .hover(|style| style.bg(move || design::hover()))
        .on_press(on_press);
    if let Some(source) = icons::icon(name) {
        button = button.child(
            svg(source)
                .w(px(size))
                .h(px(size))
                .text_color(move || color()),
        );
    }
    return button;
}

/// 纯图标，不可点击：用于行内标记（分支、标签、文件）。
pub(crate) fn icon(
    name: &'static str,
    size: f32,
    color: impl Fn() -> AdaptiveColor + 'static,
) -> impl IntoElement {
    let mut holder = view().w(px(size)).h(px(size)).flex_shrink_0();
    if let Some(source) = icons::icon(name) {
        holder = holder.child(
            svg(source)
                .w(px(size))
                .h(px(size))
                .text_color(move || color()),
        );
    }
    return holder;
}

/// 复选行：原生 `Checkbox`（按压、键盘激活、checkbox 角色与值由组件负责）+
/// 自绘指示器与文案。
///
/// 文案作为 `Checkbox` 的子元素，因此整行都是按压目标，屏幕阅读器也能读到。
/// `checked` 在构建时取值：调用方所在的 `dynamic` 会在状态变化后重建这一行。
pub(crate) fn check(
    label: impl Fn() -> String + 'static,
    checked: bool,
    size: f32,
    on_change: impl Fn(bool) + 'static,
) -> impl IntoElement {
    return nsh_base::checkbox()
        .checked(checked)
        .indicator_style(move |style| {
            let style = style
                .w(px(size))
                .h(px(size))
                .items_center()
                .justify_center()
                .rounded(px(3.0))
                .border_1();
            if checked {
                style
                    .bg(move || design::accent2())
                    .border_color(move || design::accent2())
            } else {
                style
                    .bg(move || design::field())
                    .border_color(move || design::line_strong())
            }
        })
        .indicator(dynamic(move || -> AnyElement {
            if !checked {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            }
            return view()
                .w(px(size - 5.0))
                .h(px(size - 5.0))
                .child(icon("check", size - 5.0, || {
                    design::token("primary-foreground")
                }))
                .into_element();
        }))
        .on_change(move |next: &bool| on_change(*next))
        .child(
            text(move || label())
                .text_size(px(11.0))
                .line_height(px(14.0))
                .text_color(move || design::text2()),
        );
}

