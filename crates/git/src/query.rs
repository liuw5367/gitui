//! 本模块负责仓库句柄与只读查询：打开、根路径、摘要、HEAD 形态、配置与引用集合。

use domain::{GitError, RepoSummary};
use std::path::{Path, PathBuf};

use crate::exec::run_git;
use crate::parse::parse_track;

/// 一个已经校验过的仓库。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repo {
    pub(crate) root: PathBuf,
}

impl Repo {
    /// 校验 `path` 是工作树并在 `path` 里运行 Git。
    ///
    /// `path` 不存在或不可读时返回 [`GitError::Unreadable`]；存在但不是仓库时
    /// 返回 [`GitError::NotARepository`]。
    pub fn open(path: &str) -> Result<Repo, GitError> {
        let path = Path::new(path);
        if !path.exists() {
            return Err(GitError::Unreadable {
                path: path.display().to_string(),
                reason: "路径不存在".to_string(),
            });
        }
        // 在非仓库目录里 `rev-parse --show-toplevel` 以非零退出，这是探测的
        // 正常结果；只有无法启动 Git 才向上传递。
        let probe = match run_git(path, &["rev-parse", "--show-toplevel"]) {
            Ok(output) => output,
            Err(GitError::CommandFailed { .. }) => {
                return Err(GitError::NotARepository {
                    path: path.display().to_string(),
                });
            }
            Err(error) => return Err(error),
        };
        let root = String::from_utf8_lossy(&probe).trim().to_string();
        if root.is_empty() {
            return Err(GitError::NotARepository {
                path: path.display().to_string(),
            });
        }
        return Ok(Repo {
            root: PathBuf::from(root),
        });
    }

    /// 工作树根目录的绝对路径。
    pub fn root(&self) -> &Path {
        return &self.root;
    }

    /// 仓库名（根目录的最后一段）。
    pub fn name(&self) -> String {
        return self
            .root
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| self.root.display().to_string());
    }

    /// 当前分支、与上游的领先/落后数量。
    pub fn summary(&self) -> Result<RepoSummary, GitError> {
        let branch = self.abbrev_branch()?;
        let (ahead, behind) = self.ahead_behind();
        return Ok(RepoSummary {
            path: self.root.display().to_string(),
            name: self.name(),
            branch,
            ahead,
            behind,
        });
    }

    /// 当前分支名；detached HEAD 时返回 `HEAD`。
    ///
    /// 空仓库还没有提交，`rev-parse --abbrev-ref HEAD` 会失败；这时用
    /// `symbolic-ref` 读出尚未落地的分支名（刚 `git init` 的仓库要能显示 `main`）。
    pub fn abbrev_branch(&self) -> Result<String, GitError> {
        if let Ok(output) = run_git(&self.root, &["rev-parse", "--abbrev-ref", "HEAD"]) {
            let branch = String::from_utf8_lossy(&output).trim().to_string();
            if !branch.is_empty() {
                return Ok(branch);
            }
        }
        let output = run_git(&self.root, &["symbolic-ref", "--short", "HEAD"])?;
        let branch = String::from_utf8_lossy(&output).trim().to_string();
        return Ok(if branch.is_empty() {
            "HEAD".to_string()
        } else {
            branch
        });
    }

    /// HEAD 当前指向什么（设计板 16 的 detached / 空仓库状态）。
    pub fn head_state(&self) -> HeadState {
        // 有符号引用 → 在某个分支上；否则是游离头。仓库还没有提交时两者都不是。
        if let Ok(output) = run_git(&self.root, &["symbolic-ref", "--quiet", "--short", "HEAD"]) {
            let name = String::from_utf8_lossy(&output).trim().to_string();
            if !name.is_empty() {
                // 分支存在但还没有提交：`rev-parse HEAD` 会失败。
                if run_git(&self.root, &["rev-parse", "--verify", "HEAD"]).is_err() {
                    return HeadState::Unborn { branch: name };
                }
                return HeadState::Branch(name);
            }
        }
        if let Ok(output) = run_git(&self.root, &["rev-parse", "--verify", "HEAD"]) {
            let sha = String::from_utf8_lossy(&output).trim().to_string();
            if !sha.is_empty() {
                return HeadState::Detached { sha };
            }
        }
        return HeadState::Unborn {
            branch: "main".to_string(),
        };
    }

    /// 与上游的 `(ahead, behind)`；没有上游时返回 `(0, 0)`。
    pub fn ahead_behind(&self) -> (u32, u32) {
        let output = match run_git(
            &self.root,
            &["rev-list", "--left-right", "--count", "@{upstream}...HEAD"],
        ) {
            Ok(output) => output,
            Err(_) => return (0, 0),
        };
        let text = String::from_utf8_lossy(&output);
        let mut parts = text.split_whitespace();
        let behind = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
        let ahead = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
        return (ahead, behind);
    }

    /// 读取一条配置（`git config --get <key>`）；没有该键时返回 `None`。
    pub fn config_get(&self, key: &str) -> Option<String> {
        let output = run_git(&self.root, &["config", "--get", key]).ok()?;
        let value = String::from_utf8_lossy(&output).trim().to_string();
        if value.is_empty() {
            return None;
        }
        return Some(value);
    }

    /// 只读仓库级配置（`git config --local --get <key>`）。
    ///
    /// 与 `config_get` 的区别：不回落全局，用于「本仓库是否覆盖了身份」这种判断。
    pub fn config_get_local(&self, key: &str) -> Option<String> {
        let output = run_git(&self.root, &["config", "--local", "--get", key]).ok()?;
        let value = String::from_utf8_lossy(&output).trim().to_string();
        if value.is_empty() {
            return None;
        }
        return Some(value);
    }

    /// 各远端的 URL（`git remote get-url <name>`），顺序与 `remote` 一致。
    pub fn remote_urls(&self) -> Result<Vec<(String, String)>, GitError> {
        let names = self.lines(&["remote"])?;
        let mut pairs: Vec<(String, String)> = Vec::new();
        for name in names {
            let url = run_git(&self.root, &["remote", "get-url", name.as_str()])?;
            pairs.push((name, String::from_utf8_lossy(&url).trim().to_string()));
        }
        return Ok(pairs);
    }
}

