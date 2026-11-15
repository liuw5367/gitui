//! 按钮：`nsh_base::button` 提供按压、禁用、焦点与无障碍，这里决定外观。
//!
//! 四种变体对应设计稿的 `.btn`、`.btn.primary`、透明图标按钮与危险按钮；尺寸只有
//! 三档，间距与圆角来自 `ui` 的常量，页面不再各自写高度与字号。

use crate::design::{self, metrics};
use crate::ui::icon::{icon_view, IconSize};
use crate::ui::{self, focus_color, FOCUS_WIDTH};
use nsh_core::prelude::*;

/// 按钮的语义变体。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ButtonVariant {
    /// 主操作：实心强调色。
    Primary,
    /// 次操作：描边按钮。
    Secondary,
    /// 不占版面的轻按钮：工具栏、行内图标。
    Ghost,
    /// 危险操作：实心危险色，只用于不可撤销的动作。
    Danger,
}

/// 按钮尺寸：22 / 28 / 32 / 26（工具栏）。
///
/// `Toolbar` 与 `Regular` 同字阶但更矮、内距更小，对应设计稿工具栏项的 26pt；
/// 它不替代其它档位，只为工具栏这种「图标 + 短文案、密集排布」的场合存在。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ButtonSize {
    Small,
    Regular,
    Large,
    Toolbar,
}

impl ButtonSize {
    fn height(self) -> f32 {
        return match self {
            ButtonSize::Small => ui::CONTROL_SMALL,
            ButtonSize::Regular => ui::CONTROL,
            ButtonSize::Large => ui::CONTROL_LARGE,
            ButtonSize::Toolbar => crate::design::metrics::TBTN_H,
        };
    }

    fn pad_x(self) -> f32 {
        return match self {
            ButtonSize::Small => 8.0,
            ButtonSize::Regular => 12.0,
            ButtonSize::Large => 16.0,
            ButtonSize::Toolbar => 9.0,
        };
    }

    fn line(self) -> f32 {
        return match self {
            ButtonSize::Toolbar => ui::BODY_LINE,
            _ => self.height(),
        };
    }

    fn font(self) -> f32 {
        return match self {
            ButtonSize::Small => ui::SECONDARY,
            ButtonSize::Regular | ButtonSize::Large | ButtonSize::Toolbar => ui::BODY,
        };
    }
}

/// 文字按钮。`label` 用闭包，语言切换后重建时取新文案。
/// 带前置图标的文字按钮（工具栏项：图标 + 文案）。
///
/// 与 [`button`] 同一套变体与尺寸，只是文案前多一个图标。
pub(crate) fn button_icon(
    label: impl Fn() -> String + 'static,
    icon: &'static str,
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let foreground = match variant {
        ButtonVariant::Primary | ButtonVariant::Danger => design::token("primary-foreground"),
        ButtonVariant::Secondary => design::text(),
        ButtonVariant::Ghost => design::text2(),
    };
    let mut element = nsh_base::button()
        .disabled(disabled)
        .opacity(if disabled { 0.45 } else { 1.0 })
        .h(px(size.height()))
        .px(px(size.pad_x()))
        .flex_row()
        .items_center()
        .gap(px(5.0))
        .rounded(px(metrics::RADIUS_CONTROL))
        .focus_visible(move |style| {
            style
                .outline(px(FOCUS_WIDTH))
                .outline_color(move || focus_color())
        })
        .on_press(move |_: &()| on_press());
    element = match variant {
        ButtonVariant::Primary => element
            .bg(move || design::token("primary-bg"))
            .hover(move |style| style.opacity(0.9)),
        ButtonVariant::Secondary => element
            .bg(move || design::btn())
            .border_1()
            .border_color(move || design::btn_border())
            .hover(move |style| style.bg(move || design::hover())),
        ButtonVariant::Ghost => element
            .bg(AdaptiveColor::TRANSPARENT)
            .hover(move |style| style.bg(move || design::hover())),
        ButtonVariant::Danger => element
            .bg(move || design::danger())
            .hover(move |style| style.opacity(0.9)),
    };
    return element
        .child(crate::ui::icon::inline_icon(icon))
        .child(
            text(move || label())
                .text_size(px(size.font()))
                .line_height(px(size.line()))
                .text_color(move || foreground),
        );
}

