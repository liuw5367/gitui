//! 设置页的 Signal 集合：持久化设置、当前分区与快捷键分区的编辑状态。
//!
//! 主题（`ThemeMode`）、语言、最近仓库与 Git 探测结果不在这里：它们是会话级或
//! 启动时读取的值，走 `SessionHandle` 与 `SettingsCommands` 的字段。

use nsh_core::prelude::*;

/// 侧栏分组（设计板 23 的 `.sg` 标题）。Agent 是我们新增的一页，归在应用设置里。
const GROUPS: [(&str, &[Section]); 3] = [
    (
        "settings.groupApplication",
        &[Section::General, Section::Shortcuts, Section::Agent],
    ),
    ("settings.groupGit", &[Section::Git, Section::Credentials]),
    ("settings.groupAbout", &[Section::About]),
];

/// 设置页的分区。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Section {
    General,
    Git,
    Shortcuts,
    About,
    Credentials,
    Agent,
}

impl Section {
    /// 侧栏图标（与设计稿 23 的 `.si` 一致；`keys` / `lock` 取自设计 sprite）。
    pub(crate) fn icon(self) -> &'static str {
        return match self {
            Section::General => "gear",
            Section::Shortcuts => "keys",
            Section::Git => "branch",
            Section::Credentials => "lock",
            Section::Agent => "branch",
            Section::About => "info",
        };
    }

    pub(crate) fn key(self) -> &'static str {
        return match self {
            Section::General => "settings.general",
            Section::Git => "settings.git",
            Section::Shortcuts => "settings.shortcuts",
            Section::About => "settings.about",
            Section::Credentials => "settings.credentials",
            Section::Agent => "settings.agent",
        };
    }
}

/// 侧栏按组列出分区；视图只遍历这个常量。
pub(crate) fn groups() -> [(&'static str, &'static [Section]); 3] {
    return GROUPS;
}

#[derive(Clone)]
pub(crate) struct SettingsState {
    /// 落盘的设置值（主题单独在 `ThemeMode` 信号里）。
    pub(crate) settings_state: Signal<store::Settings>,
    /// 左侧导航当前选中的分区。
    pub(crate) settings_section: Signal<Section>,
    /// 快捷键注册表（含用户改键）。
    pub(crate) shortcut_registry: Signal<crate::shortcuts::Registry>,
    /// 快捷键分区的搜索词。
    pub(crate) shortcut_query: Signal<String>,
    /// 正在录制新组合的动作 id；`None` 表示没有录制。
    pub(crate) recording_shortcut: Signal<Option<&'static str>>,
    /// 改键失败的原因（冲突或不可绑定）；成功后清空。
    pub(crate) shortcut_error: Signal<Option<String>>,
    /// Commit identity 编辑框的值（仓库级；没有打开仓库时是全局值）。
    pub(crate) identity_name: Signal<String>,
    pub(crate) identity_email: Signal<String>,
    /// 是否继承全局身份（复选框状态）。
    pub(crate) identity_use_global: Signal<bool>,
    /// 日期 pattern 的编辑值；失焦时才写回设置。
    pub(crate) date_pattern: Signal<String>,
    /// 本机 Git 的版本、执行路径与凭据助手（启动时探测一次，可在 Git 分区刷新）。
    pub(crate) git_version: Signal<String>,
    pub(crate) git_exec_path: Signal<String>,
    pub(crate) credential_helper: Signal<String>,
    /// Git 可执行文件的编辑值（Choose… / 手动输入）。
    pub(crate) git_path: Signal<String>,
    /// 扫描目录、每个目录的仓库数、聚合结果、勾选结果与上次扫描时间。
    pub(crate) scan_folders: Signal<Vec<std::path::PathBuf>>,
    pub(crate) scan_counts: Signal<Vec<usize>>,
    pub(crate) scan_found: Signal<Vec<String>>,
    pub(crate) scan_selected: Signal<Vec<String>>,
    pub(crate) scan_last_at: Signal<Option<i64>>,
    /// Agent 端点 / 数值字段的编辑值；提交时才写设置。
    pub(crate) agent_base_url: Signal<String>,
    pub(crate) agent_temperature: Signal<String>,
    pub(crate) agent_max_tokens: Signal<String>,
    /// App key 只保存在本次会话的内存信号里，不进入 Settings / 磁盘。
    pub(crate) agent_app_key: Signal<String>,
}
