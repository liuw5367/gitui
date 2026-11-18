//! 分栏：可拖拽调整尺寸的面板组，用于检查器与 diff 之间、冲突页的三栏。
//!
//! 行为来自 `nsh_base` 的 `h_resizable`/`v_resizable`（拖拽、键盘、最小尺寸、
//! 把手焦点与可见状态都由它提供）。这里只把「每栏一个元素 + 尺寸约束」收成一个
//! 数组参数，并保证把手指示线在深浅色下都可见。
//!
//! 用法：
//! ```ignore
//! ui::split::horizontal(vec![
//!     ui::split::Pane::fixed(body, 232.0).min(180.0),
//!     ui::split::Pane::flex(history),
//! ])
//! ```

use nsh_base::{h_resizable, resizable_panel, v_resizable};
use nsh_core::prelude::*;

/// 一栏。
pub(crate) struct Pane {
    content: AnyElement,
    /// 初始尺寸；`None` 表示按剩余空间分配。
    size: Option<f32>,
    min: f32,
    max: Option<f32>,
    visible: Option<std::rc::Rc<dyn Fn() -> bool>>,
}

impl Pane {
    /// 固定初始尺寸的栏（侧栏、检查器）。
    pub(crate) fn fixed(content: impl IntoElement, size: f32) -> Pane {
        return Pane {
            content: content.into_element(),
            size: Some(size),
            min: 120.0,
            max: None,
            visible: None,
        };
    }

    /// 按剩余空间分配的栏（主区）。
    pub(crate) fn flex(content: impl IntoElement) -> Pane {
        return Pane {
            content: content.into_element(),
            size: None,
            min: 200.0,
            max: None,
            visible: None,
        };
    }

    /// 最小尺寸。
    pub(crate) fn min(mut self, min: f32) -> Pane {
        self.min = min;
        return self;
    }

    /// 最大尺寸。
    pub(crate) fn max(mut self, max: f32) -> Pane {
        self.max = Some(max);
        return self;
    }

    /// 由工具栏信号控制显隐；隐藏时 NSH 会记住该栏的宽度。
    pub(crate) fn visible(mut self, visible: impl Fn() -> bool + 'static) -> Pane {
        self.visible = Some(std::rc::Rc::new(visible));
        return self;
    }
}

/// 左右分栏。
pub(crate) fn horizontal(panes: Vec<Pane>) -> impl IntoElement {
    return group(h_resizable(), panes);
}

/// 上下分栏。
pub(crate) fn vertical(panes: Vec<Pane>) -> impl IntoElement {
    return group(v_resizable(), panes);
}

/// 把手与容器共用的外观：把手用 1pt 分隔线色，不额外加粗。
fn group(mut builder: nsh_base::ResizablePanelGroupElement, panes: Vec<Pane>) -> impl IntoElement {
    builder = builder
        .w_full()
        .h_full()
        .min_h_0()
        // 命中区仍由 NSH 保持 9pt，实际可见指示线固定为 1pt。
        .handle_class("bg-border");
    for pane in panes {
        let mut panel = resizable_panel()
            .size(pane.size.map(Points))
            .min_size(Points(pane.min))
            .child(pane.content);
        if let Some(max) = pane.max {
            panel = panel.max_size(Some(Points(max)));
        }
        if let Some(visible) = pane.visible {
            panel = panel.visible(move || visible());
        }
        builder = builder.child(panel);
    }
    return builder;
}
