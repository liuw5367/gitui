//! 尺寸自由的图标：`ui::icon::icon_view` 只认 11/12/13/16 四档，而这里还剩 3 处
//! 用别的点数——9pt 的表格折叠箭头、15pt 的设置侧栏分支图标、20pt 的错误横幅
//! 图标。
//!
//! 这三个不是「忘了统一」：画板设置页的图标本身就是 14/15，行内折叠箭头是 9，
//! 错误横幅是 20。强行映射到四档等于改设计。详见 DESIGN-HANDOFF §12 第 22 条。
//!
//! 其余控件都已并入 `ui/`：按钮（`ui::button`）、复选（`ui::check`）、
//! 键帽（`ui::badge::kbd`）、图标按钮（`ui::button::icon_button`）。
//!
//! 配色取自 `design` 的 token（颜色由调用方给定，这里不绑定语义）。

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

