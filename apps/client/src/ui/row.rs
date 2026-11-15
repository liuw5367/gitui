//! 列表行与导航行：行级按压目标统一在这里，页面不再直接给 `view()` 挂 `.on_press`。
//!
//! 单行 28、双行 44；选中态用强调底与正文色；行内图标、徽章、计数由调用方以
//! `ui` 组件作为前后槽传入。行为（按压、禁用、键盘可达）来自 `nsh_base::button`。

use crate::design;
use crate::ui;
use nsh_core::prelude::*;

/// 一行列表项。
pub(crate) fn row(
    caption: impl Fn() -> String + 'static,
    subtitle: Option<Box<dyn Fn() -> String>>,
    leading: Option<AnyElement>,
    trailing: Vec<AnyElement>,
    selected: bool,
    disabled: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let height = if subtitle.is_some() {
        44.0
    } else {
        ui::ROW
    };
    let mut element = nsh_base::button()
        .disabled(disabled)
        .opacity(if disabled { 0.45 } else { 1.0 })
        .w_full()
        .h(px(height))
        .flex_row()
        .items_center()
        .gap(px(ui::GAP_SM))
        .px(px(ui::GAP))
        .rounded(px(6.0))
        .bg(move || {
            if selected {
                design::selected()
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .hover(move |style| {
            if selected {
                style
            } else {
                style.bg(move || design::hover())
            }
        })
        .focus_visible(move |style| {
            style
                .outline(px(ui::FOCUS_WIDTH))
                .outline_color(move || ui::focus_color())
        })
        .on_press(move |_: &()| on_press());
    if let Some(leading) = leading {
        element = element.child(leading);
    }
    let foreground = if selected {
        design::token("sel-strong")
    } else {
        design::text()
    };
    let mut labels = view()
        .flex_1()
        .min_w_0()
        .flex_col()
        .child(
            text(move || caption())
                .text_size(px(ui::BODY))
                .line_height(px(ui::BODY_LINE))
                .text_color(move || foreground)
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        );
    if let Some(hint) = subtitle {
        labels = labels.child(
            text(move || hint())
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::SECONDARY_LINE))
                .text_color(move || design::text3())
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        );
    }
    element = element.child(labels);
    for item in trailing {
        element = element.child(item);
    }
    return element;
}
