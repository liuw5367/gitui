//! 窗口与会话级函数：设置与 AskPass 声明式窗口、关联工作树辅助窗、会话持久化与标题栏留白。
//!
//! Settings 与 AskPass 以**声明式** `window()` 元素挂在 App 根（见 `bootstrap`）。Welcome
//! 是主窗口里的模态层；关联工作树仍使用命令式独立窗口。
//!
//! 设置使用自己的原生标题栏、最小尺寸、`open` 信号和 `on_open_change` 写回；它仍不
//! 允许 Esc 关窗，因为录制快捷键时 Esc 是取消键。

use crate::app::handles::AppHandles;
use crate::app::session::Tab;
use crate::features;
use crate::i18n;
use nsh_core::desktop::{
    TitleBarOptions, TitleBarStyle, WindowAttributes, WindowKind, WindowPosition,
};
use nsh_core::element::desktop::title_bar;
use nsh_core::prelude::*;

#[derive(Clone)]
pub(crate) struct WorktreeWindow {
    pub(crate) path: String,
    pub(crate) window: nsh_core::element::desktop::WindowRef,
    pub(crate) search: crate::features::search::commands::SearchCommands,
}

impl PartialEq for WorktreeWindow {
    fn eq(&self, other: &Self) -> bool {
        // 同一路径只登记一扇工作树窗口，关闭时会先从列表移除再允许重新打开。
        self.path == other.path
    }
}

impl Eq for WorktreeWindow {}

pub(crate) type MenuWindowEntry = (
    String,
    Option<String>,
    nsh_core::element::desktop::WindowRef,
    bool,
);

/// App 自有的 Settings 与 AskPass 窗口。挂在根视图里，`is_open` 由信号驱动。
pub(crate) fn windows(handles: &AppHandles) -> impl IntoElement + use<> {
    let settings = handles.settings.clone();
    let settings_open = settings.session.settings_open;
    let settings_window_ref = settings.session.settings_window;
    let tabs = handles.session.tabs;
    let active = handles.session.active;
    let askpass = handles.askpass.clone();
    let askpass_open = askpass.open;
    let askpass_window_ref = askpass.window_ref;
    let language = handles.language;
    // 标题在窗口元素构建时定一次：响应式标题会让整扇窗（含整棵内容树）随着语言
    // 变化重建，而内容里的复选控件不支持被这样重建（nsh 会在旧 owner 上写 prop）。
    let lang = language.get_untracked();
    let settings_title = i18n::t(lang.as_str(), "settings.title");
    let askpass_title = i18n::t(lang.as_str(), "askpass.title");

    let settings_window = nsh_core::element::desktop::window()
        .id("settings")
        .kind(WindowKind::Normal)
        // 不挂到主窗口下：Settings 使用自己的原生标题栏、最小化、缩放与关闭行为。
        .default_position(WindowPosition::Center)
        .title(settings_title)
        .default_size(size(Points(920.0), Points(700.0)))
        .attributes(settings_window_attributes())
        .open(settings_open)
        .on_open_change(move |event| {
            if !event.open {
                settings_open.set(false);
                settings_window_ref.set(None);
            }
        })
        .content(move || {
            center_this_window();
            settings_window_ref.set(Some(nsh_core::element::desktop::current_window()));
            // 设置窗口的按键在这里挂一次：录制改键、Esc 取消录制、⌫ 清空绑定。
            crate::app::hotkeys::install_settings_window(&settings);
            let window = nsh_core::element::desktop::current_window();
            on_key("mod+w", move |_| window.close());
            // 当前仓库只在开窗时取一次快照：窗口内容不订阅标签页——每次标签页变化
            // 重建整页会把旧复选控件留在已释放的 owner 上（nsh 的 prop 写入会 panic）。
            let tab = tabs.with_untracked(|tabs| tabs.get(active.get_untracked()).cloned());
            return view()
                .w_full()
                .h_full()
                .child(features::settings::view::settings(
                    &settings.state,
                    &settings,
                    tab,
                ));
        });

    let askpass_window = nsh_core::element::desktop::window()
        .id("askpass")
        .kind(WindowKind::Normal)
        .default_position(WindowPosition::Center)
        .title(askpass_title)
        .default_size(size(Points(480.0), Points(280.0)))
        .open(askpass_open)
        .on_open_change({
            let askpass = askpass.clone();
            move |event| {
                if !event.open {
                    askpass_open.set(false);
                    askpass_window_ref.set(None);
                    askpass.dismiss_active();
                }
            }
        })
        .content(move || {
            center_this_window();
            let window = nsh_core::element::desktop::current_window();
            askpass_window_ref.set(Some(window.clone()));
            on_key("mod+w", move |_| window.close());
            return askpass.view(language);
        });

    return view().child(settings_window).child(askpass_window);
}

