//! 冲突解决的用例：打开视图、切换冲突文件、接受一侧、标记已解决、继续与放弃。
//!
//! 方法体由原 `Actions` 里的闭包原样搬来，只把捕获的局部变量换成 `self.*`；
//! 这里只调 `model` 并更新本功能的状态，不写布局。

use super::state::MergeState;
use crate::features::SessionHandle;
use crate::model;
use domain::conflict::Side;
use nsh_core::prelude::*;

#[derive(Clone)]
pub(crate) struct MergeCommands {
    pub(crate) state: MergeState,
    pub(crate) session: SessionHandle,
    /// 成功提示与「放弃」确认框的文案要按当前语言取。
    pub(crate) language: Signal<SharedString>,
    /// 状态栏短提示（标记已解决、操作已继续）。
    pub(crate) notice: Signal<Option<String>>,
}

impl MergeCommands {
    /// 打开冲突视图并载入第一个冲突文件（原 `open_conflict`）。
    pub(crate) fn open_conflict(&self) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let conflict_result = self.state.result;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            model::open_conflict(tab);
            conflict_result.set(tab.conflict_text.clone());
            tabs.set(list);
        }
    }

    /// 切换到第 `index` 个冲突文件（原 `select_conflict`）。
    pub(crate) fn select_conflict(&self, index: usize) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let conflict_result = self.state.result;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            model::select_conflict(tab, index);
            conflict_result.set(tab.conflict_text.clone());
            tabs.set(list);
        }
    }

    /// 前/后一个冲突文件（原 `step_conflict`）。
    pub(crate) fn step_conflict(&self, forward: bool) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let conflict_result = self.state.result;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            model::step_conflict(tab, forward);
            conflict_result.set(tab.conflict_text.clone());
            tabs.set(list);
        }
    }

    /// 用某一侧填充结果区（原 `apply_side`）。
    pub(crate) fn apply_side(&self, side: Side) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let conflict_result = self.state.result;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            let merged = model::conflict_take_side(tab, side);
            conflict_result.set(merged);
            tabs.set(list);
        }
    }

    /// 把结果区写回文件并 `git add`（原 `mark_resolved`）。
    pub(crate) fn mark_resolved(&self) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let conflict_result = self.state.result;
        let notice = self.notice;
        let language = self.language;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            let resolved = conflict_result.get_untracked();
            match model::mark_conflict_resolved(tab, resolved.as_str()) {
                Ok(()) => {
                    conflict_result.set(tab.conflict_text.clone());
                    let lang = language.get_untracked();
                    crate::interactions::notify(
                        notice,
                        crate::i18n::t(lang.as_str(), "conflicts.marked"),
                    );
                }
                Err(detail) => tab.error = Some(detail),
            }
            tabs.set(list);
        }
    }

    /// 继续被冲突打断的操作（原 `continue_op`）。
    pub(crate) fn continue_op(&self) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let notice = self.notice;
        let language = self.language;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            match model::continue_operation(tab) {
                Ok(_) => {
                    let lang = language.get_untracked();
                    crate::interactions::notify(
                        notice,
                        crate::i18n::t(lang.as_str(), "toast.historyDone"),
                    );
                }
                Err(detail) => tab.error = Some(detail),
            }
            tabs.set(list);
        }
    }

    /// 关闭冲突视图（原 `close_conflict`）。
    pub(crate) fn close_conflict(&self) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            tab.conflict_open = false;
            tabs.set(list);
        }
    }

    /// 用整份 ours/theirs 解决某个冲突文件（原 `resolve_conflict`）。
    pub(crate) fn resolve_conflict(&self, path: String, ours: bool) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            model::resolve_conflict(tab, path.as_str(), ours);
            tabs.set(list);
        }
    }

    /// 放弃进行中的操作（原 `abort_operation`）：先强确认再 `abort`。
    pub(crate) fn abort_operation(&self) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let language = self.language;
        let language_now = language.get_untracked();
        let operation = tabs
            .get_untracked()
            .get(active.get_untracked())
            .map(|tab| tab.operation.id().to_string())
            .unwrap_or_default();
        let pending = crate::interactions::confirm(
            crate::i18n::with_arguments(
                language_now.as_str(),
                "confirm.abortTitle",
                &[("operation", operation)],
            ),
            crate::i18n::t(language_now.as_str(), "confirm.abortMessage"),
            "abort",
            crate::i18n::t(language_now.as_str(), "confirm.abortAction"),
            true,
        );
        spawn(async move {
            if !pending.await {
                return;
            }
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::abort_operation(tab);
                tabs.set(list);
            }
        });
    }
}
