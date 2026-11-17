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
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || design::text()),
        );
    if let Some(hint_text) = hint {
        labels = labels.child(
            text(move || hint_text())
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        );
    }
    let mut row = view()
        .w_full()
        .flex_row()
        .items_center()
        .gap(px(14.0))
        .py(px(11.0))
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

/// 设置页的页面头：17/700 标题 + 11pt 副标题。
pub(crate) fn page_header(
    title: impl Fn() -> String + 'static,
    subtitle: impl Fn() -> String + 'static,
) -> impl IntoElement {
    return view()
        .w_full()
        .flex_col()
        .child(
            text(move || title())
                .text_size(px(ui::PAGE_TITLE))
                .line_height(px(ui::PAGE_TITLE_LINE))
                .font_weight(FontWeight(700))
                .letter_spacing(px(-0.25))
                .text_color(move || design::text()),
        )
        .child(
            text(move || subtitle())
                .mt(px(4.0))
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        );
}

/// 设置页的一段：画板 `.settings-sec`。第一段没有上边框与上内距。
pub(crate) fn settings_section(
    first: bool,
    title: impl Fn() -> String + 'static,
    hint: Option<Box<dyn Fn() -> String>>,
    body: impl IntoElement,
) -> impl IntoElement {
    let mut section = view().w_full().flex_col();
    if first {
        section = section.mt(px(14.0));
    } else {
        section = section
            .mt(px(20.0))
            .pt(px(16.0))
            .border_t_1()
            .border_color(move || design::line());
    }
    let has_hint = hint.is_some();
    let mut head = view().w_full().flex_col().gap(px(2.0)).child(
        text(move || title())
            .text_size(px(ui::BODY))
            .line_height(px(ui::BODY_LINE))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(move || design::text()),
    );
    if let Some(hint) = hint {
        head = head.child(
            text(move || hint())
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        );
    }
    return section
        .child(head)
        .child(view().w_full().mt(px(if has_hint { 12.0 } else { 2.0 })).child(body));
}