/// 窗口内容挂载后把窗口居中一次。
///
/// `default_position(Center)` 只在窗口打开时生效，窗口 frame 随标题栏与属性变化后，
/// 再按最终尺寸请求一次居中。内容闭包每次开窗只运行一次，因此不会和用户拖动窗口打架。
fn center_this_window() {
    let window = nsh_core::element::desktop::current_window();
    request_animation_frame(move |_| window.center(None));
}

/// Install a per-window opener without capturing the callback signal it writes to.
pub(crate) fn install_worktree_opener(handles: &mut AppHandles) {
    let mut opener_handles = handles.clone();
    opener_handles.refs.open_worktree_window = None;
    let opener = std::sync::Arc::new(move |tab_id, path| {
        open_worktree_window(&opener_handles, tab_id, path)
    });
    handles.refs.open_worktree_window = Some(opener);
}

/// Close the window that currently owns keyboard focus.
pub(crate) fn close_focused_window(
    handles: &AppHandles,
    gallery_window: Signal<Option<nsh_core::element::desktop::WindowRef>>,
) {
    let focused = |window: &nsh_core::element::desktop::WindowRef| {
        window.state().is_some_and(|state| state.focused)
    };
    if let Some(window) = handles.askpass.window_ref.get_untracked()
        && focused(&window)
    {
        window.close();
        return;
    }
    if let Some(window) = gallery_window.get_untracked()
        && focused(&window)
    {
        window.close();
        return;
    }
    if let Some(window) = handles.session.settings_window.get_untracked()
        && focused(&window)
    {
        window.close();
        return;
    }
    if let Some(window) = handles
        .refs
        .worktree_windows
        .get_untracked()
        .into_iter()
        .find(|window| focused(&window.window))
    {
        window.window.close();
        return;
    }
    if nsh_core::element::desktop::main_window()
        .state()
        .is_some_and(|state| state.focused)
    {
        handles.session.close_workspace();
    }
}

/// Close all app windows while keeping the process and saved workspace alive in the Dock.
pub(crate) fn close_all_windows(
    handles: &AppHandles,
    gallery_open: Signal<bool>,
    gallery_window: Signal<Option<nsh_core::element::desktop::WindowRef>>,
) {
    if let Some(window) = handles.askpass.window_ref.get_untracked() {
        window.close();
    }
    if let Some(window) = handles.session.settings_window.get_untracked() {
        window.close();
    }
    for window in handles.refs.worktree_windows.get_untracked() {
        window.window.close();
    }
    if let Some(window) = gallery_window.get_untracked() {
        window.close();
    }
    gallery_open.set(false);
    handles.session.close_workspace();
}

/// Bring every app window to the front using AppKit's native ordering action.
pub(crate) fn bring_all_to_front() {
    #[cfg(target_os = "macos")]
    if let Some(mtm) = nsh_platform_macos::objc2::MainThreadMarker::new() {
        nsh_platform_macos::objc2_app_kit::NSApplication::sharedApplication(mtm)
            .arrangeInFront(None);
    }
}

/// The workspace commands belonging to the focused workspace window.
pub(crate) fn focused_workspace_search(
    handles: &AppHandles,
) -> Option<crate::features::search::commands::SearchCommands> {
    if nsh_core::element::desktop::main_window()
        .state()
        .is_some_and(|state| state.focused)
        && !handles.session.tabs.get_untracked().is_empty()
    {
        return Some(handles.search.clone());
    }
    return handles
        .refs
        .worktree_windows
        .get_untracked()
        .into_iter()
        .find(|window| window.window.state().is_some_and(|state| state.focused))
        .filter(|window| !window.search.session.tabs.get_untracked().is_empty())
        .map(|window| window.search);
}

/// State used to enable the front workspace's repository and panel menu actions.
pub(crate) fn focused_workspace_menu_state(handles: &AppHandles) -> Option<(usize, bool, bool)> {
    if nsh_core::element::desktop::main_window()
        .state()
        .is_some_and(|state| state.focused)
    {
        let tab_count = handles.session.tabs.get().len();
        if tab_count > 0 {
            return Some((
                tab_count,
                handles.search.sidebar_open.get(),
                handles.search.right_panel_open.get(),
            ));
        }
    }
    for window in handles.refs.worktree_windows.get() {
        if window.window.state().is_some_and(|state| state.focused) {
            let tab_count = window.search.session.tabs.get().len();
            if tab_count > 0 {
                return Some((
                    tab_count,
                    window.search.sidebar_open.get(),
                    window.search.right_panel_open.get(),
                ));
            }
        }
    }
    return None;
}

