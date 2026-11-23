//! 本模块负责读历史：提交列表与筛选、单提交元信息、区间提交、reflog、blame 与文件历史。

use domain::{Commit, GitError};

use crate::exec::portable_separator;
use crate::parse::parse_log;
use crate::query::{Refs, RemoteBranch, Repo};

#[cfg(test)]
use crate::exec::run_git;

const LOG_FORMAT: &str =
    "--pretty=format:%H%x1f%h%x1f%an%x1f%ae%x1f%at%x1f%s%x1f%B%x1f%D%x1f%P%x1e";
const FILE_HISTORY_FORMAT: &str =
    "--pretty=format:%x1e%H%x1f%h%x1f%an%x1f%ae%x1f%at%x1f%s%x1f%B%x1f%D%x1f%P";

/// An author identity from the repository history.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Author {
    pub name: String,
    pub email: String,
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

/// A file's change in its history: the path as it existed at that commit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileHistoryEntry {
    pub commit: Commit,
    pub path: String,
    /// True when the commit removed this path. Blame for this entry uses its first parent.
    pub deleted: bool,
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
    /// 多引用范围；提交只出现一次，即使可从多个引用到达。
    pub references: Vec<String>,
    /// 精确作者 email，作者之间按 OR 匹配。
    pub authors: Vec<String>,
    /// 提交消息子串（subject 与 body）。
    pub message: String,
    /// 仓库相对路径或目录前缀。
    pub path: Option<String>,
}

impl Repo {
    /// 全部引用与 stash 可达的提交，按拓扑顺序返回完整历史。
    pub fn log_all(&self) -> Result<Vec<Commit>, GitError> {
        let stashes = self.stash_entries()?;
        let mut args = vec![
            "log".to_string(),
            "--all".to_string(),
            "--topo-order".to_string(),
            "--date=unix".to_string(),
            LOG_FORMAT.to_string(),
        ];
        args.extend(stashes.iter().map(|(_, oid)| oid.clone()));
        let args = args.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self.run_git_full(&args)?;
        let mut commits = parse_log(&String::from_utf8_lossy(&output))?;
        decorate_stashes(&mut commits, &stashes);
        return Ok(commits);
    }

    /// 历史页使用的完整提交列表。没有引用过滤时遍历所有引用；指定引用时保留
    /// `LogFilter` 的范围语义，但不截断历史长度。
    pub fn log_history(&self, filter: &LogFilter) -> Result<Vec<Commit>, GitError> {
        let stashes = if reference_filters(filter).is_empty() {
            self.stash_entries()?
        } else {
            Vec::new()
        };
        let args = history_args(filter, true, &stashes);
        let args = args.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self.run_git_full(&args)?;
        let mut commits = parse_log(&String::from_utf8_lossy(&output))?;
        decorate_stashes(&mut commits, &stashes);
        filter_query(&mut commits, &filter.query);
        filter_message_and_authors(&mut commits, filter);
        return Ok(commits);
    }

    /// 图形布局使用的未筛选祖先集合；默认范围包括全部 stash，指定引用时只读对应范围。
    pub fn log_history_graph(&self, filter: &LogFilter) -> Result<Vec<Commit>, GitError> {
        let stashes = if reference_filters(filter).is_empty() {
            self.stash_entries()?
        } else {
            Vec::new()
        };
        let args = history_args(filter, false, &stashes);
        let args = args.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self.run_git_full(&args)?;
        let mut commits = parse_log(&String::from_utf8_lossy(&output))?;
        decorate_stashes(&mut commits, &stashes);
        return Ok(commits);
    }

    fn stash_entries(&self) -> Result<Vec<(String, String)>, GitError> {
        let output = self.run_git(&["stash", "list", "--format=%gd%x1f%H"])?;
        let entries = String::from_utf8_lossy(&output)
            .lines()
            .filter_map(|line| {
                let (reference, oid) = line.split_once('\u{1f}')?;
                (!reference.is_empty() && !oid.is_empty())
                    .then(|| (reference.to_string(), oid.to_string()))
            })
            .collect();
        return Ok(entries);
    }

