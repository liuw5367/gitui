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

    /// 各远端的实际取回与推送 URL；一项远端可以配置多个推送 URL。
    pub fn remote_info(&self) -> Result<Vec<RemoteInfo>, GitError> {
        let names = self.lines(&["remote"])?;
        let mut remotes = Vec::new();
        for name in names {
            let fetch_urls = self.lines(&["remote", "get-url", "--all", name.as_str()])?;
            let push_urls = self.lines(&["remote", "get-url", "--push", "--all", name.as_str()])?;
            remotes.push(RemoteInfo {
                name,
                fetch_urls,
                push_urls,
            });
        }
        return Ok(remotes);
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

/// 一个远端及其真实配置的取回、推送地址。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RemoteInfo {
    pub name: String,
    pub fetch_urls: Vec<String>,
    pub push_urls: Vec<String>,
}

/// 一条远程跟踪分支。`name` 不含远端名前缀。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RemoteBranch {
    pub remote: String,
    pub name: String,
}

impl RemoteBranch {
    pub fn full_name(&self) -> String {
        return format!("{}/{}", self.remote, self.name);
    }
}

/// 一个贮藏项。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stash {
    /// `stash@{0}` 形式的引用。
    pub reference: String,
    /// 贮藏提交的完整 OID，用于在序号变化后仍指向同一条记录。
    pub oid: String,
    pub summary: String,
}

/// 子模块登记与当前检出状态。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SubmoduleInfo {
    /// `.gitmodules` 中登记的仓库相对路径。
    pub path: String,
    pub url: String,
    /// 配置的跟踪分支；为空时不能执行「更新到最新版」。
    pub branch: Option<String>,
    pub initialized: bool,
    /// 子模块当前检出的提交。
    pub current_oid: Option<String>,
    /// 父仓库 index 当前记录的 gitlink 提交。
    pub index_oid: Option<String>,
    /// 子模块自身有暂存、未暂存或未跟踪改动。
    pub dirty: bool,
}

/// 一个已登记的 linked worktree。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorktreeInfo {
    pub path: String,
    pub branch: Option<String>,
    /// Whether this is the repository's primary worktree.
    pub main: bool,
    pub locked: bool,
    pub prunable: bool,
    pub exists: bool,
    pub dirty: bool,
}

