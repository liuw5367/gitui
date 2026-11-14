//! gitui 的根组件：窗口属性、主题装载、语言状态与欢迎页/工作区路由。
//!
//! 一个窗口、多个仓库标签页。P1 只读：欢迎页可以打开或克隆仓库，工作区显示真实
//! 历史、提交补丁与引用。写路径在后续阶段加入。

pub mod brand;
mod design;
mod i18n;
mod model;
mod screens;
mod shell;
mod widgets;
mod workspace;

use nsh_base::theme_overrides;
use nsh_core::desktop::{TitleBarOptions, TitleBarStyle, WindowAttributes};
use nsh_core::element::desktop::{main_window, title_bar};
use nsh_core::element::set_tokens;
use nsh_core::prelude::*;
use std::sync::Arc;

use model::Tab;

/// `theme.json` 与设计稿 CSS 的 light/dark 值一一对应。
const THEME_JSON: &str = include_str!("../theme.json");

/// 当前显示哪一屏。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Screen {
    Welcome,
    Workspace,
}

/// 组件间共享的动作集合。统一用 `Arc<dyn Fn>` 传递，避免把十几个信号
/// 逐个穿到子视图里。
#[derive(Clone)]
pub(crate) struct Actions {
    pub(crate) open_path: Arc<dyn Fn(String)>,
    pub(crate) choose_folder: Arc<dyn Fn()>,
    pub(crate) close_tab: Arc<dyn Fn(usize)>,
    pub(crate) activate_tab: Arc<dyn Fn(usize)>,
    pub(crate) select_commit: Arc<dyn Fn(usize)>,
    pub(crate) clone_repository: Arc<dyn Fn()>,
    pub(crate) toggle_language: Arc<dyn Fn()>,
    pub(crate) refresh: Arc<dyn Fn()>,
    pub(crate) go_welcome: Arc<dyn Fn()>,
}

