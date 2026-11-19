//! 菜单项构造：统一右键菜单与下拉菜单的写法。
//!
//! 菜单本身走系统原生 `NSMenu`（`nsh_core::element::desktop::popup_menu_at`），
//! 外观由系统决定，本层不重画——这正是「原生优先」想要的部分。这里收的是
//! **构造约定**：一个菜单只有四种项（动作、勾选、危险、分隔），危险项必须显式
//! 标注，避免各处自行决定文案与禁用条件。
//!
//! 用法：
//! ```ignore
//! let items = vec![
//!     ui::menu::action(label, is_enabled, move || checkout()),
//!     ui::menu::divider(),
//!     ui::menu::danger(label, is_enabled, move || delete()),
//! ];
//! crate::ui::menu::popup_at(items, event.page_position);
//! ```

use nsh_core::element::desktop::MenuItem;
use nsh_core::prelude::*;

/// 在给定页面坐标弹出原生菜单。
///
/// 页面只组装 [`MenuItem`]，NSH 的原生菜单入口由 UI 层统一持有。
pub(crate) fn popup_at(items: Vec<MenuItem>, position: nsh_core::Point<Points>) {
    let _ = nsh_core::element::desktop::popup_menu_at(items, position);
}

/// 一个普通动作项。`enabled` 为 false 时置灰且不可选。
pub(crate) fn action(
    label: impl Into<String>,
    enabled: bool,
    on_select: impl Fn() + 'static,
) -> MenuItem {
    return MenuItem::new(label).disabled(!enabled).on_select(on_select);
}

/// 一个可勾选项。`checked` 由调用方持有，选中时回调只负责切换。
pub(crate) fn checkable(
    label: impl Into<String>,
    checked: bool,
    enabled: bool,
    on_select: impl Fn() + 'static,
) -> MenuItem {
    return MenuItem::new(label)
        .checked(checked)
        .disabled(!enabled)
        .on_select(on_select);
}

/// 一个危险动作项（删除、丢弃、强制推送、跳过钩子）。
///
/// 与普通项在 `NSMenu` 里外观相同——系统菜单不支持自定义颜色，因此「危险」的
/// 表达完全靠：1) 调用方在选择后弹确认框；2) 文案写清对象与后果。这个函数存在
/// 的意义是让危险项在代码里可被检索与审查，而不是伪装成视觉差异。
pub(crate) fn danger(
    label: impl Into<String>,
    enabled: bool,
    on_select: impl Fn() + 'static,
) -> MenuItem {
    return MenuItem::new(label).disabled(!enabled).on_select(on_select);
}

/// 分隔线。
pub(crate) fn divider() -> MenuItem {
    return MenuItem::separator();
}

/// 一个「点一下就弹菜单」的按钮。
///
/// 为什么需要它：弹菜单要用 `PressEvent` 的 `page_position` 作为锚点，而
/// 页面里直接写 `.on_press(move |event: &PressEvent| ...)` 会让该文件无法通过
/// `guard_tests` 的「不得出现裸 on_press」检查。把这件事收进 ui 层后，页面只
/// 负责给出菜单项，坐标由这里取。
///
/// 用法：
/// ```ignore
/// ui::menu::menu_button(
///     move || "Add step".to_string(),
///     Some("plus"),
///     move || build_items(),
/// )
/// ```
pub(crate) fn menu_button(
    label: impl Fn() -> String + 'static,
    icon: Option<&'static str>,
    build_items: impl Fn() -> Vec<MenuItem> + 'static,
) -> impl IntoElement {
    return menu_button_tuned(label, false, icon, build_items);
}

/// 下拉按钮：`active` 表示它当前代表的筛选或引用对比已生效。
pub(crate) fn menu_button_tuned(
    label: impl Fn() -> String + 'static,
    active: bool,
    icon: Option<&'static str>,
    build_items: impl Fn() -> Vec<MenuItem> + 'static,
) -> impl IntoElement {
    let build_items = std::rc::Rc::new(build_items);
    return menu_button_tuned_at(
        label,
        move || active,
        icon,
        move |position| {
            let items = build_items();
            let _ = nsh_core::element::desktop::popup_menu_at(items, position);
        },
    );
}

