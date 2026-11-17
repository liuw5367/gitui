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

    /// 选中提交里的第 `index` 个文件（原 `select_file`）。
    pub(crate) fn select_file(&self, index: usize) {
        let current = self.session.active.get_untracked();
        let mut list = self.session.tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            ops::select_file(tab, index);
            self.session.tabs.set(list);
        }
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
