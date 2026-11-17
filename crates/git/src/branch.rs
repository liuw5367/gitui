//! 本模块负责引用与配置的写操作：分支、标签、远程、仓库级配置与贮藏。

use domain::GitError;

use crate::exec::run_git;
use crate::query::Repo;

impl Repo {
    /// 从某个引用新建并切换到本地分支（`git checkout -b <name> <from>`）。
    pub fn checkout_new_branch(&self, name: &str, from: &str) -> Result<(), GitError> {
        run_git(&self.root, &["checkout", "-b", name, from])?;
        return Ok(());
    }

    /// 从远端跟踪分支创建本地跟踪分支并检出。
    pub fn checkout_tracking_branch(
        &self,
        name: &str,
        remote_branch: &str,
    ) -> Result<(), GitError> {
        run_git(
            &self.root,
            &["checkout", "--track", "-b", name, remote_branch],
        )?;
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

    /// 切换到 `branch`（`git checkout <branch>`）。
    pub fn checkout_branch(&self, branch: &str) -> Result<(), GitError> {
        run_git(&self.root, &["checkout", branch])?;
        return Ok(());
    }

    /// 新建轻量标签（`git tag <name> <from>`）。
    pub fn create_tag(&self, name: &str, from: Option<&str>) -> Result<(), GitError> {
        return self.create_tag_with(name, from, "", false);
    }

    /// 新建标签；`annotated` 为真时建附注标签（带 `-a -m <message>`）。
    ///
    /// 附注标签没有说明时用标签名当说明：`git tag -a` 不给 `-m` 会开编辑器，
    /// 这里用空的 `GIT_EDITOR` 会让它失败，所以必须自带消息。
    pub fn create_tag_with(
        &self,
        name: &str,
        from: Option<&str>,
        message: &str,
        annotated: bool,
    ) -> Result<(), GitError> {
        let mut args: Vec<&str> = vec!["tag"];
        let message = message.trim();
        if annotated {
            args.push("-a");
            let text = if message.is_empty() { name } else { message };
            args.push("-m");
            args.push(text);
        }
        args.push(name);
        if let Some(from) = from {
            args.push(from);
        }
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

    /// 改写远程的 URL（`git remote set-url <name> <url>`）。
    pub fn set_remote_url(&self, name: &str, url: &str) -> Result<(), GitError> {
        run_git(&self.root, &["remote", "set-url", name, url])?;
        return Ok(());
    }

    /// 写入仓库级配置（`git config <key> <value>`）。
    pub fn config_set(&self, key: &str, value: &str) -> Result<(), GitError> {
        run_git(&self.root, &["config", key, value])?;
        return Ok(());
    }

    /// 删除仓库级配置，让全局值生效（`git config --unset <key>`）。
    ///
    /// 键本来就不存在时 `--unset` 会以非零退出，这里视为成功。
    pub fn config_unset(&self, key: &str) -> Result<(), GitError> {
        let result = run_git(&self.root, &["config", "--unset", key]);
        return match result {
            Ok(_) => Ok(()),
            Err(GitError::CommandFailed { code, .. }) if code == Some(5) => Ok(()),
            Err(error) => Err(error),
        };
    }

    /// 贮藏当前改动（`git stash push -u`）；`message` 为空时不带说明。
    pub fn stash_push(&self, message: &str) -> Result<(), GitError> {
        return self.stash_push_with(message, true);
    }

    /// 贮藏当前改动；`include_untracked` 决定是否带上未跟踪文件。
    pub fn stash_push_with(&self, message: &str, include_untracked: bool) -> Result<(), GitError> {
        let mut args: Vec<&str> = vec!["stash", "push"];
        if include_untracked {
            args.push("-u");
        }
        if !message.trim().is_empty() {
            args.push("-m");
            args.push(message.trim());
        }
        run_git(&self.root, &args)?;
        return Ok(());
    }

    /// 贮藏明确选中的路径。路径通过 literal pathspec 传入，避免通配符或冒号被解释。
    pub fn stash_push_selected(
        &self,
        message: &str,
        paths: &[&str],
        include_untracked: bool,
    ) -> Result<(), GitError> {
        if paths.is_empty() {
            return Err(GitError::CommandFailed {
                program: "git".to_string(),
                args: "stash push -- <paths>".to_string(),
                code: None,
                detail: "至少要选择一个文件".to_string(),
            });
        }
        let mut args = vec!["stash".to_string(), "push".to_string()];
        if include_untracked {
            args.push("-u".to_string());
        }
        if !message.trim().is_empty() {
            args.push("-m".to_string());
            args.push(message.trim().to_string());
        }
        args.push("--".to_string());
        args.extend(paths.iter().map(|path| format!(":(literal){path}")));
        let args = args.iter().map(String::as_str).collect::<Vec<_>>();
        run_git(&self.root, &args)?;
        return Ok(());
    }

    /// 读取某条贮藏的补丁，可附带其未跟踪文件。
    pub fn stash_show(&self, oid: &str, include_untracked: bool) -> Result<Vec<u8>, GitError> {
        let mut args = vec!["stash", "show", "--patch"];
        if include_untracked {
            args.push("--include-untracked");
        }
        args.push(oid);
        return run_git(&self.root, &args);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::testkit::*;

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
    fn selected_stash_keeps_other_paths_and_show_includes_selected_untracked_files() {
        let (_guard, path) = fixture(
            "selected-stash",
            &[("a.txt", "base a\n"), ("b.txt", "base b\n")],
            "base",
        );
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "changed a\n").expect("modify a");
        std::fs::write(repo.root().join("b.txt"), "changed b\n").expect("modify b");
        std::fs::write(repo.root().join("new.txt"), "untracked\n").expect("new file");

        repo.stash_push_selected("selected files", &["a.txt", "new.txt"], true)
            .expect("stash selected");

        let status = repo.status().expect("status");
        assert!(status.iter().any(|entry| entry.path == "b.txt"));
        assert!(!status.iter().any(|entry| entry.path == "a.txt"));
        assert!(!status.iter().any(|entry| entry.path == "new.txt"));
        let stash = repo.refs().expect("refs").stashes.remove(0);
        let patch = String::from_utf8(repo.stash_show(&stash.oid, true).expect("show stash"))
            .expect("utf8 patch");
        assert!(patch.contains("a.txt"));
        assert!(patch.contains("new.txt"));
        assert!(!patch.contains("b.txt"));
    }

    #[test]
    fn stash_can_be_pushed_popped_and_dropped() {
        let (_guard, path) = fixture("stash", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "changed\n").expect("write");

        repo.stash_push("wip").expect("stash");
        assert!(
            repo.status().expect("status").is_empty(),
            "贮藏后工作区应干净"
        );
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
        assert!(
            repo.refs()
                .expect("refs")
                .branches
                .iter()
                .any(|branch| branch.name == "topic/one" && branch.current)
        );
    }

    #[test]
    fn checkout_tracking_branch_sets_the_remote_upstream() {
        let (_guard, path, _remote_guard, _remote) = with_remote("checkout-tracking");
        let repo = Repo::open(&path).expect("open");
        repo.create_branch("topic/one", Some("main"))
            .expect("branch");
        repo.push_branch("origin", "topic/one").expect("push");
        repo.fetch("origin").expect("fetch");

        repo.checkout_tracking_branch("local/topic", "origin/topic/one")
            .expect("checkout tracking branch");
        assert_eq!(repo.abbrev_branch().expect("branch"), "local/topic");
        let upstream = run_git(
            repo.root(),
            &[
                "rev-parse",
                "--abbrev-ref",
                "--symbolic-full-name",
                "@{upstream}",
            ],
        )
        .expect("upstream");
        assert_eq!(
            String::from_utf8_lossy(&upstream).trim(),
            "origin/topic/one"
        );
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
        assert!(
            repo.refs()
                .expect("refs")
                .branches
                .iter()
                .any(|branch| branch.name == "feature")
        );

        // 切回去之后可以删。
        repo.checkout_branch("main").expect("checkout main");
        repo.delete_branch("feature").expect("delete");
        assert!(
            !repo
                .refs()
                .expect("refs")
                .branches
                .iter()
                .any(|branch| branch.name == "feature")
        );
    }

    #[test]
    fn stash_apply_keeps_the_entry_and_rename_moves_the_branch() {
        let (_guard, path) = fixture("stashapply", &[("a.txt", "one\n")], "first");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "two\n").expect("write");
        repo.stash_push("wip").expect("stash");

        let entries = repo.refs().expect("refs").stashes;
        assert_eq!(entries.len(), 1);
        repo.stash_apply(entries[0].reference.as_str())
            .expect("apply");
        assert_eq!(
            std::fs::read_to_string(repo.root().join("a.txt")).expect("read"),
            "two\n"
        );
        assert_eq!(
            repo.refs().expect("refs").stashes.len(),
            1,
            "apply 不删除贮藏"
        );

        repo.rename_branch("main", "renamed").expect("rename");
        assert_eq!(repo.abbrev_branch().expect("branch"), "renamed");
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
        assert_eq!(
            String::from_utf8_lossy(&url).trim(),
            "git@example.com:b.git"
        );
    }

    #[test]
    fn annotated_tags_carry_a_message() {
        let (_guard, path) = fixture("tag-annotated", &[("a.txt", "one\n")], "first");
        let repo = Repo::open(&path).expect("open");
        repo.create_tag_with("v1", None, "first release", true)
            .expect("annotated tag");
        repo.create_tag_with("v2", None, "", false)
            .expect("lightweight tag");
        let annotated = run_git(repo.root(), &["cat-file", "-t", "v1"]).expect("type");
        assert_eq!(String::from_utf8_lossy(&annotated).trim(), "tag");
        let message = run_git(repo.root(), &["tag", "-l", "-n1", "v1"]).expect("message");
        assert!(
            String::from_utf8_lossy(&message).contains("first release"),
            "附注标签应带说明：{message:?}"
        );
        // 轻量标签是指向提交的引用，不是 tag 对象。
        let lightweight = run_git(repo.root(), &["cat-file", "-t", "v2"]).expect("type");
        assert_eq!(String::from_utf8_lossy(&lightweight).trim(), "commit");
        // 没给说明的附注标签用标签名兜底，不会卡在编辑器上。
        repo.create_tag_with("v3", None, "  ", true)
            .expect("fallback");
        let fallback = run_git(repo.root(), &["tag", "-l", "-n1", "v3"]).expect("message");
        assert!(String::from_utf8_lossy(&fallback).contains("v3"));
    }

    #[test]
    fn stash_can_leave_untracked_files_out() {
        let (_guard, path) = fixture("stash-untracked", &[("a.txt", "one\n")], "first");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("tracked.txt"), "changed\n").expect("write");
        run_git(repo.root(), &["add", "tracked.txt"]).expect("add");
        run_git(repo.root(), &["commit", "-q", "-m", "add tracked"]).expect("commit");
        std::fs::write(repo.root().join("tracked.txt"), "changed again\n").expect("write");
        std::fs::write(repo.root().join("loose.txt"), "untracked\n").expect("write");

        repo.stash_push_with("without untracked", false)
            .expect("stash");
        assert!(
            repo.root().join("loose.txt").exists(),
            "未跟踪文件应留在工作区"
        );
        let list = repo.refs().expect("refs").stashes;
        assert_eq!(list.len(), 1);
        repo.stash_pop().expect("pop");
        assert!(
            std::fs::read_to_string(repo.root().join("tracked.txt"))
                .expect("read")
                .contains("changed again"),
            "弹回后已跟踪改动应回来"
        );
    }
}
