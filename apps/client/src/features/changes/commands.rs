//! 变更/暂存的用例：选中变更、暂存与丢弃、按 hunk/行应用、变更清单。
//!
//! 方法体由原 `Actions` 里的闭包原样搬来，只把捕获的局部变量换成 `self.*`；
//! 这里只调 `model`，不写布局。

use super::state::ChangesState;
use crate::features::SessionHandle;
use crate::features::merge::commands::MergeCommands;
use crate::model;
use nsh_core::prelude::*;

#[derive(Clone)]
pub(crate) struct ChangesCommands {
    pub(crate) state: ChangesState,
    pub(crate) session: SessionHandle,
    /// 丢弃前的确认文案与分段提示要按当前语言取。
    pub(crate) language: Signal<SharedString>,
    /// 状态栏短提示（按 hunk/行应用成功后的提示）。
    pub(crate) notice: Signal<Option<String>>,
    /// 冲突解决功能的用例：变更列表里冲突文件的「用 ours/theirs 解决」与
    /// 「解决冲突」是冲突视图的入口。
    pub(crate) merge: MergeCommands,
}

impl ChangesCommands {
    /// 选中第 `index` 个变更并读它的补丁（原 `select_change`）。
    pub(crate) fn select_change(&self, index: usize) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            model::load_change_diff(tab, index);
            self.session.tabs.set(list);
        }
    }

    /// 暂存或取消暂存第 `index` 个变更（原 `toggle_stage`）。
    pub(crate) fn toggle_stage(&self, index: usize) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            model::toggle_stage(tab, index);
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
                    model::discard_change(tab, index);
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
            model::stage_all_or_none(tab, true);
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
            model::stage_all_or_none(tab, false);
            tabs.set(list);
        }
    }

    /// 按 hunk / 单行暂存或取消暂存（原 `apply_piece`）。
    pub(crate) fn apply_piece(&self, hunk: usize, line: Option<usize>, reverse: bool) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let notice = self.notice;
        let language = self.language;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            match model::apply_piece(tab, hunk, line, reverse) {
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
            model::set_active_changelist(tab, name);
            self.session.tabs.set(list);
        }
    }

    /// 把 `path` 移入 `list_name`；`None` 表示默认清单（原 `move_to_changelist`）。
    pub(crate) fn move_to_changelist(&self, path: String, list_name: Option<String>) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            model::move_to_changelist(tab, path.as_str(), list_name.as_deref());
            self.session.tabs.set(list);
        }
    }

    /// 新建变更清单 `name`（原 `create_changelist`）。
    pub(crate) fn create_changelist(&self, name: String) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            model::create_changelist(tab, name.as_str());
            self.session.tabs.set(list);
        }
    }
}
