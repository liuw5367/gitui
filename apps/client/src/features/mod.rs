//! 功能切片：每个功能一个目录，`state` / `commands` / `view` 之外可以有 `ops`。
//!
//! `state.rs` 只放本功能的 `Signal` 集合，`commands.rs` 只放本功能的用例（调
//! `git::Repo` / `ops` 并更新 state），`ops.rs` 放本功能对 `Tab` 的写操作，
//! `view.rs` 只放本功能的渲染。

use crate::app::session::Tab;
use nsh_core::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub(crate) mod agent;
pub(crate) mod changes;
pub(crate) mod commit;
pub(crate) mod history;
pub(crate) mod merge;
pub(crate) mod recovery;
pub(crate) mod refs;
pub(crate) mod search;
pub(crate) mod settings;
pub(crate) mod sync;
pub(crate) mod welcome;

/// 标签页集合与活动页签的句柄。用例要读写当前标签页时用它，避免逐个传信号。
///
/// 除了标签页本身，还收打开仓库、返回欢迎页、重新定位、关闭与切换页签这些
/// **跨功能的会话动作** 所需的句柄：屏幕、欢迎窗口、最近仓库、数据目录与当前
/// 筛选条件。辅助工作树窗口使用独立标签集合，并关闭主会话持久化。
///
/// P4 会把 `Tab` 的壳收进 `app/session.rs`，届时这里只剩类型别名。
#[derive(Clone)]
pub(crate) struct SessionHandle {
    pub(crate) tabs: Signal<Vec<Tab>>,
    pub(crate) active: Signal<usize>,
    /// 仓库加载任务的 owner：不随欢迎窗口等短生命周期视图关闭而取消。
    pub(crate) task_owner: Owner,
    /// 欢迎窗口是否打开（独立窗口；打开仓库后关掉，关掉最后一个标签页后重新打开）。
    pub(crate) welcome_open: Signal<bool>,
    /// 设置窗口是否打开（独立窗口；菜单、工具栏与 Done 都只改这个信号）。
    pub(crate) settings_open: Signal<bool>,
    /// 当前主区模式；刷新时只重读当前页面的文件级补丁。
    pub(crate) mode: Signal<crate::Mode>,
    /// 最近仓库列表：仓库成功打开后写入，扫描发现不修改此列表。
    pub(crate) recent: Signal<Vec<store::RecentRepo>>,
    /// 应用数据目录：会话、最近仓库与设置都落在这里。
    pub(crate) store_dir: PathBuf,
    /// 当前历史筛选条件：打开仓库时用它建占位标签页。
    pub(crate) filter: Arc<dyn Fn() -> git::LogFilter>,
    /// 每次新操作读取当前设置；Repo 与后台任务保存启动时的 executable 快照。
    pub(crate) git_executable: Arc<dyn Fn() -> git::GitExecutable + Send + Sync>,
    /// 状态栏短提示（「在此初始化」的成功与失败都要提示）。
    pub(crate) notice: Signal<Option<String>>,
    /// 当前界面语言（「在此初始化」的提示文案按它取）。
    pub(crate) language: Signal<SharedString>,
    /// Auxiliary worktree windows own a single repo tab and never write the main session.
    pub(crate) auxiliary: bool,
    pub(crate) persist_session: bool,
    pub(crate) window: Signal<Option<nsh_core::element::desktop::WindowRef>>,
    /// Welcome 窗口的原生引用；关闭时清空，重复入口用它置前。
    pub(crate) welcome_window: Signal<Option<nsh_core::element::desktop::WindowRef>>,
    /// Settings 窗口的原生引用；关闭时清空，重复入口用它置前。
    pub(crate) settings_window: Signal<Option<nsh_core::element::desktop::WindowRef>>,
    /// 每个标签最多运行一个刷新任务；运行期间的重复事件合并为一次后续读取。
    pub(crate) refresh_requests: Arc<Mutex<HashMap<u64, bool>>>,
}

impl SessionHandle {
    pub(crate) fn current_git_executable(&self) -> git::GitExecutable {
        return (self.git_executable)();
    }

    pub(crate) fn open_repo(&self, path: &str) -> Result<git::Repo, domain::GitError> {
        let executable = self.current_git_executable();
        return git::Repo::open_with_executable(path, &executable);
    }

