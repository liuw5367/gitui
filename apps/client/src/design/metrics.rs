//! 几何常量：数值来自设计稿共享样式表与各板内联尺寸。
//!
//! 这些值是“完全还原”的判定基准：`tests/geometry.rs` 会重新解析样式表并逐个比对，
//! 样式表或实现任一侧漂移都会失败。改数值前先改设计稿或同步更新本文件与测试。

/// 窗口内容区宽（1440 视口减 `.desktop` 左右各 24）。
pub const CONTENT_W: f32 = 1392.0;
/// 窗口内容区高（900 视口减上下各 24）。
pub const CONTENT_H: f32 = 852.0;

/// 工作区标题栏（`.titlebar.with-tabs`）；仓库 tabs 本身保持 27pt。
pub const TITLEBAR_H: f32 = 40.0;
/// 无仓库标签页时的标题栏（`.titlebar`）。
pub const TITLEBAR_PLAIN_H: f32 = 38.0;
/// `.toolbar.v2` 紧凑高度。
pub const TOOLBAR_H: f32 = 40.0;
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
/// File diff 返回提交列表的摘要条（画板 04b）。
pub const DIFF_RETURN_H: f32 = 34.0;
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
pub const RTAB_H: f32 = native::ROW;
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

/// 原生刻度：本文件的二级基准。
///
/// 设计稿的共享样式表是「画板之间保持一致」的来源，但它的字号（8.5–10.5）与行高
/// 低于 macOS 的原生刻度。按 `plans.local/2026-11-15-native-first-ui-alignment.md`
/// 的 D3，字号、行高、控件高改以系统刻度为准；颜色仍取自 `theme.json`。
pub mod native {
    /// 正文与标签。
    pub const FONT_BODY: f32 = 13.0;
    /// 正文行高。
    pub const LINE_BODY: f32 = 18.0;
    /// 说明、帮助与表单提示。
    pub const FONT_SECONDARY: f32 = 11.0;
    /// 次要文字行高。
    pub const LINE_SECONDARY: f32 = 15.0;
    /// 元数据与表头。
    pub const FONT_META: f32 = 11.0;
    /// 代码与 diff 正文。
    pub const FONT_CODE: f32 = 12.0;
    /// 代码行高。
    pub const LINE_CODE: f32 = 18.0;

    /// 小控件。
    pub const CONTROL_SMALL: f32 = 22.0;
    /// 常规控件。
    pub const CONTROL: f32 = 28.0;
    /// 大控件。
    pub const CONTROL_LARGE: f32 = 32.0;

    /// 标准行高。
    pub const ROW: f32 = 28.0;
    /// 紧凑行高。
    pub const ROW_COMPACT: f32 = 24.0;

    /// 工具栏高度。
    pub const TOOLBAR: f32 = 52.0;
    /// 状态栏高度。
    pub const STATUS: f32 = 24.0;
}

#[cfg(test)]
mod native_scale_tests {
    //! 原生刻度的内部一致性与「不得回落」检查。
    //!
    //! 设计 CSS 不再是尺寸来源（D3），因此这里不再逐值比对样式表；改为锁住刻度内部
    //! 的关系，并检查已经迁移的界面没有把字号写回 11pt 以下。

    use super::native;

    #[test]
    fn type_scale_is_ordered_and_legible() {
        assert!(native::FONT_META >= 11.0, "元数据不得低于 11pt");
        assert!(native::FONT_SECONDARY >= 11.0, "次要文字不得低于 11pt");
        assert!(native::FONT_CODE >= 11.0, "代码不得低于 11pt");
        assert!(native::FONT_BODY >= native::FONT_SECONDARY);
        assert!(native::FONT_CODE < native::FONT_BODY);
        assert!(native::LINE_BODY > native::FONT_BODY);
        assert!(native::LINE_CODE > native::FONT_CODE);
    }

    #[test]
    fn control_and_row_heights_follow_the_system_scale() {
        assert_eq!(native::CONTROL, 28.0);
        assert_eq!(native::CONTROL_SMALL, 22.0);
        assert_eq!(native::CONTROL_LARGE, 32.0);
        assert_eq!(native::ROW, 28.0);
        assert_eq!(native::ROW_COMPACT, 24.0);
        assert_eq!(native::TOOLBAR, 52.0);
        assert_eq!(native::STATUS, 24.0);
        assert!(native::ROW_COMPACT < native::ROW);
        assert!(native::CONTROL_SMALL < native::CONTROL);
        assert!(native::CONTROL < native::CONTROL_LARGE);
    }
}
