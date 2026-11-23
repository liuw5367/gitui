//! 搜索页表单的操作：分支/标签/远程的创建与改写、贮藏与提交身份。

use crate::app::ops::refresh_refs;
use crate::app::session::Tab;
use crate::features::changes::ops::refresh_changes;

/// 新建分支；`from` 为空时基于当前 HEAD。
pub(crate) fn create_branch(
    tab: &mut Tab,
    name: &str,
    from: &str,
    checkout: bool,
) -> Result<(), String> {
    let repo = tab.open_repo().map_err(|error| error.detail())?;
    let from = if from.trim().is_empty() {
        None
    } else {
        Some(from.trim())
    };
    match (checkout, from) {
        // 建完就切：`checkout -b` 一步到位，别留一个看不见的新分支。
        (true, Some(from)) => repo
            .checkout_new_branch(name, from)
            .map_err(|error| error.detail())?,
        (true, None) => repo
            .checkout_new_branch(name, "HEAD")
            .map_err(|error| error.detail())?,
        (false, from) => repo
            .create_branch(name, from)
            .map_err(|error| error.detail())?,
    }
    refresh_refs(tab, &repo);
    if checkout {
        let filter = tab.filter.clone();
        let refreshed = crate::app::session::load_tab_with_executable(
            tab.path.as_str(),
            &filter,
            &tab.git_executable,
        );
        tab.branch = refreshed.branch;
        tab.ahead = refreshed.ahead;
        tab.behind = refreshed.behind;
        tab.status = refreshed.status;
        tab.changes = refreshed.changes;
        tab.conflicts = refreshed.conflicts;
        tab.operation = refreshed.operation;
    }
    return Ok(());
}

/// 从远端跟踪引用创建并检出本地跟踪分支。
pub(crate) fn checkout_tracking_branch(
    tab: &mut Tab,
    name: &str,
    remote_branch: &str,
) -> Result<(), String> {
    let repo = tab.open_repo().map_err(|error| error.detail())?;
    repo.checkout_tracking_branch(name, remote_branch)
        .map_err(|error| error.detail())?;
    let filter = tab.filter.clone();
    let refreshed = crate::app::session::load_tab_with_executable(
        tab.path.as_str(),
        &filter,
        &tab.git_executable,
    );
    tab.branch = refreshed.branch;
    tab.ahead = refreshed.ahead;
    tab.behind = refreshed.behind;
    tab.commits = refreshed.commits;
    tab.refs = refreshed.refs;
    tab.remote_urls = refreshed.remote_urls;
    tab.status = refreshed.status;
    tab.changes = refreshed.changes;
    tab.conflicts = refreshed.conflicts;
    tab.operation = refreshed.operation;
    tab.head = refreshed.head;
    tab.reflog = refreshed.reflog;
    tab.error = None;
    return Ok(());
}

/// 新建标签。
pub(crate) fn create_tag(
    tab: &mut Tab,
    name: &str,
    from: &str,
    message: &str,
    annotated: bool,
) -> Result<(), String> {
    let repo = tab.open_repo().map_err(|error| error.detail())?;
    let from = if from.trim().is_empty() {
        None
    } else {
        Some(from.trim())
    };
    repo.create_tag_with(name, from, message, annotated)
        .map_err(|error| error.detail())?;
    refresh_refs(tab, &repo);
    return Ok(());
}

/// 添加远程。
pub(crate) fn add_remote(tab: &mut Tab, name: &str, url: &str) -> Result<(), String> {
    let repo = tab.open_repo().map_err(|error| error.detail())?;
    repo.add_remote(name, url).map_err(|error| error.detail())?;
    refresh_refs(tab, &repo);
    return Ok(());
}

/// 贮藏当前改动。
pub(crate) fn stash_changes(
    tab: &mut Tab,
    message: &str,
    include_untracked: bool,
) -> Result<(), String> {
    let repo = tab.open_repo().map_err(|error| error.detail())?;
    repo.stash_push_with(message, include_untracked)
        .map_err(|error| error.detail())?;
    tab.refs = repo.refs().unwrap_or_default();
    refresh_changes(tab);
    tab.stash_diff = None;
    return Ok(());
}

