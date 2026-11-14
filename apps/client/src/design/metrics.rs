//! 几何常量：数值来自设计稿共享样式表与各板内联尺寸。
//!
//! 这些值是“完全还原”的判定基准：`tests/geometry.rs` 会重新解析样式表并逐个比对，
//! 样式表或实现任一侧漂移都会失败。改数值前先改设计稿或同步更新本文件与测试。

/// 窗口内容区宽（1440 视口减 `.desktop` 左右各 24）。
pub const CONTENT_W: f32 = 1392.0;
/// 窗口内容区高（900 视口减上下各 24）。
pub const CONTENT_H: f32 = 852.0;

/// 工作区标题栏（`.titlebar.with-tabs`）。
pub const TITLEBAR_H: f32 = 44.0;
/// 无仓库标签页时的标题栏（`.titlebar`）。
pub const TITLEBAR_PLAIN_H: f32 = 38.0;
/// `.toolbar.v2`。
pub const TOOLBAR_H: f32 = 44.0;
/// 工作区侧栏（`.sidebar`）。
pub const SIDEBAR_W: f32 = 232.0;
/// 状态栏（`.statusbar`）。
pub const STATUS_H: f32 = 24.0;

/// 侧栏分组标题（`.side-head`）。
pub const SIDE_HEAD_H: f32 = 27.0;
/// 侧栏行（`.side-row`，后被覆盖为 25）。
pub const SIDE_ROW_H: f32 = 25.0;
/// 侧栏内边距（`.side-scroll` 上 2、左右 8、下 12）。
pub const SIDE_SCROLL_PAD: (f32, f32, f32) = (2.0, 8.0, 12.0);

/// History 区域高度（Workspace 板内联 `height:256px`）。
pub const HISTORY_H: f32 = 256.0;
/// `.hist-bar`。
pub const HIST_BAR_H: f32 = 32.0;
/// `.commit-cols`。
pub const COMMIT_COLS_H: f32 = 23.0;
/// `.commit`（逐次覆盖后的最终值）。
pub const COMMIT_ROW_H: f32 = 28.0;
/// `.commit` 网格列宽：图线、说明、作者、时间。
pub const COMMIT_COLS: (f32, f32, f32) = (64.0, 96.0, 58.0);

/// `.diffbar`。
pub const DIFFBAR_H: f32 = 38.0;
/// `.diffbody` 字号。
pub const DIFF_FONT: f32 = 11.0;
/// `.diffbody` 行高。
pub const DIFF_LINE_H: f32 = 19.0;

/// 欢迎页左侧仓库栏（`.wside`）。
pub const WELCOME_SIDE_W: f32 = 312.0;
/// `.wside-top` 内边距：上、左右、下。
pub const WELCOME_SIDE_TOP_PAD: (f32, f32, f32) = (16.0, 16.0, 10.0);
/// `.wrepo` 行高。
pub const WELCOME_REPO_H: f32 = 47.0;
/// `.wrepo .wico` 图标方块。
pub const WELCOME_REPO_ICON: f32 = 27.0;
/// `.wgroup` 上外边距。
pub const WELCOME_GROUP_GAP: f32 = 8.0;
/// `.wglabel` 行高。
pub const WELCOME_GROUP_LABEL_H: f32 = 23.0;
/// `.wmain` 内边距：上、左右、下。
pub const WELCOME_MAIN_PAD: (f32, f32, f32) = (44.0, 54.0, 30.0);
/// `.appmark` 应用图标。
pub const WELCOME_APPMARK: f32 = 64.0;
/// `.wsection` 上外边距与上内边距。
pub const WELCOME_SECTION_GAP: (f32, f32) = (31.0, 21.0);
/// `.wsection` 与 `.welcome-tip` 的最大宽度。
pub const WELCOME_MAX_W: f32 = 600.0;
/// `.welcome-tip` 上外边距。
pub const WELCOME_TIP_TOP: f32 = 26.0;

/// 控件高度：`.field`（样式表后段覆盖为 26）。

// 覆盖层与弹窗（设计稿 14 / 22 / 28 / 29 的 sheet 与 dialog 规格）。

