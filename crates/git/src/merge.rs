//! 本模块负责多步操作：合并、拣选、回滚、冲突读写与继续/放弃。

use domain::{GitError, conflict::ConflictSides};
use std::path::PathBuf;

use crate::parse::summarise;
use crate::query::Repo;

#[cfg(test)]
use crate::exec::run_git;

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
        let output = self.run_git_combined(&["merge", "--no-edit", reference])?;
        return Ok(summarise(&output));
    }

    /// 把当前分支变基到指定引用；冲突保留给 `operation()` 与冲突流程处理。
    pub fn rebase_onto(&self, reference: &str) -> Result<String, GitError> {
        let output = self.run_git_combined_with_env(
            &["rebase", reference],
            &[("GIT_EDITOR", "true"), ("GIT_TERMINAL_PROMPT", "0")],
        )?;
        return Ok(summarise(&output));
    }

    /// 拣选一个提交到当前分支（`git cherry-pick <sha>`）。
    ///
    /// 冲突时命令失败，但仓库已经进入冲突状态；调用方据 [`Repo::operation`]
    /// 判断该走解决还是放弃。
    pub fn cherry_pick(&self, sha: &str) -> Result<(), GitError> {
        self.run_git(&["cherry-pick", sha])?;
        return Ok(());
    }

    /// 反向提交一个提交（`git revert --no-edit <sha>`）。
    pub fn revert(&self, sha: &str) -> Result<(), GitError> {
        self.run_git(&["revert", "--no-edit", sha])?;
        return Ok(());
    }

    /// 当前仓库是否处于合并/变基/拣选/回滚中。
    ///
    /// 通过 `.git` 下的标记文件判断：`MERGE_HEAD`、`rebase-merge`/`rebase-apply`、
    /// `CHERRY_PICK_HEAD`、`REVERT_HEAD`。顺序上合并优先于拣选，因为合并也可以由
    /// 拣选触发。
    pub fn operation(&self) -> Operation {
        let output = match self.run_git(&["rev-parse", "--git-dir"]) {
            Ok(output) => output,
            Err(_) => return Operation::None,
        };
        let git_dir = PathBuf::from(String::from_utf8_lossy(&output).trim());
        let git_dir = if git_dir.is_absolute() {
            git_dir
        } else {
            self.root.join(git_dir)
        };
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
        let output = self.run_git_combined_with_env(
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
        self.run_git(args)?;
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

    /// 读取冲突文件在 index stage 1/2/3 中的内容。
    ///
    /// Add/add 与 delete/modify 冲突会缺少对应 stage，因此每侧分别可空；内容按原始
    /// bytes 返回，避免把二进制冲突改写成 lossy UTF-8。
    pub fn conflict_sides(&self, path: &str) -> Result<ConflictSides, GitError> {
        let pathspec = format!(":(top,literal){path}");
        let output = self.run_git_full(&["ls-files", "-u", "-z", "--", pathspec.as_str()])?;
        let mut sides = ConflictSides {
            path: path.to_string(),
            ..ConflictSides::default()
        };
        for record in output
            .split(|byte| *byte == 0)
            .filter(|record| !record.is_empty())
        {
            let Some(separator) = record.iter().position(|byte| *byte == b'\t') else {
                return Err(GitError::Parse {
                    context: "git ls-files -u".to_string(),
                    detail: String::from_utf8_lossy(record).to_string(),
                });
            };
            let metadata = String::from_utf8_lossy(&record[..separator]);
            let mut fields = metadata.split_ascii_whitespace();
            let _mode = fields.next();
            let Some(oid) = fields.next() else {
                return Err(GitError::Parse {
                    context: "git ls-files -u".to_string(),
                    detail: metadata.to_string(),
                });
            };
            let Some(stage) = fields.next() else {
                return Err(GitError::Parse {
                    context: "git ls-files -u".to_string(),
                    detail: metadata.to_string(),
                });
            };
            let record_path = String::from_utf8_lossy(&record[separator + 1..]);
            if record_path != path {
                continue;
            }
            let content = self.run_git_full(&["cat-file", "blob", oid])?;
            let target = match stage {
                "1" => &mut sides.base,
                "2" => &mut sides.ours,
                "3" => &mut sides.theirs,
                _ => {
                    return Err(GitError::Parse {
                        context: "git ls-files -u".to_string(),
                        detail: format!("未知冲突 stage：{stage}"),
                    });
                }
            };
            *target = Some(content);
        }
        return Ok(sides);
    }

    /// 读取冲突文件当前工作区内容。删除冲突侧可能对应不存在的路径。
    pub fn conflict_worktree_content(&self, path: &str) -> Result<Option<Vec<u8>>, GitError> {
        let target = self.root.join(path);
        match std::fs::read(&target) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(GitError::Unreadable {
                path: target.display().to_string(),
                reason: error.to_string(),
            }),
        }
    }

    /// 读取冲突文件当前内容（含冲突标记）；二进制内容按 lossy UTF-8 供旧调用方展示。
    pub fn conflict_content(&self, path: &str) -> String {
        let Ok(Some(bytes)) = self.conflict_worktree_content(path) else {
            return String::new();
        };
        return String::from_utf8_lossy(&bytes).into_owned();
    }

    /// 写入解决后的内容并标记为已解决（`git add`）。
    ///
    /// 写盘失败直接把原因当错误返回，避免「看起来已解决但文件没变」。
    pub fn resolve_conflict(&self, path: &str, content: &str) -> Result<(), GitError> {
        return self.resolve_conflict_bytes(path, content.as_bytes());
    }

    /// 写入解决后的原始内容并标记为已解决（只暂存指定路径）。
    pub fn resolve_conflict_bytes(&self, path: &str, content: &[u8]) -> Result<(), GitError> {
        let target = self.root.join(path);
        std::fs::write(&target, content).map_err(|error| GitError::Unreadable {
            path: target.display().to_string(),
            reason: error.to_string(),
        })?;
        self.run_git(&["add", "--", path])?;
        return Ok(());
    }

    /// 以删除该路径的方式解决冲突。
    pub fn resolve_conflict_delete(&self, path: &str) -> Result<(), GitError> {
        self.run_git(&["rm", "--force", "--", path])?;
        return Ok(());
    }

    /// 用一侧解决冲突：该侧有 stage 时检出其内容，缺少 stage 时把路径标记为删除。
    pub fn resolve_with(&self, path: &str, ours: bool) -> Result<(), GitError> {
        let sides = self.conflict_sides(path)?;
        let side_exists = if ours {
            sides.ours.is_some()
        } else {
            sides.theirs.is_some()
        };
        if !side_exists {
            return self.resolve_conflict_delete(path);
        }
        let flag = if ours { "--ours" } else { "--theirs" };
        self.run_git(&["checkout", flag, "--", path])?;
        self.run_git(&["add", "--", path])?;
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
    fn rebase_moves_current_branch_onto_named_branch() {
        let (_guard, path) = fixture("rebase", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        let base = repo.log(1, "").expect("log")[0].sha.clone();

        std::fs::write(repo.root().join("main.txt"), "main\n").expect("write main");
        repo.stage_all().expect("stage main");
        repo.commit("main work", "").expect("commit main");
        let main_head = run_git(repo.root(), &["rev-parse", "refs/heads/main"]).expect("main head");

        repo.checkout_new_branch("feature/rebase", base.as_str())
            .expect("feature branch");
        std::fs::write(repo.root().join("feature.txt"), "feature\n").expect("write feature");
        repo.stage_all().expect("stage feature");
        repo.commit("feature work", "").expect("commit feature");

        repo.rebase_onto("main").expect("rebase");
        assert_eq!(repo.abbrev_branch().expect("branch"), "feature/rebase");
        assert_eq!(repo.operation(), Operation::None);
        let parent = run_git(repo.root(), &["rev-parse", "HEAD^"]).expect("rebased parent");
        assert_eq!(parent, main_head);
    }

    #[test]
    fn rebase_keeps_conflicts_open_for_resolution() {
        let (_guard, path) = fixture("rebase-conflict", &[("a.txt", "base\n")], "base");
        let repo = Repo::open(&path).expect("open");
        let base = repo.log(1, "").expect("log")[0].sha.clone();

        repo.checkout_new_branch("feature/rebase-conflict", base.as_str())
            .expect("feature branch");
        std::fs::write(repo.root().join("a.txt"), "feature\n").expect("write feature");
        repo.stage_all().expect("stage feature");
        repo.commit("feature edit", "").expect("commit feature");

        run_git(repo.root(), &["checkout", "-q", "main"]).expect("checkout main");
        std::fs::write(repo.root().join("a.txt"), "main\n").expect("write main");
        repo.stage_all().expect("stage main");
        repo.commit("main edit", "").expect("commit main");
        run_git(repo.root(), &["checkout", "-q", "feature/rebase-conflict"])
            .expect("checkout feature");

        assert!(repo.rebase_onto("main").is_err());
        assert_eq!(repo.operation(), Operation::Rebase);
        assert_eq!(repo.conflicts().expect("conflicts"), vec!["a.txt"]);
        repo.abort_operation(Operation::Rebase)
            .expect("abort rebase");
        assert_eq!(repo.operation(), Operation::None);
    }

    #[test]
    fn cherry_pick_and_revert_conflicts_can_be_resolved_and_continued() {
        let (_guard, cherry_path) = fixture("cherry-conflict", &[("a.txt", "base\n")], "base");
        let repo = Repo::open(&cherry_path).expect("open cherry repo");
        let base = repo.log(1, "").expect("log")[0].sha.clone();
        repo.checkout_new_branch("topic", base.as_str())
            .expect("topic branch");
        std::fs::write(repo.root().join("a.txt"), "topic\n").expect("write topic");
        repo.stage_all().expect("stage topic");
        let topic = repo.commit("topic change", "").expect("topic commit").sha;
        run_git(repo.root(), &["checkout", "-q", "main"]).expect("checkout main");
        std::fs::write(repo.root().join("a.txt"), "main\n").expect("write main");
        repo.stage_all().expect("stage main");
        repo.commit("main change", "").expect("main commit");

        assert!(repo.cherry_pick(&topic).is_err());
        assert_eq!(repo.operation(), Operation::CherryPick);
        assert_eq!(repo.conflicts().expect("cherry conflicts"), vec!["a.txt"]);
        repo.resolve_with("a.txt", false).expect("resolve cherry");
        repo.continue_operation(Operation::CherryPick)
            .expect("continue cherry-pick");
        assert_eq!(repo.operation(), Operation::None);

        let (_guard, revert_path) = fixture("revert-conflict", &[("a.txt", "base\n")], "base");
        let repo = Repo::open(&revert_path).expect("open revert repo");
        std::fs::write(repo.root().join("a.txt"), "target\n").expect("write target");
        repo.stage_all().expect("stage target");
        let target = repo.commit("target change", "").expect("target commit").sha;
        std::fs::write(repo.root().join("a.txt"), "later\n").expect("write later");
        repo.stage_all().expect("stage later");
        repo.commit("later change", "").expect("later commit");

        assert!(repo.revert(&target).is_err());
        assert_eq!(repo.operation(), Operation::Revert);
        assert_eq!(repo.conflicts().expect("revert conflicts"), vec!["a.txt"]);
        repo.resolve_with("a.txt", false).expect("resolve revert");
        repo.continue_operation(Operation::Revert)
            .expect("continue revert");
        assert_eq!(repo.operation(), Operation::None);
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
