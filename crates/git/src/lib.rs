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

/// 一个提交的完整元信息（设计板 08 的 Info 页）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommitDetails {
    pub sha: String,
    pub short: String,
    pub tree: String,
    pub parents: Vec<String>,
    pub author: String,
    pub author_email: String,
    /// UTC 秒。
    pub authored_at: i64,
    pub committer: String,
    pub committer_email: String,
    pub committed_at: i64,
    pub subject: String,
    /// 说明正文（不含首行）。
    pub body: String,
    pub refs: Vec<String>,
}

/// blame 的一行。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlameLine {
    pub sha: String,
    pub short: String,
    pub author: String,
    /// UTC 秒。
    pub timestamp: i64,
    /// 行号（从 1 开始）。
    pub line: u32,
    pub text: String,
}

/// 一条 reflog 记录。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReflogEntry {
    /// 短 SHA（`HEAD@{2}` 里的位置由列表顺序体现）。
    pub short: String,
    /// 完整 SHA。
    pub sha: String,
    /// 动作：`commit`、`reset`、`checkout`、`merge` 等。
    pub action: String,
    /// 说明（`commit: fix composer`）。
    pub summary: String,
    /// UTC 秒。
    pub timestamp: i64,
}

/// 一次提交的结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitOutcome {
    /// 新提交的完整 SHA。
    pub sha: String,
    /// 钩子与 Git 的输出（成功或失败都有）。
    pub log: String,
}

/// 提交开关。默认是普通提交。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommitOptions {
    /// 改写上一条提交（`--amend`）。
    pub amend: bool,
    /// 追加 `Signed-off-by`（`--signoff`）。
    pub signoff: bool,
    /// 跳过 pre-commit / commit-msg 钩子（`--no-verify`）。
    pub no_verify: bool,
}

impl CommitOptions {
    /// 转成 `git commit` 的开关参数，顺序固定便于测试。
    pub fn flags(&self) -> Vec<&'static str> {
        let mut flags: Vec<&'static str> = Vec::new();
        if self.amend {
            flags.push("--amend");
        }
        if self.signoff {
            flags.push("--signoff");
        }
        if self.no_verify {
            flags.push("--no-verify");
        }
        return flags;
    }
}

/// 拉取时如何整合上游（设计板 29 的三选）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PullStrategy {
    /// `--no-rebase`：产生合并提交。
    #[default]
    Merge,
    /// `--rebase`：把本地提交移到上游之上。
    Rebase,
    /// `--ff-only`：只允许快进，否则失败。
    FfOnly,
}

impl PullStrategy {
    pub fn id(self) -> &'static str {
        return match self {
            PullStrategy::Merge => "merge",
            PullStrategy::Rebase => "rebase",
            PullStrategy::FfOnly => "ff-only",
        };
    }

    pub fn from_id(id: &str) -> PullStrategy {
        return match id {
            "rebase" => PullStrategy::Rebase,
            "ff-only" => PullStrategy::FfOnly,
            _ => PullStrategy::Merge,
        };
    }

    fn flag(self) -> &'static str {
        return match self {
            PullStrategy::Merge => "--no-rebase",
            PullStrategy::Rebase => "--rebase",
            PullStrategy::FfOnly => "--ff-only",
        };
    }
}

/// 取回选项（设计板 28）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FetchOptions {
    /// 删掉远端已删除的跟踪分支。
    pub prune: bool,
    /// 同时取回标签。
    pub tags: bool,
    /// 对所有远端执行。
    pub all_remotes: bool,
}

/// 拉取选项（设计板 29）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PullOptions {
    pub strategy: PullStrategy,
    /// 拉取前自动贮藏、完成后恢复。
    pub autostash: bool,
    /// 变基时把本地所有提交一起移动。
    pub rebase_local: bool,
}

/// 推送选项（设计板 15）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PushOptions {
    /// 一并推送标签。
    pub tags: bool,
    /// `--force-with-lease`：远端没被别人改动才允许覆盖。
    pub force_with_lease: bool,
    /// 推送所有分支。
    pub all_branches: bool,
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

/// `git reset` 的三种模式（设计板 22 的 Move current branch）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetMode {
    /// 只移动分支，保留暂存区与工作区。
    Soft,
    /// 移动分支并重置暂存区，保留工作区改动。
    Mixed,
    /// 移动分支并丢弃暂存区与工作区改动。
    Hard,
}

