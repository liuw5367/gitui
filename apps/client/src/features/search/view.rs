//! 命令面板（设计板 20）：`⌘P` 打开，结果来自打开中、最近、扫描仓库和命令。
//!
//! 搜索同时匹配命令文案与仓库路径；↑↓ 在整张结果表里移动，↵ 执行命中的那一行。
//! 渲染与键盘导航共用 [`rows`]，避免「高亮一行、执行另一行」。

use super::commands::SearchCommands;
use super::state::SearchState;
use crate::design::{self, metrics};
use crate::i18n;
use nsh_core::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;

/// 面板宽度与最大高度（设计板 `.palette`）。
const WIDTH: f32 = 520.0;
pub(crate) const MAX_ROWS: usize = 9;
const DEFAULT_COMMAND_IDS: [&str; 5] = ["open", "refresh", "history", "changes", "settings"];

/// 面板里的命令（顺序即显示顺序）。每条的 `id` 由 `SearchCommands::run_command` 处理；
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
        return Command { id, key, shortcut };
    }
}

/// 面板结果里的一行。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Row {
    /// 一个仓库；`open` 表示已经在标签页里打开，来源用于分组显示。
    Repo {
        path: String,
        open: bool,
        source: RepoSource,
    },
    /// 一条命令。
    Command(&'static str),
}

/// 仓库命令面板结果的来源。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RepoSource {
    Open,
    Recent,
    Scanned,
}

/// 空查询显示最近仓库和常用命令；输入后搜索打开中、最近、扫描仓库与全部命令。
pub(crate) fn rows(
    query: &str,
    language: &str,
    open_repos: &[String],
    recent: &[String],
    scanned: &[String],
) -> Vec<Row> {
    let needle = query.trim().to_lowercase();
    let matches = |text: &str| needle.is_empty() || text.to_lowercase().contains(&needle);
    let mut rows: Vec<Row> = Vec::new();
    let open_paths: HashSet<PathBuf> = open_repos
        .iter()
        .map(|path| repository_identity(path))
        .collect();
    let mut seen = HashSet::new();

    if needle.is_empty() {
        for path in recent {
            let identity = repository_identity(path);
            if !seen.insert(identity.clone()) {
                continue;
            }
            rows.push(Row::Repo {
                path: path.clone(),
                open: open_paths.contains(&identity),
                source: RepoSource::Recent,
            });
            if rows.len() == 4 {
                break;
            }
        }
        for path in open_repos {
            if rows.len() == 4 {
                break;
            }
            let identity = repository_identity(path);
            if seen.insert(identity) {
                rows.push(Row::Repo {
                    path: path.clone(),
                    open: true,
                    source: RepoSource::Recent,
                });
            }
        }
        rows.extend(DEFAULT_COMMAND_IDS.map(Row::Command));
        rows.truncate(MAX_ROWS);
        return rows;
    }

    for path in open_repos {
        if rows.len() == MAX_ROWS {
            break;
        }
        let identity = repository_identity(path);
        if matches(path) && seen.insert(identity) {
            rows.push(Row::Repo {
                path: path.clone(),
                open: true,
                source: RepoSource::Open,
            });
        }
    }
    for path in recent {
        if rows.len() == MAX_ROWS {
            break;
        }
        let identity = repository_identity(path);
        if matches(path) && seen.insert(identity) {
            rows.push(Row::Repo {
                path: path.clone(),
                open: false,
                source: RepoSource::Recent,
            });
        }
    }
    for path in scanned {
        if rows.len() == MAX_ROWS {
            break;
        }
        if matches(path) {
            let identity = repository_identity(path);
            if seen.insert(identity) {
                rows.push(Row::Repo {
                    path: path.clone(),
                    open: false,
                    source: RepoSource::Scanned,
                });
            }
        }
    }
    rows.extend(
        commands()
            .into_iter()
            .filter(|command| {
                let label = i18n::t(language, command.key).to_lowercase();
                label.contains(&needle) || command.id.contains(&needle)
            })
            .map(|command| Row::Command(command.id)),
    );
    rows.truncate(MAX_ROWS);
    return rows;
}

fn repository_identity(path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    return path.canonicalize().unwrap_or(path);
}

