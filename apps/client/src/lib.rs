//! 根组件：窗口属性、主题装载、语言状态与屏幕路由。
//!
//! 一个窗口、多个仓库标签页；屏幕为欢迎页、仓库管理页与工作区。当前只读：
//! 打开/克隆/初始化仓库、按条件筛选历史、多提交对比、查看提交内单文件补丁。
//! 写路径在后续阶段加入。

pub mod brand;
mod design;
mod filters;
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

use filters::When;
use git::LogFilter;
use model::Tab;

/// `theme.json` 与设计稿 CSS 的 light/dark 值一一对应。
const THEME_JSON: &str = include_str!("../theme.json");

/// 当前显示哪一屏。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Screen {
    Welcome,
    RepoManager,
    Workspace,
}

/// 工作区主区的两种模式（工具栏的 History / Changes 分段）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    History,
    Changes,
}

/// 仓库管理页的三种入口（设计板 03 的分段控件）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RepoForm {
    Open,
    Clone,
    Init,
}

/// 组件间共享的动作集合。统一用 `Arc<dyn Fn>` 传递，避免把十几个信号逐个
/// 穿到子视图里。
#[derive(Clone)]
pub(crate) struct Actions {
    pub(crate) open_path: Arc<dyn Fn(String)>,
    pub(crate) choose_folder: Arc<dyn Fn()>,
    pub(crate) close_tab: Arc<dyn Fn(usize)>,
    pub(crate) activate_tab: Arc<dyn Fn(usize)>,
    pub(crate) select_commit: Arc<dyn Fn(usize)>,
    pub(crate) toggle_compare: Arc<dyn Fn(usize)>,
    pub(crate) set_compare_mode: Arc<dyn Fn(bool)>,
    pub(crate) select_file: Arc<dyn Fn(usize)>,
    pub(crate) set_mode: Arc<dyn Fn(Mode)>,
    pub(crate) select_change: Arc<dyn Fn(usize)>,
    pub(crate) toggle_stage: Arc<dyn Fn(usize)>,
    pub(crate) discard_change: Arc<dyn Fn(usize)>,
    pub(crate) stage_all: Arc<dyn Fn()>,
    pub(crate) unstage_all: Arc<dyn Fn()>,
    pub(crate) commit: Arc<dyn Fn()>,
    pub(crate) resolve_conflict: Arc<dyn Fn(String, bool)>,
    pub(crate) abort_operation: Arc<dyn Fn()>,
    pub(crate) sync: Arc<dyn Fn(model::Sync)>,
    pub(crate) apply_filter: Arc<dyn Fn()>,
    pub(crate) clear_filter: Arc<dyn Fn()>,
    pub(crate) clone_repository: Arc<dyn Fn()>,
    pub(crate) init_repository: Arc<dyn Fn()>,
    pub(crate) toggle_language: Arc<dyn Fn()>,
    pub(crate) refresh: Arc<dyn Fn()>,
    pub(crate) go_welcome: Arc<dyn Fn()>,
    pub(crate) go_repo_manager: Arc<dyn Fn()>,
    pub(crate) set_repo_form: Arc<dyn Fn(RepoForm)>,
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

    // 历史筛选：关键字在工具栏搜索框，其余条件在 History 的筛选浮层。
    let history_query = signal(String::new());
    let filter_reference = signal(String::new());
    let filter_author = signal(String::new());
    let filter_when = signal(SharedString::from(When::All.id()));
    let filter_open = signal(false);
    let inspector_open = signal(false);
    let compare_mode = signal(false);
    let mode = signal(Mode::History);
    let commit_summary = signal(String::new());
    let commit_description = signal(String::new());
    let changelist_open = signal(false);
    let now = store::now_unix();

    let session = store::load_session(&store_dir);
    let initial_filter = LogFilter::default();
    let mut initial_tabs: Vec<Tab> = session
        .paths
        .iter()
        .map(|path| model::load_tab(path, &initial_filter))
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

