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
    selected: impl Fn() -> &'static str + 'static,
    on_select: impl Fn(&'static str) + 'static,
) -> impl IntoElement {
    let on_select = std::rc::Rc::new(on_select);
    let selected: std::rc::Rc<dyn Fn() -> &'static str> = std::rc::Rc::new(selected);
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
        let chosen = selected.clone();
        let is_selected: std::rc::Rc<dyn Fn() -> bool> =
            std::rc::Rc::new(move || chosen() == id);
        let press = on_select.clone();
        let selected_state = is_selected.clone();
        let background = is_selected.clone();
        let border = is_selected.clone();
        let hover = is_selected.clone();
        let foreground = is_selected.clone();
        tabs = tabs.child(
            nsh_base::tab()
                .selected(move || selected_state())
                .selected_style(|style| style.font_weight(FontWeight::MEDIUM))
                .h(px(ui::CONTROL - 4.0))
                .px(px(10.0))
                .flex_row()
                .items_center()
                .justify_center()
                .rounded(px(metrics::RADIUS_CONTROL - 1.0))
                .bg(move || {
                    if background() {
                        design::content()
                    } else {
                        AdaptiveColor::TRANSPARENT
                    }
                })
                .border_1()
                .border_color(move || {
                    if border() {
                        design::line()
                    } else {
                        AdaptiveColor::TRANSPARENT
                    }
                })
                .hover(move |style| {
                    if hover() {
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
                        .text_color(move || {
                            if foreground() {
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
