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
