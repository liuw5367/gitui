//! 子模块登记、更新与安全移除。

use domain::GitError;

use crate::exec::run_git;
use crate::query::Repo;

impl Repo {
    /// 初始化并检出父仓库记录的子模块提交；`latest` 跟随 `.gitmodules` 分支更新。
    pub fn update_submodule(&self, path: &str, latest: bool) -> Result<(), GitError> {
        let module = self
            .refs()?
            .submodules
            .into_iter()
            .find(|module| module.path == path)
            .ok_or_else(|| {
                command_error("submodule update", "子模块登记已变化，请重新打开菜单。")
            })?;
        if latest && module.branch.is_none() {
            return Err(command_error(
                "submodule update --remote",
                "子模块没有配置跟踪分支。",
            ));
        }
        let mut args = vec!["submodule", "update", "--init"];
        if latest {
            args.push("--remote");
        }
        args.extend(["--", path]);
        run_git(&self.root, &args)?;
        return Ok(());
    }

    /// 把 `.gitmodules` 中的 URL 同步到子模块本地配置。
    pub fn sync_submodule(&self, path: &str) -> Result<(), GitError> {
        if !self
            .refs()?
            .submodules
            .iter()
            .any(|module| module.path == path)
        {
            return Err(command_error(
                "submodule sync",
                "子模块登记已变化，请重新打开菜单。",
            ));
        }
        run_git(&self.root, &["submodule", "sync", "--", path])?;
        return Ok(());
    }

    /// 按 Git 的安全规则删除子模块登记与工作目录，不强制覆盖本地改动。
    pub fn remove_submodule(&self, path: &str) -> Result<(), GitError> {
        let module = self
            .refs()?
            .submodules
            .into_iter()
            .find(|module| module.path == path)
            .ok_or_else(|| {
                command_error("rm -- <submodule>", "子模块登记已变化，请重新打开菜单。")
            })?;
        if module.initialized && (module.dirty || module.current_oid != module.index_oid) {
            return Err(command_error(
                format!("rm -- {path}").as_str(),
                "子模块有本地改动或未记录的提交；请先处理改动后再移除。",
            ));
        }
        run_git(&self.root, &["rm", "--", path])?;
        return Ok(());
    }
}

