//! 本地持久化：设置写入 TOML，运行状态与审计保持独立文本文件。
//!
//! 偏好统一保存在 `settings.toml`；会话、最近仓库、变更清单与审计继续使用文本格式：
//!
//! - `recent.txt`：最近打开的仓库，最新的在第一行，每行 `<打开时间>\t<路径>`；
//! - `session.txt`：第一行是当前 tab 下标，其后每行是一个打开中的仓库路径。
//! - `changelists.txt`：仓库变更清单；`agent-audit.txt`：Agent 审计记录。
//!
//! 所有函数都接受目录参数，便于测试用临时目录；应用层用 [`default_dir`] 取得
//! `~/.config/gitui`。写入采用“先写临时文件再改名”，避免中途失败留下半个文件。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// 最近仓库的数量上限。
const MAX_RECENT: usize = 12;

/// 一条最近仓库记录：路径、最后一次打开的时间（UTC 秒）与最近已知分支。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecentRepo {
    pub path: String,
    pub opened_at: i64,
    /// 最近一次打开时所在的分支；旧文件没有这一列时为空。
    pub branch: String,
}

/// 会话：当前选中的 tab 与打开的仓库路径。空 `paths` 表示显示欢迎页。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Session {
    /// 当前 tab 的下标；越界时按最后一个处理。
    pub active: usize,
    /// 打开的仓库路径，按 tab 顺序。
    pub paths: Vec<String>,
}

/// `~/.config/<directory>`；`APP_HOME` 可用于测试或便携使用。
///
/// 目录名由调用方传入，避免把产品名写死在存储层。
pub fn default_dir(directory: &str) -> PathBuf {
    if let Some(home) = std::env::var_os("APP_HOME") {
        return PathBuf::from(home);
    }
    let base = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    return config_dir(&base, directory);
}

/// 用户目录下的应用数据目录；单独成函数，便于测试路径规则。
fn config_dir(home: &Path, directory: &str) -> PathBuf {
    return home.join(".config").join(directory);
}

/// 读取界面语言；未设置时返回 `None`（由应用跟随系统语言）。
pub fn load_language(dir: &Path) -> Option<String> {
    let config = read_config(dir)?;
    let language = config.general.language.trim();
    if language.is_empty() {
        return None;
    }
    return Some(language.to_string());
}

/// 保存界面语言。
pub fn save_language(dir: &Path, language: &str) {
    update_config(dir, |config| config.general.language = language.to_string());
}

/// 一个变更清单：名称与其中的文件路径。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Changelist {
    pub name: String,
    pub paths: Vec<String>,
}

fn changelists_file(dir: &Path) -> PathBuf {
    return dir.join("changelists.txt");
}

/// 读取某个仓库的变更清单；文件缺失时返回空列表。
pub fn load_changelists(dir: &Path, repo: &str) -> Vec<Changelist> {
    let text = match std::fs::read_to_string(changelists_file(dir)) {
        Ok(text) => text,
        Err(_) => return Vec::new(),
    };
    let mut lists: Vec<Changelist> = Vec::new();
    for line in text.lines() {
        let mut fields = line.split('\t');
        let (Some(entry_repo), Some(name), Some(path)) =
            (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        if entry_repo != repo || name.is_empty() {
            continue;
        }
        match lists.iter_mut().find(|list| list.name == name) {
            Some(list) => {
                if !path.is_empty() {
                    list.paths.push(path.to_string());
                }
            }
            None => lists.push(Changelist {
                name: name.to_string(),
                paths: if path.is_empty() {
                    Vec::new()
                } else {
                    vec![path.to_string()]
                },
            }),
        }
    }
    return lists;
}

/// 保存某个仓库的变更清单，保留其他仓库的条目。
pub fn save_changelists(dir: &Path, repo: &str, lists: &[Changelist]) {
    let mut lines: Vec<String> = Vec::new();
    // 先写其他仓库的条目。
    if let Ok(text) = std::fs::read_to_string(changelists_file(dir)) {
        for line in text.lines() {
            if let Some((entry_repo, _)) = line.split_once('\t') {
                if entry_repo != repo && !line.trim().is_empty() {
                    lines.push(line.to_string());
                }
            }
        }
    }
    for list in lists {
        if list.name.is_empty() || list.name.contains('\t') {
            continue;
        }
        if list.paths.is_empty() {
            lines.push(format!("{}\t{}\t", repo, list.name));
        } else {
            for path in &list.paths {
                if path.contains('\t') {
                    continue;
                }
                lines.push(format!("{}\t{}\t{}", repo, list.name, path));
            }
        }
    }
    let _ = write_lines(&changelists_file(dir), &lines);
}

/// 应用设置（除语言与外观外的偏好）。
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// 变更列表按目录分组（关闭时为平铺路径）。
    pub file_tree: bool,
    /// DiffView 是否使用并排布局。
    pub diff_split: bool,
    /// DiffView 是否折叠未修改上下文。
    pub diff_fold: bool,
    /// DiffView 是否对长行软换行。
    pub diff_wrap: bool,
    /// DiffView 是否显示空格与制表符。
    pub diff_whitespace: bool,
    /// Git 生成差异时是否忽略空白变化。
    pub diff_ignore_whitespace: bool,
    /// 合并空目录：只有一层子目录的层级合并成一行。
    pub merge_empty_dirs: bool,
    /// 压缩单子目录：`a/b/c` 这种链式目录合并显示。
    pub compact_single_child: bool,
    /// 启动时恢复上次打开的仓库；关闭则显示欢迎页。
    pub restore_session: bool,
    /// 更新检查方式：`auto` / `manual` / `never`。
    pub update_mode: String,
    /// 使用 PATH 里的 git，而不是固定路径。
    pub use_system_git: bool,
    /// 手动指定的 git 路径（`use_system_git` 为假时使用）。
    pub git_path: String,
    /// 日期显示方式：`relative` / `absolute` / `system` / `custom`。
    pub date_format: String,
    /// `date_format` 为 `custom` 时使用的 pattern（`%Y-%m-%d` 风格）。
    pub date_pattern: String,
    /// Agent 的线格式：`openai` / `anthropic`。
    pub agent_protocol: String,
    /// Agent 端点的基础 URL（不带尾斜杠）。
    pub agent_base_url: String,
    /// Agent 模型 id。
    pub agent_model: String,
    /// 采样温度。
    pub agent_temperature: f32,
    /// 单次回复上限。
    pub agent_max_tokens: u32,
    /// 请求是否携带 diff hunk。
    pub agent_send_diffs: bool,
    /// 请求是否携带绝对仓库路径。
    pub agent_send_repository_path: bool,
    /// 添加远端时优先使用 SSH URL。
    pub ssh_for_new_remotes: bool,
    /// 外部编辑器、diff/merge 工具与终端。
    pub external_editor: String,
    pub diff_tool: String,
    pub merge_tool: String,
    pub terminal: String,
    /// 提交信息使用外部编辑器。
    pub external_editor_for_messages: bool,
    /// 代理方式：`system` / `none` / `manual`。
    pub proxy_mode: String,
    /// 校验 SSL 证书。
    pub verify_ssl: bool,
    /// 其余 Git 设置读取 `~/.gitconfig`。
    pub use_system_git_config: bool,
}

