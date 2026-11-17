//! 本模块负责提交与改写历史：提交、改写说明、合并提交、重置与改写前置检查。

use domain::GitError;

use crate::error::rewrite_error;
use crate::exec::{run_git, run_git_combined_with_env};
use crate::merge::Operation;
use crate::query::Repo;

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
    /// 允许空提交（`--allow-empty`），空仓库的第一次提交需要它。
    pub allow_empty: bool,
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
        if self.allow_empty {
            flags.push("--allow-empty");
        }
        return flags;
    }
}

/// 改写历史时一行的动作。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RewriteMark {
    Pick,
    Reword,
    Squash,
}

impl RewriteMark {
    fn command(self) -> &'static str {
        return match self {
            RewriteMark::Pick => "pick",
            RewriteMark::Reword => "reword",
            RewriteMark::Squash => "squash",
        };
    }
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

impl Repo {
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

    /// 改写某个提交的说明，不改内容（脚本化 `git rebase -i`）。
    ///
    /// 前置：工作区干净、没有进行中的操作、目标提交在 HEAD 的历史上。任一条不满足
    /// 就直接返回可读错误，不启动 rebase。
    pub fn reword_commit(&self, sha: &str, message: &str) -> Result<(), GitError> {
        let target = self.full_sha(sha)?;
        self.ensure_clean_for_rewrite()?;
        if !self.is_ancestor(target.as_str(), "HEAD")? {
            return Err(rewrite_error(format!("{sha} 不在当前分支的历史上")));
        }
        let base = format!("{target}^");
        return self.rewrite(&base, &[(target, RewriteMark::Reword)], message);
    }

    /// 把 `from..=to` 这一段合成一个提交，说明用 `message`。
    ///
    /// 只接受连续、不含合并提交的区间；`to` 之后的提交原样保留。
    pub fn squash_commits(&self, from: &str, to: &str, message: &str) -> Result<(), GitError> {
        let first = self.full_sha(from)?;
        let last = self.full_sha(to)?;
        self.ensure_clean_for_rewrite()?;
        if first == last {
            return Err(rewrite_error("至少要选两个提交才能合并".to_string()));
        }
        if !self.is_ancestor(first.as_str(), last.as_str())? {
            return Err(rewrite_error(format!(
                "{from} 不是 {to} 的祖先，无法合并这一段"
            )));
        }
        if !self.is_ancestor(last.as_str(), "HEAD")? {
            return Err(rewrite_error(format!("{to} 不在当前分支的历史上")));
        }
        let base = format!("{first}^");
        // 区间内的提交从旧到新列出；第一条 pick，其余 squash。
        // 用两个参数形式（`rev-list <base>..<last>` 在某些 SHA 组合下会被当成路径）。
        let range = format!("{}..{}", base, last);
        let shas: Vec<String> = self.lines(&["rev-list", "--reverse", range.as_str()])?;
        if shas.len() < 2 {
            return Err(rewrite_error("至少要选两个提交才能合并".to_string()));
        }
        let mut marks: Vec<(String, RewriteMark)> = Vec::new();
        for (index, sha) in shas.iter().enumerate() {
            let mark = if index == 0 {
                RewriteMark::Pick
            } else {
                RewriteMark::Squash
            };
            marks.push((sha.clone(), mark));
        }
        return self.rewrite(&base, &marks, message);
    }

    /// 当前分支尖端是否等于 `sha`（界面上用来提示「只能改到分支尖端」）。
    pub fn is_tip(&self, sha: &str) -> bool {
        let target = match self.full_sha(sha) {
            Ok(target) => target,
            Err(_) => return false,
        };
        return self
            .full_sha("HEAD")
            .map(|head| head == target)
            .unwrap_or(false);
    }

    /// 工作区是否干净到可以改写历史。
    pub fn can_rewrite(&self) -> Result<(), String> {
        if self.operation() != Operation::None {
            return Err("有进行中的操作，先完成或放弃它".to_string());
        }
        let entries = self.status().map_err(|error| error.detail())?;
        if !entries.is_empty() {
            return Err("工作区有未提交的改动，先提交或贮藏".to_string());
        }
        return Ok(());
    }

    fn ensure_clean_for_rewrite(&self) -> Result<(), GitError> {
        return self.can_rewrite().map_err(rewrite_error);
    }

    fn full_sha(&self, reference: &str) -> Result<String, GitError> {
        let output = run_git(&self.root, &["rev-parse", reference])?;
        return Ok(String::from_utf8_lossy(&output).trim().to_string());
    }

