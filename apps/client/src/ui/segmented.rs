//! 分段控件：设计稿的 `.seg`，行为来自 `nsh_base::tabs`（每段自己处理按压与
//! 选中语义），外观由这里统一：高 28、圆角 6、选中段使用卡片底色与描边。

use crate::design::{self, metrics};
use crate::ui;
use nsh_core::prelude::*;

/// 一段的内容。
pub(crate) struct Segment {
    /// 用于比较的稳定标识。
    pub(crate) id: &'static str,
    /// 显示文案（已翻译）。
    pub(crate) label: Box<dyn Fn() -> String>,
}

/// 分段控件。`selected` 是当前选中段的 `id`。
pub(crate) fn segmented(
    segments: Vec<Segment>,
    selected: &'static str,
    on_select: impl Fn(&'static str) + 'static,
) -> impl IntoElement {
    let on_select = std::rc::Rc::new(on_select);
    let mut tabs = nsh_base::tabs()
        .h(px(ui::CONTROL))
        .flex_row()
        .items_center()
        .gap(px(2.0))
        .p(px(2.0))
        .rounded(px(metrics::RADIUS_CONTROL))
        .bg(move || design::line_soft())
        .border_1()
        .border_color(move || design::line());
    for segment in segments {
        let id = segment.id;
        let label = segment.label;
        let is_selected = id == selected;
        let press = on_select.clone();
        tabs = tabs.child(
            nsh_base::tab()
                .selected(is_selected)
                .h(px(ui::CONTROL - 4.0))
                .px(px(10.0))
                .flex_row()
                .items_center()
                .justify_center()
                .rounded(px(metrics::RADIUS_CONTROL - 1.0))
                .bg(move || {
                    if is_selected {
                        design::content()
                    } else {
                        AdaptiveColor::TRANSPARENT
                    }
                })
                .border_1()
                .border_color(move || {
                    if is_selected {
                        design::line()
                    } else {
                        AdaptiveColor::TRANSPARENT
                    }
                })
                .hover(move |style| {
                    if is_selected {
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
                .on_press(move |_: &()| press(id))
                .child(
                    text(move || label())
                        .text_size(px(ui::BODY))
                        .line_height(px(ui::CONTROL - 4.0))
                        .font_weight(if is_selected {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(move || {
                            if is_selected {
                                design::text()
                            } else {
                                design::text2()
                            }
                        }),
                ),
        );
    }
    return tabs;
}
