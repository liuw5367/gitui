//! 跨功能的系统动作：复制到剪贴板、用编辑器打开、在 Finder 中显示、用浏览器打开。
//!
//! 这些动作原先挂在 `Actions` 上、以 `Arc` 传给各页；它们只需要状态栏提示与当前
//! 语言（失败文案按语言取），因此改成显式收参的自由函数，视图直接调用。

use nsh_core::prelude::*;

/// 复制文本到剪贴板：空文本或失败都提示（原 `Actions::copy_text`）。
pub(crate) fn copy_text(
    notice: Signal<Option<String>>,
    language: Signal<SharedString>,
    text: String,
) {
    let language_now = language.get_untracked();
    crate::interactions::copy(
        notice,
        text,
        crate::i18n::t(language_now.as_str(), "toast.copied"),
        crate::i18n::t(language_now.as_str(), "toast.copyFailed"),
    );
}

/// 用系统默认程序打开路径；失败提示（原 `Actions::open_editor`）。
pub(crate) fn open_editor(
    notice: Signal<Option<String>>,
    language: Signal<SharedString>,
    path: String,
) {
    let language_now = language.get_untracked();
    crate::interactions::open_in_editor(
        notice,
        path,
        crate::i18n::t(language_now.as_str(), "toast.openFailed"),
    );
}

/// 在 Finder 中显示路径；失败提示（原 `Actions::reveal_path`）。
pub(crate) fn reveal_path(
    notice: Signal<Option<String>>,
    language: Signal<SharedString>,
    path: String,
) {
    let language_now = language.get_untracked();
    crate::interactions::reveal(
        notice,
        path,
        crate::i18n::t(language_now.as_str(), "toast.revealFailed"),
    );
}

/// 用浏览器打开链接；失败提示（原 `Actions::open_browser`）。
pub(crate) fn open_browser(
    notice: Signal<Option<String>>,
    language: Signal<SharedString>,
    url: String,
) {
    let language_now = language.get_untracked();
    crate::interactions::open_browser(
        notice,
        url,
        crate::i18n::t(language_now.as_str(), "toast.browserFailed"),
    );
}
