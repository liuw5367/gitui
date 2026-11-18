//! 应用自己的 ui 层：`nsh_base` 提供行为、焦点与无障碍，这里只提供外观。
//!
//! 规则（由 `tools/check-ui.sh` 校验）：
//! - 页面与 workspace 只组合本模块的组件，不直接调用 `nsh_base`；
//! - 不在页面里写 `text_size(px(..))` 字面量，字号取这里的常量；
//! - 颜色只来自 `crate::design`，本层不写十六进制颜色。
//!
//! 尺寸口径是 macOS 原生刻度（见 `docs/design/TOKENS.md` 的「尺寸不是颜色 token」一节），
//! 与设计稿的 9–10.5px 字号不同：设计稿决定结构与层级，系统刻度决定字号与控件高度。

use crate::design;
use crate::design::metrics::native;
use nsh_core::prelude::*;

pub(crate) mod badge;
pub(crate) mod button;
pub(crate) mod check;
pub(crate) mod dialog;
pub(crate) mod diff_surface;
pub(crate) mod feedback;
pub(crate) mod field;
pub(crate) mod gallery;
pub(crate) mod icon;
pub(crate) mod key_value;
pub(crate) mod menu;
pub(crate) mod row;
pub(crate) mod section;
pub(crate) mod segmented;
pub(crate) mod select;
pub(crate) mod setting_row;
pub(crate) mod split;
pub(crate) mod table;
pub(crate) mod tabs;
pub(crate) mod text_input;
pub(crate) mod tree;

// ---- 字号 ----

/// 正文与标签。
pub(crate) const BODY: f32 = native::FONT_BODY;
/// 正文行高。
pub(crate) const BODY_LINE: f32 = native::LINE_BODY;
/// 说明、帮助与表单提示。
pub(crate) const SECONDARY: f32 = native::FONT_SECONDARY;
/// 次要文字行高。
pub(crate) const SECONDARY_LINE: f32 = native::LINE_SECONDARY;
/// 元数据与表头（不再低于 11）。
pub(crate) const META: f32 = native::FONT_META;
/// 代码与 diff 正文。
pub(crate) const CODE: f32 = native::FONT_CODE;
/// 代码行高。
pub(crate) const CODE_LINE: f32 = native::LINE_CODE;
/// 页面主标题（欢迎页英雄区）。
pub(crate) const TITLE_LARGE: f32 = 26.0;
/// 产品名等中型标题（About 的 20/700）。
pub(crate) const TITLE_MEDIUM: f32 = 20.0;
/// 中型标题行高。
pub(crate) const TITLE_MEDIUM_LINE: f32 = 24.0;
/// 设置页等表单页的主标题（画板 `.settings-head` 17/700）。
pub(crate) const PAGE_TITLE: f32 = 17.0;
/// 表单页主标题行高。
pub(crate) const PAGE_TITLE_LINE: f32 = 22.0;
/// 面板、侧栏与设置页分段标题。
pub(crate) const TITLE: f32 = 15.0;
/// 面板与设置页分段标题行高。
pub(crate) const TITLE_LINE: f32 = 20.0;
/// 等宽字体族。
pub(crate) const MONO: &str = "SF Mono";

// ---- 控件与区域高度 ----

/// 小控件（工具栏紧凑按钮、行内按钮）。
pub(crate) const CONTROL_SMALL: f32 = native::CONTROL_SMALL;
/// 常规控件（按钮、输入框、下拉）。
pub(crate) const CONTROL: f32 = native::CONTROL;
/// 大控件（主按钮）。
pub(crate) const CONTROL_LARGE: f32 = native::CONTROL_LARGE;
/// 标准行高（列表、表格、侧栏）。
pub(crate) const ROW: f32 = native::ROW;
/// 紧凑行高（菜单项、快捷键行）。
pub(crate) const ROW_COMPACT: f32 = native::ROW_COMPACT;
/// 工具栏。
pub(crate) const TOOLBAR: f32 = native::TOOLBAR;
/// 标题栏（含仓库页签，系统窗口按钮区域不重绘）。
pub(crate) const TITLEBAR: f32 = 44.0;
/// 状态栏。
pub(crate) const STATUS: f32 = native::STATUS;

// ---- 间距 ----

pub(crate) const GAP_XS: f32 = 4.0;
pub(crate) const GAP_SM: f32 = 6.0;
pub(crate) const GAP: f32 = 8.0;
pub(crate) const GAP_LG: f32 = 12.0;
pub(crate) const GAP_XL: f32 = 16.0;

/// 正文文字：13pt、正文色。
pub(crate) fn body(label: impl Fn() -> String + 'static) -> impl IntoElement {
    return text(move || label())
        .text_size(px(BODY))
        .line_height(px(BODY_LINE))
        .text_color(move || design::text());
}

/// 次要文字：11pt、次要色。
pub(crate) fn secondary(label: impl Fn() -> String + 'static) -> impl IntoElement {
    return text(move || label())
        .text_size(px(SECONDARY))
        .line_height(px(SECONDARY_LINE))
        .text_color(move || design::text2());
}

