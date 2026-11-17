//! 引用的操作：分支/标签/远程/贮藏的写路径与工作区是否干净的判断。

use crate::app::ops::refresh_refs;
use crate::app::session::{Tab, load_tab, reload_after_history_change};
use crate::features::changes::ops::{load_change_diff, refresh_changes};
use git::Repo;

/// 工作区是否不干净（有暂存或未暂存的改动）。
pub(crate) fn is_dirty(tab: &Tab) -> bool {
    return !tab.status.is_empty();
}

/// 切换分支并刷新标签页；`stash` 为真时先贮藏改动再切换。
///
/// 带改动切换会让 git 在会覆盖本地修改时拒绝；由界面先问「贮藏 / 带过去 / 取消」。
pub(crate) fn checkout_branch_with(tab: &mut Tab, branch: &str, stash: bool) {
    if stash {
        let repo = match Repo::open(tab.path.as_str()) {
            Ok(repo) => repo,
            Err(error) => {
                tab.error = Some(error.detail());
                return;
            }
        };
        if let Err(error) = repo.stash_push_with("", true) {
            tab.error = Some(error.detail());
            return;
        }
    }
    checkout_branch(tab, branch);
}

/// 切换分支并刷新标签页（历史、引用、变更都要跟着变）。
pub(crate) fn checkout_branch(tab: &mut Tab, branch: &str) {
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    if let Err(error) = repo.checkout_branch(branch) {
        tab.error = Some(error.detail());
        return;
    }
    let filter = tab.filter.clone();
    let refreshed = load_tab(tab.path.as_str(), &filter);
    tab.branch = refreshed.branch;
    tab.ahead = refreshed.ahead;
    tab.behind = refreshed.behind;
    tab.commits = refreshed.commits;
    tab.refs = refreshed.refs;
    tab.status = refreshed.status;
    tab.changes = refreshed.changes;
    tab.conflicts = refreshed.conflicts;
    tab.operation = refreshed.operation;
    tab.reflog = refreshed.reflog;
    tab.selected = refreshed.selected;
    tab.patch = refreshed.patch;
    tab.files = refreshed.files;
    tab.details = refreshed.details;
    tab.error = None;
}

/// 删除本地分支；失败（例如未合并或当前分支）时把原因写进错误。
pub(crate) fn delete_branch(tab: &mut Tab, branch: &str) {
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    match repo.delete_branch(branch) {
        Ok(()) => {
            tab.refs = repo.refs().unwrap_or_default();
            tab.error = None;
        }
        Err(error) => tab.error = Some(error.detail()),
    }
}

/// 从引用新建并切换到本地分支。
pub(crate) fn checkout_new_branch(tab: &mut Tab, name: &str, from: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.checkout_new_branch(name, from)
        .map_err(|error| error.detail())?;
    let filter = tab.filter.clone();
    let refreshed = load_tab(tab.path.as_str(), &filter);
    tab.branch = refreshed.branch;
    tab.refs = refreshed.refs;
    tab.commits = refreshed.commits;
    tab.selected = refreshed.selected;
    tab.error = None;
    return Ok(());
}

/// 游离头时回到默认分支（本地没有就用远端同名分支建一个）。
pub(crate) fn checkout_default_branch(tab: &mut Tab) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    let branches = repo.refs().map_err(|error| error.detail())?.branches;
    let target = if branches.iter().any(|branch| branch.name == "main") {
        "main".to_string()
    } else if branches.iter().any(|branch| branch.name == "master") {
        "master".to_string()
    } else {
        return Err("没有可回到的本地分支".to_string());
    };
    repo.checkout_branch(target.as_str())
        .map_err(|error| error.detail())?;
    let filter = tab.filter.clone();
    let refreshed = load_tab(tab.path.as_str(), &filter);
    tab.commits = refreshed.commits;
    tab.head = refreshed.head;
    tab.branch = refreshed.branch;
    tab.refs = refreshed.refs;
    tab.status = refreshed.status;
    tab.changes = refreshed.changes;
    tab.conflicts = refreshed.conflicts;
    tab.operation = refreshed.operation;
    return Ok(());
}

pub(crate) fn merge_reference(tab: &mut Tab, reference: &str) -> Result<String, String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    let summary = repo.merge(reference).map_err(|error| error.detail())?;
    refresh_changes(tab);
    let filter = tab.filter.clone();
    let refreshed = load_tab(tab.path.as_str(), &filter);
    tab.commits = refreshed.commits;
    tab.refs = refreshed.refs;
    tab.operation = refreshed.operation;
    tab.conflicts = refreshed.conflicts;
    return Ok(summary);
}

