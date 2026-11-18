//! 本模块负责读补丁与统计：区间 diff、单提交补丁、单文件补丁与逐文件行数。
//!
//! 补丁在 git 边界上就解析成 `domain::diff::Patch`，界面不再解析补丁文本；
//! 原始文本随解析结果一起返回（`Patch::raw`），供导出 patch、复制等要求字节
//! 一致的场合使用。

use domain::diff::Patch;
use domain::{DiffStat, GitError};

use crate::exec::run_git;
use crate::query::Repo;

impl Repo {
    /// 两个提交之间的区间 diff（`from..to`）。
    ///
    /// 用于多提交对比：`from` 为空时表示与工作区比较（`to` 作为起点）。
    ///
    /// 需要原始文本时用返回值的 [`Patch::raw`]。
    pub fn diff_between(&self, from: &str, to: &str) -> Result<Patch, GitError> {
        return self.diff_between_with_whitespace(from, to, false);
    }

    /// 比较两个 revision，可选择忽略空白变化。
    pub fn diff_between_with_whitespace(
        &self,
        from: &str,
        to: &str,
        ignore_whitespace: bool,
    ) -> Result<Patch, GitError> {
        let range = format!("{}..{}", from, to);
        let mut args = vec!["diff", "--no-color"];
        if ignore_whitespace {
            args.push("--ignore-all-space");
        }
        args.push(range.as_str());
        let output = run_git(&self.root, &args)?;
        let text = String::from_utf8_lossy(&output).to_string();
        return Ok(Patch::parse(text.as_str()));
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
        let output = run_git(&self.root, &args)?;
        let text = String::from_utf8_lossy(&output).to_string();
        return Ok(Patch::parse(text.as_str()));
    }

    /// 某个文件的工作区 diff：`staged` 为真时读暂存区（`--cached`）。
    ///
    /// 需要原始文本时用返回值的 [`Patch::raw`]。
    pub fn file_diff(&self, path: &str, staged: bool) -> Result<Patch, GitError> {
        let args: Vec<&str> = if staged {
            vec!["diff", "--no-color", "--cached", "--", path]
        } else {
            vec!["diff", "--no-color", "--", path]
        };
        let output = run_git(&self.root, &args)?;
        let text = String::from_utf8_lossy(&output).to_string();
        return Ok(Patch::parse(text.as_str()));
    }

    /// 一个提交改动的文件与行数统计。
    pub fn numstat(&self, sha: &str) -> Result<Vec<DiffStat>, GitError> {
        let output = run_git(
            &self.root,
            &[
                "show",
                "--numstat",
                "--format=",
                "--root",
                "--diff-merges=first-parent",
                sha,
            ],
        )?;
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
        assert_eq!(stats[0].additions, 1);
        assert_eq!(stats[0].deletions, 0);
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