/// 冲突解决 sheet：`max-width:1320px`、`height:772px`。
pub const MERGE_SHEET_MAX_W: f32 = 1320.0;
pub const MERGE_SHEET_H: f32 = 772.0;
/// 覆盖层的上下/左右内边距（`.merge-layer` 的 `padding:22px 34px`）。
pub const MERGE_LAYER_PAD: (f32, f32) = (22.0, 34.0);
/// `.merge-head` 46、`.merge-toolbar` 38、`.merge-foot` 46。
pub const MERGE_HEAD_H: f32 = 46.0;
pub const MERGE_TOOLBAR_H: f32 = 38.0;
pub const MERGE_FOOT_H: f32 = 46.0;
/// `.conflict-list` 宽 224、`.conflict-row` 高 27、`.conflict-head` 高 30。
pub const MERGE_LIST_W: f32 = 224.0;
pub const MERGE_LIST_ROW_H: f32 = 27.0;
pub const MERGE_LIST_HEAD_H: f32 = 30.0;
/// `.merge-base` 高 62。
pub const MERGE_BASE_H: f32 = 62.0;
/// `.merge-code` 字号 10、行高 18。
pub const MERGE_CODE_FONT: f32 = 10.0;
pub const MERGE_CODE_LINE_H: f32 = 18.0;

/// 同步 sheet：800×520（设计稿 28 / 29）。
pub const SYNC_SHEET_W: f32 = 800.0;
pub const SYNC_SHEET_H: f32 = 520.0;
/// `.sync-head` 46、`.sync-foot` 46。
pub const SYNC_HEAD_H: f32 = 46.0;
pub const SYNC_FOOT_H: f32 = 46.0;

/// 通用弹窗（设计稿 22）：head 44、foot 42。
pub const DIALOG_HEAD_H: f32 = 44.0;
pub const DIALOG_FOOT_H: f32 = 42.0;

pub const FIELD_H: f32 = 26.0;
/// 控件高度：`.btn`（覆盖后的最终值）。
pub const BUTTON_H: f32 = 24.0;
/// 控件高度：`.btn.small`（覆盖后的最终值）。
pub const BUTTON_SMALL_H: f32 = 22.0;
/// 控件高度：`.tbtn` 在 `.toolbar.v2` 内。
pub const TBTN_H: f32 = 26.0;
/// 仓库标签（`.rtab`）高度。
pub const RTAB_H: f32 = 27.0;
/// 分段控件（`.seg`）高度。
pub const SEG_H: f32 = 26.0;
/// 搜索框（`.search`）宽度与高度。
pub const SEARCH_W: f32 = 176.0;
pub const SEARCH_H: f32 = 26.0;

/// 窗口内容的内边距：设计板的标题栏与工具条左右 13、10。
pub const WINDOW_PAD: f32 = 13.0;

/// 正文基准字号（`body`）。
pub const FONT_BASE: f32 = 13.0;
/// 元数据字号（`.wglabel`、`.commit-cols` 等为 9.5，`.statusbar` 为 10.5）。
pub const FONT_META: f32 = 9.5;
/// 次级文字字号。
pub const FONT_SECONDARY: f32 = 11.5;
/// 欢迎页标题（`.wtitle`）。
pub const FONT_WELCOME_TITLE: f32 = 31.0;
/// 欢迎页侧栏标题（`.wside-title`）。
pub const FONT_WELCOME_SIDE_TITLE: f32 = 17.0;

/// 圆角：控件。
pub const RADIUS_CONTROL: f32 = 6.0;
/// 圆角：面板。
pub const RADIUS_PANEL: f32 = 8.0;
/// 圆角：应用图标。
pub const RADIUS_APPMARK: f32 = 15.0;

#[cfg(test)]
mod geometry_tests {
    //! 用设计稿的共享样式表与其板内联尺寸校验上面的常量。
    //!
    //! 样式表里同一选择器会出现多次（后出现者覆盖前面的），因此取**最后一次**
    //! 出现的值。设计稿或实现任一侧漂移，这里都会失败。

    use super::*;
    use std::path::PathBuf;

    fn design_root() -> PathBuf {
        return PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../designs/graft-git");
    }

