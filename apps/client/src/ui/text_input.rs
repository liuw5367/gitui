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

/// 值由结构体信号承载时的输入框（`get`/`set` 各自闭包）。
pub(crate) fn text_input_bound(
    get: impl Fn() -> String + 'static,
    placeholder: impl Fn() -> String + 'static,
    invalid: bool,
    set: impl Fn(String) + 'static,
) -> impl IntoElement {
    return style_input(
        nsh_base::input()
            .value(move || get())
            .placeholder(placeholder)
            .on_change(move |text: &str| set(text.to_string())),
        invalid,
        ui::CONTROL,
    );
}

/// 路径输入：输入框 + 右侧键帽，整体一个描边容器。
///
/// 用于 Open 的仓库路径、Clone 的目标目录与 Init 的目录；⌘V 由系统输入框处理，
/// 键帽只作提示。
pub(crate) fn path_input(
    value: Signal<String>,
    placeholder: impl Fn() -> String + 'static,
    shortcut: Option<&'static str>,
    on_change: impl Fn(&str) + 'static,
) -> impl IntoElement {
    let mut container = view()
        .h(px(ui::CONTROL))
        .flex_row()
        .items_center()
        .gap(px(ui::GAP_SM))
        .px(px(ui::GAP))
        .rounded(px(metrics::RADIUS_CONTROL))
        .border_1()
        .border_color(move || design::field_border())
        .bg(move || design::field())
        .focus_visible(move |style| {
            style
                .outline(px(ui::FOCUS_WIDTH))
                .outline_color(move || ui::focus_color())
        })
        .child(
            nsh_base::input()
                .value(value)
                .placeholder(placeholder)
                .on_change(move |text: &str| on_change(text))
                .text_size(px(ui::BODY))
                .text_color(move || design::text())
                .flex_1(),
        );
    if let Some(combo) = shortcut {
        container = container.child(ui::badge::kbd(combo));
    }
    return container;
}

/// 搜索框：左侧放大镜、右侧可选键帽，整体一个描边容器。
///
/// 输入框自身不画边框，避免容器与输入框两道描边；焦点环画在容器上。
pub(crate) fn search_field(
    value: Signal<String>,
    placeholder: impl Fn() -> String + 'static,
    shortcut: Option<&'static str>,
    on_change: impl Fn(&str) + 'static,
) -> impl IntoElement {
    return search_field_inner(value, placeholder, shortcut, on_change, None);
}

/// 与 [`search_field`] 相同，并把输入框挂到 `handle` 上，供快捷键聚焦。
pub(crate) fn search_field_ref(
    value: Signal<String>,
    placeholder: impl Fn() -> String + 'static,
    shortcut: Option<&'static str>,
    on_change: impl Fn(&str) + 'static,
    handle: &ElementRef,
) -> impl IntoElement {
    return search_field_inner(value, placeholder, shortcut, on_change, Some(*handle));
}

fn search_field_inner(
    value: Signal<String>,
    placeholder: impl Fn() -> String + 'static,
    shortcut: Option<&'static str>,
    on_change: impl Fn(&str) + 'static,
    handle: Option<ElementRef>,
) -> impl IntoElement {
    let mut input = nsh_base::input()
        .value(value)
        .placeholder(placeholder)
        .on_change(move |text: &str| on_change(text))
        .text_size(px(ui::BODY))
        .text_color(move || design::text())
        .flex_1();
    if let Some(handle) = handle {
        input = input.element_ref(&handle);
    }
    let mut container = view()
        .h(px(ui::CONTROL))
        .flex_row()
        .items_center()
        .gap(px(ui::GAP_SM))
        .px(px(ui::GAP))
        .rounded(px(metrics::RADIUS_CONTROL))
        .border_1()
        .border_color(move || design::field_border())
        .bg(move || design::field())
        .focus_visible(move |style| {
            style
                .outline(px(ui::FOCUS_WIDTH))
                .outline_color(move || ui::focus_color())
        })
        .child(ui::icon::icon_view(
            "search",
            ui::icon::IconSize::Inline,
            design::text3,
        ))
        .child(input);
    if let Some(combo) = shortcut {
        container = container.child(ui::badge::kbd(combo));
    }
    return container;
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