    /// 首次进入工作区时优先装载当前标签；其余恢复标签留待用户首次选择。
    pub(crate) fn ensure_active_tab_loaded(&self) {
        let current = self.active.get_untracked();
        let tab_id = self
            .tabs
            .with_untracked(|tabs| tabs.get(current).map(|tab| tab.id));
        if let Some(tab_id) = tab_id {
            self.ensure_tab_loaded(tab_id);
        }
    }

    /// 刷新当前活动仓库。只对已加载的主会话标签启动后台 Git 查询。
    pub(crate) fn refresh_active_tab(&self) {
        let current = self.active.get_untracked();
        let tab_id = self
            .tabs
            .with_untracked(|tabs| tabs.get(current).map(|tab| tab.id));
        if let Some(tab_id) = tab_id {
            self.refresh_tab(tab_id);
        }
    }

    fn refresh_tab(&self, tab_id: u64) {
        if self.auxiliary {
            return;
        }
        let current = self.active.get_untracked();
        let can_refresh = self.tabs.with_untracked(|tabs| {
            tabs.get(current)
                .filter(|tab| tab.id == tab_id && tab.loaded && tab.error.is_none())
                .is_some()
        });
        if !can_refresh {
            return;
        }
        let should_start = {
            let mut requests = self
                .refresh_requests
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            match requests.get_mut(&tab_id) {
                Some(pending) => {
                    *pending = true;
                    false
                }
                None => {
                    requests.insert(tab_id, false);
                    true
                }
            }
        };
        if !should_start {
            return;
        }

        let session = self.clone();
        self.task_owner.run(|| {
            spawn(async move {
                loop {
                    let current = session.active.get_untracked();
                    let request = session.tabs.with_untracked(|list| {
                        list.get(current)
                            .filter(|tab| tab.id == tab_id && tab.loaded && tab.error.is_none())
                            .map(|tab| {
                                (
                                    tab.id,
                                    tab.load_id,
                                    tab.path.clone(),
                                    crate::app::session::RefreshInput::from_tab(tab),
                                )
                            })
                    });
                    let Some((tab_id_at_start, load_id_at_start, path_at_start, refresh_input)) =
                        request
                    else {
                        session.finish_refresh_request(tab_id);
                        return;
                    };
                    let mode = session.mode.get_untracked();
                    let refresh_started = std::time::Instant::now();
                    let refresh_result = match blocking(move || {
                        crate::app::session::read_refresh_snapshot(&refresh_input, mode)
                    })
                    .await
                    {
                        Ok(result) => result,
                        Err(error) => Err(format!("仓库刷新任务失败：{error}")),
                    };
                    match refresh_result {
                        Ok(snapshot) => {
                            log::debug!(
                                target: "gitui::history",
                                "event=repo_refresh.complete tab_id={} elapsed_ms={}",
                                tab_id,
                                refresh_started.elapsed().as_millis(),
                            );
                            if session.mode.get_untracked() != mode {
                                session.request_refresh_again(tab_id);
                                if !session.repeat_or_finish_refresh(tab_id) {
                                    return;
                                }
                                continue;
                            }
                            let mut list = session.tabs.get_untracked();
                            let current = session.active.get_untracked();
                            let is_still_active = list.get(current).is_some_and(|active| {
                                active.id == tab_id_at_start
                                    && active.load_id == load_id_at_start
                                    && active.path == path_at_start
                            });
                            if !is_still_active {
                                session.finish_refresh_request(tab_id);
                                return;
                            }
                            if let Some(tab) = list.iter_mut().find(|tab| {
                                tab.id == tab_id_at_start
                                    && tab.load_id == load_id_at_start
                                    && tab.path == path_at_start
                            }) && crate::app::session::apply_refresh_snapshot(tab, snapshot)
                            {
                                session.tabs.set(list);
                            }
                        }
                        Err(detail) => {
                            log::warn!(
                                target: "gitui::history",
                                "event=repo_refresh.failed tab_id={} elapsed_ms={} reason={}",
                                tab_id,
                                refresh_started.elapsed().as_millis(),
                                detail,
                            );
                            crate::interactions::notify(session.notice, detail);
                        }
                    }
                    if !session.repeat_or_finish_refresh(tab_id) {
                        return;
                    }
                }
            });
        });
    }

    fn repeat_or_finish_refresh(&self, tab_id: u64) -> bool {
        let mut requests = self
            .refresh_requests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match requests.get_mut(&tab_id) {
            Some(pending) if *pending => {
                *pending = false;
                return true;
            }
            Some(_) => {
                requests.remove(&tab_id);
            }
            None => {}
        }
        return false;
    }

