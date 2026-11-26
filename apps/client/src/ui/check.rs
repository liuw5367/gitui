//! 复选行与开关行：行为、键盘激活与无障碍状态来自 `nsh_base`，这里负责外观。
//!
//! 整行都是按压目标（文案作为组件的子元素），行高固定 28；指示器尺寸与圆角按
//! 系统外观，选中时用强调色，未选中时用描边。

use crate::design::{self, metrics};
use crate::ui;
use nsh_core::element::IntoProp;
use nsh_core::prelude::*;

/// 可操作的 tri-state 复选框，用于工作区文件的暂存状态。
pub(crate) fn indicator_action_state<MC, MI>(
    checked: impl IntoProp<bool, MC> + 'static,
    indeterminate: impl IntoProp<bool, MI> + 'static,
    label: impl Into<SharedString>,
    on_change: impl Fn(bool) + 'static,
) -> impl IntoElement {
    let checked: Prop<bool> = checked.into_prop().into();
    let indeterminate: Prop<bool> = indeterminate.into_prop().into();
    return nsh_base::checkbox()
        .accessibility_label(Some(label.into()))
        .checked(checked)
        .indeterminate(indeterminate)
        .w(px(12.0))
        .h(px(12.0))
        .flex_shrink_0()
        .indicator_style(move |style| {
            style
                .w(px(12.0))
                .h(px(12.0))
                .items_center()
                .justify_center()
                .rounded(px(3.0))
                .border_1()
                .bg(move || {
                    if checked.get() || indeterminate.get() {
                        design::accent()
                    } else {
                        AdaptiveColor::TRANSPARENT
                    }
                })
                .border_color(move || {
                    if checked.get() || indeterminate.get() {
                        design::accent()
                    } else {
                        design::line_strong()
                    }
                })
        })
        .indicator(dynamic(move || -> AnyElement {
            if indeterminate.get() {
                return text("−")
                    .text_size(px(ui::META))
                    .text_color(move || design::content())
                    .into_element();
            }
            if checked.get() {
                return crate::ui::icon::icon_view(
                    "check",
                    crate::ui::icon::IconSize::Compact,
                    move || design::content(),
                )
                .into_element();
            }
            return view().into_element();
        }))
        .on_change(move |next: &bool| on_change(*next));
}

