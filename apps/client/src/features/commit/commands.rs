//! 提交的用例：提交、跳过钩子、面板开关、钩子日志弹层与历史改写。
//!
//! 方法体由原 `Actions` 里的闭包原样搬来，只把捕获的局部变量换成 `self.*`；
//! 这里只调本功能的 `ops` 并更新状态，不写布局。

use super::ops;
use super::state::{CommitState, RewriteKind, RewriteState};
use crate::app::session::{CommitFeedbackKind, WorkflowProgress};
use crate::features::SessionHandle;
use crate::features::sync::commands::SyncCommands;
use nsh_core::prelude::*;

#[derive(Clone)]
pub(crate) struct CommitCommands {
    pub(crate) state: CommitState,
    pub(crate) session: SessionHandle,
    /// 确认框文案与成功提示要按当前语言取。
    pub(crate) language: Signal<SharedString>,
    /// 状态栏短提示（提交后推送、改写完成）。
    pub(crate) notice: Signal<Option<String>>,
    pub(crate) sync: SyncCommands,
}

impl CommitCommands {
    /// 提交暂存内容（原 `commit`）。
    pub(crate) fn commit(&self) {
        if self.no_verify_selected() {
            self.confirm_no_verify_commit();
        } else {
            self.commit_inner(false);
        }
    }

