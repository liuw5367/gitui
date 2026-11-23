//! 提交历史的用例：选中提交/文件、对比、筛选与刷新。
//!
//! 方法体由原 `Actions` 里的闭包原样搬来，只把捕获的局部变量换成 `self.*`；
//! 这里只调本功能的 `ops` 并更新状态，不写布局。

use super::ops;
use super::state::{CommitLoadRequest, HistoryState, MultiCommitData, MultiCommitSelection};
use crate::features::SessionHandle;
use crate::features::commit::commands::CommitCommands;
use crate::features::recovery::commands::RecoveryCommands;
use crate::filters::{self, FilterError};
use crate::shared::diff::DiffCommands;
use git::{LogFilter, Repo};
use nsh_core::prelude::*;

#[derive(Clone)]
pub(crate) struct HistoryCommands {
    pub(crate) state: HistoryState,
    pub(crate) session: SessionHandle,
    pub(crate) diff: DiffCommands,
    /// 提交功能的用例：提交表右键菜单里的「改写说明」「合并提交」是提交功能的
    /// 入口，创建初始提交也复用它的流程。
    pub(crate) commit: CommitCommands,
    /// 恢复功能的用例（批 B 搬入）：右键菜单的拣选/回滚/重置都经它走。
    pub(crate) recovery: RecoveryCommands,
    /// 历史视图的入口（reflog 的「在历史中显示」要切回 History）。
    pub(crate) mode: Signal<crate::Mode>,
    /// 状态栏短提示（找不到提交时提示，而不是悄悄失败）。
    pub(crate) notice: Signal<Option<String>>,
    pub(crate) language: Signal<SharedString>,
}

impl HistoryCommands {
    /// 先显示行选中态并切到 History，再异步更新提交数据。
    pub(crate) fn request_commit_selection(&self, tab_id: u64, sha: String) {
        let active = self.session.active.get_untracked();
        let valid = self.session.tabs.with_untracked(|tabs| {
            tabs.get(active).is_some_and(|tab| {
                tab.id == tab_id && tab.commits.iter().any(|commit| commit.sha == sha)
            })
        });
        if !valid {
            return;
        }
        self.state.fast_selection.set(Some((tab_id, sha.clone())));
        let command = self.clone();
        Owner::detached().run(|| {
            spawn(async move {
                // 先呈现选中行，再克隆标签数据及更新检查器；旧点击由 fast_selection 淘汰。
                next_frame().await;
                batch(|| command.select_commit(tab_id, sha));
            });
        });
    }

    /// ⌘ 点击在 History 中增减 commit 多选，并在后台读取合并文件统计与逐项详情。
    pub(crate) fn toggle_multi_commit_selection(&self, tab_id: u64, sha: String) {
        let active = self.session.active.get_untracked();
        let tabs = self.session.tabs.get_untracked();
        let Some(tab) = tabs.get(active).filter(|tab| tab.id == tab_id) else {
            return;
        };
        if !tab.commits.iter().any(|commit| commit.sha == sha) {
            return;
        }

        self.state.fast_selection.set(None);
        let current_selection = self.state.multi_commit_selection.get_untracked();
        let has_multi_selection = current_selection
            .as_ref()
            .is_some_and(|selection| selection.tab_id == tab_id);
        let mut shas = if has_multi_selection {
            current_selection.unwrap().shas
        } else {
            let selected_sha = tab.selected_commit().map(|commit| commit.sha.clone());
            if selected_sha.as_deref() == Some(sha.as_str()) {
                vec![sha.clone()]
            } else {
                selected_sha
                    .into_iter()
                    .chain(std::iter::once(sha.clone()))
                    .collect()
            }
        };
        if has_multi_selection {
            if let Some(selected) = shas.iter().position(|selected| selected == &sha) {
                shas.remove(selected);
            } else {
                shas.push(sha);
            }
        }
        if shas.is_empty() {
            self.state.multi_commit_selection.set(None);
            self.state.multi_commit_data.set(None);
            return;
        }
        shas.sort_by_key(|selected| {
            tab.commits
                .iter()
                .position(|commit| &commit.sha == selected)
                .unwrap_or(usize::MAX)
        });

        let selection = MultiCommitSelection { tab_id, shas };
        self.mode.set(crate::Mode::History);
        self.state
            .multi_commit_selection
            .set(Some(selection.clone()));
        self.state.multi_commit_data.set(Some(MultiCommitData {
            tab_id,
            shas: selection.shas.clone(),
            files: Vec::new(),
            details: Vec::new(),
            loading: true,
            failed: false,
        }));
        self.load_multi_commit_files(selection, tab.path.clone(), tab.git_executable.clone());
    }

