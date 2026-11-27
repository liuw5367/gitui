//! 提交的操作：提交暂存区、创建首个提交、改写说明与合并提交。

use crate::app::session::{CommitFeedbackKind, Tab, reload_after_history_change};
use crate::features::changes::ops::refresh_changes;

/// 空仓库里创建第一个提交（`--allow-empty`，只为了让分支有起点）。
pub(crate) fn create_initial_commit(tab: &mut Tab, summary: &str) -> Result<(), String> {
    let repo = tab.open_repo().map_err(|error| error.detail())?;
    let options = git::CommitOptions {
        allow_empty: true,
        ..git::CommitOptions::default()
    };
    repo.commit_with(summary, "", &options)
        .map_err(|error| error.detail())?;
    let filter = tab.filter.clone();
    let refreshed = crate::app::session::load_tab_with_executable(
        tab.path.as_str(),
        &filter,
        &tab.git_executable,
    );
    tab.commits = refreshed.commits;
    tab.head = refreshed.head;
    tab.branch = refreshed.branch;
    tab.refs = refreshed.refs;
    tab.remote_urls = refreshed.remote_urls;
    tab.status = refreshed.status;
    tab.changes = refreshed.changes;
    return Ok(());
}

/// 改写提交说明。
pub(crate) fn reword_commit(tab: &mut Tab, sha: &str, message: &str) -> Result<(), String> {
    let repo = tab.open_repo().map_err(|error| error.detail())?;
    repo.reword_commit(sha, message)
        .map_err(|error| error.detail())?;
    reload_after_history_change(tab)?;
    return Ok(());
}

/// 合并连续的一段提交。
pub(crate) fn squash_commits(
    tab: &mut Tab,
    from: &str,
    to: &str,
    message: &str,
) -> Result<(), String> {
    let repo = tab.open_repo().map_err(|error| error.detail())?;
    repo.squash_commits(from, to, message)
        .map_err(|error| error.detail())?;
    reload_after_history_change(tab)?;
    return Ok(());
}

/// 当前是否能改写历史；`Err` 里是给用户看的原因。
pub(crate) fn rewrite_blocker(tab: &Tab) -> Option<String> {
    let repo = match tab.open_repo() {
        Ok(repo) => repo,
        Err(error) => return Some(error.detail()),
    };
    return repo.can_rewrite().err();
}

/// 提交暂存区；无论成败都保留钩子输出。
pub(crate) fn commit_staged(tab: &mut Tab) {
    return commit_staged_with(tab, false, crate::i18n::DEFAULT_LANGUAGE);
}

