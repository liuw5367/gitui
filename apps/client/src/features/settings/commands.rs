//! 设置页的用例：主题、语言、日期格式、Git 可执行文件、启动行为、更新方式，
//! 以及快捷键的改键与恢复默认。
//!
//! 方法体由原 `Actions`（与 `lib.rs` 里的闭包）原样搬来，只把捕获的局部变量换成
//! `self.*`；返回欢迎页这类会话动作走 `SessionHandle` 的方法。

use super::state::{Section, SettingsState};
use crate::brand::ThemeMode;
use crate::features::SessionHandle;
use crate::i18n;
use crate::shared::diff::DiffCommands;
use nsh_core::prelude::*;
use std::path::{Path, PathBuf};

/// 按持久化状态载入当前扫描结果；新发现的仓库默认允许命令面板搜索。
pub(crate) fn load_scan_search_selection(store_dir: &Path, found: &[String]) -> Vec<String> {
    let mut preferences = store::load_git_scan_search_preferences(store_dir);
    let selected = merge_scan_search_selection(found, &mut preferences);
    store::save_git_scan_search_preferences(store_dir, &preferences);
    return selected;
}

fn merge_scan_search_selection(
    found: &[String],
    preferences: &mut Vec<(String, bool)>,
) -> Vec<String> {
    let mut selected = Vec::new();
    for path in found {
        if let Some((_, enabled)) = preferences.iter().find(|(known, _)| known == path) {
            if *enabled {
                selected.push(path.clone());
            }
        } else {
            preferences.push((path.clone(), true));
            selected.push(path.clone());
        }
    }
    return selected;
}

#[derive(Clone)]
pub(crate) struct SettingsCommands {
    pub(crate) state: SettingsState,
    pub(crate) session: SessionHandle,
    /// 界面语言：切换语言与错误文案都按它取。
    pub(crate) language: Signal<SharedString>,
    /// 状态栏短提示（探测 Git、扫描仓库、恢复快捷键）。
    pub(crate) notice: Signal<Option<String>>,
    /// 设置、语言、快捷键与外观的落盘目录。
    pub(crate) store_dir: PathBuf,
    /// 外观模式：设置页的主题选择直接改它。
    pub(crate) theme_mode: Signal<ThemeMode>,
    /// diff 的显示偏好（文件树分组）由 `shared::diff` 持有。
    pub(crate) diff: DiffCommands,
}

impl SettingsCommands {
    /// 切换外观（原 `set_theme`）：立即生效并持久化。
    pub(crate) fn set_theme(&self, mode: ThemeMode) {
        self.theme_mode.set(mode);
        nsh_core::element::set_color_scheme(mode.appearance());
        store::save_appearance(&self.store_dir, mode.id());
    }

    /// 打开设置窗口（独立窗口，由 `settings_open` 驱动）。
    pub(crate) fn go_settings(&self) {
        self.refresh_identity();
        self.session.settings_open.set(true);
    }

    /// 进入设置页的指定分区；欢迎页的 Settings 与 Credentials 入口用它。
    pub(crate) fn go_settings_section(&self, section: Section) {
        self.state.settings_section.set(section);
        self.go_settings();
    }

    /// 设置窗口打开前把当前仓库的身份读进编辑信号。
    pub(crate) fn refresh_identity(&self) {
        let (name, email) = match self.current_tab() {
            Some(tab) => crate::app::ops::identity(&tab),
            None => global_identity(),
        };
        self.state
            .identity_use_global
            .set(name.is_empty() && email.is_empty());
        self.state.identity_name.set(name);
        self.state.identity_email.set(email);
    }

    fn current_tab(&self) -> Option<crate::app::session::Tab> {
        return self
            .session
            .tabs
            .get_untracked()
            .get(self.session.active.get_untracked())
            .cloned();
    }

