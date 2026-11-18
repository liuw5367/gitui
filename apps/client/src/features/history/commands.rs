//! 提交历史的用例：选中提交/文件、对比、筛选与刷新。
//!
//! 方法体由原 `Actions` 里的闭包原样搬来，只把捕获的局部变量换成 `self.*`；
//! 这里只调本功能的 `ops` 并更新状态，不写布局。

use super::ops;
use super::state::HistoryState;
use crate::features::SessionHandle;
use crate::features::commit::commands::CommitCommands;
use crate::features::recovery::commands::RecoveryCommands;
use crate::filters::When;
use crate::shared::diff::DiffCommands;
use git::Repo;
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
    /// 选中第 `index` 个提交（原 `select_commit`）。
    pub(crate) fn select_commit(&self, index: usize) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            ops::select_commit(tab, index);
            self.session.tabs.set(list);
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

    /// 在提交变更树中选中文件并打开该文件的提交差异。
    pub(crate) fn select_tree_path(&self, id: String) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            if let Some(path) = id.strip_prefix("file:") {
                if let Some(index) = tab.files.iter().position(|file| file.path == path) {
                    ops::select_file(tab, index);
                }
            } else {
                ops::select_file_in_tree(tab, None);
            }
            self.session.tabs.set(list);
        }
    }

    /// 左侧分支、远端分支或 tag 按完整 ref 定位，stash 直接传 OID。
    pub(crate) fn show_reference_at(&self, tab_id: u64, reference: String) {
        let Some(path) = self
            .session
            .tabs
            .get_untracked()
            .iter()
            .find(|tab| tab.id == tab_id)
            .map(|tab| tab.path.clone())
        else {
            return;
        };
        let sha = match Repo::open(path.as_str()).and_then(|repo| repo.resolve_commit(&reference)) {
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
        let mut filter = (self.state.filter)();
        filter.reference = None;
        if !ops::select_reference_sha(tab, sha.as_str(), &filter) {
            self.notify_reference_not_found();
            return;
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
        self.state.reference.set(String::new());
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

    /// 按仓库标签的稳定 ID 设置对比区间。
    pub(crate) fn set_compare_refs_at(
        &self,
        tab_id: u64,
        base: Option<String>,
        target: Option<String>,
    ) {
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.iter_mut().find(|tab| tab.id == tab_id) {
            ops::set_compare_refs(tab, base, target);
            self.session.tabs.set(list);
            self.mode.set(crate::Mode::History);
        }
    }

    /// 按当前条件重新读一遍当前仓库（原 `reload`）。
    fn reload(&self) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get(current).cloned() {
            list[current] = ops::reload_with_filter(&tab, &(self.state.filter)());
            self.session.tabs.set(list);
        }
    }

    /// 应用筛选条件并收起浮层（原 `apply_filter`）。
    pub(crate) fn apply_filter(&self) {
        self.state.filter_open.set(false);
        self.reload();
    }

    /// 清掉全部筛选条件并重读（原 `clear_filter`）。
    pub(crate) fn clear_filter(&self) {
        self.state.query.set(String::new());
        self.state.reference.set(String::new());
        self.state.author.set(String::new());
        self.state.when.set(SharedString::from(When::All.id()));
        self.reload();
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
