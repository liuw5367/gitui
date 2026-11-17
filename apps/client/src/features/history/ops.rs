//! 提交历史的操作：选中提交/文件、对比、按 SHA 定位与换筛选条件重载。

use crate::app::session::{Tab, load_tab};
use git::{LogFilter, Repo};

/// 按 SHA 选中一个提交（reflog 的「在历史中显示」）。
///
/// 当前筛选可能把该提交过滤掉了；找不到时先清空关键字再找一次，仍找不到返回
/// `false`，由界面提示而不是悄悄失败。
pub(crate) fn select_by_sha(tab: &mut Tab, sha: &str) -> bool {
    if let Some(index) = tab.commits.iter().position(|commit| commit.sha == sha) {
        select_commit(tab, index);
        return true;
    }
    let filter = LogFilter::default();
    if let Ok(commits) = Repo::open(tab.path.as_str())
        .and_then(|repo| repo.log_filtered(200, &filter).map_err(|error| error))
    {
        tab.commits = commits;
        tab.filter = filter;
    }
    if let Some(index) = tab.commits.iter().position(|commit| commit.sha == sha) {
        select_commit(tab, index);
        return true;
    }
    return false;
}

/// 选中第 `index` 个提交，读取补丁与文件统计，并清掉单文件选择。
pub(crate) fn select_commit(tab: &mut Tab, index: usize) {
    if index >= tab.commits.len() {
        return;
    }
    let sha = tab.commits[index].sha.clone();
    tab.selected = Some(index);
    tab.selected_file = None;
    tab.file_patch = None;
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    tab.patch = repo.patch(&sha).unwrap_or_default();
    tab.files = repo.numstat(&sha).unwrap_or_default();
    tab.details = repo.commit_details(&sha).ok();
    tab.blame.clear();
    tab.file_history.clear();
    tab.error = None;
}

/// 选中某个文件并读取它在当前提交里的补丁。
pub(crate) fn select_file(tab: &mut Tab, index: usize) {
    let Some(commit_index) = tab.selected else {
        return;
    };
    let Some(file) = tab.files.get(index) else {
        return;
    };
    let Some(commit) = tab.commits.get(commit_index) else {
        return;
    };
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    match repo.patch_for_path(&commit.sha, file.path.as_str()) {
        Ok(patch) => {
            tab.selected_file = Some(index);
            tab.file_patch = Some((file.path.clone(), patch));
            // blame 与文件历史只读，失败时留空而不是报错（例如文件在新提交里不存在）。
            tab.blame_path = file.path.clone();
            tab.blame = repo.blame(file.path.as_str()).unwrap_or_default();
            tab.file_history = repo
                .file_history(file.path.as_str(), 50)
                .unwrap_or_default();
        }
        Err(error) => tab.error = Some(error.detail()),
    }
}

/// 把第 `index` 个提交加入或移出对比集合。
pub(crate) fn toggle_compare(tab: &mut Tab, index: usize) {
    if let Some(position) = tab.compare.iter().position(|value| *value == index) {
        tab.compare.remove(position);
    } else {
        tab.compare.push(index);
        tab.compare.sort_unstable();
    }
}

/// 对比模式下要显示的补丁：区间 diff；不足两个提交时为 `None`。
pub(crate) fn compare_patch(tab: &Tab) -> Option<domain::diff::Patch> {
    let (from, to) = tab.compare_range()?;
    let from_sha = tab.commits.get(from)?.sha.clone();
    let to_sha = tab.commits.get(to)?.sha.clone();
    // 区间 diff 包含 `from` 之后的提交到 `to`，因此起点取 `from` 的父提交。
    let repo = Repo::open(tab.path.as_str()).ok()?;
    let range = format!("{}^", from_sha);
    return repo.diff_between(&range, &to_sha).ok();
}

/// 设置对比引用的两端；任一端为空表示关闭对比。
pub(crate) fn set_compare_refs(tab: &mut Tab, base: Option<String>, target: Option<String>) {
    let (Some(base), Some(target)) = (base.clone(), target.clone()) else {
        // 任一端为空表示退出对比：把历史恢复到当前分支的完整列表。
        tab.compare_base = None;
        tab.compare_target = None;
        let filter = tab.filter.clone();
        let refreshed = load_tab(tab.path.as_str(), &filter);
        tab.commits = refreshed.commits;
        tab.selected = refreshed.selected;
        tab.patch = refreshed.patch;
        tab.files = refreshed.files;
        tab.details = refreshed.details;
        tab.blame = refreshed.blame;
        tab.file_history = refreshed.file_history;
        return;
    };
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    match repo.commits_between(base.as_str(), target.as_str()) {
        Ok(commits) => {
            tab.commits = commits;
            tab.compare_base = Some(base);
            tab.compare_target = Some(target);
            tab.selected = if tab.commits.is_empty() {
                None
            } else {
                Some(0)
            };
            if let Some(index) = tab.selected {
                select_commit(tab, index);
            }
            tab.error = None;
        }
        Err(error) => tab.error = Some(error.detail()),
    }
}

