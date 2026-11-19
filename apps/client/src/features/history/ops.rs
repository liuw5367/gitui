//! 提交历史的操作：选中提交/文件、对比、按 SHA 定位与换筛选条件重载。

use crate::app::session::{Tab, load_tab};
use git::{LogFilter, Repo};
use std::collections::HashSet;

pub(crate) enum CompareFailure {
    IdenticalEndpoints,
    UnrelatedHistories,
    Git,
}

/// 按 SHA 选中一个提交（reflog 的「在历史中显示」）。
///
/// 当前筛选可能把该提交过滤掉了；找不到时先清空关键字再找一次，仍找不到返回
/// `false`，由界面提示而不是悄悄失败。
pub(crate) fn select_by_sha(tab: &mut Tab, sha: &str) -> bool {
    if let Some(index) = tab.commits.iter().position(|commit| commit.sha == sha) {
        select_commit(tab, index);
        return true;
    }
    let compare_shas = tab
        .compare
        .iter()
        .filter_map(|index| tab.commits.get(*index).map(|commit| commit.sha.clone()))
        .collect::<Vec<_>>();
    if let Ok(commits) = Repo::open(tab.path.as_str()).and_then(|repo| repo.log_all()) {
        tab.graph_commits = commits.clone();
        tab.commits = commits;
        tab.filter = LogFilter::default();
        tab.compare = compare_shas
            .iter()
            .filter_map(|sha| tab.commits.iter().position(|commit| commit.sha == *sha))
            .collect();
    }
    if let Some(index) = tab.commits.iter().position(|commit| commit.sha == sha) {
        select_commit(tab, index);
        return true;
    }
    return false;
}