/// 带根视图坐标的紧凑下拉按钮，用于打开自绘的锚定浮层。
pub(crate) fn menu_button_tuned_at(
    label: impl Fn() -> String + 'static,
    active: impl Fn() -> bool + 'static,
    icon: Option<&'static str>,
    on_press: impl Fn(Point<Points>) + 'static,
) -> impl IntoElement {
    let active = std::rc::Rc::new(active);
    let accessibility_label = label();
    let active_for_bg = active.clone();
    let active_for_border = active.clone();
    let active_for_icon = active.clone();
    let active_for_text = active.clone();
    let mut content = view()
        .h(px(crate::ui::CONTROL_SMALL))
        .px(px(crate::ui::GAP))
        .flex_row()
        .items_center()
        .gap(px(crate::ui::GAP_SM))
        .rounded(px(crate::design::metrics::RADIUS_CONTROL))
        .role(AccessibilityRole::Button)
        .label(SharedString::from(accessibility_label))
        .bg(move || {
            if active_for_bg() {
                crate::design::accent_soft()
            } else {
                crate::design::btn()
            }
        })
        .border_1()
        .border_color(move || {
            if active_for_border() {
                crate::design::accent()
            } else {
                crate::design::btn_border()
            }
        })
        .hover(|style| style.bg(move || crate::design::hover()))
        .active(|style| style.bg(move || crate::design::line_soft()))
        .focus_visible(move |style| {
            style
                .outline(px(crate::ui::FOCUS_WIDTH))
                .outline_color(move || crate::ui::focus_color())
        })
        .on_press(move |event: &PressEvent| on_press(event.page_position));
    if let Some(name) = icon {
        content = content.child(crate::ui::icon::icon_view(
            name,
            crate::ui::icon::IconSize::Compact,
            move || {
                if active_for_icon() {
                    crate::design::accent()
                } else {
                    crate::design::text2()
                }
            },
        ));
    }
    return content.child(
        text(move || label())
            .text_size(px(crate::ui::BODY))
            .line_height(px(crate::ui::BODY_LINE))
            .text_color(move || {
                if active_for_text() {
                    crate::design::accent()
                } else {
                    crate::design::text2()
                }
            }),
    );
}

/// 图标版「点一下就弹菜单」的按钮：行尾的「更多」用它。
///
/// 与 [`menu_button`] 的差别是外观与锚点：22×22 的图标按钮，`label` 只作无障碍
/// 名称；菜单在指针处弹出（`nsh_base::button` 的按压事件不带坐标）。
pub(crate) fn menu_icon_button(
    name: &'static str,
    label: impl Fn() -> String + 'static,
    build_items: impl Fn() -> Vec<MenuItem> + 'static,
) -> impl IntoElement {
    let build_items = std::rc::Rc::new(build_items);
    let a11y = label();
    let size = crate::ui::CONTROL_SMALL;
    return nsh_base::button()
        .w(px(size))
        .h(px(size))
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .rounded(px(6.0))
        .accessibility_label(Some(SharedString::from(a11y)))
        .bg(AdaptiveColor::TRANSPARENT)
        .hover(|style| style.bg(move || crate::design::hover()))
        .active(|style| style.bg(move || crate::design::line_soft()))
        // `nsh_base::button` 的按压不带坐标，这里用不带锚点的 `popup_menu`：
        // 菜单在指针处弹出，指针就落在这个按钮上。
        .on_press(move |_: &()| {
            let items = build_items();
            let _ = nsh_core::element::desktop::popup_menu(items);
        })
        .child(crate::ui::icon::icon_view(
            name,
            crate::ui::icon::IconSize::Button,
            || crate::design::text2(),
        ));
}

/// 一级菜单标题（菜单栏用，与右键菜单无关）。
pub(crate) fn section(label: impl Into<String>) -> MenuItem {
    return MenuItem::new(label);
}