/// 换一组筛选条件后重新装载。
pub(crate) fn reload_with_filter(tab: &Tab, filter: &LogFilter) -> Tab {
    let mut next = load_tab(tab.path.as_str(), filter);
    next.comparing = tab.comparing;
    next.sidebar_collapsed = tab.sidebar_collapsed.clone();
    return next;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::load_tab;
    use crate::test_support::TempRepo;
    use git::LogFilter;

    #[test]
    fn compare_range_sorts_and_needs_two_commits() {
        let repo = TempRepo::new("compare");
        let path = repo.path.display().to_string();
        // 先造提交，再装载 tab，避免读一次再整体覆盖。
        // 再提交两次，得到三个提交。
        for message in ["second", "third"] {
            std::fs::write(repo.path.join("a.txt"), format!("{message}\n")).expect("write");
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&repo.path)
                .args(["commit", "-q", "-am", message])
                .status()
                .expect("git");
            assert!(status.success());
        }
        let mut tab = load_tab(&path, &LogFilter::default());
        assert_eq!(tab.commits.len(), 3);

        toggle_compare(&mut tab, 2);
        assert_eq!(tab.compare_range(), None, "一个提交不构成区间");
        toggle_compare(&mut tab, 0);
        assert_eq!(tab.compare_range(), Some((0, 2)));
        assert!(compare_patch(&tab).is_some(), "区间 diff 应可读出");

        toggle_compare(&mut tab, 0);
        assert_eq!(tab.compare_range(), None, "移出后不再成区间");
    }

    #[test]
    fn select_by_sha_finds_the_commit_even_when_filtered_out() {
        let repo = TempRepo::new("select-sha");
        let path = repo.path.display().to_string();
        std::fs::write(repo.path.join("a.txt"), "second\n").expect("write");
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo.path)
            .args(["commit", "-q", "-am", "second"])
            .status()
            .expect("git");
        assert!(status.success());

        let mut tab = load_tab(&path, &LogFilter::default());
        assert_eq!(tab.commits.len(), 2);
        let oldest = tab.commits[1].sha.clone();

        // 用关键字把列表筛到只剩最新一条，再按 SHA 找最旧那条。
        tab.filter.query = "second".to_string();
        let filtered = load_tab(&path, &tab.filter);
        assert_eq!(filtered.commits.len(), 1, "筛选应只剩一条");
        let mut tab = filtered;
        assert!(
            select_by_sha(&mut tab, oldest.as_str()),
            "应能在清筛选后找到"
        );
        assert_eq!(
            tab.commits
                .get(tab.selected.unwrap_or(0))
                .map(|c| c.sha.clone()),
            Some(oldest)
        );

        // 不存在的 SHA 返回 false，不改变选中。
        assert!(!select_by_sha(
            &mut tab,
            "0000000000000000000000000000000000000000"
        ));
    }

    #[test]
    fn compare_refs_narrows_history_to_the_range() {
        let repo = TempRepo::new("cmp");
        let path = repo.path.display().to_string();
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&repo.path)
                .args(args)
                .status()
                .expect("git");
            assert!(status.success(), "git {:?}", args);
        };
        run(&["branch", "base"]);
        std::fs::write(repo.path.join("a.txt"), "two\n").expect("write");
        run(&["commit", "-q", "-am", "second"]);
        std::fs::write(repo.path.join("a.txt"), "three\n").expect("write");
        run(&["commit", "-q", "-am", "third"]);

        let mut tab = load_tab(&path, &LogFilter::default());
        assert_eq!(tab.commits.len(), 3);
        set_compare_refs(&mut tab, Some("base".to_string()), Some("main".to_string()));
        assert_eq!(tab.compare_base.as_deref(), Some("base"));
        assert_eq!(tab.compare_target.as_deref(), Some("main"));
        assert_eq!(tab.commits.len(), 2, "base..main 应只含两个提交");
        assert_eq!(tab.commits[0].summary, "third");
        // 选中项随之更新，详情与文件列表跟着刷新。
        assert!(tab.details.is_some());
        assert!(!tab.files.is_empty());

        // 传空表示退出对比，历史恢复。
        set_compare_refs(&mut tab, None, None);
        assert!(tab.compare_base.is_none());
        assert_eq!(tab.commits.len(), 3);
    }

    #[test]
    fn selecting_a_file_fills_blame_and_history() {
        let repo = TempRepo::new("blame");
        let path = repo.path.display().to_string();
        let mut tab = load_tab(&path, &LogFilter::default());
        assert!(!tab.files.is_empty());
        select_file(&mut tab, 0);
        assert!(!tab.blame.is_empty(), "选中文件后应有 blame 行");
        assert_eq!(tab.blame[0].text, "one");
        assert!(!tab.file_history.is_empty(), "选中文件后应有文件历史");
    }

    #[test]
    fn selecting_a_file_loads_its_patch_only() {
        let repo = TempRepo::new("file");
        // 同时改动两个文件，第二个提交的 numstat 才有两行。
        std::fs::write(repo.path.join("a.txt"), "one\ntwo\n").expect("write");
        std::fs::write(repo.path.join("b.txt"), "x\n").expect("write");
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
            .args(["commit", "-q", "-m", "two files"])
            .status()
            .expect("git");
        assert!(status.success());
        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        assert_eq!(tab.files.len(), 2);
        select_file(&mut tab, 0);
        let (path, patch) = tab.file_patch.clone().expect("file patch");
        assert!(!path.is_empty());
        // 补丁只应包含所选文件，另一个文件不应出现。
        let other = if path == "a.txt" { "b.txt" } else { "a.txt" };
        assert!(!patch.raw.contains(&format!("+++ b/{}", other)));
        assert_eq!(tab.selected_file, Some(0));
    }
}
