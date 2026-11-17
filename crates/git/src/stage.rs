//! 本模块负责工作区与暂存区的读写：状态、暂存、取消暂存、丢弃改动。

use domain::{GitError, StatusEntry};

use crate::exec::run_git;
use crate::parse::parse_status;
use crate::query::Repo;

impl Repo {
    /// 工作区与 index 的变更文件。
    pub fn status(&self) -> Result<Vec<StatusEntry>, GitError> {
        let output = run_git(&self.root, &["status", "--porcelain=v1", "-z"])?;
        return parse_status(&output);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::testkit::*;

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
        let a = entries
            .iter()
            .find(|entry| entry.path == "a.txt")
            .expect("a");
        assert!(!a.is_staged());
        assert!(a.is_unstaged());

        repo.stage(&["a.txt".to_string()]).expect("stage");
        assert!(
            repo.status()
                .expect("status")
                .iter()
                .find(|entry| entry.path == "a.txt")
                .expect("a")
                .is_staged()
        );
    }

    #[test]
    fn discard_restores_the_worktree_copy() {
        let (_guard, path) = fixture("discard", &[("a.txt", "one\n")], "base");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "changed\n").expect("write");
        assert!(
            repo.status()
                .expect("status")
                .iter()
                .any(|entry| entry.is_unstaged())
        );

        repo.discard(&["a.txt".to_string()]).expect("discard");
        assert_eq!(
            std::fs::read_to_string(repo.root().join("a.txt")).expect("read"),
            "one\n"
        );
        assert!(repo.status().expect("status").is_empty());
    }

    #[test]
    fn staging_a_single_hunk_leaves_the_other_hunks_unstaged() {
        // 造一个文件，两处改动间隔足够远，形成两个 hunk。
        let lines: String = (1..=30).map(|index| format!("line{index}\n")).collect();
        let (_guard, path) = fixture("hunk-stage", &[("f.txt", lines.as_str())], "base");
        let repo = Repo::open(&path).expect("open");
        let mut changed: Vec<String> = (1..=30).map(|index| format!("line{index}")).collect();
        changed[1] = "line2-changed".to_string();
        changed[27] = "line28-changed".to_string();
        std::fs::write(
            repo.root().join("f.txt"),
            format!("{}\n", changed.join("\n")),
        )
        .expect("write");
        let patch = run_git(repo.root(), &["diff", "--", "f.txt"]).expect("diff");
        let patch = String::from_utf8_lossy(&patch).to_string();

        let sections = domain::stage::Sections::parse(patch.as_str());
        assert_eq!(sections.files.len(), 1);
        assert_eq!(sections.files[0].hunks.len(), 2, "应形成两个 hunk");

        // 只应用第二个 hunk。
        let only_second = sections.hunk_patch(0, 1).expect("第二个 hunk");
        repo.apply_cached(only_second.as_str(), false)
            .expect("apply");

        let staged = run_git(repo.root(), &["diff", "--cached"]).expect("cached");
        let staged = String::from_utf8_lossy(&staged).to_string();
        assert!(staged.contains("line28-changed"), "第二个 hunk 应在暂存区");
        assert!(!staged.contains("line2-changed"), "第一个 hunk 不应被暂存");

        let unstaged = run_git(repo.root(), &["diff"]).expect("diff");
        let unstaged = String::from_utf8_lossy(&unstaged).to_string();
        assert!(unstaged.contains("line2-changed"), "第一个 hunk 仍在工作区");
    }

    #[test]
    fn staging_a_single_line_adds_only_that_line() {
        let lines: String = (1..=10).map(|index| format!("line{index}\n")).collect();
        let (_guard, path) = fixture("line-stage", &[("f.txt", lines.as_str())], "base");
        let repo = Repo::open(&path).expect("open");
        let mut changed: Vec<String> = (1..=10).map(|index| format!("line{index}")).collect();
        // 在 5 和 6 之间插入一行。
        changed.insert(5, "inserted".to_string());
        std::fs::write(
            repo.root().join("f.txt"),
            format!("{}\n", changed.join("\n")),
        )
        .expect("write");
        let patch = run_git(repo.root(), &["diff", "--", "f.txt"]).expect("diff");
        let patch = String::from_utf8_lossy(&patch).to_string();
        let sections = domain::stage::Sections::parse(patch.as_str());
        let hunk = &sections.files[0].hunks[0];
        let index = hunk
            .lines
            .iter()
            .position(|line| line == "+inserted")
            .expect("新增行");
        let single = sections.line_patch(0, 0, index).expect("单行补丁");
        repo.apply_cached(single.as_str(), false).expect("apply");

        let staged = run_git(repo.root(), &["diff", "--cached"]).expect("cached");
        let staged = String::from_utf8_lossy(&staged).to_string();
        assert!(staged.contains("+inserted"), "只有这一行被暂存");
        assert_eq!(
            run_git(repo.root(), &["diff", "--cached", "--numstat"])
                .map(|out| String::from_utf8_lossy(&out).trim().to_string())
                .expect("numstat"),
            "1\t0\tf.txt",
            "暂存区应只有一个新增"
        );
    }

    #[test]
    fn reverse_apply_removes_a_staged_hunk() {
        let lines: String = (1..=10).map(|index| format!("line{index}\n")).collect();
        let (_guard, path) = fixture("unstage-hunk", &[("f.txt", lines.as_str())], "base");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(
            repo.root().join("f.txt"),
            format!("{}\n", {
                let mut all: Vec<String> = (1..=10).map(|index| format!("line{index}")).collect();
                all[4] = "line5-changed".to_string();
                all.join("\n")
            }),
        )
        .expect("write");
        repo.stage_all().expect("stage");
        let staged = run_git(repo.root(), &["diff", "--cached", "--", "f.txt"]).expect("diff");
        let staged = String::from_utf8_lossy(&staged).to_string();
        let sections = domain::stage::Sections::parse(staged.as_str());
        let only = sections.hunk_patch(0, 0).expect("hunk");
        repo.apply_cached(only.as_str(), true).expect("reverse");
        let after = run_git(repo.root(), &["diff", "--cached", "--", "f.txt"]).expect("diff");
        assert!(
            String::from_utf8_lossy(&after).trim().is_empty(),
            "应已取消暂存"
        );
    }
}
