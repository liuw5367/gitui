//! 引用管理的用例：检出/新建/删除分支，推送/删除标签，远程、贮藏、身份与引用合并。
//!
//! 方法体由原 `Actions` 里的闭包原样搬来，只把捕获的局部变量换成 `self.*`；
//! 这里只调 `model` 并更新本功能的状态，不写布局。

use super::state::RefsState;
use crate::features::SessionHandle;
use crate::features::history::commands::HistoryCommands;
use crate::features::sync::commands::SyncCommands;
use crate::model;
use crate::screens;
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct RefsCommands {
    pub(crate) state: RefsState,
    pub(crate) session: SessionHandle,
    /// 确认框文案与成功提示要按当前语言取。
    pub(crate) language: Signal<SharedString>,
    /// 状态栏短提示（贮藏后切分支、推送标签成功等）。
    pub(crate) notice: Signal<Option<String>>,
    /// 提交历史的用例：Refs 表里的「比较」与「在历史中显示」是历史功能的入口。
    pub(crate) history: HistoryCommands,
    /// 历史操作入口（`checkout` / `cherry` / `revert`）：检出提交要复用它的确认流程。
    /// `cherry` 与 `revert` 仍归 recovery（批 B），届时整段随它们一起搬走。
    pub(crate) history_action: Arc<dyn Fn(&'static str, String)>,
    /// 表单弹层的状态：身份编辑要预填当前仓库级 user.name / user.email。
    pub(crate) form_state: Signal<Option<screens::FormState>>,
    /// 同步功能的用例：Refs 表里远程行的「取回」打开同步面板。
    pub(crate) sync: SyncCommands,
}

impl RefsCommands {
    /// 检出分支（原 `checkout_branch`）：有改动时先问贮藏后切、带过去还是取消。
    pub(crate) fn checkout_branch(&self, branch: String) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let language = self.language;
        let notice = self.notice;
        let current = active.get_untracked();
        let dirty = tabs
            .get_untracked()
            .get(current)
            .is_some_and(model::is_dirty);
        if !dirty {
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::checkout_branch(tab, branch.as_str());
                tabs.set(list);
            }
            return;
        }
        // 有改动时先问：贮藏后切、把改动带过去、还是取消（手册第 4 节）。
        let lang = language.get_untracked();
        let title = crate::i18n::t(lang.as_str(), "confirm.dirtyCheckoutTitle");
        let message = crate::i18n::with_arguments(
            lang.as_str(),
            "confirm.dirtyCheckoutMessage",
            &[("branch", branch.clone())],
        );
        let stash_label = crate::i18n::t(lang.as_str(), "confirm.dirtyCheckoutStash");
        let carry_label = crate::i18n::t(lang.as_str(), "confirm.dirtyCheckoutCarry");
        let cancel_label = crate::i18n::t(lang.as_str(), "common.cancel");
        let pending = nsh_base::alert_dialog()
            .title(title)
            .description(message)
            .action("stash", stash_label)
            .action("carry", carry_label)
            .cancel("cancel", cancel_label)
            .show();
        let language = language;
        let notice = notice;
        spawn(async move {
            let Ok(result) = pending.await else {
                return;
            };
            let Some(action) = result.action else {
                return;
            };
            if action.as_str() == "cancel" {
                return;
            }
            let stash = action.as_str() == "stash";
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::checkout_branch_with(tab, branch.as_str(), stash);
                let failed = tab.error.clone();
                tabs.set(list);
                if let Some(detail) = failed {
                    crate::interactions::notify(notice, detail);
                } else if stash {
                    let lang = language.get_untracked();
                    crate::interactions::notify(
                        notice,
                        crate::i18n::t(lang.as_str(), "toast.stashedBeforeCheckout"),
                    );
                }
            }
        });
    }

    /// 从某个引用新建分支并检出（原 `checkout_new_branch`）。
    pub(crate) fn checkout_new_branch(&self, name: String, from: String) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            if let Err(detail) = model::checkout_new_branch(tab, name.as_str(), from.as_str()) {
                tab.error = Some(detail);
            }
            tabs.set(list);
        }
    }

    /// 删除分支（原 `delete_branch`）：不可撤销，先确认。
    pub(crate) fn delete_branch(&self, branch: String) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let language = self.language;
        let language_now = language.get_untracked();
        let pending = crate::ui::dialog::confirm_destructive(
            crate::i18n::t(language_now.as_str(), "branch.deleteTitle"),
            crate::i18n::with_arguments(
                language_now.as_str(),
                "branch.deleteMessage",
                &[("name", branch.clone())],
            ),
            crate::i18n::t(language_now.as_str(), "branch.deleteAction"),
            crate::i18n::t(language_now.as_str(), "common.cancel"),
        );
        let tabs = tabs;
        let active = active;
        spawn(async move {
            if let Ok(result) = pending.await {
                if result.action.as_deref() != Some("delete") {
                    return;
                }
                let current = active.get_untracked();
                let mut list = tabs.get_untracked();
                if let Some(tab) = list.get_mut(current) {
                    model::delete_branch(tab, branch.as_str());
                    tabs.set(list);
                }
            }
        });
    }

    /// 推送标签（原 `push_tag`）。
    pub(crate) fn push_tag(&self, name: String) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let notice = self.notice;
        let language = self.language;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            match model::push_tag(tab, name.as_str()) {
                Ok(_) => {
                    let lang = language.get_untracked();
                    crate::interactions::notify(
                        notice,
                        crate::i18n::t(lang.as_str(), "toast.pushed"),
                    );
                }
                Err(detail) => tab.error = Some(detail),
            }
            tabs.set(list);
        }
    }

    /// 应用或弹出贮藏的共用体：`kind` 为 `apply` 或 `pop`。
    fn stash_action(&self, kind: &'static str, reference: String) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let notice = self.notice;
        let language = self.language;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            let result = if kind == "apply" {
                model::apply_stash(tab, reference.as_str())
            } else {
                model::pop_stash(tab, reference.as_str())
            };
            match result {
                Ok(()) => {
                    let lang = language.get_untracked();
                    crate::interactions::notify(
                        notice,
                        crate::i18n::t(lang.as_str(), "toast.stashDone"),
                    );
                }
                Err(detail) => tab.error = Some(detail),
            }
            tabs.set(list);
        }
    }

    /// 应用贮藏（原 `apply_stash`）：保留条目。
    pub(crate) fn apply_stash(&self, reference: String) {
        self.stash_action("apply", reference);
    }

    /// 弹出贮藏（原 `pop_stash`）：取出并删除条目。
    pub(crate) fn pop_stash(&self, reference: String) {
        self.stash_action("pop", reference);
    }

    /// 丢弃贮藏（原 `drop_stash`）：先确认。
    pub(crate) fn drop_stash(&self, reference: String) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let language = self.language;
        let language_now = language.get_untracked();
        let pending = crate::interactions::confirm(
            crate::i18n::t(language_now.as_str(), "confirm.dropStashTitle"),
            crate::i18n::with_arguments(
                language_now.as_str(),
                "confirm.dropStashMessage",
                &[("name", reference.clone())],
            ),
            "drop",
            crate::i18n::t(language_now.as_str(), "confirm.dropAction"),
            true,
        );
        spawn(async move {
            if !pending.await {
                return;
            }
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                let _ = model::drop_stash(tab, reference.as_str());
                tabs.set(list);
            }
        });
    }

    /// 删除标签（原 `drop_tag`）：先确认。
    pub(crate) fn drop_tag(&self, name: String) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let language = self.language;
        let language_now = language.get_untracked();
        let pending = crate::interactions::confirm(
            crate::i18n::t(language_now.as_str(), "confirm.dropTagTitle"),
            crate::i18n::with_arguments(
                language_now.as_str(),
                "confirm.dropTagMessage",
                &[("name", name.clone())],
            ),
            "drop",
            crate::i18n::t(language_now.as_str(), "confirm.deleteAction"),
            true,
        );
        spawn(async move {
            if !pending.await {
                return;
            }
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                let _ = model::drop_tag(tab, name.as_str());
                tabs.set(list);
            }
        });
    }

    /// 移除远程（原 `drop_remote`）：先确认。
    pub(crate) fn drop_remote(&self, name: String) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let language = self.language;
        let language_now = language.get_untracked();
        let pending = crate::interactions::confirm(
            crate::i18n::t(language_now.as_str(), "confirm.dropRemoteTitle"),
            crate::i18n::with_arguments(
                language_now.as_str(),
                "confirm.dropRemoteMessage",
                &[("name", name.clone())],
            ),
            "drop",
            crate::i18n::t(language_now.as_str(), "confirm.removeAction"),
            true,
        );
        spawn(async move {
            if !pending.await {
                return;
            }
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                let _ = model::drop_remote(tab, name.as_str());
                tabs.set(list);
            }
        });
    }

    /// 把某个引用合并进当前分支（原 `merge_reference`）。
    pub(crate) fn merge_reference(&self, reference: String) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            if let Err(detail) = model::merge_reference(tab, reference.as_str()) {
                tab.error = Some(detail);
            }
            tabs.set(list);
        }
    }

    /// 切到某个提交（原 `checkout_commit`，detached，先确认）。
    pub(crate) fn checkout_commit(&self, sha: String) {
        (self.history_action)("checkout", sha);
    }

    /// 游离头时回到 main/master（原 `checkout_default_branch`）。
    pub(crate) fn checkout_default_branch(&self) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let notice = self.notice;
        let language = self.language;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            match model::checkout_default_branch(tab) {
                Ok(()) => {
                    let lang = language.get_untracked();
                    crate::interactions::notify(
                        notice,
                        crate::i18n::t(lang.as_str(), "toast.historyDone"),
                    );
                }
                Err(detail) => crate::interactions::notify(notice, detail),
            }
            tabs.set(list);
        }
    }

    /// 打开身份编辑表单（原 `open_identity`，仓库级 user.name / user.email）。
    pub(crate) fn open_identity(&self) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let form_state = self.form_state;
        let list = tabs.get_untracked();
        let Some(tab) = list.get(active.get_untracked()) else {
            return;
        };
        let (name, email) = model::identity(tab);
        let mut state = screens::FormState::new(screens::FormKind::Identity);
        state.first = name;
        state.second = email;
        form_state.set(Some(state));
    }

    /// 清掉仓库级身份，回到全局配置（原 `clear_identity`）。
    pub(crate) fn clear_identity(&self) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let notice = self.notice;
        let language = self.language;
        let list = tabs.get_untracked();
        let Some(tab) = list.get(active.get_untracked()) else {
            return;
        };
        match model::clear_identity(tab) {
            Ok(()) => {
                let lang = language.get_untracked();
                crate::interactions::notify(
                    notice,
                    crate::i18n::t(lang.as_str(), "toast.identityCleared"),
                );
            }
            Err(detail) => crate::interactions::notify(notice, detail),
        }
    }
}
