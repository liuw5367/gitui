//! 窗口与会话级函数：欢迎页与设置声明式窗口、关联工作树辅助窗、会话持久化与标题栏留白。
//!
//! 欢迎与设置窗口以**声明式** `window()` 元素挂在 App 根（见 `bootstrap`）：nsh 0.6.0 的
//! `open_window()` 会忽略 `Window::open`（命令式建出来的窗口只能被用户关闭），而这两
//! 扇窗都要能被信号开关——打开仓库时关欢迎页、关掉最后一个标签页时重新打开它、菜单
//! 与 Done 开关设置页。
//!
//! 两者共用一个模型：各自的 id、标题（走 i18n）、初始与最小尺寸、`open` 信号与
//! `on_open_change` 写回，内容在 `features/*/view.rs`。Welcome 与 Settings 都是独立的
//! 普通窗口；设置仍不允许 Esc 关窗，因为录制快捷键时 Esc 是取消键。

use crate::app::handles::AppHandles;
use crate::app::session::Tab;
use crate::features;
use crate::i18n;
use nsh_core::desktop::{
    TitleBarOptions, TitleBarStyle, WindowAttributes, WindowKind, WindowPosition,
};
use nsh_core::element::desktop::title_bar;
use nsh_core::prelude::*;

/// App 的两个独立窗口。挂在根视图里，`is_open` 由信号驱动。
pub(crate) fn windows(handles: &AppHandles) -> impl IntoElement + use<> {
    let welcome = handles.welcome.clone();
    let welcome_open = welcome.session.welcome_open;
    let welcome_window_ref = welcome.session.welcome_window;
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
    let welcome_title = i18n::t(lang.as_str(), "welcome.windowTitle");
    let settings_title = i18n::t(lang.as_str(), "settings.title");
    let askpass_title = i18n::t(lang.as_str(), "askpass.title");

    let welcome_window = nsh_core::element::desktop::window()
        .id("welcome")
        .title(welcome_title)
        .default_size(size(Points(900.0), Points(600.0)))
        // 没有位置时 nsh 从主窗口左上角层叠（每扇窗偏移 24pt），三扇窗会互相错开而不是
        // 落在屏幕中央；欢迎页与设置窗都显式居中。
        .default_position(WindowPosition::Center)
        .attributes(attributes_with_min_size(800.0, 520.0))
        .open(welcome_open)
        .on_open_change(move |event| {
            if !event.open {
                welcome_open.set(false);
                welcome_window_ref.set(None);
            }
        })
        .content(move || {
            center_this_window();
            welcome_window_ref.set(Some(nsh_core::element::desktop::current_window()));
            // 快捷键按窗口绑定：`on_key` 属于 `current_window`，主窗口的 ⌘F 不会
            // 在欢迎窗口触发（反之亦然）。
            let focus = welcome.clone();
            on_key("mod+f", move |_| focus.focus_search());
            let open_repository = welcome.clone();
            on_key("mod+o", move |_| open_repository.open_or_choose());
            let window = nsh_core::element::desktop::current_window();
            on_key("mod+w", move |_| window.close());
            return features::welcome::view::welcome(&welcome.state, &welcome);
        });

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

    return view()
        .child(welcome_window)
        .child(settings_window)
        .child(askpass_window);
}

/// 窗口内容挂载后把窗口居中一次。
///
/// `default_position(Center)` 只在窗口打开的那一刻生效，而窗口的 frame 会随标题栏与
/// 属性在打开之后变化（实测 900×600 的欢迎页落在主窗口层叠位置、主窗口纵向偏 43pt），
/// 所以真正的居中要等窗口拿到最终尺寸后再做一次。内容闭包每次开窗只跑一次，因此在
/// 这里请求下一帧居中不会和用户拖动窗口打架。
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
    if let Some(window) = handles.session.welcome_window.get_untracked()
        && focused(&window)
    {
        window.close();
        return;
    }
    if let Some((_, window)) = handles
        .refs
        .worktree_windows
        .get_untracked()
        .into_iter()
        .find(|(_, window)| focused(window))
    {
        window.close();
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
    if let Some(window) = handles.session.welcome_window.get_untracked() {
        window.close();
    }
    for (_, window) in handles.refs.worktree_windows.get_untracked() {
        window.close();
    }
    if let Some(window) = gallery_window.get_untracked() {
        window.close();
    }
    gallery_open.set(false);
    handles.session.close_workspace();
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
    if let Some((_, existing)) = windows
        .get_untracked()
        .into_iter()
        .find(|(open_path, _)| open_path == &path)
    {
        existing.focus();
        return;
    }

    let mut worktree_handles = handles.for_worktree();
    install_worktree_opener(&mut worktree_handles);
    worktree_handles.session.open_path(path.clone());
    let session = worktree_handles.session.clone();
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
                registry.update(|open| open.retain(|(open_path, _)| open_path != &close_path));
            }
        })
        .content(move || {
            crate::app::hotkeys::install(&content_handles, auxiliary_gallery);
            worktree_window_content(content_handles.clone())
        });
    if let Some(window_ref) = nsh_core::element::desktop::open_window(window) {
        session.window.set(Some(window_ref.clone()));
        windows.update(|open| {
            open.retain(|(open_path, _)| open_path != &path);
            open.push((path, window_ref));
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
            crate::features::search::form_overlay(&search, language).into_element()
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
