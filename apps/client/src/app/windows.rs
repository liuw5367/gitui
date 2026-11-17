//! 窗口与会话级函数：欢迎页与设置两个独立窗口、会话持久化与标题栏留白。
//!
//! 两个窗口都以**声明式** `window()` 元素挂在 App 根（见 `bootstrap`）：nsh 0.5.0 的
//! `open_window()` 会忽略 `Window::open`（命令式建出来的窗口只能被用户关闭），而这两
//! 扇窗都要能被信号开关——打开仓库时关欢迎页、关掉最后一个标签页时重新打开它、菜单
//! 与 Done 开关设置页。
//!
//! 两者共用一个模型：各自的 id、标题（走 i18n）、初始与最小尺寸、`open` 信号与
//! `on_open_change` 写回，内容在 `features/*/view.rs`。窗口类型按用途分开——欢迎页是
//! 入口页（可缩放、可最小化），设置是工具窗（`Dialog`：居中于主窗口、不可最小化/
//! 最大化、Esc 可关）。

use crate::app::handles::AppHandles;
use crate::app::session::Tab;
use crate::features;
use crate::i18n;
use nsh_core::desktop::{WindowAttributes, WindowKind, WindowPosition};
use nsh_core::element::desktop::title_bar;
use nsh_core::prelude::*;

/// App 的两个独立窗口。挂在根视图里，`is_open` 由信号驱动。
pub(crate) fn windows(handles: &AppHandles) -> impl IntoElement + use<> {
    let welcome = handles.welcome.clone();
    let welcome_open = welcome.session.welcome_open;
    let settings = handles.settings.clone();
    let settings_open = settings.session.settings_open;
    let tabs = handles.session.tabs;
    let active = handles.session.active;
    let language = handles.language;
    // 标题在窗口元素构建时定一次：响应式标题会让整扇窗（含整棵内容树）随着语言
    // 变化重建，而内容里的复选控件不支持被这样重建（nsh 会在旧 owner 上写 prop）。
    let lang = language.get_untracked();
    let welcome_title = i18n::t(lang.as_str(), "welcome.windowTitle");
    let settings_title = i18n::t(lang.as_str(), "settings.title");

    let welcome_window = nsh_core::element::desktop::window()
        .id("welcome")
        .title(welcome_title)
        .default_size(size(Points(900.0), Points(600.0)))
        .attributes(attributes_with_min_size(800.0, 520.0))
        .open(welcome_open)
        .on_open_change(move |event| {
            if !event.open {
                welcome_open.set(false);
            }
        })
        .content(move || {
            // 快捷键按窗口绑定：`on_key` 属于 `current_window`，主窗口的 ⌘F 不会
            // 在欢迎窗口触发（反之亦然）。
            let focus = welcome.clone();
            on_key("mod+f", move |_| focus.focus_search());
            let open_repository = welcome.clone();
            on_key("mod+o", move |_| open_repository.open_or_choose());
            return features::welcome::view::welcome(&welcome.state, &welcome);
        });

    let settings_window = nsh_core::element::desktop::window()
        .id("settings")
        .kind(WindowKind::Dialog)
        // 不挂到主窗口下：挂上去以后它是"被拥有的对话框"，系统截图与窗口切换器都
        // 只认父窗口；居中改在屏幕上，仍然是独立的工具窗（不可最小化、Esc 可关）。
        .default_position(WindowPosition::Center)
        .title(settings_title)
        .default_size(size(Points(920.0), Points(700.0)))
        .attributes(attributes_with_min_size(760.0, 520.0))
        .open(settings_open)
        .on_open_change(move |event| {
            if !event.open {
                settings_open.set(false);
            }
        })
        .content(move || {
            // 当前仓库只在开窗时取一次快照：窗口内容不订阅标签页——每次标签页变化
            // 重建整页会把旧复选控件留在已释放的 owner 上（nsh 的 prop 写入会 panic）。
            let tab = tabs.get_untracked().get(active.get_untracked()).cloned();
            return view()
                .w_full()
                .h_full()
                .child(features::settings::view::settings(
                    &settings.state,
                    &settings,
                    tab,
                ));
        });

    return view().child(welcome_window).child(settings_window);
}

/// 窗口的通用属性：只设最小尺寸，标题栏用系统原生的（不设 `title_bar`）。
fn attributes_with_min_size(width: f32, height: f32) -> WindowAttributes {
    let mut attributes = WindowAttributes::default();
    attributes.min_size = Some(Some(size(Points(width), Points(height))));
    return attributes;
}

/// 把打开的标签页写回会话，供下次启动恢复。
pub(crate) fn persist(dir: &std::path::Path, active: Signal<usize>, tabs: Signal<Vec<Tab>>) {
    let list = tabs.get_untracked();
    let paths: Vec<String> = list.iter().map(|tab| tab.path.clone()).collect();
    let session = store::Session {
        active: active.get_untracked().min(paths.len().saturating_sub(1)),
        paths,
    };
    store::save_session(dir, &session);
}

/// 窗口标题栏留白（系统窗口按钮占用的左右宽度）。
pub(crate) fn window_insets() -> (f32, f32) {
    let edges = title_bar();
    return (edges.left.0, edges.right.0);
}