/// 把当前分支变基到指定本地分支；冲突时保留 Git 的 rebase 状态供冲突面板继续处理。
pub(crate) fn rebase_reference(tab: &mut Tab, reference: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    let result = repo.rebase_onto(reference).map_err(|error| error.detail());
    reload_after_history_change(tab)?;
    match result {
        Ok(_) => Ok(()),
        Err(_) if tab.operation == git::Operation::Rebase => {
            tab.error = None;
            Ok(())
        }
        Err(detail) => Err(detail),
    }
}

/// 推送指定本地分支，不依赖当前检出的分支。
pub(crate) fn push_branch(tab: &mut Tab, branch: &str, remote: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.push_branch(remote, branch)
        .map_err(|error| error.detail())?;
    return Ok(());
}

/// 丢弃一条贮藏。
pub(crate) fn drop_stash(tab: &mut Tab, reference: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.stash_drop(reference).map_err(|error| error.detail())?;
    refresh_refs(tab, &repo);
    tab.stash_diff = None;
    return Ok(());
}

/// 按贮藏 OID 应用、弹出或删除，避免 `stash@{n}` 重排后指向另一条记录。
pub(crate) fn apply_stash_by_oid(tab: &mut Tab, oid: &str) -> Result<bool, String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    if !repo
        .refs()
        .map_err(|error| error.detail())?
        .stashes
        .iter()
        .any(|stash| stash.oid == oid)
    {
        return Ok(false);
    }
    repo.stash_apply(oid).map_err(|error| error.detail())?;
    refresh_refs(tab, &repo);
    refresh_changes(tab);
    tab.stash_diff = None;
    return Ok(true);
}

pub(crate) fn pop_stash_by_oid(tab: &mut Tab, oid: &str) -> Result<bool, String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    if !repo
        .refs()
        .map_err(|error| error.detail())?
        .stashes
        .iter()
        .any(|stash| stash.oid == oid)
    {
        return Ok(false);
    }
    repo.stash_apply(oid).map_err(|error| error.detail())?;
    refresh_refs(tab, &repo);
    refresh_changes(tab);
    tab.stash_diff = None;
    let reference = repo
        .refs()
        .map_err(|error| error.detail())?
        .stashes
        .into_iter()
        .find(|stash| stash.oid == oid)
        .map(|stash| stash.reference)
        .ok_or_else(|| {
            "The stash was applied, but its entry changed before it could be removed.".to_string()
        })?;
    repo.stash_drop(reference.as_str())
        .map_err(|error| error.detail())?;
    refresh_refs(tab, &repo);
    refresh_changes(tab);
    tab.stash_diff = None;
    return Ok(true);
}

pub(crate) fn drop_stash_by_oid(tab: &mut Tab, oid: &str) -> Result<bool, String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    let Some(reference) = repo
        .refs()
        .map_err(|error| error.detail())?
        .stashes
        .into_iter()
        .find(|stash| stash.oid == oid)
        .map(|stash| stash.reference)
    else {
        return Ok(false);
    };
    repo.stash_drop(reference.as_str())
        .map_err(|error| error.detail())?;
    refresh_refs(tab, &repo);
    tab.stash_diff = None;
    return Ok(true);
}

pub(crate) fn show_stash_diff(
    tab: &Tab,
    oid: &str,
) -> Result<Option<(String, domain::diff::Patch)>, String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    let Some(stash) = repo
        .refs()
        .map_err(|error| error.detail())?
        .stashes
        .into_iter()
        .find(|stash| stash.oid == oid)
    else {
        return Ok(None);
    };
    let patch = repo.stash_show(oid, true).map_err(|error| error.detail())?;
    let patch = domain::diff::Patch::parse(String::from_utf8_lossy(&patch).as_ref());
    return Ok(Some((
        format!("{} · {}", stash.reference, stash.summary),
        patch,
    )));
}

/// 推送标签到首个远端。
pub(crate) fn push_tag(tab: &mut Tab, name: &str) -> Result<String, String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    let remote = repo
        .refs()
        .ok()
        .and_then(|refs| refs.remotes.first().cloned())
        .unwrap_or_else(|| "origin".to_string());
    return repo
        .push_tag(name, Some(remote.as_str()))
        .map_err(|error| error.detail());
}

