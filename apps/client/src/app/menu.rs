//! 原生菜单：菜单栏的分组、文案与加速键，随语言重建。
//!
//! 菜单项与命令面板共用同一批动作（都经 `SearchCommands::run_command` 分发），
//! 系统角色项（撤销/最小化等）由系统给文案。

use nsh_core::desktop::MenuRole;
use nsh_core::element::desktop::MenuItem;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use store::RecentRepo;

use crate::i18n;

/// 构建原生菜单。`language` 取当前语言，切换语言后重建即可。
pub(crate) fn build_menu(
    language: &str,
    run: Arc<dyn Fn(&'static str)>,
    choose_folder: Arc<dyn Fn()>,
    open_recent: Arc<dyn Fn(String)>,
    clear_recent: Arc<dyn Fn()>,
    recent: &[RecentRepo],
    has_active_repo: bool,
    combo: &dyn Fn(&str) -> String,
    shortcuts: bool,
) -> Vec<MenuItem> {
    // 录制快捷键时不给菜单挂加速键：原生 NSMenu 的键等价物会先于窗口按键表
    // 消费组合键，否则录制 ⌘O / ⌘P 等只会触发菜单命令，无法改键。
    let shortcut_item = |item: MenuItem, id: &str| {
        if shortcuts {
            item.shortcut(&combo(id))
        } else {
            item
        }
    };
    let lang = language;
    let menu = move |key: &'static str| i18n::t(lang, key);
    let menu_run = {
        let run = run.clone();
        move |id: &'static str| {
            let run = run.clone();
            return move || run(id);
        }
    };
    let recent_labels = recent_labels(recent);
    let mut recent_items = Vec::with_capacity(recent.len() + 2);
    if recent.is_empty() {
        recent_items.push(MenuItem::new(menu("menu.noRecent")).disabled(true));
    } else {
        for (repo, label) in recent.iter().zip(recent_labels) {
            recent_items.push(MenuItem::new(label).on_select({
                let open_recent = open_recent.clone();
                let path = repo.path.clone();
                move || open_recent(path.clone())
            }));
        }
    }
    recent_items.push(MenuItem::separator());
    recent_items.push(
        MenuItem::new(menu("menu.clearRecent"))
            .disabled(recent.is_empty())
            .on_select({
                let clear_recent = clear_recent.clone();
                move || clear_recent()
            }),
    );
    return vec![
        MenuItem::submenu(
            crate::brand::PRODUCT_NAME,
            vec![
                MenuItem::role(MenuRole::About),
                MenuItem::separator(),
                shortcut_item(MenuItem::new(menu("menu.settings")), "settings")
                    .on_select(menu_run("settings")),
                MenuItem::separator(),
                MenuItem::role(MenuRole::Hide),
                MenuItem::role(MenuRole::HideOthers),
                MenuItem::role(MenuRole::ShowAll),
                MenuItem::separator(),
                MenuItem::role(MenuRole::Quit),
            ],
        ),
        MenuItem::submenu(
            menu("menu.app.file"),
            vec![
                shortcut_item(MenuItem::new(menu("menu.openRepo")), "open").on_select({
                    let choose_folder = choose_folder.clone();
                    move || choose_folder()
                }),
                MenuItem::submenu(menu("menu.openRecent"), recent_items)
                    .disabled(recent.is_empty()),
                MenuItem::separator(),
                shortcut_item(MenuItem::new(menu("menu.newTab")), "newTab")
                    .on_select(menu_run("newTab")),
                MenuItem::new(menu("menu.closeTab"))
                    .disabled(!has_active_repo)
                    .on_select(menu_run("closeTab")),
                MenuItem::separator(),
            ],
        ),
        MenuItem::submenu(
            menu("menu.app.edit"),
            vec![
                MenuItem::role(MenuRole::Undo),
                MenuItem::role(MenuRole::Redo),
                MenuItem::separator(),
                MenuItem::role(MenuRole::Cut),
                MenuItem::role(MenuRole::Copy),
                MenuItem::role(MenuRole::Paste),
                MenuItem::role(MenuRole::SelectAll),
            ],
        ),
        MenuItem::submenu(
            menu("menu.app.view"),
            vec![
                shortcut_item(MenuItem::new(menu("menu.commandPalette")), "openPalette")
                    .on_select(menu_run("openPalette")),
                MenuItem::separator(),
                MenuItem::new(menu("menu.showHistory"))
                    .disabled(!has_active_repo)
                    .on_select(menu_run("history")),
                MenuItem::new(menu("menu.showChanges"))
                    .disabled(!has_active_repo)
                    .on_select(menu_run("changes")),
                MenuItem::new(menu("menu.toggleInspector"))
                    .disabled(!has_active_repo)
                    .on_select(menu_run("inspector")),
                MenuItem::separator(),
                shortcut_item(MenuItem::new(menu("menu.cycleAppearance")), "toggleTheme")
                    .on_select(menu_run("toggleTheme")),
                MenuItem::new(menu("menu.switchLanguage")).on_select(menu_run("toggleLanguage")),
            ],
        ),
        MenuItem::submenu(
            menu("menu.app.repository"),
            vec![
                MenuItem::new(menu("menu.fetch"))
                    .disabled(!has_active_repo)
                    .on_select(menu_run("fetch")),
                MenuItem::new(menu("menu.pull"))
                    .disabled(!has_active_repo)
                    .on_select(menu_run("pull")),
                MenuItem::new(menu("menu.push"))
                    .disabled(!has_active_repo)
                    .on_select(menu_run("push")),
                MenuItem::separator(),
                shortcut_item(MenuItem::new(menu("menu.refresh")), "refresh")
                    .disabled(!has_active_repo)
                    .on_select(menu_run("refresh")),
                MenuItem::new(menu("menu.repoManager")).on_select(menu_run("repoManager")),
            ],
        ),
        MenuItem::submenu(
            menu("menu.app.window"),
            vec![
                MenuItem::role(MenuRole::Minimize),
                MenuItem::role(MenuRole::Zoom),
                MenuItem::separator(),
                MenuItem::role(MenuRole::ToggleFullScreen),
            ],
        ),
        MenuItem::submenu(
            menu("menu.app.help"),
            vec![
                MenuItem::new(menu("menu.keyboardShortcuts")).on_select(menu_run("helpShortcuts")),
                MenuItem::separator(),
                MenuItem::new(menu("menu.welcome")).on_select(menu_run("welcome")),
            ],
        ),
    ];
}

