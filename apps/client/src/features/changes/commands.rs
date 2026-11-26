//! 变更/暂存的用例：选中变更、暂存与丢弃、按 hunk/行应用、变更清单。
//!
//! 方法体由原 `Actions` 里的闭包原样搬来，只把捕获的局部变量换成 `self.*`；
//! 这里只调本功能的 `ops` 与 `app::ops`，不写布局。

use super::ops;
use super::state::ChangesState;
use crate::app::ops::stage_all_or_none;
use crate::features::SessionHandle;
use crate::features::merge::commands::MergeCommands;
use crate::shared::diff::DiffState;
use nsh_core::prelude::*;

#[derive(Clone)]
pub(crate) struct ChangesCommands {
    pub(crate) state: ChangesState,
    pub(crate) session: SessionHandle,
    pub(crate) diff_state: DiffState,
    /// 丢弃前的确认文案与分段提示要按当前语言取。
    pub(crate) language: Signal<SharedString>,
    /// 状态栏短提示（按 hunk/行应用成功后的提示）。
    pub(crate) notice: Signal<Option<String>>,
    /// Main view mode, used to open the blame page for a worktree file.
    pub(crate) mode: Signal<crate::Mode>,
    /// 冲突解决功能的用例：变更列表里冲突文件的「用 ours/theirs 解决」与
    /// 「解决冲突」是冲突视图的入口。
    pub(crate) merge: MergeCommands,
}

