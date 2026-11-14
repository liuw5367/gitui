//! 应用状态与数据装载：把 `git` crate 的读取结果整理成界面直接渲染的形状。
//!
//! 只读阶段：打开仓库后读取摘要、提交列表、引用集合、首个提交的补丁与文件统计。
//! 筛选条件、对比选择与单文件补丁都由这里维护，界面只读结果。
//!
//! Git 命令在主线程上执行——本地只读命令在毫秒级；加入写路径与同步后，这些调用
//! 会移到后台任务，接口保持不变。

use domain::{Commit, DiffStat, StatusEntry};
use git::{LogFilter, Refs, Repo};

/// 一个仓库标签页的全部界面状态。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tab {
    pub path: String,
    pub name: String,
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub commits: Vec<Commit>,
    pub refs: Refs,
    pub status: Vec<StatusEntry>,
    /// 历史里出现过的作者（按最近提交顺序）。
    pub authors: Vec<String>,
    /// 当前生效的筛选条件。
    pub filter: LogFilter,
    /// 选中的提交下标；空仓库时为 `None`。
    pub selected: Option<usize>,
    /// 选中的提交的补丁文本。
    pub patch: String,
    /// 选中的提交改动的文件统计。
    pub files: Vec<DiffStat>,
    /// 选中的文件下标（`files`），用于单文件 diff。
    pub selected_file: Option<usize>,
    /// 单文件补丁：(路径, 补丁文本)。
    pub file_patch: Option<(String, String)>,
    /// 多选对比的提交下标（升序无关，按点击顺序）。
    pub compare: Vec<usize>,
    /// 对比模式：开启后点击提交是加入/移出对比集合。
    pub comparing: bool,
    /// 打开失败时的错误详情；成功时为 `None`。
    pub error: Option<String>,
}

impl Tab {
    /// 当前选中的提交。
    pub fn selected_commit(&self) -> Option<&Commit> {
        return self.selected.and_then(|index| self.commits.get(index));
    }

    /// 名字前的短标识：仓库名。
    pub fn label(&self) -> String {
        return if self.name.is_empty() {
            self.path.clone()
        } else {
            self.name.clone()
        };
    }

    /// 对比区间的起止提交：`(from, to)`，按历史位置排序。
    pub fn compare_range(&self) -> Option<(usize, usize)> {
        if self.compare.len() < 2 {
            return None;
        }
        let mut indices = self.compare.clone();
        indices.sort_unstable();
        return Some((indices[0], indices[indices.len() - 1]));
    }
}

/// 打开 `path` 并按 `filter` 装载数据；失败时返回带 `error` 的空标签页。
pub fn load_tab(path: &str, filter: &LogFilter) -> Tab {
    let repo = match Repo::open(path) {
        Ok(repo) => repo,
        Err(error) => {
            return Tab {
                path: path.to_string(),
                name: last_segment(path),
                filter: filter.clone(),
                error: Some(error.detail()),
                ..Tab::default()
            };
        }
    };
    let summary = repo.summary().unwrap_or_default();
    let commits = repo.log_filtered(200, filter).unwrap_or_default();
    let refs = repo.refs().unwrap_or_default();
    let status = repo.status().unwrap_or_default();
    let authors = repo.authors().unwrap_or_default();
    let mut tab = Tab {
        path: repo.root().display().to_string(),
        name: summary.name,
        branch: summary.branch,
        ahead: summary.ahead,
        behind: summary.behind,
        commits,
        refs,
        status,
        authors,
        filter: filter.clone(),
        ..Tab::default()
    };
    if !tab.commits.is_empty() {
        select_commit(&mut tab, 0);
    }
    return tab;
}

/// 选中第 `index` 个提交，读取补丁与文件统计，并清掉单文件选择。
pub fn select_commit(tab: &mut Tab, index: usize) {
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
    tab.error = None;
}

/// 选中某个文件并读取它在当前提交里的补丁。
pub fn select_file(tab: &mut Tab, index: usize) {
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
        }
        Err(error) => tab.error = Some(error.detail()),
    }
}