/// 贮藏所选文件；写入前重新核对路径仍有变更，避免使用已过期的表单选择。
pub(crate) fn stash_selected_files(
    tab: &mut Tab,
    message: &str,
    paths: &[String],
    include_untracked: bool,
) -> Result<(), String> {
    let repo = tab.open_repo().map_err(|error| error.detail())?;
    if paths.is_empty() {
        return Err("Select at least one changed file.".to_string());
    }
    let status = repo.status().map_err(|error| error.detail())?;
    if paths
        .iter()
        .any(|path| !status.iter().any(|entry| entry.path == *path))
    {
        return Err("A selected file changed before the stash was created. Reopen the picker and try again.".to_string());
    }
    let path_refs = paths.iter().map(String::as_str).collect::<Vec<_>>();
    repo.stash_push_selected(message, &path_refs, include_untracked)
        .map_err(|error| error.detail())?;
    crate::app::ops::refresh_refs(tab, &repo);
    refresh_changes(tab);
    tab.stash_diff = None;
    return Ok(());
}

/// 重命名分支。
pub(crate) fn rename_branch(tab: &mut Tab, old: &str, new: &str) -> Result<(), String> {
    let repo = tab.open_repo().map_err(|error| error.detail())?;
    repo.rename_branch(old, new)
        .map_err(|error| error.detail())?;
    tab.refs = repo.refs().unwrap_or_default();
    tab.branch = repo.abbrev_branch().unwrap_or_default();
    return Ok(());
}

/// 写入仓库级提交身份。
pub(crate) fn set_identity(tab: &Tab, name: &str, email: &str) -> Result<(), String> {
    let repo = tab.open_repo().map_err(|error| error.detail())?;
    repo.config_set("user.name", name)
        .map_err(|error| error.detail())?;
    repo.config_set("user.email", email)
        .map_err(|error| error.detail())?;
    return Ok(());
}

/// 改写远端的 URL。
pub(crate) fn edit_remote(tab: &mut Tab, name: &str, url: &str) -> Result<(), String> {
    let repo = tab.open_repo().map_err(|error| error.detail())?;
    repo.set_remote_url(name, url)
        .map_err(|error| error.detail())?;
    refresh_refs(tab, &repo);
    return Ok(());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::load_tab;
    use crate::features::refs::ops::drop_stash;
    use crate::test_support::TempRepo;
    use git::LogFilter;

    #[test]
    fn ref_forms_create_branches_tags_and_remotes() {
        let repo = TempRepo::new("refwrite");
        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        let before = tab.refs.branches.len();

        create_branch(&mut tab, "feature/x", "", false).expect("branch");
        assert_eq!(tab.refs.branches.len(), before + 1);
        assert!(
            tab.refs
                .branches
                .iter()
                .any(|branch| branch.name == "feature/x")
        );

        create_tag(&mut tab, "v0.0.1", "", "", false).expect("tag");
        assert_eq!(tab.refs.tags, vec!["v0.0.1".to_string()]);

        add_remote(&mut tab, "origin", "https://example.com/repo.git").expect("remote");
        assert_eq!(tab.refs.remotes, vec!["origin".to_string()]);

        // 空名字被 Git 拒绝，错误应回传给调用方。
        assert!(create_branch(&mut tab, "", "", false).is_err());
    }

    #[test]
    fn stash_changes_moves_worktree_content_away() {
        let repo = TempRepo::new("stashmodel");
        std::fs::write(repo.path.join("a.txt"), "changed\n").expect("write");
        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        assert!(!tab.changes.is_empty());

        stash_changes(&mut tab, "wip", true).expect("stash");
        assert!(tab.changes.is_empty(), "贮藏后变更列表应为空");
        assert_eq!(tab.refs.stashes.len(), 1);

        let reference = tab.refs.stashes[0].reference.clone();
        drop_stash(&mut tab, reference.as_str()).expect("drop");
        assert!(tab.refs.stashes.is_empty());
    }

    #[test]
    fn selected_file_stash_leaves_other_worktree_paths_unchanged() {
        let repo = TempRepo::new("selected-stash");
        std::fs::write(repo.path.join("a.txt"), "selected change\n").expect("modify tracked");
        std::fs::write(repo.path.join("new.txt"), "leave untracked\n").expect("new file");
        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());

        stash_selected_files(&mut tab, "selected", &["a.txt".to_string()], false)
            .expect("stash selected path");

        assert_eq!(
            std::fs::read_to_string(repo.path.join("a.txt")).expect("read tracked"),
            "one\n"
        );
        assert_eq!(
            std::fs::read_to_string(repo.path.join("new.txt")).expect("read untracked"),
            "leave untracked\n"
        );
        assert_eq!(tab.changes.len(), 1);
        assert_eq!(tab.changes[0].path, "new.txt");
        assert_eq!(tab.refs.stashes.len(), 1);
    }
}
