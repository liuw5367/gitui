//! 应用自己的 ui 层：`nsh_base` 提供行为、焦点与无障碍，这里只提供外观。
//!
//! 规则（由 `tools/check-ui.sh` 校验）：
//! - 页面与 workspace 只组合本模块的组件，不直接调用 `nsh_base`；
//! - 不在页面里写 `text_size(px(..))` 字面量，字号取这里的常量；
//! - 颜色只来自 `crate::design`，本层不写十六进制颜色。
//!
//! 尺寸口径是 macOS 原生刻度（见 `docs/DESIGN-HANDOFF.md` 的「原生刻度」一节），
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
pub(crate) mod setting_row;
pub(crate) mod split;
pub(crate) mod segmented;
pub(crate) mod tabs;
pub(crate) mod table;
pub(crate) mod tree;
pub(crate) mod text_input;


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
/// 面板与侧栏标题。
pub(crate) const TITLE: f32 = 15.0;
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

/// 一条水平分隔线。
pub(crate) fn separator() -> impl IntoElement {
    return view()
        .w_full()
        .h(px(1.0))
        .flex_shrink_0()
        .bg(move || design::line());
}

/// 键盘焦点环的颜色。用 `outline` 而不是 `border`，避免改变布局尺寸。
pub(crate) fn focus_color() -> AdaptiveColor {
    return design::token("focus");
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

    /// 已迁移的页面文件（相对 `src/`）。P0 只有 ui 层自己。
    const MIGRATED: &[&str] = &[
        "screens/welcome.rs",
        "screens/settings.rs",
        "screens/refs_view.rs",
        "workspace/commit.rs",
        "workspace/agent.rs",
        "screens/conflict_view.rs",
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
            if after.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
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
    fn migrated_pages_do_not_reach_base_or_write_literals() {
        for relative in MIGRATED {
            let path = src_root().join(relative);
            assert!(path.exists(), "MIGRATED 里的 {relative} 不存在");
            let source = read(&path);
            assert!(
                !source.contains("use nsh_base::"),
                "{relative} 直接引用了 nsh_base，改用 crate::ui"
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
}
