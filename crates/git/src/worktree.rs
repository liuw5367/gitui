//! linked worktree 的创建、移除与清理。

use domain::GitError;

use crate::exec::run_git;
use crate::query::Repo;

impl Repo {
    /// 在 `path` 创建 worktree；`start_point` 有值时从该引用新建分支。
    pub fn add_worktree(
        &self,
        path: &str,
        branch: &str,
        start_point: Option<&str>,
    ) -> Result<(), GitError> {
        let mut args = vec!["worktree", "add"];
        if let Some(start_point) = start_point {
            args.extend(["-b", branch, "--", path, start_point]);
        } else {
            args.extend(["--", path, branch]);
        }
        run_git(&self.root, &args)?;
        return Ok(());
    }

    /// 安全移除已登记的干净 linked worktree，不使用 `--force`。
    pub fn remove_worktree(&self, path: &str) -> Result<(), GitError> {
        let worktree = self
            .refs()?
            .worktrees
            .into_iter()
            .find(|worktree| same_worktree_path(worktree.path.as_str(), path))
            .ok_or_else(|| worktree_error("The worktree registration changed."))?;
        if !worktree.exists || worktree.prunable {
            return Err(worktree_error("The worktree path no longer exists."));
        }
        if worktree.locked {
            return Err(worktree_error("The worktree is locked."));
        }
        if worktree.dirty {
            return Err(worktree_error("The worktree has uncommitted changes."));
        }
        run_git(
            &self.root,
            &["worktree", "remove", "--", worktree.path.as_str()],
        )?;
        return Ok(());
    }

    /// 清理 Git 已标记为可 prune 的失效 worktree 登记。
    pub fn prune_worktrees(&self) -> Result<(), GitError> {
        run_git(&self.root, &["worktree", "prune", "--expire", "now"])?;
        return Ok(());
    }
}

fn worktree_error(detail: &str) -> GitError {
    return GitError::CommandFailed {
        program: "git".to_string(),
        args: "worktree".to_string(),
        code: None,
        detail: detail.to_string(),
    };
}

fn same_worktree_path(left: &str, right: &str) -> bool {
    return normalize_worktree_path(left) == normalize_worktree_path(right);
}

