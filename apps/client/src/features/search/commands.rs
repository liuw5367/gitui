//! 搜索功能的用例：命令分发、视图模式切换、表单浮层的开关与提交。
//!
//! 方法体由原 `Actions`（与 `lib.rs` 里的局部闭包）原样搬来，只把捕获的信号换成
//! `self.*`；会话动作（关页签、前/后切页签）走 `SessionHandle` 的方法。

use super::form_dialog::{FormKind, FormState};
use super::ops;
use super::state::SearchState;
use super::view;
use crate::app::session::Sync;
use crate::features::commit::commands::CommitCommands;
use crate::features::history::commands::HistoryCommands;
use crate::features::settings::commands::SettingsCommands;
use crate::features::sync::commands::SyncCommands;
use crate::features::welcome::commands::WelcomeCommands;
use crate::features::SessionHandle;
use nsh_core::prelude::*;

#[derive(Clone)]
pub(crate) struct SearchCommands {
    pub(crate) state: SearchState,
    pub(crate) session: SessionHandle,
    /// 面板结果的过滤与命令文案都按当前语言取。
    pub(crate) language: Signal<SharedString>,
    /// 「打开仓库」是欢迎页的入口。
    pub(crate) welcome: WelcomeCommands,
    /// 刷新、对比模式与历史筛选浮层归提交历史功能。
    pub(crate) history: HistoryCommands,
    /// `⌘↵` 在主区为 Changes 时提交，归提交功能。
    pub(crate) commit: CommitCommands,
    /// 设置页、语言与外观切换归设置功能。
    pub(crate) settings: SettingsCommands,
    /// Fetch / Pull / Push 命令打开同步面板。
    pub(crate) sync: SyncCommands,
    /// 右侧检查器的显隐：命令面板与工具栏按钮共用同一个信号。
    pub(crate) inspector_open: Signal<bool>,
}

impl SearchCommands {
    /// 切换工作区主区模式（原 `set_mode`）。
    pub(crate) fn set_mode(&self, mode: crate::Mode) {
        self.state.mode.set(mode);
    }

    /// 切到引用管理的 Refs 表（原 `set_mode_refs`）。
    pub(crate) fn set_mode_refs(&self) {
        self.set_mode(crate::Mode::Refs);
    }

    /// 切到 Agent 面板（原 `set_mode_agent`）。
    pub(crate) fn set_mode_agent(&self) {
        self.set_mode(crate::Mode::Agent);
    }

    /// 打开表单浮层（原 `open_form`）。
    pub(crate) fn open_form(&self, kind: FormKind) {
        self.state.form.set(Some(FormState::new(kind)));
    }

    /// 打开表单并把第二个字段预填（原 `open_form_from`）：从某个提交新建分支/标签、
    /// 重命名分支或改远程 URL 时用。
    pub(crate) fn open_form_from(&self, kind: FormKind, from: String) {
        let mut state = FormState::new(kind);
        state.second = from;
        self.state.form.set(Some(state));
    }

    /// 打开表单并同时预填前两个字段（身份编辑：姓名与邮箱）。
    pub(crate) fn open_form_prefilled(
        &self,
        kind: FormKind,
        first: String,
        second: String,
    ) {
        let mut state = FormState::new(kind);
        state.first = first;
        state.second = second;
        self.state.form.set(Some(state));
    }

    /// 关闭表单浮层（原 `cancel_form` 与 Esc 的浮层退出一层）。
    pub(crate) fn cancel_form(&self) {
        self.state.form.set(None);
    }

