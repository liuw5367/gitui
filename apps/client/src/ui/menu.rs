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
//! nsh_core::element::desktop::popup_menu_at(items, event.page_position);
//! ```

use nsh_core::element::desktop::MenuItem;
use nsh_core::prelude::*;

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
    let build_items = std::rc::Rc::new(build_items);
    let mut content = view()
        .h(px(crate::ui::CONTROL_SMALL))
        .px(px(crate::ui::GAP))
        .flex_row()
        .items_center()
        .gap(px(crate::ui::GAP_SM))
        .rounded(px(6.0))
        .hover(|style| style.bg(move || crate::design::hover()))
        .on_press(move |event: &PressEvent| {
            let items = build_items();
            let _ = nsh_core::element::desktop::popup_menu_at(items, event.page_position);
        });
    if let Some(name) = icon {
        content = content.child(crate::ui::icon::inline_icon(name));
    }
    return content.child(
        text(move || label())
            .text_size(px(crate::ui::BODY))
            .line_height(px(crate::ui::BODY_LINE))
            .text_color(move || crate::design::text()),
    );
}

/// 一级菜单标题（菜单栏用，与右键菜单无关）。
pub(crate) fn section(label: impl Into<String>) -> MenuItem {
    return MenuItem::new(label);
}
