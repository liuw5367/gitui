//! 引用管理的用例：检出/新建/删除分支，推送/删除标签，远程、贮藏、身份与引用合并。
//!
//! 方法体由原 `Actions` 里的闭包原样搬来，只把捕获的局部变量换成 `self.*`；
//! 这里只调本功能的 `ops` 与 `app::ops`，不写布局。

use super::ops;
use super::state::RefsState;
use crate::app::ops::identity;
use crate::features::SessionHandle;
use crate::features::history::commands::HistoryCommands;
use crate::features::recovery::commands::RecoveryCommands;
use crate::features::search::commands::SearchCommands;
use crate::features::search::form_dialog::FormKind;
use crate::features::sync::commands::SyncCommands;
use nsh_core::prelude::*;

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
    /// 恢复功能的用例（批 B 搬入）：refs 只保留「切到某个提交」这个入口，确认
    /// 流程与拣选/回滚共用 recovery 的那一份，refs 不再自带闭包。
    pub(crate) recovery: RecoveryCommands,
    /// 表单浮层归搜索功能：身份编辑要预填当前仓库级 user.name / user.email。
    pub(crate) search: SearchCommands,
    /// 同步功能的用例：Refs 表里远程行的「取回」打开同步面板。
    pub(crate) sync: SyncCommands,
}

impl RefsCommands {
    fn active_tab_id(&self) -> Option<u64> {
        return self
            .session
            .tabs
            .get_untracked()
            .get(self.session.active.get_untracked())
            .map(|tab| tab.id);
    }