/// 推送标签到明确指定的远端。
pub(crate) fn push_tag_to_remote(
    tab: &mut Tab,
    name: &str,
    remote: &str,
) -> Result<String, String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    return repo
        .push_tag(name, Some(remote))
        .map_err(|error| error.detail());
}

/// 应用贮藏但保留条目。
pub(crate) fn apply_stash(tab: &mut Tab, reference: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.stash_apply(reference)
        .map_err(|error| error.detail())?;
    tab.refs = repo.refs().unwrap_or_default();
    refresh_changes(tab);
    tab.stash_diff = None;
    return Ok(());
}

/// 弹出贮藏：取出并删除条目。
pub(crate) fn pop_stash(tab: &mut Tab, reference: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    // `git stash pop` 只取最近一条；指定引用时用 `apply` + `drop` 组合，语义一致。
    repo.stash_apply(reference)
        .map_err(|error| error.detail())?;
    repo.stash_drop(reference).map_err(|error| error.detail())?;
    tab.refs = repo.refs().unwrap_or_default();
    refresh_changes(tab);
    tab.stash_diff = None;
    return Ok(());
}

/// 清掉仓库级身份，回到全局配置。
pub(crate) fn clear_identity(tab: &Tab) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.config_unset("user.name")
        .map_err(|error| error.detail())?;
    repo.config_unset("user.email")
        .map_err(|error| error.detail())?;
    return Ok(());
}

/// 删除标签。
pub(crate) fn drop_tag(tab: &mut Tab, name: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.delete_tag(name).map_err(|error| error.detail())?;
    refresh_refs(tab, &repo);
    return Ok(());
}

/// 移除远程。
pub(crate) fn drop_remote(tab: &mut Tab, name: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.remove_remote(name).map_err(|error| error.detail())?;
    refresh_refs(tab, &repo);
    return Ok(());
}

/// 初始化/更新子模块，并刷新父仓库状态，即使 Git 在部分更新后返回失败。
pub(crate) fn update_submodule(
    tab: &mut Tab,
    path: &str,
    latest: bool,
) -> Result<(), domain::GitError> {
    let repo = Repo::open(tab.path.as_str())?;
    let result = repo.update_submodule(path, latest);
    refresh_refs(tab, &repo);
    refresh_changes(tab);
    return result;
}

/// 把 `.gitmodules` 地址同步到本地配置，并刷新父仓库状态。
pub(crate) fn sync_submodule(tab: &mut Tab, path: &str) -> Result<(), domain::GitError> {
    let repo = Repo::open(tab.path.as_str())?;
    let result = repo.sync_submodule(path);
    refresh_refs(tab, &repo);
    refresh_changes(tab);
    return result;
}

/// 按 Git 安全规则移除子模块；删除会暂存父仓库的 gitlink 与 `.gitmodules`。
pub(crate) fn remove_submodule(tab: &mut Tab, path: &str) -> Result<(), domain::GitError> {
    let repo = Repo::open(tab.path.as_str())?;
    let result = repo.remove_submodule(path);
    refresh_refs(tab, &repo);
    refresh_changes(tab);
    return result;
}

/// 暂存父仓库中的 gitlink 变更，并返回当前全部已暂存路径供提交确认使用。
pub(crate) fn stage_submodule_change(
    tab: &mut Tab,
    path: &str,
) -> Result<Option<Vec<String>>, domain::GitError> {
    let repo = Repo::open(tab.path.as_str())?;
    let module = repo
        .refs()?
        .submodules
        .into_iter()
        .find(|module| module.path == path)
        .ok_or_else(|| submodule_error("The submodule registration changed."))?;
    if !module.initialized {
        return Err(submodule_error("The submodule is not initialized."));
    }
    if module.current_oid.is_none() {
        return Err(submodule_error(
            "The current submodule commit could not be read.",
        ));
    }
    if module.current_oid == module.index_oid {
        return Ok(None);
    }
    repo.stage(&[path.to_string()])?;
    refresh_refs(tab, &repo);
    refresh_changes(tab);
    if let Some(index) = tab.changes.iter().position(|change| change.path == path) {
        tab.selected_change = Some(index);
        load_change_diff(tab, index);
    }
    let staged = repo
        .status()?
        .into_iter()
        .filter(|entry| entry.is_staged())
        .map(|entry| entry.path)
        .collect();
    return Ok(Some(staged));
}