#[cfg(test)]
mod menu_tests {
    use super::*;

    /// 菜单文案全部来自 locales：两种语言都必须能取到，且内容不同。
    #[test]
    fn menu_labels_exist_in_both_languages() {
        let keys = [
            "menu.app.file",
            "menu.app.edit",
            "menu.app.view",
            "menu.app.repository",
            "menu.app.window",
            "menu.app.help",
            "menu.openRepo",
            "menu.openRecent",
            "menu.noRecent",
            "menu.clearRecent",
            "menu.closeTab",
            "menu.commandPalette",
            "menu.showHistory",
            "menu.showChanges",
            "menu.toggleInspector",
            "menu.cycleAppearance",
            "menu.switchLanguage",
            "menu.fetch",
            "menu.pull",
            "menu.push",
            "menu.refresh",
            "menu.repoManager",
            "menu.settings",
            "menu.welcome",
            "menu.keyboardShortcuts",
        ];
        for key in keys {
            let english = i18n::t(i18n::DEFAULT_LANGUAGE, key);
            let chinese = i18n::t(i18n::ZH_HANS_LANG, key);
            assert_ne!(english, key, "{key} 在英文里缺文案");
            assert_ne!(chinese, key, "{key} 在中文里缺文案");
            assert_ne!(english, chinese, "{key} 两种语言不应相同");
        }
    }

    #[test]
    fn duplicate_recent_names_include_their_parent_directory() {
        let recent = vec![
            RecentRepo {
                path: "/work/alpha/project".to_string(),
                opened_at: 2,
                branch: String::new(),
            },
            RecentRepo {
                path: "/archive/beta/project".to_string(),
                opened_at: 1,
                branch: String::new(),
            },
        ];

        let labels = recent_labels(&recent);

        assert!(labels[0].contains("project"));
        assert!(labels[0].contains("alpha"));
        assert!(labels[1].contains("beta"));
    }
}

fn recent_labels(recent: &[RecentRepo]) -> Vec<String> {
    let mut counts = HashMap::new();
    for repo in recent {
        *counts
            .entry(repo_name(&repo.path).to_lowercase())
            .or_insert(0) += 1;
    }
    return recent
        .iter()
        .map(|repo| {
            let name = repo_name(&repo.path);
            if counts.get(&name.to_lowercase()).copied().unwrap_or(0) < 2 {
                return name;
            }
            let parent = Path::new(&repo.path)
                .parent()
                .map(|path| path.display().to_string())
                .filter(|path| !path.is_empty())
                .unwrap_or_else(|| repo.path.clone());
            return format!("{name} — {parent}");
        })
        .collect();
}

fn repo_name(path: &str) -> String {
    return Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.to_string());
}
