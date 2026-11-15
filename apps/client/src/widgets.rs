//! 尺寸自由的图标：`ui::icon::icon_view` 只认 11/12/13/16 四档，而画板里还有
//! 9/10/15/20 这些点数（小指示勾、折叠箭头、错误横幅图标）。这个文件保留
//! 「任意点数」的入口，供那些位置使用。
//!
//! 其余控件都已并入 `ui/`：按钮（`ui::button`）、复选（`ui::check`）、
//! 键帽（`ui::badge::kbd`）、图标按钮（`ui::button::icon_button`）。
//!
//! 尺寸与配色来自 `design::metrics` 与 `design` 的 token。

use crate::design::icons;
use nsh_core::prelude::*;

/// 纯图标，不可点击：用于行内标记（分支、标签、文件）。
pub(crate) fn icon(
    name: &'static str,
    size: f32,
    color: impl Fn() -> AdaptiveColor + 'static,
) -> impl IntoElement {
    let mut holder = view().w(px(size)).h(px(size)).flex_shrink_0();
    if let Some(source) = icons::icon(name) {
        holder = holder.child(
            svg(source)
                .w(px(size))
                .h(px(size))
                .text_color(move || color()),
        );
    }
    return holder;
}