/// 提交暂存区；`no_verify` 为真时跳过钩子（界面走强确认）。
///
/// 面板上的「改写上一条」、「sign-off」与「no-verify」来自 `Tab` 的开关，成功后归位，
/// 避免下一次提交意外沿用。
pub(crate) fn commit_staged_with(tab: &mut Tab, no_verify: bool, language: &str) {
    tab.commit_feedback_progress = None;
    let summary = tab.summary.trim().to_string();
    if summary.is_empty() {
        tab.hook_log = Some(crate::i18n::t(language, "commit.summaryRequired"));
        tab.hook_ok = false;
        tab.commit_feedback_kind = CommitFeedbackKind::Commit;
        return;
    }
    let options = git::CommitOptions {
        amend: tab.amend,
        signoff: tab.signoff,
        no_verify,
        allow_empty: false,
    };
    let repo = match tab.open_repo() {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            tab.hook_ok = false;
            tab.hook_log = Some(error.detail());
            tab.commit_feedback_kind = CommitFeedbackKind::Commit;
            return;
        }
    };
    match repo.commit_with(summary.as_str(), tab.description.as_str(), &options) {
        Ok(outcome) => {
            tab.hook_ok = true;
            tab.commit_feedback_kind = CommitFeedbackKind::Hooks;
            tab.hook_log = Some(if outcome.log.is_empty() {
                crate::i18n::with_arguments(
                    language,
                    "commit.completedSha",
                    &[("sha", outcome.sha.chars().take(7).collect())],
                )
            } else {
                outcome.log
            });
            tab.summary.clear();
            tab.description.clear();
            tab.amend = false;
            tab.signoff = false;
            tab.no_verify = false;
            refresh_changes(tab);
            // 提交后历史新增一条，重新读取以显示。
            let filter = tab.filter.clone();
            let refreshed = crate::app::session::load_tab_with_executable(
                tab.path.as_str(),
                &filter,
                &tab.git_executable,
            );
            tab.commits = refreshed.commits;
            tab.authors = refreshed.authors;
            tab.refs = refreshed.refs;
            tab.branch = refreshed.branch;
            tab.ahead = refreshed.ahead;
            tab.behind = refreshed.behind;
        }
        Err(error) => {
            tab.hook_ok = false;
            tab.commit_feedback_kind = if matches!(&error, domain::GitError::HookFailed { .. }) {
                CommitFeedbackKind::Hooks
            } else {
                CommitFeedbackKind::Commit
            };
            tab.hook_log = Some(error.detail());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ops::stage_all_or_none;
    use crate::app::session::load_tab;
    use crate::test_support::TempRepo;
    use git::LogFilter;

    #[test]
    fn commit_staged_creates_a_commit_and_clears_the_form() {
        let repo = TempRepo::new("commit");
        let path = repo.path.display().to_string();
        std::fs::write(repo.path.join("a.txt"), "second\n").expect("write");
        let mut tab = load_tab(&path, &LogFilter::default());
        stage_all_or_none(&mut tab, true);
        assert!(!tab.changes.is_empty());

        tab.summary = "Working tree and commit".to_string();
        tab.description = "Details.".to_string();
        commit_staged(&mut tab);

        assert!(tab.hook_ok, "提交应成功：{:?}", tab.hook_log);
        assert!(tab.summary.is_empty());
        assert!(tab.description.is_empty());
        assert!(tab.changes.is_empty(), "提交后工作区应干净");
        assert_eq!(
            tab.commits.first().map(|commit| commit.summary.as_str()),
            Some("Working tree and commit")
        );
    }

    #[test]
    fn amend_reuses_the_last_commit_and_resets_the_switch() {
        let repo = TempRepo::new("amend");
        let path = repo.path.display().to_string();
        let mut tab = load_tab(&path, &LogFilter::default());
        let before = tab.commits.len();

        // 只改说明、不动内容：amend 不需要暂存内容。
        tab.summary = "Rewritten".to_string();
        tab.amend = true;
        commit_staged(&mut tab);

        assert!(tab.hook_ok, "改写应成功：{:?}", tab.hook_log);
        assert_eq!(tab.commits.len(), before, "改写不新增提交");
        assert_eq!(tab.commits[0].summary, "Rewritten");
        assert!(!tab.amend, "开关成功后应归位");
    }

    #[test]
    fn commit_without_hooks_bypasses_a_failing_hook() {
        let repo = TempRepo::new("skip-hooks");
        let path = repo.path.display().to_string();
        let hook = repo.path.join(".git/hooks/pre-commit");
        std::fs::write(&hook, "#!/bin/sh\nexit 1\n").expect("hook");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        }
        std::fs::write(repo.path.join("a.txt"), "changed\n").expect("write");
        let mut tab = load_tab(&path, &LogFilter::default());
        stage_all_or_none(&mut tab, true);
        tab.summary = "Blocked".to_string();

        commit_staged(&mut tab);
        assert!(!tab.hook_ok, "钩子应拦住普通提交");

        tab.no_verify = true;
        commit_staged_with(&mut tab, true, crate::i18n::DEFAULT_LANGUAGE);
        assert!(tab.hook_ok, "跳过钩子应成功：{:?}", tab.hook_log);
        assert_eq!(tab.commits[0].summary, "Blocked");
        assert!(!tab.no_verify, "成功后 no-verify 开关应归位");
    }

    #[test]
    fn empty_repository_is_detected_and_can_get_a_first_commit() {
        let dir = std::env::temp_dir().join(format!(
            "nsh-app-unborn-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("dir");
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["init", "-q", "-b", "main"])
            .status()
            .expect("git init");
        assert!(status.success());

        let mut tab = load_tab(&dir.display().to_string(), &LogFilter::default());
        assert!(
            matches!(tab.head, git::HeadState::Unborn { ref branch } if branch == "main"),
            "新仓库应识别为未提交状态：{:?}",
            tab.head
        );
        assert!(tab.commits.is_empty());

        create_initial_commit(&mut tab, "Initial commit").expect("initial commit");
        assert_eq!(
            tab.head,
            git::HeadState::Branch("main".to_string()),
            "提交后应回到分支状态"
        );
        assert_eq!(tab.commits.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rewrite_wrappers_reword_and_squash_through_the_model() {
        let repo = TempRepo::new("rewrite");
        let path = repo.path.display().to_string();
        // 造三个提交。
        for (file, message) in [("b.txt", "second"), ("c.txt", "third")] {
            std::fs::write(repo.path.join(file), "x\n").expect("write");
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&repo.path)
                .args(["add", "-A"])
                .status()
                .expect("git");
            assert!(status.success());
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&repo.path)
                .args(["commit", "-q", "-m", message])
                .status()
                .expect("git");
            assert!(status.success());
        }
        let mut tab = load_tab(&path, &LogFilter::default());
        assert_eq!(tab.commits.len(), 3);
        assert!(rewrite_blocker(&tab).is_none(), "干净工作区应可改写");

        // 改写中间那条的说明。
        let middle = tab.commits[1].sha.clone();
        reword_commit(&mut tab, middle.as_str(), "second (edited)").expect("reword");
        assert_eq!(tab.commits[1].summary, "second (edited)");
        assert_eq!(tab.commits.len(), 3, "改写不改变提交数");

        // 合并最近两个。
        let newest = tab.commits[0].sha.clone();
        let older = tab.commits[1].sha.clone();
        squash_commits(&mut tab, older.as_str(), newest.as_str(), "combined").expect("squash");
        assert_eq!(tab.commits.len(), 2);
        assert_eq!(tab.commits[0].summary, "combined");
    }

    #[test]
    fn rewrite_blocker_reports_a_dirty_worktree() {
        let repo = TempRepo::new("rewrite-dirty");
        std::fs::write(repo.path.join("a.txt"), "dirty\n").expect("write");
        let tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        let blocker = rewrite_blocker(&tab).expect("脏工作区应被拦下");
        assert!(blocker.contains("未提交"), "原因应可读：{blocker}");
    }

    #[test]
    fn commit_without_summary_reports_an_error() {
        let repo = TempRepo::new("commit-empty");
        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        std::fs::write(repo.path.join("a.txt"), "changed\n").expect("write");
        stage_all_or_none(&mut tab, true);
        commit_staged(&mut tab);
        assert!(!tab.hook_ok);
        assert!(tab.hook_log.is_some());
    }
}
