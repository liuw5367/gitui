//! 快捷键：把清单里的每个组合各绑一次，按下时查注册表决定跑哪个命令。
//!
//! 注册表是可改的（设置页 25 的 Record / Restore Defaults），而 nsh 0.5.0 的
//! `on_key` 只能在挂载时绑定一次，所以绑定的是「组合」而不是「动作」。命令面板的
//! 上下移动/回车与 Esc 的层层退出也在这里，因为退出顺序要读各功能的浮层状态。

use crate::app::handles::AppHandles;
use crate::shortcuts;
use nsh_core::prelude::*;
use std::sync::Arc;

/// 安装全部全局快捷键；`gallery_open` 是组件画廊的开关（⌘⇧D，调试入口）。
pub(crate) fn install(handles: &AppHandles, gallery_open: Signal<bool>) {
    for combo in shortcuts::catalog() {
        let pressed = combo.clone();
        let settings_cmd = handles.settings.clone();
        let registry = settings_cmd.state.shortcut_registry;
        let recording = settings_cmd.state.recording_shortcut;
        let run: Arc<dyn Fn(&'static str)> = {
            let search = handles.search.clone();
            Arc::new(move |id: &'static str| search.run_command(id))
        };
        on_key(combo.as_str(), move |_| {
            // 录制期间按键只用于改键，不执行命令。
            if let Some(id) = recording.get_untracked() {
                settings_cmd.rebind_shortcut(id, pressed.clone());
                return;
            }
            if let Some(id) = registry.get_untracked().action_for(pressed.as_str()) {
                run(id);
            }
        });
    }
    // 录制中按 Backspace（Mac 的 Delete）清空当前动作的绑定。
    on_key("backspace", {
        let settings_cmd = handles.settings.clone();
        let recording = handles.settings.state.recording_shortcut;
        move |_| {
            if let Some(id) = recording.get_untracked() {
                settings_cmd.clear_shortcut(id);
            }
        }
    });

    // 组件画廊：调试入口，独立于命令注册表，避免 P0 就改动快捷键清单。
    on_key("mod+shift+d", move |_| {
        gallery_open.set(!gallery_open.get_untracked());
    });

    // 命令面板的键盘导航：↑↓ 移动高亮，↵ 执行，打开面板时高亮回到第一行。
    // 结果表由 `SearchCommands` 自己算，和高亮用的是同一份。
    on_key("down", {
        let search = handles.search.clone();
        move |_| search.move_selection(true)
    });
    on_key("up", {
        let search = handles.search.clone();
        move |_| search.move_selection(false)
    });
    on_key("enter", {
        let search = handles.search.clone();
        move |_| search.activate_selected()
    });

    // Esc 层层退出：先关最上面的浮层，再退到过滤器；都不开时什么也不做。
    on_key("escape", {
        let sync_commands_for_esc = handles.sync.clone();
        let filter_open = handles.history.state.filter_open;
        let search_state = handles.search.state;
        let palette_open = search_state.palette_open;
        let changelist_open = handles.changes.state.changelist_open;
        let sync_state = sync_commands_for_esc.state.dialog;
        let form_state = search_state.form;
        let rewrite_state = handles.commit.state.rewrite;
        let tabs = handles.session.tabs;
        let active = handles.session.active;
        let recording = handles.settings.state.recording_shortcut;
        move |_| {
            if gallery_open.get_untracked() {
                gallery_open.set(false);
                return;
            }
            // 录制中按 Esc 只取消录制，不继续关下面的浮层。
            if recording.get_untracked().is_some() {
                recording.set(None);
                return;
            }
            if rewrite_state.get_untracked().is_some() {
                rewrite_state.set(None);
                return;
            }
            if form_state.get_untracked().is_some() {
                form_state.set(None);
                return;
            }
            if sync_state.get_untracked().is_some() {
                sync_state.set(None);
                return;
            }
            if palette_open.get_untracked() {
                palette_open.set(false);
                return;
            }
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current)
                && tab.hook_open
            {
                tab.hook_open = false;
                tabs.set(list);
                return;
            }
            if changelist_open.get_untracked() {
                changelist_open.set(false);
                return;
            }
            filter_open.set(false);
        }
    });
}

/// 向 NSH 注册本 crate 的原生组件。
///
/// `#[native_module]` 入口本身必须在 crate 根（`lib.rs`，NSH 的生成代码按
/// `crate::__nsh_native_module` 调用），这里只是它的实现体。
pub(crate) fn register(registry: &mut Registry) {
    registry.register::<crate::App>();
}