/// 引用导航按 SHA 选择目标。调用方只在目标被已应用条件隐藏时传入空筛选。
pub(crate) fn select_reference_sha(tab: &mut Tab, sha: &str, filter: &LogFilter) -> bool {
    let compare_shas = tab
        .compare
        .iter()
        .filter_map(|index| tab.commits.get(*index).map(|commit| commit.sha.clone()))
        .collect::<Vec<_>>();
    let compare_refs_active = tab.compare_base.is_some() && tab.compare_target.is_some();
    let compare_refs_patch = tab.patch.clone();
    let Ok(repo) = Repo::open(tab.path.as_str()) else {
        return false;
    };
    // 引用导航必须跨所有分支定位。关键字留给表格处理，目标会作为当前选中行滚入视口。
    let mut all_branches_filter = filter.clone();
    all_branches_filter.reference = None;
    all_branches_filter.references.clear();
    all_branches_filter.query.clear();
    let Ok(mut commits) = repo.log_history(&all_branches_filter) else {
        return false;
    };
    let mut graph_commits = repo
        .log_history_graph(&all_branches_filter)
        .unwrap_or_else(|_| commits.clone());
    let mut required_shas = compare_shas.clone();
    required_shas.push(sha.to_string());
    if required_shas
        .iter()
        .any(|required| !commits.iter().any(|commit| commit.sha == *required))
    {
        let Ok(all_commits) = repo.log_all() else {
            return false;
        };
        if required_shas
            .iter()
            .any(|required| !all_commits.iter().any(|commit| commit.sha == *required))
        {
            return false;
        }
        let mut order_by_sha = std::collections::HashMap::with_capacity(all_commits.len());
        for (order, commit) in all_commits.iter().enumerate() {
            order_by_sha.insert(commit.sha.clone(), order);
        }
        for commit in &all_commits {
            if required_shas.iter().any(|required| *required == commit.sha)
                && !commits.iter().any(|existing| existing.sha == commit.sha)
            {
                commits.push(commit.clone());
            }
        }
        graph_commits = all_commits;
        commits.sort_by_key(|commit| order_by_sha.get(&commit.sha).copied().unwrap_or(usize::MAX));
    }
    tab.commits = commits;
    tab.graph_commits = graph_commits;
    tab.compare = compare_shas
        .iter()
        .filter_map(|sha| tab.commits.iter().position(|commit| commit.sha == *sha))
        .collect();
    tab.filter = filter.clone();
    tab.filter.reference = None;
    tab.filter.references.clear();
    if let Some(index) = tab.commits.iter().position(|commit| commit.sha == sha) {
        select_commit(tab, index);
        if tab.compare.len() >= 2 {
            tab.patch = compare_patch(tab).unwrap_or_default();
        } else if compare_refs_active {
            tab.patch = compare_refs_patch;
        }
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
    tab.show_commit_diff = false;
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

/// 显式打开提交补丁时才读取它，普通 commit 选择保持轻量。
pub(crate) fn show_commit_diff(tab: &mut Tab, index: usize) {
    select_commit(tab, index);
    tab.show_commit_diff = true;
}

/// 返回提交列表；引用对比退出后恢复完整历史，并尽量保留当前选中提交。
pub(crate) fn return_to_history(tab: &mut Tab) {
    let selected_sha = tab.selected_commit().map(|commit| commit.sha.clone());
    if tab.compare_base.is_some() || tab.compare_target.is_some() {
        set_compare_refs(tab, None, None);
    }
    tab.comparing = false;
    tab.compare.clear();
    tab.compare_worktree = false;

    let selected = selected_sha
        .and_then(|sha| tab.commits.iter().position(|commit| commit.sha == sha))
        .or_else(|| tab.selected.filter(|index| *index < tab.commits.len()));
    if let Some(index) = selected {
        select_commit(tab, index);
    } else {
        tab.selected = None;
        tab.show_commit_diff = false;
        tab.selected_file = None;
        tab.file_patch = None;
        tab.patch = domain::diff::Patch::default();
        tab.files.clear();
        tab.details = None;
        tab.blame.clear();
        tab.file_history.clear();
    }
}

/// 清除目录树中的文件选择与对应的只读文件信息。
pub(crate) fn select_file_in_tree(tab: &mut Tab, path: Option<&str>) {
    let Some(path) = path else {
        tab.selected_file = None;
        tab.file_patch = None;
        tab.blame.clear();
        tab.file_history.clear();
        return;
    };
    let Some(index) = tab.files.iter().position(|file| file.path == path) else {
        return;
    };
    let Some(commit) = tab.selected_commit() else {
        return;
    };
    let sha = commit.sha.clone();
    tab.selected_file = Some(index);
    tab.file_patch = None;
    tab.blame_path = path.to_string();
    match Repo::open(tab.path.as_str()) {
        Ok(repo) => {
            tab.blame = repo.blame(path).unwrap_or_default();
            tab.file_history = repo.file_history(path, 50).unwrap_or_default();
            tab.error = None;
        }
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    }
    // 保留选中提交身份，避免后续依赖可变列表位置重新解释该文件。
    if tab
        .selected_commit()
        .is_none_or(|selected| selected.sha != sha)
    {
        tab.selected_file = None;
    }
}

/// 当前历史差异的展示补丁。忽略空白只改变 Git 生成的可见差异，调用方仍保留普通补丁
/// 作为复制、导出和统计的数据来源。
pub(crate) fn diff_patch_for_tab(tab: &Tab, ignore_whitespace: bool) -> domain::diff::Patch {
    if tab.compare_range().is_some()
        || (tab.compare_base.is_some() && tab.compare_target.is_some())
        || (tab.compare_worktree && tab.compare_target.is_some())
    {
        return comparison_patch(tab, ignore_whitespace).unwrap_or_default();
    }
    let Ok(repo) = Repo::open(tab.path.as_str()) else {
        return domain::diff::Patch::default();
    };
    if let Some((path, _)) = &tab.file_patch
        && let Some(commit) = tab.selected_commit()
    {
        return repo
            .patch_for_path_with_whitespace(commit.sha.as_str(), path.as_str(), ignore_whitespace)
            .unwrap_or_default();
    }
    if let Some(commit) = tab.selected_commit() {
        return repo
            .patch_with_whitespace(commit.sha.as_str(), ignore_whitespace)
            .unwrap_or_default();
    }
    return domain::diff::Patch::default();
}

/// History 表格在关键字过滤后显示的原始提交下标；当前选中项始终可见，供引用跳转定位。
pub(crate) fn visible_commit_indices(
    commits: &[domain::Commit],
    query: &str,
    selected: Option<usize>,
) -> Vec<usize> {
    let needle = query.to_lowercase();
    return commits
        .iter()
        .enumerate()
        .filter_map(|(index, commit)| {
            let matches = needle.is_empty()
                || commit.message.to_lowercase().contains(&needle)
                || commit.summary.to_lowercase().contains(&needle)
                || commit.author.to_lowercase().contains(&needle)
                || commit.short.contains(&needle)
                || commit
                    .refs
                    .iter()
                    .any(|reference| reference.to_lowercase().contains(&needle));
            (matches || selected == Some(index)).then_some(index)
        })
        .collect();
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
    if index >= tab.commits.len() {
        return;
    }
    if tab.compare_base.is_some() || tab.compare_target.is_some() {
        tab.compare_base = None;
        tab.compare_target = None;
        tab.compare_worktree = false;
    }
    if tab.compare.contains(&index) {
        let Some((first, last)) = tab.compare_range() else {
            tab.compare.clear();
            tab.patch = domain::diff::Patch::default();
            return;
        };
        if first == last {
            tab.compare.clear();
        } else if index == first {
            tab.compare = (first + 1..=last).collect();
        } else if index == last {
            tab.compare = (first..last).collect();
        } else {
            tab.compare = vec![index];
        }
    } else {
        if let Some((first, last)) = tab.compare_range() {
            let low = first.min(index);
            let high = last.max(index);
            tab.compare = (low..=high).collect();
        } else if let Some(first) = tab.compare.first().copied() {
            let low = first.min(index);
            let high = first.max(index);
            tab.compare = (low..=high).collect();
        } else {
            tab.compare = vec![index];
        }
    }
    tab.patch = compare_patch(tab).unwrap_or_default();
}

/// Compare 的实际补丁。提交与引用使用相同端点语义；工作区模式以右端引用为基准。
pub(crate) fn comparison_patch(
    tab: &Tab,
    ignore_whitespace: bool,
) -> Result<domain::diff::Patch, CompareFailure> {
    let repo = Repo::open(tab.path.as_str()).map_err(|_| CompareFailure::Git)?;
    if tab.compare_worktree {
        let target = tab.compare_target.as_deref().ok_or(CompareFailure::Git)?;
        let target = repo
            .resolve_commit(target)
            .map_err(|_| CompareFailure::Git)?;
        return repo
            .diff_worktree_with_whitespace(target.as_str(), ignore_whitespace)
            .map_err(|_| CompareFailure::Git);
    }
    let (base, target) = if let (Some(base), Some(target)) =
        (tab.compare_base.as_deref(), tab.compare_target.as_deref())
    {
        (base.to_string(), target.to_string())
    } else {
        let (newer, older) = tab.compare_range().ok_or(CompareFailure::Git)?;
        let base = tab
            .commits
            .get(older)
            .ok_or(CompareFailure::Git)?
            .sha
            .clone();
        let target = tab
            .commits
            .get(newer)
            .ok_or(CompareFailure::Git)?
            .sha
            .clone();
        (base, target)
    };
    let base = repo
        .resolve_commit(base.as_str())
        .map_err(|_| CompareFailure::Git)?;
    let target = repo
        .resolve_commit(target.as_str())
        .map_err(|_| CompareFailure::Git)?;
    if base == target {
        return Err(CompareFailure::IdenticalEndpoints);
    }
    let base = if tab.compare_three_dot {
        repo.merge_base(base.as_str(), target.as_str())
            .map_err(|_| CompareFailure::Git)?
            .ok_or(CompareFailure::UnrelatedHistories)?
    } else {
        base
    };
    return repo
        .diff_trees_with_whitespace(base.as_str(), target.as_str(), ignore_whitespace)
        .map_err(|_| CompareFailure::Git);
}

/// Commit comparison patch. Fewer than two selected commits means no range.
pub(crate) fn compare_patch(tab: &Tab) -> Option<domain::diff::Patch> {
    tab.compare_range()?;
    return comparison_patch(tab, false).ok();
}

/// 设置对比引用端点；空值可用于先选一个端点，两个端点都空时退出比较。
pub(crate) fn set_compare_refs(tab: &mut Tab, base: Option<String>, target: Option<String>) {
    if base.is_none() && target.is_none() {
        // 任一端为空表示退出对比：把历史恢复到当前分支的完整列表。
        tab.compare_base = None;
        tab.compare_target = None;
        tab.compare_worktree = false;
        let filter = tab.filter.clone();
        let refreshed = load_tab(tab.path.as_str(), &filter);
        tab.commits = refreshed.commits;
        tab.graph_commits = refreshed.graph_commits;
        tab.selected = refreshed.selected;
        tab.patch = refreshed.patch;
        tab.files = refreshed.files;
        tab.details = refreshed.details;
        tab.blame = refreshed.blame;
        tab.file_history = refreshed.file_history;
        tab.selected_file = None;
        tab.file_patch = None;
        tab.compare.clear();
        return;
    }
    let (Some(base), Some(target)) = (base.clone(), target.clone()) else {
        tab.compare_base = base;
        tab.compare_target = target;
        tab.compare.clear();
        tab.show_commit_diff = false;
        tab.patch = domain::diff::Patch::default();
        tab.selected_file = None;
        tab.file_patch = None;
        if tab.compare_target.is_none() {
            tab.compare_worktree = false;
        }
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
            tab.compare.clear();
            tab.show_commit_diff = false;
            tab.compare_worktree = false;
            tab.compare_base = Some(base);
            tab.compare_target = Some(target);
            tab.selected_file = None;
            tab.file_patch = None;
            tab.selected = if tab.commits.is_empty() {
                None
            } else {
                Some(0)
            };
            if let Some(index) = tab.selected {
                select_commit(tab, index);
            } else {
                tab.patch = domain::diff::Patch::default();
                tab.files.clear();
                tab.details = None;
                tab.blame.clear();
                tab.file_history.clear();
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
    next.compare_three_dot = tab.compare_three_dot;

    if tab.compare_base.is_some() && tab.compare_target.is_some() {
        let visible_shas = next
            .commits
            .iter()
            .map(|commit| commit.sha.clone())
            .collect::<HashSet<_>>();
        set_compare_refs(
            &mut next,
            tab.compare_base.clone(),
            tab.compare_target.clone(),
        );
        next.commits
            .retain(|commit| visible_shas.contains(&commit.sha));
        next.selected = if next.commits.is_empty() {
            None
        } else {
            Some(0)
        };
        if let Some(index) = next.selected {
            select_commit(&mut next, index);
        } else {
            next.patch = domain::diff::Patch::default();
            next.files.clear();
            next.details = None;
            next.blame.clear();
            next.file_history.clear();
        }
    } else {
        next.compare_base = tab.compare_base.clone();
        next.compare_target = tab.compare_target.clone();
        next.compare_worktree = tab.compare_worktree;
        let selected_shas = tab
            .compare
            .iter()
            .filter_map(|index| tab.commits.get(*index).map(|commit| commit.sha.clone()))
            .collect::<Vec<_>>();
        next.compare = selected_shas
            .iter()
            .filter_map(|sha| next.commits.iter().position(|commit| commit.sha == *sha))
            .collect();
        if let Some((first, last)) = next.compare_range() {
            next.compare = (first..=last).collect();
            next.patch = compare_patch(&next).unwrap_or_default();
        }
    }
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
        assert_eq!(tab.compare_range(), Some((1, 2)), "移出端点后区间收窄");
        toggle_compare(&mut tab, 1);
        assert_eq!(tab.compare_range(), None, "移出最后一个端点后结束比较");
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
