//! 应用状态与数据装载：把 `gitui-git` 的读取结果整理成界面直接渲染的形状。
//!
//! P1 只读：打开仓库后读取摘要、提交列表、引用集合与首个提交的补丁。Git 命令在
//! 主线程上直接执行——本地只读命令在毫秒级；后续阶段加入写路径与同步时，这些
//! 调用会移到后台任务，接口不变。

use domain::{Commit, DiffStat, StatusEntry};
use git::{Refs, Repo};

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
    /// 选中的提交下标；空仓库时为 `None`。
    pub selected: Option<usize>,
    /// 选中提交的补丁文本。
    pub patch: String,
    /// 选中提交的文件统计。
    pub stats: Vec<DiffStat>,
    /// 打开失败时的错误详情；成功时为 `None`。
    pub error: Option<String>,
}

impl Tab {
    /// 当前选中的提交。
    pub fn selected_commit(&self) -> Option<&Commit> {
        return self
            .selected
            .and_then(|index| self.commits.get(index));
    }

    /// 名字前的短标识：仓库名（与设计板的 `gitui` 标签一致）。
    pub fn label(&self) -> String {
        return if self.name.is_empty() {
            self.path.clone()
        } else {
            self.name.clone()
        };
    }
}

/// 打开 `path` 并装载 P1 需要的全部数据；失败时返回带 `error` 的空标签页，
/// 让界面可以停在错误态而不是崩溃。
pub fn load_tab(path: &str) -> Tab {
    let repo = match Repo::open(path) {
        Ok(repo) => repo,
        Err(error) => {
            return Tab {
                path: path.to_string(),
                name: last_segment(path),
                error: Some(error.detail()),
                ..Tab::default()
            };
        }
    };
    let summary = repo.summary().unwrap_or_default();
    let commits = repo.log(200, "").unwrap_or_default();
    let refs = repo.refs().unwrap_or_default();
    let status = repo.status().unwrap_or_default();
    let mut tab = Tab {
        path: repo.root().display().to_string(),
        name: summary.name,
        branch: summary.branch,
        ahead: summary.ahead,
        behind: summary.behind,
        commits,
        refs,
        status,
        selected: None,
        patch: String::new(),
        stats: Vec::new(),
        error: None,
    };
    if !tab.commits.is_empty() {
        select_commit(&mut tab, 0);
    }
    return tab;
}

/// 选中第 `index` 个提交并读取它的补丁与统计。
pub fn select_commit(tab: &mut Tab, index: usize) {
    if index >= tab.commits.len() {
        return;
    }
    let sha = tab.commits[index].sha.clone();
    tab.selected = Some(index);
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    tab.patch = repo.patch(&sha).unwrap_or_default();
    tab.stats = repo.numstat(&sha).unwrap_or_default();
    tab.error = None;
}

/// 重新读取一个已打开标签页（刷新用）。
pub fn refresh(tab: &Tab) -> Tab {
    let mut next = load_tab(tab.path.as_str());
    if let Some(previous) = tab.selected {
        if previous < next.commits.len() {
            select_commit(&mut next, previous);
        }
    }
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
        let tab = load_tab(&repo.path.display().to_string());
        assert!(tab.error.is_none());
        assert_eq!(tab.branch, "main");
        assert_eq!(tab.commits.len(), 1);
        assert_eq!(tab.selected, Some(0));
        assert_eq!(tab.selected_commit().expect("commit").summary, "first");
        assert!(!tab.refs.branches.is_empty());
        assert!(!tab.patch.is_empty());
    }

    #[test]
    fn loading_a_non_repo_returns_an_error_tab() {
        // 独立目录，且不在任何仓库内部，否则 `git -C` 会向上找到父仓库。
        let path = std::env::temp_dir().join(format!(
            "nsh-app-plain-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&path).expect("dir");
        let tab = load_tab(&path.display().to_string());
        let _ = std::fs::remove_dir_all(&path);
        assert!(tab.error.is_some());
        assert!(tab.commits.is_empty());
        assert_eq!(tab.selected, None);
    }
}
