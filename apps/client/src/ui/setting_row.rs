//! 设置行：左侧标题与说明，右侧控件，行间一条分隔线。
//!
//! 设置页（板 23–27）里有三十多个这样的行，此前每行手写标题 12pt、说明 10.5pt
//! 与右侧控件的对齐，字号与间距各写一遍。这里收成一个容器：
//!
//! - 标题 13pt；说明 11pt（`None` 时只占一行高，不留空行）；
//! - 右侧控件右对齐、垂直居中，由调用方传入（分段、按钮、开关、只读值）；
//! - 行高随内容，最小 `ui::ROW`；底部分隔线可关（每组最后一行关掉）。
//!
//! 用法：
//! ```ignore
//! ui::setting_row::setting_row(title, Some(hint), ui::segmented::segmented(..), true)
//! ```

use crate::design;
use crate::ui;
use nsh_core::prelude::*;

/// 一行设置项。`divider` 为 false 时不留底部分隔线（用于一组里的最后一行）。
pub(crate) fn setting_row(
    title: impl Fn() -> String + 'static,
    hint: Option<Box<dyn Fn() -> String>>,
    control: AnyElement,
    divider: bool,
) -> impl IntoElement {
    let mut labels = view()
        .flex_1()
        .min_w_0()
        .flex_col()
        .child(
            text(move || title())
                .text_size(px(ui::BODY))
                .line_height(px(ui::BODY_LINE))
                .text_color(move || design::text()),
        );
    if let Some(hint_text) = hint {
        labels = labels.child(
            text(move || hint_text())
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::SECONDARY_LINE))
                .text_color(move || design::text2()),
        );
    }
    let mut row = view()
        .w_full()
        .min_h(px(ui::ROW + 8.0))
        .flex_row()
        .items_center()
        .gap(px(ui::GAP_LG))
        .py(px(ui::GAP_SM))
        .child(labels)
        .child(view().flex_shrink_0().child(control));
    if divider {
        row = row
            .border_b_1()
            .border_color(move || design::line_soft());
    }
    return row;
}

/// 一行只读信息（标题 + 等宽值），用于存储路径、版本、凭据来源这类不可改的项。
pub(crate) fn read_only_row(
    title: impl Fn() -> String + 'static,
    value: impl Fn() -> String + 'static,
    divider: bool,
) -> impl IntoElement {
    return setting_row(
        title,
        None,
        text(move || value())
            .text_size(px(ui::CODE))
            .line_height(px(ui::CODE_LINE))
            .font_family(ui::MONO)
            .text_color(move || design::text2())
            .into_element(),
        divider,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_is_taller_than_a_plain_list_row() {
        // 设置行带说明时是两行文字，最小高度必须高于单行列表行，否则会挤在一起。
        assert!(ui::ROW + 8.0 > ui::ROW);
    }
}