    /// 写仓库级身份；两个字段都为空时改为清空、继承全局。
    pub(crate) fn save_identity(&self, name: String, email: String) {
        let name = name.trim().to_string();
        let email = email.trim().to_string();
        if name.is_empty() && email.is_empty() {
            self.clear_identity();
            return;
        }
        if name.is_empty() || email.is_empty() {
            crate::interactions::notify(
                self.notice,
                i18n::t(
                    self.language.get_untracked().as_str(),
                    "settings.identityIncomplete",
                ),
            );
            return;
        }
        let Some(tab) = self.current_tab() else {
            crate::interactions::notify(
                self.notice,
                i18n::t(
                    self.language.get_untracked().as_str(),
                    "settings.identityNoRepo",
                ),
            );
            return;
        };
        if crate::app::ops::identity(&tab) == (name.clone(), email.clone()) {
            return;
        }
        match crate::features::search::ops::set_identity(&tab, name.as_str(), email.as_str()) {
            Ok(()) => {
                self.state.identity_use_global.set(false);
                self.refresh_identity();
            }
            Err(detail) => crate::interactions::notify(self.notice, detail),
        }
    }

    /// 清空仓库级身份，回到全局配置。
    pub(crate) fn clear_identity(&self) {
        let Some(tab) = self.current_tab() else {
            return;
        };
        match crate::features::refs::ops::clear_identity(&tab) {
            Ok(()) => {
                self.state.identity_use_global.set(true);
                self.refresh_identity();
            }
            Err(detail) => crate::interactions::notify(self.notice, detail),
        }
    }

    /// 勾选 / 取消「使用全局身份」。勾选时清空仓库级值。
    pub(crate) fn set_use_global_identity(&self, enabled: bool) {
        if enabled {
            self.clear_identity();
        } else {
            self.state.identity_use_global.set(false);
        }
    }

    /// 立即检查更新：没有发布源，给出事实性状态说明。
    pub(crate) fn check_updates(&self) {
        crate::interactions::notify(
            self.notice,
            i18n::t(
                self.language.get_untracked().as_str(),
                "settings.updateCheckUnavailable",
            ),
        );
    }

    /// 直接选一种语言（下拉用）；与 `toggle_language` 共用持久化。
    pub(crate) fn set_language(&self, next: &'static str) {
        self.language.set(SharedString::from(next));
        store::save_language(&self.store_dir, next);
    }

    /// 在英文与中文之间切换（原 `toggle_language`）。
    pub(crate) fn toggle_language(&self) {
        let next = i18n::toggle(self.language.get_untracked().as_str());
        self.language.set(SharedString::from(next));
        store::save_language(&self.store_dir, next);
    }

    /// 把 SSH 指纹或路径复制到剪贴板。
    pub(crate) fn copy_ssh_key(&self, text: String) {
        crate::shared::os::copy_text(self.notice, self.language, text);
    }

    /// 设置页里未接线的系统动作：只给事实性提示。
    pub(crate) fn settings_notice(&self, key: &'static str) {
        crate::interactions::notify(
            self.notice,
            i18n::t(self.language.get_untracked().as_str(), key),
        );
    }

    /// 修改任意设置字段并持久化；设置页的通用写入口。
    pub(crate) fn update_settings(&self, update: impl FnOnce(&mut store::Settings)) {
        let settings_state = self.state.settings_state;
        let mut next = settings_state.get_untracked();
        update(&mut next);
        settings_state.set(next.clone());
        store::save_settings(&self.store_dir, &next);
    }

    /// 测试 Agent 连接：没有 provider 运行时，给出事实性说明。
    pub(crate) fn test_agent_connection(&self) {
        crate::interactions::notify(
            self.notice,
            i18n::t(
                self.language.get_untracked().as_str(),
                "settings.agentTestUnavailable",
            ),
        );
    }

    /// 打开 Agent 日志：当前构建没有 Agent 运行时日志。
    pub(crate) fn open_agent_log(&self) {
        crate::interactions::notify(
            self.notice,
            i18n::t(
                self.language.get_untracked().as_str(),
                "settings.agentLogUnavailable",
            ),
        );
    }

    /// 选择 Git 可执行文件；选完写设置并用它探测一次。
    pub(crate) fn choose_git(&self) {
        let pending = pick_paths(nsh_core::file_dialog::OpenDialog::files());
        let this = self.clone();
        spawn(async move {
            let Ok(Some(paths)) = pending.await else {
                return;
            };
            if let Some(path) = paths.into_iter().next() {
                this.set_git_path(path.display().to_string());
            }
        });
    }