    fn request_refresh_again(&self, tab_id: u64) {
        if let Some(pending) = self
            .refresh_requests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_mut(&tab_id)
        {
            *pending = true;
        }
    }

    fn finish_refresh_request(&self, tab_id: u64) {
        self.refresh_requests
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&tab_id);
    }

    /// 按标签身份启动一次延迟装载；重复激活和在途任务不会启动重复 Git 读取。
    fn ensure_tab_loaded(&self, tab_id: u64) {
        let tabs = self.tabs;
        if self.auxiliary {
            return;
        }
        let load_id = crate::app::session::next_tab_id();
        let request = tabs.with_untracked(|list| {
            list.iter()
                .find(|tab| tab.id == tab_id)
                .filter(|tab| !tab.loaded && !tab.loading && !tab.path.is_empty())
                .map(|tab| {
                    (
                        tab.path.clone(),
                        tab.filter.clone(),
                        tab.git_executable.clone(),
                    )
                })
        });
        let Some((path, filter, executable)) = request else {
            return;
        };
        tabs.update(|list| {
            if let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) {
                tab.load_id = load_id;
                tab.loading = true;
            }
        });

        let recent = self.recent;
        let store_dir = self.store_dir.clone();
        let persist_session = self.persist_session;
        let load_started = std::time::Instant::now();
        log::debug!(
            target: "gitui::history",
            "event=repo_load.requested tab_id={}",
            tab_id,
        );
        self.task_owner.run(|| {
            spawn(async move {
                let loaded_result = blocking(move || {
                    crate::app::session::load_tab_with_executable(
                        path.as_str(),
                        &filter,
                        &executable,
                    )
                })
                .await;
                let mut loaded = match loaded_result {
                    Ok(loaded) => loaded,
                    Err(error) => {
                        tabs.update(|list| {
                            if let Some(tab) = list
                                .iter_mut()
                                .find(|tab| tab.id == tab_id && tab.load_id == load_id)
                            {
                                tab.loading = false;
                                tab.error = Some(error.to_string());
                            }
                        });
                        log::error!(
                            target: "gitui::history",
                            "event=repo_load.failed tab_id={} reason={}",
                            tab_id,
                            error,
                        );
                        return;
                    }
                };
                let branch = match &loaded.head {
                    git::HeadState::Branch(name) => Some(name.clone()),
                    git::HeadState::Unborn { branch } => Some(branch.clone()),
                    git::HeadState::Detached { .. } => None,
                };
                let loaded_path = loaded.path.clone();
                let exists = tabs.with_untracked(|list| {
                    list.iter()
                        .any(|tab| tab.id == tab_id && tab.load_id == load_id)
                });
                if !exists {
                    return;
                }
                loaded.id = tab_id;
                loaded.load_id = load_id;
                tabs.update(|list| {
                    if let Some(tab) = list
                        .iter_mut()
                        .find(|tab| tab.id == tab_id && tab.load_id == load_id)
                    {
                        *tab = loaded;
                    }
                });
                log::debug!(
                    target: "gitui::history",
                    "event=repo_load.available tab_id={} elapsed_ms={}",
                    tab_id,
                    load_started.elapsed().as_millis(),
                );
                if persist_session && let Some(branch) = branch {
                    store::set_recent_branch(&store_dir, &loaded_path, &branch);
                    recent.set(store::load_recent(&store_dir));
                }
            });
        });
    }

    /// 设置页变更 Git 路径后，更新已打开标签；已经克隆出的 Repo 仍保留旧快照。
    pub(crate) fn refresh_git_executable(&self) {
        let executable = self.current_git_executable();
        let mut tabs = self.tabs.get_untracked();
        let mut changed = false;
        for tab in &mut tabs {
            if tab.git_executable != executable {
                tab.git_executable = executable.clone();
                changed = true;
            }
        }
        if changed {
            self.tabs.set(tabs);
        }
    }

    /// 打开一个仓库（原 `Actions::open_path`）。
    ///
    /// 先插入占位标签页并切换选中态，下一帧再启动后台装载：大仓库的
    /// `git log`/`status` 不会卡住界面，用户能先看到新标签被选中。
    pub(crate) fn open_path(&self, path: String) {
        let executable = self.current_git_executable();
        let repo = match git::Repo::open_with_executable(path.trim(), &executable) {
            Ok(repo) => repo,
            Err(error) => {
                if !self.auxiliary {
                    self.go_welcome();
                }
                let language = self.language.get_untracked();
                crate::interactions::alert(
                    crate::i18n::t(language.as_str(), "welcome.openAction"),
                    crate::features::welcome::commands::localized_error(language.as_str(), &error),
                    crate::i18n::t(language.as_str(), "common.close"),
                );
                return;
            }
        };
        let path = repo.root().display().to_string();
        let welcome_open = self.welcome_open;
        let tabs = self.tabs;
        let recent = self.recent;
        let store_dir = self.store_dir.clone();
        let current_filter = self.filter.clone();
        let filter = current_filter();
        let auxiliary = self.auxiliary;
        let persist_session = self.persist_session;
        let mut list = tabs.get_untracked();
        if let Some(index) = list
            .iter()
            .position(|candidate| candidate.path == path.as_str())
        {
            self.activate_tab(index);
            if !auxiliary {
                welcome_open.set(false);
                nsh_core::element::desktop::main_window().show();
                nsh_core::element::desktop::main_window().focus();
            }
            if persist_session {
                store::remember_recent(&store_dir, &recent.get_untracked(), path.as_str());
                recent.set(store::load_recent(&store_dir));
            }
            return;
        }
        let mut placeholder = crate::app::session::placeholder_tab(path.as_str(), &filter);
        placeholder.git_executable = executable.clone();
        list.push(placeholder);
        let index = list.len() - 1;
        tabs.set(list);
        self.activate_tab(index);
        if persist_session {
            store::remember_recent(&store_dir, &recent.get_untracked(), path.as_str());
            recent.set(store::load_recent(&store_dir));
        }
        // 从欢迎页进入主窗口：关掉欢迎窗口并聚焦主窗口。
        if !auxiliary {
            welcome_open.set(false);
            nsh_core::element::desktop::main_window().show();
            nsh_core::element::desktop::main_window().focus();
        }
        if persist_session {
            self.persist();
        }
    }

    /// 刷新最近仓库记录里的当前分支，供欢迎页名称右侧的分支 tag 使用。
    ///
    /// 记录里的分支是上次打开时的快照：仓库在外部切换过分支，或旧数据没有分支列，
    /// 欢迎页就会缺 tag。这里重新读一次 HEAD；读不到仓库时保留原值，游离 HEAD
    /// 则清空分支（没有“当前分支”可显示）。
    pub(crate) fn refresh_recent_branches(&self) {
        let recent = self.recent;
        let store_dir = self.store_dir.clone();
        let executable = self.current_git_executable();
        spawn(async move {
            let list = recent.get_untracked();
            let mut changed = false;
            for entry in &list {
                let Ok(repo) = git::Repo::open_with_executable(entry.path.as_str(), &executable)
                else {
                    continue;
                };
                let branch = match repo.head_state() {
                    git::HeadState::Branch(name) => name,
                    git::HeadState::Unborn { branch } => branch,
                    git::HeadState::Detached { .. } => String::new(),
                };
                if branch != entry.branch {
                    store::set_recent_branch(&store_dir, entry.path.as_str(), &branch);
                    changed = true;
                }
            }
            if changed {
                recent.set(store::load_recent(&store_dir));
            }
        });
    }

    /// 打开欢迎窗口（独立窗口，由 `welcome_open` 驱动）。
    pub(crate) fn go_welcome(&self) {
        if let Some(window) = self.welcome_window.get_untracked() {
            self.welcome_open.set(true);
            window.focus();
            return;
        }
        // 列表要显示当前分支，而不是上次打开时的快照。
        self.refresh_recent_branches();
        self.welcome_open.set(true);
    }

    /// 打开仓库管理页（原 `Actions::go_repo_manager`）：入口就是欢迎窗口。
    pub(crate) fn go_repo_manager(&self) {
        self.go_welcome();
    }

    /// 重新定位缺失的仓库（原 `Actions::locate_repo`）：选一个新目录打开，
    /// 然后关掉这个打不开的标签页。
    pub(crate) fn locate_repo(&self) {
        let Some((old_tab_id, old_path)) = self
            .tabs
            .get_untracked()
            .get(self.active.get_untracked())
            .map(|tab| (tab.id, tab.path.clone()))
        else {
            return;
        };
        let session = self.clone();
        let answer = pick_paths(nsh_core::file_dialog::OpenDialog::folders());
        spawn(async move {
            let Ok(Some(paths)) = answer.await else {
                return;
            };
            let Some(path) = paths.first() else {
                return;
            };
            let chosen = path.display().to_string();
            let Ok(repo) = session.open_repo(chosen.as_str()) else {
                return;
            };
            let chosen_path = repo.root().display().to_string();
            session.open_path(chosen_path.clone());
            if chosen_path == old_path {
                return;
            }
            // 先打开再关旧的：关掉会让下标前移。
            session.close_tab_id(old_tab_id);
        });
    }

    /// 打开当前仓库路径的上一级（原 `Actions::open_parent`，不是仓库时的补救入口）。
    pub(crate) fn open_parent(&self) {
        let current = self.active.get_untracked();
        let parent = self.tabs.with_untracked(|list| {
            list.get(current)
                .map(|tab| crate::app::session::parent_path(tab.path.as_str()))
        });
        let Some(parent) = parent else {
            return;
        };
        if !parent.is_empty() {
            self.open_path(parent);
        }
    }

    /// 在当前标签页目录里初始化仓库（原 `Actions::init_here`）。
    pub(crate) fn init_here(&self) {
        let current = self.active.get_untracked();
        let mut list = self.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            match crate::app::session::init_here(tab) {
                Ok(()) => {
                    let lang = self.language.get_untracked();
                    crate::interactions::notify(
                        self.notice,
                        crate::i18n::t(lang.as_str(), "toast.initDone"),
                    );
                }
                Err(detail) => crate::interactions::notify(self.notice, detail),
            }
            self.tabs.set(list);
        }
    }

    /// 关闭指定标签页；未提交改动保留在磁盘上，不弹确认。
    pub(crate) fn close_tab(&self, index: usize) {
        let tab_id = self
            .tabs
            .with_untracked(|tabs| tabs.get(index).map(|tab| tab.id));
        let Some(tab_id) = tab_id else {
            return;
        };
        self.close_tab_id(tab_id);
    }

    /// 按运行期身份关闭标签，避免顺序变化时关闭错误的仓库。
    pub(crate) fn close_tab_id(&self, tab_id: u64) {
        let mut list = self.tabs.get_untracked();
        let Some(index) = list.iter().position(|tab| tab.id == tab_id) else {
            return;
        };
        let old_active = self
            .active
            .get_untracked()
            .min(list.len().saturating_sub(1));
        let removed_active = old_active == index;
        list.remove(index);
        let next_active = if list.is_empty() || removed_active {
            index.min(list.len().saturating_sub(1))
        } else if index < old_active {
            old_active - 1
        } else {
            old_active
        };
        let next_tab_id = list.get(next_active).map(|tab| tab.id);
        self.tabs.set(list);
        self.active.set(next_active);
        if let Some(tab_id) = next_tab_id {
            self.schedule_active_tab_work(tab_id);
        } else {
            self.persist();
            self.finish_empty_session();
        }
    }

    /// 关闭主窗口时保存当前标签组，供下次启动恢复，然后关闭窗口。
    pub(crate) fn close_workspace(&self) {
        self.persist();
        nsh_core::element::desktop::main_window().close();
    }

    /// 将指定标签放到新位置，并保持原活动仓库不变。
    pub(crate) fn move_tab(&self, tab_id: u64, destination: usize) {
        let mut list = self.tabs.get_untracked();
        let Some(source) = list.iter().position(|tab| tab.id == tab_id) else {
            return;
        };
        if list.is_empty() {
            return;
        }
        let destination = destination.min(list.len() - 1);
        if source == destination {
            return;
        }
        let active_id = list.get(self.active.get_untracked()).map(|tab| tab.id);
        let tab = list.remove(source);
        list.insert(destination, tab);
        let next_active =
            active_id.and_then(|active_id| list.iter().position(|tab| tab.id == active_id));
        self.tabs.set(list);
        if let Some(index) = next_active {
            self.active.set(index);
        }
        self.persist();
    }

    fn schedule_active_tab_work(&self, tab_id: u64) {
        if self.auxiliary {
            return;
        }
        let session = self.clone();
        self.task_owner.run(|| {
            spawn(async move {
                // Give the active-tab change a complete frame before loading or refreshing.
                next_frame().await;
                let active_tab_id = session.tabs.with_untracked(|list| {
                    list.get(session.active.get_untracked()).map(|tab| tab.id)
                });
                if active_tab_id != Some(tab_id) {
                    return;
                }
                session.ensure_tab_loaded(tab_id);
                session.refresh_tab(tab_id);
                session.persist();
            });
        });
    }

    /// 切到第 `index` 个标签页并写回会话（原 `Actions::activate_tab`）。
    pub(crate) fn activate_tab(&self, index: usize) {
        let started = std::time::Instant::now();
        let active = self.active;
        let tabs = self.tabs;
        let tab = tabs.with_untracked(|list| list.get(index).map(|tab| (tab.id, tab.loaded)));
        let Some((tab_id, content_cached)) = tab else {
            return;
        };
        active.set(index);
        self.schedule_active_tab_work(tab_id);
        let tab_count = tabs.with_untracked(|list| list.len());
        log::debug!(
            target: "gitui::history",
            "event=tab_activate.handler tab_id={} index={} tabs={} content_cached={} elapsed_ms={}",
            tab_id,
            index,
            tab_count,
            content_cached,
            started.elapsed().as_millis(),
        );
    }

    pub(crate) fn activate_tab_id(&self, tab_id: u64) {
        let index = self
            .tabs
            .with_untracked(|tabs| tabs.iter().position(|tab| tab.id == tab_id));
        let Some(index) = index else {
            return;
        };
        self.activate_tab(index);
    }

    /// 前/后切标签页，到头回绕（符合 macOS 习惯）；不足两个标签时不动。
    pub(crate) fn cycle_tab(&self, forward: bool) {
        let current = self.active.get_untracked();
        let next = self.tabs.with_untracked(|tabs| {
            let count = tabs.len();
            if count < 2 {
                return None;
            }
            let index = if forward {
                (current + 1) % count
            } else {
                (current + count - 1) % count
            };
            return Some(index);
        });
        let Some(next) = next else {
            return;
        };
        self.activate_tab(next);
    }

    /// 切到第 `slot` 个标签页；越界不动（不把用户送到空标签）。
    pub(crate) fn activate_slot(&self, slot: usize) {
        let exists = self.tabs.with_untracked(|tabs| tabs.get(slot).is_some());
        if !exists {
            return;
        }
        self.activate_tab(slot);
    }

    fn persist(&self) {
        if self.persist_session {
            crate::persist(&self.store_dir, self.active, self.tabs);
        }
    }

    fn finish_empty_session(&self) {
        if self.auxiliary {
            if let Some(window) = self.window.get_untracked() {
                window.close();
            }
        } else {
            nsh_core::element::desktop::main_window().hide();
            self.go_welcome();
        }
    }
}