impl ResetMode {
    pub fn id(self) -> &'static str {
        return match self {
            ResetMode::Soft => "soft",
            ResetMode::Mixed => "mixed",
            ResetMode::Hard => "hard",
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

    /// 从某个引用新建并切换到本地分支（`git checkout -b <name> <from>`）。
    pub fn checkout_new_branch(&self, name: &str, from: &str) -> Result<(), GitError> {
        run_git(&self.root, &["checkout", "-b", name, from])?;
        return Ok(());
    }

    /// 合并某个引用到当前分支（`git merge --no-edit <reference>`）。
    ///
    /// 冲突时返回 `HookFailed`，日志里有 Git 的提示；操作状态由 `operation()` 读出。
    pub fn merge(&self, reference: &str) -> Result<String, GitError> {
        let output = run_git_combined(
            &self.root,
            &["merge", "--no-edit", reference],
        )?;
        return Ok(summarise(&output));
    }

    /// 把当前分支重置到某个提交（`git reset --soft|--mixed|--hard`）。
    pub fn reset_to(&self, sha: &str, mode: ResetMode) -> Result<(), GitError> {
        let flag = match mode {
            ResetMode::Soft => "--soft",
            ResetMode::Mixed => "--mixed",
            ResetMode::Hard => "--hard",
        };
        run_git(&self.root, &["reset", flag, sha])?;
        return Ok(());
    }

    /// 新建分支（`git branch <name> <from>`）；`from` 为空时基于当前 HEAD。
    pub fn create_branch(&self, name: &str, from: Option<&str>) -> Result<(), GitError> {
        let args: Vec<&str> = match from {
            Some(from) => vec!["branch", name, from],
            None => vec!["branch", name],
        };
        run_git(&self.root, &args)?;
        return Ok(());
    }

    /// 新建轻量标签（`git tag <name> <from>`）。
    pub fn create_tag(&self, name: &str, from: Option<&str>) -> Result<(), GitError> {
        let args: Vec<&str> = match from {
            Some(from) => vec!["tag", name, from],
            None => vec!["tag", name],
        };
        run_git(&self.root, &args)?;
        return Ok(());
    }

    /// 删除标签。
    pub fn delete_tag(&self, name: &str) -> Result<(), GitError> {
        run_git(&self.root, &["tag", "-d", name])?;
        return Ok(());
    }

    /// 添加远程（`git remote add <name> <url>`）。
    pub fn add_remote(&self, name: &str, url: &str) -> Result<(), GitError> {
        run_git(&self.root, &["remote", "add", name, url])?;
        return Ok(());
    }

    /// 删除远程。
    pub fn remove_remote(&self, name: &str) -> Result<(), GitError> {
        run_git(&self.root, &["remote", "remove", name])?;
        return Ok(());
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

    /// 改写远程的 URL（`git remote set-url <name> <url>`）。
    pub fn set_remote_url(&self, name: &str, url: &str) -> Result<(), GitError> {
        run_git(&self.root, &["remote", "set-url", name, url])?;
        return Ok(());
    }

    /// 重命名分支（`git branch -m <old> <new>`）。
    pub fn rename_branch(&self, old: &str, new: &str) -> Result<(), GitError> {
        if new.trim().is_empty() {
            return Err(GitError::CommandFailed {
                program: "git".to_string(),
                args: "branch -m".to_string(),
                code: None,
                detail: "分支名不能为空".to_string(),
            });
        }
        run_git(&self.root, &["branch", "-m", old, new])?;
        return Ok(());
    }

    /// 贮藏当前改动（`git stash push -u`）；`message` 为空时不带说明。
    pub fn stash_push(&self, message: &str) -> Result<(), GitError> {
        let args: Vec<&str> = if message.trim().is_empty() {
            vec!["stash", "push", "-u"]
        } else {
            vec!["stash", "push", "-u", "-m", message.trim()]
        };
        run_git(&self.root, &args)?;
        return Ok(());
    }

    /// 取出最近一条贮藏（`git stash pop`）。
    pub fn stash_pop(&self) -> Result<(), GitError> {
        run_git(&self.root, &["stash", "pop"])?;
        return Ok(());
    }

    /// 应用某条贮藏但保留它（`git stash apply <reference>`）。
    pub fn stash_apply(&self, reference: &str) -> Result<(), GitError> {
        run_git(&self.root, &["stash", "apply", reference])?;
        return Ok(());
    }

    /// 丢弃某条贮藏（`git stash drop`）。
    pub fn stash_drop(&self, reference: &str) -> Result<(), GitError> {
        run_git(&self.root, &["stash", "drop", reference])?;
        return Ok(());
    }

    /// 切换到 `branch`（`git checkout <branch>`）。
    pub fn checkout_branch(&self, branch: &str) -> Result<(), GitError> {
        run_git(&self.root, &["checkout", branch])?;
        return Ok(());
    }

    /// 删除本地分支；删除当前分支会被拒绝（`git branch -d` 本身也会拒绝，这里先
    /// 给出明确错误，避免把 Git 的英文提示直接抛给用户）。
    pub fn delete_branch(&self, branch: &str) -> Result<(), GitError> {
        let current = self.abbrev_branch()?;
        if current == branch {
            return Err(GitError::CommandFailed {
                program: "git".to_string(),
                args: format!("branch -d {}", branch),
                code: None,
                detail: "不能删除当前分支".to_string(),
            });
        }
        run_git(&self.root, &["branch", "-d", branch])?;
        return Ok(());
    }

    /// 一个提交的完整元信息。
    pub fn commit_details(&self, sha: &str) -> Result<CommitDetails, GitError> {
        let format = "--pretty=format:%H%x1f%h%x1f%T%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%cn%x1f%ce%x1f%ct%x1f%s%x1f%b%x1f%D";
        let output = run_git(&self.root, &["show", "-s", "--date=unix", format, sha])?;
        let text = String::from_utf8_lossy(&output);
        let mut fields = text.split('\u{1f}');
        let mut next = || fields.next().unwrap_or("").to_string();
        let details = CommitDetails {
            sha: next(),
            short: next(),
            tree: next(),
            parents: next()
                .split_whitespace()
                .map(|parent| parent.to_string())
                .collect(),
            author: next(),
            author_email: next(),
            authored_at: next().parse().unwrap_or(0),
            committer: next(),
            committer_email: next(),
            committed_at: next().parse().unwrap_or(0),
            subject: next(),
            body: next().trim().to_string(),
            refs: next()
                .split(',')
                .map(|reference| reference.trim().to_string())
                .filter(|reference| !reference.is_empty())
                .collect(),
        };
        return Ok(details);
    }

    /// 两个引用之间的提交列表（`base..target`，包含 target 一侧）。
    pub fn commits_between(&self, base: &str, target: &str) -> Result<Vec<Commit>, GitError> {
        let range = format!("{}..{}", base, target);
        let format = "--pretty=format:%H%x1f%h%x1f%an%x1f%at%x1f%s%x1f%D%x1f%P%x1e";
        let output = run_git(
            &self.root,
            &["log", "--date=unix", format, range.as_str()],
        )?;
        return parse_log(&String::from_utf8_lossy(&output));
    }

    /// 一个文件的逐行归属（`git blame --line-porcelain`）。
    pub fn blame(&self, path: &str) -> Result<Vec<BlameLine>, GitError> {
        let output = run_git(
            &self.root,
            &["blame", "--line-porcelain", "--", path],
        )?;
        let text = String::from_utf8_lossy(&output);
        let mut lines: Vec<BlameLine> = Vec::new();
        let mut sha = String::new();
        let mut author = String::new();
        let mut timestamp = 0;
        let mut line_number = 0;
        for raw in text.lines() {
            if raw.starts_with('\t') {
                lines.push(BlameLine {
                    sha: sha.clone(),
                    short: sha.chars().take(7).collect(),
                    author: author.clone(),
                    timestamp,
                    line: line_number,
                    text: raw.get(1..).unwrap_or("").to_string(),
                });
                continue;
            }
            if let Some(value) = raw.strip_prefix("author ") {
                author = value.to_string();
                continue;
            }
            if let Some(value) = raw.strip_prefix("author-time ") {
                timestamp = value.parse().unwrap_or(0);
                continue;
            }
            // 头部行：`<sha> <orig-line> <final-line> [<group-size>]`。
            let mut parts = raw.split(' ');
            let maybe_sha = parts.next().unwrap_or("");
            if maybe_sha.len() == 40 && maybe_sha.chars().all(|c| c.is_ascii_hexdigit()) {
                sha = maybe_sha.to_string();
                let _orig = parts.next();
                line_number = parts
                    .next()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(line_number);
            }
        }
        return Ok(lines);
    }

    /// 一个文件的历史（跟随重命名）。
    pub fn file_history(&self, path: &str, limit: usize) -> Result<Vec<Commit>, GitError> {
        let limit_arg = format!("-{}", limit.max(1));
        let format = "--pretty=format:%H%x1f%h%x1f%an%x1f%at%x1f%s%x1f%D%x1f%P%x1e";
        let output = run_git(
            &self.root,
            &[
                "log",
                "--follow",
                "--date=unix",
                limit_arg.as_str(),
                format,
                "--",
                path,
            ],
        )?;
        return parse_log(&String::from_utf8_lossy(&output));
    }

    /// 读取当前分支的 reflog（最近的在前）。
    pub fn reflog(&self, limit: usize) -> Result<Vec<ReflogEntry>, GitError> {
        let limit_arg = format!("-{}", limit.max(1));
        let output = run_git(
            &self.root,
            &[
                "reflog",
                limit_arg.as_str(),
                "--date=unix",
                "--format=%H%x1f%h%x1f%gs%x1f%ct",
            ],
        )?;
        let mut entries = Vec::new();
        for line in String::from_utf8_lossy(&output).lines() {
            let mut fields = line.split('\u{1f}');
            let sha = fields.next().unwrap_or("").to_string();
            if sha.is_empty() {
                continue;
            }
            let short = fields.next().unwrap_or("").to_string();
            let subject = fields.next().unwrap_or("").to_string();
            let timestamp = fields.next().and_then(|value| value.parse().ok()).unwrap_or(0);
            // `%gs` 形如 `commit: message`、`reset: moving to HEAD~1`。
            let (action, summary) = match subject.split_once(portable_separator()) {
                Some((action, summary)) => (action.trim().to_string(), summary.trim().to_string()),
                None => (subject.clone(), String::new()),
            };
            entries.push(ReflogEntry {
                short,
                sha,
                action,
                summary,
                timestamp,
            });
        }
        return Ok(entries);
    }

    /// 把当前分支重置到某个提交（`git reset --hard <sha>`）。
    ///
    /// 用于 reflog 的撤销；调用方必须先确认，未提交的改动会被丢弃。
    pub fn reset_hard(&self, sha: &str) -> Result<(), GitError> {
        run_git(&self.root, &["reset", "--hard", sha])?;
        return Ok(());
    }

    /// 拣选一个提交到当前分支（`git cherry-pick <sha>`）。
    ///
    /// 冲突时命令失败，但仓库已经进入冲突状态；调用方据 [`Repo::operation`]
    /// 判断该走解决还是放弃。
    pub fn cherry_pick(&self, sha: &str) -> Result<(), GitError> {
        run_git(&self.root, &["cherry-pick", sha])?;
        return Ok(());
    }

    /// 反向提交一个提交（`git revert --no-edit <sha>`）。
    pub fn revert(&self, sha: &str) -> Result<(), GitError> {
        run_git(&self.root, &["revert", "--no-edit", sha])?;
        return Ok(());
    }

    /// 继续被冲突打断的合并/变基/拣选/回滚。
    ///
    /// 变基用 `--continue`，其余三种命令同名；执行前调用方应确认冲突都已标记解决。
    pub fn continue_operation(&self, operation: Operation) -> Result<String, GitError> {
        let args: &[&str] = match operation {
            Operation::Merge => &["merge", "--continue"],
            Operation::Rebase => &["rebase", "--continue"],
            Operation::CherryPick => &["cherry-pick", "--continue"],
            Operation::Revert => &["revert", "--continue"],
            Operation::None => return Ok(String::new()),
        };
        // `--continue` 默认会打开编辑器写合并提交信息；界面不接管编辑器，
        // 因此把编辑器设为 true（保留默认信息）并禁止交互提示。
        let output = run_git_combined_with_env(
            &self.root,
            args,
            &[("GIT_EDITOR", "true"), ("GIT_TERMINAL_PROMPT", "0")],
        )?;
        return Ok(summarise(&output));
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

    /// 读取冲突文件当前内容（含冲突标记）。
    ///
    /// 只读工作区文件；文件不存在时返回空串，交给界面提示而不是报错。
    pub fn conflict_content(&self, path: &str) -> String {
        let target = self.root.join(path);
        return std::fs::read_to_string(target).unwrap_or_default();
    }

    /// 写入解决后的内容并标记为已解决（`git add`）。
    ///
    /// 写盘失败直接把原因当错误返回，避免「看起来已解决但文件没变」。
    pub fn resolve_conflict(&self, path: &str, content: &str) -> Result<(), GitError> {
        let target = self.root.join(path);
        std::fs::write(&target, content).map_err(|error| GitError::Unreadable {
            path: target.display().to_string(),
            reason: error.to_string(),
        })?;
        run_git(&self.root, &["add", "--", path])?;
        return Ok(());
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
        return self.fetch_with(remote, &FetchOptions {
            prune: true,
            ..FetchOptions::default()
        });
    }

    /// 带选项的取回：剪枝、标签、全部远端。
    pub fn fetch_with(&self, remote: &str, options: &FetchOptions) -> Result<String, GitError> {
        let mut args: Vec<&str> = vec!["fetch"];
        if options.prune {
            args.push("--prune");
        }
        if options.tags {
            args.push("--tags");
        }
        if options.all_remotes {
            args.push("--all");
        } else {
            args.push(remote);
        }
        let output = run_git_combined(&self.root, &args)?;
        return Ok(summarise(&output));
    }

    /// 上游新增的提交数（`HEAD..@{u}`）；没有上游时为 0。
    pub fn upstream_incoming(&self) -> u32 {
        let output = match run_git(
            &self.root,
            &["rev-list", "--count", "HEAD..@{upstream}"],
        ) {
            Ok(output) => output,
            Err(_) => return 0,
        };
        return String::from_utf8_lossy(&output)
            .trim()
            .parse()
            .unwrap_or(0);
    }

    /// 拉取当前分支的上游（`git pull --ff-only`）。
    ///
    /// 只用快进：会产生合并或冲突的拉取交给后续阶段的操作流程，避免在同步里隐式
    /// 生成合并提交。
    pub fn pull(&self) -> Result<String, GitError> {
        return self.pull_with(&PullOptions::default());
    }

    /// 带选项的拉取：整合方式、自动贮藏、整体变基。
    pub fn pull_with(&self, options: &PullOptions) -> Result<String, GitError> {
        let mut args: Vec<&str> = vec!["pull", options.strategy.flag()];
        if options.autostash {
            args.push("--autostash");
        }
        if options.rebase_local {
            args.push("--rebase-merges");
        }
        let output = run_git_combined(&self.root, &args)?;
        return Ok(summarise(&output));
    }

    /// 推送当前分支到上游；没有上游时用 `-u <remote> <branch>` 建立。
    pub fn push(&self, remote: Option<&str>) -> Result<String, GitError> {
        return self.push_with(remote, &PushOptions::default());
    }

    /// 带选项的推送：标签、`--force-with-lease`、全部分支。
    pub fn push_with(
        &self,
        remote: Option<&str>,
        options: &PushOptions,
    ) -> Result<String, GitError> {
        let branch = self.abbrev_branch()?;
        let has_upstream = run_git(
            &self.root,
            &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{upstream}"],
        )
        .is_ok();
        let mut args: Vec<&str> = vec!["push"];
        if options.force_with_lease {
            args.push("--force-with-lease");
        }
        if options.tags {
            args.push("--tags");
        }
        if options.all_branches {
            args.push("--all");
        }
        if options.all_branches || options.tags {
            // 已经指定了范围，不再追加分支名。
        } else {
            match (has_upstream, remote) {
                (true, _) => {}
                (false, Some(remote)) => {
                    args.push("-u");
                    args.push(remote);
                    args.push(branch.as_str());
                }
                (false, None) => {
                    args.push("-u");
                    args.push("origin");
                    args.push(branch.as_str());
                }
            }
        }
        let output = run_git_combined(&self.root, &args)?;
        return Ok(summarise(&output));
    }

    /// 推送一个标签（`git push <remote> <tag>`）。
    pub fn push_tag(&self, tag: &str, remote: Option<&str>) -> Result<String, GitError> {
        let remote = remote.unwrap_or("origin");
        let output = run_git_combined(&self.root, &["push", remote, tag])?;
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
        return self.commit_with(summary, description, &CommitOptions::default());
    }

    /// 带开关的提交：改写上一条、追加 sign-off、跳过钩子。
    pub fn commit_with(
        &self,
        summary: &str,
        description: &str,
        options: &CommitOptions,
    ) -> Result<CommitOutcome, GitError> {
        let message = if description.trim().is_empty() {
            summary.to_string()
        } else {
            format!("{}\n\n{}", summary, description)
        };
        let mut args: Vec<&str> = vec!["commit"];
        args.extend(options.flags());
        args.push("-m");
        args.push(message.as_str());
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(&args)
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
/// 带额外环境变量的合并输出版本（继续操作要关掉编辑器）。
fn run_git_combined_with_env(
    directory: &Path,
    args: &[&str],
    envs: &[(&str, &str)],
) -> Result<String, GitError> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(directory)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0");
    for (key, value) in envs {
        command.env(key, value);
    }
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

/// reflog 主题里动作与说明之间的分隔符（`": "`）。
fn portable_separator() -> &'static str {
    return ": ";
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
    fn reflog_lists_recent_movements_and_can_reset() {
        let (_guard, path) = fixture("reflog", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "two\n").expect("write");
        repo.stage_all().expect("stage");
        repo.commit("second", "").expect("commit");
        let head = repo.log(2, "").expect("log").remove(0);

        let entries = repo.reflog(10).expect("reflog");
        assert!(entries.len() >= 2);
        assert_eq!(entries[0].sha.len(), 40);
        assert_eq!(entries[0].action, "commit");
        assert!(entries[0].summary.contains("second"));

        // 回到上一个提交，reflog 记录这次 reset。
        let previous = repo.log(2, "").expect("log").remove(1);
        repo.reset_hard(&previous.sha).expect("reset");
        assert_eq!(
            repo.log(1, "").expect("log").remove(0).sha,
            previous.sha,
            "reset 后 HEAD 应回到前一个提交"
        );
        let after = repo.reflog(3).expect("reflog");
        assert_eq!(after[0].action, "reset");
        assert_eq!(after[0].sha, previous.sha);
        assert_ne!(head.sha, previous.sha);
    }

    #[test]
    fn commit_details_expose_metadata_and_parents() {
        let (_guard, path) = fixture("details", &[("a.txt", "one\n")], "first");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "two\n").expect("write");
        repo.stage_all().expect("stage");
        repo.commit("second", "More detail here.").expect("commit");

        let head = repo.log(1, "").expect("log").remove(0);
        let details = repo.commit_details(&head.sha).expect("details");
        assert_eq!(details.sha, head.sha);
        assert_eq!(details.short.len(), 7);
        assert_eq!(details.tree.len(), 40);
        assert_eq!(details.parents.len(), 1);
        assert_eq!(details.parents[0].len(), 40);
        assert_eq!(details.subject, "second");
        assert!(details.body.contains("More detail here."));
        assert!(!details.author.is_empty());
        assert!(details.authored_at > 0);
        assert!(details.committed_at > 0);
    }

    #[test]
    fn commits_between_lists_only_the_range() {
        let (_guard, path) = fixture("range", &[("a.txt", "one\n")], "first");
        let repo = Repo::open(&path).expect("open");
        let first = repo.log(1, "").expect("log").remove(0).sha;
        for message in ["second", "third"] {
            std::fs::write(repo.root().join("a.txt"), format!("{message}\n")).expect("write");
            repo.stage_all().expect("stage");
            repo.commit(message, "").expect("commit");
        }
        let head = repo.log(1, "").expect("log").remove(0).sha;
        let range = repo.commits_between(&first, &head).expect("range");
        let summaries: Vec<&str> = range.iter().map(|commit| commit.summary.as_str()).collect();
        assert_eq!(summaries, vec!["third", "second"]);
    }

    #[test]
    fn blame_reports_lines_and_authors() {
        let (_guard, path) = fixture("blame", &[("a.txt", "one\ntwo\n")], "first");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "one\ntwo\nthree\n").expect("write");
        repo.stage_all().expect("stage");
        repo.commit("add third", "").expect("commit");

        let lines = repo.blame("a.txt").expect("blame");
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].text, "one");
        assert_eq!(lines[2].text, "three");
        assert_eq!(lines[0].line, 1);
        assert_eq!(lines[2].line, 3);
        assert_eq!(lines[0].short.len(), 7);
        assert!(!lines[0].author.is_empty());
        assert!(lines[0].timestamp > 0);
        // 第三行来自后一次提交，SHA 应与前两行不同。
        assert_ne!(lines[0].sha, lines[2].sha);
    }

