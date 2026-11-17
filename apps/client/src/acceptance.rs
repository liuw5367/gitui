//! 验收开关：`tools/shots.sh` 用来直接进到某个页面或浮层截图。
//!
//! **只在 debug 构建里读环境变量**。`cargo build --release` 与 `nsh package` 出来的
//! 应用不读任何 `GITUI_*`，发布二进制不会被环境变量改掉启动行为。
//!
//! 变量清单（`tools/shots.sh` 的头注释也列了一份）：
//! `GITUI_REPO`、`GITUI_MODE`、`GITUI_INSPECTOR`、`GITUI_PALETTE`、`GITUI_GALLERY`、
//! `GITUI_FORM`、`GITUI_SYNC`、`GITUI_SETTINGS`、`GITUI_SETTINGS_SECTION`、`GITUI_RECORD`、
//! `GITUI_PENDING_KEY`、`GITUI_RESTORE_CONFIRM`、`GITUI_NO_WELCOME`、`GITUI_WELCOME_FORM`、
//! `GITUI_WELCOME_STATE`、`GITUI_WELCOME_RECENT`。
//! `GITUI_DIRTY` 由脚本自己改文件，应用不读。

/// `name` 的值是否为 `1`。release 构建里恒为 false。
#[cfg(debug_assertions)]
pub(crate) fn flag(name: &str) -> bool {
    return std::env::var(name)
        .map(|value| value == "1")
        .unwrap_or(false);
}

#[cfg(not(debug_assertions))]
pub(crate) fn flag(_name: &str) -> bool {
    return false;
}

/// `name` 的值，去掉首尾空白；空串当作未设置。release 构建里恒为 None。
#[cfg(debug_assertions)]
pub(crate) fn value(name: &str) -> Option<String> {
    let value = std::env::var(name).ok()?;
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    return Some(value.to_string());
}

#[cfg(not(debug_assertions))]
pub(crate) fn value(_name: &str) -> Option<String> {
    return None;
}

#[cfg(all(test, debug_assertions))]
mod tests {
    //! 只测「未设置的变量读不到」这一条：release 分支由 `cfg` 保证，无值得测的分支。

    use super::{flag, value};

    #[test]
    fn an_unset_variable_reads_as_absent() {
        assert!(!flag("GITUI_ACCEPTANCE_TEST_UNSET"));
        assert_eq!(value("GITUI_ACCEPTANCE_TEST_UNSET"), None);
    }
}