#[cfg(test)]
mod session_tab_tests {
    use super::*;

    #[derive(Clone, PartialEq, Props)]
    struct TabActivationFixtureProps {
        path: String,
        git_wrapper: String,
        started: String,
        release: String,
        selected_painted: String,
        loading_painted: String,
    }

    #[component]
    fn TabActivationFixture(props: TabActivationFixtureProps) -> impl IntoElement {
        let mut session = session_with_tabs(&[1, 2], 0);
        session.auxiliary = false;
        let executable = git::GitExecutable::from_path(props.git_wrapper)
            .with_env("GITUI_TEST_STARTED", props.started)
            .with_env("GITUI_TEST_RELEASE", props.release);
        session.tabs.update(|tabs| {
            tabs[0].name = "repo-1".to_string();
            tabs[0].path = props.path.clone();
            tabs[0].loaded = true;
            let id = tabs[1].id;
            let mut placeholder = crate::app::session::placeholder_tab(
                props.path.as_str(),
                &git::LogFilter::default(),
            );
            placeholder.id = id;
            placeholder.git_executable = executable;
            tabs[1] = placeholder;
        });
        let active = session.active;
        let tabs = session.tabs;
        let activate = session.clone();
        let selected_painted = std::path::PathBuf::from(props.selected_painted);
        let loading_painted = std::path::PathBuf::from(props.loading_painted);
        return view()
            .child(crate::ui::button::button(
                || "Select repository 2".to_string(),
                crate::ui::button::ButtonVariant::Secondary,
                crate::ui::button::ButtonSize::Small,
                false,
                move || activate.activate_tab(1),
            ))
            .child(dynamic(move || {
                let index = active.get();
                let (loaded, loading) = tabs.with(|list| {
                    list.get(index)
                        .map(|tab| (tab.loaded, tab.loading))
                        .unwrap_or((false, false))
                });
                let selected_painted = selected_painted.clone();
                let loading_painted = loading_painted.clone();
                let state = if loaded {
                    "loaded"
                } else if loading {
                    "loading"
                } else {
                    "waiting"
                };
                view()
                    .w_full()
                    .h(px(if index == 1 && loaded {
                        34.0
                    } else if index == 1 && loading {
                        32.0
                    } else if index == 1 {
                        30.0
                    } else {
                        28.0
                    }))
                    .on_layout(move |_: &Bounds<Points>| {
                        if index == 1 && !loaded && !loading && !selected_painted.exists() {
                            let _ = std::fs::write(&selected_painted, "painted");
                        }
                        if index == 1 && loading && !loading_painted.exists() {
                            let _ = std::fs::write(&loading_painted, "painted");
                        }
                    })
                    .child(text(format!("selected {index} {state}")))
                    .into_element()
            }));
    }