/// 应用根组件。
#[component]
pub fn App() -> impl IntoElement {
    let tokens = theme_overrides(THEME_JSON).expect("theme.json 可以解析");
    set_tokens(tokens);

    let mut title_bar = TitleBarOptions::default();
    title_bar.style = TitleBarStyle::Hidden;
    title_bar.height = Some(Points(44.0));
    let mut window = WindowAttributes::default();
    window.title = Some(brand::PRODUCT_NAME.to_string());
    window.title_bar = Some(title_bar);
    window.min_size = Some(Some(size(Points(1100.0), Points(700.0))));
    main_window().set(window);

    let store_dir = store::default_dir(brand::STORAGE_DIR);
    let language = signal(SharedString::from(
        store::load_language(&store_dir)
            .unwrap_or_else(|| i18n::DEFAULT_LANGUAGE.to_string()),
    ));

    let session = store::load_session(&store_dir);
    let mut initial_tabs: Vec<Tab> = session
        .paths
        .iter()
        .map(|path| model::load_tab(path))
        .filter(|tab| tab.error.is_none())
        .collect();
    let screen = signal(if initial_tabs.is_empty() {
        Screen::Welcome
    } else {
        Screen::Workspace
    });
    let active = signal(session.active.min(initial_tabs.len().saturating_sub(1)));
    if initial_tabs.is_empty() {
        initial_tabs.clear();
    }
    let tabs = signal(initial_tabs);
    let recent = signal(store::load_recent(&store_dir));

    // 欢迎页的克隆表单与搜索。
    let welcome_query = signal(String::new());
    let clone_url = signal(String::new());
    let clone_destination = signal(String::new());
    let clone_error = signal(Option::<String>::None);
    let cloning = signal(false);
    let history_query = signal(String::new());

    let open_path: Arc<dyn Fn(String)> = {
        let tabs = tabs;
        let active = active;
        let recent = recent;
        let screen = screen;
        let store_dir = store_dir.clone();
        Arc::new(move |path: String| {
            let tab = model::load_tab(path.as_str());
            let mut list = tabs.get_untracked();
            let existing = list
                .iter()
                .position(|candidate| candidate.path == tab.path);
            match existing {
                Some(index) => {
                    list[index] = tab;
                    active.set(index);
                }
                None => {
                    list.push(tab);
                    active.set(list.len() - 1);
                }
            }
            tabs.set(list);
            store::remember_recent(&store_dir, &recent.get_untracked(), path.as_str());
            recent.set(store::load_recent(&store_dir));
            screen.set(Screen::Workspace);
            persist(&store_dir, active, tabs);
        })
    };

    let choose_folder: Arc<dyn Fn()> = {
        let open_path = open_path.clone();
        Arc::new(move || {
            let answer = pick_paths(nsh_core::file_dialog::OpenDialog::folders());
            let open_path = open_path.clone();
            spawn(async move {
                if let Ok(Some(paths)) = answer.await {
                    if let Some(path) = paths.first() {
                        open_path(path.display().to_string());
                    }
                }
            });
        })
    };

    let close_tab: Arc<dyn Fn(usize)> = {
        let tabs = tabs;
        let active = active;
        let screen = screen;
        let store_dir = store_dir.clone();
        Arc::new(move |index: usize| {
            let mut list = tabs.get_untracked();
            if index >= list.len() {
                return;
            }
            list.remove(index);
            let next_active = if list.is_empty() {
                0
            } else {
                active.get_untracked().min(list.len() - 1)
            };
            tabs.set(list.clone());
            active.set(next_active);
            if list.is_empty() {
                screen.set(Screen::Welcome);
            }
            persist(&store_dir, active, tabs);
        })
    };

    let activate_tab: Arc<dyn Fn(usize)> = {
        let active = active;
        let tabs = tabs;
        let store_dir = store_dir.clone();
        Arc::new(move |index: usize| {
            if index < tabs.get_untracked().len() {
                active.set(index);
                persist(&store_dir, active, tabs);
            }
        })
    };

    let select_commit: Arc<dyn Fn(usize)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |index: usize| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::select_commit(tab, index);
                tabs.set(list);
            }
        })
    };

    let refresh: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move || {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get(current).cloned() {
                let next = model::refresh(&tab);
                list[current] = next;
                tabs.set(list);
            }
        })
    };

    let clone_repository: Arc<dyn Fn()> = {
        let clone_url = clone_url;
        let clone_destination = clone_destination;
        let clone_error = clone_error;
        let cloning = cloning;
        let open_path = open_path.clone();
        Arc::new(move || {
            let url = clone_url.get_untracked();
            let destination = clone_destination.get_untracked();
            if url.trim().is_empty() {
                clone_error.set(Some("缺少仓库地址".to_string()));
                return;
            }
            if destination.trim().is_empty() {
                clone_error.set(Some("缺少目标位置".to_string()));
                return;
            }
            cloning.set(true);
            clone_error.set(None);
            let open_path = open_path.clone();
            let clone_error = clone_error;
            let cloning = cloning;
            spawn(async move {
                let result = git::clone(url.trim(), destination.trim());
                cloning.set(false);
                match result {
                    Ok(()) => open_path(destination.trim().to_string()),
                    Err(error) => clone_error.set(Some(error.detail())),
                }
            });
        })
    };

    let toggle_language: Arc<dyn Fn()> = {
        let language = language;
        let store_dir = store_dir.clone();
        Arc::new(move || {
            let next = i18n::toggle(language.get_untracked().as_str());
            language.set(SharedString::from(next));
            store::save_language(&store_dir, next);
        })
    };

    let go_welcome: Arc<dyn Fn()> = {
        let screen = screen;
        Arc::new(move || screen.set(Screen::Welcome))
    };

    let actions = Actions {
        open_path: open_path.clone(),
        choose_folder: choose_folder.clone(),
        close_tab,
        activate_tab,
        select_commit,
        clone_repository,
        toggle_language,
        refresh,
        go_welcome,
    };

    on_key("mod+o", {
        let choose_folder = actions.choose_folder.clone();
        move |_| choose_folder()
    });
    on_key("mod+w", {
        let close_tab = actions.close_tab.clone();
        let active = active;
        move |_| close_tab(active.get_untracked())
    });
    on_key("mod+r", {
        let refresh = actions.refresh.clone();
        move |_| refresh()
    });

    let language_value = language;
    let screen_value = screen;
    let actions_for_view = actions.clone();
    let recent_value = recent;

    return view()
        .w_full()
        .h_full()
        .flex_col()
        .bg(move || design::content())
        .text_color(move || design::text())
        .child(
            view()
                .w_full()
                .h_full()
                .flex_col()
                .child(dynamic(move || -> AnyElement {
                    match screen_value.get() {
                        Screen::Welcome => screens::welcome(
                            language_value,
                            recent_value,
                            welcome_query,
                            clone_url,
                            clone_destination,
                            clone_error,
                            cloning,
                            actions_for_view.clone(),
                        )
                        .into_element(),
                        Screen::Workspace => screens::workspace(
                            language_value,
                            tabs,
                            active,
                            history_query,
                            actions_for_view.clone(),
                        )
                        .into_element(),
                    }
                })),
        );
}

/// 把打开的标签页写回会话，供下次启动恢复。
pub(crate) fn persist(
    dir: &std::path::Path,
    active: Signal<usize>,
    tabs: Signal<Vec<Tab>>,
) {
    let list = tabs.get_untracked();
    let paths: Vec<String> = list.iter().map(|tab| tab.path.clone()).collect();
    let session = store::Session {
        active: active.get_untracked().min(paths.len().saturating_sub(1)),
        paths,
    };
    store::save_session(dir, &session);
}

/// 供子模块使用的窗口标题栏留白。
pub(crate) fn window_insets() -> (f32, f32) {
    let edges = title_bar();
    return (edges.left.0, edges.right.0);
}

#[native_module("app/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}

pub use design::metrics;
pub(crate) use widgets as ui;