fn command_error(args: &str, detail: &str) -> GitError {
    return GitError::CommandFailed {
        program: "git".to_string(),
        args: args.to_string(),
        code: None,
        detail: detail.to_string(),
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::testkit::*;
    use std::path::Path;
    use std::process::Command;

    fn add_local_submodule(parent: &str, source: &str, path: &str) {
        let status = Command::new("git")
            .arg("-C")
            .arg(parent)
            .args(["-c", "protocol.file.allow=always", "submodule", "add", "-q"])
            .arg(source)
            .arg(path)
            .status()
            .expect("git submodule add");
        assert!(status.success());
        run_git(Path::new(parent), &["add", ".gitmodules", path]).expect("stage module");
        run_git(Path::new(parent), &["commit", "-q", "-m", "add submodule"])
            .expect("commit module");
    }

    #[test]
    fn refs_read_submodule_url_branch_and_checkout_state() {
        let (_child_guard, child_path) = fixture("submodule-child", &[("a.txt", "one\n")], "base");
        let (_parent_guard, parent_path) =
            fixture("submodule-parent", &[("root.txt", "root\n")], "base");
        add_local_submodule(&parent_path, child_path.as_str(), "deps/child");
        run_git(
            Path::new(&parent_path),
            &[
                "config",
                "--file",
                ".gitmodules",
                "submodule.deps/child.branch",
                "main",
            ],
        )
        .expect("set tracked branch");
        run_git(Path::new(&parent_path), &["add", ".gitmodules"]).expect("stage module config");
        run_git(
            Path::new(&parent_path),
            &["commit", "-q", "-m", "configure branch"],
        )
        .expect("commit module config");

        let parent = Repo::open(parent_path.as_str()).expect("parent repo");
        let refs = parent.refs().expect("refs");
        assert_eq!(refs.submodules.len(), 1);
        let module = &refs.submodules[0];
        assert_eq!(module.path, "deps/child");
        assert_eq!(module.url, child_path);
        assert_eq!(module.branch.as_deref(), Some("main"));
        assert!(module.initialized);
        assert_eq!(module.current_oid, module.index_oid);
        assert!(!module.dirty);

        run_git(
            Path::new(&parent_path),
            &["submodule", "deinit", "-f", "--", "deps/child"],
        )
        .expect("deinitialize module");
        let uninitialized = parent.refs().expect("refs").submodules.remove(0);
        assert!(!uninitialized.initialized);
        assert_eq!(uninitialized.path, "deps/child");
        assert_eq!(uninitialized.url, child_path);
    }

    #[test]
    fn update_latest_follows_configured_branch_and_changes_only_the_gitlink() {
        let (_child_guard, child_path) =
            fixture("submodule-update-child", &[("a.txt", "one\n")], "base");
        let (_parent_guard, parent_path) =
            fixture("submodule-update-parent", &[("root.txt", "root\n")], "base");
        add_local_submodule(&parent_path, child_path.as_str(), "deps/child");
        run_git(
            Path::new(&parent_path),
            &[
                "config",
                "--file",
                ".gitmodules",
                "submodule.deps/child.branch",
                "main",
            ],
        )
        .expect("set tracked branch");
        run_git(Path::new(&parent_path), &["add", ".gitmodules"]).expect("stage module config");
        run_git(
            Path::new(&parent_path),
            &["commit", "-q", "-m", "configure branch"],
        )
        .expect("commit module config");
        let parent = Repo::open(parent_path.as_str()).expect("parent repo");
        run_git(
            &Path::new(&parent_path).join("deps/child"),
            &["config", "protocol.file.allow", "always"],
        )
        .expect("allow local source in fixture");
        let before = parent.refs().expect("refs").submodules.remove(0);

        std::fs::write(Path::new(&child_path).join("a.txt"), "two\n").expect("child edit");
        run_git(Path::new(&child_path), &["add", "a.txt"]).expect("stage child");
        run_git(Path::new(&child_path), &["commit", "-q", "-m", "second"]).expect("child commit");
        parent
            .update_submodule("deps/child", true)
            .expect("update remote branch");

        let after = parent.refs().expect("refs").submodules.remove(0);
        assert_ne!(after.current_oid, before.current_oid);
        assert_eq!(after.index_oid, before.index_oid);
        assert_ne!(after.current_oid, after.index_oid);
    }

    #[test]
    fn remove_refuses_dirty_submodules_then_removes_a_clean_registration() {
        let (_child_guard, child_path) =
            fixture("submodule-remove-child", &[("a.txt", "one\n")], "base");
        let (_parent_guard, parent_path) =
            fixture("submodule-remove-parent", &[("root.txt", "root\n")], "base");
        add_local_submodule(&parent_path, child_path.as_str(), "deps/child");
        let parent = Repo::open(parent_path.as_str()).expect("parent repo");
        let nested_path = Path::new(&parent_path).join("deps/child");
        std::fs::write(nested_path.join("a.txt"), "dirty\n").expect("dirty child");
        assert!(parent.remove_submodule("deps/child").is_err());
        assert_eq!(parent.refs().expect("refs").submodules.len(), 1);

        std::fs::write(nested_path.join("a.txt"), "one\n").expect("restore child");
        let recorded_oid = parent
            .refs()
            .expect("refs")
            .submodules
            .remove(0)
            .index_oid
            .expect("recorded gitlink");
        std::fs::write(nested_path.join("a.txt"), "committed locally\n").expect("child edit");
        run_git(&nested_path, &["add", "a.txt"]).expect("stage child edit");
        run_git(&nested_path, &["commit", "-q", "-m", "unrecorded commit"])
            .expect("commit child edit");
        assert!(parent.remove_submodule("deps/child").is_err());
        run_git(&nested_path, &["checkout", "-q", recorded_oid.as_str()])
            .expect("return to recorded gitlink");
        parent
            .remove_submodule("deps/child")
            .expect("remove clean child");
        assert!(parent.refs().expect("refs").submodules.is_empty());
        assert!(!nested_path.exists());
    }
}