/// Live, visible or minimized windows for the Window menu. AskPass is transient and excluded.
pub(crate) fn menu_window_entries(
    handles: &AppHandles,
    gallery_window: Signal<Option<nsh_core::element::desktop::WindowRef>>,
    language: &str,
) -> Vec<MenuWindowEntry> {
    let mut entries = Vec::new();
    if handles.session.settings_open.get()
        && let Some(window) = handles.session.settings_window.get()
    {
        append_menu_window(
            &mut entries,
            i18n::t(language, "settings.title"),
            None,
            window,
        );
    }
    append_menu_window(
        &mut entries,
        crate::brand::PRODUCT_NAME.to_string(),
        None,
        nsh_core::element::desktop::main_window(),
    );
    for window in handles.refs.worktree_windows.get() {
        let title = std::path::Path::new(&window.path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| window.path.clone());
        append_menu_window(
            &mut entries,
            title,
            Some(window.path.clone()),
            window.window,
        );
    }
    if let Some(window) = gallery_window.get() {
        append_menu_window(
            &mut entries,
            i18n::t(language, "menu.uiGallery"),
            None,
            window,
        );
    }
    return entries;
}

fn append_menu_window(
    entries: &mut Vec<MenuWindowEntry>,
    title: String,
    detail: Option<String>,
    window: nsh_core::element::desktop::WindowRef,
) {
    if let Some(state) = window.state()
        && (state.visible || state.minimized)
    {
        entries.push((title, detail, window, state.focused));
    }
}

/// Open a registered linked worktree in its own one-repository native window.
pub(crate) fn open_worktree_window(handles: &AppHandles, tab_id: u64, path: String) {
    let Some(parent) = handles
        .session
        .tabs
        .get_untracked()
        .iter()
        .find(|tab| tab.id == tab_id)
        .map(|tab| (tab.path.clone(), tab.git_executable.clone()))
    else {
        return;
    };
    let (parent, git_executable) = parent;
    let registered = git::Repo::open_with_executable(parent.as_str(), &git_executable)
        .ok()
        .and_then(|repo| repo.refs().ok())
        .and_then(|refs| refs.worktrees.into_iter().find(|tree| tree.path == path));
    let Some(registered) = registered.filter(|tree| tree.exists && !tree.prunable) else {
        crate::interactions::notify(
            handles.session.notice,
            crate::i18n::t(
                handles.language.get_untracked().as_str(),
                "worktree.targetChanged",
            ),
        );
        return;
    };
    let Ok(repo) = git::Repo::open_with_executable(registered.path.as_str(), &git_executable)
    else {
        crate::interactions::notify(
            handles.session.notice,
            crate::i18n::t(
                handles.language.get_untracked().as_str(),
                "worktree.targetChanged",
            ),
        );
        return;
    };
    let repo_root = repo
        .root()
        .canonicalize()
        .unwrap_or_else(|_| repo.root().to_path_buf());
    let requested = std::path::Path::new(registered.path.as_str())
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(registered.path.as_str()));
    if repo_root != requested {
        crate::interactions::notify(
            handles.session.notice,
            crate::i18n::t(
                handles.language.get_untracked().as_str(),
                "worktree.targetChanged",
            ),
        );
        return;
    }
    let path = repo.root().display().to_string();
    let windows = handles.refs.worktree_windows;
    if let Some(existing) = windows
        .get_untracked()
        .into_iter()
        .find(|window| window.path == path)
    {
        existing.window.focus();
        return;
    }

    let mut worktree_handles = handles.for_worktree();
    install_worktree_opener(&mut worktree_handles);
    worktree_handles.session.open_path(path.clone());
    let session = worktree_handles.session.clone();
    let search = worktree_handles.search.clone();
    let content_handles = worktree_handles.clone();
    let id = worktree_window_id(path.as_str());
    let title = repo.name();
    let registry = windows;
    let close_path = path.clone();
    let auxiliary_gallery = signal(false);
    let window = nsh_core::element::desktop::window()
        .id(id)
        .title(title)
        .default_size(size(Points(1280.0), Points(820.0)))
        .attributes(worktree_window_attributes())
        .on_open_change(move |change| {
            if !change.open {
                registry.update(|open| open.retain(|window| window.path != close_path));
            }
        })
        .content(move || {
            crate::app::hotkeys::install(&content_handles, auxiliary_gallery);
            worktree_window_content(content_handles.clone())
        });
    if let Some(window_ref) = nsh_core::element::desktop::open_window(window) {
        session.window.set(Some(window_ref.clone()));
        windows.update(|open| {
            open.retain(|window| window.path != path);
            open.push(WorktreeWindow {
                path,
                window: window_ref,
                search,
            });
        });
    } else {
        crate::interactions::notify(
            handles.session.notice,
            crate::i18n::t(
                handles.language.get_untracked().as_str(),
                "worktree.windowUnavailable",
            ),
        );
    }
}

