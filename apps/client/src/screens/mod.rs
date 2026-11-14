//! 屏幕：欢迎页、工作区与设置。
//!
//! `lib.rs` 只负责状态与动作，屏幕函数只负责布局与呈现。欢迎页是**独立窗口**
//! （`id = "welcome"`），标签页工作区与设置在同一个窗口的 `main` 里。

mod changes_view;
mod conflict_view;
mod form_dialog;
mod hook_sheet;
mod palette;
mod refs_view;
mod settings;
mod sync_dialog;
mod welcome;
mod workspace;

pub(crate) use changes_view::changes_view;
pub(crate) use conflict_view::conflict_view;
pub(crate) use form_dialog::{form_dialog, FormKind, FormState};
pub(crate) use hook_sheet::hook_sheet;
pub(crate) use palette::palette;
pub(crate) use refs_view::{refs_view, RefsTab};
pub(crate) use settings::{settings, Section as SettingsSection};
pub(crate) use sync_dialog::{sync_dialog, SyncDialogState};
pub(crate) use welcome::welcome;
pub(crate) use workspace::workspace;

/// 命令面板的命令列表（供根组件与测试共用）。
pub(crate) fn palette_commands() -> Vec<palette::Command> {
    return palette::commands();
}

/// 当前查询命中的命令 id（顺序即显示顺序，已被上限截断）。
///
/// 根组件的键盘导航与面板渲染用同一份判断，避免「高亮一行、执行另一行」。
pub(crate) fn palette_ids(query: &str, language: &str) -> Vec<&'static str> {
    let needle = query.to_lowercase();
    return palette::commands()
        .into_iter()
        .filter(|command| {
            if needle.is_empty() {
                return true;
            }
            let label = crate::i18n::t(language, command.key).to_lowercase();
            return label.contains(&needle) || command.id.contains(&needle);
        })
        .take(palette::MAX_ROWS)
        .map(|command| command.id)
        .collect();
}
