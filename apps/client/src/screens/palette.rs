//! 命令面板（设计板 20）：`⌘P` 打开，结果分三组——打开中的仓库、最近仓库、命令。
//!
//! 搜索同时匹配命令文案与仓库路径；↑↓ 在整张结果表里移动，↵ 执行命中的那一行。
//! 渲染与键盘导航共用 [`rows`]，避免「高亮一行、执行另一行」。

use crate::design::{self, metrics};
use crate::i18n;
use crate::widgets;
use nsh_core::prelude::*;

/// 面板宽度与最大高度（设计板 `.palette`）。
const WIDTH: f32 = 520.0;
pub(crate) const MAX_ROWS: usize = 9;

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

/// 面板结果里的一行。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Row {
    /// 一个仓库（`open` 表示已经在标签页里打开）。
    Repo { path: String, open: bool },
    /// 一条命令。
    Command(&'static str),
}

/// 面板的三组结果，按「打开中 → 最近 → 命令」排列，已按查询过滤并截断。
pub(crate) fn rows(
    query: &str,
    language: &str,
    open_repos: &[String],
    recent: &[String],
) -> Vec<Row> {
    let needle = query.to_lowercase();
    let matches = |text: &str| needle.is_empty() || text.to_lowercase().contains(&needle);
    let mut rows: Vec<Row> = Vec::new();
    for path in open_repos {
        if matches(path) {
            rows.push(Row::Repo {
                path: path.clone(),
                open: true,
            });
        }
    }
    for path in recent {
        if open_repos.iter().any(|open| open == path) || !matches(path) {
            continue;
        }
        rows.push(Row::Repo {
            path: path.clone(),
            open: false,
        });
    }
    let mut commands: Vec<Row> = commands()
        .into_iter()
        .filter(|command| {
            let label = i18n::t(language, command.key).to_lowercase();
            label.contains(&needle) || command.id.contains(&needle)
        })
        .map(|command| Row::Command(command.id))
        .collect();
    rows.append(&mut commands);
    rows.truncate(MAX_ROWS);
    return rows;
}