/// 一个本地分支。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Branch {
    pub name: String,
    pub current: bool,
    pub ahead: u32,
    pub behind: u32,
}

/// 一个贮藏项。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stash {
    /// `stash@{0}` 形式的引用。
    pub reference: String,
    pub summary: String,
}

/// 侧栏引用的完整集合。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Refs {
    pub branches: Vec<Branch>,
    pub remotes: Vec<String>,
    pub tags: Vec<String>,
    pub stashes: Vec<Stash>,
    pub submodules: Vec<String>,
    pub worktrees: Vec<String>,
}

/// HEAD 的三种形态。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeadState {
    /// 在某个分支上，且分支已有提交。
    Branch(String),
    /// 游离头（检出了某个提交）。
    Detached { sha: String },
    /// 分支还没有任何提交（空仓库 / 刚初始化的仓库）。
    Unborn { branch: String },
}

impl Default for HeadState {
    /// 没读到任何信息时按「空仓库的 main」处理，与 `load_tab` 的默认分支一致。
    fn default() -> Self {
        return HeadState::Unborn {
            branch: "main".to_string(),
        };
    }
}

/// 本机 Git 的信息（设置页 Git 分区）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GitInfo {
    /// `git --version` 的原文，例如 `git version 2.56.0`。
    pub version: String,
    /// `git --exec-path`。
    pub exec_path: String,
}

/// 读取本机 Git 版本与执行路径；找不到 Git 时返回错误。
pub fn info() -> Result<GitInfo, GitError> {
    let version = run_git(Path::new("."), &["--version"])?;
    let exec_path = run_git(Path::new("."), &["--exec-path"])?;
    return Ok(GitInfo {
        version: String::from_utf8_lossy(&version).trim().to_string(),
        exec_path: String::from_utf8_lossy(&exec_path).trim().to_string(),
    });
}

/// 读取某个仓库的 `credential.helper`；未配置时为空串。
pub fn credential_helper(repo: &Path) -> Result<String, GitError> {
    let output = run_git(repo, &["config", "--get", "credential.helper"]).unwrap_or_default();
    return Ok(String::from_utf8_lossy(&output).trim().to_string());
}

