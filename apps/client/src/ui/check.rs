//! 复选行与开关行：行为、键盘激活与无障碍状态来自 `nsh_base`，这里负责外观。
//!
//! 整行都是按压目标（文案作为组件的子元素），行高固定 28；指示器尺寸与圆角按
//! 系统外观，选中时用强调色，未选中时用描边。

use crate::design::{self, metrics};
use crate::ui;
use nsh_core::prelude::*;

/// 复选行：左侧 14pt 指示器 + 13pt 文案。
pub(crate) fn check_row(
    caption: impl Fn() -> String + 'static,
    checked: bool,
    disabled: bool,
    on_change: impl Fn(bool) + 'static,
) -> impl IntoElement {
    return nsh_base::checkbox()
        .checked(checked)
        .disabled(disabled)
        .opacity(if disabled { 0.45 } else { 1.0 })
        .h(px(ui::ROW))
        .flex_row()
        .items_center()
        .gap(px(ui::GAP))
        .indicator_style(move |style| {
            let style = style
                .w(px(14.0))
                .h(px(14.0))
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .border_1();
            if checked {
                style
                    .bg(move || design::token("primary-bg"))
                    .border_color(move || design::token("primary-bg"))
            } else {
                style
                    .bg(move || design::field())
                    .border_color(move || design::field_border())
            }
        })
        .indicator(dynamic(move || -> AnyElement {
            if !checked {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            }
            return view()
                .w(px(9.0))
                .h(px(9.0))
                .child(ui::icon::icon_view("check", ui::icon::IconSize::Button, || {
                    design::token("primary-foreground")
                }))
                .into_element();
        }))
        .on_change(move |next: &bool| on_change(*next))
        .child(
            text(move || caption())
                .text_size(px(ui::BODY))
                .line_height(px(ui::BODY_LINE))
                .text_color(move || design::text()),
        );
}

/// 开关行：右侧系统样式的开关，左侧文案与可选说明。
pub(crate) fn switch_row(
    caption: impl Fn() -> String + 'static,
    hint: Option<Box<dyn Fn() -> String>>,
    checked: bool,
    disabled: bool,
    on_change: impl Fn(bool) + 'static,
) -> impl IntoElement {
    let mut row = view()
        .w_full()
        .min_h(px(ui::ROW))
        .flex_row()
        .items_center()
        .gap(px(ui::GAP_LG))
        .opacity(if disabled { 0.45 } else { 1.0 });
    let mut labels = view()
        .flex_1()
        .min_w_0()
        .flex_col()
        .child(
            text(move || caption())
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
    row = row.child(labels).child(
        nsh_base::switch()
            .checked(checked)
            .disabled(disabled)
            .w(px(38.0))
            .h(px(22.0))
            .rounded(px(11.0))
            .p(px(2.0))
            .track_style(move |style| {
                let style = style.w(px(38.0)).h(px(22.0)).rounded(px(11.0));
                if checked {
                    style.bg(move || design::token("primary-bg"))
                } else {
                    style
                        .bg(move || design::line_strong())
                        .border_1()
                        .border_color(move || design::field_border())
                }
            })
            .thumb_style(move |style| {
                style
                    .w(px(18.0))
                    .h(px(18.0))
                    .rounded(px(9.0))
                    .bg(move || design::content())
            })
            .on_change(move |next: &bool| on_change(*next)),
    );
    return row;
}

/// 复选行的边框容器：用于成组的复选项（设计稿的 `.checkrow`）。
pub(crate) fn check_group(children: impl IntoElement) -> impl IntoElement {
    return view()
        .w_full()
        .flex_col()
        .gap(px(ui::GAP_XS))
        .p(px(ui::GAP))
        .rounded(px(metrics::RADIUS_CONTROL))
        .border_1()
        .border_color(move || design::line())
        .child(children);
}