    pub(crate) fn retry_multi_commit_files(&self) {
        let Some(selection) = self.state.multi_commit_selection.get_untracked() else {
            return;
        };
        let active = self.session.active.get_untracked();
        let tabs = self.session.tabs.get_untracked();
        let Some(tab) = tabs.get(active).filter(|tab| tab.id == selection.tab_id) else {
            return;
        };
        self.state.multi_commit_data.set(Some(MultiCommitData {
            tab_id: selection.tab_id,
            shas: selection.shas.clone(),
            files: Vec::new(),
            details: Vec::new(),
            loading: true,
            failed: false,
        }));
        self.load_multi_commit_files(selection, tab.path.clone(), tab.git_executable.clone());
    }

    fn load_multi_commit_files(
        &self,
        selection: MultiCommitSelection,
        path: String,
        git_executable: git::GitExecutable,
    ) {
        let tab_id = selection.tab_id;
        let shas = selection.shas.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result =
                ops::load_multi_commit_data(path.as_str(), &git_executable, shas.as_slice());
            let _ = sender.send(result);
        });
        let state = self.state.clone();
        Owner::detached().run(|| {
            spawn(async move {
                loop {
                    match receiver.try_recv() {
                        Ok(result) => {
                            if state.multi_commit_selection.get_untracked().as_ref()
                                == Some(&selection)
                            {
                                let (files, details, failed) = match result {
                                    Ok((files, details)) => (files, details, false),
                                    Err(_) => (
                                        Vec::new(),
                                        std::iter::repeat_with(|| None)
                                            .take(selection.shas.len())
                                            .collect(),
                                        true,
                                    ),
                                };
                                state.multi_commit_data.set(Some(MultiCommitData {
                                    tab_id,
                                    shas: selection.shas,
                                    files,
                                    details,
                                    loading: false,
                                    failed,
                                }));
                            }
                            return;
                        }
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => return,
                        Err(std::sync::mpsc::TryRecvError::Empty) => {
                            timer(std::time::Duration::from_millis(16)).await;
                        }
                    }
                }
            });
        });
    }

    /// 更新指定仓库标签页中的提交选择，Git 数据随后在后台读取。
    fn select_commit(&self, tab_id: u64, requested_sha: String) {
        if !self
            .state
            .fast_selection
            .get_untracked()
            .as_ref()
            .is_some_and(|(selected_tab, selected_sha)| {
                *selected_tab == tab_id && selected_sha == &requested_sha
            })
        {
            return;
        }
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        let is_active_tab = list.get(current).is_some_and(|tab| tab.id == tab_id);
        let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) else {
            self.clear_fast_selection(tab_id, requested_sha.as_str());
            return;
        };
        if !is_active_tab {
            self.clear_fast_selection(tab_id, requested_sha.as_str());
            return;
        }
        self.state.compare_mode.set(false);
        tab.comparing = false;
        tab.compare.clear();
        tab.compare_base = None;
        tab.compare_target = None;
        tab.compare_worktree = false;
        self.state.multi_commit_selection.set(None);
        self.state.multi_commit_data.set(None);
        self.mode.set(crate::Mode::History);
        let loading = self.state.commit_info_loading;
        let already_loading = loading
            .get_untracked()
            .contains(&(tab_id, requested_sha.clone()));
        if tab
            .selected_commit()
            .is_some_and(|commit| commit.sha == requested_sha)
            && (tab.details.is_some() && tab.error.is_none() || already_loading)
        {
            tab.show_commit_diff = false;
            tab.selected_file = None;
            tab.file_patch = None;
            self.session.tabs.set(list);
            self.clear_fast_selection(tab_id, requested_sha.as_str());
            return;
        }
        let Some(index) = tab
            .commits
            .iter()
            .position(|commit| commit.sha == requested_sha)
        else {
            self.clear_fast_selection(tab_id, requested_sha.as_str());
            return;
        };
        let Some((sha, short)) = ops::prepare_commit_selection(tab, index) else {
            self.clear_fast_selection(tab_id, requested_sha.as_str());
            return;
        };
        let path = tab.path.clone();
        let git_executable = tab.git_executable.clone();
        let tabs = self.session.tabs;
        loading.update(|items| {
            items.retain(|(loading_tab, _)| *loading_tab != tab_id);
            items.push((tab_id, sha.clone()));
        });
        tabs.set(list);
        self.clear_fast_selection(tab_id, sha.as_str());

        let request = CommitLoadRequest {
            tab_id,
            path,
            git_executable,
            sha,
            short,
        };
        let start = self
            .state
            .commit_load_queue
            .lock()
            .unwrap()
            .enqueue(request);
        if let Some(request) = start {
            self.start_commit_load(request);
        }
    }

    fn start_commit_load(&self, request: CommitLoadRequest) {
        let CommitLoadRequest {
            tab_id,
            path,
            git_executable,
            sha,
            short,
        } = request;
        let tabs = self.session.tabs;
        let loading = self.state.commit_info_loading;
        let command = self.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        let load_sha = sha.clone();
        std::thread::spawn(move || {
            let result = ops::load_commit_selection(
                path.as_str(),
                &git_executable,
                load_sha.as_str(),
                short.as_str(),
            );
            let _ = sender.send(result);
        });
        Owner::detached().run(|| {
            spawn(async move {
                loop {
                    match receiver.try_recv() {
                        Ok(result) => {
                            let mut current_tabs = tabs.get_untracked();
                            let mut changed = false;
                            if let Some(tab) = current_tabs.iter_mut().find(|tab| {
                                tab.id == tab_id
                                    && command
                                        .state
                                        .fast_selection
                                        .get_untracked()
                                        .as_ref()
                                        .is_none_or(|(selected_tab, selected_sha)| {
                                            *selected_tab != tab_id || selected_sha == &sha
                                        })
                                    && tab
                                        .selected_commit()
                                        .is_some_and(|commit| commit.sha == sha)
                            }) {
                                match result {
                                    Ok(data) => ops::apply_commit_selection(tab, data),
                                    Err(error) => tab.error = Some(error),
                                }
                                changed = true;
                            }
                            if changed {
                                tabs.set(current_tabs);
                            }
                            loading.update(|items| {
                                items.retain(|(loading_tab, loading_sha)| {
                                    *loading_tab != tab_id || loading_sha != &sha
                                });
                            });
                            command.start_next_commit_load();
                            return;
                        }
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                            loading.update(|items| {
                                items.retain(|(loading_tab, loading_sha)| {
                                    *loading_tab != tab_id || loading_sha != &sha
                                });
                            });
                            command.start_next_commit_load();
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

    fn start_next_commit_load(&self) {
        let next = self.state.commit_load_queue.lock().unwrap().finish();
        if let Some(request) = next {
            let already_loaded = self
                .session
                .tabs
                .get_untracked()
                .iter()
                .find(|tab| tab.id == request.tab_id)
                .is_some_and(|tab| {
                    tab.details.is_some()
                        && tab.error.is_none()
                        && tab
                            .selected_commit()
                            .is_some_and(|commit| commit.sha == request.sha)
                });
            if already_loaded {
                self.start_next_commit_load();
            } else {
                self.start_commit_load(request);
            }
        }
    }

    fn clear_fast_selection(&self, tab_id: u64, sha: &str) {
        if self
            .state
            .fast_selection
            .get_untracked()
            .as_ref()
            .is_some_and(|(selected_tab, selected_sha)| {
                *selected_tab == tab_id && selected_sha == sha
            })
        {
            self.state.fast_selection.set(None);
        }
    }

    /// 从提交菜单打开当前提交补丁，普通选择仍保持图表视图。
    pub(crate) fn show_commit_diff(&self, index: usize) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            ops::show_commit_diff(tab, index);
            self.session.tabs.set(list);
        }
    }

    /// 从提交差异返回完整提交列表，并保留当前选中的提交。
    pub(crate) fn return_to_history(&self) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            ops::return_to_history(tab);
            self.session.tabs.set(list);
        }
    }

    /// 在提交变更树中选中文件并打开该文件的提交差异。
    pub(crate) fn select_tree_path(&self, id: String) {
        let Some(path) = id.strip_prefix("file:") else {
            return;
        };
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            if let Some(index) = tab.files.iter().position(|file| file.path == path) {
                ops::select_file(tab, index);
            }
            self.session.tabs.set(list);
        }
    }

    /// Enter the Blame / file history page for the selected commit file.
    pub(crate) fn show_file_history(&self, tab_id: u64, path: String) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        let Some(tab) = list.get_mut(current).filter(|tab| tab.id == tab_id) else {
            return;
        };
        let Some(commit) = tab.selected_commit() else {
            return;
        };
        let sha = commit.sha.clone();
        ops::open_blame(tab, &path, &sha, crate::Mode::History);
        self.session.tabs.set(list);
        self.mode.set(crate::Mode::Blame);
    }

    /// Enter file history for a file at an explicit commit revision.
    pub(crate) fn show_file_history_at(&self, tab_id: u64, path: String, sha: String) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        let Some(tab) = list.get_mut(current).filter(|tab| tab.id == tab_id) else {
            return;
        };
        ops::open_blame(tab, &path, &sha, crate::Mode::History);
        self.session.tabs.set(list);
        self.mode.set(crate::Mode::Blame);
    }

    pub(crate) fn open_blame(&self) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        let Some(tab) = list.get_mut(current) else {
            return;
        };
        if tab.blame_path.is_empty() && !tab.files.is_empty() {
            let file_index = tab.selected_file.unwrap_or(0);
            ops::select_file(tab, file_index);
        }
        if tab.blame_path.is_empty() {
            return;
        }
        tab.blame_return_mode = Some(crate::Mode::History);
        self.mode.set(crate::Mode::Blame);
        self.session.tabs.set(list);
    }

    pub(crate) fn select_blame_history(&self, sha: String) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            ops::select_blame_history(tab, sha.as_str());
            self.session.tabs.set(list);
        }
    }

    pub(crate) fn focus_blame_commit(&self, sha: String, line: u32) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            ops::focus_blame_commit(tab, sha, line);
            self.session.tabs.set(list);
        }
    }

    pub(crate) fn toggle_blame_diff(&self, show_diff: bool) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            tab.blame_show_diff = show_diff;
            self.session.tabs.set(list);
        }
    }

    /// Leave the blame page and return to its originating workbench context.
    pub(crate) fn return_from_blame(&self) {
        let current = self.session.active.get_untracked();
        let target = self
            .session
            .tabs
            .get_untracked()
            .get(current)
            .map(|tab| {
                tab.blame_return_mode.unwrap_or_else(|| {
                    if tab.blame_revision == "HEAD" {
                        crate::Mode::Changes
                    } else {
                        crate::Mode::History
                    }
                })
            })
            .unwrap_or(crate::Mode::History);
        self.mode.set(target);
    }

    /// 左侧分支、远端分支或 tag 按完整 ref 定位，stash 直接传 OID。
    pub(crate) fn show_reference_at(&self, tab_id: u64, reference: String) {
        let Some((path, git_executable)) = self
            .session
            .tabs
            .get_untracked()
            .iter()
            .find(|tab| tab.id == tab_id)
            .map(|tab| (tab.path.clone(), tab.git_executable.clone()))
        else {
            return;
        };
        let sha = match Repo::open_with_executable(path.as_str(), &git_executable)
            .and_then(|repo| repo.resolve_commit(&reference))
        {
            Ok(sha) => sha,
            Err(_) => {
                self.notify_reference_not_found();
                return;
            }
        };
        self.show_reference_sha_at(tab_id, sha);
    }

    /// 贮藏引用已保存完整提交 OID，无需按会变化的 stash 序号解析。
    pub(crate) fn show_commit_at(&self, tab_id: u64, sha: String) {
        self.show_reference_sha_at(tab_id, sha);
    }

    fn show_reference_sha_at(&self, tab_id: u64, sha: String) {
        let mut list = self.session.tabs.get_untracked();
        let Some(index) = list.iter().position(|tab| tab.id == tab_id) else {
            return;
        };
        let tab = &mut list[index];
        let hidden_by_filter = !tab.commits.iter().any(|commit| commit.sha == sha);
        if hidden_by_filter {
            if !ops::select_reference_sha(tab, sha.as_str(), &LogFilter::default()) {
                self.notify_reference_not_found();
                return;
            }
        } else if let Some(commit_index) = tab.commits.iter().position(|commit| commit.sha == sha) {
            ops::select_commit(tab, commit_index);
        }
        let selected = tab.selected.unwrap_or(0);
        let visible = ops::visible_commit_indices(
            &tab.commits,
            self.state.query.get_untracked().as_str(),
            tab.selected,
        );
        let Some(row) = visible.iter().position(|commit| *commit == selected) else {
            self.notify_reference_not_found();
            return;
        };
        self.state.scroll_request.set(Some(row));
        self.session.tabs.set(list);
        self.session.active.set(index);
        self.mode.set(crate::Mode::History);
    }

    fn notify_reference_not_found(&self) {
        let lang = self.language.get_untracked();
        crate::interactions::notify(
            self.notice,
            crate::i18n::t(lang.as_str(), "history.referenceNotFound"),
        );
    }

    /// 把第 `index` 个提交加入或移出对比集合（原 `toggle_compare`）。
    pub(crate) fn toggle_compare(&self, index: usize) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            ops::toggle_compare(tab, index);
            self.session.tabs.set(list);
        }
    }

    /// 进入或退出对比模式（原 `set_compare_mode`）。
    pub(crate) fn set_compare_mode(&self, enabled: bool) {
        self.state.compare_mode.set(enabled);
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            if enabled && (tab.compare_base.is_some() || tab.compare_target.is_some()) {
                ops::set_compare_refs(tab, None, None);
            }
            tab.comparing = enabled;
            if !enabled {
                tab.compare.clear();
            }
            self.session.tabs.set(list);
        }
    }

    /// 设置对比区间的两端（原 `set_compare_refs`）。
    pub(crate) fn set_compare_refs(&self, base: Option<String>, target: Option<String>) {
        let Some(tab_id) = self
            .session
            .tabs
            .get_untracked()
            .get(self.session.active.get_untracked())
            .map(|tab| tab.id)
        else {
            return;
        };
        self.set_compare_refs_at(tab_id, base, target);
    }

    /// 在 Compare refs 窗口确认一组端点与选项，只刷新一次当前标签页。
    pub(crate) fn apply_compare_refs(
        &self,
        base: Option<String>,
        target: Option<String>,
        three_dot: bool,
        worktree: bool,
    ) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            let refs_changed = tab.compare_base != base || tab.compare_target != target;
            let options_changed = tab.compare_three_dot != three_dot
                || tab.compare_worktree != (worktree && tab.compare_target.is_some());
            if !refs_changed && !options_changed {
                return;
            }
            if !refs_changed {
                tab.compare_three_dot = three_dot;
                tab.compare_worktree = worktree && tab.compare_target.is_some();
                self.session.tabs.set(list);
                return;
            }
            self.state.compare_mode.set(false);
            tab.comparing = false;
            tab.compare_three_dot = three_dot;
            ops::set_compare_refs(tab, base, target);
            tab.compare_worktree = worktree && tab.compare_target.is_some();
            self.session.tabs.set(list);
            self.mode.set(crate::Mode::History);
        }
    }

    /// 按仓库标签的稳定 ID 设置对比区间。
    pub(crate) fn set_compare_refs_at(
        &self,
        tab_id: u64,
        base: Option<String>,
        target: Option<String>,
    ) {
        self.state.compare_mode.set(false);
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) {
            tab.comparing = false;
            ops::set_compare_refs(tab, base, target);
            self.session.tabs.set(list);
            self.mode.set(crate::Mode::History);
        }
    }

    /// Select two-dot or three-dot comparison semantics.
    pub(crate) fn begin_filter(&self) {
        let current = self.session.active.get_untracked();
        let Some(tab_id) = self
            .session
            .tabs
            .get_untracked()
            .get(current)
            .map(|tab| tab.id)
        else {
            return;
        };
        self.begin_filter_for_tab(tab_id);
    }

    pub(crate) fn begin_filter_for_tab(&self, tab_id: u64) {
        if self.state.filter_tab_id.get_untracked() == Some(tab_id) {
            return;
        }
        let current = self.session.active.get_untracked();
        let Some(filter) = self
            .session
            .tabs
            .get_untracked()
            .get(current)
            .filter(|tab| tab.id == tab_id)
            .map(|tab| tab.filter.clone())
        else {
            return;
        };
        self.state.filter_tab_id.set(Some(tab_id));
        self.state.draft.set(filters::draft_from_filter(&filter));
    }

    pub(crate) fn begin_quick_filter(&self, tab_id: u64) {
        let continuing_quick_filter = self.state.filter_menu.get_untracked().is_some()
            && self.state.filter_tab_id.get_untracked() == Some(tab_id);
        self.begin_filter_for_tab(tab_id);
        if !continuing_quick_filter {
            let draft = self.state.draft.get_untracked();
            self.state
                .quick_filter_baseline
                .set(Some((draft.references, draft.authors)));
        }
    }

    pub(crate) fn toggle_branch_filter(&self, reference: String) {
        self.state.draft.update(|draft| {
            if draft.references.contains(&reference) {
                draft.references.retain(|item| item != &reference);
            } else {
                draft.references.push(reference);
            }
        });
    }
    pub(crate) fn clear_branch_filter(&self) {
        self.state.draft.update(|draft| draft.references.clear());
    }
    pub(crate) fn toggle_author_filter(&self, email: String) {
        self.state.draft.update(|draft| {
            if draft.authors.contains(&email) {
                draft.authors.retain(|item| item != &email);
            } else {
                draft.authors.push(email);
            }
        });
    }
    pub(crate) fn clear_author_filter(&self) {
        self.state.draft.update(|draft| draft.authors.clear());
    }
    pub(crate) fn apply_quick_filter(&self) {
        self.state.filter_menu.set(None);
        self.state.filter_open.set(false);
        let Some(tab_id) = self.active_filter_tab_id() else {
            return;
        };
        let current = self.session.active.get_untracked();
        let Some(mut filter) = self
            .session
            .tabs
            .get_untracked()
            .get(current)
            .filter(|tab| tab.id == tab_id)
            .map(|tab| tab.filter.clone())
        else {
            return;
        };
        let draft = self.state.draft.get_untracked();
        let baseline = self.state.quick_filter_baseline.get_untracked();
        if quick_filter_selection_unchanged(baseline.as_ref(), &filter, &draft) {
            return;
        }
        filter.reference = None;
        filter.references = draft.references.clone();
        filter.authors = draft.authors.clone();
        self.state
            .quick_filter_baseline
            .set(Some((draft.references, draft.authors)));
        self.reload_with_filter(&filter);
    }

    /// 按当前条件重新读一遍当前仓库（原 `reload`）。
    fn reload_with_filter(&self, filter: &LogFilter) {
        self.start_filter_load(filter, false);
    }

    fn start_filter_load(&self, filter: &LogFilter, force: bool) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        let Some(tab) = list.get_mut(current) else {
            return;
        };
        if !force && tab.filter == *filter && tab.error.is_none() {
            return;
        }
        tab.load_id = crate::app::session::next_tab_id();
        tab.filter = filter.clone();
        tab.loading = true;
        let snapshot = tab.clone();
        let tab_id = tab.id;
        let load_id = tab.load_id;
        let filter = filter.clone();
        let tabs = self.session.tabs;
        tabs.set(list);
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(ops::reload_with_filter(&snapshot, &filter));
        });
        self.session.task_owner.run(|| {
            spawn(async move {
                loop {
                    match receiver.try_recv() {
                        Ok(mut loaded) => {
                            let mut list = tabs.get_untracked();
                            if let Some(tab) = list
                                .iter_mut()
                                .find(|tab| tab.id == tab_id && tab.load_id == load_id)
                            {
                                loaded.sidebar_collapsed = tab.sidebar_collapsed.clone();
                                loaded.summary = tab.summary.clone();
                                loaded.description = tab.description.clone();
                                *tab = loaded;
                                tabs.set(list);
                            }
                            return;
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => {
                            timer(std::time::Duration::from_millis(16)).await
                        }
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => return,
                    }
                }
            })
        });
    }

    fn reload(&self) {
        self.start_filter_load(&(self.state.filter)(), true);
    }

    /// 应用筛选条件并收起浮层（原 `apply_filter`）。
    pub(crate) fn apply_filter(&self) {
        let Some(_tab_id) = self.active_filter_tab_id() else {
            return;
        };
        let filter = match filters::build_draft(&self.state.draft.get_untracked()) {
            Ok(filter) => filter,
            Err(error) => {
                self.notify_filter_error(error);
                return;
            }
        };
        self.state.filter_open.set(false);
        self.state.filter_menu.set(None);
        self.reload_with_filter(&filter);
    }

    /// 清掉全部筛选条件并重读（原 `clear_filter`）。
    pub(crate) fn clear_filter(&self) {
        let Some(_tab_id) = self.active_filter_tab_id() else {
            return;
        };
        self.state.draft.set(filters::FilterDraft::default());
        self.state.filter_open.set(false);
        self.state.filter_menu.set(None);
        self.reload_with_filter(&LogFilter::default());
    }

    fn active_filter_tab_id(&self) -> Option<u64> {
        let tab_id = self.state.filter_tab_id.get_untracked()?;
        let current = self.session.active.get_untracked();
        return self
            .session
            .tabs
            .get_untracked()
            .get(current)
            .filter(|tab| tab.id == tab_id)
            .map(|_| tab_id);
    }

    fn notify_filter_error(&self, error: FilterError) {
        let key = match error {
            FilterError::InvalidSince => "filter.invalidStartDate",
            FilterError::InvalidUntil => "filter.invalidEndDate",
            FilterError::ReversedRange => "filter.invalidDateRange",
        };
        let lang = self.language.get_untracked();
        crate::interactions::notify(self.notice, crate::i18n::t(lang.as_str(), key));
    }

    /// 刷新当前仓库（原 `refresh`）。
    pub(crate) fn refresh(&self) {
        self.reload();
    }

    /// reflog 的「在历史中显示」：切到 History 并选中该提交（原 `show_in_history`）。
    pub(crate) fn show_in_history(&self, sha: String) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            if ops::select_by_sha(tab, sha.as_str()) {
                self.mode.set(crate::Mode::History);
                self.session.tabs.set(list);
            } else {
                let lang = self.language.get_untracked();
                crate::interactions::notify(
                    self.notice,
                    crate::i18n::t(lang.as_str(), "reflog.notFound"),
                );
            }
        }
    }
}

