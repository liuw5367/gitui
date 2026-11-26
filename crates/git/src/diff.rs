//! 本模块负责读补丁与统计：区间 diff、单提交补丁、单文件补丁与逐文件行数。
//!
//! 补丁在 git 边界上就解析成 `domain::diff::Patch`，界面不再解析补丁文本；
//! 原始文本随解析结果一起返回（`Patch::raw`），供导出 patch、复制等要求字节
//! 一致的场合使用。

use domain::diff::Patch;
use domain::{DiffStat, GitError};

use crate::query::Repo;

#[cfg(test)]
use crate::exec::run_git;

impl Repo {
    /// 比较两个 tree-ish 端点；`from` 为空时以 `to` 为基准比较当前工作区。
    ///
    /// 需要原始文本时用返回值的 [`Patch::raw`]。
    pub fn diff_between(&self, from: &str, to: &str) -> Result<Patch, GitError> {
        return self.diff_between_with_whitespace(from, to, false);
    }

    /// 比较两个 revision/tree-ish 端点，可选择忽略空白变化。
    pub fn diff_between_with_whitespace(
        &self,
        from: &str,
        to: &str,
        ignore_whitespace: bool,
    ) -> Result<Patch, GitError> {
        if from.is_empty() {
            return self.diff_worktree_with_whitespace(to, ignore_whitespace);
        }
        let mut args = vec!["diff", "--no-color"];
        if ignore_whitespace {
            args.push("--ignore-all-space");
        }
        args.push(from);
        args.push(to);
        return self.patch_with_full_context(&args);
    }

    /// Merge base for two commits. Unrelated histories return `Ok(None)`.
    pub fn merge_base(&self, left: &str, right: &str) -> Result<Option<String>, GitError> {
        let output = match self.run_git(&["merge-base", left, right]) {
            Ok(output) => output,
            Err(GitError::CommandFailed { code: Some(1), .. }) => return Ok(None),
            Err(error) => return Err(error),
        };
        return Ok(Some(String::from_utf8_lossy(&output).trim().to_string()));
    }

    /// Compare two tree-ish endpoints directly (2-dot semantics).
    pub fn diff_trees_with_whitespace(
        &self,
        base: &str,
        target: &str,
        ignore_whitespace: bool,
    ) -> Result<Patch, GitError> {
        return self.diff_between_with_whitespace(base, target, ignore_whitespace);
    }

    /// Compare a commit or ref with the current working tree, including staged and unstaged
    /// tracked changes. Git's regular `diff <revision>` does not include untracked files.
    pub fn diff_worktree_with_whitespace(
        &self,
        revision: &str,
        ignore_whitespace: bool,
    ) -> Result<Patch, GitError> {
        let mut args = vec!["diff", "--no-color"];
        if ignore_whitespace {
            args.push("--ignore-all-space");
        }
        args.push(revision);
        return self.patch_with_full_context(&args);
    }

    /// 某个提交里单个文件的补丁。
    ///
    /// 需要原始文本时用返回值的 [`Patch::raw`]。
    pub fn patch_for_path(&self, sha: &str, path: &str) -> Result<Patch, GitError> {
        return self.patch_for_path_with_whitespace(sha, path, false);
    }

    /// 某个提交里单个文件的补丁，可选择忽略空白变化。
    pub fn patch_for_path_with_whitespace(
        &self,
        sha: &str,
        path: &str,
        ignore_whitespace: bool,
    ) -> Result<Patch, GitError> {
        return self.show_commit_patch(sha, Some(path), ignore_whitespace);
    }

    /// 一个提交的补丁文本（不含 commit 头）。
    ///
    /// 需要原始文本时用返回值的 [`Patch::raw`]。
    pub fn patch(&self, sha: &str) -> Result<Patch, GitError> {
        return self.patch_with_whitespace(sha, false);
    }

    /// 某个提交相对于其第一父提交的补丁；根提交与空 tree 比较。
    ///
    /// Git 默认可能以 combined diff 显示 merge commit，这里固定第一父提交语义。
    pub fn patch_with_whitespace(
        &self,
        sha: &str,
        ignore_whitespace: bool,
    ) -> Result<Patch, GitError> {
        return self.show_commit_patch(sha, None, ignore_whitespace);
    }

