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
use std::path::PathBuf;

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
    /// 本机 Git 的版本、执行路径与凭据助手：启动时探测一次，设置页只读展示。
    pub(crate) git_info: (String, String, String),
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
        self.session.settings_open.set(true);
    }

    /// 进入设置页的指定分区；欢迎页的 Settings 与 Credentials 入口用它。
    pub(crate) fn go_settings_section(&self, section: Section) {
        self.state.settings_section.set(section);
        self.go_settings();
    }

    /// 在英文与中文之间切换（原 `toggle_language`）。
    pub(crate) fn toggle_language(&self) {
        let next = i18n::toggle(self.language.get_untracked().as_str());
        self.language.set(SharedString::from(next));
        store::save_language(&self.store_dir, next);
    }

    /// 选择 Git 可执行文件（原 `choose_git`）：选完立刻用它探测一次。
    pub(crate) fn choose_git(&self) {
        let notice = self.notice;
        let language = self.language;
        // 只让选可执行文件；选完把路径记进设置并立刻用它探测一次。
        let pending = pick_paths(nsh_core::file_dialog::OpenDialog::files());
        spawn(async move {
            let Ok(Some(paths)) = pending.await else {
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            let lang = language.get_untracked();
            match std::process::Command::new(&path).arg("--version").output() {
                Ok(output) if output.status.success() => {
                    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    crate::interactions::notify(notice, format!("{version} · {}", path.display()));
                }
                _ => {
                    crate::interactions::notify(
                        notice,
                        i18n::t(lang.as_str(), "settings.detectFailed"),
                    );
                }
            }
        });
    }

    /// 扫描已配置目录里的仓库并加入最近列表（原 `add_found_repos`）。
    pub(crate) fn add_found_repos(&self) {
        let store_dir = self.store_dir.clone();
        let recent = self.session.recent;
        let notice = self.notice;
        let language = self.language;
        let lang = language.get_untracked();
        let folders = store::load_git_scan_folders(&store_dir);
        let found = store::scan_git_repositories(&folders);
        if found.is_empty() {
            crate::interactions::notify(notice, i18n::t(lang.as_str(), "settings.scanEmpty"));
            return;
        }
        let mut list = recent.get_untracked();
        for path in &found {
            store::remember_recent(&store_dir, &list, path.as_str());
            list = store::load_recent(&store_dir);
        }
        recent.set(list);
        crate::interactions::notify(
            notice,
            i18n::with_arguments(
                lang.as_str(),
                "settings.scanAdded",
                &[("count", found.len().to_string())],
            ),
        );
    }

    /// 自动探测 Git（原 `detect_git`）：`git --version` 的结果放进状态栏提示。
    pub(crate) fn detect_git(&self) {
        let notice = self.notice;
        let language = self.language;
        let lang = language.get_untracked();
        match git::info() {
            Ok(info) => crate::interactions::notify(
                notice,
                format!("{} · {}", info.version, info.exec_path),
            ),
            Err(_error) => {
                crate::interactions::notify(notice, i18n::t(lang.as_str(), "settings.detectFailed"))
            }
        }
    }

    /// 切换「使用 PATH 里的 git」（原 `toggle_git_path`）。
    pub(crate) fn toggle_git_path(&self) {
        let settings_state = self.state.settings_state;
        let store_dir = self.store_dir.clone();
        let mut next = settings_state.get_untracked();
        let enabled = !next.use_system_git;
        next.use_system_git = enabled;
        settings_state.set(next.clone());
        store::save_settings(&store_dir, &next);
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

    /// 改键（原 `rebind_shortcut`）：写入注册表并持久化；冲突/不可绑定时把原因
    /// 交给设置页显示。
    pub(crate) fn rebind_shortcut(&self, id: &'static str, combo: String) {
        let registry = self.state.shortcut_registry;
        let recording = self.state.recording_shortcut;
        let error = self.state.shortcut_error;
        let store_dir = self.store_dir.clone();
        let language = self.language;
        let mut next = registry.get_untracked();
        let outcome = next.rebind(id, combo.as_str());
        let lang = language.get_untracked();
        match outcome {
            Ok(()) => {
                registry.set(next.clone());
                store::save_shortcuts(&store_dir, &next.overrides());
                error.set(None);
                recording.set(None);
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

    /// 开始录制某个动作的新组合（原 `record_shortcut`）。
    pub(crate) fn record_shortcut(&self, id: &'static str) {
        self.state.shortcut_error.set(None);
        self.state.recording_shortcut.set(Some(id));
    }

    /// 恢复默认快捷键（原 `restore_shortcuts`）：先确认（会覆盖用户的自定义），
    /// 再重置并清空持久化。
    pub(crate) fn restore_shortcuts(&self) {
        let registry = self.state.shortcut_registry;
        let recording = self.state.recording_shortcut;
        let error = self.state.shortcut_error;
        let store_dir = self.store_dir.clone();
        let notice = self.notice;
        let language = self.language;
        let lang = language.get_untracked();
        let title = i18n::t(lang.as_str(), "confirm.restoreShortcutsTitle");
        let message = i18n::t(lang.as_str(), "confirm.restoreShortcutsMessage");
        let action_label = i18n::t(lang.as_str(), "confirm.restoreShortcutsAction");
        let done = i18n::t(lang.as_str(), "shortcut.restored");
        spawn(async move {
            let ok = crate::interactions::confirm(
                title,
                message,
                "restoreShortcuts",
                action_label,
                false,
            )
            .await;
            if !ok {
                return;
            }
            let mut next = registry.get_untracked();
            next.restore_defaults();
            registry.set(next.clone());
            store::save_shortcuts(&store_dir, &next.overrides());
            recording.set(None);
            error.set(None);
            crate::interactions::notify(notice, done);
        });
    }
}
