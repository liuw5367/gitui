//! 设计层：颜色 token、几何常量与图标。
//!
//! 颜色来自 `theme.json`（与设计稿 CSS 的 `:root` / `.theme-dark` 同值），几何来自
//! 共享样式表与各设计板的内联尺寸。界面只通过本模块取这两类值，避免散落的字面量。

pub mod icons;
pub mod metrics;

use nsh_base::theme;
use nsh_core::prelude::*;

/// 取一个设计 token；未定义时退回前景色，便于早发现拼写错误。
pub(crate) fn token(name: &'static str) -> AdaptiveColor {
    let theme = theme();
    return theme.color(name).unwrap_or(theme.foreground);
}

// 表面
pub(crate) fn content() -> AdaptiveColor {
    return token("content");
}

pub(crate) fn panel() -> AdaptiveColor {
    return token("panel");
}

pub(crate) fn panel2() -> AdaptiveColor {
    return token("panel2");
}

pub(crate) fn sidebar() -> AdaptiveColor {
    return token("sidebar");
}

pub(crate) fn bar() -> AdaptiveColor {
    return token("bar");
}

pub(crate) fn titlebar() -> AdaptiveColor {
    return token("titlebar");
}

pub(crate) fn status() -> AdaptiveColor {
    return token("status");
}

pub(crate) fn field() -> AdaptiveColor {
    return token("field");
}

// 文本
pub(crate) fn text() -> AdaptiveColor {
    return token("text");
}

pub(crate) fn text2() -> AdaptiveColor {
    return token("text2");
}

pub(crate) fn text3() -> AdaptiveColor {
    return token("text3");
}

// 线条
pub(crate) fn line() -> AdaptiveColor {
    return token("line");
}

pub(crate) fn line_soft() -> AdaptiveColor {
    return token("line-soft");
}

pub(crate) fn line_strong() -> AdaptiveColor {
    return token("line-strong");
}

// 强调与语义色
/// 设计稿里的 `--accent`（链接与图标蓝），与 `primary`（主按钮底色）不同。
pub(crate) fn accent() -> AdaptiveColor {
    return token("blue");
}

pub(crate) fn accent2() -> AdaptiveColor {
    return token("accent-2");
}

pub(crate) fn accent_soft() -> AdaptiveColor {
    return token("accent-soft");
}

pub(crate) fn selected() -> AdaptiveColor {
    return token("sel");
}

pub(crate) fn hover() -> AdaptiveColor {
    return token("hover");
}

pub(crate) fn branch_bg() -> AdaptiveColor {
    return token("branch-bg");
}

pub(crate) fn branch_fg() -> AdaptiveColor {
    return token("branch-fg");
}

pub(crate) fn tag_bg() -> AdaptiveColor {
    return token("tag-bg");
}

pub(crate) fn tag_fg() -> AdaptiveColor {
    return token("tag-fg");
}

pub(crate) fn remote_fg() -> AdaptiveColor {
    return token("remote-fg");
}

pub(crate) fn add_bg() -> AdaptiveColor {
    return token("add-bg");
}

pub(crate) fn add_fg() -> AdaptiveColor {
    return token("add-fg");
}

pub(crate) fn del_bg() -> AdaptiveColor {
    return token("del-bg");
}

pub(crate) fn del_fg() -> AdaptiveColor {
    return token("del-fg");
}

pub(crate) fn warn() -> AdaptiveColor {
    return token("warn");
}

pub(crate) fn danger() -> AdaptiveColor {
    return token("danger");
}

pub(crate) fn ok() -> AdaptiveColor {
    return token("ok");
}

pub(crate) fn field_border() -> AdaptiveColor {
    return token("field-border");
}

pub(crate) fn btn() -> AdaptiveColor {
    return token("btn");
}

pub(crate) fn btn_border() -> AdaptiveColor {
    return token("btn-border");
}

