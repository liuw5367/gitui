//! 图标：设计稿的 SVG sprite，尺寸与颜色由调用方通过枚举给定。
//!
//! nsh 0.5.0 没有 SF Symbols 的接口（只有 iOS 导航栏的 header symbol），因此图标
//! 沿用设计稿的路径数据。尺寸只允许三档，避免每处各写一个数字。

use crate::design::{self, icons};
use nsh_core::prelude::*;

/// 图标尺寸三档：按钮内 12、行内 13、工具栏与页头 16。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IconSize {
    Button,
    Inline,
    Toolbar,
}

impl IconSize {
    pub(crate) fn points(self) -> f32 {
        return match self {
            IconSize::Button => 12.0,
            IconSize::Inline => 13.0,
            IconSize::Toolbar => 16.0,
        };
    }
}

/// 悬停与默认两种前景色，与 `view()` 的悬停行为一起使用时只改颜色。
pub(crate) type IconColor = fn() -> AdaptiveColor;

/// 纯图标：不可点击，用于行内标记（分支、标签、文件、状态）。
pub(crate) fn icon_view(
    name: &'static str,
    size: IconSize,
    color: impl Fn() -> AdaptiveColor + 'static,
) -> impl IntoElement {
    let points = size.points();
    let mut holder = view().w(px(points)).h(px(points)).flex_shrink_0();
    if let Some(source) = icons::icon(name) {
        holder = holder.child(
            svg(source)
                .w(px(points))
                .h(px(points))
                .text_color(move || color()),
        );
    }
    return holder;
}

/// 品牌标记：欢迎页的应用图标，按传入点数绘制。
///
/// 这是唯一允许自定义点数的图标：它属于品牌元素，不属于界面图标体系（12/13/16）。
pub(crate) fn brand_mark(
    name: &'static str,
    points: f32,
    color: impl Fn() -> AdaptiveColor + 'static,
) -> impl IntoElement {
    let mut holder = view().w(px(points)).h(px(points)).flex_shrink_0();
    if let Some(source) = icons::icon(name) {
        holder = holder.child(
            svg(source)
                .w(px(points))
                .h(px(points))
                .text_color(move || color()),
        );
    }
    return holder;
}

/// 行内默认色图标（三级文字色）。
pub(crate) fn inline_icon(name: &'static str) -> impl IntoElement {
    return icon_view(name, IconSize::Inline, design::text3);
}

/// 工具栏默认色图标。
pub(crate) fn toolbar_icon(name: &'static str) -> impl IntoElement {
    return icon_view(name, IconSize::Toolbar, design::text2);
}