/// 面板宽度内的结果表。`open_repo` 打开一个仓库行（已打开的切过去）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn palette(
    language: Signal<SharedString>,
    query: Signal<String>,
    open_repos: Vec<String>,
    recent: Vec<String>,
    on_run: std::sync::Arc<dyn Fn(&'static str)>,
    on_open_repo: std::sync::Arc<dyn Fn(String)>,
    open_signal: Signal<bool>,
    selected: Signal<usize>,
) -> impl IntoElement {
    let language_now = language.get_untracked();
    let language_for_empty = language_now.clone();
    let result = rows(
        query.get_untracked().as_str(),
        language_now.as_str(),
        &open_repos,
        &recent,
    );
    let highlighted = selected.get().min(result.len().saturating_sub(1));
    let mut children: Vec<AnyElement> = Vec::new();
    let mut last_group = "";
    for (index, row) in result.iter().enumerate() {
        let group = match row {
            Row::Repo { open: true, .. } => "palette.groupOpen",
            Row::Repo { open: false, .. } => "palette.groupRecent",
            Row::Command(_) => "palette.groupCommands",
        };
        if group != last_group {
            children.push(
                view()
                    .w_full()
                    .h(px(20.0))
                    .px(px(10.0))
                    .items_center()
                    .child(
                        text({
                            let language_now = language_now.clone();
                            move || i18n::t(language_now.as_str(), group)
                        })
                            .text_size(px(9.5))
                            .line_height(px(12.0))
                            .font_weight(FontWeight(600))
                            .text_color(move || design::text3()),
                    )
                    .into_element(),
            );
            last_group = group;
        }
        let is_highlighted = index == highlighted;
        let element: AnyElement = match row {
            Row::Repo { path, open } => {
                let is_open = *open;
                let path = path.clone();
                let label = path.rsplit('/').next().unwrap_or(path.as_str()).to_string();
                let hint = if is_open {
                    i18n::t(language_now.as_str(), "palette.switch")
                } else {
                    path.clone()
                };
                let run = on_open_repo.clone();
                let open_signal = open_signal;
                view()
                    .w_full()
                    .h(px(30.0))
                    .px(px(10.0))
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .rounded(px(5.0))
                    .bg(move || {
                        if is_highlighted {
                            design::selected()
                        } else {
                            AdaptiveColor::TRANSPARENT
                        }
                    })
                    .hover(|style| style.bg(move || design::hover()))
                    .on_press(move || {
                        run(path.clone());
                        open_signal.set(false);
                    })
                    .child(widgets::icon(
                        if is_open { "folder" } else { "clock" },
                        12.0,
                        design::text3,
                    ))
                    .child(
                        text(label)
                            .flex_1()
                            .text_size(px(12.5))
                            .text_color(move || design::text())
                            .text_overflow(nsh_core::EllipsizeMode::Tail),
                    )
                    .child(
                        text(hint)
                            .flex_shrink_0()
                            .text_size(px(10.0))
                            .text_color(move || design::text3()),
                    )
                    .into_element()
            }
            Row::Command(id) => {
                let command = commands()
                    .into_iter()
                    .find(|command| command.id == *id)
                    .expect("命中结果里的命令一定存在");
                let run = on_run.clone();
                let id = command.id;
                let label = i18n::t(language_now.as_str(), command.key);
                let shortcut = command.shortcut;
                let open_signal = open_signal;
                view()
                    .w_full()
                    .h(px(30.0))
                    .px(px(10.0))
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .rounded(px(5.0))
                    .bg(move || {
                        if is_highlighted {
                            design::selected()
                        } else {
                            AdaptiveColor::TRANSPARENT
                        }
                    })
                    .hover(|style| style.bg(move || design::hover()))
                    .on_press(move || {
                        run(id);
                        open_signal.set(false);
                    })
                    .child(widgets::icon("search", 12.0, design::text3))
                    .child(
                        text(label)
                            .flex_1()
                            .text_size(px(12.5))
                            .text_color(move || design::text()),
                    )
                    .child(
                        text(shortcut)
                            .text_size(px(10.0))
                            .text_color(move || design::text3()),
                    )
                    .into_element()
            }
        };
        children.push(element);
    }
    if children.is_empty() {
        children.push(
            view()
                .w_full()
                .h(px(30.0))
                .px(px(10.0))
                .items_center()
                .child(
                    text(move || i18n::t(language_for_empty.as_str(), "palette.empty"))
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
        .on_press(move || open_signal.set(false))
        .child(
            view()
                .w(px(WIDTH))
                .mt(px(-120.0))
                .p(px(6.0))
                .flex_col()
                .gap(px(3.0))
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
                        .child(widgets::icon("search", 12.0, design::text3))
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
                .child(children),
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

    #[test]
    fn rows_put_open_repos_first_then_recent_then_commands() {
        let open = vec!["/work/alpha".to_string(), "/work/beta".to_string()];
        let recent = vec!["/work/beta".to_string(), "/old/gamma".to_string()];
        let all = rows("", "en", &open, &recent);
        // 已打开的排在最前，已打开的仓库不会在「最近」里重复出现。
        assert_eq!(
            all[0],
            Row::Repo {
                path: "/work/alpha".to_string(),
                open: true
            }
        );
        assert_eq!(
            all[1],
            Row::Repo {
                path: "/work/beta".to_string(),
                open: true
            }
        );
        assert_eq!(
            all[2],
            Row::Repo {
                path: "/old/gamma".to_string(),
                open: false
            }
        );
        assert!(matches!(all[3], Row::Command(_)));
        // 查询按仓库路径与命令文案同时过滤。
        let only_gamma = rows("gamma", "en", &open, &recent);
        assert_eq!(only_gamma.len(), 1);
        assert_eq!(
            only_gamma[0],
            Row::Repo {
                path: "/old/gamma".to_string(),
                open: false
            }
        );
        // 截断到上限，避免面板长到屏幕外。
        assert!(rows("", "en", &open, &recent).len() <= MAX_ROWS);
    }
}