fn quick_filter_unchanged(filter: &LogFilter, draft: &filters::FilterDraft) -> bool {
    let applied_references = if filter.references.is_empty() {
        filter.reference.iter().cloned().collect::<Vec<_>>()
    } else {
        filter.references.clone()
    };
    return same_filter_values(&applied_references, &draft.references)
        && same_filter_values(&filter.authors, &draft.authors);
}

fn quick_filter_selection_unchanged(
    baseline: Option<&(Vec<String>, Vec<String>)>,
    filter: &LogFilter,
    draft: &filters::FilterDraft,
) -> bool {
    if let Some((references, authors)) = baseline {
        return same_filter_values(references, &draft.references)
            && same_filter_values(authors, &draft.authors);
    }
    return quick_filter_unchanged(filter, draft);
}

fn same_filter_values(left: &[String], right: &[String]) -> bool {
    let mut left = left.to_vec();
    let mut right = right.to_vec();
    left.sort();
    left.dedup();
    right.sort();
    right.dedup();
    return left == right;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_legacy_single_branch_filter_is_unchanged_after_normalization() {
        let filter = LogFilter {
            reference: Some(String::from("refs/heads/main")),
            ..LogFilter::default()
        };
        let draft = filters::FilterDraft {
            references: vec![String::from("refs/heads/main")],
            ..filters::FilterDraft::default()
        };
        assert!(quick_filter_unchanged(&filter, &draft));
    }

    #[test]
    fn an_unchanged_quick_draft_does_not_restore_a_cleared_applied_filter() {
        let baseline = (
            vec![String::from("refs/heads/main")],
            vec![String::from("author@example.com")],
        );
        let filter = LogFilter::default();
        let draft = filters::FilterDraft {
            references: baseline.0.clone(),
            authors: baseline.1.clone(),
            ..filters::FilterDraft::default()
        };
        assert!(quick_filter_selection_unchanged(
            Some(&baseline),
            &filter,
            &draft
        ));
    }

    #[test]
    fn quick_filter_values_compare_as_unordered_selections() {
        let filter = LogFilter {
            references: vec![String::from("refs/heads/a"), String::from("refs/heads/b")],
            authors: vec![String::from("a@example.com"), String::from("b@example.com")],
            ..LogFilter::default()
        };
        let draft = filters::FilterDraft {
            references: vec![String::from("refs/heads/b"), String::from("refs/heads/a")],
            authors: vec![String::from("b@example.com"), String::from("a@example.com")],
            ..filters::FilterDraft::default()
        };
        assert!(quick_filter_unchanged(&filter, &draft));
    }
}
