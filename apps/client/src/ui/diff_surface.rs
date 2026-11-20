//! diff 视图：把 `nsh_base::DiffView` 收成应用统一的入口。
//!
//! 行为（分栏/统一、折叠展开、软换行、行号、指示条、hunk 导航）由 `DiffView`
//! 提供；本层固定外观，并把只读历史 hunk 的复制动作接到应用剪贴板。

use crate::ui;
use nsh_base::{DiffHunk, DiffIndicators, DiffLayout, DiffViewApi, diff_view};
use nsh_core::prelude::*;

/// 当前 DiffView 的导航入口。历史差异只读，不显示变更暂存按钮。
#[derive(Clone)]
pub(crate) struct DiffHandle {
    reference: ComponentRef<nsh_base::DiffView>,
}

impl DiffHandle {
    pub(crate) fn new() -> Self {
        return Self {
            reference: component_ref(),
        };
    }

    pub(crate) fn next_hunk(&self) -> Option<u32> {
        return self
            .reference
            .api::<DiffViewApi>()
            .and_then(|api| api.next_hunk());
    }

    pub(crate) fn previous_hunk(&self) -> Option<u32> {
        return self
            .reference
            .api::<DiffViewApi>()
            .and_then(|api| api.previous_hunk());
    }
}

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
/// `expand_all` 控制未改动上下文是否全部展开；语言由 patch 的文件路径逐文件识别。
pub(crate) fn diff_surface(
    patch: SharedString,
    layout: Layout,
    wrap: bool,
    expand_all: bool,
    handle: &DiffHandle,
    language: Signal<SharedString>,
    on_hunk_copy: std::sync::Arc<dyn Fn(&DiffHunk)>,
) -> impl IntoElement {
    let layout_name = match layout {
        Layout::Split => "split",
        Layout::Unified => "unified",
    };
    let patch_text = patch.as_str();
    log::debug!(
        target: "gitui::diff",
        "event=diff_surface.mount bytes={} files={} hunks={} layout={} wrap={} expand_all={}",
        patch_text.len(),
        patch_text.lines().filter(|line| line.starts_with("diff --git ")).count(),
        patch_text.lines().filter(|line| line.starts_with("@@")).count(),
        layout_name,
        wrap,
        expand_all
    );
    let layout = match layout {
        Layout::Split => DiffLayout::Split,
        Layout::Unified => DiffLayout::Unified,
    };
    let on_hunk_copy = std::rc::Rc::new(on_hunk_copy);
    return diff_view()
        .component_ref(&handle.reference)
        .patch(Some(patch))
        .layout(layout)
        .soft_wrap(wrap)
        .expand_all(expand_all)
        .line_numbers(true)
        .background(false)
        .font_family(ui::MONO)
        .font_size(Points(ui::CODE))
        .line_height(Points(ui::CODE_LINE))
        .indicators(DiffIndicators::Classic)
        .render_hunk_actions(move |hunk: &DiffHunk| -> AnyElement {
            let handler = on_hunk_copy.clone();
            let hunk = hunk.clone();
            return crate::ui::button::button(
                {
                    let language = language;
                    move || crate::i18n::t(language.get().as_str(), "diff.copyHunk")
                },
                crate::ui::button::ButtonVariant::Ghost,
                crate::ui::button::ButtonSize::Small,
                false,
                move || handler(&hunk),
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