    /// 保存手动输入的 Git 路径，并立即探测它。
    pub(crate) fn set_git_path(&self, path: String) {
        let path = path.trim().to_string();
        let settings_state = self.state.settings_state;
        let store_dir = self.store_dir.clone();
        let mut next = settings_state.get_untracked();
        next.git_path = path.clone();
        next.use_system_git = false;
        settings_state.set(next.clone());
        store::save_settings(&store_dir, &next);
        self.state.git_path.set(path.clone());
        self.probe_git(Some(path));
    }

    /// 切换「使用 PATH 里的 Git」。
    pub(crate) fn set_use_system_git(&self, enabled: bool) {
        let settings_state = self.state.settings_state;
        let store_dir = self.store_dir.clone();
        let mut next = settings_state.get_untracked();
        next.use_system_git = enabled;
        settings_state.set(next.clone());
        store::save_settings(&store_dir, &next);
        let configured = self.state.git_path.get_untracked();
        self.probe_git(if enabled { None } else { Some(configured) });
    }

    /// 自动探测 Git：PATH 或设置里的路径，结果显示在 Git 分区。
    pub(crate) fn detect_git(&self) {
        let settings = self.state.settings_state.get_untracked();
        let configured = self.state.git_path.get_untracked();
        let path = if settings.use_system_git || configured.trim().is_empty() {
            None
        } else {
            Some(configured)
        };
        self.probe_git(path);
    }