impl Default for Settings {
    fn default() -> Self {
        return Settings {
            file_tree: true,
            diff_split: false,
            diff_fold: false,
            diff_wrap: false,
            diff_whitespace: false,
            diff_ignore_whitespace: false,
            merge_empty_dirs: true,
            compact_single_child: true,
            restore_session: true,
            update_mode: "auto".to_string(),
            use_system_git: true,
            git_path: String::new(),
            date_format: "relative".to_string(),
            date_pattern: "%b %d, %Y".to_string(),
            agent_protocol: "openai".to_string(),
            agent_base_url: "https://api.openai.com".to_string(),
            agent_model: "gpt-4.1".to_string(),
            agent_temperature: 0.2,
            agent_max_tokens: 4096,
            agent_send_diffs: true,
            agent_send_repository_path: false,
            ssh_for_new_remotes: true,
            external_editor: "vscode".to_string(),
            diff_tool: "builtin".to_string(),
            merge_tool: "builtin".to_string(),
            terminal: "terminal".to_string(),
            external_editor_for_messages: false,
            proxy_mode: "system".to_string(),
            verify_ssl: true,
            use_system_git_config: true,
        };
    }
}

const CONFIG_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredConfig {
    schema_version: u32,
    #[serde(default)]
    general: GeneralPreferences,
    #[serde(default)]
    git: GitPreferences,
    #[serde(default)]
    shortcuts: Vec<ShortcutPreference>,
    #[serde(default)]
    agent: AgentPreferencesSection,
}

