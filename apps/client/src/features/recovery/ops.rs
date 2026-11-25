//! 恢复的操作：检出提交、拣选、回滚、重置与按 reflog 撤销。

use crate::app::session::{Tab, reload_after_history_change};
use crate::features::changes::ops::refresh_changes;

/// 把某个引用合并进当前分支。
/// 切到某个提交（detached HEAD）。调用方先确认，因为离开当前分支容易误提交。
pub(crate) fn checkout_commit(tab: &mut Tab, sha: &str) -> Result<String, String> {
    let repo = tab.open_repo().map_err(|error| error.detail())?;
    repo.checkout_branch(sha).map_err(|error| error.detail())?;
    return reload_after_history_change(tab);
}

/// 拣选一个提交到当前分支；冲突时进入冲突态，由冲突界面接手。
pub(crate) fn cherry_pick(tab: &mut Tab, sha: &str) -> Result<String, String> {
    let repo = tab.open_repo().map_err(|error| error.detail())?;
    let outcome = repo.cherry_pick(sha);
    let summary = reload_after_history_change(tab)?;
    match outcome {
        Ok(()) => return Ok(summary),
        Err(error) => return Err(error.detail()),
    }
}

/// 反向提交一个提交；冲突处理同上。
pub(crate) fn revert_commit(tab: &mut Tab, sha: &str) -> Result<String, String> {
    let repo = tab.open_repo().map_err(|error| error.detail())?;
    let outcome = repo.revert(sha);
    let summary = reload_after_history_change(tab)?;
    match outcome {
        Ok(()) => return Ok(summary),
        Err(error) => return Err(error.detail()),
    }
}

/// 把当前分支重置到某个提交。
pub(crate) fn reset_to(tab: &mut Tab, sha: &str, mode: git::ResetMode) -> Result<(), String> {
    let repo = tab.open_repo().map_err(|error| error.detail())?;
    repo.reset_to(sha, mode).map_err(|error| error.detail())?;
    refresh_changes(tab);
    let filter = tab.filter.clone();
    let refreshed = crate::app::session::load_tab_with_executable(
        tab.path.as_str(),
        &filter,
        &tab.git_executable,
    );
    tab.commits = refreshed.commits;
    tab.branch = refreshed.branch;
    tab.head = refreshed.head;
    tab.head_oid = refreshed.head_oid;
    tab.ahead = refreshed.ahead;
    tab.behind = refreshed.behind;
    return Ok(());
}

/// 把当前分支重置到某条 reflog 记录（不可撤销，调用方需确认）。
pub(crate) fn undo_to_reflog(tab: &mut Tab, sha: &str) {
    let repo = match tab.open_repo() {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    match repo.reset_hard(sha) {
        Ok(()) => {
            refresh_changes(tab);
            let filter = tab.filter.clone();
            let refreshed = crate::app::session::load_tab_with_executable(
                tab.path.as_str(),
                &filter,
                &tab.git_executable,
            );
            tab.commits = refreshed.commits;
            tab.branch = refreshed.branch;
            tab.head = refreshed.head;
            tab.head_oid = refreshed.head_oid;
            tab.ahead = refreshed.ahead;
            tab.behind = refreshed.behind;
        }
        Err(error) => tab.error = Some(error.detail()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ops::stage_all_or_none;
    use crate::app::session::load_tab;
    use crate::features::commit::ops::commit_staged;
    use crate::test_support::TempRepo;
    use git::LogFilter;

    #[test]
    fn cherry_pick_and_revert_round_trip_through_the_model() {
        let repo = TempRepo::new("pick-revert");
        let path = repo.path.display().to_string();
        std::fs::write(repo.path.join("b.txt"), "bee\n").expect("write");
        let mut tab = load_tab(&path, &LogFilter::default());
        stage_all_or_none(&mut tab, true);
        tab.summary = "add b".to_string();
        commit_staged(&mut tab);
        let picked = tab.commits[0].sha.clone();

        // 回到初始提交，再拣选回来。
        let base = tab.commits[1].sha.clone();
        reset_to(&mut tab, base.as_str(), git::ResetMode::Hard).expect("reset");
        assert!(!repo.path.join("b.txt").exists());

        cherry_pick(&mut tab, picked.as_str()).expect("cherry-pick");
        assert!(repo.path.join("b.txt").exists(), "拣选应带过来文件");
        assert_eq!(tab.commits[0].summary, "add b");

        let picked = tab.commits[0].sha.clone();
        revert_commit(&mut tab, picked.as_str()).expect("revert");
        assert!(!repo.path.join("b.txt").exists(), "回滚应删掉文件");
        assert!(tab.commits[0].summary.contains("Revert"));
    }

    #[test]
    fn reflog_can_be_loaded_on_demand_and_undo_resets_the_branch() {
        let repo = TempRepo::new("reflog");
        let path = repo.path.display().to_string();
        std::fs::write(repo.path.join("a.txt"), "two\n").expect("write");
        let mut tab = load_tab(&path, &LogFilter::default());
        stage_all_or_none(&mut tab, true);
        tab.summary = "second".to_string();
        commit_staged(&mut tab);

        // 提交后 reflog 顶部应是这次 commit。
        let mut tab = load_tab(&path, &LogFilter::default());
        let reflog = git::Repo::open(path.as_str())
            .expect("repo")
            .reflog(100)
            .expect("reflog");
        assert!(!reflog.is_empty());
        assert_eq!(reflog[0].action, "commit");
        let newest = tab.commits[0].sha.clone();

        // 撤销到 reflog 里第一条之外的记录（即上一个提交）。
        let target_sha = reflog
            .iter()
            .find(|entry| entry.sha != newest)
            .expect("older entry")
            .sha
            .clone();
        undo_to_reflog(&mut tab, target_sha.as_str());

        assert_eq!(tab.commits[0].sha, target_sha, "分支应回到目标提交");
        let reflog_after = git::Repo::open(path.as_str())
            .expect("repo")
            .reflog(3)
            .expect("reflog after undo");
        assert!(
            !reflog_after.is_empty() && reflog_after[0].action == "reset",
            "撤销本身也应留下 reflog 记录"
        );
    }
}