    fn probe_git(&self, path: Option<String>) {
        let language = self.language.get_untracked();
        let result = match path {
            Some(path) => std::process::Command::new(path.as_str())
                .arg("--version")
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|output| {
                    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    (version, path)
                }),
            None => git::info().ok().map(|info| (info.version, info.exec_path)),
        };
        match result {
            Some((version, exec)) => {
                self.state.git_version.set(version);
                self.state.git_exec_path.set(exec);
            }
            None => {
                self.state.git_version.set(String::new());
                crate::interactions::notify(
                    self.notice,
                    i18n::t(language.as_str(), "settings.detectFailed"),
                );
            }
        }
    }

    /// 添加扫描目录；目录写入后立即重新扫描。
    pub(crate) fn add_scan_folder(&self) {
        let pending = pick_paths(nsh_core::file_dialog::OpenDialog::folders());
        let this = self.clone();
        spawn(async move {
            let Ok(Some(paths)) = pending.await else {
                return;
            };
            let mut folders = this.state.scan_folders.get_untracked();
            for path in paths {
                if !folders.contains(&path) {
                    folders.push(path);
                }
            }
            store::save_git_scan_folders(&this.store_dir, &folders);
            this.state.scan_folders.set(folders);
            this.scan_now();
        });
    }

    /// 移除一个扫描目录并重新扫描。
    pub(crate) fn remove_scan_folder(&self, path: String) {
        let mut folders = self.state.scan_folders.get_untracked();
        folders.retain(|folder| folder.display().to_string() != path);
        store::save_git_scan_folders(&self.store_dir, &folders);
        self.state.scan_folders.set(folders);
        self.scan_now();
    }

    /// 逐目录扫描，更新每个目录的计数、聚合结果与上次扫描时间。
    pub(crate) fn scan_now(&self) {
        let folders = self.state.scan_folders.get_untracked();
        let counts: Vec<usize> = folders
            .iter()
            .map(|folder| store::scan_git_repositories(std::slice::from_ref(folder)).len())
            .collect();
        let found = store::scan_git_repositories(&folders);
        let selected = load_scan_search_selection(&self.store_dir, &found);
        self.state.scan_counts.set(counts);
        self.state.scan_selected.set(selected);
        self.state.scan_found.set(found);
        self.state.scan_last_at.set(if folders.is_empty() {
            None
        } else {
            Some(store::now_unix())
        });
    }

    /// 勾选 / 取消一个扫描结果。
    pub(crate) fn toggle_found(&self, path: String) {
        let mut selected = self.state.scan_selected.get_untracked();
        if let Some(index) = selected.iter().position(|item| item == &path) {
            selected.remove(index);
        } else {
            selected.push(path);
        }
        self.state.scan_selected.set(selected);
        self.persist_scan_search_selection();
    }

    /// 全选 / 全不选扫描结果。
    pub(crate) fn select_all_found(&self, selected: bool) {
        if selected {
            self.state
                .scan_selected
                .set(self.state.scan_found.get_untracked());
        } else {
            self.state.scan_selected.set(Vec::new());
        }
        self.persist_scan_search_selection();
    }

    /// 只更新当前扫描结果的搜索资格，保留已移出扫描目录的历史选择。
    fn persist_scan_search_selection(&self) {
        let found = self.state.scan_found.get_untracked();
        let selected = self.state.scan_selected.get_untracked();
        let mut preferences = store::load_git_scan_search_preferences(&self.store_dir);
        for path in found {
            let enabled = selected.iter().any(|item| item == &path);
            if let Some((_, saved)) = preferences.iter_mut().find(|(known, _)| known == &path) {
                *saved = enabled;
            } else {
                preferences.push((path, enabled));
            }
        }
        store::save_git_scan_search_preferences(&self.store_dir, &preferences);
    }

    /// 设置启动方式（原 `set_launch_restore`）：true = 恢复上次会话。
    pub(crate) fn set_launch_restore(&self, restore: bool) {
        let settings_state = self.state.settings_state;
        let store_dir = self.store_dir.clone();
        let mut next = settings_state.get_untracked();
        next.restore_session = restore;
        settings_state.set(next.clone());
        store::save_settings(&store_dir, &next);
    }

    /// 切换日期显示方式（原 `set_date_style`）：改设置立刻生效并持久化。
    pub(crate) fn set_date_style(&self, style: crate::i18n::DateStyle) {
        let settings_state = self.state.settings_state;
        let store_dir = self.store_dir.clone();
        crate::dates::set_style(style);
        let mut next = settings_state.get_untracked();
        next.date_format = style.id().to_string();
        settings_state.set(next.clone());
        store::save_settings(&store_dir, &next);
    }

    /// 设置自定义日期 pattern（原 `set_date_pattern`）：同时切到 Custom。
    pub(crate) fn set_date_pattern(&self, pattern: String) {
        self.state.date_pattern.set(pattern.clone());
        let settings_state = self.state.settings_state;
        let store_dir = self.store_dir.clone();
        crate::dates::set_pattern(pattern.as_str());
        let mut next = settings_state.get_untracked();
        next.date_pattern = pattern;
        // 输入 pattern 时自动切到 Custom，否则用户改了看不到效果。
        next.date_format = crate::i18n::DateStyle::Custom.id().to_string();
        crate::dates::set_style(crate::i18n::DateStyle::Custom);
        settings_state.set(next.clone());
        store::save_settings(&store_dir, &next);
    }

    /// 设置更新检查方式（原 `set_update_mode`）。
    pub(crate) fn set_update_mode(&self, mode: String) {
        let settings_state = self.state.settings_state;
        let store_dir = self.store_dir.clone();
        let mut next = settings_state.get_untracked();
        next.update_mode = mode;
        settings_state.set(next.clone());
        store::save_settings(&store_dir, &next);
    }

    /// 接受设置页录制出的组合；冲突时保留录制状态并显示错误。
    pub(crate) fn rebind_shortcut(&self, id: &'static str, combo: String) {
        let registry = self.state.shortcut_registry;
        let recording = self.state.recording_shortcut;
        let error = self.state.shortcut_error;
        let store_dir = self.store_dir.clone();
        let language = self.language;
        // 回显按下的组合：成功时录制结束、卡片收起；失败时留在卡片里给用户看。
        self.state.pending_keystroke.set(Some(combo.clone()));
        let mut next = registry.get_untracked();
        let outcome = next.rebind(id, combo.as_str());
        let lang = language.get_untracked();
        match outcome {
            Ok(()) => {
                registry.set(next.clone());
                store::save_shortcuts(&store_dir, &next.overrides());
                error.set(None);
                recording.set(None);
                self.state.pending_keystroke.set(None);
            }
            // 失败时保持录制状态，用户可以直接改按另一个组合。
            Err(crate::shortcuts::BindError::NotBindable) => {
                error.set(Some(i18n::t(lang.as_str(), "shortcut.notBindable")));
            }
            Err(crate::shortcuts::BindError::Conflict(other)) => {
                let action = crate::shortcuts::ACTIONS
                    .iter()
                    .find(|candidate| candidate.id == other);
                let name = action
                    .map(|action| crate::shortcuts::label(action, lang.as_str()))
                    .unwrap_or_else(|| other.to_string());
                error.set(Some(i18n::with_arguments(
                    lang.as_str(),
                    "shortcut.conflict",
                    &[("action", name)],
                )));
            }
        }
    }

    /// 解绑一个动作。
    pub(crate) fn clear_shortcut(&self, id: &'static str) {
        let registry = self.state.shortcut_registry;
        let mut next = registry.get_untracked();
        next.clear(id);
        registry.set(next.clone());
        store::save_shortcuts(&self.store_dir, &next.overrides());
        self.state.recording_shortcut.set(None);
        self.state.shortcut_error.set(None);
        self.state.pending_keystroke.set(None);
    }

    /// 开始录制某个动作的新组合（原 `record_shortcut`）。
    pub(crate) fn record_shortcut(&self, id: &'static str) {
        self.state.shortcut_error.set(None);
        self.state.pending_keystroke.set(None);
        self.state.recording_shortcut.set(Some(id));
        // 闪烁由设置窗口内容里的定时器驱动（`app::hotkeys::install_settings_window`）：
        // 在这里 spawn 会随录制态重建的子树一起被取消。
    }

    /// 取消录制（Esc）：不改任何绑定，只退出录制态并清掉上一条冲突提示。
    pub(crate) fn cancel_recording(&self) {
        self.state.recording_shortcut.set(None);
        self.state.shortcut_error.set(None);
        self.state.pending_keystroke.set(None);
    }

    pub(crate) fn reset_shortcut(&self, id: &'static str) {
        let registry = self.state.shortcut_registry;
        let mut next = registry.get_untracked();
        let default = crate::shortcuts::ACTIONS
            .iter()
            .find(|action| action.id == id)
            .map(|action| action.default);
        next.clear(id);
        if let Some(default) = default
            && !default.is_empty()
        {
            let _ = next.rebind(id, default);
        }
        registry.set(next.clone());
        store::save_shortcuts(&self.store_dir, &next.overrides());
        self.state.recording_shortcut.set(None);
        self.state.shortcut_error.set(None);
    }

    /// 恢复全部默认快捷键。确认弹层由设置页自己画（`view.rs`），这里只执行。
    pub(crate) fn restore_shortcuts(&self) {
        let registry = self.state.shortcut_registry;
        let mut next = registry.get_untracked();
        next.restore_defaults();
        registry.set(next.clone());
        store::save_shortcuts(&self.store_dir, &next.overrides());
        self.state.recording_shortcut.set(None);
        self.state.shortcut_error.set(None);
        self.state.restore_confirm.set(false);
        let done = i18n::t(self.language.get_untracked().as_str(), "shortcut.restored");
        crate::interactions::notify(self.notice, done);
    }
}

#[cfg(test)]
mod scan_search_tests {
    use super::*;

    #[test]
    fn scan_search_selection_keeps_choices_and_enables_new_repositories() {
        let found = vec![
            "/scan/alpha".to_string(),
            "/scan/beta".to_string(),
            "/scan/gamma".to_string(),
        ];
        let mut preferences = vec![
            ("/scan/alpha".to_string(), false),
            ("/scan/beta".to_string(), true),
            ("/removed/old".to_string(), false),
        ];

        let selected = merge_scan_search_selection(&found, &mut preferences);

        assert_eq!(selected, vec!["/scan/beta", "/scan/gamma"]);
        assert_eq!(preferences[2], ("/removed/old".to_string(), false));
        assert_eq!(preferences[3], ("/scan/gamma".to_string(), true));
    }
}

/// 没有打开仓库时编辑框显示的全局身份。
fn global_identity() -> (String, String) {
    let read = |key: &str| {
        return std::process::Command::new("git")
            .args(["config", "--get", key])
            .output()
            .ok()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_default();
    };
    return (read("user.name"), read("user.email"));
}