/// 面板宽度内的结果表。面板按窗口居中，点遮罩关闭。
///
/// `open_repos` / `recent` 由调用方从会话里取，保持本函数不直接读取标签页之外的
/// 状态（渲染与键盘导航因此能用同一份结果）。
pub(crate) fn palette(
    state: &SearchState,
    cmd: &SearchCommands,
    open_repos: Vec<String>,
    recent: Vec<String>,
) -> impl IntoElement {
    let language = cmd.language;
    let query = state.palette_query;
    let selected = state.palette_selected;
    let language_now = language.get_untracked();
    let language_for_empty = language_now.clone();
    let run: std::sync::Arc<dyn Fn(&'static str)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |id: &'static str| cmd.run_command(id))
    };
    let open_repo: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |path: String| cmd.session.open_path(path))
    };
    let open_signal = state.palette_open;
    let scan_selected = cmd.settings.state.scan_selected;
    let list = dynamic(move || -> AnyElement {
        let language_now = language_now.clone();
        let language_for_empty = language_for_empty.clone();
        let result = rows(
            query.get().as_str(),
            language_now.as_str(),
            &open_repos,
            &recent,
            &scan_selected.get(),
        );
        let highlighted = selected.get().min(result.len().saturating_sub(1));
        let mut children: Vec<AnyElement> = Vec::new();
        let mut last_group = "";
        for (index, row) in result.iter().enumerate() {
            let group = match row {
                Row::Repo {
                    source: RepoSource::Open,
                    ..
                } => "palette.groupOpen",
                Row::Repo {
                    source: RepoSource::Recent,
                    ..
                } => "palette.groupRecent",
                Row::Repo {
                    source: RepoSource::Scanned,
                    ..
                } => "palette.groupScanned",
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
                            .text_size(px(crate::ui::SECONDARY))
                            .line_height(px(crate::ui::SECONDARY_LINE))
                            .font_weight(FontWeight(600))
                            .text_color(move || design::text3()),
                        )
                        .into_element(),
                );
                last_group = group;
            }
            let is_highlighted = index == highlighted;
            let element: AnyElement = match row {
                Row::Repo { path, open, source } => {
                    let is_open = *open;
                    let icon = match source {
                        RepoSource::Open => "folder",
                        RepoSource::Recent => {
                            if is_open {
                                "folder"
                            } else {
                                "clock"
                            }
                        }
                        RepoSource::Scanned => "search",
                    };
                    let path = path.clone();
                    let label = path.rsplit('/').next().unwrap_or(path.as_str()).to_string();
                    let hint = if is_open {
                        i18n::t(language_now.as_str(), "palette.switch")
                    } else {
                        path.clone()
                    };
                    let run = open_repo.clone();
                    let open_signal = open_signal;
                    crate::ui::row::row(
                        move || label.clone(),
                        None,
                        Some(
                            crate::ui::icon::icon_view(
                                icon,
                                crate::ui::icon::IconSize::Button,
                                design::text3,
                            )
                            .into_element(),
                        ),
                        vec![
                            text(hint.clone())
                                .flex_shrink_0()
                                .text_size(px(crate::ui::SECONDARY))
                                .text_color(move || design::text3())
                                .into_element(),
                        ],
                        is_highlighted,
                        false,
                        move || {
                            run(path.clone());
                            open_signal.set(false);
                        },
                    )
                    .into_element()
                }
                Row::Command(id) => {
                    let command = commands()
                        .into_iter()
                        .find(|command| command.id == *id)
                        .expect("命中结果里的命令一定存在");
                    let run = run.clone();
                    let id = command.id;
                    let label = i18n::t(language_now.as_str(), command.key);
                    let shortcut = command.shortcut;
                    let open_signal = open_signal;
                    crate::ui::row::row(
                        move || label.clone(),
                        None,
                        Some(
                            crate::ui::icon::icon_view(
                                "search",
                                crate::ui::icon::IconSize::Button,
                                design::text3,
                            )
                            .into_element(),
                        ),
                        vec![
                            text(shortcut)
                                .text_size(px(crate::ui::SECONDARY))
                                .text_color(move || design::text3())
                                .into_element(),
                        ],
                        is_highlighted,
                        false,
                        move || {
                            run(id);
                            open_signal.set(false);
                        },
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
                            .text_size(px(crate::ui::SECONDARY))
                            .text_color(move || design::text3()),
                    )
                    .into_element(),
            );
        }
        return view().w_full().child(children).into_element();
    });

    // 遮罩与「点外面关闭」交给 ui 层（已迁移页面不直接写按压回调）。内层仍按
    // 原版排布：铺满、居中、面板上移 120。
    let dismiss = {
        let cmd = cmd.clone();
        move || cmd.state.palette_open.set(false)
    };
    return crate::ui::dialog::modal_open(
        move || i18n::t(language.get().as_str(), "palette.placeholder"),
        100,
        view().w_full().h_full().items_center().child(
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
                        .child(crate::ui::icon::icon_view(
                            "search",
                            crate::ui::icon::IconSize::Button,
                            design::text3,
                        ))
                        .child(crate::ui::text_input::text_input(
                            query,
                            move || i18n::t(language.get().as_str(), "palette.placeholder"),
                            false,
                            move |text: &str| query.set(text.to_string()),
                        )),
                )
                .child(list),
        ),
        dismiss,
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
    fn empty_query_shows_recent_repositories_and_common_commands() {
        let open = vec!["/work/alpha".to_string(), "/work/beta".to_string()];
        let recent = vec!["/work/beta".to_string(), "/old/gamma".to_string()];
        let scanned = vec!["/scan/delta".to_string()];
        let all = rows("", "en", &open, &recent, &scanned);
        // 最近列表保留时间顺序；当前打开状态仍标在对应历史行上。
        assert_eq!(
            all[0],
            Row::Repo {
                path: "/work/beta".to_string(),
                open: true,
                source: RepoSource::Recent,
            }
        );
        assert_eq!(
            all[1],
            Row::Repo {
                path: "/old/gamma".to_string(),
                open: false,
                source: RepoSource::Recent,
            }
        );
        assert_eq!(
            all[2],
            Row::Repo {
                path: "/work/alpha".to_string(),
                open: true,
                source: RepoSource::Recent,
            }
        );
        assert_eq!(all[3], Row::Command("open"));
        assert_eq!(all[7], Row::Command("settings"));
        assert!(
            !all.iter()
                .any(|row| matches!(row, Row::Repo { path, .. } if path == "/scan/delta"))
        );
    }

    #[test]
    fn query_matches_scanned_repositories_and_deduplicates_history() {
        let open = vec!["/work/alpha".to_string()];
        let recent = vec!["/work/alpha".to_string(), "/old/gamma".to_string()];
        let scanned = vec!["/work/alpha".to_string(), "/scan/delta".to_string()];
        // 查询按仓库路径与命令文案同时过滤。
        let only_gamma = rows("gamma", "en", &open, &recent, &scanned);
        assert_eq!(only_gamma.len(), 1);
        assert_eq!(
            only_gamma[0],
            Row::Repo {
                path: "/old/gamma".to_string(),
                open: false,
                source: RepoSource::Recent,
            }
        );

        let only_scanned = rows("delta", "en", &open, &recent, &scanned);
        assert_eq!(
            only_scanned,
            vec![Row::Repo {
                path: "/scan/delta".to_string(),
                open: false,
                source: RepoSource::Scanned,
            }]
        );

        let duplicate = rows("alpha", "en", &open, &recent, &scanned);
        assert_eq!(
            duplicate,
            vec![Row::Repo {
                path: "/work/alpha".to_string(),
                open: true,
                source: RepoSource::Open,
            }]
        );
    }

    #[test]
    fn empty_query_keeps_four_repositories_and_five_commands() {
        let recent: Vec<String> = (0..8).map(|index| format!("/work/repo-{index}")).collect();
        let all = rows("", "en", &[], &recent, &[]);
        assert_eq!(all.len(), MAX_ROWS);
        assert_eq!(
            all.iter()
                .filter(|row| matches!(row, Row::Repo { .. }))
                .count(),
            4
        );
        assert_eq!(
            all.iter()
                .filter(|row| matches!(row, Row::Command(_)))
                .count(),
            5
        );
    }
}