fn worktree_window_content(handles: AppHandles) -> impl IntoElement + use<> {
    let tabs = handles.session.tabs;
    let active = handles.session.active;
    let current_tab = computed(move || tabs.with(|list| list.get(active.get()).cloned()));
    let language = handles.language;
    let search = handles.search.clone();
    let refs = handles.refs.clone();
    let settings_open = handles.session.settings_open;
    let window = nsh_core::element::desktop::current_window();
    on_key("mod+w", move |_| window.close());
    return view()
        .w_full()
        .h_full()
        .flex_col()
        .overflow_hidden()
        .bg(move || crate::design::content())
        .text_color(move || crate::design::text())
        .child(crate::app::workspace::workspace(&handles))
        .child(crate::features::commit::rewrite_dialog::rewrite_dialog(
            &handles.commit.state,
            &handles.commit,
            language,
        ))
        .child(crate::features::merge::view::conflict_view(
            &handles.merge.state,
            &handles.merge,
            language,
            current_tab,
        ))
        .child(crate::features::commit::hook_sheet::hook_sheet(
            &handles.commit.state,
            &handles.commit,
            language,
            current_tab,
        ))
        .child(crate::features::sync::view::sync_dialog(
            &handles.sync.state,
            &handles.sync,
            language,
            tabs,
            current_tab,
        ))
        .child(dynamic(move || -> AnyElement {
            if settings_open.get() {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            }
            crate::features::search::form_overlay(&search, &refs, language).into_element()
        }))
        .child(dynamic({
            let search = handles.search.clone();
            let recent = handles.session.recent;
            move || -> AnyElement {
                if !search.state.palette_open.get() {
                    return view().w(px(0.0)).h(px(0.0)).into_element();
                }
                let paths =
                    tabs.with(|tabs| tabs.iter().map(|tab| tab.path.clone()).collect::<Vec<_>>());
                let recent_paths = recent
                    .get()
                    .iter()
                    .map(|entry| entry.path.clone())
                    .collect();
                return crate::features::search::view::palette(
                    &search.state,
                    &search,
                    paths,
                    recent_paths,
                )
                .into_element();
            }
        }));
}

fn worktree_window_id(path: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    return format!("worktree:{:016x}", hasher.finish());
}

fn worktree_window_attributes() -> WindowAttributes {
    let mut attributes = attributes_with_min_size(1000.0, 640.0);
    let mut title_bar = TitleBarOptions::default();
    title_bar.style = TitleBarStyle::Hidden;
    title_bar.height = Some(Points(crate::design::metrics::TITLEBAR_H));
    attributes.title_bar = Some(title_bar);
    attributes.close_on_press = Some(true);
    return attributes;
}

/// 设置窗口的属性：保留快捷键录制期间的 Esc 行为，不让 Esc 关闭窗口。
fn settings_window_attributes() -> WindowAttributes {
    let mut attributes = attributes_with_min_size(760.0, 520.0);
    attributes.close_on_escape = Some(false);
    return attributes;
}

/// 窗口的通用属性：只设最小尺寸，标题栏用系统原生的（不设 `title_bar`）。
fn attributes_with_min_size(width: f32, height: f32) -> WindowAttributes {
    let mut attributes = WindowAttributes::default();
    attributes.min_size = Some(Some(size(Points(width), Points(height))));
    return attributes;
}

/// 把打开的标签页写回会话，供下次启动恢复。
pub(crate) fn persist(dir: &std::path::Path, active: Signal<usize>, tabs: Signal<Vec<Tab>>) {
    let started = std::time::Instant::now();
    let (tab_count, paths) = tabs.with_untracked(|list| {
        (
            list.len(),
            list.iter().map(|tab| tab.path.clone()).collect::<Vec<_>>(),
        )
    });
    let session = store::Session {
        active: active.get_untracked().min(paths.len().saturating_sub(1)),
        paths,
    };
    store::save_session(dir, &session);
    log::debug!(
        target: "gitui::history",
        "event=session.persist tabs={} elapsed_ms={}",
        tab_count,
        started.elapsed().as_millis(),
    );
}

/// 窗口标题栏留白（系统窗口按钮占用的左右宽度）。
pub(crate) fn window_insets() -> (f32, f32) {
    let edges = title_bar();
    return (edges.left.0, edges.right.0);
}