    fn session_with_tabs(ids: &[u64], active_index: usize) -> SessionHandle {
        let tabs = ids
            .iter()
            .map(|id| Tab {
                id: *id,
                path: format!("/repo-{id}"),
                name: format!("repo-{id}"),
                ..Tab::default()
            })
            .collect();
        return SessionHandle {
            tabs: signal(tabs),
            active: signal(active_index),
            welcome_open: signal(false),
            settings_open: signal(false),
            mode: signal(crate::Mode::History),
            recent: signal(Vec::new()),
            store_dir: std::path::PathBuf::new(),
            filter: Arc::new(git::LogFilter::default),
            git_executable: Arc::new(git::GitExecutable::system),
            notice: signal(None),
            language: signal(SharedString::from("en")),
            task_owner: Owner::current(),
            auxiliary: true,
            persist_session: false,
            window: signal(None),
            welcome_window: signal(None),
            settings_window: signal(None),
            refresh_requests: Arc::new(Mutex::new(HashMap::new())),
        };
    }

    #[test]
    fn changing_git_executable_updates_open_tabs_and_keeps_operation_snapshot() {
        let runtime = nsh_core::element::reactive::Runtime::default();
        let _entered = runtime.enter();
        let (_owner, mut session) =
            nsh_core::element::reactive::create_root(|| session_with_tabs(&[1], 0));
        let selected = Arc::new(std::sync::Mutex::new(git::GitExecutable::from_path(
            "/custom/git-old",
        )));
        let resolver_state = selected.clone();
        session.git_executable = Arc::new(move || resolver_state.lock().unwrap().clone());
        let mut tabs = session.tabs.get_untracked();
        tabs[0].git_executable = git::GitExecutable::from_path("/custom/git-old");
        let in_flight_snapshot = tabs[0].git_executable.clone();
        session.tabs.set(tabs);

        *selected.lock().unwrap() = git::GitExecutable::from_path("/custom/git-new");
        session.refresh_git_executable();

        let current = session.tabs.get_untracked();
        assert_eq!(
            current[0].git_executable,
            git::GitExecutable::from_path("/custom/git-new")
        );
        assert_eq!(
            in_flight_snapshot,
            git::GitExecutable::from_path("/custom/git-old")
        );
    }

