//! 引用管理的用例：检出/新建/删除分支，推送/删除标签，远程、贮藏、身份与引用合并。
//!
//! 方法体由原 `Actions` 里的闭包原样搬来，只把捕获的局部变量换成 `self.*`；
//! 这里只调本功能的 `ops` 与 `app::ops`，不写布局。

use super::ops;
use super::state::{
    ReflogDetailState, ReflogEntryKey, ReflogFileDiffState, ReflogFileDiffStatus, RefsState,
    RefsTab,
};
use crate::app::ops::identity;
use crate::app::session::Tab;
use crate::features::SessionHandle;
use crate::features::history::commands::HistoryCommands;
use crate::features::recovery::commands::RecoveryCommands;
use crate::features::search::commands::SearchCommands;
use crate::features::search::form_dialog::FormKind;
use crate::features::sync::commands::SyncCommands;
use nsh_core::prelude::*;
use std::sync::{Arc, mpsc};
use std::time::Duration;

fn reflog_selection_exists(tabs: &[Tab], selection: &ReflogEntryKey) -> bool {
    return tabs.iter().any(|tab| {
        tab.id == selection.tab_id
            && tab.path == selection.path
            && tab.reflog.get(selection.index).is_some_and(|entry| {
                selection.matches_entry(tab.id, tab.path.as_str(), selection.index, entry)
            })
    });
}

#[derive(Clone)]
enum RemoteOperation {
    PushTag {
        name: String,
        remote: Option<String>,
    },
    PushBranch {
        branch: String,
        remote: String,
    },
    UpdateSubmodule {
        path: String,
        latest: bool,
    },
    UpdateSubmodules {
        paths: Vec<String>,
    },
    PullBranch {
        remote: String,
        branch: String,
    },
    DeleteBranch {
        remote: String,
        branch: String,
    },
}

impl RemoteOperation {
    fn run(&self, tab: &mut crate::app::session::Tab) -> Result<(), String> {
        match self {
            Self::PushTag { name, remote } => {
                let result = if let Some(remote) = remote {
                    ops::push_tag_to_remote(tab, name, remote)
                } else {
                    ops::push_tag(tab, name)
                };
                result.map(|_| ())
            }
            Self::PushBranch { branch, remote } => ops::push_branch(tab, branch, remote),
            Self::UpdateSubmodule { path, latest } => {
                ops::update_submodule(tab, path, *latest).map_err(|error| error.detail())
            }
            Self::UpdateSubmodules { paths } => {
                let failures = paths
                    .iter()
                    .filter_map(|path| {
                        ops::update_submodule(tab, path, false)
                            .err()
                            .map(|error| format!("{path}: {}", error.detail()))
                    })
                    .collect::<Vec<_>>();
                if failures.is_empty() {
                    Ok(())
                } else {
                    Err(failures.join("\n"))
                }
            }
            Self::PullBranch { remote, branch } => {
                ops::pull_remote_branch(tab, remote, branch).map(|_| ())
            }
            Self::DeleteBranch { remote, branch } => ops::delete_remote_branch(tab, remote, branch),
        }
    }

    fn apply_tab_updates(
        &self,
        target: &mut crate::app::session::Tab,
        updated: &crate::app::session::Tab,
    ) {
        match self {
            Self::PushTag { .. } | Self::PushBranch { .. } => {}
            Self::DeleteBranch { .. } => {
                target.refs = updated.refs.clone();
                target.remote_urls = updated.remote_urls.clone();
            }
            Self::UpdateSubmodule { .. } | Self::UpdateSubmodules { .. } => {
                target.refs = updated.refs.clone();
                target.remote_urls = updated.remote_urls.clone();
                target.status = updated.status.clone();
                target.changes = updated.changes.clone();
            }
            Self::PullBranch { .. } => {
                target.refs = updated.refs.clone();
                target.remote_urls = updated.remote_urls.clone();
                target.status = updated.status.clone();
                target.changes = updated.changes.clone();
                target.branch = updated.branch.clone();
                target.ahead = updated.ahead;
                target.behind = updated.behind;
                target.commits = updated.commits.clone();
                target.conflicts = updated.conflicts.clone();
                target.operation = updated.operation;
                target.head = updated.head.clone();
                target.reflog = updated.reflog.clone();
                target.error = updated.error.clone();
            }
        }
    }