    fn show_commit_patch(
        &self,
        sha: &str,
        path: Option<&str>,
        ignore_whitespace: bool,
    ) -> Result<Patch, GitError> {
        let mut args = vec![
            "show",
            "--no-color",
            "--patch",
            "--format=",
            "--root",
            "--diff-merges=first-parent",
        ];
        if ignore_whitespace {
            args.push("--ignore-all-space");
        }
        args.push(sha);
        if let Some(path) = path {
            args.push("--");
            args.push(path);
        }
        return self.patch_with_full_context(&args);
    }

    /// 某个文件的工作区 diff：`staged` 为真时读暂存区（`--cached`）。
    ///
    /// 需要原始文本时用返回值的 [`Patch::raw`]。
    pub fn file_diff(&self, path: &str, staged: bool) -> Result<Patch, GitError> {
        return self.file_diff_with_whitespace(path, staged, false);
    }

    /// 某个文件的工作区 diff，可选择忽略纯空白差异。
    pub fn file_diff_with_whitespace(
        &self,
        path: &str,
        staged: bool,
        ignore_whitespace: bool,
    ) -> Result<Patch, GitError> {
        let mut args = vec!["diff", "--no-color"];
        if staged {
            args.push("--cached");
        }
        if ignore_whitespace {
            args.push("--ignore-all-space");
        }
        args.push("--");
        args.push(path);
        return self.patch_with_full_context(&args);
    }

    /// 保留 Git 默认上下文补丁，并额外读一份足够大上下文的补丁供全文显示。
    pub(crate) fn patch_with_full_context(&self, args: &[&str]) -> Result<Patch, GitError> {
        let output = self.run_git(args)?;
        let text = String::from_utf8_lossy(&output).to_string();
        let mut patch = Patch::parse(text.as_str());

        let mut full_args = args.to_vec();
        let option_index = if args.first() == Some(&"stash") { 2 } else { 1 };
        full_args.insert(option_index, "--unified=999999999");
        if let Ok(output) = self.run_git_full(&full_args) {
            patch.full_context = Some(String::from_utf8_lossy(&output).to_string());
        }
        return Ok(patch);
    }

    /// 一个提交改动的文件与行数统计。
    pub fn numstat(&self, sha: &str) -> Result<Vec<DiffStat>, GitError> {
        let output = self.run_git(&[
            "show",
            "--numstat",
            "--no-renames",
            "-z",
            "--format=",
            "--root",
            "--diff-merges=first-parent",
            sha,
        ])?;
        let mut stats = Vec::new();
        for record in output
            .split(|byte| *byte == 0)
            .filter(|record| !record.is_empty())
        {
            let line = String::from_utf8_lossy(record);
            let mut parts = line.splitn(3, '\t');
            let additions = parts.next().unwrap_or("");
            let deletions = parts.next().unwrap_or("");
            let path = parts.next().unwrap_or("");
            stats.push(DiffStat {
                status: 'M',
                path: path.to_string(),
                additions: additions.parse().unwrap_or(0),
                deletions: deletions.parse().unwrap_or(0),
            });
        }
        // name-status 提供真实文件类型，不能从增删行数推断新增或删除（空文件和二进制也有状态）。
        let output = self.run_git(&[
            "show",
            "--name-status",
            "-z",
            "--no-renames",
            "--format=",
            "--root",
            "--diff-merges=first-parent",
            sha,
        ])?;
        let fields = output.split(|byte| *byte == 0).collect::<Vec<_>>();
        for pair in fields.chunks_exact(2) {
            let status = pair[0].first().copied().unwrap_or(b'M') as char;
            let path = String::from_utf8_lossy(pair[1]);
            if let Some(file) = stats.iter_mut().find(|file| file.path == path) {
                file.status = status;
            }
        }
        return Ok(stats);
    }

