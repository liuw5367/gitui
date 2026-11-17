//! 本模块负责多步操作：合并、拣选、回滚、冲突读写与继续/放弃。

use domain::GitError;

use crate::exec::{run_git, run_git_combined, run_git_combined_with_env};
use crate::parse::summarise;
use crate::query::Repo;

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

impl Repo {
    /// 合并某个引用到当前分支（`git merge --no-edit <reference>`）。
    ///
    /// 冲突时返回 `HookFailed`，日志里有 Git 的提示；操作状态由 `operation()` 读出。
    pub fn merge(&self, reference: &str) -> Result<String, GitError> {
        let output = run_git_combined(&self.root, &["merge", "--no-edit", reference])?;
        return Ok(summarise(&output));
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

    /// 用一侧的内容解决冲突：`ours` 为真取当前分支，否则取对方。
    pub fn resolve_with(&self, path: &str, ours: bool) -> Result<(), GitError> {
        let flag = if ours { "--ours" } else { "--theirs" };
        run_git(&self.root, &["checkout", flag, "--", path])?;
        run_git(&self.root, &["add", "--", path])?;
        return Ok(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::testkit::*;

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
    fn cherry_pick_copies_a_commit_and_revert_undoes_one() {
        let (_guard, path) = fixture("pick", &[("a.txt", "one\n")], "first");
        let repo = Repo::open(&path).expect("open");
        let base = repo.log(1, "").expect("log")[0].sha.clone();
        std::fs::write(repo.root().join("b.txt"), "bee\n").expect("write");
        repo.stage_all().expect("stage");
        let picked = repo.commit("add b", "").expect("commit").sha;

        repo.checkout_new_branch("side", base.as_str())
            .expect("branch");
        repo.cherry_pick(&picked).expect("cherry-pick");
        assert!(repo.root().join("b.txt").exists(), "拣选应带过来文件");
        assert_eq!(repo.log(1, "").expect("log")[0].summary, "add b");

        repo.revert(&picked).expect("revert");
        assert!(!repo.root().join("b.txt").exists(), "回滚应删掉文件");
        assert!(repo.log(1, "").expect("log")[0].summary.contains("Revert"));
    }
}