    fn success_notice_key(&self) -> &'static str {
        match self {
            Self::PushTag { .. } | Self::PushBranch { .. } => "toast.pushed",
            Self::UpdateSubmodule { latest: true, .. } => "submodule.updatedLatest",
            Self::UpdateSubmodule { latest: false, .. } => "submodule.updated",
            Self::UpdateSubmodules { .. } => "submodule.updatedAll",
            Self::PullBranch { .. } | Self::DeleteBranch { .. } => "toast.historyDone",
        }
    }
}

struct RemoteOperationResult {
    tab: crate::app::session::Tab,
    result: Result<(), String>,
    cancelled: bool,
}

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
    pub(crate) askpass: crate::askpass::AskPassClient,
    /// Root session and auxiliary-window registry guard worktree removal.
    pub(crate) main_tabs: Signal<Vec<crate::app::session::Tab>>,
    pub(crate) worktree_windows: Signal<Vec<(String, nsh_core::element::desktop::WindowRef)>>,
    pub(crate) open_worktree_window: Option<Arc<dyn Fn(u64, String)>>,
}

impl RefsCommands {
    fn run_remote_operation(&self, tab_id: u64, operation: RemoteOperation) {
        let tabs = self.session.tabs;
        let Some(tab) = tabs
            .get_untracked()
            .into_iter()
            .find(|tab| tab.id == tab_id)
        else {
            return;
        };
        let path = tab.path.clone();
        let (sender, receiver) = mpsc::channel();
        let askpass = self.askpass.clone();
        let operation_for_worker = operation.clone();
        std::thread::spawn(move || {
            let executable = tab.git_executable.clone();
            let mut tab = tab;
            let result = match crate::askpass::AskPassSession::new(executable.clone(), askpass) {
                Ok(session) => {
                    tab.git_executable = session.executable().clone();
                    let result = operation_for_worker.run(&mut tab);
                    let cancelled = session.was_cancelled();
                    tab.git_executable = executable;
                    let _ = sender.send(RemoteOperationResult {
                        tab,
                        result,
                        cancelled,
                    });
                    return;
                }
                Err(error) => Err(error.to_string()),
            };
            let _ = sender.send(RemoteOperationResult {
                tab,
                result,
                cancelled: false,
            });
        });

        let language = self.language;
        let notice = self.notice;
        let operation_for_ui = operation;
        Owner::detached().run(|| {
            spawn(async move {
                loop {
                    match receiver.try_recv() {
                        Ok(result) => {
                            let mut current_tabs = tabs.get_untracked();
                            if let Some(current) = current_tabs
                                .iter_mut()
                                .find(|tab| tab.id == tab_id && tab.path == path)
                            {
                                operation_for_ui.apply_tab_updates(current, &result.tab);
                                tabs.set(current_tabs);
                            }
                            let lang = language.get_untracked();
                            if result.cancelled {
                                crate::interactions::notify(
                                    notice,
                                    crate::i18n::t(lang.as_str(), "askpass.cancelled"),
                                );
                            } else if let Err(detail) = result.result {
                                crate::interactions::notify(notice, detail);
                            } else {
                                crate::interactions::notify(
                                    notice,
                                    crate::i18n::t(
                                        lang.as_str(),
                                        operation_for_ui.success_notice_key(),
                                    ),
                                );
                            }
                            return;
                        }
                        Err(mpsc::TryRecvError::Disconnected) => return,
                        Err(mpsc::TryRecvError::Empty) => timer(Duration::from_millis(50)).await,
                    }
                }
            });
        });
    }

    /// 从工具栏打开引用主区的 Reflog 页签，并显示提交检查器。
    pub(crate) fn open_reflog(&self) {
        self.state.selected_tab.set(RefsTab::Reflog);
        self.search.state.mode.set(crate::Mode::Refs);
        self.search.right_panel_open.set(true);
    }

