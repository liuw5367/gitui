//! 区域标题与空态、骨架：面板内重复出现的结构件。
//!
//! 设计稿在各板里都有一层「面板标题 + 计数 + 右侧动作」的条，尺寸从 23 到 32 不等；
//! 这里统一为 28 高、13pt 标题、11pt 计数，右侧动作由调用方以 `ui` 组件传入。

use crate::design;
use crate::ui;
use nsh_core::prelude::*;

/// 面板标题条：标题 + 可选计数 + 右侧动作槽。
pub(crate) fn pane_header(
    caption: impl Fn() -> String + 'static,
    counter: Option<Box<dyn Fn() -> String>>,
    actions: Vec<AnyElement>,
) -> impl IntoElement {
    let mut row = view()
        .w_full()
        .h(px(ui::ROW))
        .flex_row()
        .items_center()
        .gap(px(ui::GAP_SM))
        .px(px(ui::GAP_LG))
        .child(
            text(move || caption())
                .text_size(px(ui::BODY))
                .line_height(px(ui::ROW))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || design::text()),
        );
    if let Some(value) = counter {
        row = row.child(ui::badge::count(move || value()));
    }
    row = row.child(view().flex_1());
    for action in actions {
        row = row.child(action);
    }
    return row;
}

/// 分组标题：11pt、500 字重、二级色、sentence case。
pub(crate) fn section_label(caption: impl Fn() -> String + 'static) -> impl IntoElement {
    return view()
        .w_full()
        .h(px(ui::ROW_COMPACT))
        .flex_row()
        .items_center()
        .px(px(ui::GAP_LG))
        .child(
            text(move || caption())
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::ROW_COMPACT))
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || design::text2()),
        );
}

/// 欢迎页左栏的分组标题：全大写、600 字重、0.5px 字距。
///
/// 主窗口侧栏用 sentence case；欢迎页画板把这里做成大写，作为入口页的区分。
pub(crate) fn section_label_upper(caption: impl Fn() -> String + 'static) -> impl IntoElement {
    return view()
        .w_full()
        .h(px(23.0))
        .flex_row()
        .items_center()
        .px(px(ui::GAP_LG))
        .child(
            text(move || caption().to_uppercase())
                .text_size(px(ui::SECONDARY))
                .line_height(px(23.0))
                .font_weight(FontWeight::SEMIBOLD)
                .letter_spacing(px(0.5))
                .text_color(move || design::text2()),
        );
}

/// 空态：标题 + 说明 + 可选动作，垂直居中。
pub(crate) fn empty(
    caption: impl Fn() -> String + 'static,
    description: Option<Box<dyn Fn() -> String>>,
    action: Option<AnyElement>,
) -> impl IntoElement {
    let mut block = view()
        .w_full()
        .flex_1()
        .min_h_0()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(ui::GAP_SM))
        .p(px(ui::GAP_XL))
        .child(
            text(move || caption())
                .text_size(px(ui::BODY))
                .line_height(px(ui::BODY_LINE))
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || design::text2()),
        );
    if let Some(hint) = description {
        block = block.child(
            text(move || hint())
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        );
    }
    if let Some(element) = action {
        block = block.child(element);
    }
    return block;
}

/// 骨架行：加载时占位，行高与真实行一致，避免加载完成后跳动。
pub(crate) fn skeleton_row(widths: &'static [f32]) -> impl IntoElement {
    let mut row = view()
        .w_full()
        .h(px(ui::ROW))
        .flex_row()
        .items_center()
        .gap(px(ui::GAP));
    for width in widths {
        row = row.child(
            view()
                .w(px(*width))
                .h(px(10.0))
                .rounded(px(5.0))
                .bg(move || design::line_soft()),
        );
    }
    return row;
}
