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
/// 筛选条件。这些动作原先以闭包形式放在 `Actions` 里，多个功能都要调，因此
/// 改成这里的方法。
///
/// P4 会把 `Tab` 的壳收进 `app/session.rs`，届时这里只剩类型别名。
#[derive(Clone)]
pub(crate) struct SessionHandle {
    pub(crate) tabs: Signal<Vec<Tab>>,
    pub(crate) active: Signal<usize>,
    /// 当前屏幕（工作区 / 设置）；打开仓库与返回欢迎页都要切它。
    pub(crate) screen: Signal<crate::Screen>,
    /// 欢迎窗口是否打开（它是独立窗口，打开仓库后要关掉）。
    pub(crate) welcome_open: Signal<bool>,
    /// 最近仓库列表：打开、克隆或扫描后写入。
    pub(crate) recent: Signal<Vec<store::RecentRepo>>,
    /// 应用数据目录：会话、最近仓库与设置都落在这里。
    pub(crate) store_dir: PathBuf,
    /// 当前历史筛选条件：打开仓库时用它建占位标签页。
    pub(crate) filter: Arc<dyn Fn() -> git::LogFilter>,
    /// 状态栏短提示（「在此初始化」的成功与失败都要提示）。
    pub(crate) notice: Signal<Option<String>>,
    /// 当前界面语言（「在此初始化」的提示文案按它取）。
    pub(crate) language: Signal<SharedString>,
}

impl SessionHandle {
    /// 打开一个仓库（原 `Actions::open_path`）。
    ///
    /// 先插入一个「加载中」的占位标签页，再在后台读仓库：大仓库的
    /// `git log`/`status` 不再卡住界面，用户也能看到骨架。
    pub(crate) fn open_path(&self, path: String) {
        let welcome_open = self.welcome_open;
        let tabs = self.tabs;
        let active = self.active;
        let recent = self.recent;
        let screen = self.screen;
        let store_dir = self.store_dir.clone();
        let current_filter = self.filter.clone();
        let filter = current_filter();
        let mut list = tabs.get_untracked();
        let existing = list
            .iter()
            .position(|candidate| candidate.path == path.as_str());
        match existing {
            Some(index) => {
                list[index].loading = true;
                active.set(index);
            }
            None => {
                list.push(crate::app::session::placeholder_tab(path.as_str(), &filter));
                active.set(list.len() - 1);
            }
        }
        tabs.set(list);
        store::remember_recent(&store_dir, &recent.get_untracked(), path.as_str());
        recent.set(store::load_recent(&store_dir));
        screen.set(crate::Screen::Workspace);
        // 从欢迎页进入主窗口：关掉欢迎窗口并聚焦主窗口。
        welcome_open.set(false);
        nsh_core::element::desktop::main_window().focus();
        crate::persist(&store_dir, active, tabs);

        let path_for_load = path.clone();
        spawn(async move {
            let loaded = crate::app::session::load_tab(path_for_load.as_str(), &filter);
            let mut list = tabs.get_untracked();
            // 加载期间用户可能关掉了这个标签页，找不到就丢弃结果。
            let Some(index) = list
                .iter()
                .position(|candidate| candidate.path == loaded.path)
            else {
                return;
            };
            list[index] = loaded;
            tabs.set(list);
        });
    }

    /// 返回欢迎页（原 `Actions::go_welcome`）：切回工作区并打开欢迎窗口。
    pub(crate) fn go_welcome(&self) {
        self.screen.set(crate::Screen::Workspace);
        self.welcome_open.set(true);
    }

    /// 打开仓库管理页（原 `Actions::go_repo_manager`）：入口就是欢迎窗口。
    pub(crate) fn go_repo_manager(&self) {
        self.go_welcome();
    }

    /// 重新定位缺失的仓库（原 `Actions::locate_repo`）：选一个新目录打开，
    /// 然后关掉这个打不开的标签页。
    pub(crate) fn locate_repo(&self) {
        let index = self.active.get_untracked();
        let session = self.clone();
        let answer = pick_paths(nsh_core::file_dialog::OpenDialog::folders());
        spawn(async move {
            let Ok(Some(paths)) = answer.await else {
                return;
            };
            let Some(path) = paths.first() else {
                return;
            };
            session.open_path(path.display().to_string());
            // 先打开再关旧的：关掉会让下标前移。
            session.close_tab(index);
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

    /// 关闭指定标签页（原 `Actions::close_tab`）：有未提交改动时先确认。
    pub(crate) fn close_tab(&self, index: usize) {
        let welcome_open = self.welcome_open;
        let tabs = self.tabs;
        let active = self.active;
        let store_dir = self.store_dir.clone();
        let language = self.language;
        // 关闭动作本身；有未提交改动时先确认再走这里。
        let perform: Arc<dyn Fn(usize)> = {
            let welcome_open = welcome_open;
            let tabs = tabs;
            let active = active;
            let store_dir = store_dir.clone();
            Arc::new(move |index: usize| {
                let mut list = tabs.get_untracked();
                if index >= list.len() {
                    return;
                }
                list.remove(index);
                let next_active = if list.is_empty() {
                    0
                } else {
                    active.get_untracked().min(list.len() - 1)
                };
                tabs.set(list.clone());
                active.set(next_active);
                if list.is_empty() {
                    welcome_open.set(true);
                }
                crate::persist(&store_dir, active, tabs);
            })
        };
        let list = tabs.get_untracked();
        let Some(tab) = list.get(index) else {
            return;
        };
        let changes = tab.status.len();
        let name = tab.label();
        if changes == 0 {
            perform(index);
            return;
        }
        let language_now = language.get_untracked();
        let pending = crate::interactions::confirm(
            crate::i18n::with_arguments(
                language_now.as_str(),
                "confirm.closeTitle",
                &[("name", name)],
            ),
            crate::i18n::with_arguments(
                language_now.as_str(),
                "confirm.closeMessage",
                &[("count", changes.to_string())],
            ),
            "close",
            crate::i18n::t(language_now.as_str(), "confirm.closeAction"),
            false,
        );
        let perform = perform.clone();
        spawn(async move {
            if pending.await {
                perform(index);
            }
        });
    }

    /// 切到第 `index` 个标签页并写回会话（原 `Actions::activate_tab`）。
    pub(crate) fn activate_tab(&self, index: usize) {
        let active = self.active;
        let tabs = self.tabs;
        let store_dir = self.store_dir.clone();
        if index < tabs.get_untracked().len() {
            active.set(index);
            crate::persist(&store_dir, active, tabs);
        }
    }

    /// 前/后切标签页，到头回绕（符合 macOS 习惯）；不足两个标签时不动。
    pub(crate) fn cycle_tab(&self, forward: bool) {
        let tabs = self.tabs;
        let active = self.active;
        let screen = self.screen;
        let count = tabs.get_untracked().len();
        if count < 2 {
            return;
        }
        let current = active.get_untracked();
        let next = if forward {
            (current + 1) % count
        } else {
            (current + count - 1) % count
        };
        active.set(next);
        screen.set(crate::Screen::Workspace);
    }

    /// 切到第 `slot` 个标签页；越界不动（不把用户送到空标签）。
    pub(crate) fn activate_slot(&self, slot: usize) {
        let tabs = self.tabs;
        let active = self.active;
        let screen = self.screen;
        if slot < tabs.get_untracked().len() {
            active.set(slot);
            screen.set(crate::Screen::Workspace);
        }
    }
}
