//! 原生菜单：菜单栏的分组、文案与加速键，随语言重建。
//!
//! 菜单项与命令面板共用同一批动作（都经 `SearchCommands::run_command` 分发），
//! 系统角色项（撤销/最小化等）由系统给文案。

use nsh_core::desktop::MenuRole;
use nsh_core::element::desktop::MenuItem;
use std::sync::Arc;

use crate::i18n;

/// 构建原生菜单。`language` 取当前语言，切换语言后重建即可。
pub(crate) fn build_menu(
    language: &str,
    run: Arc<dyn Fn(&'static str)>,
    choose_folder: Arc<dyn Fn()>,
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
    return vec![
        MenuItem::submenu(
            menu("menu.app.file"),
            vec![
                shortcut_item(MenuItem::new(menu("menu.openRepo")), "open").on_select({
                    let choose_folder = choose_folder.clone();
                    move || choose_folder()
                }),
                shortcut_item(MenuItem::new(menu("menu.newTab")), "newTab")
                    .on_select(menu_run("newTab")),
                MenuItem::new(menu("menu.closeTab")).on_select(menu_run("closeTab")),
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
                MenuItem::new(menu("menu.showHistory")).on_select(menu_run("history")),
                MenuItem::new(menu("menu.showChanges")).on_select(menu_run("changes")),
                MenuItem::new(menu("menu.toggleInspector")).on_select(menu_run("inspector")),
                MenuItem::separator(),
                shortcut_item(MenuItem::new(menu("menu.cycleAppearance")), "toggleTheme")
                    .on_select(menu_run("toggleTheme")),
                MenuItem::new(menu("menu.switchLanguage")).on_select(menu_run("toggleLanguage")),
            ],
        ),
        MenuItem::submenu(
            menu("menu.app.repository"),
            vec![
                MenuItem::new(menu("menu.fetch")).on_select(menu_run("fetch")),
                MenuItem::new(menu("menu.pull")).on_select(menu_run("pull")),
                MenuItem::new(menu("menu.push")).on_select(menu_run("push")),
                MenuItem::separator(),
                shortcut_item(MenuItem::new(menu("menu.refresh")), "refresh")
                    .on_select(menu_run("refresh")),
                MenuItem::new(menu("menu.repoManager")).on_select(menu_run("repoManager")),
                MenuItem::separator(),
                shortcut_item(MenuItem::new(menu("menu.settings")), "settings")
                    .on_select(menu_run("settings")),
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
                MenuItem::new(menu("menu.welcome")).on_select(menu_run("welcome")),
                MenuItem::separator(),
                MenuItem::role(MenuRole::About),
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
        ];
        for key in keys {
            let english = i18n::t(i18n::DEFAULT_LANGUAGE, key);
            let chinese = i18n::t(i18n::ZH_HANS_LANG, key);
            assert_ne!(english, key, "{key} 在英文里缺文案");
            assert_ne!(chinese, key, "{key} 在中文里缺文案");
            assert_ne!(english, chinese, "{key} 两种语言不应相同");
        }
    }
}