    /// 工作区或暂存区的逐文件统计；`staged` 为真时读 `--cached`。
    pub fn numstat_section(&self, staged: bool) -> Result<Vec<DiffStat>, GitError> {
        let args: Vec<&str> = if staged {
            vec!["diff", "--numstat", "--cached"]
        } else {
            vec!["diff", "--numstat"]
        };
        let output = self.run_git(&args)?;
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
                status: 'M',
                path: path.to_string(),
                additions: additions.parse().unwrap_or(0),
                deletions: deletions.parse().unwrap_or(0),
            });
        }

        if !staged {
            let output = self.run_git(&["ls-files", "--others", "--exclude-standard", "-z"])?;
            for path in output
                .split(|byte| *byte == 0)
                .filter(|path| !path.is_empty())
            {
                let path = String::from_utf8_lossy(path).to_string();
                let diff = self.run_git_allow_exit_code(
                    &[
                        "diff",
                        "--no-index",
                        "--numstat",
                        "-z",
                        "--",
                        "/dev/null",
                        path.as_str(),
                    ],
                    1,
                )?;
                let Some(record) = diff.split(|byte| *byte == 0).find(|line| !line.is_empty())
                else {
                    continue;
                };
                let line = String::from_utf8_lossy(record);
                let mut parts = line.splitn(3, '\t');
                let additions = parts.next().unwrap_or("");
                let deletions = parts.next().unwrap_or("");
                if parts.next().is_none() {
                    continue;
                }
                stats.push(DiffStat {
                    status: 'A',
                    path,
                    additions: additions.parse().unwrap_or(0),
                    deletions: deletions.parse().unwrap_or(0),
                });
            }
        }
        return Ok(stats);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::testkit::*;

    #[test]
    fn patch_and_numstat_describe_one_commit() {
        let (_guard, path) = fixture("patch", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "one\ntwo\n").expect("write");
        run_git(repo.root(), &["commit", "-q", "-am", "add line"]).expect("commit");
        let head = repo.log(1, "").expect("log").remove(0);
        let patch = repo.patch(&head.sha).expect("patch");
        assert_eq!(patch.files.len(), 1);
        assert!(patch.lines.iter().any(|line| line.text == "two"));
        // 导出一类的调用方要字节一致的原文，随解析结果一起返回。
        assert!(patch.raw.contains("+two"));
        let stats = repo.numstat(&head.sha).expect("numstat");
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].path, "a.txt");
        assert_eq!(stats[0].status, 'M');
        assert_eq!(stats[0].additions, 1);
        assert_eq!(stats[0].deletions, 0);
    }

    #[test]
    fn commit_stats_preserve_empty_binary_and_unicode_file_status() {
        let (_guard, path) = fixture(
            "stat-status",
            &[("deleted.txt", "old\n"), ("modified.txt", "base\n")],
            "base",
        );
        let repo = Repo::open(&path).expect("open");
        std::fs::remove_file(repo.root().join("deleted.txt")).unwrap();
        std::fs::write(repo.root().join("modified.txt"), "base\nmore\n").unwrap();
        std::fs::write(repo.root().join("空文件.txt"), "").unwrap();
        std::fs::write(repo.root().join("binary.dat"), [0, 1, 2]).unwrap();
        run_git(repo.root(), &["add", "-A"]).unwrap();
        run_git(repo.root(), &["commit", "-qm", "status"]).unwrap();
        let stats = repo.numstat("HEAD").unwrap();
        for (path, status) in [
            ("deleted.txt", 'D'),
            ("modified.txt", 'M'),
            ("空文件.txt", 'A'),
            ("binary.dat", 'A'),
        ] {
            assert_eq!(
                stats.iter().find(|file| file.path == path).unwrap().status,
                status
            );
        }
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
        assert_eq!(patch.files.len(), 1);
        assert_eq!(patch.files[0].path, "a.txt");
        assert!(patch.raw.contains("a.txt"));
        assert!(!patch.raw.contains("b.txt"));
        assert!(patch.raw.contains("+two"));
    }

    #[test]
    fn file_diff_reads_worktree_and_index() {
        let (_guard, path) = fixture("filediff", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "two\n").expect("write");

        let worktree = repo.file_diff("a.txt", false).expect("worktree diff");
        assert!(!worktree.lines.is_empty());
        assert!(worktree.raw.contains("-one"));
        assert!(worktree.raw.contains("+two"));
        assert!(
            repo.file_diff("a.txt", true)
                .expect("index diff")
                .lines
                .is_empty()
        );

        repo.stage(&["a.txt".to_string()]).expect("stage");
        let staged = repo.file_diff("a.txt", true).expect("index diff");
        assert!(!staged.lines.is_empty());
        assert!(staged.raw.contains("+two"));
        assert!(
            repo.file_diff("a.txt", false)
                .expect("worktree diff")
                .lines
                .is_empty()
        );
    }
}