    // 欢迎页与仓库管理页的输入框。
    let welcome_query = signal(String::new());
    let clone_url = signal(String::new());
    let clone_destination = signal(String::new());
    let clone_error = signal(Option::<String>::None);
    let cloning = signal(false);
    let init_path = signal(String::new());
    let repo_form = signal(RepoForm::Open);

    let current_filter: Arc<dyn Fn() -> LogFilter> = {
        let history_query = history_query;
        let filter_reference = filter_reference;
        let filter_author = filter_author;
        let filter_when = filter_when;
        Arc::new(move || {
            return filters::build(
                history_query.get_untracked().as_str(),
                filter_reference.get_untracked().as_str(),
                filter_author.get_untracked().as_str(),
                When::from_key(filter_when.get_untracked().as_str()),
                now,
            );
        })
    };

    let open_path: Arc<dyn Fn(String)> = {
        let tabs = tabs;
        let active = active;
        let recent = recent;
        let screen = screen;
        let store_dir = store_dir.clone();
        let current_filter = current_filter.clone();
        Arc::new(move |path: String| {
            let tab = model::load_tab(path.as_str(), &current_filter());
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

    let toggle_compare: Arc<dyn Fn(usize)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |index: usize| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::toggle_compare(tab, index);
                tabs.set(list);
            }
        })
    };

    let set_compare_mode: Arc<dyn Fn(bool)> = {
        let tabs = tabs;
        let active = active;
        let compare_mode = compare_mode;
        Arc::new(move |enabled: bool| {
            compare_mode.set(enabled);
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                tab.comparing = enabled;
                if !enabled {
                    tab.compare.clear();
                }
                tabs.set(list);
            }
        })
    };

    let select_file: Arc<dyn Fn(usize)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |index: usize| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::select_file(tab, index);
                tabs.set(list);
            }
        })
    };

    let reload: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        let current_filter = current_filter.clone();
        Arc::new(move || {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get(current).cloned() {
                list[current] = model::reload_with_filter(&tab, &current_filter());
                tabs.set(list);
            }
        })
    };

    let apply_filter: Arc<dyn Fn()> = {
        let reload = reload.clone();
        let filter_open = filter_open;
        Arc::new(move || {
            filter_open.set(false);
            reload();
        })
    };

    let clear_filter: Arc<dyn Fn()> = {
        let history_query = history_query;
        let filter_reference = filter_reference;
        let filter_author = filter_author;
        let filter_when = filter_when;
        let reload = reload.clone();
        Arc::new(move || {
            history_query.set(String::new());
            filter_reference.set(String::new());
            filter_author.set(String::new());
            filter_when.set(SharedString::from(When::All.id()));
            reload();
        })
    };

    let refresh: Arc<dyn Fn()> = {
        let reload = reload.clone();
        Arc::new(move || reload())
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

    let init_repository: Arc<dyn Fn()> = {
        let init_path = init_path;
        let clone_error = clone_error;
        let open_path = open_path.clone();
        Arc::new(move || {
            let path = init_path.get_untracked();
            if path.trim().is_empty() {
                clone_error.set(Some("缺少目录".to_string()));
                return;
            }
            match git::init_repository(path.trim()) {
                Ok(()) => open_path(path.trim().to_string()),
                Err(error) => clone_error.set(Some(error.detail())),
            }
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

    let go_repo_manager: Arc<dyn Fn()> = {
        let screen = screen;
        Arc::new(move || screen.set(Screen::RepoManager))
    };

    let set_repo_form: Arc<dyn Fn(RepoForm)> = {
        let repo_form = repo_form;
        Arc::new(move |form: RepoForm| repo_form.set(form))
    };

    let set_mode: Arc<dyn Fn(Mode)> = {
        let mode = mode;
        Arc::new(move |next: Mode| mode.set(next))
    };

    let select_change: Arc<dyn Fn(usize)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |index: usize| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::load_change_diff(tab, index);
                tabs.set(list);
            }
        })
    };

    let toggle_stage: Arc<dyn Fn(usize)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |index: usize| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::toggle_stage(tab, index);
                tabs.set(list);
            }
        })
    };

    // 丢弃不可撤销：先弹系统确认框，确认后才动工作区。
    let discard_change: Arc<dyn Fn(usize)> = {
        let tabs = tabs;
        let active = active;
        let language = language;
        Arc::new(move |index: usize| {
            let list = tabs.get_untracked();
            let current = active.get_untracked();
            let Some(tab) = list.get(current).cloned() else {
                return;
            };
            let Some(entry) = tab.changes.get(index).cloned() else {
                return;
            };
            let language_now = language.get_untracked();
            let pending = nsh_base::alert_dialog()
                .title(i18n::t(language_now.as_str(), "changes.discardTitle"))
                .description(i18n::with_arguments(
                    language_now.as_str(),
                    "changes.discardMessage",
                    &[("path", entry.path.clone())],
                ))
                .destructive(
                    "discard",
                    i18n::t(language_now.as_str(), "changes.discardAction"),
                )
                .cancel("cancel", i18n::t(language_now.as_str(), "common.cancel"))
                .show();
            let tabs = tabs;
            let active = active;
            spawn(async move {
                if let Ok(result) = pending.await {
                    if result.action.as_deref() != Some("discard") {
                        return;
                    }
                    let current = active.get_untracked();
                    let mut list = tabs.get_untracked();
                    if let Some(tab) = list.get_mut(current) {
                        model::discard_change(tab, index);
                        tabs.set(list);
                    }
                }
            });
        })
    };

    let stage_all: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move || {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::stage_all_or_none(tab, true);
                tabs.set(list);
            }
        })
    };

    let unstage_all: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move || {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::stage_all_or_none(tab, false);
                tabs.set(list);
            }
        })
    };

    let commit: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        let commit_summary = commit_summary;
        let commit_description = commit_description;
        Arc::new(move || {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                tab.summary = commit_summary.get_untracked();
                tab.description = commit_description.get_untracked();
                model::commit_staged(tab);
                if tab.hook_ok {
                    commit_summary.set(String::new());
                    commit_description.set(String::new());
                }
                tabs.set(list);
            }
        })
    };

    let resolve_conflict: Arc<dyn Fn(String, bool)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |path: String, ours: bool| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::resolve_conflict(tab, path.as_str(), ours);
                tabs.set(list);
            }
        })
    };

    let sync: Arc<dyn Fn(model::Sync)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |kind: model::Sync| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::sync(tab, kind);
                tabs.set(list);
            }
        })
    };

    let abort_operation: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move || {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::abort_operation(tab);
                tabs.set(list);
            }
        })
    };

    let actions = Actions {
        open_path: open_path.clone(),
        choose_folder: choose_folder.clone(),
        close_tab,
        activate_tab,
        select_commit,
        toggle_compare,
        set_compare_mode,
        select_file,
        set_mode,
        select_change,
        toggle_stage,
        discard_change,
        stage_all,
        unstage_all,
        commit,
        resolve_conflict,
        abort_operation,
        sync,
        apply_filter,
        clear_filter,
        clone_repository,
        init_repository,
        toggle_language,
        refresh,
        go_welcome,
        go_repo_manager,
        set_repo_form,
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
    on_key("mod+f", {
        let filter_open = filter_open;
        move |_| filter_open.set(true)
    });
    on_key("escape", {
        let filter_open = filter_open;
        move |_| filter_open.set(false)
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
                        Screen::RepoManager => screens::repo_manager(
                            language_value,
                            recent_value,
                            welcome_query,
                            clone_url,
                            clone_destination,
                            init_path,
                            clone_error,
                            cloning,
                            repo_form,
                            actions_for_view.clone(),
                        )
                        .into_element(),
                        Screen::Workspace => screens::workspace(
                            language_value,
                            tabs,
                            active,
                            history_query,
                            filter_reference,
                            filter_author,
                            filter_when,
                            filter_open,
                            inspector_open,
                            compare_mode,
                            mode,
                            commit_summary,
                            commit_description,
                            changelist_open,
                            actions_for_view.clone(),
                        )
                        .into_element(),
                    }
                })),
        );
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

/// 供子模块使用的设计常量。
pub use design::metrics;

#[native_module("app/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}