/// 把第 `index` 个提交加入或移出对比集合。
pub fn toggle_compare(tab: &mut Tab, index: usize) {
    if let Some(position) = tab.compare.iter().position(|value| *value == index) {
        tab.compare.remove(position);
    } else {
        tab.compare.push(index);
        tab.compare.sort_unstable();
    }
}

/// 对比模式下要显示的补丁：区间 diff；不足两个提交时为 `None`。
pub fn compare_patch(tab: &Tab) -> Option<String> {
    let (from, to) = tab.compare_range()?;
    let from_sha = tab.commits.get(from)?.sha.clone();
    let to_sha = tab.commits.get(to)?.sha.clone();
    // 区间 diff 包含 `from` 之后的提交到 `to`，因此起点取 `from` 的父提交。
    let repo = Repo::open(tab.path.as_str()).ok()?;
    return repo.diff_between(&format!("{}^", from_sha), &to_sha).ok();
}

/// 换一组筛选条件后重新装载。
pub fn reload_with_filter(tab: &Tab, filter: &LogFilter) -> Tab {
    let mut next = load_tab(tab.path.as_str(), filter);
    next.comparing = tab.comparing;
    return next;
}

fn last_segment(path: &str) -> String {
    return path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_string();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TempRepo {
        path: PathBuf,
    }

    impl TempRepo {
        fn new(name: &str) -> TempRepo {
            let unique = format!(
                "nsh-app-model-test-{}-{}-{}",
                name,
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_nanos())
                    .unwrap_or(0)
            );
            let path = std::env::temp_dir().join(unique);
            std::fs::create_dir_all(&path).expect("temp dir");
            let run = |args: &[&str]| {
                let status = std::process::Command::new("git")
                    .arg("-C")
                    .arg(&path)
                    .args(args)
                    .status()
                    .expect("git");
                assert!(status.success(), "git {:?}", args);
            };
            run(&["init", "-q", "-b", "main"]);
            run(&["config", "user.name", "Test"]);
            run(&["config", "user.email", "test@example.com"]);
            std::fs::write(path.join("a.txt"), "one\n").expect("write");
            run(&["add", "-A"]);
            run(&["commit", "-q", "-m", "first"]);
            return TempRepo { path };
        }
    }

    impl Drop for TempRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn loading_a_repo_fills_the_tab_and_selects_the_first_commit() {
        let repo = TempRepo::new("load");
        let tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        assert!(tab.error.is_none());
        assert_eq!(tab.branch, "main");
        assert_eq!(tab.commits.len(), 1);
        assert_eq!(tab.selected, Some(0));
        assert_eq!(tab.selected_commit().expect("commit").summary, "first");
        assert!(!tab.refs.branches.is_empty());
        assert!(!tab.patch.is_empty());
        assert_eq!(tab.files.len(), 1);
        assert_eq!(tab.authors, vec!["Test".to_string()]);
    }

    #[test]
    fn loading_a_non_repo_returns_an_error_tab() {
        let path = std::env::temp_dir().join(format!(
            "nsh-app-plain-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&path).expect("dir");
        let tab = load_tab(&path.display().to_string(), &LogFilter::default());
        let _ = std::fs::remove_dir_all(&path);
        assert!(tab.error.is_some());
        assert!(tab.commits.is_empty());
        assert_eq!(tab.selected, None);
    }

    #[test]
    fn keyword_filter_narrows_the_history() {
        let repo = TempRepo::new("filter");
        let path = repo.path.display().to_string();
        let all = load_tab(&path, &LogFilter::default());
        assert_eq!(all.commits.len(), 1);
        let filtered = load_tab(
            &path,
            &LogFilter {
                query: "no-such-summary".to_string(),
                ..LogFilter::default()
            },
        );
        assert!(filtered.commits.is_empty());
        assert_eq!(filtered.selected, None);
    }

    #[test]
    fn compare_range_sorts_and_needs_two_commits() {
        let repo = TempRepo::new("compare");
        let path = repo.path.display().to_string();
        let mut tab = load_tab(&path, &LogFilter::default());
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
        tab = load_tab(&path, &LogFilter::default());
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
        assert!(!patch.contains(&format!("+++ b/{}", other)));
        assert_eq!(tab.selected_file, Some(0));
    }
}