#[cfg(test)]
mod theme_tests {
    //! `theme.json` 与设计稿共享样式表的颜色变量必须一致。
    //!
    //! 这是「设计稿 → 实现」链路上最后一段没有测试覆盖的地方：几何有
    //! `geometry_tests` 锁着，颜色此前全靠人工抄写。两边任一侧改色都会在这里失败。
    //!
    //! 三处**语义不同**的键在下面显式排除，而不是让测试变松：
    //! - `accent`：设计里 `--accent` 是主色（#007AFF），我们用它存选中底色，
    //!   主色放在 `blue` 键（`design::accent()` 读的就是它）。
    //! - `desk1` / `desk2`：桌面渐变，按 DESIGN-HANDOFF 不属于窗口内容，未移植。

    use std::collections::BTreeMap;
    use std::path::PathBuf;

    const THEME: &str = include_str!("../../theme.json");

    /// 语义与设计不同、因此不参与比对的键。
    const SEMANTIC: [&str; 3] = ["accent", "desk1", "desk2"];

    fn design_root() -> PathBuf {
        return PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../designs/graft-git");
    }

    /// 取出某个选择器块里的 `--x: value` 变量。
    fn variables(css: &str, selector: &str) -> BTreeMap<String, String> {
        let needle = format!("{selector}{{");
        let start = css
            .find(needle.as_str())
            .unwrap_or_else(|| panic!("设计 CSS 里找不到 {selector} 块（路径或格式变了？）"));
        let body_start = start + needle.len();
        let body_end = css[body_start..]
            .find('}')
            .map(|offset| body_start + offset)
            .expect("变量块未闭合");
        let body = &css[body_start..body_end];
        let mut out = BTreeMap::new();
        for declaration in body.split(';') {
            let Some((key, value)) = declaration.split_once(':') else {
                continue;
            };
            let key = key.trim();
            if !key.starts_with("--") {
                continue;
            }
            out.insert(
                key.trim_start_matches("--").to_string(),
                value.trim().to_string(),
            );
        }
        return out;
    }

    fn normalize(value: &str) -> String {
        let value = value.trim().to_lowercase();
        if let Some(hex) = value.strip_prefix('#') {
            if hex.len() == 3 {
                let expanded: String = hex.chars().flat_map(|c| [c, c]).collect();
                return format!("#{expanded}");
            }
            return format!("#{hex}");
        }
        // `rgba(0,0,0,.045)` 与 `rgba(0,0,0,0.045)` 是同一个颜色。
        return value.replace(",.", ",0.").replace("(,.", "(0.");
    }

    fn ours(mode: &str) -> BTreeMap<String, String> {
        let parsed: serde_json::Value = serde_json::from_str(THEME).expect("theme.json 可以解析");
        let block = parsed["cssVars"][mode]
            .as_object()
            .unwrap_or_else(|| panic!("theme.json 缺少 cssVars.{mode}"));
        let mut out = BTreeMap::new();
        for (key, value) in block {
            if let Some(text) = value.as_str() {
                // 我们叫 panel2，设计叫 panel-2。
                out.insert(key.clone(), text.to_string());
            }
        }
        return out;
    }

    fn compare(mode: &str, selector: &str) {
        let path = design_root().join("project/assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin");
        let css = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("读取设计 CSS 失败：{error}"));
        let design = variables(&css, selector);
        let ours = ours(mode);
        assert!(!design.is_empty(), "{selector} 里没有变量");

        let mut problems: Vec<String> = Vec::new();
        for (key, design_value) in &design {
            if SEMANTIC.contains(&key.as_str()) {
                continue;
            }
            let lookup = if key == "panel-2" {
                "panel2"
            } else {
                key.as_str()
            };
            match ours.get(lookup) {
                None => problems.push(format!("缺少 {lookup}（设计值 {design_value}）")),
                Some(value) => {
                    if normalize(value) != normalize(design_value) {
                        problems.push(format!("{lookup}: 设计 {design_value} / 我们 {value}"));
                    }
                }
            }
        }
        assert!(
            problems.is_empty(),
            "{mode} 有 {} 个颜色与设计不一致：\n  {}",
            problems.len(),
            problems.join("\n  ")
        );
    }

    #[test]
    fn light_colors_match_the_design() {
        compare("light", ":root");
    }

    #[test]
    fn dark_colors_match_the_design() {
        compare("dark", ".theme-dark");
    }
}
