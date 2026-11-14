//! 命令面板（设计板 20）：`⌘P` 打开，搜索过滤，回车或点击执行。
//!
//! 面板浮在整屏之上；命令来自应用动作，未接入的动作不出现在列表里（避免选中后
//! 什么也不发生）。

use crate::design::{self, metrics};
use crate::i18n;
use nsh_core::prelude::*;

/// 面板宽度与最大高度（设计板 `.palette`）。
const WIDTH: f32 = 520.0;
const MAX_ROWS: usize = 9;

/// 面板里的命令（顺序即显示顺序）。每条的 `id` 由 `Actions::run_command` 处理；
/// 未接入的动作不列在这里。
pub(crate) const COMMANDS: [(&str, &str, &str); 15] = [
    ("open", "command.openRepo", "⌘O"),
    ("openPalette", "command.openPalette", "⌘P"),
    ("closeTab", "command.closeTab", "⌘W"),
    ("refresh", "command.refresh", "⌘R"),
    ("history", "command.history", ""),
    ("changes", "command.changes", ""),
    ("inspector", "command.inspector", ""),
    ("compare", "command.compare", ""),
    ("fetch", "command.fetch", ""),
    ("pull", "command.pull", ""),
    ("push", "command.push", ""),
    ("repoManager", "command.repoManager", ""),
    ("toggleLanguage", "command.toggleLanguage", ""),
    ("toggleTheme", "command.toggleTheme", "⇧⌘A"),
    ("settings", "command.settings", "⌘,"),
];

/// 命令列表：`COMMANDS` 的 [`Command`] 视图。
pub(crate) fn commands() -> Vec<Command> {
    return COMMANDS
        .iter()
        .map(|(id, key, shortcut)| Command::new(id, key, shortcut))
        .collect();
}

/// 一条可执行命令。
#[derive(Clone)]
pub(crate) struct Command {
    /// 稳定的标识，用于搜索与执行。
    pub id: &'static str,
    /// 文案键。
    pub key: &'static str,
    /// 右侧显示的快捷键提示。
    pub shortcut: &'static str,
}

impl Command {
    pub fn new(id: &'static str, key: &'static str, shortcut: &'static str) -> Command {
        return Command {
            id,
            key,
            shortcut,
        };
    }
}

pub(crate) fn palette(
    language: Signal<SharedString>,
    query: Signal<String>,
    commands: Vec<Command>,
    on_run: std::sync::Arc<dyn Fn(&'static str)>,
    open: Signal<bool>,
) -> impl IntoElement {
    let filtered: Vec<Command> = commands
        .iter()
        .filter(|command| {
            let needle = query.get().to_lowercase();
            if needle.is_empty() {
                return true;
            }
            let label = i18n::t(language.get().as_str(), command.key).to_lowercase();
            return label.contains(&needle) || command.id.contains(&needle);
        })
        .take(MAX_ROWS)
        .cloned()
        .collect();
    let mut rows: Vec<AnyElement> = Vec::new();
    for (index, command) in filtered.iter().enumerate() {
        let id = command.id;
        let run = on_run.clone();
        let key = command.key;
        let shortcut = command.shortcut;
        rows.push(
            view()
                .w_full()
                .h(px(30.0))
                .px(px(10.0))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .rounded(px(5.0))
                .bg(move || {
                    if index == 0 {
                        design::selected()
                    } else {
                        AdaptiveColor::TRANSPARENT
                    }
                })
                .hover(|style| style.bg(move || design::hover()))
                .on_press(move || {
                    run(id);
                    open.set(false);
                })
                .child(
                    text(move || i18n::t(language.get().as_str(), key))
                        .flex_1()
                        .text_size(px(12.5))
                        .text_color(move || design::text()),
                )
                .child(
                    text(shortcut)
                        .text_size(px(10.0))
                        .text_color(move || design::text3()),
                )
                .into_element(),
        );
    }
    if rows.is_empty() {
        rows.push(
            view()
                .w_full()
                .h(px(30.0))
                .px(px(10.0))
                .items_center()
                .child(
                    text(move || i18n::t(language.get().as_str(), "palette.empty"))
                        .text_size(px(11.0))
                        .text_color(move || design::text3()),
                )
                .into_element(),
        );
    }

    return view()
        .absolute()
        .inset(px(0.0))
        .z_index(100)
        .items_center()
        .bg(move || design::token("overlay"))
        .on_press(move || open.set(false))
        .child(
            view()
                .w(px(WIDTH))
                .mt(px(-120.0))
                .p(px(6.0))
                .flex_col()
                .gap(px(4.0))
                .rounded(px(metrics::RADIUS_PANEL))
                .border_1()
                .border_color(move || design::line())
                .bg(move || design::token("popover"))
                .child(
                    view()
                        .w_full()
                        .h(px(30.0))
                        .px(px(8.0))
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child(crate::widgets::icon("search", 12.0, design::text3))
                        .child(
                            nsh_base::input()
                                .value(query)
                                .on_change(move |text: &str| query.set(text.to_string()))
                                .placeholder(move || {
                                    i18n::t(language.get().as_str(), "palette.placeholder")
                                })
                                .text_size(px(12.5))
                                .flex_1(),
                        ),
                )
                .child(rows),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_command_has_a_label_key_and_unique_id() {
        let mut ids: Vec<&str> = Vec::new();
        for command in commands() {
            assert!(!command.key.is_empty(), "{} 缺少文案键", command.id);
            assert!(!ids.contains(&command.id), "命令 id 重复：{}", command.id);
            ids.push(command.id);
        }
        assert!(ids.contains(&"toggleTheme"));
    }
}