/// 元数据：11pt、三级色。
pub(crate) fn meta(label: impl Fn() -> String + 'static) -> impl IntoElement {
    return text(move || label())
        .text_size(px(META))
        .line_height(px(SECONDARY_LINE))
        .text_color(move || design::text3());
}

/// 等宽代码文字：12pt、18 行高。
pub(crate) fn code(label: impl Fn() -> String + 'static) -> impl IntoElement {
    return text(move || label())
        .text_size(px(CODE))
        .line_height(px(CODE_LINE))
        .font_family(MONO)
        .text_color(move || design::text());
}

/// 一条竖直分隔线，用于横向排布的工具栏与分段之间。
pub(crate) fn v_separator() -> impl IntoElement {
    return view()
        .w(px(1.0))
        .h(px(16.0))
        .flex_shrink_0()
        .bg(move || design::line());
}

/// 一条水平分隔线。
pub(crate) fn separator() -> impl IntoElement {
    return view()
        .w_full()
        .h(px(1.0))
        .flex_shrink_0()
        .bg(move || design::line());
}

/// 主题焦点 outline 的颜色：与聚焦时的描边同一种蓝（`design::accent()` 读的 `blue`
/// token）。
///
/// `theme.json` 里的 `focus` 不是颜色，而是一条 box-shadow 效果值（见
/// `docs/design/TOKENS.md` 的「效果值不是颜色」一节）：`theme.color("focus")` 解析不出
/// 颜色，只会退回前景色，焦点环于是被画成了近黑色。焦点环需要的是可描边的颜色，
/// 所以取 `blue`，与输入框、按钮聚焦时的边框一致。
pub(crate) fn focus_color() -> AdaptiveColor {
    return design::accent();
}

/// 焦点环的宽度。
pub(crate) const FOCUS_WIDTH: f32 = 2.0;

#[cfg(test)]
mod guard_tests {
    //! ui 层的机械约束：字号只取刻度常量，颜色只来自 `design`。
    //!
    //! 与 `tools/check-ui.sh` 同源：脚本用于本地与 CI 的前置检查，这里保证 `cargo test`
    //! 也能发现问题。已经迁到 ui 层的页面在 `MIGRATED` 里登记，逐个阶段追加，P8 时应
    //! 覆盖全部页面文件。

    use std::fs;
    use std::path::{Path, PathBuf};

    /// 焦点环不能再去读 `focus` 这条 effect token。
    ///
    /// 回归点：`theme.json` 的 `focus` 是 box-shadow 效果值、不是颜色，`token("focus")`
    /// 解析失败后会**静默**退回前景色——焦点环因此被画成近黑色，聚焦与否几乎看不出
    /// 区别。现在取 `design::accent()`（强调蓝），与输入框、按钮聚焦时的边框一致；
    /// 这条机械断言防止有人把它改回去。属于源码扫描，因为主题 token 需要完整运行时，
    /// 单元测试里装不进去。
    #[test]
    fn focus_ring_does_not_read_the_focus_effect_token() {
        let source = read(&src_root().join("ui/mod.rs"));
        let start = source
            .find("pub(crate) fn focus_color()")
            .expect("ui/mod.rs 里应有 focus_color()");
        let body_start = start + source[start..].find('{').expect("函数体");
        // 函数体以顶格 `}` 结束（`\n` + `}`），用它确定边界。
        let body_end = body_start + source[body_start..].find("\n}").expect("函数体结束");
        let body = &source[body_start..body_end];
        assert!(
            !body.contains("token(\"focus\")"),
            "focus_color() 又去读 effect token `focus` 了：它不是颜色，会退回前景色"
        );
        assert!(
            body.contains("accent()"),
            "focus_color() 应取 design::accent()（强调蓝）"
        );
    }

    /// 关闭的全屏冲突层必须退出命中测试，否则它会挡住整个工作区的按钮和输入框。
    #[test]
    fn closed_conflict_layer_allows_pointer_events_through() {
        let source = read(&src_root().join("features/merge/view.rs"));
        let start = source
            .find("pub(crate) fn conflict_view")
            .expect("merge/view.rs 里应有 conflict_view()");
        let end = source[start..]
            .find(".items_center()")
            .map(|offset| start + offset)
            .expect("冲突层布局");
        let layer = &source[start..end];
        assert!(
            layer.contains(".pointer_events(move ||"),
            "冲突层需要按打开状态切换命中测试"
        );
        assert!(
            layer.contains("PointerEvents::Auto") && layer.contains("PointerEvents::None"),
            "冲突层打开时接收输入，关闭时必须穿透输入"
        );
    }

