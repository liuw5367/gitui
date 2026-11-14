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

#[derive(Debug, Clone, Default)]
pub struct Status {
    pub branch: String,
    pub staged: usize,
    pub unstaged: usize,
    /// Raw `git status --porcelain=v2` entry lines (no branch headers).
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Commit {
    pub sha: String,
    pub short: String,
    pub author: String,
    pub date: String,
    pub summary: String,
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
            } else if line.starts_with("u ") {
                status.unstaged += 1;
                status.lines.push(line.to_string());
            } else if line.starts_with("? ") {
                status.unstaged += 1;
                status.lines.push(line.to_string());
            }
        }

        Ok(status)
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

    /// Commit the staged changes with `git commit -F -` (hooks run).
    /// On failure the error carries the hook stdout/stderr.
    pub fn commit(&self, message: &str) -> Result<String, GitError> {
        let mut command = Command::new("git");
        command
            .arg("--no-pager")
            .arg("-C")
            .arg(&self.root)
            .args(["commit", "-F", "-"])
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
                "git commit failed (hooks may have failed)
{}
{}",
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
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempRepo {
        path: PathBuf,
    }

    impl TempRepo {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "gitui-git-test-{}-{unique}",
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
