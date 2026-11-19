//! 命令面板（设计板 20）：`⌘P` 打开，结果来自打开中、最近、扫描仓库和命令。
//!
//! 搜索同时匹配命令文案与仓库路径；↑↓ 在整张结果表里移动，↵ 执行命中的那一行。
//! 渲染与键盘导航共用 [`rows`]，避免「高亮一行、执行另一行」。

use super::commands::SearchCommands;
use super::state::SearchState;
use crate::design;
use crate::i18n;
use crate::ui;
use nsh_core::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;

/// 面板宽度与高度（设计板 `.palette`）。
const WIDTH: f32 = 620.0;
const HEIGHT: f32 = 512.0;
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum RepositoryIdentity {
    FilesystemObject { device: u64, inode: u64 },
    Path(PathBuf),
}

/// 空查询先显示常用命令，再显示最近仓库；输入后仍保持命令在仓库之前。
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
    let open_paths: HashSet<RepositoryIdentity> = open_repos
        .iter()
        .map(|path| repository_identity(path))
        .collect();
    let mut seen = HashSet::new();

    if needle.is_empty() {
        rows.extend(DEFAULT_COMMAND_IDS.map(Row::Command));
        let mut repositories = Vec::new();
        for path in recent {
            let identity = repository_identity(path);
            if !seen.insert(identity.clone()) {
                continue;
            }
            repositories.push(Row::Repo {
                path: path.clone(),
                open: open_paths.contains(&identity),
                source: RepoSource::Recent,
            });
            if repositories.len() == 4 {
                break;
            }
        }
        for path in open_repos {
            if repositories.len() == 4 {
                break;
            }
            let identity = repository_identity(path);
            if seen.insert(identity) {
                repositories.push(Row::Repo {
                    path: path.clone(),
                    open: true,
                    source: RepoSource::Recent,
                });
            }
        }
        rows.extend(repositories);
        return rows;
    }

    rows.extend(
        commands()
            .into_iter()
            .filter(|command| {
                let label = i18n::t(language, command.key).to_lowercase();
                let id = command.id.to_lowercase();
                label.contains(&needle) || id.contains(&needle)
            })
            .map(|command| Row::Command(command.id)),
    );
    for path in open_repos {
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
    return rows;
}

fn repository_identity(path: &str) -> RepositoryIdentity {
    let path = PathBuf::from(path);
    #[cfg(unix)]
    if let Ok(metadata) = std::fs::metadata(&path) {
        use std::os::unix::fs::MetadataExt;
        if metadata.ino() != 0 {
            return RepositoryIdentity::FilesystemObject {
                device: metadata.dev(),
                inode: metadata.ino(),
            };
        }
    }
    return RepositoryIdentity::Path(path.canonicalize().unwrap_or(path));
}