fn normalize_worktree_path(path: &str) -> std::path::PathBuf {
    let mut ancestor = std::path::Path::new(path);
    let mut missing = Vec::new();
    while !ancestor.exists() {
        let Some(name) = ancestor.file_name() else {
            break;
        };
        missing.push(name.to_os_string());
        let Some(parent) = ancestor.parent() else {
            break;
        };
        ancestor = parent;
    }
    let mut normalized = ancestor
        .canonicalize()
        .unwrap_or_else(|_| ancestor.to_path_buf());
    for part in missing.iter().rev() {
        normalized.push(part);
    }
    return normalized;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::testkit::*;
    use std::path::Path;

    #[test]
    fn add_worktree_tracks_branch_and_excludes_main_tree() {
        let (_guard, root) = fixture("worktree-add", &[("a.txt", "one\n")], "base");
        let target_guard = TempDir::new("worktree-add-target");
        run_git(Path::new(&root), &["branch", "feature/worktree"]).expect("branch");
        let target = target_guard.path().join("linked");
        let repo = Repo::open(root.as_str()).expect("parent repo");
        repo.add_worktree(target.to_string_lossy().as_ref(), "feature/worktree", None)
            .expect("add linked worktree");

        let worktrees = repo.refs().expect("refs").worktrees;
        assert_eq!(worktrees.len(), 1);
        assert_eq!(
            Path::new(&worktrees[0].path)
                .canonicalize()
                .expect("canonical path"),
            target.canonicalize().expect("canonical target")
        );
        assert_eq!(worktrees[0].branch.as_deref(), Some("feature/worktree"));
        assert!(worktrees[0].exists);
        assert!(!worktrees[0].dirty);
        repo.remove_worktree(target.to_string_lossy().as_ref())
            .expect("remove linked worktree");
    }

    #[test]
    fn remove_refuses_dirty_or_locked_worktrees_then_removes_clean_one() {
        let (_guard, root) = fixture("worktree-remove", &[("a.txt", "one\n")], "base");
        let target_guard = TempDir::new("worktree-remove-target");
        let target = target_guard.path().join("linked");
        let target = target.to_string_lossy().to_string();
        let repo = Repo::open(root.as_str()).expect("parent repo");
        assert!(repo.remove_worktree(root.as_str()).is_err());
        repo.add_worktree(target.as_str(), "feature/remove", Some("HEAD"))
            .expect("add new branch worktree");

        std::fs::write(Path::new(&target).join("a.txt"), "dirty\n").expect("dirty worktree");
        assert!(repo.remove_worktree(target.as_str()).is_err());
        std::fs::write(Path::new(&target).join("a.txt"), "one\n").expect("restore worktree");
        run_git(
            Path::new(&root),
            &["worktree", "lock", "--", target.as_str()],
        )
        .expect("lock worktree");
        assert!(repo.remove_worktree(target.as_str()).is_err());
        run_git(
            Path::new(&root),
            &["worktree", "unlock", "--", target.as_str()],
        )
        .expect("unlock worktree");

        repo.remove_worktree(target.as_str())
            .expect("remove clean worktree");
        assert!(repo.refs().expect("refs").worktrees.is_empty());
        assert!(!Path::new(&target).exists());
    }

    #[test]
    fn prune_removes_only_stale_worktree_registration() {
        let (_guard, root) = fixture("worktree-prune", &[("a.txt", "one\n")], "base");
        let target_guard = TempDir::new("worktree-prune-target");
        let stale_path = target_guard.path().join("stale");
        let stale = stale_path.to_string_lossy().to_string();
        let locked_path = target_guard.path().join("locked");
        let locked = locked_path.to_string_lossy().to_string();
        let valid_path = target_guard.path().join("valid");
        let valid = valid_path.to_string_lossy().to_string();
        let repo = Repo::open(root.as_str()).expect("parent repo");
        repo.add_worktree(stale.as_str(), "feature/prune", Some("HEAD"))
            .expect("add stale worktree");
        repo.add_worktree(locked.as_str(), "feature/prune-locked", Some("HEAD"))
            .expect("add locked worktree");
        repo.add_worktree(valid.as_str(), "feature/prune-valid", Some("HEAD"))
            .expect("add valid worktree");
        run_git(
            Path::new(&root),
            &["worktree", "lock", "--", locked.as_str()],
        )
        .expect("lock worktree");
        std::fs::remove_dir_all(&stale).expect("remove stale worktree files");
        std::fs::remove_dir_all(&locked).expect("remove locked worktree files");
        let before = repo.refs().expect("refs").worktrees;
        assert_eq!(before.len(), 3);
        assert!(before.iter().any(|tree| {
            same_worktree_path(tree.path.as_str(), stale.as_str()) && tree.prunable
        }));
        assert!(before.iter().any(|tree| {
            same_worktree_path(tree.path.as_str(), locked.as_str()) && tree.locked
        }));
        assert!(
            before.iter().any(|tree| {
                same_worktree_path(tree.path.as_str(), valid.as_str()) && tree.exists
            })
        );

        repo.prune_worktrees().expect("prune stale registration");
        let after = repo.refs().expect("refs").worktrees;
        assert!(
            !after
                .iter()
                .any(|tree| same_worktree_path(tree.path.as_str(), stale.as_str()))
        );
        assert!(after.iter().any(|tree| {
            same_worktree_path(tree.path.as_str(), locked.as_str()) && tree.locked
        }));
        assert!(
            after.iter().any(|tree| {
                same_worktree_path(tree.path.as_str(), valid.as_str()) && tree.exists
            })
        );
        repo.remove_worktree(valid.as_str())
            .expect("remove remaining valid worktree");
    }
}
