//! 页面组装用的句柄集合：一次构造成员，按需 `clone()` 传给各入口。
//!
//! 主窗口版式（`app::workspace`）组装标题栏、侧栏、主体与浮层时需要所有功能的
//! 用例句柄；收成一个结构后，版式函数只有一个参数，新增功能只改这里与构造点。

use crate::features::SessionHandle;
use crate::features::agent::commands::AgentCommands;
use crate::features::changes::commands::ChangesCommands;
use crate::features::commit::commands::CommitCommands;
use crate::features::history::commands::HistoryCommands;
use crate::features::merge::commands::MergeCommands;
use crate::features::recovery::commands::RecoveryCommands;
use crate::features::refs::commands::RefsCommands;
use crate::features::search::commands::SearchCommands;
use crate::features::settings::commands::SettingsCommands;
use crate::features::sync::commands::SyncCommands;
use crate::features::welcome::commands::WelcomeCommands;
use crate::shared::diff::DiffCommands;
use nsh_core::prelude::*;
use std::sync::Arc;

/// 各功能的用例句柄与当前语言；`bootstrap` 构造一次，页面按需克隆。
#[derive(Clone)]
pub(crate) struct AppHandles {
    pub(crate) session: SessionHandle,
    pub(crate) history: HistoryCommands,
    pub(crate) changes: ChangesCommands,
    pub(crate) commit: CommitCommands,
    pub(crate) refs: RefsCommands,
    pub(crate) sync: SyncCommands,
    pub(crate) merge: MergeCommands,
    pub(crate) agent: AgentCommands,
    pub(crate) settings: SettingsCommands,
    pub(crate) search: SearchCommands,
    pub(crate) welcome: WelcomeCommands,
    // `recovery` 与 `diff` 没有独立的页面入口：前者挂在 refs / history 的浮层上，
    // 后者由 history 与 settings 各自持有；这里仍留一份，组装根将来要直接取用时
    // 不必再改各功能的构造代码。
    #[allow(dead_code)]
    pub(crate) recovery: RecoveryCommands,
    #[allow(dead_code)]
    pub(crate) diff: DiffCommands,
    pub(crate) language: Signal<SharedString>,
}

impl AppHandles {
    /// Fork workspace state for an auxiliary one-repo worktree window.
    pub(crate) fn for_worktree(&self) -> Self {
        let tabs = signal(Vec::<crate::app::session::Tab>::new());
        let active = signal(0);
        let filter: Arc<dyn Fn() -> git::LogFilter> = {
            let tabs = tabs;
            let active = active;
            Arc::new(move || {
                return tabs
                    .get_untracked()
                    .get(active.get_untracked())
                    .map(|tab| tab.filter.clone())
                    .unwrap_or_default();
            })
        };
        let session = crate::features::SessionHandle {
            tabs,
            active,
            task_owner: self.session.task_owner,
            welcome_open: self.session.welcome_open,
            settings_open: self.session.settings_open,
            recent: self.session.recent,
            store_dir: self.session.store_dir.clone(),
            filter: filter.clone(),
            notice: signal(None),
            language: self.language,
            auxiliary: true,
            persist_session: false,
            window: signal(None),
            welcome_window: signal(None),
            settings_window: signal(None),
        };

        let mut handles = self.clone();
        handles.session = session.clone();
        handles.diff.session = session.clone();
        handles.recovery.session = session.clone();
        handles.merge.session = session.clone();
        handles.changes.session = session.clone();
        handles.changes.state.changelist_open = signal(false);
        handles.commit.session = session.clone();
        handles.commit.state = crate::features::commit::state::CommitState {
            summary: signal(String::new()),
            description: signal(String::new()),
            rewrite: signal(None),
        };
        handles.agent.session = session.clone();
        handles.agent.commit_summary = handles.commit.state.summary;
        handles.agent.state.plan = signal(None);
        handles.history.session = session.clone();
        handles.history.diff = handles.diff.clone();
        handles.history.commit = handles.commit.clone();
        handles.history.recovery = handles.recovery.clone();
        handles.history.mode = signal(crate::Mode::History);
        let query = signal(handles.history.state.query.get_untracked());
        let draft = signal(handles.history.state.draft.get_untracked());
        handles.history.state = crate::features::history::state::HistoryState {
            query,
            draft,
            filter_open: signal(false),
            filter_position: signal((0.0, 0.0)),
            filter_menu: signal(None),
            filter_tab_id: signal(None),
            quick_filter_baseline: signal(None),
            file_tree_expanded: signal(None),
            compare_mode: signal(false),
            compare_refs_open: signal(false),
            compare_refs_query: signal(String::new()),
            compare_refs_draft: signal(crate::features::history::state::CompareRefsDraft::default()),
            fast_selection: signal(None),
            commit_info_loading: signal(handles.history.state.commit_info_loading.get_untracked()),
            commit_load_queue: std::sync::Arc::default(),
            table_ref: component_ref(),
            scroll_request: signal(None),
            filter: filter.clone(),
            author_column_width: signal(handles.history.state.author_column_width.get_untracked()),
            date_column_width: signal(handles.history.state.date_column_width.get_untracked()),
        };
        handles.sync.session = session.clone();
        handles.sync.state.dialog = signal(None);
        handles.refs.session = session.clone();
        handles.refs.history = handles.history.clone();
        handles.refs.recovery = handles.recovery.clone();
        handles.refs.sync = handles.sync.clone();
        handles.refs.state.reflog_query = signal(String::new());
        handles.refs.open_worktree_window = None;
        handles.search.session = session.clone();
        handles.search.history = handles.history.clone();
        handles.search.commit = handles.commit.clone();
        handles.search.settings = handles.settings.clone();
        handles.search.sync = handles.sync.clone();
        handles.search.state = crate::features::search::state::SearchState {
            palette_open: signal(false),
            palette_selected: signal(0),
            palette_query: signal(String::new()),
            mode: handles.history.mode,
            form: signal(None),
        };
        handles.search.sidebar_open = signal(true);
        handles.search.right_panel_open =
            signal(handles.history.mode.get_untracked() == crate::Mode::Changes);
        handles.settings.session = session;
        handles.settings.diff = handles.diff.clone();
        handles.search.settings = handles.settings.clone();
        handles.refs.search = handles.search.clone();
        handles.language = self.language;
        return handles;
    }
}