impl Repo {
    /// 侧栏需要的引用集合：本地分支（含当前分支与领先/落后）、远程、标签、
    /// 贮藏、子模块与工作树。
    pub fn refs(&self) -> Result<Refs, GitError> {
        let branches = self.branches()?;
        let remotes = self.lines(&["remote"])?;
        let tags = self.lines(&[
            "for-each-ref",
            "--sort=-creatordate",
            "--format=%(refname:short)",
            "refs/tags",
        ])?;
        let stashes = self.stashes()?;
        let submodules = self.submodules();
        let worktrees = self.worktrees()?;
        return Ok(Refs {
            branches,
            remotes,
            tags,
            stashes,
            submodules,
            worktrees,
        });
    }

    fn branches(&self) -> Result<Vec<Branch>, GitError> {
        let output = run_git(
            &self.root,
            &[
                "for-each-ref",
                "--format=%(refname:short)%1f%(HEAD)%1f%(upstream:track)",
                "refs/heads",
            ],
        )?;
        let mut branches = Vec::new();
        for line in String::from_utf8_lossy(&output).lines() {
            let mut fields = line.split('\u{1f}');
            let name = fields.next().unwrap_or("").to_string();
            if name.is_empty() {
                continue;
            }
            let current = fields.next().unwrap_or("") == "*";
            let (ahead, behind) = parse_track(fields.next().unwrap_or(""));
            branches.push(Branch {
                name,
                current,
                ahead,
                behind,
            });
        }
        branches.sort_by(|left, right| {
            right
                .current
                .cmp(&left.current)
                .then_with(|| left.name.cmp(&right.name))
        });
        return Ok(branches);
    }

    fn stashes(&self) -> Result<Vec<Stash>, GitError> {
        let output = run_git(&self.root, &["stash", "list", "--format=%gd%x1f%s"])?;
        let mut stashes = Vec::new();
        for line in String::from_utf8_lossy(&output).lines() {
            let mut fields = line.split('\u{1f}');
            let reference = fields.next().unwrap_or("").to_string();
            if reference.is_empty() {
                continue;
            }
            stashes.push(Stash {
                reference,
                summary: fields.next().unwrap_or("").to_string(),
            });
        }
        return Ok(stashes);
    }

    fn submodules(&self) -> Vec<String> {
        let text = match std::fs::read_to_string(self.root.join(".gitmodules")) {
            Ok(text) => text,
            Err(_) => return Vec::new(),
        };
        let mut paths = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if let Some(path) = line.strip_prefix("path =") {
                let path = path.trim();
                if !path.is_empty() {
                    paths.push(path.to_string());
                }
            }
        }
        return paths;
    }

    fn worktrees(&self) -> Result<Vec<String>, GitError> {
        let output = run_git(&self.root, &["worktree", "list", "--porcelain"])?;
        let mut worktrees = Vec::new();
        for line in String::from_utf8_lossy(&output).lines() {
            if let Some(path) = line.strip_prefix("worktree ") {
                worktrees.push(path.to_string());
            }
        }
        // 主工作树就是仓库本身，侧栏只列附加工作树。
        let root = self.root.display().to_string();
        worktrees.retain(|path| *path != root);
        return Ok(worktrees);
    }

    pub(crate) fn lines(&self, args: &[&str]) -> Result<Vec<String>, GitError> {
        let output = run_git(&self.root, args)?;
        return Ok(String::from_utf8_lossy(&output)
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect());
    }
}

#[cfg(test)]
pub(crate) mod testkit {
    use super::*;

