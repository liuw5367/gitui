//! 窗口与会话级函数：欢迎窗口、会话持久化与标题栏留白。
//!
//! 欢迎页是独立窗口（`id = "welcome"`），打开仓库后随 `welcome_open` 信号关闭；
//! `persist` 把打开的标签页写回会话，供下次启动恢复。

use crate::app::handles::AppHandles;
use crate::app::session::Tab;
use crate::brand;
use crate::features;
use nsh_core::desktop::WindowAttributes;
use nsh_core::element::desktop::title_bar;
use nsh_core::prelude::*;

/// 打开欢迎页窗口（独立窗口，主窗口不受影响）。
///
/// 同一 id 只建一次；`open` 绑定到信号，从欢迎页打开仓库后窗口自动关闭。
pub(crate) fn open_welcome_window(handles: &AppHandles) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static OPENED: AtomicBool = AtomicBool::new(false);
    if OPENED.swap(true, Ordering::SeqCst) {
        return;
    }
    let cmd = handles.welcome.clone();
    let open = cmd.session.welcome_open;
    let mut attributes = WindowAttributes::default();
    attributes.min_size = Some(Some(size(Points(1000.0), Points(560.0))));
    let welcome = nsh_core::element::desktop::window()
        .id("welcome")
        .title(brand::PRODUCT_NAME)
        .default_size(size(Points(1040.0), Points(690.0)))
        .attributes(attributes)
        .open(open)
        .on_open_change(move |event| {
            if !event.open {
                open.set(false);
            }
        })
        .content(move || {
            return features::welcome::view::welcome(&cmd.state, &cmd);
        });
    let _ = nsh_core::element::desktop::open_window(welcome);
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
