//! Git 执行层：只在这里拼装并运行 Git 命令，并把输出解析为 `gitui-core` 的类型。
//!
//! 写路径与同步在后续阶段加入；当前提供 P1 需要的只读能力（仓库校验、状态、
//! 日志、单提交补丁与统计、克隆）。

use domain::{Commit, DiffStat, GitError, RepoSummary, StatusEntry};
use std::path::{Path, PathBuf};
use std::process::Command;

/// 单条命令的输出上限，避免异常输出占满内存；超出的部分截断。
const MAX_OUTPUT: usize = 8 * 1024 * 1024;
/// 展示在错误里的 stderr 长度上限。
const MAX_DETAIL: usize = 4096;

/// 仓库处于哪种多步操作中（决定顶部的操作条与可用动作）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Operation {
    /// 没有进行中的操作。
    #[default]
    None,
    Merge,
    Rebase,
    CherryPick,
    Revert,
}

impl Operation {
    /// 存储/日志里的短标识。
    pub fn id(self) -> &'static str {
        return match self {
            Operation::None => "none",
            Operation::Merge => "merge",
            Operation::Rebase => "rebase",
            Operation::CherryPick => "cherry-pick",
            Operation::Revert => "revert",
        };
    }
}

/// 一次提交的结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitOutcome {
    /// 新提交的完整 SHA。
    pub sha: String,
    /// 钩子与 Git 的输出（成功或失败都有）。
    pub log: String,
}

/// 历史筛选条件。空字段表示不限制。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LogFilter {
    /// 关键字：摘要、作者、短 SHA、引用名。
    pub query: String,
    /// 只保留该作者。
    pub author: Option<String>,
    /// `--since` 的值（`2026-11-01` 或 `7 days ago`）。
    pub since: Option<String>,
    /// `--until` 的值。
    pub until: Option<String>,
    /// 只在该引用（分支、标签、`origin/main`）上读取。
    pub reference: Option<String>,
}

/// 把 Git 的输出压成一行可展示的摘要：去掉空行，保留最后一条有效信息。
fn summarise(output: &str) -> String {
    let mut lines: Vec<&str> = output
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();
    if lines.is_empty() {
        return String::new();
    }
    // fetch/push 的进度行很多，最后几行才是结果（`main -> main`、`up to date`）。
    if lines.len() > 3 {
        lines = lines.split_off(lines.len() - 3);
    }
    return lines.join("\n");
}

