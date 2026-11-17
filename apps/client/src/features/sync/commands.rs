//! 同步的用例：打开/关闭面板、切换选项、执行 fetch/pull/push 与清掉失败状态。
//!
//! 方法体由原 `Actions` 里的闭包原样搬来，只把捕获的局部变量换成 `self.*`；
//! 这里只调本功能的 `ops` 并更新状态，不写布局。

use super::ops;
use super::state::{SyncDialogState, SyncState};
use crate::app::session::{Sync, SyncOptions};
use crate::features::SessionHandle;
use nsh_core::prelude::*;

#[derive(Clone)]
pub(crate) struct SyncCommands {
    pub(crate) state: SyncState,
    pub(crate) session: SessionHandle,
    /// 确认框与结果提示要按当前语言取。
    pub(crate) language: Signal<SharedString>,
    /// 状态栏短提示（非快进回退的拉取结果）。
    pub(crate) notice: Signal<Option<String>>,
}

impl SyncCommands {
    /// 打开同步面板；已经打开时只切 tab，保留当前选项（原 `open_sync`）。
    pub(crate) fn open_sync(&self, kind: Sync) {
        let sync_state = self.state.dialog;
        let mut state = sync_state
            .get_untracked()
            .unwrap_or_else(|| SyncDialogState::new(kind));
        state.kind = kind;
        state.result = None;
        sync_state.set(Some(state));
    }

    /// 选项开关（原 `set_sync_option`）：键名固定，策略三项互斥。
    pub(crate) fn set_sync_option(&self, key: &'static str, value: bool) {
        let sync_state = self.state.dialog;
        let Some(mut state) = sync_state.get_untracked() else {
            return;
        };
        match key {
            "prune" => state.options.fetch.prune = value,
            "fetchTags" => state.options.fetch.tags = value,
            "allRemotes" => state.options.fetch.all_remotes = value,
            "autostash" => state.options.pull.autostash = value,
            "rebaseLocal" => state.options.pull.rebase_local = value,
            "pushTags" => state.options.push.tags = value,
            "forceLease" => state.options.push.force_with_lease = value,
            "allBranches" => state.options.push.all_branches = value,
            "strategyMerge" => state.options.pull.strategy = git::PullStrategy::Merge,
            "strategyRebase" => state.options.pull.strategy = git::PullStrategy::Rebase,
            "strategyFfOnly" => state.options.pull.strategy = git::PullStrategy::FfOnly,
            _ => return,
        }
        sync_state.set(Some(state));
    }

    /// 执行同步动作，并把结果留在面板里（原 `run_sync`）。
    ///
    /// 推送被远端拒绝（非快进）时不自动强推，而是弹「拉取变基 / 拉取合并 / 取消」。
    pub(crate) fn run_sync(&self, kind: Sync) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let sync_state = self.state.dialog;
        let Some(state) = sync_state.get_untracked() else {
            return;
        };
        let options = state.options.clone();
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            ops::sync_with(tab, kind, &options);
            let result = match (&tab.sync_message, tab.sync_failed) {
                (Some(message), false) => Ok(message.clone()),
                (Some(message), true) => Err(message.clone()),
                (None, _) => Ok(String::new()),
            };
            let rejected = kind == Sync::Push
                && tab.sync_failed
                && tab.sync_fault == Some(domain::NetworkFault::Rejected);
            tabs.set(list);
            sync_state.set(Some(SyncDialogState {
                kind,
                options: options.clone(),
                result: Some(result),
                running: false,
            }));
            if !rejected {
                return;
            }
            // 远端已前进：不自动强推，让用户在「拉取变基 / 拉取合并 / 取消」里选。
            let lang = self.language.get_untracked();
            let title = crate::i18n::t(lang.as_str(), "confirm.nonFastForwardTitle");
            let message = crate::i18n::t(lang.as_str(), "confirm.nonFastForwardMessage");
            let rebase_label = crate::i18n::t(lang.as_str(), "confirm.nonFastForwardRebase");
            let merge_label = crate::i18n::t(lang.as_str(), "confirm.nonFastForwardMerge");
            let cancel_label = crate::i18n::t(lang.as_str(), "common.cancel");
            let pending = crate::ui::dialog::alert_with_actions(
                title,
                message,
                vec![("rebase", rebase_label), ("merge", merge_label)],
                None,
                cancel_label,
            );
            let language = self.language;
            let notice = self.notice;
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
                let strategy = if action.as_str() == "rebase" {
                    git::PullStrategy::Rebase
                } else {
                    git::PullStrategy::Merge
                };
                let pull_options = SyncOptions {
                    pull: git::PullOptions {
                        strategy,
                        ..options.pull.clone()
                    },
                    ..options.clone()
                };
                let mut list = tabs.get_untracked();
                if let Some(tab) = list.get_mut(active.get_untracked()) {
                    ops::sync_with(tab, Sync::Pull, &pull_options);
                    let pulled = !tab.sync_failed;
                    // 拉取成功后再推一次；失败就只报拉取结果，不再连环弹框。
                    if pulled {
                        ops::sync_with(tab, Sync::Push, &options);
                    }
                    let outcome = match (&tab.sync_message, tab.sync_failed) {
                        (Some(message), false) => Some((message.clone(), false)),
                        (Some(message), true) => Some((message.clone(), true)),
                        (None, _) => None,
                    };
                    tabs.set(list);
                    if let Some((message, failed)) = outcome {
                        if failed {
                            crate::interactions::notify(notice, message);
                        } else {
                            let lang = language.get_untracked();
                            crate::interactions::notify(
                                notice,
                                crate::i18n::t(lang.as_str(), "toast.pushed"),
                            );
                        }
                    }
                }
            });
        }
    }

    /// 关闭同步面板（原 `close_sync`）。
    pub(crate) fn close_sync(&self) {
        let sync_state = self.state.dialog;
        sync_state.set(None);
    }

    /// 清掉同步失败状态（原 `clear_sync_fault`，重试或用户主动关闭横幅）。
    pub(crate) fn clear_sync_fault(&self) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            ops::clear_sync_fault(tab);
            tabs.set(list);
        }
    }
}