    fn is_ancestor(&self, ancestor: &str, descendant: &str) -> Result<bool, GitError> {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["merge-base", "--is-ancestor", ancestor, descendant])
            .env("GIT_TERMINAL_PROMPT", "0")
            .status()
            .map_err(|error| GitError::Spawn {
                program: "git".to_string(),
                reason: error.to_string(),
            })?;
        // 0 = 是祖先，1 = 不是，其它按失败处理。
        return Ok(status.code() == Some(0));
    }

    /// 生成 todo 与提交说明，跑一次脚本化 `rebase -i`；失败一律回滚。
    fn rewrite(
        &self,
        base: &str,
        marks: &[(String, RewriteMark)],
        message: &str,
    ) -> Result<(), GitError> {
        // todo 由我们自己写全，不用 sed 改 git 生成的模板。
        let shas: Vec<String> =
            self.lines(&["rev-list", "--reverse", format!("{base}..HEAD").as_str()])?;
        if shas.is_empty() {
            return Err(rewrite_error("没有需要重写的提交".to_string()));
        }
        let mut todo = String::new();
        for sha in &shas {
            let mark = marks
                .iter()
                .find(|(target, _)| target == sha)
                .map(|(_, mark)| *mark)
                .unwrap_or(RewriteMark::Pick);
            todo.push_str(mark.command());
            todo.push(' ');
            todo.push_str(sha);
            todo.push('\n');
        }
        let dir = std::env::temp_dir();
        let stamp = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        );
        let todo_path = dir.join(format!("gitclient-todo-{stamp}.txt"));
        let editor_path = dir.join(format!("gitclient-editor-{stamp}.sh"));
        let sequence_path = dir.join(format!("gitclient-sequence-{stamp}.sh"));
        std::fs::write(&todo_path, &todo).map_err(|error| GitError::Unreadable {
            path: todo_path.display().to_string(),
            reason: error.to_string(),
        })?;
        // 序列编辑器：把 git 的模板整体覆盖成我们的 todo。
        let sequence = format!("#!/bin/sh\ncat \"{}\" > \"$1\"\n", todo_path.display());
        std::fs::write(&sequence_path, sequence).map_err(|error| GitError::Unreadable {
            path: sequence_path.display().to_string(),
            reason: error.to_string(),
        })?;
        // 提交信息编辑器：写入用户给的说明。
        let message_path = dir.join(format!("gitclient-message-{stamp}.txt"));
        std::fs::write(&message_path, message).map_err(|error| GitError::Unreadable {
            path: message_path.display().to_string(),
            reason: error.to_string(),
        })?;
        let editor = format!("#!/bin/sh\ncat \"{}\" > \"$1\"\n", message_path.display());
        std::fs::write(&editor_path, editor).map_err(|error| GitError::Unreadable {
            path: editor_path.display().to_string(),
            reason: error.to_string(),
        })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for path in [&sequence_path, &editor_path] {
                let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755));
            }
        }
        let result = run_git_combined_with_env(
            &self.root,
            &["rebase", "-i", base],
            &[
                (
                    "GIT_SEQUENCE_EDITOR",
                    sequence_path.to_str().unwrap_or("true"),
                ),
                ("GIT_EDITOR", editor_path.to_str().unwrap_or("true")),
                ("GIT_TERMINAL_PROMPT", "0"),
            ],
        );
        for path in [&todo_path, &sequence_path, &editor_path, &message_path] {
            let _ = std::fs::remove_file(path);
        }
        if result.is_err() {
            // 失败就回到改写前，别把仓库留在半截状态。
            let _ = run_git(&self.root, &["rebase", "--abort"]);
        }
        result?;
        return Ok(());
    }

    /// 把当前分支重置到某个提交（`git reset --hard <sha>`）。
    ///
    /// 用于 reflog 的撤销；调用方必须先确认，未提交的改动会被丢弃。
    pub fn reset_hard(&self, sha: &str) -> Result<(), GitError> {
        run_git(&self.root, &["reset", "--hard", sha])?;
        return Ok(());
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::testkit::*;
    use std::path::Path;

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
                assert!(
                    log.contains("lint failed"),
                    "钩子输出应包含在日志里：{}",
                    log
                );
            }
            other => panic!("期望 HookFailed，实际 {:?}", other),
        }
        // 失败后暂存区保留，历史没有新增提交。
        assert_eq!(repo.log(10, "").expect("log").len(), 1);
    }

    #[test]
    fn commit_options_map_to_flags_in_fixed_order() {
        assert!(CommitOptions::default().flags().is_empty());
        let all = CommitOptions {
            amend: true,
            signoff: true,
            no_verify: true,
            allow_empty: true,
        };
        assert_eq!(
            all.flags(),
            vec!["--amend", "--signoff", "--no-verify", "--allow-empty"]
        );
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
        assert!(
            repo.status()
                .expect("status")
                .iter()
                .any(|entry| entry.is_staged())
        );

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
    fn reword_rewrites_one_message_and_keeps_the_tree() {
        let (_guard, path) = fixture("reword", &[("a.txt", "one\n")], "first");
        let repo = Repo::open(&path).expect("open");
        for (file, message) in [("b.txt", "second"), ("c.txt", "third")] {
            std::fs::write(repo.root().join(file), "x\n").expect("write");
            repo.stage_all().expect("stage");
            repo.commit(message, "").expect("commit");
        }
        let before = repo.log(10, "").expect("log");
        assert_eq!(before.len(), 3);
        let tree_before = run_git(repo.root(), &["rev-parse", "HEAD^{tree}"]).expect("tree");
        let target = before[1].sha.clone();

        repo.reword_commit(target.as_str(), "second (reworded)")
            .expect("reword");

        let after = repo.log(10, "").expect("log");
        assert_eq!(after.len(), 3, "改写不改变提交数");
        // 顺序是新的在前：第三条（原 second）现在是 after[1]。
        assert_eq!(after[1].summary, "second (reworded)");
        // 内容没变：树对象一致。
        let tree_after = run_git(repo.root(), &["rev-parse", "HEAD^{tree}"]).expect("tree");
        assert_eq!(tree_before, tree_after, "改写说明不应改动内容");
        // HEAD 之后的提交被重放，SHA 会变但说明保留。
        assert_eq!(after[0].summary, "third");
        assert_eq!(repo.operation(), Operation::None, "不应留下进行中的操作");
    }

    #[test]
    fn squash_squashes_a_contiguous_range() {
        let (_guard, path) = fixture("squash", &[("a.txt", "one\n")], "first");
        let repo = Repo::open(&path).expect("open");
        for (file, message) in [("b.txt", "second"), ("c.txt", "third")] {
            std::fs::write(repo.root().join(file), "x\n").expect("write");
            repo.stage_all().expect("stage");
            repo.commit(message, "").expect("commit");
        }
        let before = repo.log(10, "").expect("log");
        let tree_before = run_git(repo.root(), &["rev-parse", "HEAD^{tree}"]).expect("tree");
        // 合并最近两个（third + second）。
        let newest = before[0].sha.clone();
        let older = before[1].sha.clone();
        repo.squash_commits(older.as_str(), newest.as_str(), "squashed")
            .expect("squash");

        let after = repo.log(10, "").expect("log");
        assert_eq!(after.len(), 2, "两个并成一个");
        assert_eq!(after[0].summary, "squashed");
        assert_eq!(after[1].summary, "first");
        let tree_after = run_git(repo.root(), &["rev-parse", "HEAD^{tree}"]).expect("tree");
        assert_eq!(tree_before, tree_after, "合并后内容必须一致");
        // 所有文件都还在。
        for file in ["a.txt", "b.txt", "c.txt"] {
            assert!(repo.root().join(file).exists(), "{file} 应保留");
        }
    }

    #[test]
    fn rewriting_is_refused_on_a_dirty_worktree() {
        let (_guard, path) = fixture("dirty-rewrite", &[("a.txt", "one\n")], "first");
        let repo = Repo::open(&path).expect("open");
        let head = repo.log(1, "").expect("log")[0].sha.clone();
        std::fs::write(repo.root().join("a.txt"), "changed\n").expect("write");

        let error = repo.reword_commit(head.as_str(), "nope");
        assert!(error.is_err(), "脏工作区必须拒绝改写");
        let detail = error.unwrap_err().detail();
        assert!(detail.contains("未提交"), "错误应说明原因：{detail}");
        assert_eq!(
            repo.log(1, "").expect("log")[0].summary,
            "first",
            "拒绝时不应改动历史"
        );
    }

    #[test]
    fn squash_needs_two_commits() {
        let (_guard, path) = fixture("squash-single", &[("a.txt", "one\n")], "only");
        let repo = Repo::open(&path).expect("open");
        let head = repo.log(1, "").expect("log")[0].sha.clone();
        let error = repo.squash_commits(head.as_str(), head.as_str(), "nope");
        assert!(error.is_err(), "单提交不该允许合并：{:?}", error);
        let detail = error.unwrap_err().detail();
        assert!(
            detail.contains("两个提交"),
            "错误应说明原因，实际：{detail}"
        );
    }
}