/// 面板宽度内的结果表。面板按画板位置出现，点击遮罩不关闭。
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
    on_key_with(
        "escape",
        KeyOptions {
            when: nsh_core::keymap::When::Always,
            repeat: false,
            ..Default::default()
        },
        move |_| open_signal.set(false),
    );
    let move_down = cmd.clone();
    on_key_with(
        "down",
        KeyOptions {
            when: nsh_core::keymap::When::Always,
            repeat: true,
            ..Default::default()
        },
        move |_| move_down.move_selection(true),
    );
    let move_up = cmd.clone();
    on_key_with(
        "up",
        KeyOptions {
            when: nsh_core::keymap::When::Always,
            repeat: true,
            ..Default::default()
        },
        move |_| move_up.move_selection(false),
    );
    let activate = cmd.clone();
    on_key_with(
        "enter",
        KeyOptions {
            when: nsh_core::keymap::When::Always,
            repeat: false,
            ..Default::default()
        },
        move |_| activate.activate_selected(),
    );
    let scan_selected = cmd.settings.state.scan_selected;
    let results_scroll = element_ref();
    let list = scroll()
        .w_full()
        .flex_1()
        .min_h_0()
        .element_ref(&results_scroll)
        .child(dynamic(move || -> AnyElement {
            let language_now = language_now.clone();
            let language_for_empty = language_for_empty.clone();
            let result = rows(
                query.get().as_str(),
                language_now.as_str(),
                &open_repos,
                &recent,
                &scan_selected.get(),
            );
            let mut children: Vec<AnyElement> = Vec::new();
            let mut row_refs = Vec::with_capacity(result.len());
            let mut last_group = "";
            for (index, row) in result.iter().enumerate() {
                let row_ref = element_ref();
                row_refs.push(row_ref);
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
                        crate::ui::section::palette_section_label({
                            let language_now = language_now.clone();
                            move || i18n::t(language_now.as_str(), group)
                        })
                        .into_element(),
                    );
                    last_group = group;
                }
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
                        let hint = path.clone();
                        let run = open_repo.clone();
                        let open_signal = open_signal;
                        let leading = view()
                            .w(px(26.0))
                            .h(px(26.0))
                            .flex_shrink_0()
                            .items_center()
                            .justify_center()
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(move || {
                                if selected.get() == index {
                                    design::line_strong()
                                } else {
                                    design::line()
                                }
                            })
                            .bg(move || {
                                if selected.get() == index {
                                    design::content()
                                } else {
                                    design::panel()
                                }
                            })
                            .child(crate::ui::icon::icon_view(
                                icon,
                                crate::ui::icon::IconSize::Inline,
                                move || {
                                    if selected.get() == index {
                                        design::accent()
                                    } else {
                                        design::text3()
                                    }
                                },
                            ))
                            .into_element();
                        let trailing = vec![
                            dynamic(move || -> AnyElement {
                                if selected.get() == index {
                                    crate::ui::badge::kbd("↵").into_element()
                                } else {
                                    view().w(px(0.0)).h(px(0.0)).into_element()
                                }
                            })
                            .into_element(),
                        ];
                        crate::ui::row::palette_row(
                            move || label.clone(),
                            Some((Box::new(move || hint.clone()), true)),
                            Some(leading),
                            trailing,
                            move || selected.get() == index,
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
                        let language_for_hint = language_now.clone();
                        let shortcut = command.shortcut;
                        let open_signal = open_signal;
                        let icon = match id {
                            "open" | "repoManager" => "folder",
                            "openPalette" => "search",
                            "closeTab" => "file",
                            "refresh" => "refresh",
                            "history" => "clock",
                            "changes" => "diff",
                            "inspector" => "sidebar",
                            "compare" => "split",
                            "fetch" | "pull" => "down",
                            "push" => "up",
                            "toggleLanguage" => "globe",
                            "toggleTheme" => "sparkle",
                            "settings" => "gear",
                            _ => "search",
                        };
                        let leading = view()
                            .w(px(26.0))
                            .h(px(26.0))
                            .flex_shrink_0()
                            .items_center()
                            .justify_center()
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(move || {
                                if selected.get() == index {
                                    design::line_strong()
                                } else {
                                    design::line()
                                }
                            })
                            .bg(move || {
                                if selected.get() == index {
                                    design::content()
                                } else {
                                    design::panel()
                                }
                            })
                            .child(crate::ui::icon::icon_view(
                                icon,
                                crate::ui::icon::IconSize::Inline,
                                move || {
                                    if selected.get() == index {
                                        design::accent()
                                    } else {
                                        design::text3()
                                    }
                                },
                            ))
                            .into_element();
                        let trailing = if shortcut.is_empty() {
                            Vec::new()
                        } else {
                            vec![crate::ui::badge::kbd(shortcut).into_element()]
                        };
                        crate::ui::row::palette_row(
                            move || label.clone(),
                            Some((
                                Box::new(move || {
                                    i18n::t(language_for_hint.as_str(), "palette.commandHint")
                                }),
                                false,
                            )),
                            Some(leading),
                            trailing,
                            move || selected.get() == index,
                            move || {
                                run(id);
                                open_signal.set(false);
                            },
                        )
                        .into_element()
                    }
                };
                children.push(
                    view()
                        .w_full()
                        .element_ref(&row_ref)
                        .child(element)
                        .into_element(),
                );
            }
            // 搜索框保持焦点时，上下键只更新高亮；选中行离开视口后立即滚入视图。
            effect(move || {
                if let Some(row_ref) = row_refs.get(selected.get()) {
                    row_ref.reveal(
                        nsh_core::RevealOptions::default()
                            .vertical(nsh_core::ScrollAlignment::Nearest)
                            .animated(false),
                    );
                }
            });
            if children.is_empty() {
                let language_for_empty_hint = language_for_empty.clone();
                view()
                    .w_full()
                    .h_full()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(ui::GAP_SM))
                    .child(
                        view()
                            .w(px(40.0))
                            .h(px(40.0))
                            .mb(px(ui::GAP_XS))
                            .items_center()
                            .justify_center()
                            .rounded(px(10.0))
                            .bg(move || design::panel2())
                            .child(crate::ui::icon::icon_view(
                                "search",
                                crate::ui::icon::IconSize::Illustration,
                                design::text3,
                            )),
                    )
                    .child(
                        text(move || i18n::t(language_for_empty.as_str(), "palette.empty"))
                            .text_size(px(ui::BODY))
                            .line_height(px(ui::BODY_LINE))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(move || design::text2()),
                    )
                    .child(
                        text(move || {
                            i18n::t(language_for_empty_hint.as_str(), "palette.emptyHint")
                        })
                        .text_size(px(ui::SECONDARY))
                        .line_height(px(ui::SECONDARY_LINE))
                        .text_color(move || design::text3()),
                    )
                    .into_element()
            } else {
                view()
                    .w_full()
                    .flex_col()
                    .px(px(ui::GAP))
                    .py(px(ui::GAP_XS))
                    .child(children)
                    .into_element()
            }
        }));

    // 点外层不关闭，按页面画板只由 Esc 退出；面板自身截获按压事件。
    let dismiss = {
        let cmd = cmd.clone();
        move || cmd.state.palette_open.set(false)
    };
    let panel = crate::ui::dialog::modality(
        view()
            .w(px(WIDTH))
            .h(px(HEIGHT))
            .flex_col()
            .overflow_hidden()
            .rounded(px(12.0))
            .border_1()
            .border_color(move || design::line_strong())
            .bg(move || design::token("popover"))
            .shadow_lg()
            .child(
                view()
                    .w_full()
                    .h(px(50.0))
                    .flex_row()
                    .items_center()
                    .gap(px(9.0))
                    .px(px(14.0))
                    .border_b_1()
                    .border_color(move || design::line())
                    .child(crate::ui::icon::icon_view(
                        "search",
                        crate::ui::icon::IconSize::Inline,
                        design::text3,
                    ))
                    .child(view().flex_1().min_w_0().child(
                        crate::ui::text_input::palette_search_input(
                            query,
                            move || i18n::t(language.get().as_str(), "palette.placeholder"),
                            move |text: &str| {
                                query.set(text.to_string());
                                selected.set(0);
                                // 新查询从第一条结果开始，也回到结果列表顶部。
                                results_scroll.scroll_to_offset(px(0.0), false);
                            },
                            true,
                        ),
                    ))
                    .child(crate::ui::badge::kbd("esc")),
            )
            .child(view().w_full().flex_1().min_h_0().flex_col().child(list))
            .child(
                view()
                    .w_full()
                    .h(px(30.0))
                    .flex_row()
                    .items_center()
                    .gap(px(ui::GAP_SM))
                    .px(px(12.0))
                    .border_t_1()
                    .border_color(move || design::line())
                    .child(crate::ui::badge::kbd("↑↓"))
                    .child(
                        text(move || i18n::t(language.get().as_str(), "palette.navigate"))
                            .text_size(px(ui::META))
                            .text_color(move || design::text3()),
                    )
                    .child(crate::ui::badge::kbd("↵"))
                    .child(
                        text(move || i18n::t(language.get().as_str(), "palette.openSelected"))
                            .text_size(px(ui::META))
                            .text_color(move || design::text3()),
                    )
                    .child(crate::ui::badge::kbd("esc"))
                    .child(
                        text(move || i18n::t(language.get().as_str(), "palette.close"))
                            .text_size(px(ui::META))
                            .text_color(move || design::text3()),
                    )
                    .child(view().flex_1())
                    .child(crate::ui::badge::kbd("⌘P")),
            ),
    );
    return crate::ui::dialog::modal_open_transparent(
        move || i18n::t(language.get().as_str(), "palette.placeholder"),
        100,
        view()
            .w_full()
            .h_full()
            .flex_col()
            .items_center()
            .justify_start()
            .pt(px(92.0))
            .child(panel),
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
    fn empty_query_shows_common_commands_before_recent_repositories() {
        let open = vec!["/work/alpha".to_string(), "/work/beta".to_string()];
        let recent = vec!["/work/beta".to_string(), "/old/gamma".to_string()];
        let scanned = vec!["/scan/delta".to_string()];
        let all = rows("", "en", &open, &recent, &scanned);
        assert_eq!(all[0], Row::Command("open"));
        assert_eq!(all[4], Row::Command("settings"));
        assert_eq!(
            all[5],
            Row::Repo {
                path: "/work/beta".to_string(),
                open: true,
                source: RepoSource::Recent,
            }
        );
        assert_eq!(
            all[6],
            Row::Repo {
                path: "/old/gamma".to_string(),
                open: false,
                source: RepoSource::Recent,
            }
        );
        assert_eq!(
            all[7],
            Row::Repo {
                path: "/work/alpha".to_string(),
                open: true,
                source: RepoSource::Recent,
            }
        );
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
        assert_eq!(all.len(), 9);
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
