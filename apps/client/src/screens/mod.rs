//! 屏幕：欢迎页、工作区与设置。
//!
//! `lib.rs` 只负责状态与动作，屏幕函数只负责布局与呈现。欢迎页是**独立窗口**
//! （`id = "welcome"`），标签页工作区与设置在同一个窗口的 `main` 里。

mod blame_view;
mod conflict_view;
mod form_dialog;
mod hook_sheet;
mod palette;
mod refs_view;
mod rewrite_dialog;
mod settings;
mod sync_dialog;
mod welcome;
mod workspace;

pub(crate) use blame_view::blame_view;
pub(crate) use conflict_view::conflict_view;
pub(crate) use form_dialog::{form_dialog, FormKind, FormState};
pub(crate) use hook_sheet::hook_sheet;
pub(crate) use palette::{palette, Row as PaletteRow};
pub(crate) use refs_view::{refs_view, RefsTab};
pub(crate) use rewrite_dialog::{rewrite_dialog, RewriteKind, RewriteState};
pub(crate) use settings::{settings, Section as SettingsSection, ShortcutUi};
pub(crate) use sync_dialog::{sync_dialog, SyncDialogState};
pub(crate) use welcome::{welcome, CloneOptions, InitFiles};
pub(crate) use workspace::workspace;

/// 当前查询命中的结果行（顺序即显示顺序，已被上限截断）。
///
/// 根组件的键盘导航与面板渲染用同一份判断，避免「高亮一行、执行另一行」。
pub(crate) fn palette_rows(
    query: &str,
    language: &str,
    open_repos: &[String],
    recent: &[String],
) -> Vec<palette::Row> {
    return palette::rows(query, language, open_repos, recent);
}