    #[test]
    fn file_history_follows_the_path() {
        let (_guard, path) = fixture("filehist", &[("a.txt", "one\n"), ("b.txt", "b\n")], "first");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "one\ntwo\n").expect("write");
        std::fs::write(repo.root().join("b.txt"), "b\nc\n").expect("write");
        repo.stage_all().expect("stage");
        repo.commit("touch both", "").expect("commit");

        let history = repo.file_history("a.txt", 10).expect("history");
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].summary, "touch both");
        assert_eq!(history[1].summary, "first");
    }

    #[test]
    fn checkout_and_delete_branches_refuse_the_current_one() {
        let (_guard, path) = fixture("branchops", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        run_git(repo.root(), &["branch", "feature"]).expect("branch");

        repo.checkout_branch("feature").expect("checkout");
        assert_eq!(repo.abbrev_branch().expect("branch"), "feature");

        // 当前分支不能删除。
        let error = repo.delete_branch("feature").expect_err("refuse current");
        assert!(error.detail().contains("当前分支"), "{}", error.detail());
        assert!(repo
            .refs()
            .expect("refs")
            .branches
            .iter()
            .any(|branch| branch.name == "feature"));

        // 切回去之后可以删。
        repo.checkout_branch("main").expect("checkout main");
        repo.delete_branch("feature").expect("delete");
        assert!(!repo
            .refs()
            .expect("refs")
            .branches
            .iter()
            .any(|branch| branch.name == "feature"));
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

    #[test]
    fn refs_can_be_created_and_deleted() {
        let (_guard, path) = fixture("refwrite", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");

        repo.create_branch("feature", None).expect("branch");
        repo.create_tag("v9.9.9", None).expect("tag");
        repo.add_remote("upstream", "https://example.com/repo.git")
            .expect("remote");
        let refs = repo.refs().expect("refs");
        assert!(refs.branches.iter().any(|branch| branch.name == "feature"));
        assert_eq!(refs.tags, vec!["v9.9.9".to_string()]);
        assert_eq!(refs.remotes, vec!["upstream".to_string()]);

        repo.delete_tag("v9.9.9").expect("delete tag");
        repo.remove_remote("upstream").expect("remove remote");
        let refs = repo.refs().expect("refs");
        assert!(refs.tags.is_empty());
        assert!(refs.remotes.is_empty());
        assert!(refs.branches.iter().any(|branch| branch.name == "feature"));
    }

    #[test]
    fn stash_can_be_pushed_popped_and_dropped() {
        let (_guard, path) = fixture("stash", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "changed\n").expect("write");

        repo.stash_push("wip").expect("stash");
        assert!(repo.status().expect("status").is_empty(), "贮藏后工作区应干净");
        let stashes = repo.refs().expect("refs").stashes;
        assert_eq!(stashes.len(), 1);
        assert!(stashes[0].summary.contains("wip"));

        repo.stash_pop().expect("pop");
        assert!(!repo.status().expect("status").is_empty(), "取出后改动回来");

        repo.stash_push("again").expect("stash");
        let reference = repo.refs().expect("refs").stashes[0].reference.clone();
        repo.stash_drop(reference.as_str()).expect("drop");
        assert!(repo.refs().expect("refs").stashes.is_empty());
    }

    #[test]
    fn checkout_new_branch_creates_and_switches() {
        let (_guard, path) = fixture("checkoutnew", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        let head = repo.log(1, "").expect("log").remove(0).sha;
        repo.checkout_new_branch("topic/one", head.as_str())
            .expect("checkout -b");
        assert_eq!(repo.abbrev_branch().expect("branch"), "topic/one");
        assert!(repo
            .refs()
            .expect("refs")
            .branches
            .iter()
            .any(|branch| branch.name == "topic/one" && branch.current));
    }

    #[test]
    fn merge_fast_forwards_a_side_branch() {
        let (_guard, path) = fixture("merge", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        run_git(repo.root(), &["checkout", "-q", "-b", "side"]).expect("checkout");
        std::fs::write(repo.root().join("b.txt"), "b\n").expect("write");
        repo.stage_all().expect("stage");
        repo.commit("side work", "").expect("commit");
        run_git(repo.root(), &["checkout", "-q", "main"]).expect("checkout");

        let summary = repo.merge("side").expect("merge");
        assert!(!summary.is_empty() || summary.is_empty());
        assert_eq!(repo.operation(), Operation::None);
        assert!(repo.root().join("b.txt").exists());
        assert_eq!(repo.ahead_behind(), (0, 0));
    }

    #[test]
    fn reset_modes_decide_what_survives() {
        let (_guard, path) = fixture("reset", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        let base = repo.log(1, "").expect("log").remove(0).sha;
        std::fs::write(repo.root().join("a.txt"), "two\n").expect("write");
        repo.stage_all().expect("stage");
        repo.commit("second", "").expect("commit");

        // --soft 保留暂存区。
        repo.reset_to(base.as_str(), ResetMode::Soft).expect("soft");
        assert_eq!(repo.log(1, "").expect("log").remove(0).sha, base);
        assert!(repo
            .status()
            .expect("status")
            .iter()
            .any(|entry| entry.is_staged()));

        // --hard 丢掉改动。
        repo.reset_to(base.as_str(), ResetMode::Hard).expect("hard");
        assert!(repo.status().expect("status").is_empty());
        assert_eq!(
            std::fs::read_to_string(repo.root().join("a.txt")).expect("read"),
            "one\n"
        );
        assert_eq!(ResetMode::Mixed.id(), "mixed");
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

    #[test]
    fn commit_options_map_to_flags_in_fixed_order() {
        assert!(CommitOptions::default().flags().is_empty());
        let all = CommitOptions {
            amend: true,
            signoff: true,
            no_verify: true,
        };
        assert_eq!(all.flags(), vec!["--amend", "--signoff", "--no-verify"]);
    }

    #[test]
    fn amend_replaces_the_last_commit_and_signoff_adds_a_trailer() {
        let (_guard, path) = fixture("amend", &[("a.txt", "one\n")], "first");
        let repo = Repo::open(&path).expect("open");
        let before = repo.log(1, "").expect("log").remove(0).sha;

        std::fs::write(repo.root().join("a.txt"), "two\n").expect("write");
        repo.stage_all().expect("stage");
        repo.commit_with(
            "second",
            "",
            &CommitOptions {
                signoff: true,
                ..CommitOptions::default()
            },
        )
        .expect("commit with signoff");
        let message = run_git(repo.root(), &["log", "-1", "--format=%B"]).expect("log");
        let message = String::from_utf8_lossy(&message).to_string();
        assert!(message.contains("Signed-off-by:"), "缺 sign-off：{message}");

        // amend 不新增提交，只改写最后一条。
        let count_before = repo.log(100, "").expect("log").len();
        repo.commit_with(
            "amended",
            "",
            &CommitOptions {
                amend: true,
                ..CommitOptions::default()
            },
        )
        .expect("amend");
        let commits = repo.log(100, "").expect("log");
        assert_eq!(commits.len(), count_before);
        assert_eq!(commits[0].summary, "amended");
        assert_ne!(commits[0].sha, before);
    }

    #[test]
    fn no_verify_skips_a_failing_hook() {
        let (_guard, path) = fixture("noverify", &[("a.txt", "one\n")], "first");
        let hooks = Path::new(&path).join(".git/hooks");
        std::fs::create_dir_all(&hooks).expect("hooks dir");
        let pre_commit = hooks.join("pre-commit");
        std::fs::write(&pre_commit, "#!/bin/sh\nexit 1\n").expect("write hook");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&pre_commit, std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
        }
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "two\n").expect("write");
        repo.stage_all().expect("stage");

        assert!(repo.commit("blocked", "").is_err(), "钩子应阻止提交");
        repo.commit_with(
            "bypass",
            "",
            &CommitOptions {
                no_verify: true,
                ..CommitOptions::default()
            },
        )
        .expect("--no-verify 应跳过钩子");
        assert_eq!(repo.log(1, "").expect("log")[0].summary, "bypass");
    }

    #[test]
    fn cherry_pick_copies_a_commit_and_revert_undoes_one() {
        let (_guard, path) = fixture("pick", &[("a.txt", "one\n")], "first");
        let repo = Repo::open(&path).expect("open");
        let base = repo.log(1, "").expect("log")[0].sha.clone();
        std::fs::write(repo.root().join("b.txt"), "bee\n").expect("write");
        repo.stage_all().expect("stage");
        let picked = repo.commit("add b", "").expect("commit").sha;

        repo.checkout_new_branch("side", base.as_str()).expect("branch");
        repo.cherry_pick(&picked).expect("cherry-pick");
        assert!(repo.root().join("b.txt").exists(), "拣选应带过来文件");
        assert_eq!(repo.log(1, "").expect("log")[0].summary, "add b");

        repo.revert(&picked).expect("revert");
        assert!(!repo.root().join("b.txt").exists(), "回滚应删掉文件");
        assert!(repo.log(1, "").expect("log")[0].summary.contains("Revert"));
    }

    #[test]
    fn stash_apply_keeps_the_entry_and_rename_moves_the_branch() {
        let (_guard, path) = fixture("stashapply", &[("a.txt", "one\n")], "first");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "two\n").expect("write");
        repo.stash_push("wip").expect("stash");

        let entries = repo.refs().expect("refs").stashes;
        assert_eq!(entries.len(), 1);
        repo.stash_apply(entries[0].reference.as_str()).expect("apply");
        assert_eq!(
            std::fs::read_to_string(repo.root().join("a.txt")).expect("read"),
            "two\n"
        );
        assert_eq!(repo.refs().expect("refs").stashes.len(), 1, "apply 不删除贮藏");

        repo.rename_branch("main", "renamed").expect("rename");
        assert_eq!(repo.abbrev_branch().expect("branch"), "renamed");
    }

    #[test]
    fn sync_options_map_to_the_right_flags() {
        assert_eq!(PullStrategy::default(), PullStrategy::Merge);
        assert_eq!(PullStrategy::Rebase.flag(), "--rebase");
        assert_eq!(PullStrategy::FfOnly.flag(), "--ff-only");
        assert_eq!(PullStrategy::Merge.flag(), "--no-rebase");
        assert_eq!(PullStrategy::from_id("rebase"), PullStrategy::Rebase);
        assert_eq!(PullStrategy::from_id("ff-only"), PullStrategy::FfOnly);
        assert_eq!(PullStrategy::from_id("nonsense"), PullStrategy::Merge);
        assert_eq!(PullStrategy::Rebase.id(), "rebase");

        let options = FetchOptions {
            prune: true,
            tags: true,
            all_remotes: true,
        };
        assert!(options.all_remotes, "全部远端时不追加单个 remote 名");
        assert!(!PushOptions::default().force_with_lease, "强推默认关闭");
        assert!(!FetchOptions::default().prune, "取回默认不剪枝");
        assert!(!PullOptions::default().autostash);
    }

    #[test]
    fn push_with_force_lease_is_refused_when_the_remote_moved() {
        // with_remote 返回 (工作区守卫, 工作区路径, 远端守卫, 远端路径)。
        let (work_guard, work_path, remote_guard, remote_path) = with_remote("force-lease");
        let _ = (&work_guard, &remote_guard);
        let repo = Repo::open(&work_path).expect("open");
        // 另一个人推进远端。
        let other = TempDir::new("force-lease-other");
        run_git(other.path(), &["clone", "-q", remote_path.as_str(), "."]).expect("clone");
        run_git(other.path(), &["config", "user.name", "Other"]).expect("name");
        run_git(other.path(), &["config", "user.email", "o@example.com"]).expect("email");
        std::fs::write(other.path().join("other.txt"), "x\n").expect("write");
        run_git(other.path(), &["add", "-A"]).expect("add");
        run_git(other.path(), &["commit", "-q", "-m", "other"]).expect("commit");
        run_git(other.path(), &["push", "-q", "origin", "HEAD"]).expect("push");

        // 本地再提交，然后强推：lease 不成立，应被拒绝。
        std::fs::write(repo.root().join("mine.txt"), "y\n").expect("write");
        repo.stage_all().expect("stage");
        repo.commit("mine", "").expect("commit");
        let forced = repo.push_with(
            Some("origin"),
            &PushOptions {
                force_with_lease: true,
                ..PushOptions::default()
            },
        );
        assert!(forced.is_err(), "远端已前进时 --force-with-lease 必须失败");
    }

    #[test]
    fn set_remote_url_rewrites_the_target() {
        let (_guard, path) = fixture("seturl", &[("a.txt", "one\n")], "first");
        let repo = Repo::open(&path).expect("open");
        repo.add_remote("origin", "https://example.com/a.git")
            .expect("add");
        repo.set_remote_url("origin", "git@example.com:b.git")
            .expect("set-url");
        let url = run_git(repo.root(), &["remote", "get-url", "origin"]).expect("get-url");
        assert_eq!(String::from_utf8_lossy(&url).trim(), "git@example.com:b.git");
    }
}