    pub(crate) fn toggle_sidebar_fold(&self, tab_id: u64, key: String) {
        let mut list = self.session.tabs.get_untracked();
        let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) else {
            return;
        };
        if !tab.sidebar_collapsed.remove(key.as_str()) {
            tab.sidebar_collapsed.insert(key);
        }
        self.session.tabs.set(list);
    }

    pub(crate) fn set_sidebar_folds(&self, tab_id: u64, keys: Vec<String>, collapsed: bool) {
        let mut list = self.session.tabs.get_untracked();
        let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) else {
            return;
        };
        for key in keys {
            if collapsed {
                tab.sidebar_collapsed.insert(key);
            } else {
                tab.sidebar_collapsed.remove(key.as_str());
            }
        }
        self.session.tabs.set(list);
    }

    /// 检出分支（原 `checkout_branch`）：有改动时先问贮藏后切、带过去还是取消。
    pub(crate) fn checkout_branch(&self, branch: String) {
        let Some(tab_id) = self.active_tab_id() else {
            return;
        };
        self.checkout_branch_at(tab_id, branch);
    }

    pub(crate) fn checkout_branch_at(&self, tab_id: u64, branch: String) {
        let tabs = self.session.tabs;
        let language = self.language;
        let notice = self.notice;
        let dirty = tabs
            .get_untracked()
            .iter()
            .find(|tab| tab.id == tab_id)
            .is_some_and(ops::is_dirty);
        if !dirty {
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) {
                ops::checkout_branch(tab, branch.as_str());
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
        let pending = crate::ui::dialog::alert_with_actions(
            title,
            message,
            vec![("stash", stash_label), ("carry", carry_label)],
            None,
            cancel_label,
        );
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
            if let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) {
                ops::checkout_branch_with(tab, branch.as_str(), stash);
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
            if let Err(detail) = ops::checkout_new_branch(tab, name.as_str(), from.as_str()) {
                tab.error = Some(detail);
            }
            tabs.set(list);
        }
    }

    /// 删除分支（原 `delete_branch`）：不可撤销，先确认。
    pub(crate) fn delete_branch(&self, branch: String) {
        let Some(tab_id) = self.active_tab_id() else {
            return;
        };
        self.delete_branch_at(tab_id, branch);
    }

    pub(crate) fn delete_branch_at(&self, tab_id: u64, branch: String) {
        let tabs = self.session.tabs;
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
        spawn(async move {
            if let Ok(result) = pending.await {
                if result.action.as_deref() != Some("delete") {
                    return;
                }
                let mut list = tabs.get_untracked();
                if let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) {
                    ops::delete_branch(tab, branch.as_str());
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
            match ops::push_tag(tab, name.as_str()) {
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

    pub(crate) fn push_tag_to_remote_at(&self, tab_id: u64, name: String, remote: String) {
        let tabs = self.session.tabs;
        let notice = self.notice;
        let language = self.language;
        let mut list = tabs.get_untracked();
        let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) else {
            return;
        };
        if !tab.refs.tags.iter().any(|tag| tag == &name)
            || !tab.refs.remotes.iter().any(|item| item == &remote)
        {
            crate::interactions::notify(
                notice,
                crate::i18n::t(language.get_untracked().as_str(), "tag.targetChanged"),
            );
            return;
        }
        match ops::push_tag_to_remote(tab, name.as_str(), remote.as_str()) {
            Ok(_) => crate::interactions::notify(
                notice,
                crate::i18n::t(language.get_untracked().as_str(), "toast.pushed"),
            ),
            Err(detail) => crate::interactions::notify(notice, detail),
        }
        tabs.set(list);
    }

    pub(crate) fn update_submodule_at(&self, tab_id: u64, path: String, latest: bool) {
        let mut list = self.session.tabs.get_untracked();
        let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) else {
            return;
        };
        let result = ops::update_submodule(tab, path.as_str(), latest);
        self.session.tabs.set(list);
        match result {
            Ok(()) => crate::interactions::notify(
                self.notice,
                crate::i18n::t(
                    self.language.get_untracked().as_str(),
                    if latest {
                        "submodule.updatedLatest"
                    } else {
                        "submodule.updated"
                    },
                ),
            ),
            Err(error) => crate::interactions::notify(
                self.notice,
                crate::features::welcome::commands::localized_error(
                    self.language.get_untracked().as_str(),
                    &error,
                ),
            ),
        }
    }

    pub(crate) fn update_all_submodules_at(&self, tab_id: u64, paths: Vec<String>) {
        if paths.is_empty() {
            return;
        }
        let language = self.language;
        let language_now = language.get_untracked();
        let display_paths = paths.join("\n");
        let pending = crate::interactions::confirm(
            crate::i18n::t(language_now.as_str(), "submodule.updateAllTitle"),
            crate::i18n::with_arguments(
                language_now.as_str(),
                "submodule.updateAllMessage",
                &[("count", paths.len().to_string()), ("paths", display_paths)],
            ),
            "update",
            crate::i18n::t(language_now.as_str(), "submodule.updateAll"),
            crate::i18n::t(language_now.as_str(), "common.cancel"),
            false,
        );
        let tabs = self.session.tabs;
        let notice = self.notice;
        let tab_id_for_update = tab_id;
        spawn(async move {
            if !pending.await {
                return;
            }
            let mut list = tabs.get_untracked();
            let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id_for_update) else {
                return;
            };
            let mut failures = Vec::new();
            for path in paths {
                if let Err(error) = ops::update_submodule(tab, path.as_str(), false) {
                    let lang = language.get_untracked();
                    let message =
                        crate::features::welcome::commands::localized_error(lang.as_str(), &error);
                    failures.push(format!("{path}: {message}"));
                }
            }
            tabs.set(list);
            if failures.is_empty() {
                crate::interactions::notify(
                    notice,
                    crate::i18n::t(language.get_untracked().as_str(), "submodule.updatedAll"),
                );
            } else {
                let lang = language.get_untracked();
                crate::interactions::alert(
                    crate::i18n::t(lang.as_str(), "submodule.updateAllFailedTitle"),
                    crate::i18n::with_arguments(
                        lang.as_str(),
                        "submodule.updateAllFailedMessage",
                        &[("paths", failures.join("\n"))],
                    ),
                    crate::i18n::t(lang.as_str(), "common.close"),
                );
            }
        });
    }

    pub(crate) fn sync_submodule_at(&self, tab_id: u64, path: String) {
        let mut list = self.session.tabs.get_untracked();
        let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) else {
            return;
        };
        let result = ops::sync_submodule(tab, path.as_str());
        self.session.tabs.set(list);
        match result {
            Ok(()) => crate::interactions::notify(
                self.notice,
                crate::i18n::t(
                    self.language.get_untracked().as_str(),
                    "submodule.urlSynced",
                ),
            ),
            Err(error) => crate::interactions::notify(
                self.notice,
                crate::features::welcome::commands::localized_error(
                    self.language.get_untracked().as_str(),
                    &error,
                ),
            ),
        }
    }

    pub(crate) fn open_submodule_at(&self, tab_id: u64, path: String) {
        let Some(tab) = self
            .session
            .tabs
            .get_untracked()
            .into_iter()
            .find(|tab| tab.id == tab_id)
        else {
            return;
        };
        let Some(module) = tab
            .refs
            .submodules
            .iter()
            .find(|module| module.path == path)
        else {
            crate::interactions::notify(
                self.notice,
                crate::i18n::t(
                    self.language.get_untracked().as_str(),
                    "submodule.targetChanged",
                ),
            );
            return;
        };
        if !module.initialized {
            crate::interactions::notify(
                self.notice,
                crate::i18n::t(
                    self.language.get_untracked().as_str(),
                    "submodule.notInitialized",
                ),
            );
            return;
        }
        let parent = std::path::Path::new(tab.path.as_str()).canonicalize();
        let path = std::path::Path::new(tab.path.as_str())
            .join(path)
            .canonicalize();
        let Some(path) = path.ok().filter(|path| {
            parent
                .as_ref()
                .is_ok_and(|parent| path.as_path() != parent.as_path() && path.starts_with(parent))
        }) else {
            crate::interactions::notify(
                self.notice,
                crate::i18n::t(
                    self.language.get_untracked().as_str(),
                    "submodule.targetChanged",
                ),
            );
            return;
        };
        let Ok(repo) = git::Repo::open(path.to_string_lossy().as_ref()) else {
            crate::interactions::notify(
                self.notice,
                crate::i18n::t(
                    self.language.get_untracked().as_str(),
                    "submodule.targetChanged",
                ),
            );
            return;
        };
        if repo.root().canonicalize().ok().as_ref() != Some(&path) {
            crate::interactions::notify(
                self.notice,
                crate::i18n::t(
                    self.language.get_untracked().as_str(),
                    "submodule.targetChanged",
                ),
            );
            return;
        }
        self.session.open_path(repo.root().display().to_string());
    }

    pub(crate) fn stage_submodule_change_at(&self, tab_id: u64, path: String) {
        let tabs = self.session.tabs;
        let mut list = tabs.get_untracked();
        let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) else {
            return;
        };
        let staged_paths = match ops::stage_submodule_change(tab, path.as_str()) {
            Ok(Some(paths)) => paths,
            Ok(None) => {
                crate::interactions::notify(
                    self.notice,
                    crate::i18n::t(self.language.get_untracked().as_str(), "submodule.noChange"),
                );
                tabs.set(list);
                return;
            }
            Err(error) => {
                tabs.set(list);
                crate::interactions::notify(
                    self.notice,
                    crate::features::welcome::commands::localized_error(
                        self.language.get_untracked().as_str(),
                        &error,
                    ),
                );
                return;
            }
        };
        tabs.set(list);

        let language = self.language;
        let language_now = language.get_untracked();
        let pending = crate::interactions::confirm(
            crate::i18n::t(language_now.as_str(), "submodule.commitTitle"),
            crate::i18n::with_arguments(
                language_now.as_str(),
                "submodule.commitMessage",
                &[("paths", staged_paths.join("\n"))],
            ),
            "commit",
            crate::i18n::t(language_now.as_str(), "submodule.continueToCommit"),
            crate::i18n::t(language_now.as_str(), "common.cancel"),
            false,
        );
        let session = self.session.clone();
        let search = self.search.clone();
        let notice = self.notice;
        spawn(async move {
            if !pending.await {
                return;
            }
            if !session
                .tabs
                .get_untracked()
                .iter()
                .any(|tab| tab.id == tab_id)
            {
                crate::interactions::notify(
                    notice,
                    crate::i18n::t(language.get_untracked().as_str(), "submodule.targetChanged"),
                );
                return;
            }
            session.activate_tab_id(tab_id);
            search.state.mode.set(crate::Mode::Changes);
        });
    }

    pub(crate) fn remove_submodule_at(&self, tab_id: u64, path: String) {
        let language = self.language;
        let language_now = language.get_untracked();
        let pending = crate::interactions::confirm(
            crate::i18n::t(language_now.as_str(), "submodule.removeTitle"),
            crate::i18n::with_arguments(
                language_now.as_str(),
                "submodule.removeMessage",
                &[("path", path.clone())],
            ),
            "remove",
            crate::i18n::t(language_now.as_str(), "submodule.remove"),
            crate::i18n::t(language_now.as_str(), "common.cancel"),
            true,
        );
        let tabs = self.session.tabs;
        let notice = self.notice;
        spawn(async move {
            if !pending.await {
                return;
            }
            let mut list = tabs.get_untracked();
            let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) else {
                return;
            };
            let result = ops::remove_submodule(tab, path.as_str());
            tabs.set(list);
            match result {
                Ok(()) => crate::interactions::notify(
                    notice,
                    crate::i18n::t(language.get_untracked().as_str(), "submodule.removed"),
                ),
                Err(error) => crate::interactions::notify(
                    notice,
                    crate::features::welcome::commands::localized_error(
                        language.get_untracked().as_str(),
                        &error,
                    ),
                ),
            }
        });
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
                ops::apply_stash(tab, reference.as_str())
            } else {
                ops::pop_stash(tab, reference.as_str())
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

    fn stash_action_at(&self, kind: &'static str, tab_id: u64, oid: String) {
        let tabs = self.session.tabs;
        let language = self.language;
        let notice = self.notice;
        let mut list = tabs.get_untracked();
        let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) else {
            return;
        };
        let result = if kind == "apply" {
            ops::apply_stash_by_oid(tab, oid.as_str())
        } else {
            ops::pop_stash_by_oid(tab, oid.as_str())
        };
        tabs.set(list);
        match result {
            Ok(true) => crate::interactions::notify(
                notice,
                crate::i18n::t(language.get_untracked().as_str(), "toast.stashDone"),
            ),
            Ok(false) => crate::interactions::notify(
                notice,
                crate::i18n::t(language.get_untracked().as_str(), "stash.targetChanged"),
            ),
            Err(detail) => crate::interactions::notify(notice, detail),
        }
    }

    pub(crate) fn apply_stash_at(&self, tab_id: u64, oid: String) {
        self.stash_action_at("apply", tab_id, oid);
    }

    pub(crate) fn pop_stash_at(&self, tab_id: u64, oid: String) {
        self.stash_action_at("pop", tab_id, oid);
    }

    pub(crate) fn show_stash_diff_at(&self, tab_id: u64, oid: String) {
        let tabs = self.session.tabs;
        let notice = self.notice;
        let mut list = tabs.get_untracked();
        let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) else {
            return;
        };
        match ops::show_stash_diff(tab, oid.as_str()) {
            Ok(Some(diff)) => {
                tab.stash_diff = Some(diff);
                tabs.set(list);
                self.session.activate_tab_id(tab_id);
                self.search.state.mode.set(crate::Mode::Changes);
            }
            Ok(None) => crate::interactions::notify(
                notice,
                crate::i18n::t(
                    self.language.get_untracked().as_str(),
                    "stash.targetChanged",
                ),
            ),
            Err(detail) => crate::interactions::notify(notice, detail),
        }
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
            crate::i18n::t(language_now.as_str(), "common.cancel"),
            true,
        );
        spawn(async move {
            if !pending.await {
                return;
            }
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                let _ = ops::drop_stash(tab, reference.as_str());
                tabs.set(list);
            }
        });
    }

    pub(crate) fn drop_stash_at(
        &self,
        tab_id: u64,
        oid: String,
        reference: String,
        summary: String,
    ) {
        let tabs = self.session.tabs;
        let language = self.language;
        let notice = self.notice;
        let short_oid = oid.chars().take(8).collect::<String>();
        let display_name = format!("{reference} · {summary} ({short_oid})");
        let language_now = language.get_untracked();
        let pending = crate::interactions::confirm(
            crate::i18n::t(language_now.as_str(), "stash.dropTitle"),
            crate::i18n::with_arguments(
                language_now.as_str(),
                "stash.dropMessage",
                &[("name", display_name)],
            ),
            "drop",
            crate::i18n::t(language_now.as_str(), "stash.drop"),
            crate::i18n::t(language_now.as_str(), "common.cancel"),
            true,
        );
        spawn(async move {
            if !pending.await {
                return;
            }
            let mut list = tabs.get_untracked();
            let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) else {
                return;
            };
            match ops::drop_stash_by_oid(tab, oid.as_str()) {
                Ok(true) => crate::interactions::notify(
                    notice,
                    crate::i18n::t(language.get_untracked().as_str(), "toast.stashDone"),
                ),
                Ok(false) => crate::interactions::notify(
                    notice,
                    crate::i18n::t(language.get_untracked().as_str(), "stash.targetChanged"),
                ),
                Err(detail) => crate::interactions::notify(notice, detail),
            }
            tabs.set(list);
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
            crate::i18n::t(language_now.as_str(), "common.cancel"),
            true,
        );
        spawn(async move {
            if !pending.await {
                return;
            }
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                let _ = ops::drop_tag(tab, name.as_str());
                tabs.set(list);
            }
        });
    }

    pub(crate) fn drop_tag_at(&self, tab_id: u64, name: String) {
        let tabs = self.session.tabs;
        let language = self.language;
        let notice = self.notice;
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
            crate::i18n::t(language_now.as_str(), "common.cancel"),
            true,
        );
        spawn(async move {
            if !pending.await {
                return;
            }
            let mut list = tabs.get_untracked();
            let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) else {
                return;
            };
            if !tab.refs.tags.iter().any(|tag| tag == &name) {
                crate::interactions::notify(
                    notice,
                    crate::i18n::t(language.get_untracked().as_str(), "tag.targetChanged"),
                );
                return;
            }
            match ops::drop_tag(tab, name.as_str()) {
                Ok(()) => crate::interactions::notify(
                    notice,
                    crate::i18n::t(language.get_untracked().as_str(), "toast.historyDone"),
                ),
                Err(detail) => crate::interactions::notify(notice, detail),
            }
            tabs.set(list);
        });
    }

    /// 移除远程（原 `drop_remote`）：先确认。
    pub(crate) fn drop_remote(&self, name: String) {
        let Some(tab_id) = self.active_tab_id() else {
            return;
        };
        self.drop_remote_at(tab_id, name);
    }

    pub(crate) fn drop_remote_at(&self, tab_id: u64, name: String) {
        let tabs = self.session.tabs;
        let language = self.language;
        let notice = self.notice;
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
            crate::i18n::t(language_now.as_str(), "common.cancel"),
            true,
        );
        spawn(async move {
            if !pending.await {
                return;
            }
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) {
                if !tab.refs.remotes.iter().any(|remote| remote == &name) {
                    crate::interactions::notify(
                        notice,
                        crate::i18n::t(language.get_untracked().as_str(), "remote.targetChanged"),
                    );
                    return;
                }
                let result = ops::drop_remote(tab, name.as_str());
                tabs.set(list);
                if let Err(detail) = result {
                    crate::interactions::notify(notice, detail);
                }
            }
        });
    }

    /// 把某个引用合并进当前分支（原 `merge_reference`）。
    pub(crate) fn merge_reference(&self, reference: String) {
        let Some(tab_id) = self.active_tab_id() else {
            return;
        };
        self.merge_reference_at(tab_id, reference);
    }

    pub(crate) fn merge_reference_at(&self, tab_id: u64, reference: String) {
        let tabs = self.session.tabs;
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) {
            if let Err(detail) = ops::merge_reference(tab, reference.as_str()) {
                tab.error = Some(detail);
            }
            tabs.set(list);
        }
    }

    pub(crate) fn rebase_current_on(
        &self,
        tab_id: u64,
        current_branch: String,
        target_branch: String,
    ) {
        let tabs = self.session.tabs;
        let Some(tab) = tabs
            .get_untracked()
            .into_iter()
            .find(|tab| tab.id == tab_id)
        else {
            return;
        };
        if ops::is_dirty(&tab) || tab.operation != git::Operation::None {
            return;
        }
        let language_now = self.language.get_untracked();
        let pending = crate::interactions::confirm(
            crate::i18n::t(language_now.as_str(), "confirm.rebaseBranchTitle"),
            crate::i18n::with_arguments(
                language_now.as_str(),
                "confirm.rebaseBranchMessage",
                &[
                    ("current", current_branch.clone()),
                    ("target", target_branch.clone()),
                ],
            ),
            "rebase-branch",
            crate::i18n::t(language_now.as_str(), "confirm.rebaseBranchAction"),
            crate::i18n::t(language_now.as_str(), "common.cancel"),
            true,
        );
        let language = self.language;
        let notice = self.notice;
        spawn(async move {
            if !pending.await {
                return;
            }
            let mut list = tabs.get_untracked();
            let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) else {
                return;
            };
            if tab.branch != current_branch
                || !(tab
                    .refs
                    .branches
                    .iter()
                    .any(|branch| branch.name == target_branch)
                    || tab
                        .refs
                        .remote_branches
                        .iter()
                        .any(|branch| branch.full_name() == target_branch))
            {
                crate::interactions::notify(
                    notice,
                    crate::i18n::t(language.get_untracked().as_str(), "branch.targetChanged"),
                );
                return;
            }
            if ops::is_dirty(tab) || tab.operation != git::Operation::None {
                crate::interactions::notify(
                    notice,
                    crate::i18n::t(
                        language.get_untracked().as_str(),
                        "branch.rebaseRequiresClean",
                    ),
                );
                return;
            }
            match ops::rebase_reference(tab, target_branch.as_str()) {
                Ok(()) => {
                    let rebase_conflict = tab.operation == git::Operation::Rebase;
                    tabs.set(list);
                    if !rebase_conflict {
                        crate::interactions::notify(
                            notice,
                            crate::i18n::t(language.get_untracked().as_str(), "toast.historyDone"),
                        );
                    }
                }
                Err(detail) => {
                    tab.error = Some(detail.clone());
                    tabs.set(list);
                    crate::interactions::notify(notice, detail);
                }
            }
        });
    }

    pub(crate) fn push_branch_at(&self, tab_id: u64, branch: String) {
        let tabs = self.session.tabs;
        let mut list = tabs.get_untracked();
        let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) else {
            return;
        };
        match ops::push_branch(tab, branch.as_str(), "origin") {
            Ok(()) => {
                tabs.set(list);
                crate::interactions::notify(
                    self.notice,
                    crate::i18n::t(self.language.get_untracked().as_str(), "toast.pushed"),
                );
            }
            Err(detail) => {
                tabs.set(list);
                crate::interactions::notify(self.notice, detail);
            }
        }
    }

    pub(crate) fn pull_remote_branch_at(&self, tab_id: u64, remote: String, branch: String) {
        let tabs = self.session.tabs;
        let language = self.language;
        let notice = self.notice;
        let Some(tab) = tabs
            .get_untracked()
            .into_iter()
            .find(|tab| tab.id == tab_id)
        else {
            return;
        };
        if ops::is_dirty(&tab) || tab.operation != git::Operation::None {
            crate::interactions::notify(
                notice,
                crate::i18n::t(
                    language.get_untracked().as_str(),
                    "remote.pullRequiresClean",
                ),
            );
            return;
        }
        let current_branch = tab.branch.clone();
        let full_name = format!("{remote}/{branch}");
        let language_now = language.get_untracked();
        let pending = crate::interactions::confirm(
            crate::i18n::t(language_now.as_str(), "remote.pullTitle"),
            crate::i18n::with_arguments(
                language_now.as_str(),
                "remote.pullMessage",
                &[
                    ("source", full_name.clone()),
                    ("target", current_branch.clone()),
                ],
            ),
            "pull",
            crate::i18n::t(language_now.as_str(), "remote.pullAction"),
            crate::i18n::t(language_now.as_str(), "common.cancel"),
            false,
        );
        spawn(async move {
            if !pending.await {
                return;
            }
            let mut list = tabs.get_untracked();
            let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) else {
                return;
            };
            if tab.branch != current_branch
                || !tab.refs.remotes.iter().any(|item| item == &remote)
                || !tab
                    .refs
                    .remote_branches
                    .iter()
                    .any(|item| item.remote == remote && item.name == branch)
            {
                crate::interactions::notify(
                    notice,
                    crate::i18n::t(language.get_untracked().as_str(), "remote.targetChanged"),
                );
                return;
            }
            if ops::is_dirty(tab) || tab.operation != git::Operation::None {
                crate::interactions::notify(
                    notice,
                    crate::i18n::t(
                        language.get_untracked().as_str(),
                        "remote.pullRequiresClean",
                    ),
                );
                return;
            }
            let result = ops::pull_remote_branch(tab, remote.as_str(), branch.as_str());
            tabs.set(list);
            match result {
                Ok(_) => crate::interactions::notify(
                    notice,
                    crate::i18n::t(language.get_untracked().as_str(), "toast.historyDone"),
                ),
                Err(detail) => crate::interactions::notify(notice, detail),
            }
        });
    }

    pub(crate) fn delete_remote_branch_at(&self, tab_id: u64, remote: String, branch: String) {
        let tabs = self.session.tabs;
        let language = self.language;
        let notice = self.notice;
        let full_name = format!("{remote}/{branch}");
        let language_now = language.get_untracked();
        let pending = crate::interactions::confirm(
            crate::i18n::with_arguments(
                language_now.as_str(),
                "remote.deleteBranchTitle",
                &[("name", full_name.clone())],
            ),
            crate::i18n::with_arguments(
                language_now.as_str(),
                "remote.deleteBranchMessage",
                &[("name", full_name.clone())],
            ),
            "delete",
            crate::i18n::t(language_now.as_str(), "branch.deleteAction"),
            crate::i18n::t(language_now.as_str(), "common.cancel"),
            true,
        );
        spawn(async move {
            if !pending.await {
                return;
            }
            let mut list = tabs.get_untracked();
            let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) else {
                return;
            };
            if !tab.refs.remotes.iter().any(|item| item == &remote)
                || !tab
                    .refs
                    .remote_branches
                    .iter()
                    .any(|item| item.remote == remote && item.name == branch)
            {
                crate::interactions::notify(
                    notice,
                    crate::i18n::t(language.get_untracked().as_str(), "remote.targetChanged"),
                );
                return;
            }
            match ops::delete_remote_branch(tab, remote.as_str(), branch.as_str()) {
                Ok(()) => crate::interactions::notify(
                    notice,
                    crate::i18n::t(language.get_untracked().as_str(), "toast.historyDone"),
                ),
                Err(detail) => crate::interactions::notify(notice, detail),
            }
            tabs.set(list);
        });
    }

    /// 切到某个提交（原 `checkout_commit`，detached，先确认）。
    pub(crate) fn checkout_commit(&self, sha: String) {
        self.recovery.checkout_commit(sha);
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
            match ops::checkout_default_branch(tab) {
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
        let list = tabs.get_untracked();
        let Some(tab) = list.get(active.get_untracked()) else {
            return;
        };
        let (name, email) = identity(tab);
        self.search
            .open_form_prefilled(FormKind::Identity, name, email);
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
        match ops::clear_identity(tab) {
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
