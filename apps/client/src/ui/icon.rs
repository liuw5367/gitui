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
    // 图标名写错时以前会静默渲染空占位（画廊里「Disabled icon」就是这么丢的）。
    // 调试构建下直接报出来，发布构建仍保持静默以免崩溃。
    debug_assert!(
        icons::icon(name).is_some(),
        "图标名不存在：{name}（可用名字见 design/icons.rs）"
    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// 扫描 `src/` 里所有 `icon_view("name"` / `icon_button("name"` / `icon("name"`
    /// 的字面量，确认图标表里都有。图标名写错时，以前只会渲染空占位。
    #[test]
    fn every_icon_literal_exists_in_the_sprite() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut names: Vec<String> = Vec::new();
        let mut stack = vec![root];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = fs::read_dir(&dir) else { continue };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                    let Ok(source) = fs::read_to_string(&path) else { continue };
                    for marker in ["icon_view(\"", "icon_button(\"", "icon(\""] {
                        let mut rest = source.as_str();
                        while let Some(index) = rest.find(marker) {
                            let after = &rest[index + marker.len()..];
                            if let Some(end) = after.find('"') {
                                names.push(after[..end].to_string());
                            }
                            rest = &after[..];
                            if rest.is_empty() { break; }
                            rest = &rest[1..];
                        }
                    }
                }
            }
        }
        assert!(!names.is_empty(), "没扫到任何图标字面量");
        // `icon_view(name, ..)` 这类调用里的 `name` 是参数名而不是图标名，跳过。
        names.retain(|name| name != "name");
        let missing: Vec<&String> = names
            .iter()
            .filter(|name| icons::icon(name.as_str()).is_none())
            .collect();
        assert!(missing.is_empty(), "这些图标名不在 sprite 里：{missing:?}");
    }
}