/// 一个已经校验过的仓库。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repo {
    root: PathBuf,
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

    /// 与上游的 `(ahead, behind)`；没有上游时返回 `(0, 0)`。
    pub fn ahead_behind(&self) -> (u32, u32) {
        let output = match run_git(&self.root, &["rev-list", "--left-right", "--count", "@{upstream}...HEAD"])
        {
            Ok(output) => output,
            Err(_) => return (0, 0),
        };
        let text = String::from_utf8_lossy(&output);
        let mut parts = text.split_whitespace();
        let behind = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
        let ahead = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
        return (ahead, behind);
    }

    /// 最近的提交，最多 `limit` 条。`query` 非空时按摘要、作者、短 SHA 过滤。
    pub fn log(&self, limit: usize, query: &str) -> Result<Vec<Commit>, GitError> {
        let limit_arg = format!("-{}", limit.max(1));
        // 字段分隔符用单元分隔符（U+001F），记录分隔符用记录分隔符（U+001E），
        // 避免摘要里的标点与分隔符冲突。
        let format = "--pretty=format:%H%x1f%h%x1f%an%x1f%at%x1f%s%x1f%D%x1f%P%x1e";
        let output = run_git(
            &self.root,
            &["log", limit_arg.as_str(), "--date=unix", format],
        )?;
        let mut commits = parse_log(&String::from_utf8_lossy(&output))?;
        if !query.is_empty() {
            let query = query.to_lowercase();
            commits.retain(|commit| {
                commit.summary.to_lowercase().contains(&query)
                    || commit.author.to_lowercase().contains(&query)
                    || commit.short.contains(&query)
                    || commit
                        .refs
                        .iter()
                        .any(|reference| reference.to_lowercase().contains(&query))
            });
        }
        return Ok(commits);
    }

    /// 带筛选的提交列表：分支/引用、作者、时间范围与关键字。
    ///
    /// 关键字在本地匹配（摘要、作者、短 SHA、引用）；其余条件交给 `git log`，
    /// 避免把整个历史读进内存。
    pub fn log_filtered(&self, limit: usize, filter: &LogFilter) -> Result<Vec<Commit>, GitError> {
        let limit_arg = format!("-{}", limit.max(1));
        let format = "--pretty=format:%H%x1f%h%x1f%an%x1f%at%x1f%s%x1f%D%x1f%P%x1e";
        let mut args: Vec<&str> = vec!["log", limit_arg.as_str(), "--date=unix", format];
        let reference = filter.reference.clone().unwrap_or_default();
        if !reference.is_empty() {
            args.push(reference.as_str());
        }
        let since_arg = filter
            .since
            .as_ref()
            .map(|since| format!("--since={}", since));
        let until_arg = filter
            .until
            .as_ref()
            .map(|until| format!("--until={}", until));
        let author_arg = filter
            .author
            .as_ref()
            .filter(|author| !author.is_empty())
            .map(|author| format!("--author={}", author));
        if let Some(value) = since_arg.as_deref() {
            args.push(value);
        }
        if let Some(value) = until_arg.as_deref() {
            args.push(value);
        }
        if let Some(value) = author_arg.as_deref() {
            args.push(value);
        }
        let output = run_git(&self.root, &args)?;
        let mut commits = parse_log(&String::from_utf8_lossy(&output))?;
        if !filter.query.is_empty() {
            let needle = filter.query.to_lowercase();
            commits.retain(|commit| {
                commit.summary.to_lowercase().contains(&needle)
                    || commit.author.to_lowercase().contains(&needle)
                    || commit.short.contains(&needle)
                    || commit
                        .refs
                        .iter()
                        .any(|reference| reference.to_lowercase().contains(&needle))
            });
        }
        return Ok(commits);
    }

    /// 历史里出现过的作者，按最近提交的先后去重。
    pub fn authors(&self) -> Result<Vec<String>, GitError> {
        let output = run_git(&self.root, &["log", "--format=%an"])?;
        let mut authors: Vec<String> = Vec::new();
        for line in String::from_utf8_lossy(&output).lines() {
            let name = line.trim();
            if name.is_empty() {
                continue;
            }
            if !authors.iter().any(|existing| existing == name) {
                authors.push(name.to_string());
            }
        }
        return Ok(authors);
    }

    /// 两个提交之间的区间 diff（`from..to`）。
    ///
    /// 用于多提交对比：`from` 为空时表示与工作区比较（`to` 作为起点）。
    pub fn diff_between(&self, from: &str, to: &str) -> Result<String, GitError> {
        let range = format!("{}..{}", from, to);
        let output = run_git(&self.root, &["diff", "--no-color", range.as_str()])?;
        return Ok(String::from_utf8_lossy(&output).to_string());
    }

    /// 某个提交里单个文件的补丁。
    pub fn patch_for_path(&self, sha: &str, path: &str) -> Result<String, GitError> {
        let output = run_git(
            &self.root,
            &["show", "--no-color", "--patch", "--format=", sha, "--", path],
        )?;
        return Ok(String::from_utf8_lossy(&output).to_string());
    }

    /// 一个提交的补丁文本（不含 commit 头）。
    pub fn patch(&self, sha: &str) -> Result<String, GitError> {
        let output = run_git(
            &self.root,
            &["show", "--no-color", "--patch", "--format=", sha],
        )?;
        return Ok(String::from_utf8_lossy(&output).to_string());
    }

    /// 一个提交改动的文件与行数统计。
    pub fn numstat(&self, sha: &str) -> Result<Vec<DiffStat>, GitError> {
        let output = run_git(&self.root, &["show", "--numstat", "--format=", sha])?;
        let mut stats = Vec::new();
        for line in String::from_utf8_lossy(&output).lines() {
            if line.trim().is_empty() {
                continue;
            }
            let mut parts = line.splitn(3, '\t');
            let additions = parts.next().unwrap_or("");
            let deletions = parts.next().unwrap_or("");
            let path = parts.next().unwrap_or("");
            stats.push(DiffStat {
                path: path.to_string(),
                additions: additions.parse().unwrap_or(0),
                deletions: deletions.parse().unwrap_or(0),
            });
        }
        return Ok(stats);
    }

    /// 工作区与 index 的变更文件。
    pub fn status(&self) -> Result<Vec<StatusEntry>, GitError> {
        let output = run_git(&self.root, &["status", "--porcelain=v1", "-z"])?;
        return parse_status(&output);
    }

    /// 当前仓库是否处于合并/变基/拣选/回滚中。
    ///
    /// 通过 `.git` 下的标记文件判断：`MERGE_HEAD`、`rebase-merge`/`rebase-apply`、
    /// `CHERRY_PICK_HEAD`、`REVERT_HEAD`。顺序上合并优先于拣选，因为合并也可以由
    /// 拣选触发。
    pub fn operation(&self) -> Operation {
        let git_dir = self.root.join(".git");
        if git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists() {
            return Operation::Rebase;
        }
        if git_dir.join("MERGE_HEAD").exists() {
            return Operation::Merge;
        }
        if git_dir.join("CHERRY_PICK_HEAD").exists() {
            return Operation::CherryPick;
        }
        if git_dir.join("REVERT_HEAD").exists() {
            return Operation::Revert;
        }
        return Operation::None;
    }

    /// 冲突文件（暂存区与工作区都为改动的 `U` 状态）。
    pub fn conflicts(&self) -> Result<Vec<String>, GitError> {
        let entries = self.status()?;
        return Ok(entries
            .into_iter()
            .filter(|entry| {
                const UNMERGED: [char; 3] = ['D', 'A', 'U'];
                // porcelain v1 的冲突标记：DD、AU、UD、UA、DU、AA、UU。
                let combined = [entry.index, entry.worktree];
                (entry.index == 'U' || entry.worktree == 'U')
                    || (combined[0] != combined[1]
                        && UNMERGED.contains(&combined[0])
                        && UNMERGED.contains(&combined[1]))
                    || (entry.index == 'A' && entry.worktree == 'A')
                    || (entry.index == 'D' && entry.worktree == 'D')
            })
            .map(|entry| entry.path)
            .collect());
    }

    /// 用一侧的内容解决冲突：`ours` 为真取当前分支，否则取对方。
    pub fn resolve_with(&self, path: &str, ours: bool) -> Result<(), GitError> {
        let flag = if ours { "--ours" } else { "--theirs" };
        run_git(&self.root, &["checkout", flag, "--", path])?;
        run_git(&self.root, &["add", "--", path])?;
        return Ok(());
    }

    /// 放弃进行中的操作（`git merge --abort` 等）。
    pub fn abort_operation(&self, operation: Operation) -> Result<(), GitError> {
        let args: &[&str] = match operation {
            Operation::Merge => &["merge", "--abort"],
            Operation::Rebase => &["rebase", "--abort"],
            Operation::CherryPick => &["cherry-pick", "--abort"],
            Operation::Revert => &["revert", "--abort"],
            Operation::None => return Ok(()),
        };
        run_git(&self.root, args)?;
        return Ok(());
    }

    /// 从远程取回引用（`git fetch --prune <remote>`）。
    pub fn fetch(&self, remote: &str) -> Result<String, GitError> {
        let output = run_git_combined(&self.root, &["fetch", "--prune", remote])?;
        return Ok(summarise(&output));
    }

    /// 拉取当前分支的上游（`git pull --ff-only`）。
    ///
    /// 只用快进：会产生合并或冲突的拉取交给后续阶段的操作流程，避免在同步里隐式
    /// 生成合并提交。
    pub fn pull(&self) -> Result<String, GitError> {
        let output = run_git_combined(&self.root, &["pull", "--ff-only"])?;
        return Ok(summarise(&output));
    }

    /// 推送当前分支到上游；没有上游时用 `-u <remote> <branch>` 建立。
    pub fn push(&self, remote: Option<&str>) -> Result<String, GitError> {
        let branch = self.abbrev_branch()?;
        let has_upstream = run_git(
            &self.root,
            &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{upstream}"],
        )
        .is_ok();
        let args: Vec<&str> = match (has_upstream, remote) {
            (true, _) => vec!["push"],
            (false, Some(remote)) => vec!["push", "-u", remote, branch.as_str()],
            (false, None) => vec!["push", "-u", "origin", branch.as_str()],
        };
        let output = run_git_combined(&self.root, &args)?;
        return Ok(summarise(&output));
    }

    /// 把工作区的改动加入暂存区（`git add -- <paths>`）。
    pub fn stage(&self, paths: &[String]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }
        let mut args: Vec<&str> = vec!["add", "--"];
        args.extend(paths.iter().map(|path| path.as_str()));
        run_git(&self.root, &args)?;
        return Ok(());
    }

    /// 暂存全部改动，包含未跟踪文件（`git add -A`）。
    pub fn stage_all(&self) -> Result<(), GitError> {
        run_git(&self.root, &["add", "-A"])?;
        return Ok(());
    }

    /// 取消暂存（`git restore --staged -- <paths>`）。
    pub fn unstage(&self, paths: &[String]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }
        let mut args: Vec<&str> = vec!["restore", "--staged", "--"];
        args.extend(paths.iter().map(|path| path.as_str()));
        run_git(&self.root, &args)?;
        return Ok(());
    }

    /// 取消全部暂存（`git restore --staged .`）。
    pub fn unstage_all(&self) -> Result<(), GitError> {
        run_git(&self.root, &["restore", "--staged", "."])?;
        return Ok(());
    }

    /// 丢弃工作区的改动，恢复为暂存区（或 `HEAD`）的内容。
    ///
    /// 这是不可撤销操作，调用方必须先确认；未跟踪文件不在处理范围内。
    pub fn discard(&self, paths: &[String]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }
        let mut args: Vec<&str> = vec!["restore", "--worktree", "--"];
        args.extend(paths.iter().map(|path| path.as_str()));
        run_git(&self.root, &args)?;
        return Ok(());
    }

    /// 工作区或暂存区的逐文件统计；`staged` 为真时读 `--cached`。
    pub fn numstat_section(&self, staged: bool) -> Result<Vec<DiffStat>, GitError> {
        let args: Vec<&str> = if staged {
            vec!["diff", "--numstat", "--cached"]
        } else {
            vec!["diff", "--numstat"]
        };
        let output = run_git(&self.root, &args)?;
        let mut stats = Vec::new();
        for line in String::from_utf8_lossy(&output).lines() {
            let mut parts = line.splitn(3, '\t');
            let additions = parts.next().unwrap_or("");
            let deletions = parts.next().unwrap_or("");
            let path = parts.next().unwrap_or("");
            if path.is_empty() {
                continue;
            }
            stats.push(DiffStat {
                path: path.to_string(),
                additions: additions.parse().unwrap_or(0),
                deletions: deletions.parse().unwrap_or(0),
            });
        }
        return Ok(stats);
    }

    /// 某个文件的工作区 diff：`staged` 为真时读暂存区（`--cached`）。
    pub fn file_diff(&self, path: &str, staged: bool) -> Result<String, GitError> {
        let args: Vec<&str> = if staged {
            vec!["diff", "--no-color", "--cached", "--", path]
        } else {
            vec!["diff", "--no-color", "--", path]
        };
        let output = run_git(&self.root, &args)?;
        return Ok(String::from_utf8_lossy(&output).to_string());
    }

    /// 提交暂存区，返回提交 SHA 与钩子输出。
    ///
    /// 钩子的 stdout/stderr 由 `git commit` 合并返回；失败时错误里带有同样的
    /// 输出，界面据此显示钩子日志（设计板的 `hook-box`）。
    pub fn commit(&self, summary: &str, description: &str) -> Result<CommitOutcome, GitError> {
        let message = if description.trim().is_empty() {
            summary.to_string()
        } else {
            format!("{}\n\n{}", summary, description)
        };
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["commit", "-m", message.as_str()])
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .map_err(|error| GitError::Spawn {
                program: "git".to_string(),
                reason: error.to_string(),
            })?;
        let log = {
            let mut text = String::from_utf8_lossy(&output.stdout).to_string();
            text.push_str(&String::from_utf8_lossy(&output.stderr));
            text.trim().to_string()
        };
        if !output.status.success() {
            return Err(GitError::HookFailed { log });
        }
        let sha = run_git(&self.root, &["rev-parse", "HEAD"])?;
        return Ok(CommitOutcome {
            sha: String::from_utf8_lossy(&sha).trim().to_string(),
            log,
        });
    }

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

    fn lines(&self, args: &[&str]) -> Result<Vec<String>, GitError> {
        let output = run_git(&self.root, args)?;
        return Ok(String::from_utf8_lossy(&output)
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect());
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

/// 解析 `%(upstream:track)`：`[ahead 2]`、`[behind 1]`、`[ahead 2, behind 1]`。
fn parse_track(track: &str) -> (u32, u32) {
    let mut ahead = 0;
    let mut behind = 0;
    for part in track.trim_matches(|c| c == '[' || c == ']').split(',') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix("ahead ") {
            ahead = rest.trim().parse().unwrap_or(0);
        } else if let Some(rest) = part.strip_prefix("behind ") {
            behind = rest.trim().parse().unwrap_or(0);
        }
    }
    return (ahead, behind);
}