    /// 组件画廊应使用独立窗口，不得作为主窗口上的全屏遮罩。
    #[test]
    fn gallery_uses_an_independent_window() {
        let source = read(&src_root().join("ui/gallery.rs"));
        let start = source
            .find("pub(crate) fn gallery")
            .expect("ui/gallery.rs 里应有 gallery()");
        let end = source[start..]
            .find("fn body(")
            .map(|offset| start + offset)
            .expect("画廊 body()");
        let gallery = &source[start..end];
        assert!(
            gallery.contains("desktop::window()") && gallery.contains(".open(open)"),
            "画廊应通过独立的 desktop window 显隐"
        );
        assert!(
            !gallery.contains(".absolute()"),
            "画廊不应在主窗口中创建全屏遮罩"
        );
    }

    /// 已迁移的页面文件（相对 `src/`）。P0 只有 ui 层自己。
    const MIGRATED: &[&str] = &[
        "features/welcome/view.rs",
        "features/settings/view.rs",
        "features/settings/general.rs",
        "features/settings/git.rs",
        "features/settings/shortcuts.rs",
        "features/settings/about.rs",
        "features/settings/agent.rs",
        "features/settings/credentials.rs",
        "features/refs/view.rs",
        "features/commit/view.rs",
        "features/agent/view.rs",
        "features/merge/view.rs",
        "features/sync/view.rs",
        "features/search/form_dialog.rs",
        "features/commit/hook_sheet.rs",
        "features/commit/rewrite_dialog.rs",
        "features/history/inspector.rs",
        "features/changes/view.rs",
        "features/history/view.rs",
        "features/search/view.rs",
        "features/history/blame.rs",
        "app/workspace.rs",
    ];

    /// 尚未完成页面层迁移、但已禁止直接调用 NSH 基础组件的交互文件。
    const BASE_ONLY: &[&str] = &[
        "features/sync/commands.rs",
        "features/refs/commands.rs",
        "interactions.rs",
        "shell/mod.rs",
    ];

    fn src_root() -> PathBuf {
        return PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    }

    fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                rust_files(&path, out);
            } else if path.extension().map(|ext| ext == "rs").unwrap_or(false) {
                out.push(path);
            }
        }
    }

    /// 找出 `text_size(px(<数字>` 形式的字号字面量。
    fn has_text_size_literal(source: &str) -> bool {
        let needle = "text_size(px(";
        let mut rest = source;
        while let Some(index) = rest.find(needle) {
            let after = &rest[index + needle.len()..];
            if after
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
            {
                return true;
            }
            rest = after;
        }
        return false;
    }

    /// 找出十六进制颜色（`design` 之外的模块不得自带色板）。
    fn has_hex_color(source: &str) -> bool {
        let bytes = source.as_bytes();
        let mut index = 0;
        while index + 3 < bytes.len() {
            if bytes[index] == b'#' {
                let hex = &bytes[index + 1..(index + 7).min(bytes.len())];
                if hex.len() >= 3 && hex.iter().all(|byte| byte.is_ascii_hexdigit()) {
                    return true;
                }
            }
            index += 1;
        }
        return false;
    }

    /// 已迁移文件不能导入或完全限定调用 nsh_base。
    fn has_nsh_base_reference(source: &str) -> bool {
        return source.lines().any(|line| {
            let code = line.split("//").next().unwrap_or_default();
            code.contains("nsh_base::")
        });
    }

    fn read(path: &Path) -> String {
        return fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("读取 {} 失败：{error}", path.display()));
    }

    #[test]
    fn ui_source_uses_scale_constants_and_tokens() {
        let mut files = Vec::new();
        rust_files(&src_root().join("ui"), &mut files);
        assert!(!files.is_empty(), "没找到 ui 源文件");
        for path in files {
            let source = read(&path);
            let name = path.display();
            assert!(
                !has_text_size_literal(&source),
                "{name} 使用了字号字面量，改用 ui::BODY / ui::SECONDARY / ui::META / ui::CODE"
            );
            assert!(
                !has_hex_color(&source),
                "{name} 出现了十六进制颜色，颜色只能取自 crate::design"
            );
        }
    }

    #[test]
    fn migrated_files_do_not_reach_base_or_write_literals() {
        for relative in MIGRATED {
            let path = src_root().join(relative);
            assert!(path.exists(), "MIGRATED 里的 {relative} 不存在");
            let source = read(&path);
            assert!(
                !has_nsh_base_reference(&source),
                "{relative} 直接引用或调用了 nsh_base，改用 crate::ui"
            );
            assert!(
                !source.contains(".on_press("),
                "{relative} 里还有裸 on_press，行级按压应改用 ui::row 或 ui 按钮"
            );
            assert!(
                !has_text_size_literal(&source),
                "{relative} 仍使用字号字面量"
            );
            assert!(!has_hex_color(&source), "{relative} 仍自带十六进制颜色");
        }
    }

    #[test]
    fn base_only_files_do_not_reach_nsh_base() {
        for relative in BASE_ONLY {
            let path = src_root().join(relative);
            assert!(path.exists(), "BASE_ONLY 里的 {relative} 不存在");
            let source = read(&path);
            assert!(
                !has_nsh_base_reference(&source),
                "{relative} 直接引用或调用了 nsh_base，改用 crate::ui"
            );
        }
    }
}