    #[test]
    fn selected_tab_is_painted_before_its_git_load_finishes() {
        use std::os::unix::fs::PermissionsExt;

        let repo = crate::test_support::TempRepo::new("tab-activation");
        let gate_dir = std::env::temp_dir().join(format!(
            "nsh-app-tab-activation-{}-{}",
            std::process::id(),
            crate::app::session::next_tab_id()
        ));
        std::fs::create_dir_all(&gate_dir).expect("gate dir");
        let git_wrapper = gate_dir.join("git-wrapper.sh");
        let started = gate_dir.join("started");
        let release = gate_dir.join("release");
        let selected_painted = gate_dir.join("selected-painted");
        let loading_painted = gate_dir.join("loading-painted");
        std::fs::write(
            &git_wrapper,
            "#!/bin/sh\n\
             if [ ! -e \"$GITUI_TEST_RELEASE\" ]; then\n\
               : > \"$GITUI_TEST_STARTED\"\n\
               while [ ! -e \"$GITUI_TEST_RELEASE\" ]; do /bin/sleep 0.01; done\n\
             fi\n\
             exec git \"$@\"\n",
        )
        .expect("write git wrapper");
        std::fs::set_permissions(&git_wrapper, std::fs::Permissions::from_mode(0o755))
            .expect("make git wrapper executable");
        let started_for_release = started.clone();
        let release_for_thread = release.clone();
        let loading_painted_for_release = loading_painted.clone();
        let release_thread = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while (!started_for_release.exists() || !loading_painted_for_release.exists())
                && std::time::Instant::now() < deadline
            {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            if started_for_release.exists() {
                let _ = std::fs::write(release_for_thread, "continue");
            }
        });
        let path = repo.path.display().to_string();
        let git_wrapper_path = git_wrapper.display().to_string();
        let started_path = started.display().to_string();
        let release_path = release.display().to_string();
        let selected_painted_path = selected_painted.display().to_string();
        let loading_painted_path = loading_painted.display().to_string();
        let mut app = nsh_testing::Headless::start(nsh_engine::EngineConfig {
            viewport: size(px(500.0), px(120.0)),
            ..nsh_engine::EngineConfig::new(nsh_core::element::Root::new::<TabActivationFixture>(
                move || TabActivationFixtureProps {
                    path: path.clone(),
                    git_wrapper: git_wrapper_path.clone(),
                    started: started_path.clone(),
                    release: release_path.clone(),
                    selected_painted: selected_painted_path.clone(),
                    loading_painted: loading_painted_path.clone(),
                },
            ))
        })
        .expect("headless app");
        app.first_render().expect("first render");
        app.wait_for("selected 0 loaded").expect("initial tab");
        app.press("Select repository 2").expect("select second tab");
        assert!(
            selected_painted.exists(),
            "selected state should paint before loading starts"
        );
        assert!(
            loading_painted.exists(),
            "loading state should paint while Git is still running"
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !started.exists() && std::time::Instant::now() < deadline {
            app.next_frame(std::time::Duration::from_millis(20))
                .expect("advance loading");
        }
        assert!(started.exists(), "Git load should have started");
        while !release.exists() && std::time::Instant::now() < deadline {
            app.next_frame(std::time::Duration::from_millis(20))
                .expect("wait for Git release");
        }
        assert!(
            release.exists(),
            "test should release the Git load after paint"
        );
        app.wait_for_within("selected 1 loaded", std::time::Duration::from_secs(10))
            .expect("loaded tab");
        let _ = release_thread.join();
        drop(app);
        let _ = std::fs::remove_dir_all(gate_dir);
    }

