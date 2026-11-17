//! 引用的操作：分支/标签/远程/贮藏的写路径与工作区是否干净的判断。

use crate::app::ops::refresh_refs;
use crate::app::session::{Tab, load_tab};
use crate::features::changes::ops::refresh_changes;
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

/// 丢弃一条贮藏。
pub(crate) fn drop_stash(tab: &mut Tab, reference: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.stash_drop(reference).map_err(|error| error.detail())?;
    refresh_refs(tab, &repo);
    return Ok(());
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

/// 应用贮藏但保留条目。
pub(crate) fn apply_stash(tab: &mut Tab, reference: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.stash_apply(reference)
        .map_err(|error| error.detail())?;
    tab.refs = repo.refs().unwrap_or_default();
    refresh_changes(tab);
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
