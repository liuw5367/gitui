//! 恢复的用例：撤销 reflog、重置到某个提交、拣选与回滚提交。
//!
//! 方法体由原 `Actions` 里的闭包原样搬来，只把捕获的局部变量换成 `self.*`；
//! 这里只调本功能的 `ops` 并更新当前标签页，不写布局。

use super::ops;
use crate::features::SessionHandle;
use crate::i18n;
use crate::interactions;
use nsh_core::prelude::*;

#[derive(Clone)]
pub(crate) struct RecoveryCommands {
    pub(crate) session: SessionHandle,
    /// 确认框文案与成功提示要按当前语言取。
    pub(crate) language: Signal<SharedString>,
    /// 状态栏短提示（检出/拣选/回滚的结果）。
    pub(crate) notice: Signal<Option<String>>,
}

impl RecoveryCommands {
    /// 历史操作共用体：`kind` 为 `checkout`、`cherry` 或 `revert`，都先确认再落
    /// 到 `ops` → git。检出与拣选/回滚共用同一个确认框，因此留在同一处。
    fn history_action(&self, kind: &'static str, sha: String) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let language = self.language;
        let notice = self.notice;
        let language_now = language.get_untracked();
        let lang = language_now.as_str();
        let (title_key, message_key, action_key) = match kind {
            "checkout" => (
                "confirm.checkoutTitle",
                "confirm.checkoutMessage",
                "confirm.checkoutAction",
            ),
            "cherry" => (
                "confirm.cherryPickTitle",
                "confirm.cherryPickMessage",
                "confirm.cherryPickAction",
            ),
            _ => (
                "confirm.revertTitle",
                "confirm.revertMessage",
                "confirm.revertAction",
            ),
        };
        let short = sha.chars().take(7).collect::<String>();
        let pending = interactions::confirm(
            i18n::with_arguments(lang, title_key, &[("short", short.clone())]),
            i18n::t(lang, message_key),
            "history",
            i18n::t(lang, action_key),
            i18n::t(lang, "common.cancel"),
            kind == "checkout",
        );
        spawn(async move {
            if !pending.await {
                return;
            }
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                let result = match kind {
                    "checkout" => ops::checkout_commit(tab, sha.as_str()),
                    "cherry" => ops::cherry_pick(tab, sha.as_str()),
                    _ => ops::revert_commit(tab, sha.as_str()),
                };
                match result {
                    Ok(_) => {
                        let lang = language.get_untracked();
                        interactions::notify(notice, i18n::t(lang.as_str(), "toast.historyDone"));
                    }
                    Err(detail) => tab.error = Some(detail),
                }
                tabs.set(list);
            }
        });
    }

    /// 切到某个提交（原 `checkout_commit`，detached，先确认）。refs 的检出入口用
    /// 它，所以确认流程留在这里，refs 不再自带一份。
    pub(crate) fn checkout_commit(&self, sha: String) {
        self.history_action("checkout", sha);
    }

    /// 撤销 reflog 里的第 `index` 条（原 `undo_reflog`）：不可撤销，先确认。
    pub(crate) fn undo_reflog(&self, index: usize) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let language = self.language;
        let list = tabs.get_untracked();
        let current = active.get_untracked();
        let Some(tab) = list.get(current).cloned() else {
            return;
        };
        let Some(entry) = tab.reflog.get(index).cloned() else {
            return;
        };
        let language_now = language.get_untracked();
        let pending = crate::ui::dialog::confirm_destructive(
            i18n::t(language_now.as_str(), "reflog.undoTitle"),
            i18n::with_arguments(
                language_now.as_str(),
                "reflog.undoMessage",
                &[("short", entry.short.clone())],
            ),
            i18n::t(language_now.as_str(), "reflog.undoAction"),
            i18n::t(language_now.as_str(), "common.cancel"),
        );
        spawn(async move {
            if let Ok(result) = pending.await {
                if result.action.as_deref() != Some("undo") {
                    return;
                }
                let current = active.get_untracked();
                let mut list = tabs.get_untracked();
                if let Some(tab) = list.get_mut(current) {
                    ops::undo_to_reflog(tab, index);
                    tabs.set(list);
                }
            }
        });
    }

    /// 拣选提交（原 `cherry_pick`）：先确认。
    pub(crate) fn cherry_pick(&self, sha: String) {
        self.history_action("cherry", sha);
    }

    /// 反向提交（原 `revert_commit`）：先确认。
    pub(crate) fn revert_commit(&self, sha: String) {
        self.history_action("revert", sha);
    }

    /// 重置到某个提交（原 `reset_to`）：`--hard` 会丢改动，先确认。
    pub(crate) fn reset_to(&self, sha: String, mode: git::ResetMode) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let language = self.language;
        let language_now = language.get_untracked();
        let pending = crate::ui::dialog::confirm_destructive(
            i18n::t(language_now.as_str(), "branch.resetTitle"),
            i18n::with_arguments(
                language_now.as_str(),
                "branch.resetMessage",
                &[("short", sha.chars().take(7).collect::<String>())],
            ),
            i18n::t(language_now.as_str(), "branch.resetAction"),
            i18n::t(language_now.as_str(), "common.cancel"),
        );
        spawn(async move {
            if let Ok(result) = pending.await {
                if result.action.as_deref() != Some("reset") {
                    return;
                }
                let current = active.get_untracked();
                let mut list = tabs.get_untracked();
                if let Some(tab) = list.get_mut(current) {
                    if let Err(detail) = ops::reset_to(tab, sha.as_str(), mode) {
                        tab.error = Some(detail);
                    }
                    tabs.set(list);
                }
            }
        });
    }
}