/// 复选行：左侧 14pt 指示器 + 13pt 文案。
/// 一个 12×12 的方框标记：选中时填强调色并画对勾。
///
/// 用于「不能整行勾选」的场合——例如表格单元格里表示某文件已暂存。它不是
/// 按钮，没有自己的按压：点击由外层（行或单元格）负责。
pub(crate) fn indicator(checked: bool) -> impl IntoElement {
    return view()
        .w(px(12.0))
        .h(px(12.0))
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .rounded(px(3.0))
        .border_1()
        .border_color(move || {
            if checked {
                design::accent()
            } else {
                design::line_strong()
            }
        })
        .bg(move || {
            if checked {
                design::accent()
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .child(
            text(if checked {
                "✓".to_string()
            } else {
                String::new()
            })
            .text_size(px(ui::META))
            .text_color(move || design::content()),
        );
}

/// 行内复选：与 [`check_row`] 同一套外观，但高度与指示器尺寸由调用方给。
///
/// `check_row` 是列表用的整行（28 高、14pt 指示器）；表单里偶尔需要在 20–22 高的
/// 一行内放复选（「创建后检出」「跳过钩子」），用它。
pub(crate) fn check_inline<MC, MD>(
    caption: impl Fn() -> String + 'static,
    checked: impl IntoProp<bool, MC> + 'static,
    row_height: f32,
    indicator: f32,
    disabled: impl IntoProp<bool, MD> + 'static,
    on_change: impl Fn(bool) + 'static,
) -> impl IntoElement {
    let checked: Prop<bool> = checked.into_prop().into();
    let disabled: Prop<bool> = disabled.into_prop().into();
    return nsh_base::checkbox()
        .checked(checked)
        .disabled(disabled)
        .opacity(move || if disabled.get() { 0.45_f32 } else { 1.0_f32 })
        .h(px(row_height))
        .flex_row()
        .items_center()
        .gap(px(ui::GAP_SM))
        .indicator_style(move |style| {
            let style = style
                .w(px(indicator))
                .h(px(indicator))
                .items_center()
                .justify_center()
                .rounded(px(3.0))
                .border_1();
            style
                .bg(move || {
                    if checked.get() {
                        design::token("primary-bg")
                    } else {
                        design::field()
                    }
                })
                .border_color(move || {
                    if checked.get() {
                        design::token("primary-bg")
                    } else {
                        design::field_border()
                    }
                })
        })
        .indicator(dynamic(move || -> AnyElement {
            if !checked.get() {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            }
            return crate::ui::icon::icon_view(
                "check",
                crate::ui::icon::IconSize::Compact,
                move || design::token("primary-foreground"),
            )
            .into_element();
        }))
        .on_change(move |next: &bool| on_change(*next))
        .child(
            text(move || caption())
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::SECONDARY_LINE))
                .text_color(move || design::text()),
        );
}

pub(crate) fn check_row<MC, MD>(
    caption: impl Fn() -> String + 'static,
    checked: impl IntoProp<bool, MC> + 'static,
    disabled: impl IntoProp<bool, MD> + 'static,
    on_change: impl Fn(bool) + 'static,
) -> impl IntoElement {
    let checked: Prop<bool> = checked.into_prop().into();
    let disabled: Prop<bool> = disabled.into_prop().into();
    return nsh_base::checkbox()
        .checked(checked)
        .disabled(disabled)
        .opacity(move || if disabled.get() { 0.45_f32 } else { 1.0_f32 })
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
            style
                .bg(move || {
                    if checked.get() {
                        design::token("primary-bg")
                    } else {
                        design::field()
                    }
                })
                .border_color(move || {
                    if checked.get() {
                        design::token("primary-bg")
                    } else {
                        design::field_border()
                    }
                })
        })
        .indicator(dynamic(move || -> AnyElement {
            if !checked.get() {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            }
            return view()
                .w(px(9.0))
                .h(px(9.0))
                .child(ui::icon::icon_view(
                    "check",
                    ui::icon::IconSize::Button,
                    || design::token("primary-foreground"),
                ))
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
    let mut labels = view().flex_1().min_w_0().flex_col().child(
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

/// 复选行，用按钮实现整行按压；用于会被 `dynamic` 重建的表单。
///
/// `nsh_base::checkbox` 的 `checked` 是绑定到当前 owner 的 prop，动态区域销毁时
/// 会读到已释放的 owner。表单里的复选改用按压目标 + 自绘方框，避开这条销毁路径；
/// 键盘与无障碍语义仍由按钮提供，方框只表达状态。
pub(crate) fn check_row_button(
    caption: impl Fn() -> String + 'static,
    checked: bool,
    on_change: impl Fn(bool) + 'static,
) -> impl IntoElement {
    let label: SharedString = caption().into();
    return nsh_base::button()
        .accessibility_label(Some(label))
        .h(px(ui::ROW))
        .w_full()
        .flex_row()
        .items_center()
        .gap(px(ui::GAP))
        .child(
            view()
                .w(px(14.0))
                .h(px(14.0))
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .border_1()
                .border_color(move || {
                    if checked {
                        design::token("primary-bg")
                    } else {
                        design::field_border()
                    }
                })
                .bg(move || {
                    if checked {
                        design::token("primary-bg")
                    } else {
                        design::field()
                    }
                })
                .child(if checked {
                    ui::icon::icon_view("check", ui::icon::IconSize::Button, || {
                        design::token("primary-foreground")
                    })
                    .into_element()
                } else {
                    view().w(px(0.0)).h(px(0.0)).into_element()
                }),
        )
        .on_press(move |_: &()| on_change(!checked))
        .child(
            text(move || caption())
                .text_size(px(ui::BODY))
                .line_height(px(ui::BODY_LINE))
                .text_color(move || design::text()),
        );
}

/// 行右的独立复选：只有 14×14 方框，无障碍名由 `label` 提供。
///
/// 设置页的「标签在左、复选在右」行用它，避免把整行做成复选而改变标签排版。
pub(crate) fn check_control(
    label: impl Fn() -> String + 'static,
    checked: bool,
    disabled: bool,
    on_change: impl Fn(bool) + 'static,
) -> impl IntoElement {
    let accessibility = label();
    return nsh_base::button()
        .accessibility_label(Some(SharedString::from(accessibility)))
        .disabled(disabled)
        .opacity(if disabled { 0.45 } else { 1.0 })
        .w(px(18.0))
        .h(px(18.0))
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .focus_visible(move |style| {
            style
                .outline(px(ui::FOCUS_WIDTH))
                .outline_color(move || ui::focus_color())
        })
        .on_press(move |_: &()| on_change(!checked))
        .child(
            view()
                .w(px(14.0))
                .h(px(14.0))
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .border_1()
                .border_color(move || {
                    if checked {
                        design::token("primary-bg")
                    } else {
                        design::field_border()
                    }
                })
                .bg(move || {
                    if checked {
                        design::token("primary-bg")
                    } else {
                        design::field()
                    }
                })
                .child(if checked {
                    ui::icon::icon_view("check", ui::icon::IconSize::Compact, || {
                        design::token("primary-foreground")
                    })
                    .into_element()
                } else {
                    view().w(px(0.0)).h(px(0.0)).into_element()
                }),
        );
}
