//! Read-only Git access for the M1 slice.
//!
//! The crate runs the system `git` CLI with an argument array (never a shell)
//! and parses machine-readable output. It has no write operations.

use std::error::Error;
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct GitError(pub String);

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for GitError {}

#[derive(Debug, Clone)]
pub struct Repo {
    root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusEntry {
    pub path: String,
    pub x: char,
    pub y: char,
}

impl StatusEntry {
    pub fn is_staged(&self) -> bool {
        self.x != '.' && self.x != ' ' && self.x != '?'
    }

    pub fn is_unstaged(&self) -> bool {
        self.y != '.' && self.y != ' '
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Merge,
    Rebase,
    CherryPick,
    Revert,
}

impl Operation {
    pub fn label(self) -> &'static str {
        match self {
            Self::Merge => "MERGING",
            Self::Rebase => "REBASING",
            Self::CherryPick => "CHERRY-PICKING",
            Self::Revert => "REVERTING",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stash {
    pub selector: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    pub name: String,
    pub current: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConflictStages {
    pub base: String,
    pub ours: String,
    pub theirs: String,
}

#[derive(Debug, Clone, Default)]
pub struct Status {
    pub branch: String,
    pub ahead: usize,
    pub behind: usize,
    pub staged: usize,
    pub unstaged: usize,
    /// Raw `git status --porcelain=v2` entry lines (no branch headers).
    pub lines: Vec<String>,
    pub entries: Vec<StatusEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Commit {
    pub sha: String,
    pub short: String,
    pub author: String,
    pub date: String,
    pub summary: String,
}

fn status_path(line: &str) -> Option<String> {
    let fields = if line.starts_with("1 ") {
        9
    } else if line.starts_with("2 ") {
        10
    } else if line.starts_with("u ") {
        11
    } else {
        return None;
    };
    line.splitn(fields, ' ')
        .nth(fields - 1)
        .map(|path| path.split('\t').next().unwrap_or(path).to_string())
}

impl Repo {
    /// Open an existing worktree and resolve its top-level directory.
    pub fn open(path: &str) -> Result<Self, GitError> {
        let output = Command::new("git")
            .arg("--no-pager")
            .arg("-C")
            .arg(path)
            .args(["rev-parse", "--show-toplevel"])
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_PAGER", "cat")
            .output()
            .map_err(|error| GitError(format!("could not run git: {error}")))?;

        if !output.status.success() {
            return Err(GitError(format!(
                "not a Git repository: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }

        let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if root.is_empty() {
            return Err(GitError("git returned an empty repository root".to_string()));
        }
        Ok(Self {
            root: PathBuf::from(root),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Working tree status, branch and raw porcelain v2 entries.
    pub fn status(&self) -> Result<Status, GitError> {
        let output = self.git(&["status", "--porcelain=v2", "--branch"])?;
        let mut status = Status {
            branch: "HEAD".to_string(),
            ..Status::default()
        };

        for line in output.lines() {
            if let Some(branch) = line.strip_prefix("# branch.head ") {
                status.branch = branch.trim().to_string();
            } else if let Some(ab) = line.strip_prefix("# branch.ab ") {
                for part in ab.split_whitespace() {
                    if let Some(value) = part.strip_prefix('+') {
                        status.ahead = value.parse().unwrap_or(0);
                    } else if let Some(value) = part.strip_prefix('-') {
                        status.behind = value.parse().unwrap_or(0);
                    }
                }
            } else if line.starts_with("# ") {
                continue;
            } else if line.starts_with("1 ") || line.starts_with("2 ") {
                let bytes = line.as_bytes();
                let x = bytes.get(2).copied().unwrap_or(b'.');
                let y = bytes.get(3).copied().unwrap_or(b'.');
                if x != b'.' && x != b' ' {
                    status.staged += 1;
                }
                if y != b'.' && y != b' ' {
                    status.unstaged += 1;
                }
                status.lines.push(line.to_string());
                if let Some(path) = status_path(line) {
                    status.entries.push(StatusEntry {
                        path,
                        x: x as char,
                        y: y as char,
                    });
                }
            } else if line.starts_with("u ") {
                status.unstaged += 1;
                status.lines.push(line.to_string());
                if let Some(path) = status_path(line) {
                    status.entries.push(StatusEntry { path, x: 'U', y: 'U' });
                }
            } else if line.starts_with("? ") {
                status.unstaged += 1;
                status.lines.push(line.to_string());
                if let Some(path) = line.strip_prefix("? ").map(str::to_string) {
                    status.entries.push(StatusEntry { path, x: '?', y: '?' });
                }
            }
        }

        Ok(status)
    }

    /// Stash entries, newest first.
    pub fn stashes(&self) -> Result<Vec<Stash>, GitError> {
        let output = self.git(&["stash", "list", "--format=%gd%x1f%s"])?;
        Ok(output
            .lines()
            .filter_map(|line| {
                let (selector, message) = line.split_once('\u{1f}')?;
                Some(Stash {
                    selector: selector.to_string(),
                    message: message.to_string(),
                })
            })
            .collect())
    }

    /// Stash the working tree and index.
    pub fn stash_push(&self, message: &str) -> Result<String, GitError> {
        if message.is_empty() {
            self.git(&["stash", "push"])
        } else {
            self.git(&["stash", "push", "-m", message])
        }
    }

    /// Apply a stash without removing it.
    pub fn stash_apply(&self, selector: &str) -> Result<String, GitError> {
        self.git(&["stash", "apply", selector])
    }

    /// Apply a stash and remove it.
    pub fn stash_pop(&self, selector: &str) -> Result<String, GitError> {
        self.git(&["stash", "pop", selector])
    }

    /// Drop a stash.
    pub fn stash_drop(&self, selector: &str) -> Result<String, GitError> {
        self.git(&["stash", "drop", selector])
    }

    /// Local tags.
    pub fn tags(&self) -> Result<Vec<Tag>, GitError> {
        let output = self.git(&["tag", "--list"])?;
        Ok(output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|name| Tag { name: name.to_string() })
            .collect())
    }

    /// Create a lightweight tag at HEAD.
    pub fn create_tag(&self, name: &str) -> Result<String, GitError> {
        self.git(&["tag", name])
    }

    /// Delete a local tag.
    pub fn delete_tag(&self, name: &str) -> Result<String, GitError> {
        self.git(&["tag", "-d", name])
    }

    /// Local branches.
    pub fn branches(&self) -> Result<Vec<Branch>, GitError> {
        let output = self.git(&[
            "for-each-ref",
            "--format=%(refname:short)%09%(HEAD)",
            "refs/heads",
        ])?;
        Ok(output
            .lines()
            .filter_map(|line| {
                let (name, head) = line.split_once('\t')?;
                let name = name.trim();
                if name.is_empty() {
                    return None;
                }
                Some(Branch {
                    name: name.to_string(),
                    current: head.trim() == "*",
                })
            })
            .collect())
    }

    /// Rename a local branch.
    pub fn rename_branch(&self, old: &str, new: &str) -> Result<String, GitError> {
        self.git(&["branch", "-m", old, new])
    }

    /// Delete a local branch. Git refuses the currently checked-out branch.
    pub fn delete_branch(&self, name: &str) -> Result<String, GitError> {
        self.git(&["branch", "-d", name])
    }

    /// Create a branch at HEAD and switch to it.
    pub fn create_branch(&self, name: &str) -> Result<String, GitError> {
        self.git(&["switch", "-c", name])
    }

    /// Switch to an existing local branch.
    pub fn checkout(&self, branch: &str) -> Result<String, GitError> {
        self.git(&["switch", branch])
    }

    /// Paths with unmerged entries.
    pub fn conflicts(&self) -> Result<Vec<String>, GitError> {
        let output = self.git(&["diff", "--name-only", "--diff-filter=U", "--"])?;
        Ok(output
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect())
    }

    /// Base / ours / theirs contents for a conflicted path.
    pub fn conflict_stages(&self, path: &str) -> Result<ConflictStages, GitError> {
        let read = |stage: u8| -> String {
            self.git(&["show", &format!(":{stage}:{path}")])
                .unwrap_or_default()
        };
        Ok(ConflictStages {
            base: read(1),
            ours: read(2),
            theirs: read(3),
        })
    }

    /// Write a resolved result and stage it with `git add`.
    pub fn resolve(&self, path: &str, result: &str) -> Result<(), GitError> {
        let target = self.root.join(path);
        if let Some(parent) = target.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&target, result)
            .map_err(|error| GitError(format!("could not write {path}: {error}")))?;
        self.git(&["add", "--", path])?;
        Ok(())
    }

    /// The operation in progress, if any, from the Git state files.
    pub fn operation(&self) -> Option<Operation> {
        let git = self.root.join(".git");
        if git.join("MERGE_HEAD").exists() {
            Some(Operation::Merge)
        } else if git.join("rebase-merge").is_dir() || git.join("rebase-apply").is_dir() {
            Some(Operation::Rebase)
        } else if git.join("CHERRY_PICK_HEAD").exists() {
            Some(Operation::CherryPick)
        } else if git.join("REVERT_HEAD").exists() {
            Some(Operation::Revert)
        } else {
            None
        }
    }

    /// Newest commits, up to `limit`.
    pub fn log(&self, limit: usize) -> Result<Vec<Commit>, GitError> {
        let limit = limit.to_string();
        let output = self.git(&[
            "log",
            "-n",
            &limit,
            "--format=%H%x1f%h%x1f%an%x1f%ar%x1f%s",
        ])?;

        Ok(output
            .lines()
            .filter_map(|line| {
                let fields: Vec<&str> = line.split('\u{1f}').collect();
                if fields.len() < 5 {
                    return None;
                }
                Some(Commit {
                    sha: fields[0].to_string(),
                    short: fields[1].to_string(),
                    author: fields[2].to_string(),
                    date: fields[3].to_string(),
                    summary: fields[4].to_string(),
                })
            })
            .collect())
    }

    /// The patch a commit introduced, without a commit header.
    pub fn diff(&self, sha: &str) -> Result<String, GitError> {
        if sha.is_empty() {
            return Err(GitError("no commit selected".to_string()));
        }
        self.git(&[
            "show",
            "--format=",
            "--patch",
            "--no-color",
            "--no-ext-diff",
            sha,
        ])
    }

    /// Stage every working-tree change. M2 first write operation.
    pub fn stage_all(&self) -> Result<(), GitError> {
        self.git(&["add", "-A"])?;
        Ok(())
    }

    /// Unstage everything, back to HEAD.
    pub fn unstage_all(&self) -> Result<(), GitError> {
        self.git(&["reset", "-q", "HEAD", "--", "."])?;
        Ok(())
    }

    /// Stage one path.
    pub fn stage_path(&self, path: &str) -> Result<(), GitError> {
        self.git(&["add", "--", path])?;
        Ok(())
    }

    /// Unstage one path.
    pub fn unstage_path(&self, path: &str) -> Result<(), GitError> {
        self.git(&["restore", "--staged", "--", path])?;
        Ok(())
    }

    /// Commit the staged changes with `git commit -F -` (hooks run).
    /// On failure the error carries the hook stdout/stderr.
    pub fn commit(&self, message: &str) -> Result<String, GitError> {
        self.commit_with(message, false, false)
    }

    /// Commit with `--no-verify`; hooks are skipped. Used only after an
    /// explicit user confirmation in the app.
    pub fn commit_no_verify(&self, message: &str) -> Result<String, GitError> {
        self.commit_with(message, true, false)
    }

    /// Fetch and prune `origin`; no working-tree changes.
    pub fn fetch(&self) -> Result<String, GitError> {
        self.git(&["fetch", "--prune", "origin"])
    }

    /// Fast-forward pull from the configured upstream.
    pub fn pull_ff_only(&self) -> Result<String, GitError> {
        self.git(&["pull", "--ff-only"])
    }

    /// Push the current HEAD to `origin`.
    pub fn push(&self) -> Result<String, GitError> {
        self.git(&["push", "origin", "HEAD"])
    }

    /// Continue a resolved merge.
    pub fn merge_continue(&self) -> Result<String, GitError> {
        self.git(&["merge", "--continue"])
    }

    /// Abort the current merge.
    pub fn merge_abort(&self) -> Result<String, GitError> {
        self.git(&["merge", "--abort"])
    }

    /// Amend the last commit with the staged content and `message`.
    pub fn amend(&self, message: &str) -> Result<String, GitError> {
        self.commit_with(message, false, true)
    }

    fn commit_with(
        &self,
        message: &str,
        no_verify: bool,
        amend: bool,
    ) -> Result<String, GitError> {
        let mut command = Command::new("git");
        command
            .arg("--no-pager")
            .arg("-C")
            .arg(&self.root);
        let mut args = vec!["commit"];
        if amend {
            args.push("--amend");
        }
        if no_verify {
            args.push("--no-verify");
        }
        args.extend(["-F", "-"]);
        command
            .args(&args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_PAGER", "cat")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command
            .spawn()
            .map_err(|error| GitError(format!("could not run git: {error}")))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(message.as_bytes())
                .map_err(|error| GitError(format!("could not write commit message: {error}")))?;
        }
        let output = child
            .wait_with_output()
            .map_err(|error| GitError(format!("could not wait for git: {error}")))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if !output.status.success() {
            return Err(GitError(format!(
                "git commit failed (hooks may have failed)\n{}\n{}",
                stdout.trim(),
                stderr.trim()
            )));
        }
        Ok(stdout)
    }

    fn git(&self, args: &[&str]) -> Result<String, GitError> {
        let mut command = Command::new("git");
        command
            .arg("--no-pager")
            .arg("-C")
            .arg(&self.root)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_PAGER", "cat")
            .env("GIT_EDITOR", "true")
            .env("GIT_MERGE_AUTOEDIT", "no")
            .env("GIT_OPTIONAL_LOCKS", "0");

        let output = command
            .output()
            .map_err(|error| GitError(format!("could not run git: {error}")))?;

        if !output.status.success() {
            return Err(GitError(format!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempRepo {
        path: PathBuf,
    }

    impl TempRepo {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "gitui-git-test-{}-{unique}-{sequence}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            run(&path, &["init", "-q", "-b", "main"]);
            run(&path, &["config", "user.name", "Test"]);
            run(&path, &["config", "user.email", "test@example.com"]);
            fs::write(path.join("a.txt"), "one\ntwo\n").unwrap();
            run(&path, &["add", "a.txt"]);
            run(&path, &["commit", "-qm", "base"]);
            Self { path }
        }

        fn path(&self) -> &str {
            self.path.to_str().unwrap()
        }
    }

    impl Drop for TempRepo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn subprocess_git(args: &[&str]) {
        let status = Command::new("git").args(args).status().unwrap();
        assert!(status.success(), "git {args:?} failed");
    }

    fn run(path: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("--no-pager")
            .arg("-C")
            .arg(path)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?} failed");
    }

    #[test]
    fn opens_and_reads_clean_status() {
        let fixture = TempRepo::new();
        let repo = Repo::open(fixture.path()).unwrap();
        assert_eq!(
            repo.root().canonicalize().unwrap(),
            fixture.path.canonicalize().unwrap()
        );

        let status = repo.status().unwrap();
        assert_eq!(status.branch, "main");
        assert_eq!(status.staged, 0);
        assert_eq!(status.unstaged, 0);
        assert!(status.lines.is_empty());
    }

    #[test]
    fn reports_staged_and_unstaged_paths() {
        let fixture = TempRepo::new();
        fs::write(fixture.path.join("a.txt"), "one\nTWO\n").unwrap();
        fs::write(fixture.path.join("b.txt"), "new\n").unwrap();
        run(&fixture.path, &["add", "b.txt"]);

        let status = Repo::open(fixture.path()).unwrap().status().unwrap();
        assert_eq!(status.staged, 1);
        assert_eq!(status.unstaged, 1);
        assert_eq!(status.lines.len(), 2);
    }

    #[test]
    fn reads_history_and_a_commit_patch() {
        let fixture = TempRepo::new();
        fs::write(fixture.path.join("a.txt"), "one\nTWO\n").unwrap();
        run(&fixture.path, &["commit", "-qam", "second"]);

        let repo = Repo::open(fixture.path()).unwrap();
        let commits = repo.log(10).unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].summary, "second");
        assert_eq!(commits[1].summary, "base");

        let patch = repo.diff(&commits[0].sha).unwrap();
        assert!(patch.contains("-two"));
        assert!(patch.contains("+TWO"));
    }

    #[test]
    fn stages_and_unstages_all() {
        let fixture = TempRepo::new();
        fs::write(fixture.path.join("a.txt"), "one\nTWO\n").unwrap();
        fs::write(fixture.path.join("b.txt"), "new\n").unwrap();

        let repo = Repo::open(fixture.path()).unwrap();
        assert_eq!(repo.status().unwrap().unstaged, 2);
        repo.stage_all().unwrap();
        let staged = repo.status().unwrap();
        assert_eq!(staged.staged, 2);
        assert_eq!(staged.unstaged, 0);
        repo.unstage_all().unwrap();
        let unstaged = repo.status().unwrap();
        assert_eq!(unstaged.staged, 0);
        assert_eq!(unstaged.unstaged, 2);
    }

    #[test]
    fn commits_staged_changes_and_reports_hook_failure() {
        let fixture = TempRepo::new();
        fs::write(fixture.path.join("a.txt"), "one\nTWO\n").unwrap();
        let repo = Repo::open(fixture.path()).unwrap();
        repo.stage_all().unwrap();
        repo.commit("update file").unwrap();
        assert_eq!(repo.log(1).unwrap()[0].summary, "update file");

        fs::write(fixture.path.join("a.txt"), "one\nTHREE\n").unwrap();
        repo.stage_all().unwrap();
        let hook = fixture.path.join(".git/hooks/pre-commit");
        fs::write(&hook, "#!/bin/sh\necho hook failed >&2\nexit 1\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&hook).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&hook, permissions).unwrap();
        }
        let error = repo.commit("should fail").unwrap_err();
        assert!(error.to_string().contains("hook failed"));
        assert_eq!(repo.log(1).unwrap()[0].summary, "update file");

        repo.commit_no_verify("bypass hooks").unwrap();
        assert_eq!(repo.log(1).unwrap()[0].summary, "bypass hooks");

        let _ = fs::remove_file(&hook);
        fs::write(fixture.path.join("a.txt"), "one\nFOUR\n").unwrap();
        repo.stage_all().unwrap();
        let before = repo.log(10).unwrap().len();
        repo.amend("amended").unwrap();
        let after = repo.log(10).unwrap();
        assert_eq!(after.len(), before);
        assert_eq!(after[0].summary, "amended");
    }

    #[test]
    fn stages_and_unstages_one_path() {
        let fixture = TempRepo::new();
        fs::write(fixture.path.join("a.txt"), "one\nTWO\n").unwrap();
        fs::write(fixture.path.join("b.txt"), "new\n").unwrap();
        let repo = Repo::open(fixture.path()).unwrap();
        let status = repo.status().unwrap();
        assert_eq!(status.entries.len(), 2);
        let b = status
            .entries
            .iter()
            .find(|entry| entry.path == "b.txt")
            .unwrap();
        assert!(b.is_unstaged());
        repo.stage_path("b.txt").unwrap();
        let after = repo.status().unwrap();
        let b = after.entries.iter().find(|entry| entry.path == "b.txt").unwrap();
        assert!(b.is_staged());
        repo.unstage_path("b.txt").unwrap();
        let back = repo.status().unwrap();
        let b = back.entries.iter().find(|entry| entry.path == "b.txt").unwrap();
        assert!(b.is_unstaged());
    }

    #[test]
    fn fetches_and_pushes_origin() {
        let fixture = TempRepo::new();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let remote = std::env::temp_dir().join(format!(
            "gitui-git-remote-{}-{unique}.git",
            std::process::id()
        ));
        subprocess_git(&["init", "--bare", "-q", remote.to_str().unwrap()]);
        run(&fixture.path, &["remote", "add", "origin", remote.to_str().unwrap()]);
        let repo = Repo::open(fixture.path()).unwrap();
        repo.push().unwrap();
        let has_main = Command::new("git")
            .arg("--git-dir")
            .arg(&remote)
            .args(["rev-parse", "--verify", "main"])
            .output()
            .unwrap();
        assert!(has_main.status.success());
        repo.fetch().unwrap();
        let _ = fs::remove_dir_all(remote);
    }

    #[test]
    fn pulls_fast_forward_from_origin() {
        let fixture = TempRepo::new();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let remote = std::env::temp_dir().join(format!(
            "gitui-git-pull-{}-{unique}.git",
            std::process::id()
        ));
        let clone = std::env::temp_dir().join(format!(
            "gitui-git-pull-clone-{}-{unique}",
            std::process::id()
        ));
        subprocess_git(&["init", "--bare", "-q", remote.to_str().unwrap()]);
        run(&fixture.path, &["remote", "add", "origin", remote.to_str().unwrap()]);
        run(&fixture.path, &["push", "-u", "origin", "main"]);
        subprocess_git(&["clone", "-q", remote.to_str().unwrap(), clone.to_str().unwrap()]);
        run(&clone, &["config", "user.name", "S5"]);
        run(&clone, &["config", "user.email", "s5@example.com"]);
        fs::write(clone.join("remote.txt"), "remote\n").unwrap();
        run(&clone, &["add", "remote.txt"]);
        run(&clone, &["commit", "-qm", "remote commit"]);
        run(&clone, &["push", "-q", "origin", "main"]);

        let repo = Repo::open(fixture.path()).unwrap();
        repo.pull_ff_only().unwrap();
        assert_eq!(repo.log(1).unwrap()[0].summary, "remote commit");
        let _ = fs::remove_dir_all(remote);
        let _ = fs::remove_dir_all(clone);
    }

    #[test]
    fn detects_a_merge_in_progress() {
        use std::process::Command;
        let fixture = TempRepo::new();
        run(&fixture.path, &["checkout", "-qb", "feature"]);
        fs::write(fixture.path.join("a.txt"), "one\nFEATURE\n").unwrap();
        run(&fixture.path, &["commit", "-qam", "feature"]);
        run(&fixture.path, &["checkout", "-q", "main"]);
        fs::write(fixture.path.join("a.txt"), "one\nMAIN\n").unwrap();
        run(&fixture.path, &["commit", "-qam", "main"]);
        let status = Command::new("git")
            .arg("-C")
            .arg(&fixture.path)
            .args(["merge", "feature"])
            .status()
            .unwrap();
        assert!(!status.success());
        let repo = Repo::open(fixture.path()).unwrap();
        assert_eq!(repo.operation(), Some(Operation::Merge));

        let conflicts = repo.conflicts().unwrap();
        assert_eq!(conflicts, vec!["a.txt"]);
        let stages = repo.conflict_stages("a.txt").unwrap();
        assert!(stages.base.contains("two"));
        assert!(stages.ours.contains("MAIN"));
        assert!(stages.theirs.contains("FEATURE"));

        repo.resolve("a.txt", "one\nMERGED\n").unwrap();
        assert!(repo.conflicts().unwrap().is_empty());
        repo.merge_continue().unwrap();
        assert_eq!(repo.operation(), None);
    }

    #[test]
    fn aborts_a_merge() {
        let fixture = TempRepo::new();
        run(&fixture.path, &["checkout", "-qb", "feature"]);
        fs::write(fixture.path.join("a.txt"), "one\nFEATURE\n").unwrap();
        run(&fixture.path, &["commit", "-qam", "feature"]);
        run(&fixture.path, &["checkout", "-q", "main"]);
        fs::write(fixture.path.join("a.txt"), "one\nMAIN\n").unwrap();
        run(&fixture.path, &["commit", "-qam", "main"]);
        let status = Command::new("git")
            .arg("-C")
            .arg(&fixture.path)
            .args(["merge", "feature"])
            .status()
            .unwrap();
        assert!(!status.success());
        let repo = Repo::open(fixture.path()).unwrap();
        assert_eq!(repo.operation(), Some(Operation::Merge));
        repo.merge_abort().unwrap();
        assert_eq!(repo.operation(), None);
        assert!(repo.conflicts().unwrap().is_empty());
    }

    #[test]
    fn lists_and_checks_out_branches() {
        let fixture = TempRepo::new();
        run(&fixture.path, &["branch", "feature"]);
        let repo = Repo::open(fixture.path()).unwrap();
        let branches = repo.branches().unwrap();
        assert!(branches.iter().any(|branch| branch.name == "main" && branch.current));
        assert!(branches.iter().any(|branch| branch.name == "feature" && !branch.current));
        repo.checkout("feature").unwrap();
        assert!(repo
            .branches()
            .unwrap()
            .iter()
            .any(|branch| branch.name == "feature" && branch.current));
    }

    #[test]
    fn creates_and_switches_to_a_branch() {
        let fixture = TempRepo::new();
        let repo = Repo::open(fixture.path()).unwrap();
        repo.create_branch("created").unwrap();
        assert!(repo
            .branches()
            .unwrap()
            .iter()
            .any(|branch| branch.name == "created" && branch.current));
    }

    #[test]
    fn deletes_a_merged_branch_and_refuses_current() {
        let fixture = TempRepo::new();
        run(&fixture.path, &["branch", "feature"]);
        let repo = Repo::open(fixture.path()).unwrap();
        repo.delete_branch("feature").unwrap();
        assert!(!repo.branches().unwrap().iter().any(|branch| branch.name == "feature"));

        let error = repo.delete_branch("main").unwrap_err();
        assert!(!error.to_string().is_empty());
        assert!(repo.branches().unwrap().iter().any(|branch| branch.name == "main"));
    }

    #[test]
    fn lists_creates_and_deletes_tags() {
        let fixture = TempRepo::new();
        let repo = Repo::open(fixture.path()).unwrap();
        assert!(repo.tags().unwrap().is_empty());
        repo.create_tag("v1.0.0").unwrap();
        assert_eq!(repo.tags().unwrap(), vec![Tag { name: "v1.0.0".to_string() }]);
        repo.delete_tag("v1.0.0").unwrap();
        assert!(repo.tags().unwrap().is_empty());
    }

    #[test]
    fn stashes_applies_and_drops() {
        let fixture = TempRepo::new();
        fs::write(fixture.path.join("a.txt"), "one\nSTASHED\n").unwrap();
        let repo = Repo::open(fixture.path()).unwrap();
        repo.stash_push("test stash").unwrap();
        let stashes = repo.stashes().unwrap();
        assert_eq!(stashes.len(), 1);
        assert!(stashes[0].message.contains("test stash"));
        assert_eq!(repo.status().unwrap().unstaged, 0);

        repo.stash_apply(&stashes[0].selector).unwrap();
        assert_eq!(repo.status().unwrap().unstaged, 1);
        repo.stash_drop(&stashes[0].selector).unwrap();
        assert!(repo.stashes().unwrap().is_empty());
    }

    #[test]
    fn renames_a_local_branch() {
        let fixture = TempRepo::new();
        run(&fixture.path, &["branch", "feature"]);
        let repo = Repo::open(fixture.path()).unwrap();
        repo.rename_branch("feature", "renamed").unwrap();
        let branches = repo.branches().unwrap();
        assert!(!branches.iter().any(|branch| branch.name == "feature"));
        assert!(branches.iter().any(|branch| branch.name == "renamed"));
    }

    #[test]
    fn rejects_a_non_repository() {
        let path = std::env::temp_dir().join(format!(
            "gitui-git-not-repo-{}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        let error = Repo::open(path.to_str().unwrap()).unwrap_err();
        assert!(error.to_string().contains("not a Git repository"));
        let _ = fs::remove_dir_all(path);
    }
}