/// 侧栏引用的完整集合。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Refs {
    pub branches: Vec<Branch>,
    pub remotes: Vec<String>,
    pub remote_info: Vec<RemoteInfo>,
    pub remote_branches: Vec<RemoteBranch>,
    pub tags: Vec<String>,
    pub stashes: Vec<Stash>,
    pub submodules: Vec<SubmoduleInfo>,
    pub worktrees: Vec<WorktreeInfo>,
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
        let remote_info = self.remote_info()?;
        let remotes = remote_info
            .iter()
            .map(|remote| remote.name.clone())
            .collect::<Vec<_>>();
        let remote_branches = self.remote_branches(&remotes)?;
        let tags = self.lines(&[
            "for-each-ref",
            "--sort=-creatordate",
            "--format=%(refname:short)",
            "refs/tags",
        ])?;
        let stashes = self.stashes()?;
        let submodules = self.submodules()?;
        let worktrees = self.worktrees()?;
        return Ok(Refs {
            branches,
            remotes,
            remote_info,
            remote_branches,
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

    fn remote_branches(&self, remotes: &[String]) -> Result<Vec<RemoteBranch>, GitError> {
        let output = run_git(
            &self.root,
            &[
                "for-each-ref",
                "--format=%(refname)%1f%(symref)",
                "refs/remotes",
            ],
        )?;
        let mut branches = Vec::new();
        for line in String::from_utf8_lossy(&output).lines() {
            let mut fields = line.split('\u{1f}');
            let reference = fields.next().unwrap_or("");
            let symref = fields.next().unwrap_or("");
            let Some(short_name) = reference.strip_prefix("refs/remotes/") else {
                continue;
            };
            if !symref.is_empty() {
                continue;
            }
            let Some(remote) = remotes
                .iter()
                .filter(|remote| short_name.starts_with(&format!("{remote}/")))
                .max_by_key(|remote| remote.len())
            else {
                continue;
            };
            let name = short_name[remote.len() + 1..].to_string();
            if name.is_empty() {
                continue;
            }
            branches.push(RemoteBranch {
                remote: remote.clone(),
                name,
            });
        }
        return Ok(branches);
    }

    fn stashes(&self) -> Result<Vec<Stash>, GitError> {
        let output = run_git(&self.root, &["stash", "list", "--format=%gd%x1f%H%x1f%s"])?;
        let mut stashes = Vec::new();
        for line in String::from_utf8_lossy(&output).lines() {
            let mut fields = line.split('\u{1f}');
            let reference = fields.next().unwrap_or("").to_string();
            if reference.is_empty() {
                continue;
            }
            stashes.push(Stash {
                reference,
                oid: fields.next().unwrap_or("").to_string(),
                summary: fields.next().unwrap_or("").to_string(),
            });
        }
        return Ok(stashes);
    }

    fn submodules(&self) -> Result<Vec<SubmoduleInfo>, GitError> {
        if !self.root.join(".gitmodules").exists() {
            return Ok(Vec::new());
        }
        let output = match run_git(
            &self.root,
            &[
                "config",
                "--file",
                ".gitmodules",
                "--get-regexp",
                r"^submodule\..*\.path$",
            ],
        ) {
            Ok(output) => output,
            Err(GitError::CommandFailed { code: Some(1), .. }) => return Ok(Vec::new()),
            Err(error) => return Err(error),
        };
        let mut modules = Vec::new();
        for line in String::from_utf8_lossy(&output).lines() {
            let Some((key, path)) = line.split_once(' ') else {
                continue;
            };
            let Some(name) = key
                .strip_prefix("submodule.")
                .and_then(|key| key.strip_suffix(".path"))
            else {
                continue;
            };
            let path = path.trim();
            if path.is_empty() {
                continue;
            }
            let relative_path = Path::new(path);
            if relative_path.is_absolute()
                || relative_path
                    .components()
                    .any(|component| !matches!(component, std::path::Component::Normal(_)))
            {
                return Err(GitError::Parse {
                    context: "submodule path".to_string(),
                    detail: "path must stay within the parent repository".to_string(),
                });
            }
            let url = self.submodule_config(name, "url").unwrap_or_default();
            let branch = self.submodule_config(name, "branch");
            let module_path = self.root.join(path);
            let parent_root = self.root.canonicalize().ok();
            let child = Repo::open(module_path.to_string_lossy().as_ref())
                .ok()
                .filter(|repo| {
                    match (
                        repo.root.canonicalize(),
                        module_path.canonicalize(),
                        parent_root.as_ref(),
                    ) {
                        (Ok(root), Ok(module), Some(parent)) => {
                            root == module && root != *parent && root.starts_with(parent)
                        }
                        _ => false,
                    }
                });
            let current_oid = child.as_ref().and_then(|repo| {
                run_git(&repo.root, &["rev-parse", "--verify", "HEAD"])
                    .ok()
                    .map(|output| String::from_utf8_lossy(&output).trim().to_string())
            });
            let dirty = child
                .as_ref()
                .map(|repo| {
                    repo.status()
                        .map(|status| !status.is_empty())
                        .unwrap_or(true)
                })
                .unwrap_or(false);
            let index_oid = run_git(&self.root, &["ls-files", "--stage", "--", path])
                .ok()
                .and_then(|output| {
                    String::from_utf8_lossy(&output).lines().find_map(|line| {
                        let (entry, _) = line.split_once('\t')?;
                        let mut fields = entry.split_whitespace();
                        let mode = fields.next()?;
                        let oid = fields.next()?;
                        (mode == "160000").then(|| oid.to_string())
                    })
                });
            modules.push(SubmoduleInfo {
                path: path.to_string(),
                url,
                branch,
                initialized: child.is_some(),
                current_oid,
                index_oid,
                dirty,
            });
        }
        return Ok(modules);
    }

    fn submodule_config(&self, name: &str, field: &str) -> Option<String> {
        let key = format!("submodule.{name}.{field}");
        let output = run_git(
            &self.root,
            &["config", "--file", ".gitmodules", "--get", key.as_str()],
        )
        .ok()?;
        let value = String::from_utf8_lossy(&output).trim().to_string();
        return (!value.is_empty()).then_some(value);
    }

    fn worktrees(&self) -> Result<Vec<WorktreeInfo>, GitError> {
        let output = run_git(&self.root, &["worktree", "list", "--porcelain"])?;
        let mut worktrees = Vec::new();
        let mut current: Option<WorktreeInfo> = None;
        for line in String::from_utf8_lossy(&output).lines() {
            if let Some(path) = line.strip_prefix("worktree ") {
                if let Some(worktree) = current.take() {
                    worktrees.push(worktree);
                }
                current = Some(WorktreeInfo {
                    path: path.to_string(),
                    ..WorktreeInfo::default()
                });
            } else if let Some(worktree) = current.as_mut() {
                if let Some(branch) = line.strip_prefix("branch refs/heads/") {
                    worktree.branch = Some(branch.to_string());
                } else if line == "locked" || line.starts_with("locked ") {
                    worktree.locked = true;
                } else if line == "prunable" || line.starts_with("prunable ") {
                    worktree.prunable = true;
                }
            }
        }
        if let Some(worktree) = current {
            worktrees.push(worktree);
        }
        // Git porcelain lists the primary worktree first, regardless of which linked
        // worktree opened the repository. Keep that identity for safe remove menus.
        let main_path = worktrees.first().map(|worktree| worktree.path.clone());
        let root = self
            .root
            .canonicalize()
            .unwrap_or_else(|_| self.root.clone());
        for worktree in &mut worktrees {
            worktree.main = main_path.as_deref() == Some(worktree.path.as_str());
            let path = Path::new(&worktree.path);
            worktree.exists = path.is_dir();
            if worktree.exists {
                if let Ok(path) = path.canonicalize() {
                    if let Ok(repo) = Repo::open(path.to_string_lossy().as_ref()) {
                        let repo_root = repo
                            .root()
                            .canonicalize()
                            .unwrap_or_else(|_| repo.root().to_path_buf());
                        worktree.dirty = if repo_root == path {
                            repo.status()
                                .map(|status| !status.is_empty())
                                .unwrap_or(true)
                        } else {
                            true
                        };
                    } else {
                        worktree.dirty = true;
                    }
                } else {
                    worktree.dirty = true;
                }
            }
        }
        worktrees.retain(|worktree| {
            Path::new(&worktree.path)
                .canonicalize()
                .map(|path| path != root)
                .unwrap_or_else(|_| Path::new(&worktree.path) != self.root)
        });
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
        assert!(!refs.stashes[0].oid.is_empty());
        assert!(refs.submodules.is_empty());
        assert!(refs.worktrees.is_empty());
        assert!(refs.remotes.is_empty());
    }

    #[test]
    fn refs_include_remote_urls_and_tracking_branches_without_head_aliases() {
        let (_guard, path, _remote_guard, remote_path) = with_remote("remote-refs");
        let repo = Repo::open(&path).expect("open");
        repo.create_branch("feature/nested", Some("main"))
            .expect("branch");
        repo.create_branch("origin/feature/nested", Some("main"))
            .expect("local branch with ambiguous short ref");
        repo.push_branch("origin", "feature/nested")
            .expect("push branch");
        repo.fetch("origin").expect("fetch");
        run_git(
            repo.root(),
            &[
                "config",
                "remote.origin.pushurl",
                "https://push.example.com/repo.git",
            ],
        )
        .expect("push URL");
        run_git(repo.root(), &["remote", "set-head", "origin", "main"]).expect("set head");

        let refs = repo.refs().expect("refs");
        assert_eq!(refs.remote_info.len(), 1);
        assert_eq!(refs.remote_info[0].name, "origin");
        assert_eq!(refs.remote_info[0].fetch_urls, vec![remote_path.clone()]);
        assert_eq!(
            refs.remote_info[0].push_urls,
            vec!["https://push.example.com/repo.git".to_string()]
        );
        assert!(
            refs.remote_branches
                .iter()
                .any(|branch| branch.full_name() == "origin/feature/nested")
        );
        assert!(
            !refs
                .remote_branches
                .iter()
                .any(|branch| branch.full_name() == "origin/HEAD")
        );
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