    /// 提交本体：成功后按「Push after commit」决定是否立刻推送（原 `commit_inner`）。
    fn commit_inner(&self, no_verify: bool) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let commit_summary = self.state.summary;
        let commit_description = self.state.description;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        let Some(tab) = list.get_mut(current) else {
            return;
        };
        let tab_id = tab.id;
        let push_after_requested = tab.push_after_commit;
        tab.summary = commit_summary.get_untracked();
        tab.description = commit_description.get_untracked();
        ops::commit_staged_with(tab, no_verify, self.language.get_untracked().as_str());
        if push_after_requested {
            let commit_succeeded = tab.hook_ok;
            tab.commit_feedback_progress = Some(WorkflowProgress {
                step: if commit_succeeded { 2 } else { 1 },
                total: 2,
                running: commit_succeeded,
            });
            if commit_succeeded {
                tab.commit_feedback_kind = CommitFeedbackKind::PushRunning;
            }
        }
        if tab.hook_ok {
            commit_summary.set(String::new());
            commit_description.set(String::new());
        }
        let push_after = push_after_requested && tab.hook_ok;
        let force_push_after = tab.force_push_after_commit;
        if push_after {
            tab.push_after_commit = false;
            tab.force_push_after_commit = false;
        }
        tabs.set(list);
        if push_after {
            if !self.sync.run_push_after_commit(tab_id, force_push_after) {
                self.report_push_not_started(tab_id);
            }
        }
    }

    fn report_push_not_started(&self, tab_id: u64) {
        let tabs = self.session.tabs;
        let mut list = tabs.get_untracked();
        let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) else {
            return;
        };
        tab.commit_feedback_kind = CommitFeedbackKind::Push;
        tab.commit_feedback_progress = Some(WorkflowProgress {
            step: 2,
            total: 2,
            running: false,
        });
        tab.hook_ok = false;
        tab.hook_log = Some(crate::i18n::t(
            self.language.get_untracked().as_str(),
            "commit.pushNotStarted",
        ));
        tabs.set(list);
    }

    /// 跳过钩子提交（原 `commit_without_hooks`）：先强确认再提交（`--no-verify`）。
    pub(crate) fn commit_without_hooks(&self) {
        self.confirm_no_verify_commit();
    }

    fn no_verify_selected(&self) -> bool {
        let current = self.session.active.get_untracked();
        return self
            .session
            .tabs
            .with_untracked(|tabs| tabs.get(current).is_some_and(|tab| tab.no_verify));
    }

    fn confirm_no_verify_commit(&self) {
        let language = self.language;
        let language_now = language.get_untracked();
        let pending = crate::interactions::confirm(
            crate::i18n::t(language_now.as_str(), "confirm.noVerifyTitle"),
            crate::i18n::t(language_now.as_str(), "confirm.noVerifyMessage"),
            "no_verify",
            crate::i18n::t(language_now.as_str(), "confirm.noVerifyAction"),
            crate::i18n::t(language_now.as_str(), "common.cancel"),
            true,
        );
        let this = self.clone();
        spawn(async move {
            if pending.await {
                this.commit_inner(true);
            }
        });
    }

    /// 更新 Changes 面板中的提交选项。
    pub(crate) fn set_commit_option(&self, kind: &'static str, value: bool) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if kind == "amend" && value {
            if let Some(tab) = list.get(current)
                && let Ok(repo) = tab.open_repo()
                && let Ok(sha) = repo.resolve_commit("HEAD")
                && let Ok(details) = repo.commit_details(sha.as_str())
            {
                self.state.summary.set(details.subject);
                self.state.description.set(details.body);
            }
        }
        if let Some(tab) = list.get_mut(current) {
            match kind {
                "amend" => tab.amend = value,
                "signoff" => tab.signoff = value,
                "pushAfter" => {
                    tab.push_after_commit = value;
                    if !value {
                        tab.force_push_after_commit = false;
                    }
                }
                "forcePush" => tab.force_push_after_commit = value && tab.push_after_commit,
                "noVerify" => tab.no_verify = value,
                _ => return,
            }
            tabs.set(list);
        }
    }

    /// 打开或关闭钩子完整日志弹层（原 `toggle_hook_log`）。
    pub(crate) fn toggle_hook_log(&self, open: bool) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            tab.hook_open = open;
            tabs.set(list);
        }
    }

    /// 关闭提交结果块，并同步关闭其完整日志弹层。
    pub(crate) fn dismiss_hook_status(&self) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            tab.hook_log = None;
            tab.hook_open = false;
            tab.commit_feedback_kind = CommitFeedbackKind::Hooks;
            tabs.set(list);
        }
    }

    /// 打开「改写提交说明」弹窗（原 `open_reword`）。
    pub(crate) fn open_reword(&self, sha: String, short: String, summary: String) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let rewrite_state = self.state.rewrite;
        let blocker = {
            let list = tabs.get_untracked();
            list.get(active.get_untracked())
                .and_then(ops::rewrite_blocker)
        };
        rewrite_state.set(Some(RewriteState::new(
            RewriteKind::Reword {
                sha,
                short,
                summary: summary.clone(),
            },
            summary,
            blocker,
        )));
    }

    /// 打开「合并提交」弹窗（原 `open_squash`）。
    pub(crate) fn open_squash(&self, from: String, to: String, commits: Vec<(String, String)>) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let rewrite_state = self.state.rewrite;
        let blocker = {
            let list = tabs.get_untracked();
            list.get(active.get_untracked())
                .and_then(ops::rewrite_blocker)
        };
        // 结果说明默认取最旧那条提交的说明，用户可改。
        let message = commits
            .last()
            .map(|(_, summary)| summary.clone())
            .unwrap_or_default();
        rewrite_state.set(Some(RewriteState::new(
            RewriteKind::Squash { from, to, commits },
            message,
            blocker,
        )));
    }

    /// 改写弹窗里编辑说明（原 `set_rewrite_message`）。
    pub(crate) fn set_rewrite_message(&self, message: String) {
        let rewrite_state = self.state.rewrite;
        let mut state = match rewrite_state.get_untracked() {
            Some(state) => state,
            None => return,
        };
        state.message = message;
        state.error = None;
        rewrite_state.set(Some(state));
    }

    /// 提交改写（原 `submit_rewrite`）。
    pub(crate) fn submit_rewrite(&self) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let rewrite_state = self.state.rewrite;
        let notice = self.notice;
        let language = self.language;
        let Some(state) = rewrite_state.get_untracked() else {
            return;
        };
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        let Some(tab) = list.get_mut(current) else {
            return;
        };
        let result = match &state.kind {
            RewriteKind::Reword { sha, .. } => {
                ops::reword_commit(tab, sha.as_str(), state.message.as_str())
            }
            RewriteKind::Squash { from, to, .. } => {
                ops::squash_commits(tab, from.as_str(), to.as_str(), state.message.as_str())
            }
        };
        match result {
            Ok(()) => {
                tabs.set(list);
                rewrite_state.set(None);
                let lang = language.get_untracked();
                crate::interactions::notify(notice, crate::i18n::t(lang.as_str(), "rewrite.done"));
            }
            Err(detail) => {
                tabs.set(list);
                let mut next = state;
                next.error = Some(detail);
                rewrite_state.set(Some(next));
            }
        }
    }

    /// 关闭改写弹窗（原 `close_rewrite`）。
    pub(crate) fn close_rewrite(&self) {
        let rewrite_state = self.state.rewrite;
        rewrite_state.set(None);
    }

    /// 空仓库里创建第一个提交（原 `create_initial_commit`）。
    pub(crate) fn create_initial_commit(&self) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let notice = self.notice;
        let language = self.language;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            let lang = language.get_untracked();
            let message = crate::i18n::t(lang.as_str(), "state.initialCommitMessage");
            match ops::create_initial_commit(tab, message.as_str()) {
                Ok(()) => crate::interactions::notify(
                    notice,
                    crate::i18n::t(lang.as_str(), "toast.historyDone"),
                ),
                Err(detail) => crate::interactions::notify(notice, detail),
            }
            tabs.set(list);
        }
    }
}
