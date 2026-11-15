//! diff 视图：把 `nsh_base::DiffView` 收成应用统一的入口。
//!
//! 行为（分栏/统一、折叠展开、软换行、行号、指示条、hunk 操作、行内批注）全部
//! 由 `DiffView` 提供；本层只固定外观：等宽 12pt、行高 18、行号开启、指示条走
//! 经典 +/−（与设计板 04/05 一致），并把「hunk 上的动作」暴露成回调。

use crate::ui;
use nsh_base::{DiffHunk, DiffIndicators, DiffLayout, diff_view};
use nsh_core::prelude::*;

/// 两种视图。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Layout {
    /// 左右分栏。
    Split,
    /// 统一纵向。
    Unified,
}

/// 一块 diff。`patch` 是 unified diff 文本；`wrap` 为真时软换行。
///
/// `on_hunk_action` 给出时，每个 hunk 上会有一个「暂存此块」按钮。
pub(crate) fn diff_surface(
    patch: SharedString,
    layout: Layout,
    wrap: bool,
    on_hunk_action: Option<Box<dyn Fn(&DiffHunk)>>,
) -> impl IntoElement {
    let handler = on_hunk_action.map(std::rc::Rc::new);
    let layout = match layout {
        Layout::Split => DiffLayout::Split,
        Layout::Unified => DiffLayout::Unified,
    };
    return diff_view()
        .patch(Some(patch))
        .layout(layout)
        .soft_wrap(wrap)
        .line_numbers(true)
        .background(false)
        .font_family(ui::MONO)
        .font_size(Points(ui::CODE))
        .line_height(Points(ui::CODE_LINE))
        .indicators(DiffIndicators::Classic)
        .render_hunk_actions(move |hunk: &DiffHunk| -> AnyElement {
            let handler = handler.clone();
            let hunk = hunk.clone();
            let disabled = handler.is_none();
            return crate::ui::button::button(
                move || "Stage hunk".to_string(),
                crate::ui::button::ButtonVariant::Ghost,
                crate::ui::button::ButtonSize::Small,
                disabled,
                move || {
                    if let Some(handler) = &handler {
                        handler(&hunk);
                    }
                },
            )
            .into_element();
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_values_are_distinct() {
        // 两个视图用于「Unified / Split」切换，不能等价。
        assert_ne!(Layout::Split, Layout::Unified);
    }
}
