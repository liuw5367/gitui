//! 按钮：`nsh_base::button` 提供按压、禁用、焦点与无障碍，这里决定外观。
//!
//! 四种变体对应设计稿的 `.btn`、`.btn.primary`、透明图标按钮与危险按钮；尺寸只有
//! 三档，间距与圆角来自 `ui` 的常量，页面不再各自写高度与字号。

use crate::design::{self, metrics};
use crate::ui::icon::{IconSize, icon_view};
use crate::ui::{self, FOCUS_WIDTH, focus_color};
use nsh_core::element::IntoProp;
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
    return button_icon_inner(label, None, icon, variant, size, disabled, on_press);
}

/// 带前置图标和独立无障碍名称的文字按钮。
pub(crate) fn button_icon_with_accessibility_label(
    label: impl Fn() -> String + 'static,
    accessibility_label: impl Fn() -> String + 'static,
    icon: &'static str,
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    return button_icon_inner(
        label,
        Some(SharedString::from(accessibility_label())),
        icon,
        variant,
        size,
        disabled,
        on_press,
    );
}

fn button_icon_inner(
    label: impl Fn() -> String + 'static,
    accessibility_label: Option<SharedString>,
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
    if let Some(accessibility_label) = accessibility_label {
        element = element.accessibility_label(Some(accessibility_label));
    }
    return element.child(crate::ui::icon::inline_icon(icon)).child(
        text(move || label())
            .text_size(px(size.font()))
            .line_height(px(size.line()))
            .text_color(move || foreground),
    );
}

/// 按钮外壳：变体、尺寸、焦点与按压行为。内容由 [`button`] / [`button_hint`] 补。
fn button_shell<MD>(
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: impl IntoProp<bool, MD> + 'static,
    on_press: impl Fn() + 'static,
) -> nsh_base::ButtonElement {
    let disabled: Prop<bool> = disabled.into_prop().into();
    let mut element = nsh_base::button()
        .disabled(disabled)
        .opacity(move || if disabled.get() { 0.45 } else { 1.0_f32 })
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
            .active(move |style| style.bg(move || design::line_soft()).opacity(0.9)),
        ButtonVariant::Ghost => element
            .bg(AdaptiveColor::TRANSPARENT)
            .hover(move |style| style.bg(move || design::hover()))
            .active(move |style| style.bg(move || design::line_soft())),
        ButtonVariant::Danger => element
            .bg(move || design::danger())
            .hover(move |style| style.opacity(0.9))
            .active(move |style| style.opacity(0.8)),
    };
    return element;
}

/// 变体的前景色。
fn foreground(variant: ButtonVariant) -> AdaptiveColor {
    return match variant {
        ButtonVariant::Primary | ButtonVariant::Danger => design::token("primary-foreground"),
        ButtonVariant::Secondary => design::text(),
        ButtonVariant::Ghost => design::text2(),
    };
}

pub(crate) fn button<MD>(
    label: impl Fn() -> String + 'static,
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: impl IntoProp<bool, MD> + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let color = foreground(variant);
    return button_shell(variant, size, disabled, on_press).child(
        text(move || label())
            .text_size(px(size.font()))
            .line_height(px(size.line()))
            .font_weight(FontWeight::MEDIUM)
            .text_color(move || color),
    );
}

/// 文字按钮 + 行内快捷键提示：欢迎页的 `Open Repository  ⌘O`。
///
/// 提示用同一前景色的弱化不透明度，不再套一层键帽——键帽是给输入框旁与行尾用的，
/// 放在实心主按钮上会变成第二块底色。
pub(crate) fn button_hint(
    label: impl Fn() -> String + 'static,
    hint: impl Fn() -> String + 'static,
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let color = foreground(variant);
    return button_shell(variant, size, disabled, on_press)
        .child(
            text(move || label())
                .text_size(px(size.font()))
                .line_height(px(size.line()))
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || color),
        )
        .child(
            text(move || hint())
                .text_size(px(size.font() - 1.0))
                .line_height(px(size.line()))
                .text_color(move || color)
                .opacity(0.72),
        );
}

/// 图标按钮：内容只有一个图标，`label` 同时作为无障碍名称。
pub(crate) fn icon_button(
    name: &'static str,
    label: impl Fn() -> String + 'static,
    size: ButtonSize,
    disabled: bool,
    on_press: impl Fn() + 'static,
) -> nsh_base::ButtonElement {
    return icon_button_tuned(name, label, size, None, None, disabled, on_press);
}

/// 同 [`icon_button`]，但可指定图标尺寸与前景色。
pub(crate) fn icon_button_tuned(
    name: &'static str,
    label: impl Fn() -> String + 'static,
    size: ButtonSize,
    icon_size_override: Option<IconSize>,
    foreground: Option<std::rc::Rc<dyn Fn() -> AdaptiveColor>>,
    disabled: bool,
    on_press: impl Fn() + 'static,
) -> nsh_base::ButtonElement {
    return icon_shell(
        name,
        label,
        size,
        icon_size_override,
        foreground,
        disabled,
        on_press,
    );
}