impl Default for StoredConfig {
    fn default() -> Self {
        return StoredConfig {
            schema_version: CONFIG_SCHEMA_VERSION,
            general: GeneralPreferences::default(),
            git: GitPreferences::default(),
            shortcuts: Vec::new(),
            agent: AgentPreferencesSection::default(),
        };
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
struct GeneralPreferences {
    language: String,
    appearance: String,
    file_tree: bool,
    diff_split: bool,
    diff_fold: bool,
    diff_wrap: bool,
    diff_whitespace: bool,
    diff_ignore_whitespace: bool,
    merge_empty_dirs: bool,
    compact_single_child: bool,
    restore_session: bool,
    update_mode: String,
    date_format: String,
    date_pattern: String,
}

impl Default for GeneralPreferences {
    fn default() -> Self {
        let settings = Settings::default();
        return GeneralPreferences {
            language: String::new(),
            appearance: String::new(),
            file_tree: settings.file_tree,
            diff_split: settings.diff_split,
            diff_fold: settings.diff_fold,
            diff_wrap: settings.diff_wrap,
            diff_whitespace: settings.diff_whitespace,
            diff_ignore_whitespace: settings.diff_ignore_whitespace,
            merge_empty_dirs: settings.merge_empty_dirs,
            compact_single_child: settings.compact_single_child,
            restore_session: settings.restore_session,
            update_mode: settings.update_mode,
            date_format: settings.date_format,
            date_pattern: settings.date_pattern,
        };
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
struct GitPreferences {
    use_system_git: bool,
    git_path: String,
    ssh_for_new_remotes: bool,
    external_editor: String,
    diff_tool: String,
    merge_tool: String,
    terminal: String,
    external_editor_for_messages: bool,
    proxy_mode: String,
    verify_ssl: bool,
    use_system_git_config: bool,
    scan_folders: Vec<String>,
    scan_search_preferences: Vec<ScanSearchPreference>,
}

impl Default for GitPreferences {
    fn default() -> Self {
        let settings = Settings::default();
        return GitPreferences {
            use_system_git: settings.use_system_git,
            git_path: settings.git_path,
            ssh_for_new_remotes: settings.ssh_for_new_remotes,
            external_editor: settings.external_editor,
            diff_tool: settings.diff_tool,
            merge_tool: settings.merge_tool,
            terminal: settings.terminal,
            external_editor_for_messages: settings.external_editor_for_messages,
            proxy_mode: settings.proxy_mode,
            verify_ssl: settings.verify_ssl,
            use_system_git_config: settings.use_system_git_config,
            scan_folders: Vec::new(),
            scan_search_preferences: Vec::new(),
        };
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
struct AgentPreferencesSection {
    provider: AgentProviderPreferences,
    runtime: AgentRuntimePreferences,
    permissions: BTreeMap<String, String>,
}

impl Default for AgentPreferencesSection {
    fn default() -> Self {
        return AgentPreferencesSection {
            provider: AgentProviderPreferences::default(),
            runtime: AgentRuntimePreferences::default(),
            permissions: BTreeMap::new(),
        };
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
struct AgentProviderPreferences {
    protocol: String,
    base_url: String,
    model: String,
    temperature: f32,
    max_tokens: u32,
    send_diffs: bool,
    send_repository_path: bool,
}

impl Default for AgentProviderPreferences {
    fn default() -> Self {
        let settings = Settings::default();
        return AgentProviderPreferences {
            protocol: settings.agent_protocol,
            base_url: settings.agent_base_url,
            model: settings.agent_model,
            temperature: settings.agent_temperature,
            max_tokens: settings.agent_max_tokens,
            send_diffs: settings.agent_send_diffs,
            send_repository_path: settings.agent_send_repository_path,
        };
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
struct AgentRuntimePreferences {
    automation: String,
    review_millis: u32,
    protected_branches: Vec<String>,
}

impl Default for AgentRuntimePreferences {
    fn default() -> Self {
        return AgentRuntimePreferences {
            automation: "off".to_string(),
            review_millis: 8000,
            protected_branches: vec!["main".to_string()],
        };
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ShortcutPreference {
    id: String,
    combo: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ScanSearchPreference {
    path: String,
    enabled: bool,
}

/// Agent 的本地偏好。字符串 ID 与 `crates/agent` 保持解耦。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentPreferences {
    pub automation: String,
    pub review_millis: u32,
    pub protected_branches: Vec<String>,
    pub permissions: BTreeMap<String, String>,
}

impl Default for AgentPreferences {
    fn default() -> Self {
        return AgentPreferences {
            automation: "off".to_string(),
            review_millis: 8000,
            protected_branches: vec!["main".to_string()],
            permissions: BTreeMap::new(),
        };
    }
}

impl StoredConfig {
    fn settings(&self) -> Settings {
        return Settings {
            file_tree: self.general.file_tree,
            diff_split: self.general.diff_split,
            diff_fold: self.general.diff_fold,
            diff_wrap: self.general.diff_wrap,
            diff_whitespace: self.general.diff_whitespace,
            diff_ignore_whitespace: self.general.diff_ignore_whitespace,
            merge_empty_dirs: self.general.merge_empty_dirs,
            compact_single_child: self.general.compact_single_child,
            restore_session: self.general.restore_session,
            update_mode: self.general.update_mode.clone(),
            use_system_git: self.git.use_system_git,
            git_path: self.git.git_path.clone(),
            date_format: self.general.date_format.clone(),
            date_pattern: self.general.date_pattern.clone(),
            agent_protocol: self.agent.provider.protocol.clone(),
            agent_base_url: self.agent.provider.base_url.clone(),
            agent_model: self.agent.provider.model.clone(),
            agent_temperature: self.agent.provider.temperature,
            agent_max_tokens: self.agent.provider.max_tokens,
            agent_send_diffs: self.agent.provider.send_diffs,
            agent_send_repository_path: self.agent.provider.send_repository_path,
            ssh_for_new_remotes: self.git.ssh_for_new_remotes,
            external_editor: self.git.external_editor.clone(),
            diff_tool: self.git.diff_tool.clone(),
            merge_tool: self.git.merge_tool.clone(),
            terminal: self.git.terminal.clone(),
            external_editor_for_messages: self.git.external_editor_for_messages,
            proxy_mode: self.git.proxy_mode.clone(),
            verify_ssl: self.git.verify_ssl,
            use_system_git_config: self.git.use_system_git_config,
        };
    }

    fn set_settings(&mut self, settings: &Settings) {
        self.general.file_tree = settings.file_tree;
        self.general.diff_split = settings.diff_split;
        self.general.diff_fold = settings.diff_fold;
        self.general.diff_wrap = settings.diff_wrap;
        self.general.diff_whitespace = settings.diff_whitespace;
        self.general.diff_ignore_whitespace = settings.diff_ignore_whitespace;
        self.general.merge_empty_dirs = settings.merge_empty_dirs;
        self.general.compact_single_child = settings.compact_single_child;
        self.general.restore_session = settings.restore_session;
        self.general.update_mode = settings.update_mode.clone();
        self.general.date_format = settings.date_format.clone();
        self.general.date_pattern = settings.date_pattern.clone();

        self.git.use_system_git = settings.use_system_git;
        self.git.git_path = settings.git_path.clone();
        self.git.ssh_for_new_remotes = settings.ssh_for_new_remotes;
        self.git.external_editor = settings.external_editor.clone();
        self.git.diff_tool = settings.diff_tool.clone();
        self.git.merge_tool = settings.merge_tool.clone();
        self.git.terminal = settings.terminal.clone();
        self.git.external_editor_for_messages = settings.external_editor_for_messages;
        self.git.proxy_mode = settings.proxy_mode.clone();
        self.git.verify_ssl = settings.verify_ssl;
        self.git.use_system_git_config = settings.use_system_git_config;

        self.agent.provider.protocol = settings.agent_protocol.clone();
        self.agent.provider.base_url = settings.agent_base_url.clone();
        self.agent.provider.model = settings.agent_model.clone();
        self.agent.provider.temperature = settings.agent_temperature;
        self.agent.provider.max_tokens = settings.agent_max_tokens;
        self.agent.provider.send_diffs = settings.agent_send_diffs;
        self.agent.provider.send_repository_path = settings.agent_send_repository_path;
    }

    fn agent_preferences(&self) -> AgentPreferences {
        return AgentPreferences {
            automation: self.agent.runtime.automation.clone(),
            review_millis: self.agent.runtime.review_millis,
            protected_branches: self.agent.runtime.protected_branches.clone(),
            permissions: self.agent.permissions.clone(),
        };
    }

    fn set_agent_preferences(&mut self, preferences: AgentPreferences) {
        self.agent.runtime.automation = preferences.automation;
        self.agent.runtime.review_millis = preferences.review_millis;
        self.agent.runtime.protected_branches = preferences.protected_branches;
        self.agent.permissions = preferences.permissions;
    }
}

fn settings_file(dir: &Path) -> PathBuf {
    return dir.join("settings.toml");
}

fn read_config(dir: &Path) -> Option<StoredConfig> {
    let text = match std::fs::read_to_string(settings_file(dir)) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Some(StoredConfig::default());
        }
        Err(_) => return None,
    };
    let config: StoredConfig = toml::from_str(&text).ok()?;
    if config.schema_version != CONFIG_SCHEMA_VERSION {
        return None;
    }
    return Some(config);
}

fn write_config(dir: &Path, config: &StoredConfig) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let text = toml::to_string_pretty(config).map_err(std::io::Error::other)?;
    let file = settings_file(dir);
    let temporary = file.with_extension("toml.tmp");
    std::fs::write(&temporary, text)?;
    std::fs::rename(&temporary, file)?;
    return Ok(());
}

fn update_config(dir: &Path, update: impl FnOnce(&mut StoredConfig)) {
    let Some(mut config) = read_config(dir) else {
        return;
    };
    update(&mut config);
    let _ = write_config(dir, &config);
}

/// 读取设置；文件缺失时返回默认值。
pub fn load_settings(dir: &Path) -> Settings {
    return read_config(dir)
        .map(|config| config.settings())
        .unwrap_or_default();
}

/// 保存设置。
pub fn save_settings(dir: &Path, settings: &Settings) {
    update_config(dir, |config| config.set_settings(settings));
}

/// 读取 Agent 偏好；权限缺省时由 `crates/agent` 的策略默认值接管。
pub fn load_agent_preferences(dir: &Path) -> AgentPreferences {
    return read_config(dir)
        .map(|config| config.agent_preferences())
        .unwrap_or_default();
}

/// 保存 Agent 偏好。
pub fn save_agent_preferences(dir: &Path, preferences: AgentPreferences) {
    update_config(dir, |config| config.set_agent_preferences(preferences));
}

/// 修改 Agent 偏好的指定字段，同时保留其余配置分区。
pub fn update_agent_preferences(dir: &Path, update: impl FnOnce(&mut AgentPreferences)) {
    update_config(dir, |config| {
        let mut preferences = config.agent_preferences();
        update(&mut preferences);
        config.set_agent_preferences(preferences);
    });
}

/// 用于扫描仓库的目录列表，保存在 `settings.toml` 的 Git 分区。
pub fn load_git_scan_folders(dir: &Path) -> Vec<PathBuf> {
    return read_config(dir)
        .map(|config| {
            config
                .git
                .scan_folders
                .into_iter()
                .map(PathBuf::from)
                .collect()
        })
        .unwrap_or_default();
}

/// 保存扫描目录。
pub fn save_git_scan_folders(dir: &Path, folders: &[PathBuf]) {
    update_config(dir, |config| {
        config.git.scan_folders = folders
            .iter()
            .map(|folder| folder.display().to_string())
            .collect();
    });
}

/// 扫描结果是否允许进入命令面板搜索，保存在 `settings.toml` 的 Git 分区。
///
/// 文件缺失或某条路径没有显式记录时，由应用按默认启用处理。
pub fn load_git_scan_search_preferences(dir: &Path) -> Vec<(String, bool)> {
    return read_config(dir)
        .map(|config| {
            config
                .git
                .scan_search_preferences
                .into_iter()
                .map(|preference| (preference.path, preference.enabled))
                .collect()
        })
        .unwrap_or_default();
}

/// 保存扫描结果的命令面板搜索资格。
pub fn save_git_scan_search_preferences(dir: &Path, preferences: &[(String, bool)]) {
    update_config(dir, |config| {
        config.git.scan_search_preferences = preferences
            .iter()
            .filter(|(path, _)| !path.is_empty())
            .map(|(path, enabled)| ScanSearchPreference {
                path: path.clone(),
                enabled: *enabled,
            })
            .collect();
    });
}

/// 快捷键覆盖项；只保存与默认值不同的绑定。
///
/// 只存与默认值不同的项：默认值随版本演进时，没改过的动作自动跟上新默认。
pub fn load_shortcuts(dir: &Path) -> Vec<(String, String)> {
    return read_config(dir)
        .map(|config| {
            config
                .shortcuts
                .into_iter()
                .map(|shortcut| (shortcut.id, shortcut.combo))
                .collect()
        })
        .unwrap_or_default();
}

/// 保存快捷键覆盖项。
pub fn save_shortcuts(dir: &Path, overrides: &[(String, String)]) {
    update_config(dir, |config| {
        config.shortcuts = overrides
            .iter()
            .map(|(id, combo)| ShortcutPreference {
                id: id.clone(),
                combo: combo.clone(),
            })
            .collect();
    });
}

/// 在给定目录下找 Git 仓库（含 `.git` 的目录），最多下探三层。
///
/// 限深是刻意的：再深会扫到 `node_modules` 之类的树里，代价大而收益小。
/// 结果按路径排序，保证同样的输入得到同样的顺序。
pub fn scan_git_repositories(folders: &[PathBuf]) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for folder in folders {
        collect_repositories(folder, 0, &mut found);
    }
    found.sort();
    found.dedup();
    return found;
}

fn collect_repositories(folder: &Path, depth: usize, found: &mut Vec<String>) {
    if !folder.is_dir() {
        return;
    }
    if folder.join(".git").exists() {
        found.push(folder.display().to_string());
        // 仓库内部不再下探：嵌套仓库很少见，继续扫只会拖慢。
        return;
    }
    if depth >= 3 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    let mut children: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    children.sort();
    for child in children {
        // 跳过明显的噪音目录。
        let name = child
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        if name == "node_modules" || name == "target" || name.starts_with('.') {
            continue;
        }
        collect_repositories(child.as_path(), depth + 1, found);
    }
}

/// Agent 的审计日志：每行 `时间\t动作\t结果\t详情`。
pub fn load_agent_audit(dir: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(dir.join("agent-audit.txt")) else {
        return Vec::new();
    };
    return text.lines().map(|line| line.to_string()).collect();
}

/// 保存 Agent 的审计日志（最新的在前，调用方负责截断）。
pub fn save_agent_audit(dir: &Path, entries: &[String]) {
    let _ = write_lines(&dir.join("agent-audit.txt"), entries);
}

/// 读取外观偏好；未设置时返回 `None`（跟随系统）。
pub fn load_appearance(dir: &Path) -> Option<String> {
    let config = read_config(dir)?;
    let value = config.general.appearance.trim();
    if value.is_empty() {
        return None;
    }
    return Some(value.to_string());
}

/// 保存外观偏好。
pub fn save_appearance(dir: &Path, appearance: &str) {
    update_config(dir, |config| {
        config.general.appearance = appearance.to_string()
    });
}

fn recent_file(dir: &Path) -> PathBuf {
    return dir.join("recent.txt");
}

fn session_file(dir: &Path) -> PathBuf {
    return dir.join("session.txt");
}

/// 读取最近仓库；文件缺失或为空时返回空列表。
pub fn load_recent(dir: &Path) -> Vec<RecentRepo> {
    let text = match std::fs::read_to_string(recent_file(dir)) {
        Ok(text) => text,
        Err(_) => return Vec::new(),
    };
    let mut repos = Vec::new();
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        // 旧格式只有路径；随后是 `<时间>\t<路径>`；当前还会带可选的
        // `<时间>\t<路径>\t<分支>`，第三列缺失时分支为空。
        let (opened_at, path, branch) = match line.split_once('\t') {
            Some((timestamp, rest)) => {
                let (path, branch) = match rest.split_once('\t') {
                    Some((path, branch)) => (path, branch),
                    None => (rest, ""),
                };
                (timestamp.parse().unwrap_or(0), path, branch)
            }
            None => (0, line, ""),
        };
        if path.is_empty() {
            continue;
        }
        repos.push(RecentRepo {
            path: path.to_string(),
            opened_at,
            branch: branch.to_string(),
        });
    }
    repos.truncate(MAX_RECENT);
    return repos;
}

/// 当前 Unix 时间（秒）。用于记录打开时间，不依赖额外 crate。
pub fn now_unix() -> i64 {
    return std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
}

/// 写回最近仓库；分支为空时仍用两列格式，旧版本可以继续读。
fn write_recent(dir: &Path, repos: &[RecentRepo]) {
    let lines: Vec<String> = repos
        .iter()
        .map(|repo| {
            if repo.branch.is_empty() {
                format!("{}\t{}", repo.opened_at, repo.path)
            } else {
                format!("{}\t{}\t{}", repo.opened_at, repo.path, repo.branch)
            }
        })
        .collect();
    let _ = write_lines(&recent_file(dir), &lines);
}

/// 保存最近仓库；把 `path` 放到最前（时间记为现在），去重并截断。
///
/// 同一路径再次打开时保留之前记录的分支，时间更新为现在。
pub fn remember_recent(dir: &Path, repos: &[RecentRepo], path: &str) {
    let branch = repos
        .iter()
        .find(|repo| repo.path == path)
        .map(|repo| repo.branch.clone())
        .unwrap_or_default();
    let mut next = Vec::with_capacity(repos.len() + 1);
    next.push(RecentRepo {
        path: path.to_string(),
        opened_at: now_unix(),
        branch,
    });
    for existing in repos {
        if existing.path != path && !existing.path.is_empty() {
            next.push(existing.clone());
        }
    }
    next.truncate(MAX_RECENT);
    write_recent(dir, &next);
}

/// 从最近仓库里移除一条，返回剩余列表。
pub fn forget_recent(dir: &Path, repos: &[RecentRepo], path: &str) -> Vec<RecentRepo> {
    let next: Vec<RecentRepo> = repos
        .iter()
        .filter(|repo| repo.path != path)
        .cloned()
        .collect();
    write_recent(dir, &next);
    return next;
}

/// 记录某个最近仓库的分支，保持原来的打开时间。
///
/// 分支只在从仓库读到后写入；读取失败或路径已不在列表里时什么也不做。
pub fn set_recent_branch(dir: &Path, path: &str, branch: &str) {
    let mut repos = load_recent(dir);
    let mut changed = false;
    for repo in &mut repos {
        if repo.path == path && repo.branch != branch {
            repo.branch = branch.to_string();
            changed = true;
        }
    }
    if changed {
        write_recent(dir, &repos);
    }
}

/// 读取会话；文件缺失或格式不符时返回空会话。
pub fn load_session(dir: &Path) -> Session {
    let text = match std::fs::read_to_string(session_file(dir)) {
        Ok(text) => text,
        Err(_) => return Session::default(),
    };
    let mut lines = text.lines();
    let active: usize = lines
        .next()
        .and_then(|line| line.trim().parse().ok())
        .unwrap_or(0);
    let paths: Vec<String> = lines
        .map(|line| line.trim_end().to_string())
        .filter(|line| !line.is_empty())
        .collect();
    if paths.is_empty() {
        return Session::default();
    }
    return Session {
        active: active.min(paths.len() - 1),
        paths,
    };
}

/// 保存会话。
pub fn save_session(dir: &Path, session: &Session) {
    let mut lines = Vec::with_capacity(session.paths.len() + 1);
    lines.push(session.active.to_string());
    lines.extend(session.paths.iter().cloned());
    let _ = write_lines(&session_file(dir), &lines);
}

fn write_lines(file: &Path, lines: &[String]) -> std::io::Result<()> {
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut text = String::new();
    for line in lines {
        // 含换行的路径无法用逐行格式表达，直接跳过。
        if line.contains('\n') || line.contains('\r') {
            continue;
        }
        text.push_str(line);
        text.push('\n');
    }
    let temporary = file.with_extension("txt.tmp");
    std::fs::write(&temporary, text)?;
    std::fs::rename(&temporary, file)?;
    return Ok(());
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(name: &str) -> TempDir {
            let unique = format!(
                "store-test-{}-{}-{}",
                name,
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_nanos())
                    .unwrap_or(0)
            );
            let path = std::env::temp_dir().join(unique);
            std::fs::create_dir_all(&path).expect("temp dir");
            return TempDir { path };
        }

        fn path(&self) -> &Path {
            return &self.path;
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn recent_puts_the_latest_first_and_deduplicates() {
        let dir = TempDir::new("recent");
        remember_recent(dir.path(), &[], "/a");
        remember_recent(dir.path(), &load_recent(dir.path()), "/b");
        remember_recent(dir.path(), &load_recent(dir.path()), "/a");
        let paths: Vec<String> = load_recent(dir.path())
            .into_iter()
            .map(|repo| repo.path)
            .collect();
        assert_eq!(paths, vec!["/a".to_string(), "/b".to_string()]);
        assert!(load_recent(dir.path())[0].opened_at > 0);
    }

    #[test]
    fn recent_caps_the_list() {
        let dir = TempDir::new("cap");
        for index in 0..20 {
            let path = format!("/repo-{index}");
            let current = load_recent(dir.path());
            remember_recent(dir.path(), &current, &path);
        }
        let paths = load_recent(dir.path());
        assert_eq!(paths.len(), MAX_RECENT);
        assert_eq!(paths[0].path, "/repo-19");
    }

    #[test]
    fn agent_preferences_and_audit_round_trip() {
        let dir = TempDir::new("agent");
        assert_eq!(
            load_agent_preferences(dir.path()),
            AgentPreferences::default()
        );
        save_agent_preferences(
            dir.path(),
            AgentPreferences {
                automation: "commit".to_string(),
                review_millis: 5000,
                protected_branches: vec!["main".to_string(), "feature,preview".to_string()],
                permissions: BTreeMap::from([
                    ("push".to_string(), "deny".to_string()),
                    ("fetch".to_string(), "allow".to_string()),
                ]),
            },
        );
        update_agent_preferences(dir.path(), |preferences| {
            preferences
                .permissions
                .insert("commit".to_string(), "allow".to_string());
        });
        let loaded = load_agent_preferences(dir.path());
        assert_eq!(loaded.automation, "commit");
        assert_eq!(loaded.review_millis, 5000);
        assert_eq!(
            loaded.protected_branches,
            vec!["main".to_string(), "feature,preview".to_string()]
        );
        assert_eq!(
            loaded.permissions.get("push").map(String::as_str),
            Some("deny")
        );
        assert_eq!(
            loaded.permissions.get("commit").map(String::as_str),
            Some("allow")
        );

        update_agent_preferences(dir.path(), |preferences| {
            preferences.review_millis = 3000;
        });
        let loaded = load_agent_preferences(dir.path());
        assert_eq!(loaded.review_millis, 3000);
        assert_eq!(
            loaded.permissions.get("push").map(String::as_str),
            Some("deny")
        );

        save_agent_audit(dir.path(), &["1\tfetch\tok\t2 refs".to_string()]);
        assert_eq!(load_agent_audit(dir.path()).len(), 1);
        assert!(dir.path().join("agent-audit.txt").exists());
    }

    #[test]
    fn settings_round_trip_with_defaults() {
        let dir = TempDir::new("settings");
        assert_eq!(load_settings(dir.path()), Settings::default());
        save_settings(
            dir.path(),
            &Settings {
                file_tree: false,
                diff_split: true,
                diff_fold: true,
                diff_wrap: true,
                diff_whitespace: true,
                diff_ignore_whitespace: true,
                merge_empty_dirs: false,
                compact_single_child: false,
                restore_session: false,
                update_mode: "never".to_string(),
                use_system_git: false,
                git_path: "/opt/homebrew/bin/git".to_string(),
                date_format: "custom".to_string(),
                date_pattern: "%Y/%m/%d".to_string(),
                agent_protocol: "anthropic".to_string(),
                agent_base_url: "https://gateway.example".to_string(),
                agent_model: "claude-sonnet-4-5".to_string(),
                agent_temperature: 0.7,
                agent_max_tokens: 8192,
                agent_send_diffs: false,
                agent_send_repository_path: true,
                ..Settings::default()
            },
        );
        assert!(dir.path().join("settings.toml").exists());
        assert!(!dir.path().join("settings.txt").exists());
        let loaded = load_settings(dir.path());
        assert!(!loaded.file_tree);
        assert!(loaded.diff_split);
        assert!(loaded.diff_fold);
        assert!(loaded.diff_wrap);
        assert!(loaded.diff_whitespace);
        assert!(loaded.diff_ignore_whitespace);
        assert!(!loaded.restore_session);
        assert!(!loaded.merge_empty_dirs);
        assert!(!loaded.compact_single_child);
        assert!(Settings::default().merge_empty_dirs, "默认合并空目录");
        assert_eq!(loaded.update_mode, "never", "更新方式应持久化");
        assert_eq!(loaded.date_format, "custom");
        assert!(!loaded.use_system_git);
        assert_eq!(loaded.git_path, "/opt/homebrew/bin/git");
        assert_eq!(loaded.date_pattern, "%Y/%m/%d");
        assert_eq!(loaded.agent_protocol, "anthropic");
        assert_eq!(loaded.agent_base_url, "https://gateway.example");
        assert_eq!(loaded.agent_model, "claude-sonnet-4-5");
        assert_eq!(loaded.agent_temperature, 0.7);
        assert_eq!(loaded.agent_max_tokens, 8192);
        assert!(!loaded.agent_send_diffs);
        assert!(loaded.agent_send_repository_path);
        assert_eq!(Settings::default().update_mode, "auto");
    }

    #[test]
    fn partial_toml_uses_defaults_for_missing_fields() {
        let dir = TempDir::new("partial-toml");
        std::fs::write(
            dir.path().join("settings.toml"),
            "schema_version = 1\n\n[general]\nfile_tree = false\n",
        )
        .expect("write partial TOML");

        let loaded = load_settings(dir.path());
        assert!(!loaded.file_tree);
        assert_eq!(loaded.update_mode, Settings::default().update_mode);
        assert_eq!(loaded.diff_split, Settings::default().diff_split);
        assert_eq!(loaded.diff_fold, Settings::default().diff_fold);
        assert_eq!(loaded.diff_wrap, Settings::default().diff_wrap);
        assert_eq!(loaded.diff_whitespace, Settings::default().diff_whitespace);
        assert_eq!(
            loaded.diff_ignore_whitespace,
            Settings::default().diff_ignore_whitespace
        );
        assert_eq!(loaded.agent_model, Settings::default().agent_model);
    }

    #[test]
    fn preference_sections_share_one_file_without_clobbering_each_other() {
        let dir = TempDir::new("preference-sections");
        save_settings(
            dir.path(),
            &Settings {
                file_tree: false,
                git_path: "/tmp/git=工具/βin".to_string(),
                ..Settings::default()
            },
        );
        save_language(dir.path(), "zh-Hans");
        save_appearance(dir.path(), "dark");
        save_shortcuts(
            dir.path(),
            &[("commit".to_string(), "mod+shift+c".to_string())],
        );
        let scan_folder = PathBuf::from("/work/βeta,=directory\t1");
        save_git_scan_folders(dir.path(), std::slice::from_ref(&scan_folder));
        save_git_scan_search_preferences(dir.path(), &[("/work/repo,=β\t1".to_string(), false)]);
        update_agent_preferences(dir.path(), |preferences| {
            preferences.automation = "commit".to_string();
            preferences
                .permissions
                .insert("push".to_string(), "deny".to_string());
        });

        assert!(!load_settings(dir.path()).file_tree);
        assert_eq!(load_settings(dir.path()).git_path, "/tmp/git=工具/βin");
        assert_eq!(load_language(dir.path()).as_deref(), Some("zh-Hans"));
        assert_eq!(load_appearance(dir.path()).as_deref(), Some("dark"));
        assert_eq!(
            load_shortcuts(dir.path()),
            vec![("commit".to_string(), "mod+shift+c".to_string())]
        );
        assert_eq!(load_git_scan_folders(dir.path()), vec![scan_folder]);
        assert_eq!(
            load_git_scan_search_preferences(dir.path()),
            vec![("/work/repo,=β\t1".to_string(), false)]
        );
        let agent = load_agent_preferences(dir.path());
        assert_eq!(agent.automation, "commit");
        assert_eq!(
            agent.permissions.get("push").map(String::as_str),
            Some("deny")
        );

        let text =
            std::fs::read_to_string(dir.path().join("settings.toml")).expect("read TOML config");
        for section in [
            "[general]",
            "[git]",
            "[shortcuts]",
            "[agent.provider]",
            "[agent.runtime]",
            "[agent.permissions]",
        ] {
            assert!(text.contains(section), "missing {section}: {text}");
        }
    }

    #[test]
    fn malformed_or_newer_toml_is_preserved_and_not_overwritten() {
        let dir = TempDir::new("invalid-toml");
        for text in [
            "schema_version = [\n",
            "schema_version = 2\n",
            "schema_version = 1\n\n[general]\nunknown = true\n",
            "[general]\nfile_tree = false\n",
        ] {
            let file = dir.path().join("settings.toml");
            std::fs::write(&file, text).expect("write invalid TOML");
            assert_eq!(load_settings(dir.path()), Settings::default());
            save_appearance(dir.path(), "dark");
            assert_eq!(std::fs::read_to_string(file).expect("read TOML"), text);
        }
    }

    #[test]
    fn legacy_preference_text_files_are_ignored_and_left_untouched() {
        let dir = TempDir::new("legacy-preferences");
        let legacy = [
            (
                "settings.txt",
                "file_tree=false\nagent_model=legacy-model\n",
            ),
            ("language.txt", "zh-Hans\n"),
            ("appearance.txt", "dark\n"),
            ("shortcuts.txt", "push=mod+shift+p\n"),
            ("agent-policies.txt", "push=deny\nautomation=commit\n"),
            ("git-scan-folders.txt", "/work\n"),
            ("git-scan-repositories.txt", "-/work/repo\n"),
        ];
        for (name, contents) in legacy {
            std::fs::write(dir.path().join(name), contents).expect("write legacy preference");
        }

        assert_eq!(load_settings(dir.path()), Settings::default());
        assert_eq!(load_language(dir.path()), None);
        assert_eq!(load_appearance(dir.path()), None);
        assert!(load_shortcuts(dir.path()).is_empty());
        assert_eq!(
            load_agent_preferences(dir.path()),
            AgentPreferences::default()
        );
        assert!(load_git_scan_folders(dir.path()).is_empty());
        assert!(load_git_scan_search_preferences(dir.path()).is_empty());

        save_language(dir.path(), "en");
        save_settings(
            dir.path(),
            &Settings {
                file_tree: false,
                ..Settings::default()
            },
        );
        assert!(dir.path().join("settings.toml").exists());
        for (name, contents) in legacy {
            assert_eq!(
                std::fs::read_to_string(dir.path().join(name)).expect("read legacy preference"),
                contents
            );
        }
    }

    #[test]
    fn appearance_round_trips() {
        let dir = TempDir::new("appearance");
        assert_eq!(load_appearance(dir.path()), None);
        save_appearance(dir.path(), "dark");
        assert_eq!(load_appearance(dir.path()), Some("dark".to_string()));
    }

    #[test]
    fn language_round_trips() {
        let dir = TempDir::new("language");
        assert_eq!(load_language(dir.path()), None);
        save_language(dir.path(), "zh-Hans");
        assert_eq!(load_language(dir.path()), Some("zh-Hans".to_string()));
    }

    #[test]
    fn changelists_round_trip_per_repository() {
        let dir = TempDir::new("changelists");
        let lists = vec![
            Changelist {
                name: "Commit composer".to_string(),
                paths: vec!["src/a.ts".to_string(), "src/b.ts".to_string()],
            },
            Changelist {
                name: "Agent cleanup".to_string(),
                paths: vec!["src/agent.ts".to_string()],
            },
            Changelist {
                name: "Empty list".to_string(),
                paths: Vec::new(),
            },
        ];
        save_changelists(dir.path(), "/repo-one", &lists);
        assert_eq!(load_changelists(dir.path(), "/repo-one"), lists);

        // 另一个仓库有自己的清单，两者互不影响。
        save_changelists(
            dir.path(),
            "/repo-two",
            &[Changelist {
                name: "Other".to_string(),
                paths: vec!["README.md".to_string()],
            }],
        );
        assert_eq!(load_changelists(dir.path(), "/repo-one"), lists);
        assert_eq!(load_changelists(dir.path(), "/repo-two").len(), 1);
    }

    #[test]
    fn missing_files_fall_back_to_empty_state() {
        let dir = TempDir::new("missing");
        assert!(load_recent(dir.path()).is_empty());
        assert_eq!(load_session(dir.path()), Session::default());
    }

    #[test]
    fn session_round_trips_and_clamps_active() {
        let dir = TempDir::new("session");
        save_session(
            dir.path(),
            &Session {
                active: 9,
                paths: vec!["/a".to_string(), "/b".to_string()],
            },
        );
        let loaded = load_session(dir.path());
        assert_eq!(loaded.active, 1);
        assert_eq!(loaded.paths, vec!["/a".to_string(), "/b".to_string()]);
    }

    #[test]
    fn recent_reads_old_two_column_lines() {
        let dir = TempDir::new("recent-old");
        std::fs::write(dir.path().join("recent.txt"), "100\t/repo\n").expect("write");
        let repos = load_recent(dir.path());
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].branch, "");
    }

    #[test]
    fn branch_round_trips_without_changing_the_time() {
        let dir = TempDir::new("recent-branch");
        remember_recent(dir.path(), &[], "/repo");
        let before = load_recent(dir.path())[0].opened_at;
        set_recent_branch(dir.path(), "/repo", "feature/hero");
        let after = load_recent(dir.path());
        assert_eq!(after[0].branch, "feature/hero");
        assert_eq!(after[0].opened_at, before);
    }

    #[test]
    fn remembering_again_keeps_the_branch() {
        let dir = TempDir::new("recent-keep-branch");
        remember_recent(dir.path(), &[], "/repo");
        set_recent_branch(dir.path(), "/repo", "main");
        remember_recent(dir.path(), &load_recent(dir.path()), "/repo");
        assert_eq!(load_recent(dir.path())[0].branch, "main");
    }

    #[test]
    fn default_dir_lives_under_dot_config() {
        assert_eq!(
            config_dir(Path::new("/Users/mia"), "gitui"),
            PathBuf::from("/Users/mia/.config/gitui")
        );
    }

    #[test]
    fn forgetting_one_recent_keeps_the_others() {
        let dir = TempDir::new("forget");
        remember_recent(dir.path(), &[], "/a");
        remember_recent(dir.path(), &load_recent(dir.path()), "/b");
        let next = forget_recent(dir.path(), &load_recent(dir.path()), "/a");
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].path, "/b");
        assert_eq!(load_recent(dir.path())[0].path, "/b");
    }

    #[test]
    fn scan_finds_repositories_two_levels_down() {
        let root = TempDir::new("scan");
        // 根目录自己就是仓库。
        std::fs::create_dir_all(root.path().join("own/.git")).expect("own");
        // 一层深的仓库。
        std::fs::create_dir_all(root.path().join("one/repo/.git")).expect("one");
        // 两层深与三层深的仓库都应找到。
        std::fs::create_dir_all(root.path().join("a/b/repo/.git")).expect("two");
        // 四层深：默认不扫。
        std::fs::create_dir_all(root.path().join("x/y/z/repo/.git")).expect("three");
        // 噪音目录不扫。
        std::fs::create_dir_all(root.path().join("node_modules/pkg/.git")).expect("noise");

        let found = scan_git_repositories(&[root.path().to_path_buf()]);
        assert_eq!(found.len(), 3, "应找到 3 个仓库：{found:?}");
        assert!(found.iter().any(|path| path.ends_with("/own")));
        assert!(found.iter().any(|path| path.ends_with("/one/repo")));
        assert!(found.iter().any(|path| path.ends_with("/a/b/repo")));
        assert!(!found.iter().any(|path| path.contains("node_modules")));
        assert!(!found.iter().any(|path| path.contains("/x/y/z/")));

        // 顺序稳定，便于断言与截图。
        let again = scan_git_repositories(&[root.path().to_path_buf()]);
        assert_eq!(found, again, "同样输入应得到同样顺序");
    }

    #[test]
    fn scan_folders_persist() {
        let dir = TempDir::new("scan-folders");
        assert!(load_git_scan_folders(dir.path()).is_empty());
        let folders = vec![PathBuf::from("/tmp/a"), PathBuf::from("/tmp/b")];
        save_git_scan_folders(dir.path(), &folders);
        assert_eq!(load_git_scan_folders(dir.path()), folders);
    }

    #[test]
    fn scan_search_preferences_persist_with_toml_escaping() {
        let dir = TempDir::new("scan-search-preferences");
        assert!(load_git_scan_search_preferences(dir.path()).is_empty());
        let preferences = vec![
            ("/work/alpha".to_string(), true),
            ("/work/βeta,=one\tunit".to_string(), false),
        ];
        save_git_scan_search_preferences(dir.path(), &preferences);
        assert_eq!(load_git_scan_search_preferences(dir.path()), preferences);
    }

    #[test]
    fn shortcuts_persist_as_overrides() {
        let dir = TempDir::new("shortcuts");
        assert!(load_shortcuts(dir.path()).is_empty());
        let overrides = vec![
            ("push".to_string(), "mod+shift+e".to_string()),
            ("commit".to_string(), "mod+shift+b".to_string()),
        ];
        save_shortcuts(dir.path(), &overrides);
        assert_eq!(load_shortcuts(dir.path()), overrides);
        // 空列表要真的清空文件（恢复默认后再存一次）。
        save_shortcuts(dir.path(), &[]);
        assert!(load_shortcuts(dir.path()).is_empty());
    }
}