    #[test]
    fn moving_repository_tabs_keeps_the_active_repository_by_identity() {
        let runtime = nsh_core::element::reactive::Runtime::default();
        let _entered = runtime.enter();
        let (owner, session) =
            nsh_core::element::reactive::create_root(|| session_with_tabs(&[1, 2, 3], 1));
        session.move_tab(3, 0);

        let ids = session
            .tabs
            .get_untracked()
            .iter()
            .map(|tab| tab.id)
            .collect::<Vec<_>>();
        assert_eq!(ids, vec![3, 1, 2]);
        assert_eq!(session.active.get_untracked(), 2);

        session.move_tab(2, 0);
        let list = session.tabs.get_untracked();
        assert_eq!(
            list.iter().map(|tab| tab.id).collect::<Vec<_>>(),
            vec![2, 3, 1]
        );
        assert_eq!(session.active.get_untracked(), 0);
        owner.dispose();
    }

    #[test]
    fn closing_repository_tabs_keeps_or_selects_the_expected_neighbor() {
        let runtime = nsh_core::element::reactive::Runtime::default();
        let _entered = runtime.enter();
        let (owner, session) =
            nsh_core::element::reactive::create_root(|| session_with_tabs(&[1, 2, 3, 4], 1));
        session.close_tab_id(1);
        assert_eq!(
            session
                .tabs
                .get_untracked()
                .iter()
                .map(|tab| tab.id)
                .collect::<Vec<_>>(),
            vec![2, 3, 4]
        );
        assert_eq!(session.active.get_untracked(), 0);

        session.close_tab_id(2);
        assert_eq!(
            session
                .tabs
                .get_untracked()
                .iter()
                .map(|tab| tab.id)
                .collect::<Vec<_>>(),
            vec![3, 4]
        );
        assert_eq!(session.active.get_untracked(), 0);

        session.close_tab_id(4);
        assert_eq!(
            session.tabs.get_untracked()[session.active.get_untracked()].id,
            3
        );
        session.close_tab_id(3);
        assert!(session.tabs.get_untracked().is_empty());
        owner.dispose();
    }
}