fn submodule_error(detail: &str) -> domain::GitError {
    return domain::GitError::CommandFailed {
        program: "git".to_string(),
        args: "submodule".to_string(),
        code: None,
        detail: detail.to_string(),
    };
}

pub(crate) fn pull_remote_branch(
    tab: &mut Tab,
    remote: &str,
    branch: &str,
) -> Result<String, String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    let result = repo
        .pull_remote_branch(remote, branch)
        .map_err(|error| error.detail());
    refresh_changes(tab);
    let filter = tab.filter.clone();
    let refreshed = load_tab(tab.path.as_str(), &filter);
    tab.branch = refreshed.branch;
    tab.ahead = refreshed.ahead;
    tab.behind = refreshed.behind;
    tab.commits = refreshed.commits;
    tab.refs = refreshed.refs;
    tab.remote_urls = refreshed.remote_urls;
    tab.status = refreshed.status;
    tab.conflicts = refreshed.conflicts;
    tab.operation = refreshed.operation;
    tab.head = refreshed.head;
    tab.reflog = refreshed.reflog;
    tab.error = None;
    return result;
}

pub(crate) fn delete_remote_branch(
    tab: &mut Tab,
    remote: &str,
    branch: &str,
) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.delete_remote_branch(remote, branch)
        .map_err(|error| error.detail())?;
    let fetch_result = repo.fetch(remote).map_err(|error| error.detail());
    refresh_refs(tab, &repo);
    fetch_result.map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::load_tab;
    use crate::features::search::ops::{add_remote, edit_remote, rename_branch, stash_changes};
    use crate::test_support::TempRepo;
    use git::LogFilter;

    #[test]
    fn detached_head_can_return_to_the_default_branch() {
        let repo = TempRepo::new("detached");
        let path = repo.path.display().to_string();
        std::fs::write(repo.path.join("b.txt"), "x\n").expect("write");
        for args in [vec!["add", "-A"], vec!["commit", "-q", "-m", "second"]] {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&repo.path)
                .args(&args)
                .status()
                .expect("git");
            assert!(status.success(), "git {args:?}");
        }

        let tab = load_tab(&path, &LogFilter::default());
        let target = tab.commits[1].sha.clone();
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo.path)
            .args(["checkout", "-q", target.as_str()])
            .status()
            .expect("git");
        assert!(status.success());

        let mut tab = load_tab(&path, &LogFilter::default());
        assert!(
            matches!(tab.head, git::HeadState::Detached { .. }),
            "检出提交后应是游离头：{:?}",
            tab.head
        );

        checkout_default_branch(&mut tab).expect("back to main");
        assert_eq!(tab.head, git::HeadState::Branch("main".to_string()));
    }

    #[test]
    fn refs_actions_rename_branch_and_apply_stash() {
        let repo = TempRepo::new("refs-actions");
        let path = repo.path.display().to_string();

        // 贮藏：改动先藏起来，列表里有条目；应用后改动回来且条目还在。
        std::fs::write(repo.path.join("a.txt"), "stashed\n").expect("write");
        let mut tab = load_tab(&path, &LogFilter::default());
        stash_changes(&mut tab, "wip", true).expect("stash");
        assert_eq!(tab.refs.stashes.len(), 1);
        let reference = tab.refs.stashes[0].reference.clone();

        apply_stash(&mut tab, reference.as_str()).expect("apply");
        assert_eq!(
            std::fs::read_to_string(repo.path.join("a.txt")).expect("read"),
            "stashed\n"
        );
        assert_eq!(tab.refs.stashes.len(), 1, "apply 不删除条目");

        // 先把工作区复原，否则再应用会与本地改动冲突。
        std::fs::write(repo.path.join("a.txt"), "one\n").expect("restore");

        // 弹出：条目消失。
        pop_stash(&mut tab, reference.as_str()).expect("pop");
        assert!(tab.refs.stashes.is_empty(), "pop 删除条目");

        // 重命名分支：引用列表与当前分支名一起变。
        rename_branch(&mut tab, "main", "renamed").expect("rename");
        assert_eq!(tab.branch, "renamed");
        assert!(
            tab.refs
                .branches
                .iter()
                .any(|branch| branch.name == "renamed")
        );

        // 远端 URL 列表与编辑。
        add_remote(&mut tab, "origin", "https://example.com/a.git").expect("add remote");
        assert!(tab.remote_urls.iter().any(|(name, _)| name == "origin"));
        edit_remote(&mut tab, "origin", "git@example.com:b.git").expect("edit remote");
        assert!(
            tab.remote_urls
                .iter()
                .any(|(_, url)| url == "git@example.com:b.git")
        );
    }

    #[test]
    fn stash_actions_by_oid_keep_the_original_target_after_stack_reorders() {
        let repo = TempRepo::new("stash-oid");
        let path = repo.path.display().to_string();
        let mut tab = load_tab(&path, &LogFilter::default());

        std::fs::write(repo.path.join("a.txt"), "first stash\n").expect("first change");
        stash_changes(&mut tab, "first", true).expect("first stash");
        let first_oid = tab.refs.stashes[0].oid.clone();

        std::fs::write(repo.path.join("a.txt"), "second stash\n").expect("second change");
        stash_changes(&mut tab, "second", true).expect("second stash");
        let second_oid = tab.refs.stashes[0].oid.clone();
        assert_ne!(first_oid, second_oid);

        let (label, patch) = show_stash_diff(&tab, first_oid.as_str())
            .expect("stash diff query")
            .expect("first stash still listed");
        assert!(label.contains("first"));
        assert!(patch.raw.contains("first stash"));

        assert!(apply_stash_by_oid(&mut tab, first_oid.as_str()).expect("apply by oid"));
        assert_eq!(
            std::fs::read_to_string(repo.path.join("a.txt")).expect("read applied"),
            "first stash\n"
        );
        std::fs::write(repo.path.join("a.txt"), "one\n").expect("restore base");

        assert!(pop_stash_by_oid(&mut tab, first_oid.as_str()).expect("pop by oid"));
        assert_eq!(
            std::fs::read_to_string(repo.path.join("a.txt")).expect("read popped"),
            "first stash\n"
        );
        assert!(tab.refs.stashes.iter().any(|stash| stash.oid == second_oid));
        assert!(!tab.refs.stashes.iter().any(|stash| stash.oid == first_oid));
        assert!(drop_stash_by_oid(&mut tab, second_oid.as_str()).expect("drop by oid"));
        assert!(tab.refs.stashes.is_empty());
    }

    #[test]
    fn checkout_switches_branch_and_refreshes_history() {
        let repo = TempRepo::new("checkout");
        let path = repo.path.display().to_string();
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&repo.path)
                .args(args)
                .status()
                .expect("git");
            assert!(status.success(), "git {:?}", args);
        };
        run(&["branch", "feature"]);
        let mut tab = load_tab(&path, &LogFilter::default());
        assert_eq!(tab.branch, "main");

        checkout_branch(&mut tab, "feature");
        assert_eq!(tab.branch, "feature");
        assert!(tab.error.is_none());
        assert!(
            tab.refs
                .branches
                .iter()
                .any(|branch| branch.name == "feature" && branch.current)
        );
    }

    #[test]
    fn deleting_the_current_branch_is_refused() {
        let repo = TempRepo::new("deletecurrent");
        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        assert_eq!(tab.branch, "main");
        delete_branch(&mut tab, "main");
        assert!(tab.error.is_some(), "应报告不能删除当前分支");
        assert!(tab.refs.branches.iter().any(|branch| branch.name == "main"));
    }

    #[test]
    fn dirty_checkout_can_stash_first() {
        let root = TempRepo::new("dirty-checkout");
        let branch = "feature/dirty";
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&root.path)
                .args(args)
                .status()
                .expect("git");
            assert!(status.success(), "git {:?}", args);
        };
        run(&["branch", branch]);
        let filter = LogFilter::default();
        let mut tab = load_tab(root.path.display().to_string().as_str(), &filter);
        assert!(!is_dirty(&tab), "干净仓库不应被判为脏");
        // 制造一个未暂存改动。
        std::fs::write(root.path.join("a.txt"), "two\n").expect("write");
        let tab_dirty = load_tab(root.path.display().to_string().as_str(), &filter);
        assert!(is_dirty(&tab_dirty));
        // 先贮藏再切换：工作区干净，改动在贮藏里。
        checkout_branch_with(&mut tab, branch, true);
        assert_eq!(tab.branch, branch);
        assert!(!is_dirty(&tab), "贮藏后工作区应干净");
        assert_eq!(tab.refs.stashes.len(), 1, "改动应在贮藏里");
        assert_eq!(
            std::fs::read_to_string(root.path.join("a.txt")).expect("read"),
            "one\n",
            "切分支后文件回到分支上的内容"
        );
    }
}
