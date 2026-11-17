//! 交互基座：确认框、剪贴板、外部打开与短暂提示。
//!
//! 危险动作统一走 [`confirm`]：标题说明动作，正文写明对象与后果，主按钮在
//! `destructive` 时用红色。复制与外部打开都用 0.5.0 的 prelude API，失败落到
//! 状态栏提示而不是静默。

use crate::ui;
use nsh_core::prelude::*;
use std::time::Duration;

/// 提示在状态栏停留的时长。
pub(crate) const NOTICE_MILLIS: u64 = 2400;

/// 确认框的结果是否等于期望动作。
///
/// 被 Esc 或点背景关闭时 `action` 为 `None`，与「点了别的按钮」一样算未确认。
pub(crate) fn confirmed(action: Option<&str>, expected: &str) -> bool {
    return action == Some(expected);
}

/// 弹一次确认框；用户点了主按钮返回 `true`。
///
/// `action_id` 是这个调用自己的标识，回调只认它，避免与其它对话框串味。
pub(crate) fn confirm(
    title: String,
    message: String,
    action_id: &'static str,
    action_label: String,
    cancel_label: String,
    destructive: bool,
) -> impl std::future::Future<Output = bool> + 'static {
    let pending = ui::dialog::alert_with_actions(
        title,
        message,
        vec![(action_id, action_label)],
        if destructive { Some(action_id) } else { None },
        cancel_label,
    );
    return async move {
        return match pending.await {
            Ok(result) => confirmed(result.action.as_deref(), action_id),
            Err(_) => false,
        };
    };
}

/// 弹一次说明框；只有一个关闭按钮，用于能力尚未接通时的真实状态说明。
pub(crate) fn alert(title: String, message: String, close_label: String) {
    spawn(async move {
        let _ = ui::dialog::notice(title, message, close_label).await;
    });
}

/// 在状态栏显示一条提示，到点自动清除。
pub(crate) fn notify(notice: Signal<Option<String>>, message: impl Into<String>) {
    let message = message.into();
    notice.set(Some(message));
    spawn(async move {
        timer(Duration::from_millis(NOTICE_MILLIS)).await;
        notice.set(None);
    });
}

/// 复制文本到系统剪贴板，成功后提示。
pub(crate) fn copy(
    notice: Signal<Option<String>>,
    text: impl Into<String>,
    copied_notice: String,
    failed_notice: String,
) {
    let text = text.into();
    if text.is_empty() {
        notify(notice, failed_notice);
        return;
    }
    set_clipboard(text);
    notify(notice, copied_notice);
}

/// 用系统默认程序打开文件；失败提示。
pub(crate) fn open_in_editor(
    notice: Signal<Option<String>>,
    path: impl Into<std::path::PathBuf>,
    failed_notice: String,
) {
    let path = path.into();
    spawn(async move {
        if let Err(error) = open_path(path).await {
            notify(
                notice,
                format!("{}: {}", failed_notice, first_line(error.as_str())),
            );
        }
    });
}

/// 在 Finder 中显示文件；失败提示。
pub(crate) fn reveal(
    notice: Signal<Option<String>>,
    path: impl Into<std::path::PathBuf>,
    failed_notice: String,
) {
    let path = path.into();
    spawn(async move {
        if let Err(error) = reveal_path(path).await {
            notify(
                notice,
                format!("{}: {}", failed_notice, first_line(error.as_str())),
            );
        }
    });
}

/// 用浏览器打开链接；失败提示。
pub(crate) fn open_browser(
    notice: Signal<Option<String>>,
    url: impl Into<String>,
    failed_notice: String,
) {
    let url = url.into();
    spawn(async move {
        if !open_url(url).await {
            notify(notice, failed_notice);
        }
    });
}

/// 只保留第一行，避免把 git 的整段输出塞进状态栏。
fn first_line(text: &str) -> String {
    return text.lines().next().unwrap_or("").trim().to_string();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirm_only_accepts_its_own_action() {
        assert!(confirmed(Some("discard"), "discard"));
        assert!(!confirmed(Some("cancel"), "discard"));
        assert!(!confirmed(None, "discard"));
    }

    #[test]
    fn first_line_trims_and_keeps_one_line() {
        assert_eq!(first_line("  bad path \nmore"), "bad path");
        assert_eq!(first_line(""), "");
    }
}
