//! 页签条：面板内的互斥视图切换（检查器 Files / Info / Blame、同步 Fetch / Pull / Push）。
//!
//! 行为来自 `nsh_base::tabs`/`tab`（每一页签自己处理按压与选中语义，键盘可进入），
//! 外观是系统页签：行高 28、选中项文字转正文色并在底部画 2pt 强调线。

use crate::design;
use crate::ui;
use nsh_core::prelude::*;

/// 一个页签。
pub(crate) struct TabItem {
    /// 稳定标识，用于比较。
    pub(crate) id: &'static str,
    /// 显示文案（已翻译）。
    pub(crate) label: Box<dyn Fn() -> String>,
}

/// 页签条。`selected` 是当前页签的 `id`。
pub(crate) fn tabs(
    items: Vec<TabItem>,
    selected: &'static str,
    on_select: impl Fn(&'static str) + 'static,
) -> impl IntoElement {
    let on_select = std::rc::Rc::new(on_select);
    let mut bar = nsh_base::tabs()
        .w_full()
        .h(px(ui::ROW))
        .flex_row()
        .items_end()
        .gap(px(ui::GAP_XL))
        .border_b_1()
        .border_color(move || design::line_soft());
    for item in items {
        let id = item.id;
        let label = item.label;
        let is_selected = id == selected;
        let press = on_select.clone();
        bar = bar.child(
            nsh_base::tab()
                .selected(is_selected)
                .h(px(ui::ROW))
                .flex_col()
                .justify_end()
                .gap(px(6.0))
                .focus_visible(move |style| {
                    style
                        .outline(px(ui::FOCUS_WIDTH))
                        .outline_color(move || ui::focus_color())
                })
                .on_press(move |_: &()| press(id))
                .child(
                    text(move || label())
                        .text_size(px(ui::BODY))
                        .line_height(px(ui::BODY_LINE))
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
                )
                .child(view().w_full().h(px(2.0)).bg(move || {
                    if is_selected {
                        design::token("primary-bg")
                    } else {
                        AdaptiveColor::TRANSPARENT
                    }
                })),
        );
    }
    return bar;
}