    pub(crate) fn fixture(name: &str, files: &[(&str, &str)], message: &str) -> (TempDir, String) {
        let dir = TempDir::new(name);
        let path = dir.path();
        run_git(path, &["init", "-q", "-b", "main"]).expect("git init");
        run_git(path, &["config", "user.name", "Test"]).expect("user.name");
        run_git(path, &["config", "user.email", "test@example.com"]).expect("user.email");
        for (file, contents) in files {
            let target = path.join(file);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).expect("parent");
            }
            std::fs::write(&target, contents).expect("write");
        }
        run_git(path, &["add", "-A"]).expect("add");
        run_git(path, &["commit", "-q", "-m", message]).expect("commit");
        let path_text = path.display().to_string();
        return (dir, path_text);
    }

    pub(crate) struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        pub(crate) fn new(name: &str) -> TempDir {
            let unique = format!(
                "git-test-{}-{}-{}",
                name,
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_nanos())
                    .unwrap_or(0)
            );
            let path = std::env::temp_dir().join(unique);
            std::fs::create_dir_all(&path).expect("temp dir");
            return TempDir { path };
        }

        pub(crate) fn path(&self) -> &Path {
            return &self.path;
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    /// 建一个 bare 远程，并把工作仓库推上去。
    ///
    /// 远程目录的守卫要一并返回：它在 drop 时删除整个裸仓库，留在函数里会让
    /// 后续命令找不到远程。
    pub(crate) fn with_remote(name: &str) -> (TempDir, String, TempDir, String) {
        let (guard, path) = fixture(name, &[("a.txt", "one\n")], "base");
        let remote_dir = TempDir::new(&format!("{}-remote", name));
        let remote = remote_dir.path().display().to_string();
        run_git(Path::new("."), &["init", "-q", "--bare", remote.as_str()]).expect("bare init");
        let repo = Repo::open(&path).expect("open");
        run_git(repo.root(), &["remote", "add", "origin", remote.as_str()]).expect("remote add");
        run_git(repo.root(), &["push", "-q", "-u", "origin", "main"]).expect("push");
        return (guard, path, remote_dir, remote);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::testkit::*;

    #[test]
    fn open_rejects_missing_path() {
        let error = Repo::open("/definitely/not/a/path").expect_err("missing path");
        assert!(matches!(error, GitError::Unreadable { .. }));
    }

    #[test]
    fn open_rejects_directory_without_git() {
        let dir = TempDir::new("plain");
        let path = dir.path().display().to_string();
        let error = Repo::open(&path).expect_err("not a repo");
        assert!(matches!(error, GitError::NotARepository { .. }));
    }

    #[test]
    fn summary_reports_branch_name() {
        let (_guard, path) = fixture("summary", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        let summary = repo.summary().expect("summary");
        assert_eq!(summary.branch, "main");
        assert_eq!(
            summary.name,
            std::path::Path::new(&path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
        );
        assert_eq!((summary.ahead, summary.behind), (0, 0));
    }

    #[test]
    fn refs_lists_branch_tags_and_stashes() {
        let (_guard, path) = fixture("refs", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        run_git(repo.root(), &["tag", "v1.0.0"]).expect("tag");
        run_git(repo.root(), &["branch", "feature"]).expect("branch");
        std::fs::write(repo.root().join("a.txt"), "two\n").expect("write");
        run_git(repo.root(), &["stash", "push", "-m", "wip"]).expect("stash");

        let refs = repo.refs().expect("refs");
        let current = refs
            .branches
            .iter()
            .find(|branch| branch.current)
            .expect("current branch");
        assert_eq!(current.name, "main");
        assert!(refs.branches.iter().any(|branch| branch.name == "feature"));
        assert_eq!(refs.tags, vec!["v1.0.0".to_string()]);
        assert_eq!(refs.stashes.len(), 1);
        assert!(refs.stashes[0].reference.starts_with("stash@{0}"));
        assert!(refs.submodules.is_empty());
        assert!(refs.worktrees.is_empty());
        assert!(refs.remotes.is_empty());
    }

    #[test]
    fn info_reports_the_installed_git() {
        let info = info().expect("git info");
        assert!(info.version.starts_with("git version"), "{}", info.version);
        assert!(!info.exec_path.is_empty());
    }

    #[test]
    fn credential_helper_reads_the_repository_config() {
        let (_guard, path) = fixture("helper", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        // 仓库级配置优先于全局；这台机器上全局可能已经配了助手，因此只断言
        // 仓库级设置能被读到。
        run_git(repo.root(), &["config", "credential.helper", "store"]).expect("config");
        assert_eq!(credential_helper(repo.root()).expect("helper"), "store");
    }
}