/// 同 [`icon_button_tuned`]，并把按下位置交给调用方，用于锚定自定义浮层。
pub(crate) fn icon_button_tuned_at(
    name: &'static str,
    label: impl Fn() -> String + 'static,
    size: ButtonSize,
    icon_size_override: Option<IconSize>,
    foreground: Option<std::rc::Rc<dyn Fn() -> AdaptiveColor>>,
    disabled: bool,
    on_press: impl Fn(Point<Points>, Point<Points>) + 'static,
) -> impl IntoElement {
    let icon_size = icon_size_override.unwrap_or(match size {
        ButtonSize::Small => IconSize::Button,
        ButtonSize::Regular | ButtonSize::Large | ButtonSize::Toolbar => IconSize::Toolbar,
    });
    let (width, height) = match size {
        ButtonSize::Toolbar => (26.0, 24.0),
        other => (other.height(), other.height()),
    };
    let label_for_a11y = label();
    return view()
        .disabled(disabled)
        .opacity(if disabled { 0.45 } else { 1.0 })
        .role(AccessibilityRole::Button)
        .label(SharedString::from(label_for_a11y))
        .w(px(width))
        .h(px(height))
        .flex_shrink_0()
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
        .on_press(move |event: &PressEvent| on_press(event.page_position, event.position))
        .child(icon_view(name, icon_size, move || match &foreground {
            Some(color) => color(),
            None => design::text2(),
        }));
}

/// 图标按钮 + hover 提示：指针停留半秒后在按钮上方显示 `tip`。
///
/// 用于只有图标、没有文字的入口（欢迎页底栏的检查更新与设置）：图标本身认不出来
/// 时，提示就是它的文案。
pub(crate) fn icon_button_tip(
    name: &'static str,
    label: impl Fn() -> String + 'static,
    size: ButtonSize,
    disabled: bool,
    tip: impl Fn() -> String + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    // `nsh_base` 的按钮元素不带 `.tooltip`（提示是视图上的 overlay），所以套一层
    // 与按钮同尺寸的 view，把提示挂在它上面。
    return view()
        .flex_shrink_0()
        .child(icon_shell(
            name, label, size, None, None, disabled, on_press,
        ))
        .tooltip(tooltip_bubble(tip));
}

/// 同 [`icon_button_tip`]，并可调整图标颜色与按钮底色。
pub(crate) fn icon_button_tuned_tip(
    name: &'static str,
    label: impl Fn() -> String + 'static,
    size: ButtonSize,
    icon_size_override: Option<IconSize>,
    foreground: Option<std::rc::Rc<dyn Fn() -> AdaptiveColor>>,
    background: Option<std::rc::Rc<dyn Fn() -> AdaptiveColor>>,
    disabled: bool,
    tip: impl Fn() -> String + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let button = icon_shell(
        name,
        label,
        size,
        icon_size_override,
        foreground,
        disabled,
        on_press,
    )
    .bg(move || match &background {
        Some(color) => color(),
        None => AdaptiveColor::TRANSPARENT,
    });
    return view()
        .flex_shrink_0()
        .child(button)
        .tooltip(tooltip_bubble(tip));
}

/// 图标按钮的外壳（无障碍名、尺寸、悬停底色与焦点环），内容由调用方补。
fn icon_shell(
    name: &'static str,
    label: impl Fn() -> String + 'static,
    size: ButtonSize,
    icon_size_override: Option<IconSize>,
    foreground: Option<std::rc::Rc<dyn Fn() -> AdaptiveColor>>,
    disabled: bool,
    on_press: impl Fn() + 'static,
) -> nsh_base::ButtonElement {
    let icon_size = icon_size_override.unwrap_or(match size {
        ButtonSize::Small => IconSize::Button,
        ButtonSize::Regular | ButtonSize::Large | ButtonSize::Toolbar => IconSize::Toolbar,
    });
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
        .flex_shrink_0()
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
        .child(icon_view(name, icon_size, move || match &foreground {
            Some(color) => color(),
            None => design::text2(),
        }));
}

/// 提示气泡：按钮上方的小卡片（`nsh_base` 的 tooltip 只提供定位与时机，外观在这里）。
fn tooltip_bubble(label: impl Fn() -> String + 'static) -> nsh_base::TooltipElement {
    return nsh_base::tooltip().child(
        view()
            .px(px(ui::GAP))
            .py(px(ui::GAP_XS))
            .rounded(px(metrics::RADIUS_CONTROL))
            .border_1()
            .border_color(move || design::line())
            .bg(move || design::panel())
            .child(
                text(move || label())
                    .text_size(px(ui::SECONDARY))
                    .line_height(px(ui::SECONDARY_LINE))
                    .text_color(move || design::text()),
            ),
    );
}

pub(crate) fn file_list_toggle(
    language: Signal<SharedString>,
    tree: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let label = move || {
        crate::i18n::t(
            language.get().as_str(),
            if tree {
                "changelists.flat"
            } else {
                "changelists.tree"
            },
        )
    };
    icon_button_tip(
        if tree { "tree" } else { "unified" },
        label,
        ButtonSize::Small,
        false,
        label,
        on_press,
    )
}
pub(crate) fn text_tooltip(label: String) -> nsh_base::TooltipElement {
    tooltip_bubble(move || label.clone())
}
