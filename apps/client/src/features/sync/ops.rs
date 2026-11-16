//! 同步的操作：按选项执行 fetch/pull/push 与清掉同步失败状态。

use crate::app::session::{Sync, SyncOptions, Tab, load_tab};
use crate::features::changes::ops::refresh_changes;
use git::Repo;

/// 清掉同步失败的状态（重试或用户主动关闭横幅）。
pub(crate) fn clear_sync_fault(tab: &mut Tab) {
    tab.sync_fault = None;
}

pub(crate) fn sync_with(tab: &mut Tab, kind: Sync, options: &SyncOptions) {
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    let remote = repo
        .refs()
        .ok()
        .and_then(|refs| refs.remotes.first().cloned())
        .unwrap_or_else(|| "origin".to_string());
    let result = match kind {
        Sync::Fetch => repo.fetch_with(remote.as_str(), &options.fetch),
        Sync::Pull => repo.pull_with(&options.pull),
        Sync::Push => repo.push_with(Some(remote.as_str()), &options.push),
    };
    match result {
        Ok(message) => {
            tab.sync_failed = false;
            tab.sync_fault = None;
            tab.sync_message = Some(if message.trim().is_empty() {
                format!("{} 完成", kind.id())
            } else {
                message
            });
            refresh_changes(tab);
            // 领先/落后与历史都要跟着刷新。
            let filter = tab.filter.clone();
            let refreshed = load_tab(tab.path.as_str(), &filter);
            tab.branch = refreshed.branch;
            tab.ahead = refreshed.ahead;
            tab.behind = refreshed.behind;
            tab.commits = refreshed.commits;
            tab.refs = refreshed.refs;
            tab.changelists = refreshed.changelists;
        }
        Err(error) => {
            tab.sync_failed = true;
            tab.sync_fault = git::classify_network_error(error.detail().as_str());
            tab.sync_message = Some(error.detail());
            refresh_changes(tab);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ops::{stage_all_or_none, sync};
    use crate::app::session::{Sync, load_tab};
    use crate::features::commit::ops::commit_staged;
    use git::LogFilter;

    #[test]
    fn sync_failures_are_classified_for_the_banner() {
        // 直接验证归类函数接到界面上用的那条路径。
        assert_eq!(
            git::classify_network_error("fatal: Authentication failed for 'https://x/'"),
            Some(domain::NetworkFault::AuthRequired)
        );
        let mut tab = Tab::default();
        assert_eq!(tab.sync_fault, None);
        tab.sync_fault = Some(domain::NetworkFault::Offline);
        clear_sync_fault(&mut tab);
        assert_eq!(tab.sync_fault, None);
    }

    #[test]
    fn sync_pushes_and_reports_clean_state() {
        // 建 bare 远程并让工作仓库跟踪它。
        let base = std::env::temp_dir().join(format!(
            "nsh-app-sync-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&base).expect("dir");
        let remote = base.join("remote.git");
        let repo_dir = base.join("local");
        let run = |directory: &std::path::Path, args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(directory)
                .args(args)
                .status()
                .expect("git");
            assert!(status.success(), "git {:?}", args);
        };
        run(&base, &["init", "-q", "--bare", remote.to_str().expect("utf8")]);
        run(&base, &["init", "-q", "-b", "main", repo_dir.to_str().expect("utf8")]);
        run(&repo_dir, &["config", "user.name", "Test"]);
        run(&repo_dir, &["config", "user.email", "test@example.com"]);
        std::fs::write(repo_dir.join("a.txt"), "one\n").expect("write");
        run(&repo_dir, &["add", "-A"]);
        run(&repo_dir, &["commit", "-q", "-m", "base"]);
        run(
            &repo_dir,
            &["remote", "add", "origin", remote.to_str().expect("utf8")],
        );
        run(&repo_dir, &["push", "-q", "-u", "origin", "main"]);

        let mut tab = load_tab(&repo_dir.display().to_string(), &LogFilter::default());
        assert_eq!((tab.ahead, tab.behind), (0, 0));

        // 本地再提交一次，推送后领先数回到 0，并留下同步信息。
        std::fs::write(repo_dir.join("a.txt"), "two\n").expect("write");
        stage_all_or_none(&mut tab, true);
        tab.summary = "second".to_string();
        commit_staged(&mut tab);
        assert_eq!(tab.ahead, 1, "提交后应领先远程一个提交");

        sync(&mut tab, Sync::Push);
        assert!(!tab.sync_failed, "推送应成功：{:?}", tab.sync_message);
        assert!(tab.sync_message.is_some());
        assert_eq!(tab.ahead, 0, "推送后不再领先");

        sync(&mut tab, Sync::Fetch);
        assert!(!tab.sync_failed);

        let _ = std::fs::remove_dir_all(&base);
    }
}
