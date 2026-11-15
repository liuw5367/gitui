//! 模态对话框：遮罩 + 居中面板 + 页头/正文/页脚三段。
//!
//! 行为与设计稿一致：点击遮罩关闭、内容区截获按压、Esc 由调用方在全局浮层栈里处理。
//! 页头 44、页脚 48、正文内边距 16；面板最大高度 760，正文自己滚动。
//!
//! 返回值是 `AnyElement`：`open` 为 false 时返回零尺寸占位。调用方在构建时读一次
//! `open`（信号读取会订阅重建），不需要在这里再做一层 `dynamic`。

use crate::design::{self, metrics};
use crate::ui;
use nsh_core::prelude::*;

/// 一个模态对话框。
pub(crate) fn dialog(
    open: bool,
    width: f32,
    title: impl Fn() -> String + 'static,
    body: impl IntoElement,
    footer: Vec<AnyElement>,
    on_dismiss: impl Fn() + 'static,
) -> AnyElement {
    if !open {
        return view().w(px(0.0)).h(px(0.0)).into_element();
    }
    let mut panel = view()
        .w(px(width))
        .max_h(px(760.0))
        .flex_col()
        .rounded(px(metrics::RADIUS_PANEL + 2.0))
        .border_1()
        .border_color(move || design::line_strong())
        .bg(move || design::token("popover"))
        .shadow_lg()
        // 内容区的按压不再上浮到遮罩，避免点正文就关掉对话框。
        .on_press(|| {})
        .child(
            view()
                .w_full()
                .h(px(44.0))
                .flex_row()
                .items_center()
                .gap(px(ui::GAP))
                .px(px(ui::GAP_XL))
                .border_b_1()
                .border_color(move || design::line())
                .child(
                    text(move || title())
                        .flex_1()
                        .min_w_0()
                        .text_size(px(ui::BODY))
                        .line_height(px(44.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || design::text())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                ),
        )
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .overflow_hidden()
                .p(px(ui::GAP_XL))
                .child(body),
        );
    if !footer.is_empty() {
        let mut row = view()
            .w_full()
            .h(px(48.0))
            .flex_row()
            .items_center()
            .justify_end()
            .gap(px(ui::GAP))
            .px(px(ui::GAP_XL))
            .border_t_1()
            .border_color(move || design::line());
        for element in footer {
            row = row.child(element);
        }
        panel = panel.child(row);
    }
    return view()
        .absolute()
        .inset(px(0.0))
        .z_index(120)
        .items_center()
        .justify_center()
        .bg(move || design::token("overlay"))
        .on_press(move || on_dismiss())
        .child(panel)
        .into_element();
}