    /// 选择一条 reflog 记录并只读加载其提交元信息与文件统计。
    pub(crate) fn select_reflog_entry(&self, selection: ReflogEntryKey) {
        let tabs = self.session.tabs.get_untracked();
        let active = self.session.active.get_untracked();
        let Some(tab) = tabs.get(active).filter(|tab| {
            tab.id == selection.tab_id
                && tab.path == selection.path
                && tab.reflog.get(selection.index).is_some_and(|entry| {
                    selection.matches_entry(tab.id, tab.path.as_str(), selection.index, entry)
                })
        }) else {
            return;
        };

        let current = self.state.reflog_detail.get_untracked();
        let previous_selection = self.state.reflog_selection.get_untracked();
        if previous_selection.as_ref() != Some(&selection) {
            self.state.reflog_file_diff.set(None);
        }
        let already_loading_or_loaded = self
            .state
            .reflog_selection
            .get_untracked()
            .as_ref()
            .is_some_and(|selected| selected == &selection)
            && current.as_ref().is_some_and(|detail| match detail {
                ReflogDetailState::Loading(selected) => selected == &selection,
                ReflogDetailState::Loaded {
                    selection: selected,
                    ..
                } => selected == &selection,
                ReflogDetailState::Failed(_) => false,
            });
        self.state.selected_tab.set(RefsTab::Reflog);
        self.state.reflog_selection.set(Some(selection.clone()));
        self.search.right_panel_open.set(true);
        if already_loading_or_loaded {
            return;
        }

        self.state
            .reflog_detail
            .set(Some(ReflogDetailState::Loading(selection.clone())));
        let path = tab.path.clone();
        let git_executable = tab.git_executable.clone();
        let sha = selection.sha.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = git::Repo::open_with_executable(path.as_str(), &git_executable)
                .and_then(|repo| {
                    let details = repo.commit_details(sha.as_str())?;
                    let files = repo.numstat(sha.as_str())?;
                    Ok((details, files))
                })
                .map_err(|_| ());
            let _ = sender.send(result);
        });

        let detail_state = self.state.reflog_detail;
        let current_selection = self.state.reflog_selection;
        let tabs = self.session.tabs;
        Owner::detached().run(|| {
            spawn(async move {
                loop {
                    match receiver.try_recv() {
                        Ok(result) => {
                            let still_exists = tabs.get_untracked().iter().any(|tab| {
                                tab.id == selection.tab_id
                                    && tab.path == selection.path
                                    && tab.reflog.get(selection.index).is_some_and(|entry| {
                                        selection.matches_entry(
                                            tab.id,
                                            tab.path.as_str(),
                                            selection.index,
                                            entry,
                                        )
                                    })
                            });
                            let still_selected = current_selection
                                .get_untracked()
                                .as_ref()
                                .is_some_and(|selected| selected == &selection);
                            if still_exists && still_selected {
                                detail_state.set(Some(match result {
                                    Ok((details, files)) => ReflogDetailState::Loaded {
                                        selection,
                                        details,
                                        files,
                                    },
                                    Err(()) => ReflogDetailState::Failed(selection),
                                }));
                            }
                            return;
                        }
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            let still_exists = tabs.get_untracked().iter().any(|tab| {
                                tab.id == selection.tab_id
                                    && tab.path == selection.path
                                    && tab.reflog.get(selection.index).is_some_and(|entry| {
                                        selection.matches_entry(
                                            tab.id,
                                            tab.path.as_str(),
                                            selection.index,
                                            entry,
                                        )
                                    })
                            });
                            if still_exists
                                && current_selection
                                    .get_untracked()
                                    .as_ref()
                                    .is_some_and(|selected| selected == &selection)
                            {
                                detail_state.set(Some(ReflogDetailState::Failed(selection)));
                            }
                            return;
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => {
                            timer(std::time::Duration::from_millis(16)).await;
                        }
                    }
                }
            });
        });
    }

    /// 从 Reflog 检查器文件树打开提交内的单文件差异。
    pub(crate) fn select_reflog_file(&self, selection: ReflogEntryKey, path: String) {
        self.select_reflog_file_with_whitespace(
            selection,
            path,
            self.history.diff.state.ignore_whitespace.get_untracked(),
            false,
        );
    }

    fn select_reflog_file_with_whitespace(
        &self,
        selection: ReflogEntryKey,
        path: String,
        ignore_whitespace: bool,
        force: bool,
    ) {
        if self.state.selected_tab.get_untracked() != RefsTab::Reflog
            || self.state.reflog_selection.get_untracked().as_ref() != Some(&selection)
        {
            return;
        }
        let current_diff = self.state.reflog_file_diff.get_untracked();
        if !force
            && current_diff.as_ref().is_some_and(|current| {
                current.selection == selection
                    && current.path == path
                    && (matches!(current.status, ReflogFileDiffStatus::Loading)
                        || matches!(current.status, ReflogFileDiffStatus::Loaded { .. }))
            })
        {
            return;
        }
        let files = match self.state.reflog_detail.get_untracked() {
            Some(ReflogDetailState::Loaded {
                selection: loaded,
                files,
                ..
            }) if loaded == selection && files.iter().any(|file| file.path == path) => files,
            _ => return,
        };
        let tabs = self.session.tabs.get_untracked();
        let Some(tab) = tabs
            .iter()
            .find(|tab| tab.id == selection.tab_id && tab.path == selection.path)
            .filter(|_| reflog_selection_exists(tabs.as_slice(), &selection))
        else {
            return;
        };
        self.state.reflog_file_diff.set(Some(ReflogFileDiffState {
            selection: selection.clone(),
            path: path.clone(),
            files: files.clone(),
            status: ReflogFileDiffStatus::Loading,
        }));
        let repo_path = tab.path.clone();
        let git_executable = tab.git_executable.clone();
        let sha = selection.sha.clone();
        let query_path = path.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = git::Repo::open_with_executable(repo_path.as_str(), &git_executable)
                .and_then(|repo| {
                    let raw_patch = repo.patch_for_path_with_whitespace(
                        sha.as_str(),
                        query_path.as_str(),
                        false,
                    )?;
                    let patch = if ignore_whitespace {
                        repo.patch_for_path_with_whitespace(
                            sha.as_str(),
                            query_path.as_str(),
                            true,
                        )?
                    } else {
                        raw_patch.clone()
                    };
                    Ok((patch, raw_patch))
                })
                .map_err(|_| ());
            let _ = sender.send(result);
        });

        let diff_state = self.state.reflog_file_diff;
        let current_selection = self.state.reflog_selection;
        let detail_state = self.state.reflog_detail;
        let tabs = self.session.tabs;
        Owner::detached().run(|| {
            spawn(async move {
                loop {
                    match receiver.try_recv() {
                        Ok(result) => {
                            let still_exists = reflog_selection_exists(
                                tabs.get_untracked().as_slice(),
                                &selection,
                            );
                            let still_selected = current_selection
                                .get_untracked()
                                .as_ref()
                                .is_some_and(|selected| selected == &selection);
                            let still_loading =
                                diff_state.get_untracked().as_ref().is_some_and(|current| {
                                    current.selection == selection
                                        && current.path == path
                                        && current.status == ReflogFileDiffStatus::Loading
                                });
                            let details_current = matches!(
                                detail_state.get_untracked(),
                                Some(ReflogDetailState::Loaded { selection: loaded, .. })
                                    if loaded == selection
                            );
                            if still_exists && still_selected && still_loading && details_current {
                                diff_state.set(Some(ReflogFileDiffState {
                                    selection,
                                    path,
                                    files,
                                    status: match result {
                                        Ok((patch, raw_patch)) => {
                                            ReflogFileDiffStatus::Loaded { patch, raw_patch }
                                        }
                                        Err(()) => ReflogFileDiffStatus::Failed,
                                    },
                                }));
                            }
                            return;
                        }
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            let still_exists = reflog_selection_exists(
                                tabs.get_untracked().as_slice(),
                                &selection,
                            );
                            let still_selected = current_selection
                                .get_untracked()
                                .as_ref()
                                .is_some_and(|selected| selected == &selection);
                            let still_loading =
                                diff_state.get_untracked().as_ref().is_some_and(|current| {
                                    current.selection == selection
                                        && current.path == path
                                        && current.status == ReflogFileDiffStatus::Loading
                                });
                            let details_current = matches!(
                                detail_state.get_untracked(),
                                Some(ReflogDetailState::Loaded { selection: loaded, .. })
                                    if loaded == selection
                            );
                            if still_exists && still_selected && still_loading && details_current {
                                diff_state.set(Some(ReflogFileDiffState {
                                    selection,
                                    path,
                                    files,
                                    status: ReflogFileDiffStatus::Failed,
                                }));
                            }
                            return;
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => {
                            let _ = next_frame().await;
                        }
                    }
                }
            });
        });
    }

    pub(crate) fn select_reflog_file_relative(&self, offset: isize) {
        let Some(current) = self.state.reflog_file_diff.get_untracked() else {
            return;
        };
        let Some(index) = current
            .files
            .iter()
            .position(|file| file.path == current.path)
        else {
            return;
        };
        let next = index as isize + offset;
        if next < 0 || next >= current.files.len() as isize {
            return;
        }
        let path = current.files[next as usize].path.clone();
        self.select_reflog_file(current.selection, path);
    }

    pub(crate) fn reload_reflog_file_diff(&self, ignore_whitespace: bool) {
        let Some(current) = self.state.reflog_file_diff.get_untracked() else {
            return;
        };
        self.select_reflog_file_with_whitespace(
            current.selection,
            current.path,
            ignore_whitespace,
            true,
        );
    }

    pub(crate) fn return_to_reflog(&self) {
        self.state.reflog_file_diff.set(None);
    }

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

    pub(crate) fn new_worktree_form_at(&self, tab_id: u64) {
        self.search
            .open_form_from_tab(tab_id, FormKind::NewWorktree, String::new());
    }

    pub(crate) fn open_worktree_window_at(&self, tab_id: u64, path: String) {
        let registered = self
            .session
            .tabs
            .get_untracked()
            .iter()
            .find(|tab| tab.id == tab_id)
            .is_some_and(|tab| {
                tab.refs
                    .worktrees
                    .iter()
                    .any(|worktree| worktree.path == path && worktree.exists)
            });
        if !registered {
            self.notify_worktree("worktree.targetChanged");
            return;
        }
        let Some(open) = self.open_worktree_window.as_ref() else {
            self.notify_worktree("worktree.windowUnavailable");
            return;
        };
        open(tab_id, path);
    }

    pub(crate) fn remove_worktree_at(&self, tab_id: u64, path: String) {
        if self.worktree_is_open(path.as_str()) {
            self.notify_worktree("worktree.cannotRemoveOpen");
            return;
        }
        let safe = self
            .worktree_info(tab_id, path.as_str())
            .is_some_and(|worktree| {
                worktree.exists
                    && !worktree.main
                    && !worktree.prunable
                    && !worktree.locked
                    && !worktree.dirty
            });
        if !safe {
            self.notify_worktree("worktree.cannotRemove");
            return;
        }
        let language = self.language;
        let lang = language.get_untracked();
        let pending = crate::ui::dialog::confirm_destructive(
            crate::i18n::t(lang.as_str(), "worktree.removeTitle"),
            crate::i18n::with_arguments(
                lang.as_str(),
                "worktree.removeMessage",
                &[("path", path.clone())],
            ),
            crate::i18n::t(lang.as_str(), "worktree.removeAction"),
            crate::i18n::t(lang.as_str(), "common.cancel"),
        );
        let commands = self.clone();
        spawn(async move {
            if !matches!(pending.await, Ok(result) if result.action.as_deref() == Some("delete")) {
                return;
            }
            if commands.worktree_is_open(path.as_str()) {
                commands.notify_worktree("worktree.cannotRemoveOpen");
                return;
            }
            let tabs = commands.session.tabs;
            let mut list = tabs.get_untracked();
            let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) else {
                commands.notify_worktree("worktree.targetChanged");
                return;
            };
            let still_safe = commands
                .fresh_worktree(tab.path.as_str(), path.as_str())
                .is_some_and(|worktree| {
                    worktree.exists
                        && !worktree.main
                        && !worktree.prunable
                        && !worktree.locked
                        && !worktree.dirty
                });
            if !still_safe {
                commands.notify_worktree("worktree.cannotRemove");
                return;
            }
            match ops::remove_worktree(tab, path.as_str()) {
                Ok(()) => crate::interactions::notify(
                    commands.notice,
                    crate::i18n::t(
                        commands.language.get_untracked().as_str(),
                        "worktree.removed",
                    ),
                ),
                Err(detail) => crate::interactions::notify(commands.notice, detail),
            }
            tabs.set(list);
        });
    }

    pub(crate) fn prune_worktrees_at(&self, tab_id: u64) {
        let Some(parent) = self
            .session
            .tabs
            .get_untracked()
            .iter()
            .find(|tab| tab.id == tab_id)
            .map(|tab| tab.path.clone())
        else {
            return;
        };
        let mut candidates = self
            .fresh_worktrees(parent.as_str())
            .into_iter()
            .filter(|worktree| worktree.prunable && !worktree.locked)
            .map(|worktree| worktree.path)
            .collect::<Vec<_>>();
        candidates.sort();
        if candidates.is_empty() {
            self.notify_worktree("worktree.pruneEmpty");
            return;
        }
        if candidates
            .iter()
            .any(|path| self.worktree_is_open(path.as_str()))
        {
            self.notify_worktree("worktree.cannotPruneOpen");
            return;
        }
        let language = self.language;
        let lang = language.get_untracked();
        let pending = crate::ui::dialog::confirm_destructive(
            crate::i18n::t(lang.as_str(), "worktree.pruneTitle"),
            crate::i18n::with_arguments(
                lang.as_str(),
                "worktree.pruneMessage",
                &[("paths", candidates.join("\n"))],
            ),
            crate::i18n::t(lang.as_str(), "worktree.pruneAction"),
            crate::i18n::t(lang.as_str(), "common.cancel"),
        );
        let commands = self.clone();
        spawn(async move {
            if !matches!(pending.await, Ok(result) if result.action.as_deref() == Some("delete")) {
                return;
            }
            let tabs = commands.session.tabs;
            let mut list = tabs.get_untracked();
            let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) else {
                commands.notify_worktree("worktree.targetChanged");
                return;
            };
            let mut latest = commands
                .fresh_worktrees(tab.path.as_str())
                .into_iter()
                .filter(|worktree| worktree.prunable && !worktree.locked)
                .map(|worktree| worktree.path)
                .collect::<Vec<_>>();
            latest.sort();
            if latest != candidates {
                commands.notify_worktree("worktree.pruneChanged");
                return;
            }
            if latest
                .iter()
                .any(|path| commands.worktree_is_open(path.as_str()))
            {
                commands.notify_worktree("worktree.cannotPruneOpen");
                return;
            }
            match ops::prune_worktrees(tab) {
                Ok(()) => crate::interactions::notify(
                    commands.notice,
                    crate::i18n::t(
                        commands.language.get_untracked().as_str(),
                        "worktree.pruned",
                    ),
                ),
                Err(detail) => crate::interactions::notify(commands.notice, detail),
            }
            tabs.set(list);
        });
    }

    pub(crate) fn worktree_is_open(&self, path: &str) -> bool {
        return self
            .main_tabs
            .get_untracked()
            .iter()
            .any(|tab| Self::same_worktree_path(tab.path.as_str(), path))
            || self
                .worktree_windows
                .get_untracked()
                .iter()
                .any(|(open_path, _)| Self::same_worktree_path(open_path.as_str(), path));
    }

    fn same_worktree_path(left: &str, right: &str) -> bool {
        return Self::normalize_worktree_path(left) == Self::normalize_worktree_path(right);
    }

    fn normalize_worktree_path(path: &str) -> std::path::PathBuf {
        let mut ancestor = std::path::Path::new(path);
        let mut missing = Vec::new();
        while !ancestor.exists() {
            let Some(name) = ancestor.file_name() else {
                break;
            };
            missing.push(name.to_os_string());
            let Some(parent) = ancestor.parent() else {
                break;
            };
            ancestor = parent;
        }
        let mut normalized = ancestor
            .canonicalize()
            .unwrap_or_else(|_| ancestor.to_path_buf());
        for part in missing.iter().rev() {
            normalized.push(part);
        }
        return normalized;
    }

    fn worktree_info(&self, tab_id: u64, path: &str) -> Option<git::WorktreeInfo> {
        let parent = self
            .session
            .tabs
            .get_untracked()
            .iter()
            .find(|tab| tab.id == tab_id)?
            .path
            .clone();
        return self.fresh_worktree(parent.as_str(), path);
    }

    fn fresh_worktree(&self, parent: &str, path: &str) -> Option<git::WorktreeInfo> {
        self.session
            .open_repo(parent)
            .ok()?
            .refs()
            .ok()?
            .worktrees
            .into_iter()
            .find(|worktree| worktree.path == path)
    }

    fn fresh_worktrees(&self, parent: &str) -> Vec<git::WorktreeInfo> {
        self.session
            .open_repo(parent)
            .ok()
            .and_then(|repo| repo.refs().ok())
            .map(|refs| refs.worktrees)
            .unwrap_or_default()
    }

    fn notify_worktree(&self, key: &'static str) {
        crate::interactions::notify(
            self.notice,
            crate::i18n::t(self.language.get_untracked().as_str(), key),
        );
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
        if let Some(tab_id) = self.active_tab_id() {
            self.run_remote_operation(tab_id, RemoteOperation::PushTag { name, remote: None });
        }
    }

    pub(crate) fn push_tag_to_remote_at(&self, tab_id: u64, name: String, remote: String) {
        let notice = self.notice;
        let language = self.language;
        let Some(tab) = self
            .session
            .tabs
            .get_untracked()
            .into_iter()
            .find(|tab| tab.id == tab_id)
        else {
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
        let _ = (notice, language);
        self.run_remote_operation(
            tab_id,
            RemoteOperation::PushTag {
                name,
                remote: Some(remote),
            },
        );
    }

    pub(crate) fn update_submodule_at(&self, tab_id: u64, path: String, latest: bool) {
        self.run_remote_operation(tab_id, RemoteOperation::UpdateSubmodule { path, latest });
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
        let commands = self.clone();
        spawn(async move {
            if !pending.await {
                return;
            }
            commands.run_remote_operation(tab_id, RemoteOperation::UpdateSubmodules { paths });
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
        let Ok(repo) =
            git::Repo::open_with_executable(path.to_string_lossy().as_ref(), &tab.git_executable)
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
                    let message =
                        if tab.operation == git::Operation::None && !tab.conflicts.is_empty() {
                            crate::i18n::t(lang.as_str(), "conflicts.stashApply")
                        } else {
                            crate::i18n::t(lang.as_str(), "toast.stashDone")
                        };
                    crate::interactions::notify(notice, message);
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
        let has_conflict = tab.operation == git::Operation::None && !tab.conflicts.is_empty();
        tabs.set(list);
        match result {
            Ok(true) => {
                let lang = language.get_untracked();
                let message = if has_conflict {
                    crate::i18n::t(lang.as_str(), "conflicts.stashApply")
                } else {
                    crate::i18n::t(lang.as_str(), "toast.stashDone")
                };
                crate::interactions::notify(notice, message)
            }
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
        match ops::show_stash_diff(
            tab,
            oid.as_str(),
            self.history.diff.state.ignore_whitespace.get_untracked(),
        ) {
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
        self.run_remote_operation(
            tab_id,
            RemoteOperation::PushBranch {
                branch,
                remote: "origin".to_string(),
            },
        );
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
        let commands = self.clone();
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
            drop(list);
            commands.run_remote_operation(tab_id, RemoteOperation::PullBranch { remote, branch });
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
        let commands = self.clone();
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
            drop(list);
            commands.run_remote_operation(tab_id, RemoteOperation::DeleteBranch { remote, branch });
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