/// 在 `directory` 初始化一个空仓库（`git init -b main`）。
pub fn init_repository(directory: &str) -> Result<(), GitError> {
    let path = Path::new(directory);
    std::fs::create_dir_all(path).map_err(|error| GitError::Unreadable {
        path: path.display().to_string(),
        reason: error.to_string(),
    })?;
    run_git(Path::new("."), &["init", "-q", "-b", "main", directory])?;
    return Ok(());
}

/// 克隆 `url` 到 `destination`；目标目录必须不存在或为空。
pub fn clone(url: &str, destination: &str) -> Result<(), GitError> {
    let destination_path = Path::new(destination);
    if let Some(parent) = destination_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|error| GitError::Unreadable {
                path: parent.display().to_string(),
                reason: error.to_string(),
            })?;
        }
    }
    let destination_text = destination_path.display().to_string();
    run_git(Path::new("."), &["clone", url, destination_text.as_str()])?;
    return Ok(());
}

/// 在 `directory` 里运行 `git args...`，返回 stdout。
pub fn run_git(directory: &Path, args: &[&str]) -> Result<Vec<u8>, GitError> {
    let program = "git";
    let mut command = Command::new(program);
    command
        .arg("-C")
        .arg(directory)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0");
    let output = command.output().map_err(|error| GitError::Spawn {
        program: program.to_string(),
        reason: error.to_string(),
    })?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = truncate(&detail, MAX_DETAIL);
        let joined = args.join(" ");
        // `rev-parse --show-toplevel` 在非仓库目录失败属于正常探测，调用方
        // 需要区分“不是仓库”和“命令出错”，因此这里保留原始错误。
        return Err(GitError::CommandFailed {
            program: program.to_string(),
            args: joined,
            code: output.status.code(),
            detail,
        });
    }
    let mut stdout = output.stdout;
    stdout.truncate(MAX_OUTPUT);
    return Ok(stdout);
}