pub(crate) fn button(
    label: impl Fn() -> String + 'static,
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let mut element = nsh_base::button()
        .disabled(disabled)
        .opacity(if disabled { 0.45 } else { 1.0 })
        .h(px(size.height()))
        .px(px(size.pad_x()))
        .flex_row()
        .items_center()
        .justify_center()
        .gap(px(ui::GAP_SM))
        .rounded(px(metrics::RADIUS_CONTROL))
        .focus_visible(move |style| {
            style
                .outline(px(FOCUS_WIDTH))
                .outline_color(move || focus_color())
        })
        .on_press(move |_: &()| on_press());

    element = match variant {
        ButtonVariant::Primary => element
            .bg(move || design::token("primary-bg"))
            .hover(move |style| style.opacity(0.9))
            .active(move |style| style.opacity(0.8)),
        ButtonVariant::Secondary => element
            .bg(move || design::btn())
            .border_1()
            .border_color(move || design::btn_border())
            .hover(move |style| style.bg(move || design::hover()))
            .active(move |style| {
                style
                    .bg(move || design::line_soft())
                    .opacity(0.9)
            }),
        ButtonVariant::Ghost => element
            .bg(AdaptiveColor::TRANSPARENT)
            .hover(move |style| style.bg(move || design::hover()))
            .active(move |style| style.bg(move || design::line_soft())),
        ButtonVariant::Danger => element
            .bg(move || design::danger())
            .hover(move |style| style.opacity(0.9))
            .active(move |style| style.opacity(0.8)),
    };

    let foreground = match variant {
        ButtonVariant::Primary | ButtonVariant::Danger => design::token("primary-foreground"),
        ButtonVariant::Secondary => design::text(),
        ButtonVariant::Ghost => design::text2(),
    };
    return element.child(
        text(move || label())
            .text_size(px(size.font()))
            .line_height(px(size.line()))
            .font_weight(FontWeight::MEDIUM)
            .text_color(move || foreground),
    );
}

/// 图标按钮：内容只有一个图标，`label` 同时作为无障碍名称。
pub(crate) fn icon_button(
    name: &'static str,
    label: impl Fn() -> String + 'static,
    size: ButtonSize,
    disabled: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let icon_size = match size {
        ButtonSize::Small => IconSize::Button,
        ButtonSize::Regular | ButtonSize::Large | ButtonSize::Toolbar => IconSize::Toolbar,
    };
    // 画板的两个图标按钮类不是同一个比例：`.iconbtn` 是 24×24 正方形，
    // `.tb-icon` 是 26×24（工具栏里略宽，视觉上更像一块「按钮」而非圆点）。
    let (width, height) = match size {
        ButtonSize::Toolbar => (26.0, 24.0),
        other => (other.height(), other.height()),
    };
    let label_for_a11y = label();
    return nsh_base::button()
        .disabled(disabled)
        .opacity(if disabled { 0.45 } else { 1.0 })
        .accessibility_label(Some(SharedString::from(label_for_a11y)))
        .w(px(width))
        .h(px(height))
        .items_center()
        .justify_center()
        .rounded(px(metrics::RADIUS_CONTROL))
        .bg(AdaptiveColor::TRANSPARENT)
        .hover(move |style| style.bg(move || design::hover()))
        .active(move |style| style.bg(move || design::line_soft()))
        .focus_visible(move |style| {
            style
                .outline(px(FOCUS_WIDTH))
                .outline_color(move || focus_color())
        })
        .on_press(move |_: &()| on_press())
        .child(icon_view(name, icon_size, design::text2));
}