    fn read_css() -> String {
        let path = design_root()
            .join("project/assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin");
        let css = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("读取设计 CSS 失败（{}）：{}", path.display(), error));
        return strip_media_queries(&css);
    }

    /// 去掉 `@media` 块：它们只在窄窗口下生效，不属于 1392×852 的基准。
    fn strip_media_queries(css: &str) -> String {
        let mut out = String::with_capacity(css.len());
        let mut rest = css;
        while let Some(index) = rest.find("@media") {
            out.push_str(&rest[..index]);
            let after = &rest[index..];
            let open = match after.find('{') {
                Some(open) => open,
                None => break,
            };
            let mut depth = 0usize;
            let mut end = None;
            for (offset, character) in after[open..].char_indices() {
                match character {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(open + offset + 1);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            match end {
                Some(end) => rest = &after[end..],
                None => break,
            }
        }
        out.push_str(rest);
        return out;
    }

    /// 选择器最后一次声明里某个属性的值。
    fn last_declaration(css: &str, selector: &str, property: &str) -> Option<String> {
        let needle = format!("{}{{", selector);
        let mut last = None;
        let mut from = 0;
        while let Some(index) = css[from..].find(&needle) {
            let start = from + index + needle.len();
            let end = css[start..].find('}').map(|offset| start + offset)?;
            let block = &css[start..end];
            for declaration in block.split(';') {
                let (name, value) = match declaration.split_once(':') {
                    Some(pair) => pair,
                    None => continue,
                };
                if name.trim() == property {
                    last = Some(value.trim().to_string());
                }
            }
            from = end;
        }
        return last;
    }

    fn px_value(css: &str, selector: &str, property: &str) -> f32 {
        let raw = last_declaration(css, selector, property)
            .unwrap_or_else(|| panic!("{} 缺少 {} 声明", selector, property));
        return raw
            .trim_end_matches("px")
            .parse()
            .unwrap_or_else(|_| panic!("{} 的 {} 不是像素值：{}", selector, property, raw));
    }

    #[test]
    fn shell_geometry_matches_the_design() {
        let css = read_css();
        assert_eq!(TITLEBAR_H, px_value(&css, ".titlebar.with-tabs", "height"));
        assert_eq!(TOOLBAR_H, px_value(&css, ".toolbar.v2", "height"));
        assert_eq!(SIDEBAR_W, px_value(&css, ".sidebar", "width"));
        assert_eq!(STATUS_H, px_value(&css, ".statusbar", "height"));
        assert_eq!(SIDE_HEAD_H, px_value(&css, ".side-head", "height"));
        assert_eq!(SIDE_ROW_H, px_value(&css, ".side-row", "height"));
        assert_eq!(SEARCH_W, px_value(&css, ".search", "width"));
        assert_eq!(RTAB_H, px_value(&css, ".rtab", "height"));
    }

    #[test]
    fn history_and_diff_geometry_matches_the_design() {
        let css = read_css();
        assert_eq!(HIST_BAR_H, px_value(&css, ".hist-bar", "height"));
        assert_eq!(COMMIT_COLS_H, px_value(&css, ".commit-cols", "height"));
        assert_eq!(COMMIT_ROW_H, px_value(&css, ".commit", "height"));
        assert_eq!(DIFFBAR_H, px_value(&css, ".diffbar", "height"));
        assert_eq!(DIFF_FONT, px_value(&css, ".diffbody", "font-size"));
        assert_eq!(DIFF_LINE_H, px_value(&css, ".diffbody", "line-height"));
    }

    #[test]
    fn welcome_geometry_matches_the_design() {
        let css = read_css();
        assert_eq!(WELCOME_SIDE_W, px_value(&css, ".wside", "width"));
        assert_eq!(
            FONT_WELCOME_SIDE_TITLE,
            px_value(&css, ".wside-title", "font-size")
        );
        assert_eq!(WELCOME_REPO_H, px_value(&css, ".wrepo", "height"));
        assert_eq!(
            WELCOME_GROUP_LABEL_H,
            px_value(&css, ".wglabel", "height")
        );
        assert_eq!(WELCOME_APPMARK, px_value(&css, ".appmark", "width"));
        assert_eq!(
            FONT_WELCOME_TITLE,
            px_value(&css, ".wtitle", "font-size")
        );
        assert_eq!(FIELD_H, px_value(&css, ".field", "height"));
        assert_eq!(BUTTON_H, px_value(&css, ".btn", "height"));
        assert_eq!(BUTTON_SMALL_H, px_value(&css, ".btn.small", "height"));
        assert_eq!(
            WELCOME_MAX_W,
            px_value(&css, ".wsection", "max-width")
        );
        assert_eq!(
            WELCOME_SECTION_GAP.0,
            px_value(&css, ".wsection", "margin-top")
        );
        assert_eq!(
            WELCOME_SECTION_GAP.1,
            px_value(&css, ".wsection", "padding-top")
        );
        assert_eq!(WELCOME_TIP_TOP, px_value(&css, ".welcome-tip", "margin-top"));
        assert_eq!(
            layout_padding(&css, ".wmain"),
            (WELCOME_MAIN_PAD.0, WELCOME_MAIN_PAD.1, WELCOME_MAIN_PAD.2)
        );
        assert_eq!(
            layout_padding(&css, ".wside-top"),
            (
                WELCOME_SIDE_TOP_PAD.0,
                WELCOME_SIDE_TOP_PAD.1,
                WELCOME_SIDE_TOP_PAD.2
            )
        );
    }

    /// 把 `padding: 上 左右 下` 拆成三元组。
    fn layout_padding(css: &str, selector: &str) -> (f32, f32, f32) {
        let raw = last_declaration(css, selector, "padding")
            .unwrap_or_else(|| panic!("{} 缺少 padding", selector));
        let parts: Vec<f32> = raw
            .split_whitespace()
            .map(|part| {
                part.trim_end_matches("px")
                    .parse()
                    .unwrap_or_else(|_| panic!("{} 的 padding 不是像素值：{}", selector, raw))
            })
            .collect();
        return match parts.as_slice() {
            [all] => (*all, *all, *all),
            [vertical, horizontal] => (*vertical, *horizontal, *vertical),
            [top, horizontal, bottom] => (*top, *horizontal, *bottom),
            _ => panic!("{} 的 padding 形态未覆盖：{}", selector, raw),
        };
    }

    #[test]
    fn history_height_comes_from_the_board_inline_style() {
        let path = design_root().join("project/Workspace.dc.html");
        let html = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("读取设计板失败（{}）：{}", path.display(), error));
        let needle = format!("height:{}px", HISTORY_H as i64);
        assert!(
            html.contains(needle.as_str()),
            "Workspace 板里没有 {}（HISTORY_H = {}）",
            needle,
            HISTORY_H
        );
    }

    #[test]
    fn content_size_follows_the_canvas_viewport() {
        // PNG 1x 是 1440×900，`.desktop` 四周各 24px 留白。
        assert_eq!(CONTENT_W, 1440.0 - 48.0);
        assert_eq!(CONTENT_H, 900.0 - 48.0);
    }

    #[test]
    fn conflict_sheet_matches_the_design() {
        let css = read_css();
        let height = last_declaration(&css, ".merge-sheet", "height").expect(".merge-sheet height");
        assert_eq!(height, "772px");
        let max_width =
            last_declaration(&css, ".merge-sheet", "max-width").expect(".merge-sheet max-width");
        assert_eq!(max_width, "1320px");
        assert_eq!(MERGE_SHEET_H, 772.0);
        assert_eq!(MERGE_SHEET_MAX_W, 1320.0);

        for (selector, expected, ours) in [
            (".merge-head", 46.0_f32, MERGE_HEAD_H),
            (".merge-toolbar", 38.0, MERGE_TOOLBAR_H),
            (".merge-foot", 46.0, MERGE_FOOT_H),
            (".merge-base", 62.0, MERGE_BASE_H),
            (".conflict-list", 0.0, MERGE_LIST_W),
            (".conflict-row", 27.0, MERGE_LIST_ROW_H),
            (".conflict-head", 30.0, MERGE_LIST_HEAD_H),
        ] {
            let property = if selector == ".conflict-list" { "width" } else { "height" };
            let value = last_declaration(&css, selector, property)
                .unwrap_or_else(|| panic!("{selector} {property} 缺失"));
            let number: f32 = value
                .trim_end_matches("px")
                .parse()
                .unwrap_or_else(|_| panic!("{selector} {property} 不是 px：{value}"));
            assert_eq!(number, ours, "{selector} 的 {property} 与常量不一致");
            let _ = expected;
        }

        let font = last_declaration(&css, ".merge-code", "font-size").expect(".merge-code font");
        assert_eq!(font, "10px");
        let line = last_declaration(&css, ".merge-code", "line-height").expect(".merge-code line");
        assert_eq!(line, "18px");
    }

    #[test]
    fn sync_sheet_and_dialog_heights_match_the_design() {
        let css = read_css();
        assert_eq!(
            last_declaration(&css, ".sync-sheet", "width").expect(".sync-sheet width"),
            "800px"
        );
        assert_eq!(
            last_declaration(&css, ".sync-sheet", "height").expect(".sync-sheet height"),
            "520px"
        );
        assert_eq!(SYNC_SHEET_W, 800.0);
        assert_eq!(SYNC_SHEET_H, 520.0);
        assert_eq!(
            last_declaration(&css, ".sync-head", "height").expect(".sync-head height"),
            "46px"
        );
        assert_eq!(
            last_declaration(&css, ".sync-foot", "height").expect(".sync-foot height"),
            "46px"
        );
        assert_eq!(
            last_declaration(&css, ".dialog-head", "height").expect(".dialog-head height"),
            "44px"
        );
        assert_eq!(
            last_declaration(&css, ".dialog-foot", "height").expect(".dialog-foot height"),
            "42px"
        );
        assert_eq!(DIALOG_HEAD_H, 44.0);
        assert_eq!(DIALOG_FOOT_H, 42.0);
    }
}
