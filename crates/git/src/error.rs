//! 本模块负责 Git 错误：把 Git 的英文输出措辞归类成 `domain` 的失败形态，
//! 并提供其余模块共用的错误构造助手。

use domain::{GitError, NetworkFault};

/// 网络类失败的细分原因（设计板 15/16 的 offline 与 auth 状态）。
///
/// 只在 Git 的输出里出现已知特征串时才归类，猜不出来就返回 `None`，由界面显示
/// 原始输出，不做假判断。
pub fn classify_network_error(detail: &str) -> Option<NetworkFault> {
    let lowered = detail.to_lowercase();
    let auth_markers = [
        "authentication failed",
        "could not read username",
        "could not read password",
        "permission denied (publickey)",
        "invalid username or password",
        "terminal prompts disabled",
        "403 forbidden",
        "401 unauthorized",
    ];
    if auth_markers.iter().any(|marker| lowered.contains(marker)) {
        return Some(NetworkFault::AuthRequired);
    }
    let offline_markers = [
        "could not resolve host",
        "network is unreachable",
        "connection timed out",
        "connection refused",
        "operation timed out",
        "failed to connect",
        "no route to host",
    ];
    if offline_markers
        .iter()
        .any(|marker| lowered.contains(marker))
    {
        return Some(NetworkFault::Offline);
    }
    let rejection_markers = ["non-fast-forward", "rejected", "fetch first"];
    if rejection_markers
        .iter()
        .any(|marker| lowered.contains(marker))
    {
        return Some(NetworkFault::Rejected);
    }
    return None;
}

/// 改写类操作的错误：统一走 `CommandFailed`，界面只需展示 `detail`。
pub(crate) fn rewrite_error(detail: String) -> GitError {
    return GitError::CommandFailed {
        program: "git".to_string(),
        args: "rebase -i".to_string(),
        code: None,
        detail,
    };
}

#[cfg(test)]
mod network_tests {
    use super::*;

    #[test]
    fn auth_failures_are_classified() {
        for detail in [
            "fatal: Authentication failed for 'https://example.com/a.git/'",
            "fatal: could not read Username for 'https://example.com': terminal prompts disabled",
            "git@example.com: Permission denied (publickey).",
            "remote: Invalid username or password.",
        ] {
            assert_eq!(
                classify_network_error(detail),
                Some(NetworkFault::AuthRequired),
                "{detail}"
            );
        }
    }

    #[test]
    fn offline_failures_are_classified() {
        for detail in [
            "fatal: unable to access 'https://example.com/': Could not resolve host: example.com",
            "ssh: connect to host example.com port 22: Connection refused",
            "fatal: Connection timed out",
        ] {
            assert_eq!(
                classify_network_error(detail),
                Some(NetworkFault::Offline),
                "{detail}"
            );
        }
    }

    #[test]
    fn rejections_are_classified_and_unknowns_stay_unknown() {
        assert_eq!(
            classify_network_error("! [rejected]        main -> main (non-fast-forward)"),
            Some(NetworkFault::Rejected)
        );
        assert_eq!(classify_network_error("fatal: 不是 git 仓库"), None);
        assert_eq!(classify_network_error(""), None);
    }
}