    /// 将完整引用解析为提交 SHA。`^{commit}` 同时支持轻量与 annotated tag。
    pub fn resolve_commit(&self, reference: &str) -> Result<String, GitError> {
        let revision = format!("{reference}^{{commit}}");
        let output = self.run_git(&[
            "rev-parse",
            "--verify",
            "--end-of-options",
            revision.as_str(),
        ])?;
        return Ok(String::from_utf8_lossy(&output).trim().to_string());
    }

    /// 最近的提交，最多 `limit` 条。`query` 非空时按摘要、作者、短 SHA 过滤。
    pub fn log(&self, limit: usize, query: &str) -> Result<Vec<Commit>, GitError> {
        let limit_arg = format!("-{}", limit.max(1));
        // 字段分隔符用单元分隔符（U+001F），记录分隔符用记录分隔符（U+001E），
        // 避免摘要里的标点与分隔符冲突。
        let output = self.run_git(&["log", limit_arg.as_str(), "--date=unix", LOG_FORMAT])?;
        let mut commits = parse_log(&String::from_utf8_lossy(&output))?;
        filter_query(&mut commits, query);
        return Ok(commits);
    }

    /// 带筛选的提交列表：分支/引用、作者、时间范围与关键字。
    ///
    /// 关键字在本地匹配（摘要、作者、短 SHA、引用）；其余条件交给 `git log`，
    /// 避免把整个历史读进内存。
    pub fn log_filtered(&self, limit: usize, filter: &LogFilter) -> Result<Vec<Commit>, GitError> {
        let limit_arg = format!("-{}", limit.max(1));
        let mut args = vec![
            "log".to_string(),
            limit_arg,
            "--date=unix".to_string(),
            LOG_FORMAT.to_string(),
        ];
        push_revisions(&mut args, filter);
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
            args.push(value.to_string());
        }
        if let Some(value) = until_arg.as_deref() {
            args.push(value.to_string());
        }
        if let Some(value) = author_arg.as_deref() {
            args.push(value.to_string());
        }
        push_path(&mut args, filter.path.as_deref());
        let args = args.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self.run_git(&args)?;
        let mut commits = parse_log(&String::from_utf8_lossy(&output))?;
        filter_query(&mut commits, &filter.query);
        filter_message_and_authors(&mut commits, filter);
        return Ok(commits);
    }

    /// 历史里出现过的作者，按最近提交的先后去重。
    pub fn authors(&self) -> Result<Vec<String>, GitError> {
        let output = self.run_git(&["log", "--all", "--format=%an"])?;
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

    /// 历史里出现过的作者身份，按最近提交先后按 email 去重。
    pub fn authors_with_email(&self) -> Result<Vec<Author>, GitError> {
        let output = self.run_git(&["log", "--all", "--format=%an%x1f%ae"])?;
        let mut authors: Vec<Author> = Vec::new();
        for line in String::from_utf8_lossy(&output).lines() {
            let Some((name, email)) = line.split_once('\u{1f}') else {
                continue;
            };
            let author = Author {
                name: name.trim().to_string(),
                email: email.trim().to_string(),
            };
            if author.email.is_empty() || author.name.is_empty() {
                continue;
            }
            if !authors
                .iter()
                .any(|existing| existing.email == author.email)
            {
                authors.push(author);
            }
        }
        return Ok(authors);
    }

    /// 一个提交的完整元信息。
    pub fn commit_details(&self, sha: &str) -> Result<CommitDetails, GitError> {
        let format = "--pretty=format:%H%x1f%h%x1f%T%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%cn%x1f%ce%x1f%ct%x1f%s%x1f%b%x1f%D";
        let output = self.run_git(&["show", "-s", "--date=unix", format, sha])?;
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
        let output = self.run_git(&["log", "--date=unix", LOG_FORMAT, range.as_str()])?;
        return parse_log(&String::from_utf8_lossy(&output));
    }

    /// A file's line attribution at `revision`; `--follow` tracks renames backward.
    pub fn blame(&self, path: &str, revision: &str) -> Result<Vec<BlameLine>, GitError> {
        let output = self.run_git(&[
            "blame",
            "--line-porcelain",
            "--follow",
            revision,
            "--",
            path,
        ])?;
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

    /// A file's history through `revision` (following renames), with the path at every commit.
    pub fn file_history(
        &self,
        path: &str,
        revision: &str,
        limit: usize,
    ) -> Result<Vec<FileHistoryEntry>, GitError> {
        let limit_arg = format!("-{}", limit.max(1));
        let output = self.run_git(&[
            "log",
            "--follow",
            "--name-status",
            "-z",
            "--date=unix",
            limit_arg.as_str(),
            FILE_HISTORY_FORMAT,
            revision,
            "--",
            path,
        ])?;
        return parse_file_history(&String::from_utf8_lossy(&output), path);
    }

    /// 读取当前分支的 reflog（最近的在前）。
    pub fn reflog(&self, limit: usize) -> Result<Vec<ReflogEntry>, GitError> {
        let limit_arg = format!("-{}", limit.max(1));
        let output = self.run_git(&[
            "reflog",
            limit_arg.as_str(),
            "--date=unix",
            "--format=%H%x1f%h%x1f%gs%x1f%ct",
        ])?;
        let mut entries = Vec::new();
        for line in String::from_utf8_lossy(&output).lines() {
            let mut fields = line.split('\u{1f}');
            let sha = fields.next().unwrap_or("").to_string();
            if sha.is_empty() {
                continue;
            }
            let short = fields.next().unwrap_or("").to_string();
            let subject = fields.next().unwrap_or("").to_string();
            let timestamp = fields
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
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
}

fn parse_file_history(output: &str, initial_path: &str) -> Result<Vec<FileHistoryEntry>, GitError> {
    let tokens: Vec<&str> = output.split('\0').collect();
    let mut entries = Vec::new();
    let mut current_path = initial_path.to_string();
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index].trim_start_matches('\n');
        if !token.starts_with('\u{1e}') {
            index += 1;
            continue;
        }
        let (record, status) = token.rsplit_once('\n').unwrap_or((token, ""));
        let status = status.trim_start_matches('\n');
        let mut commit = parse_log(record)?
            .into_iter()
            .next()
            .ok_or_else(|| GitError::Parse {
                context: "git file history".to_string(),
                detail: "提交记录为空".to_string(),
            })?;
        let status_kind = status.as_bytes().first().copied().unwrap_or_default();
        let first_path = tokens.get(index + 1).copied().unwrap_or_default();
        let (commit_path, deleted, old_path, new_path, consumed) = match status_kind {
            b'R' | b'C' => {
                let second_path = tokens.get(index + 2).copied().unwrap_or_default();
                (
                    second_path.to_string(),
                    false,
                    Some(first_path),
                    Some(second_path),
                    4,
                )
            }
            b'M' | b'A' | b'D' | b'T' | b'U' | b'X' | b'B' => {
                (current_path.clone(), status_kind == b'D', None, None, 3)
            }
            _ => (current_path.clone(), false, None, None, 1),
        };
        if let (Some(old), Some(new)) = (old_path, new_path)
            && new == current_path
        {
            current_path = old.to_string();
        }
        // Keep the subject in sync with the normal log parser if the format ever changes.
        if commit.summary.is_empty() && !commit.message.is_empty() {
            commit.summary = commit
                .message
                .lines()
                .next()
                .unwrap_or_default()
                .to_string();
        }
        entries.push(FileHistoryEntry {
            commit,
            path: commit_path,
            deleted,
        });
        index += consumed;
    }
    return Ok(entries);
}

fn history_args(
    filter: &LogFilter,
    include_filters: bool,
    stashes: &[(String, String)],
) -> Vec<String> {
    let mut args = vec![
        "log".to_string(),
        "--topo-order".to_string(),
        "--date=unix".to_string(),
        LOG_FORMAT.to_string(),
    ];
    push_revisions(&mut args, filter);
    if reference_filters(filter).is_empty() {
        args.extend(stashes.iter().map(|(_, oid)| oid.clone()));
    }
    if include_filters {
        if let Some(since) = filter.since.as_ref() {
            args.push(format!("--since={since}"));
        }
        if let Some(until) = filter.until.as_ref() {
            args.push(format!("--until={until}"));
        }
        if let Some(author) = filter.author.as_ref().filter(|author| !author.is_empty()) {
            args.push(format!("--author={author}"));
        }
        push_path(&mut args, filter.path.as_deref());
    }
    return args;
}

fn push_revisions(args: &mut Vec<String>, filter: &LogFilter) {
    let references = reference_filters(filter);
    if references.is_empty() {
        args.push("--all".to_string());
    } else {
        args.extend(references);
    }
}

fn reference_filters(filter: &LogFilter) -> Vec<String> {
    return if filter.references.is_empty() {
        filter.reference.iter().cloned().collect::<Vec<_>>()
    } else {
        filter.references.clone()
    }
    .into_iter()
    .filter(|reference| !reference.is_empty())
    .collect::<Vec<_>>();
}

fn decorate_stashes(commits: &mut [Commit], stashes: &[(String, String)]) {
    for commit in commits {
        let Some((reference, _)) = stashes.iter().find(|(_, oid)| oid == &commit.sha) else {
            continue;
        };
        commit.refs.retain(|reference| reference != "refs/stash");
        let label = format!("stash: {reference}");
        if !commit.refs.contains(&label) {
            commit.refs.push(label);
        }
    }
}

/// Adds every local or remote branch whose history contains each visible commit.
/// `graph` must include the branch tips and their ancestors; the visible list may be filtered.
pub fn decorate_commit_branch_history(commits: &mut [Commit], graph: &[Commit], refs: &Refs) {
    if graph.is_empty() {
        return;
    }

    let mut branch_names = refs
        .branches
        .iter()
        .map(|branch| branch.name.clone())
        .collect::<std::collections::HashSet<_>>();
    branch_names.extend(refs.remote_branches.iter().map(RemoteBranch::full_name));
    let current_branch = refs
        .branches
        .iter()
        .find(|branch| branch.current)
        .map(|branch| branch.name.as_str());

    let indexes = graph
        .iter()
        .enumerate()
        .map(|(index, commit)| (commit.sha.as_str(), index))
        .collect::<std::collections::HashMap<_, _>>();
    let mut parent_indexes = vec![Vec::new(); graph.len()];
    let mut remaining_children = vec![0usize; graph.len()];
    for (child_index, commit) in graph.iter().enumerate() {
        for parent in &commit.parents {
            let Some(parent_index) = indexes.get(parent.as_str()).copied() else {
                continue;
            };
            parent_indexes[child_index].push(parent_index);
            remaining_children[parent_index] += 1;
        }
    }

    let mut branch_sets = vec![std::collections::HashSet::<String>::new(); graph.len()];
    for (index, commit) in graph.iter().enumerate() {
        for reference in &commit.refs {
            let branch = reference
                .strip_prefix("HEAD -> ")
                .or_else(|| reference.strip_prefix("refs/heads/"))
                .or_else(|| reference.strip_prefix("refs/remotes/"))
                .unwrap_or(reference.as_str());
            if branch_names.contains(branch) {
                branch_sets[index].insert(branch.to_string());
            }
        }
    }

    let mut ready = remaining_children
        .iter()
        .enumerate()
        .filter_map(|(index, count)| (*count == 0).then_some(index))
        .collect::<std::collections::VecDeque<_>>();
    while let Some(child_index) = ready.pop_front() {
        let child_branches = branch_sets[child_index].clone();
        for parent_index in &parent_indexes[child_index] {
            branch_sets[*parent_index].extend(child_branches.iter().cloned());
            remaining_children[*parent_index] -= 1;
            if remaining_children[*parent_index] == 0 {
                ready.push_back(*parent_index);
            }
        }
    }

    for commit in commits {
        let Some(index) = indexes.get(commit.sha.as_str()).copied() else {
            continue;
        };
        let mut branches = branch_sets[index].iter().cloned().collect::<Vec<_>>();
        branches.sort_by(|left, right| {
            let left_current = current_branch == Some(left.as_str());
            let right_current = current_branch == Some(right.as_str());
            right_current
                .cmp(&left_current)
                .then_with(|| left.cmp(right))
        });
        commit.branches = branches;
    }
}

fn push_path(args: &mut Vec<String>, path: Option<&str>) {
    let Some(path) = path.map(str::trim).filter(|path| !path.is_empty()) else {
        return;
    };
    args.push("--".to_string());
    args.push(format!(":(top,literal){path}"));
}

fn filter_message_and_authors(commits: &mut Vec<Commit>, filter: &LogFilter) {
    if !filter.authors.is_empty() {
        commits.retain(|commit| {
            filter
                .authors
                .iter()
                .any(|email| email.eq_ignore_ascii_case(&commit.author_email))
        });
    }
    let message = filter.message.trim().to_lowercase();
    if !message.is_empty() {
        commits.retain(|commit| commit.message.to_lowercase().contains(&message));
    }
}

fn filter_query(commits: &mut Vec<Commit>, query: &str) {
    if query.is_empty() {
        return;
    }
    let needle = query.to_lowercase();
    commits.retain(|commit| {
        commit.message.to_lowercase().contains(&needle)
            || commit.summary.to_lowercase().contains(&needle)
            || commit.author.to_lowercase().contains(&needle)
            || commit.short.contains(&needle)
            || commit
                .refs
                .iter()
                .any(|reference| reference.to_lowercase().contains(&needle))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::testkit::*;
    use std::process::Command as StdCommand;

    #[test]
    fn branch_history_reaches_ancestor_commits_and_prioritizes_current_branch() {
        let mut visible = vec![Commit {
            sha: String::from("a"),
            ..Default::default()
        }];
        let graph = vec![
            Commit {
                sha: String::from("b"),
                refs: vec![String::from("x")],
                parents: vec![String::from("a")],
                ..Default::default()
            },
            Commit {
                sha: String::from("c"),
                refs: vec![String::from("y")],
                parents: vec![String::from("a")],
                ..Default::default()
            },
            Commit {
                sha: String::from("a"),
                ..Default::default()
            },
        ];
        let refs = Refs {
            branches: vec![
                crate::query::Branch {
                    name: String::from("x"),
                    current: false,
                    ..Default::default()
                },
                crate::query::Branch {
                    name: String::from("y"),
                    current: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        decorate_commit_branch_history(&mut visible, &graph, &refs);

        assert_eq!(
            visible[0].branches,
            vec![String::from("y"), String::from("x")]
        );
    }

    #[test]
    fn shared_ancestor_lists_every_branch_tip_that_reaches_it() {
        let (_guard, path) = fixture("branch-ancestry", &[("a.txt", "base\n")], "ancestor");
        let repo = Repo::open(&path).expect("open");
        run_git(repo.root(), &["branch", "x"]).expect("create x");
        run_git(repo.root(), &["branch", "y"]).expect("create y");
        run_git(repo.root(), &["checkout", "-q", "x"]).expect("checkout x");
        run_git(
            repo.root(),
            &["commit", "-q", "--allow-empty", "-m", "x tip"],
        )
        .expect("x tip");
        run_git(repo.root(), &["checkout", "-q", "y"]).expect("checkout y");
        run_git(
            repo.root(),
            &["commit", "-q", "--allow-empty", "-m", "y tip"],
        )
        .expect("y tip");

        let graph = repo.log_all().expect("all history");
        let refs = repo.refs().expect("refs");
        let mut ancestor = vec![
            graph
                .iter()
                .find(|commit| commit.summary == "ancestor")
                .expect("ancestor commit")
                .clone(),
        ];
        decorate_commit_branch_history(&mut ancestor, &graph, &refs);

        assert_eq!(
            ancestor[0].branches,
            vec![String::from("y"), String::from("main"), String::from("x")]
        );
    }

    #[test]
    fn log_reads_commits_newest_first() {
        let (_guard, path) = fixture("log", &[("a.txt", "one\n")], "first commit");
        let repo = Repo::open(&path).expect("open");
        run_git(
            repo.root(),
            &["commit", "-q", "--allow-empty", "-m", "second commit"],
        )
        .expect("commit");
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
        run_git(
            repo.root(),
            &["commit", "-q", "--allow-empty", "-m", "fix runner"],
        )
        .expect("commit");
        let commits = repo.log(10, "parser").expect("log");
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].summary, "add parser");
        let none = repo.log(10, "nothing-matches").expect("log");
        assert!(none.is_empty());
    }

    #[test]
    fn filtered_log_honours_reference_author_and_keyword() {
        let (_guard, path) = fixture("filtered", &[("a.txt", "one\n")], "first");
        let repo = Repo::open(&path).expect("open");
        run_git(repo.root(), &["checkout", "-q", "-b", "side"]).expect("checkout");
        run_git(
            repo.root(),
            &["commit", "-q", "--allow-empty", "-m", "side work"],
        )
        .expect("commit");
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
    fn history_and_graph_include_every_stash_commit() {
        let (_guard, path) = fixture("stash-history", &[("a.txt", "base\n")], "base");
        let repo = Repo::open(&path).expect("open");
        for (contents, message) in [("first\n", "first stash"), ("second\n", "second stash")] {
            std::fs::write(repo.root().join("a.txt"), contents).expect("change tracked file");
            run_git(repo.root(), &["stash", "push", "-m", message]).expect("stash push");
        }

        let stashes = repo.stash_entries().expect("stash entries");
        assert_eq!(stashes.len(), 2);
        let history = repo.log_history(&LogFilter::default()).expect("history");
        let all = repo.log_all().expect("all commits");
        let graph = repo
            .log_history_graph(&LogFilter::default())
            .expect("graph");
        for (reference, oid) in stashes {
            let stash_label = format!("stash: {reference}");
            assert!(
                history
                    .iter()
                    .any(|commit| { commit.sha == oid && commit.refs.contains(&stash_label) })
            );
            assert!(all.iter().any(|commit| commit.sha == oid));
            assert!(graph.iter().any(|commit| commit.sha == oid));
        }
    }

    #[test]
    fn authors_are_unique_and_ordered() {
        let (_guard, path) = fixture("authors", &[("a.txt", "one\n")], "first");
        let repo = Repo::open(&path).expect("open");
        run_git(
            repo.root(),
            &[
                "-c",
                "user.name=Other",
                "commit",
                "-q",
                "--allow-empty",
                "-m",
                "second",
            ],
        )
        .expect("commit");
        let authors = repo.authors().expect("authors");
        // 按提交顺序（新的在前），因此后提交的作者排在最前面。
        assert_eq!(authors, vec!["Other".to_string(), "Test".to_string()]);
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
        let first = repo.log(1, "").expect("first commit").remove(0);
        std::fs::write(repo.root().join("a.txt"), "one\ntwo\nthree\n").expect("write");
        repo.stage_all().expect("stage");
        repo.commit("add third", "").expect("commit");

        let lines = repo.blame("a.txt", "HEAD").expect("current blame");
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

        let old_lines = repo.blame("a.txt", &first.sha).expect("historical blame");
        assert_eq!(old_lines.len(), 2);
        assert!(old_lines.iter().all(|line| line.text != "three"));
    }

    #[test]
    fn file_history_follows_the_path() {
        let (_guard, path) = fixture("filehist", &[("a.txt", "one\n"), ("b.txt", "b\n")], "first");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "one\ntwo\n").expect("write");
        std::fs::write(repo.root().join("b.txt"), "b\nc\n").expect("write");
        repo.stage_all().expect("stage");
        repo.commit("touch both", "").expect("commit");
        std::fs::rename(repo.root().join("a.txt"), repo.root().join("renamed.txt"))
            .expect("rename");
        repo.stage_all().expect("stage rename");
        repo.commit("rename file", "").expect("rename commit");
        std::fs::remove_file(repo.root().join("renamed.txt")).expect("delete");
        repo.stage_all().expect("stage delete");
        repo.commit("delete file", "").expect("delete commit");

        let head = repo.log(1, "").expect("head").remove(0).sha;
        let history = repo
            .file_history("renamed.txt", &head, 10)
            .expect("history");
        assert_eq!(history.len(), 4);
        assert_eq!(history[0].commit.summary, "delete file");
        assert!(history[0].deleted);
        assert_eq!(history[0].path, "renamed.txt");
        assert_eq!(history[1].commit.summary, "rename file");
        assert_eq!(history[1].path, "renamed.txt");
        assert_eq!(history[2].commit.summary, "touch both");
        assert_eq!(history[2].path, "a.txt");
        assert_eq!(history[3].commit.summary, "first");
        assert_eq!(history[3].path, "a.txt");

        let rename_lines = repo
            .blame("a.txt", history[2].commit.sha.as_str())
            .expect("blame before rename");
        assert_eq!(rename_lines.len(), 2);
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
