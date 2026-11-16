//! 键值行：左标签 + 右值，用于提交信息、设置摘要、存储路径这类元数据。
//!
//! 标签固定宽 96、11pt 二级色；值 13pt，等宽内容（SHA、tree、路径）由调用方用
//! `mono` 标记，走 12pt `ui::MONO`。行高 24，与 `ui::ROW_COMPACT` 一致。

use crate::design;
use crate::ui;
use nsh_core::prelude::*;

/// 标签列宽度。设计板 08 的 `.insp-k` 实测在 84–100 之间，取 96。
const LABEL_WIDTH: f32 = 96.0;

/// 一行键值。
pub(crate) fn key_value(
    label: impl Fn() -> String + 'static,
    value: impl Fn() -> String + 'static,
    mono: bool,
) -> impl IntoElement {
    let mut line = text(move || value())
        .flex_1()
        .min_w_0()
        .text_size(px(if mono { ui::CODE } else { ui::BODY }))
        .line_height(px(ui::CODE_LINE))
        .text_color(move || design::text())
        .text_overflow(nsh_core::EllipsizeMode::Tail);
    if mono {
        line = line.font_family(ui::MONO);
    }
    return view()
        .w_full()
        .min_h(px(ui::ROW_COMPACT))
        .flex_row()
        .items_center()
        .gap(px(ui::GAP))
        .child(
            text(move || label())
                .w(px(LABEL_WIDTH))
                .flex_shrink_0()
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::SECONDARY_LINE))
                .text_color(move || design::text2()),
        )
        .child(line);
}

/// 一组键值行，行间 2pt 间距。
pub(crate) fn key_values(rows: Vec<AnyElement>) -> impl IntoElement {
    let mut column = view().w_full().flex_col().gap(px(2.0));
    for row in rows {
        column = column.child(row);
    }
    return column;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_column_is_wide_enough_for_the_longest_key() {
        // Info 页最长的键是 "Committed"（9 字符），11pt 下约 62pt；96 留出余量，
        // 同时不至于把右侧值挤到换行。
        assert!(LABEL_WIDTH >= 88.0);
        assert!(LABEL_WIDTH <= 120.0);
    }
}