impl ChangesCommands {
    pub(crate) fn open_blame(&self, path: String) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            crate::features::history::ops::open_blame(
                tab,
                path.as_str(),
                "HEAD",
                crate::Mode::Changes,
            );
            self.session.tabs.set(list);
            self.mode.set(crate::Mode::Blame);
        }
    }

    /// 选中第 `index` 个变更、读取补丁并切到 Changes 模式显示 Diff。
    pub(crate) fn select_change(&self, index: usize) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        let Some(tab) = list.get_mut(current) else {
            return;
        };
        if index >= tab.changes.len() {
            return;
        }
        tab.stash_diff = None;
        ops::load_change_diff_with_whitespace(
            tab,
            index,
            self.diff_state.ignore_whitespace.get_untracked(),
        );
        self.session.tabs.set(list);
        self.mode.set(crate::Mode::Changes);
    }

    /// 按文件路径选中变更，避免暂存重排后沿用旧列表索引。
    pub(crate) fn select_change_path(&self, path: String) {
        let current = self.session.active.get_untracked();
        let index = self
            .session
            .tabs
            .get_untracked()
            .get(current)
            .and_then(|tab| tab.changes.iter().position(|change| change.path == path));
        if let Some(index) = index {
            self.select_change(index);
        }
    }

    pub(crate) fn reload_selected_diff(&self, ignore_whitespace: bool) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            if let Some(index) = tab.selected_change {
                ops::load_change_diff_with_whitespace(tab, index, ignore_whitespace);
            }
            self.session.tabs.set(list);
        }
    }

    /// 从工作区 Diff 返回提交列表。
    pub(crate) fn clear_selected_change(&self) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            tab.selected_change = None;
            tab.change_diff = domain::diff::Patch::default();
            tab.change_hunk_staged.clear();
            self.session.tabs.set(list);
        }
    }

    pub(crate) fn clear_stash_diff_at(&self, tab_id: u64) {
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) {
            tab.stash_diff = None;
            self.session.tabs.set(list);
        }
    }

    pub(crate) fn reload_stash_diff_at(&self, tab_id: u64, oid: String, ignore_whitespace: bool) {
        let mut tabs = self.session.tabs.get_untracked();
        let Some(index) = tabs.iter().position(|tab| tab.id == tab_id) else {
            return;
        };
        if !tabs[index]
            .stash_diff
            .as_ref()
            .is_some_and(|stash| stash.oid == oid)
        {
            return;
        }
        let patch = tabs[index].open_repo().and_then(|repo| {
            repo.stash_show_patch_with_whitespace(oid.as_str(), true, ignore_whitespace)
        });
        if let Ok(patch) = patch {
            if let Some(stash) = tabs[index].stash_diff.as_mut() {
                stash.patch = patch;
            }
            self.session.tabs.set(tabs);
        }
    }

    /// 暂存或取消暂存第 `index` 个变更（原 `toggle_stage`）。
    pub(crate) fn toggle_stage(&self, index: usize) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            ops::toggle_stage(
                tab,
                index,
                self.diff_state.ignore_whitespace.get_untracked(),
            );
            self.session.tabs.set(list);
        }
    }

    /// 按文件路径切换到明确的暂存状态，避免清单重排后索引指向另一条变更。
    pub(crate) fn set_stage_path(&self, path: String, stage: bool) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            ops::set_stage_path(
                tab,
                path.as_str(),
                stage,
                self.diff_state.ignore_whitespace.get_untracked(),
            );
            self.session.tabs.set(list);
        }
    }

    /// 丢弃第 `index` 个变更（原 `discard_change`）：先确认，确认后才动工作区。
    pub(crate) fn discard_change(&self, index: usize) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let language = self.language;
        let list = tabs.get_untracked();
        let current = active.get_untracked();
        let Some(tab) = list.get(current).cloned() else {
            return;
        };
        let Some(entry) = tab.changes.get(index).cloned() else {
            return;
        };
        let language_now = language.get_untracked();
        let pending = crate::ui::dialog::confirm_destructive(
            crate::i18n::t(language_now.as_str(), "changes.discardTitle"),
            crate::i18n::with_arguments(
                language_now.as_str(),
                "changes.discardMessage",
                &[("path", entry.path.clone())],
            ),
            crate::i18n::t(language_now.as_str(), "changes.discardAction"),
            crate::i18n::t(language_now.as_str(), "common.cancel"),
        );
        spawn(async move {
            if let Ok(result) = pending.await {
                if result.action.as_deref() != Some("discard") {
                    return;
                }
                let current = active.get_untracked();
                let mut list = tabs.get_untracked();
                if let Some(tab) = list.get_mut(current) {
                    ops::discard_change(tab, index);
                    tabs.set(list);
                }
            }
        });
    }

    /// 全部暂存（原 `stage_all`）。
    pub(crate) fn stage_all(&self) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            stage_all_or_none(tab, true);
            tabs.set(list);
        }
    }

    /// 全部取消暂存（原 `unstage_all`）。
    pub(crate) fn unstage_all(&self) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            stage_all_or_none(tab, false);
            tabs.set(list);
        }
    }

    /// 暂存或取消暂存指定清单中的全部文件。
    pub(crate) fn stage_changelist(&self, name: Option<String>, stage: bool) {
        let current = self.session.active.get_untracked();
        let mut tabs = self.session.tabs.get_untracked();
        if let Some(tab) = tabs.get_mut(current) {
            ops::stage_changelist(
                tab,
                name.as_deref(),
                stage,
                self.diff_state.ignore_whitespace.get_untracked(),
            );
            self.session.tabs.set(tabs);
        }
    }

    /// 暂存或取消暂存当前清单下某个目录中的全部文件。
    pub(crate) fn stage_directory(&self, directory: String, name: Option<String>, stage: bool) {
        let current = self.session.active.get_untracked();
        let mut tabs = self.session.tabs.get_untracked();
        if let Some(tab) = tabs.get_mut(current) {
            ops::stage_directory(
                tab,
                name.as_deref(),
                directory.as_str(),
                stage,
                self.diff_state.ignore_whitespace.get_untracked(),
            );
            self.session.tabs.set(tabs);
        }
    }

    /// 按 hunk 暂存或取消暂存。
    pub(crate) fn apply_piece(&self, hunk: usize, reverse: bool, ignore_whitespace: bool) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let notice = self.notice;
        let language = self.language;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            match ops::apply_piece_with_whitespace(tab, hunk, reverse, ignore_whitespace) {
                Ok(()) => {
                    let lang = language.get_untracked();
                    crate::interactions::notify(
                        notice,
                        crate::i18n::t(lang.as_str(), "changes.pieceApplied"),
                    );
                }
                Err(detail) => tab.error = Some(detail),
            }
            tabs.set(list);
        }
    }

    /// 把当前清单设为 `name`；`None` 表示默认清单（原 `set_active_changelist`）。
    pub(crate) fn set_active_changelist(&self, name: Option<String>) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            ops::set_active_changelist(tab, name);
            self.session.tabs.set(list);
        }
    }

    /// 把 `path` 移入 `list_name`；`None` 表示默认清单（原 `move_to_changelist`）。
    pub(crate) fn move_to_changelist(&self, path: String, list_name: Option<String>) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            ops::move_to_changelist(tab, path.as_str(), list_name.as_deref());
            self.session.tabs.set(list);
        }
    }

    pub(crate) fn delete_changelist(&self, name: String) {
        let current = self.session.active.get_untracked();
        let mut tabs = self.session.tabs.get_untracked();
        if let Some(tab) = tabs.get_mut(current) {
            ops::delete_changelist(tab, name.as_str());
            self.state.collapsed_changelists.update(|by_repo| {
                if let Some(groups) = by_repo.get_mut(&tab.id) {
                    groups.remove(&Some(name.clone()));
                }
            });
            self.state.changelist_tree_expanded.update(|by_group| {
                by_group.remove(&(tab.id, Some(name.clone())));
            });
            self.session.tabs.set(tabs);
        }
    }

    pub(crate) fn open_rename_changelist(&self, name: String) {
        self.state.rename_changelist_value.set(name.clone());
        self.state.rename_changelist.set(Some(name));
    }

    pub(crate) fn close_rename_changelist(&self) {
        self.state.rename_changelist.set(None);
        self.state.rename_changelist_value.set(String::new());
    }

    /// 提交清单重命名；名称重复或为空时保持对话框打开。
    pub(crate) fn rename_changelist(&self) -> bool {
        let Some(old_name) = self.state.rename_changelist.get_untracked() else {
            return false;
        };
        let new_name = self.state.rename_changelist_value.get_untracked();
        let current = self.session.active.get_untracked();
        let mut tabs = self.session.tabs.get_untracked();
        let Some(tab) = tabs.get_mut(current) else {
            return false;
        };
        if !ops::rename_changelist(tab, old_name.as_str(), new_name.as_str()) {
            return false;
        }

        let repo_id = tab.id;
        self.state.collapsed_changelists.update(|by_repo| {
            if let Some(groups) = by_repo.get_mut(&repo_id)
                && groups.remove(&Some(old_name.clone()))
            {
                groups.insert(Some(new_name.trim().to_string()));
            }
        });
        self.state.changelist_tree_expanded.update(|by_group| {
            if let Some(expanded) = by_group.remove(&(repo_id, Some(old_name.clone()))) {
                by_group.insert((repo_id, Some(new_name.trim().to_string())), expanded);
            }
        });
        self.session.tabs.set(tabs);
        return true;
    }

    /// 新建变更清单 `name`（原 `create_changelist`）。
    pub(crate) fn create_changelist(&self, name: String) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            ops::create_changelist(tab, name.as_str());
            self.session.tabs.set(list);
        }
    }
}