    /// 提交表单：按种类执行对应的 Git 写操作，失败时把原因留在对话框里。
    pub(crate) fn submit_form(&self, form: FormState) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let form_state = self.state.form;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        let Some(tab) = list.get_mut(current) else {
            return;
        };
        let first = form.first.trim().to_string();
        let second = form.second.trim().to_string();
        if first.is_empty() {
            let mut next = form.clone();
            next.error = Some("名称不能为空".to_string());
            form_state.set(Some(next));
            return;
        }
        let result = match form.kind {
            FormKind::NewBranch => {
                ops::create_branch(tab, first.as_str(), second.as_str(), form.flag)
            }
            FormKind::NewTag => ops::create_tag(
                tab,
                first.as_str(),
                second.as_str(),
                form.third.as_str(),
                form.flag,
            ),
            FormKind::AddRemote => ops::add_remote(tab, first.as_str(), second.as_str()),
            FormKind::NewStash => ops::stash_changes(tab, first.as_str(), form.flag),
            // 重命名与改 URL：`second` 是被改对象的原名，`first` 是新值。
            FormKind::RenameBranch => {
                ops::rename_branch(tab, second.as_str(), first.as_str())
            }
            FormKind::EditRemote => {
                ops::edit_remote(tab, second.as_str(), first.as_str())
            }
            // 身份：`first` 是姓名，`second` 是邮箱。
            FormKind::Identity => ops::set_identity(tab, first.as_str(), second.as_str()),
        };
        tabs.set(list);
        match result {
            Ok(()) => form_state.set(None),
            Err(detail) => {
                let mut next = form;
                next.error = Some(detail);
                form_state.set(Some(next));
            }
        }
    }

    /// 执行一条命令：命令面板、原生菜单与快捷键共用这个分发入口（原 `run_command`）。
    ///
    /// 命令 id 就是快捷键注册表里的动作 id；未接入的 id 直接忽略。
    pub(crate) fn run_command(&self, id: &'static str) {
        match id {
            "open" => self.welcome.choose_folder(),
            "openPalette" => {
                // 打开时把高亮拉回第一行。
                self.state.palette_selected.set(0);
                self.state.palette_open.set(true);
            }
            "closeTab" => self.session.close_tab(self.session.active.get_untracked()),
            "refresh" => self.history.refresh(),
            "history" => self.set_mode(crate::Mode::History),
            "changes" => self.set_mode(crate::Mode::Changes),
            "inspector" => self
                .inspector_open
                .set(!self.inspector_open.get_untracked()),
            "compare" => {
                let compare_mode = self.history.state.compare_mode;
                compare_mode.set(!compare_mode.get_untracked())
            }
            "fetch" => self.sync.open_sync(Sync::Fetch),
            "pull" => self.sync.open_sync(Sync::Pull),
            "push" => self.sync.open_sync(Sync::Push),
            // ⌘F：欢迎窗口打开时聚焦左栏搜索；主窗口里打开历史过滤浮层。
            "filter" => {
                if self.session.welcome_open.get_untracked() {
                    self.welcome.focus_search();
                } else {
                    self.set_mode(crate::Mode::History);
                    self.history.state.filter_open.set(true);
                }
            }
            // ⇧⌘F 始终是历史过滤。
            "filterHistory" => {
                self.set_mode(crate::Mode::History);
                self.history.state.filter_open.set(true);
            }
            // ⌘↵：只在 Changes 里提交，避免在历史视图误提交。
            "primary" => {
                if self.state.mode.get_untracked() == crate::Mode::Changes {
                    self.commit.commit();
                }
            }
            "prevTab" => self.session.cycle_tab(false),
            "nextTab" => self.session.cycle_tab(true),
            "tab1" => self.session.activate_slot(0),
            "tab2" => self.session.activate_slot(1),
            "tab3" => self.session.activate_slot(2),
            "tab4" => self.session.activate_slot(3),
            "tab5" => self.session.activate_slot(4),
            "tab6" => self.session.activate_slot(5),
            "tab7" => self.session.activate_slot(6),
            "tab8" => self.session.activate_slot(7),
            "tab9" => self.session.activate_slot(8),
            "welcome" => self.session.go_welcome(),
            "repoManager" => self.session.go_repo_manager(),
            "settings" => self.settings.go_settings(),
            "toggleLanguage" => self.settings.toggle_language(),
            "toggleTheme" => {
                let next = self.settings.theme_mode.get_untracked().next();
                self.settings.set_theme(next);
            }
            _ => {}
        }
    }

    /// 面板当前的命中结果（渲染与键盘导航共用，避免「高亮一行、执行另一行」）。
    fn rows(&self) -> Vec<view::Row> {
        let open_paths: Vec<String> = self
            .session
            .tabs
            .get_untracked()
            .iter()
            .map(|tab| tab.path.clone())
            .collect();
        let recent_paths: Vec<String> = self
            .session
            .recent
            .get_untracked()
            .iter()
            .map(|entry| entry.path.clone())
            .collect();
        return view::rows(
            self.state.palette_query.get_untracked().as_str(),
            self.language.get_untracked().as_str(),
            &open_paths,
            &recent_paths,
        );
    }

    /// 上下移动高亮；面板没打开时不动（原 ↑/↓ 的按键处理）。
    pub(crate) fn move_selection(&self, down: bool) {
        if !self.state.palette_open.get_untracked() {
            return;
        }
        let selected = self.state.palette_selected;
        if !down {
            let next = selected.get_untracked().saturating_sub(1);
            selected.set(next);
            return;
        }
        let count = self.rows().len();
        if count == 0 {
            return;
        }
        let next = (selected.get_untracked() + 1).min(count - 1);
        selected.set(next);
    }

    /// 执行当前高亮行并关闭面板（原 ↵ 的按键处理）。
    pub(crate) fn activate_selected(&self) {
        if !self.state.palette_open.get_untracked() {
            return;
        }
        let rows = self.rows();
        let index = self
            .state
            .palette_selected
            .get_untracked()
            .min(rows.len().saturating_sub(1));
        let Some(row) = rows.get(index).cloned() else {
            return;
        };
        match row {
            view::Row::Command(id) => self.run_command(id),
            view::Row::Repo { path, .. } => self.session.open_path(path),
        }
        self.state.palette_open.set(false);
    }
}