/// 运行 Git 并把 stdout 与 stderr 合并返回。
///
/// 同步命令（fetch/pull/push）的进度与结果都写在 stderr，只读 stdout 会得到空串。
fn run_git_combined(directory: &Path, args: &[&str]) -> Result<String, GitError> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(directory)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0");
    let output = command.output().map_err(|error| GitError::Spawn {
        program: "git".to_string(),
        reason: error.to_string(),
    })?;
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !stderr.trim().is_empty() {
        text.push_str(stderr.as_str());
    }
    if !output.status.success() {
        return Err(GitError::CommandFailed {
            program: "git".to_string(),
            args: args.join(" "),
            code: output.status.code(),
            detail: truncate(text.trim(), MAX_DETAIL),
        });
    }
    return Ok(text);
}

fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let kept: String = text.chars().take(limit).collect();
    return format!("{}…", kept);
}

fn parse_log(text: &str) -> Result<Vec<Commit>, GitError> {
    let mut commits = Vec::new();
    for record in text.split('\u{1e}') {
        let record = record.trim_start_matches('\n');
        if record.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = record.split('\u{1f}').collect();
        if fields.len() < 6 {
            return Err(GitError::Parse {
                context: "git log".to_string(),
                detail: format!("字段数量为 {}：{}", fields.len(), record),
            });
        }
        let refs = fields[5]
            .split(',')
            .map(|reference| reference.trim().to_string())
            .filter(|reference| !reference.is_empty())
            .collect::<Vec<_>>();
        let parents = fields
            .get(6)
            .map(|parents| {
                parents
                    .split_whitespace()
                    .map(|parent| parent.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        commits.push(Commit {
            sha: fields[0].to_string(),
            short: fields[1].to_string(),
            author: fields[2].to_string(),
            date: fields[3].to_string(),
            summary: fields[4].to_string(),
            refs,
            parents,
        });
    }
    return Ok(commits);
}

fn parse_status(output: &[u8]) -> Result<Vec<StatusEntry>, GitError> {
    let mut entries = Vec::new();
    let text = String::from_utf8_lossy(output);
    for record in text.split('\0') {
        if record.is_empty() {
            continue;
        }
        let mut chars = record.chars();
        let index = chars.next().unwrap_or(' ');
        let worktree = chars.next().unwrap_or(' ');
        // 第 3 个字符是分隔空格。
        let path: String = chars.skip(1).collect();
        if path.is_empty() {
            continue;
        }
        entries.push(StatusEntry {
            path,
            index,
            worktree,
        });
    }
    return Ok(entries);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command as StdCommand;

    /// 建一个临时仓库并在其中提交 `files` 的内容。返回 (守卫, 路径)。
    fn fixture(name: &str, files: &[(&str, &str)], message: &str) -> (TempDir, String) {
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

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(name: &str) -> TempDir {
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

        fn path(&self) -> &Path {
            return &self.path;
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

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
    fn log_reads_commits_newest_first() {
        let (_guard, path) = fixture("log", &[("a.txt", "one\n")], "first commit");
        let repo = Repo::open(&path).expect("open");
        run_git(repo.root(), &["commit", "-q", "--allow-empty", "-m", "second commit"]).expect("commit");
        let commits = repo.log(10, "").expect("log");
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].summary, "second commit");
        assert_eq!(commits[1].summary, "first commit");
        assert_eq!(commits[0].sha.len(), 40);
        assert_eq!(commits[0].short.len(), 7);
        assert_eq!(commits[0].parents.len(), 1);
        assert_eq!(commits[0].parents[0].len(), 40);
    }

    #[test]
    fn log_filters_by_query() {
        let (_guard, path) = fixture("filter", &[("a.txt", "one\n")], "add parser");
        let repo = Repo::open(&path).expect("open");
        run_git(repo.root(), &["commit", "-q", "--allow-empty", "-m", "fix runner"]).expect("commit");
        let commits = repo.log(10, "parser").expect("log");
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].summary, "add parser");
        let none = repo.log(10, "nothing-matches").expect("log");
        assert!(none.is_empty());
    }

    #[test]
    fn status_reports_staged_and_untracked() {
        let (_guard, path) = fixture("status", &[("tracked.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("tracked.txt"), "two\n").expect("write");
        run_git(repo.root(), &["add", "tracked.txt"]).expect("add");
        std::fs::write(repo.root().join("untracked.txt"), "new\n").expect("write");
        let entries = repo.status().expect("status");
        let tracked = entries
            .iter()
            .find(|entry| entry.path == "tracked.txt")
            .expect("tracked entry");
        assert!(tracked.is_staged());
        let untracked = entries
            .iter()
            .find(|entry| entry.path == "untracked.txt")
            .expect("untracked entry");
        assert!(untracked.is_unstaged());
        assert!(!untracked.is_staged());
    }

    #[test]
    fn patch_and_numstat_describe_one_commit() {
        let (_guard, path) = fixture("patch", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "one\ntwo\n").expect("write");
        run_git(repo.root(), &["commit", "-q", "-am", "add line"]).expect("commit");
        let head = repo.log(1, "").expect("log").remove(0);
        let patch = repo.patch(&head.sha).expect("patch");
        assert!(patch.contains("+two"));
        let stats = repo.numstat(&head.sha).expect("numstat");
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].path, "a.txt");
        assert_eq!(stats[0].additions, 1);
        assert_eq!(stats[0].deletions, 0);
    }

    #[test]
    fn summary_reports_branch_name() {
        let (_guard, path) = fixture("summary", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        let summary = repo.summary().expect("summary");
        assert_eq!(summary.branch, "main");
        assert_eq!(summary.name, std::path::Path::new(&path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string());
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
    fn track_strings_are_parsed() {
        assert_eq!(parse_track(""), (0, 0));
        assert_eq!(parse_track("[ahead 2]"), (2, 0));
        assert_eq!(parse_track("[behind 3]"), (0, 3));
        assert_eq!(parse_track("[ahead 2, behind 1]"), (2, 1));
    }

    #[test]
    fn filtered_log_honours_reference_author_and_keyword() {
        let (_guard, path) = fixture("filtered", &[("a.txt", "one\n")], "first");
        let repo = Repo::open(&path).expect("open");
        run_git(repo.root(), &["checkout", "-q", "-b", "side"]).expect("checkout");
        run_git(repo.root(), &["commit", "-q", "--allow-empty", "-m", "side work"]).expect("commit");
        run_git(repo.root(), &["checkout", "-q", "main"]).expect("checkout");

        let only_side = repo
            .log_filtered(
                50,
                &LogFilter {
                    reference: Some("side".to_string()),
                    ..LogFilter::default()
                },
            )
            .expect("log");
        assert_eq!(only_side.len(), 2);
        assert_eq!(only_side[0].summary, "side work");

        let keyword = repo
            .log_filtered(
                50,
                &LogFilter {
                    query: "first".to_string(),
                    ..LogFilter::default()
                },
            )
            .expect("log");
        assert_eq!(keyword.len(), 1);

        let other_author = repo
            .log_filtered(
                50,
                &LogFilter {
                    author: Some("Nobody".to_string()),
                    ..LogFilter::default()
                },
            )
            .expect("log");
        assert!(other_author.is_empty());
    }

    #[test]
    fn authors_are_unique_and_ordered() {
        let (_guard, path) = fixture("authors", &[("a.txt", "one\n")], "first");
        let repo = Repo::open(&path).expect("open");
        run_git(
            repo.root(),
            &["-c", "user.name=Other", "commit", "-q", "--allow-empty", "-m", "second"],
        )
        .expect("commit");
        let authors = repo.authors().expect("authors");
        // 按提交顺序（新的在前），因此后提交的作者排在最前面。
        assert_eq!(authors, vec!["Other".to_string(), "Test".to_string()]);
    }

    #[test]
    fn patch_for_path_limits_the_patch_to_one_file() {
        let (_guard, path) = fixture("pathpatch", &[("a.txt", "one\n"), ("b.txt", "x\n")], "base");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "one\ntwo\n").expect("write");
        std::fs::write(repo.root().join("b.txt"), "x\ny\n").expect("write");
        run_git(repo.root(), &["commit", "-q", "-am", "two files"]).expect("commit");
        let head = repo.log(1, "").expect("log").remove(0);
        let patch = repo.patch_for_path(&head.sha, "a.txt").expect("patch");
        assert!(patch.contains("a.txt"));
        assert!(!patch.contains("b.txt"));
        assert!(patch.contains("+two"));
    }

    #[test]
    fn init_repository_creates_a_repo_on_main() {
        let base = std::env::temp_dir().join(format!(
            "git-init-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        let target = base.join("fresh");
        init_repository(&target.display().to_string()).expect("init");
        let repo = Repo::open(&target.display().to_string()).expect("open");
        assert_eq!(repo.abbrev_branch().unwrap_or_default(), "main");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn stage_and_unstage_move_paths_between_sections() {
        let (_guard, path) = fixture("stage", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "two\n").expect("write");
        std::fs::write(repo.root().join("b.txt"), "new\n").expect("write");

        repo.stage_all().expect("stage all");
        let staged: Vec<String> = repo
            .status()
            .expect("status")
            .into_iter()
            .filter(|entry| entry.is_staged())
            .map(|entry| entry.path)
            .collect();
        assert!(staged.contains(&"a.txt".to_string()));
        assert!(staged.contains(&"b.txt".to_string()));

        repo.unstage(&["a.txt".to_string()]).expect("unstage");
        let entries = repo.status().expect("status");
        let a = entries.iter().find(|entry| entry.path == "a.txt").expect("a");
        assert!(!a.is_staged());
        assert!(a.is_unstaged());

        repo.stage(&["a.txt".to_string()]).expect("stage");
        assert!(repo
            .status()
            .expect("status")
            .iter()
            .find(|entry| entry.path == "a.txt")
            .expect("a")
            .is_staged());
    }

    #[test]
    fn discard_restores_the_worktree_copy() {
        let (_guard, path) = fixture("discard", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "changed\n").expect("write");
        assert!(repo.status().expect("status").iter().any(|entry| entry.is_unstaged()));

        repo.discard(&["a.txt".to_string()]).expect("discard");
        assert_eq!(
            std::fs::read_to_string(repo.root().join("a.txt")).expect("read"),
            "one\n"
        );
        assert!(repo.status().expect("status").is_empty());
    }

    #[test]
    fn commit_records_summary_and_description() {
        let (_guard, path) = fixture("commit", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "two\n").expect("write");
        repo.stage_all().expect("stage");
        let outcome = repo
            .commit("add second line", "A longer description.")
            .expect("commit");
        assert_eq!(outcome.sha.len(), 40);
        let head = repo.log(1, "").expect("log").remove(0);
        assert_eq!(head.sha, outcome.sha);
        assert_eq!(head.summary, "add second line");
    }

    #[test]
    fn failing_hook_reports_its_log() {
        let (_guard, path) = fixture("hooks", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        // 让 pre-commit 钩子失败。
        let hook = repo.root().join(".git/hooks/pre-commit");
        std::fs::write(&hook, "#!/bin/sh\necho 'lint failed' >&2\nexit 1\n").expect("write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(&hook).expect("meta").permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&hook, permissions).expect("chmod");
        }
        std::fs::write(repo.root().join("a.txt"), "two\n").expect("write");
        repo.stage_all().expect("stage");

        let error = repo.commit("should fail", "").expect_err("hook failure");
        match error {
            GitError::HookFailed { log } => {
                assert!(log.contains("lint failed"), "钩子输出应包含在日志里：{}", log);
            }
            other => panic!("期望 HookFailed，实际 {:?}", other),
        }
        // 失败后暂存区保留，历史没有新增提交。
        assert_eq!(repo.log(10, "").expect("log").len(), 1);
    }

    #[test]
    fn file_diff_reads_worktree_and_index() {
        let (_guard, path) = fixture("filediff", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "two\n").expect("write");

        let worktree = repo.file_diff("a.txt", false).expect("worktree diff");
        assert!(worktree.contains("-one"));
        assert!(worktree.contains("+two"));
        assert!(repo.file_diff("a.txt", true).expect("index diff").is_empty());

        repo.stage(&["a.txt".to_string()]).expect("stage");
        let staged = repo.file_diff("a.txt", true).expect("index diff");
        assert!(staged.contains("+two"));
        assert!(repo.file_diff("a.txt", false).expect("worktree diff").is_empty());
    }

    #[test]
    fn operation_reports_conflicting_merge() {
        let (_guard, path) = fixture("mergeop", &[("a.txt", "base\n")], "base");
        let repo = Repo::open(&path).expect("open");
        assert_eq!(repo.operation(), Operation::None);

        run_git(repo.root(), &["checkout", "-q", "-b", "side"]).expect("checkout");
        std::fs::write(repo.root().join("a.txt"), "side\n").expect("write");
        run_git(repo.root(), &["commit", "-q", "-am", "side"]).expect("commit");
        run_git(repo.root(), &["checkout", "-q", "main"]).expect("checkout");
        std::fs::write(repo.root().join("a.txt"), "main\n").expect("write");
        run_git(repo.root(), &["commit", "-q", "-am", "main"]).expect("commit");
        // 合并冲突是预期结果，命令会失败。
        let _ = run_git(repo.root(), &["merge", "side"]);

        assert_eq!(repo.operation(), Operation::Merge);
        let conflicts = repo.conflicts().expect("conflicts");
        assert_eq!(conflicts, vec!["a.txt".to_string()]);

        // 取当前分支一侧并标记为已解决，冲突清单清空。
        repo.resolve_with("a.txt", true).expect("resolve ours");
        assert!(repo.conflicts().expect("conflicts").is_empty());
        assert_eq!(
            std::fs::read_to_string(repo.root().join("a.txt")).expect("read"),
            "main\n"
        );

        // 放弃合并回到干净状态。
        repo.abort_operation(Operation::Merge).expect("abort");
        assert_eq!(repo.operation(), Operation::None);
        assert!(repo.status().expect("status").is_empty());
    }

    #[test]
    fn operation_detects_rebase_directory() {
        let (_guard, path) = fixture("rebaseop", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        std::fs::create_dir_all(repo.root().join(".git/rebase-merge")).expect("mkdir");
        assert_eq!(repo.operation(), Operation::Rebase);
        std::fs::remove_dir_all(repo.root().join(".git/rebase-merge")).expect("rmdir");
        assert_eq!(repo.operation(), Operation::None);
    }

    /// 建一个 bare 远程，并把工作仓库推上去。
    ///
    /// 远程目录的守卫要一并返回：它在 drop 时删除整个裸仓库，留在函数里会让
    /// 后续命令找不到远程。
    fn with_remote(name: &str) -> (TempDir, String, TempDir, String) {
        let (guard, path) = fixture(name, &[("a.txt", "one\n")], "base");
        let remote_dir = TempDir::new(&format!("{}-remote", name));
        let remote = remote_dir.path().display().to_string();
        run_git(Path::new("."), &["init", "-q", "--bare", remote.as_str()]).expect("bare init");
        let repo = Repo::open(&path).expect("open");
        run_git(repo.root(), &["remote", "add", "origin", remote.as_str()]).expect("remote add");
        run_git(repo.root(), &["push", "-q", "-u", "origin", "main"]).expect("push");
        return (guard, path, remote_dir, remote);
    }

    #[test]
    fn push_and_fetch_report_results() {
        let (guard, path, _remote_guard, _remote) = with_remote("sync");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "two\n").expect("write");
        repo.stage_all().expect("stage");
        repo.commit("second", "").expect("commit");

        let pushed = repo.push(None).expect("push");
        assert!(!pushed.is_empty(), "推送应有输出");
        assert_eq!(repo.ahead_behind(), (0, 0));

        // 再取一次远端：没有新提交时也应有可展示的输出或空串，不应报错。
        repo.fetch("origin").expect("fetch");
        assert_eq!(repo.summary().expect("summary").ahead, 0);
        drop(guard);
    }

    #[test]
    fn pull_fast_forwards_from_the_remote() {
        let (_guard, path, _remote_guard, remote) = with_remote("pull");
        // 用第二个克隆向远程推一个新提交。
        let other_dir = TempDir::new("pull-other");
        let other = other_dir.path().display().to_string();
        run_git(Path::new("."), &["clone", "-q", remote.as_str(), other.as_str()]).expect("clone");
        run_git(Path::new(other.as_str()), &["config", "user.name", "Other"]).expect("name");
        run_git(Path::new(other.as_str()), &["config", "user.email", "o@example.com"]).expect("mail");
        std::fs::write(Path::new(other.as_str()).join("b.txt"), "b\n").expect("write");
        run_git(Path::new(other.as_str()), &["add", "-A"]).expect("add");
        run_git(Path::new(other.as_str()), &["commit", "-q", "-m", "remote work"]).expect("commit");
        run_git(Path::new(other.as_str()), &["push", "-q", "origin", "main"]).expect("push");

        let repo = Repo::open(&path).expect("open");
        repo.fetch("origin").expect("fetch");
        assert_eq!(repo.ahead_behind(), (0, 1), "应落后一个提交");
        repo.pull().expect("pull");
        assert_eq!(repo.ahead_behind(), (0, 0));
        assert_eq!(
            std::fs::read_to_string(repo.root().join("b.txt")).expect("read"),
            "b\n"
        );
    }

    #[test]
    fn push_failure_keeps_the_rejection_text() {
        let (_guard, path, _remote_guard, remote) = with_remote("pushfail");
        // 让远程前进，本地推送会被拒（non-fast-forward）。
        let other_dir = TempDir::new("pushfail-other");
        let other = other_dir.path().display().to_string();
        run_git(Path::new("."), &["clone", "-q", remote.as_str(), other.as_str()]).expect("clone");
        run_git(Path::new(other.as_str()), &["config", "user.name", "Other"]).expect("name");
        run_git(Path::new(other.as_str()), &["config", "user.email", "o@example.com"]).expect("mail");
        std::fs::write(Path::new(other.as_str()).join("c.txt"), "c\n").expect("write");
        run_git(Path::new(other.as_str()), &["add", "-A"]).expect("add");
        run_git(Path::new(other.as_str()), &["commit", "-q", "-m", "remote"]).expect("commit");
        run_git(Path::new(other.as_str()), &["push", "-q", "origin", "main"]).expect("push");

        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "local\n").expect("write");
        repo.stage_all().expect("stage");
        repo.commit("local", "").expect("commit");
        let error = repo.push(None).expect_err("push should be rejected");
        let detail = error.detail();
        assert!(
            detail.to_lowercase().contains("reject")
                || detail.to_lowercase().contains("fetch first")
                || detail.to_lowercase().contains("non-fast-forward"),
            "推送拒绝的原因应在详情里：{}",
            detail
        );
    }

    #[test]
    fn external_git_matches_our_reading() {
        // 用系统 git 的结果作为期望值，验证解析没有偏差。
        let (_guard, path) = fixture("cross", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        let expected = StdCommand::new("git")
            .arg("-C")
            .arg(repo.root())
            .args(["rev-parse", "HEAD"])
            .output()
            .expect("git rev-parse");
        let expected = String::from_utf8_lossy(&expected.stdout).trim().to_string();
        let commits = repo.log(1, "").expect("log");
        assert_eq!(commits[0].sha, expected);
    }
}
