//! 功能切片：每个功能一个目录，`state` / `commands` / `view` 之外可以有 `ops`。
//!
//! `state.rs` 只放本功能的 `Signal` 集合，`commands.rs` 只放本功能的用例（调
//! `git::Repo` / `ops` 并更新 state），`ops.rs` 放本功能对 `Tab` 的写操作，
//! `view.rs` 只放本功能的渲染。

use crate::app::session::Tab;
use nsh_core::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;

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
        let tab_id = self.tabs.get_untracked().get(current).map(|tab| tab.id);
        if let Some(tab_id) = tab_id {
            self.ensure_tab_loaded(tab_id);
        }
    }

    /// 按标签身份启动一次延迟装载；重复激活和在途任务不会启动重复 Git 读取。
    fn ensure_tab_loaded(&self, tab_id: u64) {
        let tabs = self.tabs;
        let mut list = tabs.get_untracked();
        let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) else {
            return;
        };
        if tab.loaded || tab.loading || tab.path.is_empty() || self.auxiliary {
            return;
        }

        let load_id = crate::app::session::next_tab_id();
        tab.load_id = load_id;
        tab.loading = true;
        let path = tab.path.clone();
        let filter = tab.filter.clone();
        let executable = tab.git_executable.clone();
        tabs.set(list);

        let recent = self.recent;
        let store_dir = self.store_dir.clone();
        let persist_session = self.persist_session;
        self.task_owner.run(|| {
            spawn(async move {
                let mut loaded = crate::app::session::load_tab_with_executable(
                    path.as_str(),
                    &filter,
                    &executable,
                );
                let branch = match &loaded.head {
                    git::HeadState::Branch(name) => Some(name.clone()),
                    git::HeadState::Unborn { branch } => Some(branch.clone()),
                    git::HeadState::Detached { .. } => None,
                };
                let loaded_path = loaded.path.clone();
                let mut list = tabs.get_untracked();
                let Some(index) = list
                    .iter()
                    .position(|tab| tab.id == tab_id && tab.load_id == load_id)
                else {
                    return;
                };
                loaded.id = tab_id;
                loaded.load_id = load_id;
                list[index] = loaded;
                tabs.set(list);
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
    /// 先插入一个「加载中」的占位标签页，再在后台读仓库：大仓库的
    /// `git log`/`status` 不再卡住界面，用户也能看到骨架。
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
        let active = self.active;
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
            let tab_id = list[index].id;
            active.set(index);
            tabs.set(list);
            self.ensure_tab_loaded(tab_id);
            if !auxiliary {
                welcome_open.set(false);
                nsh_core::element::desktop::main_window().show();
                nsh_core::element::desktop::main_window().focus();
            }
            if persist_session {
                store::remember_recent(&store_dir, &recent.get_untracked(), path.as_str());
                recent.set(store::load_recent(&store_dir));
                crate::persist(&store_dir, active, tabs);
            }
            return;
        }
        let mut placeholder = crate::app::session::placeholder_tab(path.as_str(), &filter);
        placeholder.git_executable = executable.clone();
        list.push(placeholder);
        active.set(list.len() - 1);
        let tab_id = list.last().map(|tab| tab.id).unwrap_or_default();
        let load_id = list
            .iter()
            .find(|tab| tab.id == tab_id)
            .map(|tab| tab.load_id)
            .unwrap_or_default();
        tabs.set(list);
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
            crate::persist(&store_dir, active, tabs);
        }

        let path_for_load = path.clone();
        let branch_dir = store_dir.clone();
        // 欢迎窗口关闭会销毁其内容 owner；在会话 owner 下启动，保证切换窗口后任务仍能完成。
        self.task_owner.run(|| {
            spawn(async move {
                let loaded = crate::app::session::load_tab_with_executable(
                    path_for_load.as_str(),
                    &filter,
                    &executable,
                );
                // 读到 HEAD 后把分支写回最近仓库，欢迎页下次打开时能显示 chip。
                let branch = match &loaded.head {
                    git::HeadState::Branch(name) => Some(name.clone()),
                    git::HeadState::Unborn { branch } => Some(branch.clone()),
                    git::HeadState::Detached { .. } => None,
                };
                let loaded_path = loaded.path.clone();
                let mut list = tabs.get_untracked();
                // 关闭后同路径重开会得到新 load_id，旧任务不能写回新标签。
                let Some(index) = list
                    .iter()
                    .position(|candidate| candidate.id == tab_id && candidate.load_id == load_id)
                else {
                    return;
                };
                let mut loaded = loaded;
                loaded.id = tab_id;
                loaded.load_id = load_id;
                list[index] = loaded;
                tabs.set(list);
                if persist_session && let Some(branch) = branch {
                    store::set_recent_branch(&branch_dir, &loaded_path, &branch);
                    recent.set(store::load_recent(&branch_dir));
                }
            });
        });
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
        let list = self.tabs.get_untracked();
        let Some(tab) = list.get(current) else {
            return;
        };
        let parent = crate::app::session::parent_path(tab.path.as_str());
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
        let Some(tab) = self.tabs.get_untracked().get(index).cloned() else {
            return;
        };
        self.close_tab_id(tab.id);
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
            self.ensure_tab_loaded(tab_id);
        }
        self.persist();
        if self.tabs.get_untracked().is_empty() {
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
        self.tabs.set(list.clone());
        if let Some(active_id) = active_id {
            if let Some(index) = list.iter().position(|tab| tab.id == active_id) {
                self.active.set(index);
            }
        }
        self.persist();
    }

    /// 切到第 `index` 个标签页并写回会话（原 `Actions::activate_tab`）。
    pub(crate) fn activate_tab(&self, index: usize) {
        let active = self.active;
        let tabs = self.tabs;
        let session = self.clone();
        let list = tabs.get_untracked();
        let Some(tab_id) = list.get(index).map(|tab| tab.id) else {
            return;
        };
        active.set(index);
        session.ensure_tab_loaded(tab_id);
        session.persist();
    }

    pub(crate) fn activate_tab_id(&self, tab_id: u64) {
        let Some(index) = self
            .tabs
            .get_untracked()
            .iter()
            .position(|tab| tab.id == tab_id)
        else {
            return;
        };
        self.activate_tab(index);
    }

    /// 前/后切标签页，到头回绕（符合 macOS 习惯）；不足两个标签时不动。
    pub(crate) fn cycle_tab(&self, forward: bool) {
        let tabs = self.tabs;
        let active = self.active;
        let list = tabs.get_untracked();
        let count = list.len();
        if count < 2 {
            return;
        }
        let current = active.get_untracked();
        let next = if forward {
            (current + 1) % count
        } else {
            (current + count - 1) % count
        };
        let tab_id = list[next].id;
        active.set(next);
        self.ensure_tab_loaded(tab_id);
        self.persist();
    }

    /// 切到第 `slot` 个标签页；越界不动（不把用户送到空标签）。
    pub(crate) fn activate_slot(&self, slot: usize) {
        let tabs = self.tabs;
        let active = self.active;
        let list = tabs.get_untracked();
        let Some(tab_id) = list.get(slot).map(|tab| tab.id) else {
            return;
        };
        active.set(slot);
        self.ensure_tab_loaded(tab_id);
        self.persist();
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
