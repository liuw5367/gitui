//! 合并与冲突的操作：冲突视图的浏览、按侧解决、标记已解决与继续/放弃操作。

use crate::app::session::{ConflictDraft, ResolvedConflict, Tab, reload_after_history_change};
use crate::features::changes::ops::refresh_changes;
use git::Repo;

#[derive(Debug)]
pub(crate) enum MarkResolvedError {
    UnresolvedMarkers,
    BinaryNeedsSide,
    Git(String),
}

#[derive(Debug)]
pub(crate) enum ContinueError {
    UnresolvedConflicts,
    PendingDrafts,
    NoOperation,
    OperationChanged,
    Git(String),
}

#[derive(Debug)]
pub(crate) enum AbortError {
    NoOperation,
    OperationChanged,
    Git(String),
}

pub(crate) fn operation_label(language: &str, operation: git::Operation) -> String {
    let key = match operation {
        git::Operation::None => "operation.none",
        git::Operation::Merge => "operation.merge",
        git::Operation::Rebase => "operation.rebase",
        git::Operation::CherryPick => "operation.cherry-pick",
        git::Operation::Revert => "operation.revert",
    };
    return crate::i18n::t(language, key);
}

/// 打开冲突视图，并载入第一个冲突文件的内容。
pub(crate) fn open_conflict(tab: &mut Tab) {
    tab.conflict_open = true;
    for path in &tab.conflicts {
        if !tab.conflict_paths.contains(path) {
            tab.conflict_paths.push(path.clone());
        }
    }
    tab.conflict_index = tab
        .conflict_paths
        .iter()
        .position(|path| tab.conflicts.contains(path))
        .unwrap_or(0);
    load_conflict_text(tab);
}

/// 切换到第 `index` 个冲突文件。
pub(crate) fn select_conflict(tab: &mut Tab, index: usize) {
    if index >= tab.conflict_paths.len() {
        return;
    }
    tab.conflict_index = index;
    load_conflict_text(tab);
}

/// 前一个/后一个冲突文件（到头就停住，不回绕，避免误操作）。
pub(crate) fn step_conflict(tab: &mut Tab, forward: bool) {
    if tab.conflict_paths.is_empty() {
        return;
    }
    let next = if forward {
        (tab.conflict_index + 1).min(tab.conflict_paths.len() - 1)
    } else {
        tab.conflict_index.saturating_sub(1)
    };
    select_conflict(tab, next);
}

fn load_conflict_text(tab: &mut Tab) {
    let Some(path) = tab.conflict_paths.get(tab.conflict_index).cloned() else {
        tab.conflict_text.clear();
        tab.conflict_sides = None;
        tab.conflict_binary = false;
        return;
    };
    if tab.conflicts.contains(&path) {
        tab.resolved_conflicts.remove(&path);
    }
    if let Some(resolved) = tab.resolved_conflicts.get(&path) {
        tab.conflict_sides = resolved.sides.clone();
        tab.conflict_binary = matches!(resolved.result, ConflictDraft::Binary(_));
        tab.conflict_text = match &resolved.result {
            ConflictDraft::Text(value) => value.clone(),
            ConflictDraft::Binary(_) | ConflictDraft::Delete => String::new(),
        };
        return;
    }
    let repo = match tab.open_repo() {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            tab.conflict_text.clear();
            tab.conflict_sides = None;
            tab.conflict_binary = false;
            return;
        }
    };
    let worktree = match repo.conflict_worktree_content(path.as_str()) {
        Ok(worktree) => worktree,
        Err(error) => {
            tab.error = Some(error.detail());
            None
        }
    };
    match repo.conflict_sides(path.as_str()) {
        Ok(sides) => tab.conflict_sides = Some(sides),
        Err(error) => {
            tab.error = Some(error.detail());
            tab.conflict_sides = None;
        }
    }
    tab.conflict_binary = worktree
        .as_deref()
        .is_some_and(|bytes| conflict_text(bytes).is_none());
    tab.conflict_text = worktree
        .as_deref()
        .and_then(conflict_text)
        .unwrap_or_default();
}

/// 当前文件的结果值来自草稿、已解决快照或工作区文件。
pub(crate) fn conflict_result(tab: &Tab) -> String {
    let Some(path) = tab.conflict_paths.get(tab.conflict_index) else {
        return String::new();
    };
    if let Some(resolved) = tab.resolved_conflicts.get(path) {
        return match &resolved.result {
            ConflictDraft::Text(value) => value.clone(),
            ConflictDraft::Binary(_) | ConflictDraft::Delete => String::new(),
        };
    }
    return match tab.conflict_drafts.get(path) {
        Some(ConflictDraft::Text(value)) => value.clone(),
        Some(ConflictDraft::Binary(_) | ConflictDraft::Delete) => String::new(),
        None => tab.conflict_text.clone(),
    };
}

/// 立即把编辑器变化写入当前仓库标签的路径草稿。
pub(crate) fn update_conflict_result(tab: &mut Tab, value: String) {
    let Some(path) = tab.conflict_paths.get(tab.conflict_index).cloned() else {
        return;
    };
    if !tab.conflicts.contains(&path) {
        return;
    }
    tab.conflict_drafts.insert(path, ConflictDraft::Text(value));
}

/// 把当前冲突文件的某一侧整体写进结果。
pub(crate) fn conflict_take_side(tab: &mut Tab, side: domain::conflict::Side) -> String {
    let Some(path) = tab.conflict_paths.get(tab.conflict_index).cloned() else {
        return String::new();
    };
    if !tab.conflicts.contains(&path) {
        return String::new();
    }
    let draft = match side {
        domain::conflict::Side::Both => {
            ConflictDraft::Text(domain::conflict::merge(tab.conflict_text.as_str(), side))
        }
        domain::conflict::Side::Ours | domain::conflict::Side::Theirs => {
            let content = tab.conflict_sides.as_ref().and_then(|sides| match side {
                domain::conflict::Side::Ours => sides.ours.clone(),
                domain::conflict::Side::Theirs => sides.theirs.clone(),
                domain::conflict::Side::Both => None,
            });
            match content {
                None => ConflictDraft::Delete,
                Some(bytes) => conflict_text(&bytes)
                    .map(ConflictDraft::Text)
                    .unwrap_or(ConflictDraft::Binary(bytes)),
            }
        }
    };
    tab.conflict_drafts.insert(path, draft);
    return conflict_result(tab);
}

/// 标记当前冲突文件为已解决：写入结果并 `git add`。
pub(crate) fn mark_conflict_resolved(
    tab: &mut Tab,
    resolved: &str,
) -> Result<(), MarkResolvedError> {
    let Some(path) = tab.conflict_paths.get(tab.conflict_index).cloned() else {
        return Ok(());
    };
    if !tab.conflicts.contains(&path) {
        return Ok(());
    }
    let repo = tab
        .open_repo()
        .map_err(|error| MarkResolvedError::Git(error.detail()))?;
    let result = tab
        .conflict_drafts
        .get(&path)
        .cloned()
        .unwrap_or_else(|| ConflictDraft::Text(resolved.to_string()));
    match &result {
        ConflictDraft::Binary(bytes) => repo.resolve_conflict_bytes(path.as_str(), bytes),
        ConflictDraft::Delete => repo.resolve_conflict_delete(path.as_str()),
        ConflictDraft::Text(_) => {
            if tab.conflict_binary && !tab.conflict_drafts.contains_key(&path) {
                return Err(MarkResolvedError::BinaryNeedsSide);
            }
            if domain::conflict::has_markers(resolved) {
                return Err(MarkResolvedError::UnresolvedMarkers);
            }
            repo.resolve_conflict(path.as_str(), resolved)
        }
    }
    .map_err(|error| MarkResolvedError::Git(error.detail()))?;
    tab.conflict_drafts.remove(&path);
    let sides = tab.conflict_sides.clone();
    tab.resolved_conflicts
        .insert(path.clone(), ResolvedConflict { sides, result });
    tab.conflicts = repo
        .conflicts()
        .map_err(|error| MarkResolvedError::Git(error.detail()))?;
    for unresolved in &tab.conflicts {
        if !tab.conflict_paths.contains(unresolved) {
            tab.conflict_paths.push(unresolved.clone());
        }
    }
    refresh_changes(tab);
    tab.conflict_index = tab
        .conflict_paths
        .iter()
        .position(|candidate| tab.conflicts.contains(candidate))
        .or_else(|| {
            tab.conflict_paths
                .iter()
                .position(|candidate| candidate == &path)
        })
        .unwrap_or(0);
    load_conflict_text(tab);
    return Ok(());
}

/// 继续被冲突打断的操作；成功后刷新整个 tab。
pub(crate) fn continue_operation(tab: &mut Tab) -> Result<String, ContinueError> {
    let repo = tab
        .open_repo()
        .map_err(|error| ContinueError::Git(error.detail()))?;
    let operation = repo.operation();
    let conflicts = repo
        .conflicts()
        .map_err(|error| ContinueError::Git(error.detail()))?;
    if operation != tab.operation {
        refresh_operation_state(tab, &repo, false, true, true).map_err(ContinueError::Git)?;
        return Err(ContinueError::OperationChanged);
    }
    tab.operation = operation;
    tab.conflicts = conflicts;
    if operation == git::Operation::None {
        sync_conflict_editor(tab, false);
        return Err(ContinueError::NoOperation);
    }
    if !tab.conflicts.is_empty() {
        sync_conflict_editor(tab, true);
        return Err(ContinueError::UnresolvedConflicts);
    }
    if !tab.conflict_drafts.is_empty() {
        return Err(ContinueError::PendingDrafts);
    }
    let summary = match repo.continue_operation(operation) {
        Ok(summary) => summary,
        Err(error) => {
            let detail = error.detail();
            refresh_operation_state(tab, &repo, true, true, false).map_err(ContinueError::Git)?;
            if tab.operation != git::Operation::None && !tab.conflicts.is_empty() {
                return Err(ContinueError::UnresolvedConflicts);
            }
            return Err(ContinueError::Git(detail));
        }
    };
    let branch = reload_after_history_change(tab).map_err(ContinueError::Git)?;
    return Ok(if summary.trim().is_empty() {
        branch
    } else {
        summary
    });
}

/// 用一侧内容解决冲突（`ours` 为真取当前分支）。
pub(crate) fn resolve_conflict(tab: &mut Tab, path: &str, ours: bool) {
    let repo = match tab.open_repo() {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    let resolved_snapshot = if tab.operation == git::Operation::None {
        None
    } else {
        repo.conflict_sides(path).ok().map(|sides| {
            let bytes = if ours {
                sides.ours.clone()
            } else {
                sides.theirs.clone()
            };
            let result = bytes
                .map(|bytes| {
                    conflict_text(&bytes)
                        .map(ConflictDraft::Text)
                        .unwrap_or(ConflictDraft::Binary(bytes))
                })
                .unwrap_or(ConflictDraft::Delete);
            ResolvedConflict {
                sides: Some(sides),
                result,
            }
        })
    };
    match repo.resolve_with(path, ours) {
        Ok(()) => {
            tab.conflict_drafts.remove(path);
            if let Some(resolved) = resolved_snapshot {
                tab.resolved_conflicts.insert(path.to_string(), resolved);
                if !tab.conflict_paths.iter().any(|candidate| candidate == path) {
                    tab.conflict_paths.push(path.to_string());
                }
            }
            refresh_changes(tab);
            tab.conflicts = match repo.conflicts() {
                Ok(conflicts) => conflicts,
                Err(error) => {
                    tab.error = Some(error.detail());
                    return;
                }
            };
            for unresolved in &tab.conflicts {
                tab.resolved_conflicts.remove(unresolved);
                if !tab.conflict_paths.contains(unresolved) {
                    tab.conflict_paths.push(unresolved.clone());
                }
            }
            if let Some(index) = tab.conflict_paths.iter().position(|item| item == path) {
                tab.conflict_index = index;
                load_conflict_text(tab);
            }
        }
        Err(error) => tab.error = Some(error.detail()),
    }
}

/// 放弃确认时显示的操作若已过期，就只刷新状态，不放弃后来开始的另一项操作。
pub(crate) fn abort_operation(
    tab: &mut Tab,
    expected_operation: git::Operation,
) -> Result<(), AbortError> {
    let repo = tab
        .open_repo()
        .map_err(|error| AbortError::Git(error.detail()))?;
    let operation = repo.operation();
    if operation == git::Operation::None {
        refresh_operation_state(tab, &repo, true, false, true).map_err(AbortError::Git)?;
        return Err(AbortError::NoOperation);
    }
    if operation != expected_operation || operation != tab.operation {
        refresh_operation_state(tab, &repo, true, false, true).map_err(AbortError::Git)?;
        return Err(AbortError::OperationChanged);
    }
    if let Err(error) = repo.abort_operation(operation) {
        let detail = error.detail();
        refresh_operation_state(tab, &repo, true, false, false).map_err(AbortError::Git)?;
        return Err(AbortError::Git(detail));
    }
    reload_after_history_change(tab).map_err(AbortError::Git)?;
    return Ok(());
}

fn refresh_operation_state(
    tab: &mut Tab,
    repo: &Repo,
    reload_history: bool,
    open_conflicts: bool,
    clear_drafts: bool,
) -> Result<(), String> {
    let operation = repo.operation();
    let conflicts = repo.conflicts().map_err(|error| error.detail())?;
    let previous_operation = tab.operation;
    let selected_path = tab
        .conflict_paths
        .get(tab.conflict_index)
        .filter(|selected| conflicts.contains(selected))
        .cloned();
    let previous_paths = tab.conflict_paths.clone();
    let previous_resolved = tab.resolved_conflicts.clone();
    let previous_drafts = tab.conflict_drafts.clone();
    if reload_history {
        reload_after_history_change(tab)?;
    } else {
        refresh_changes(tab);
    }
    tab.operation = operation;
    tab.conflicts = conflicts;
    if operation != previous_operation || operation == git::Operation::None {
        tab.conflict_paths.clear();
        tab.resolved_conflicts.clear();
        tab.conflict_drafts.clear();
    } else {
        tab.conflict_paths = previous_paths;
        for path in &tab.conflicts {
            tab.resolved_conflicts.remove(path);
            if !tab.conflict_paths.contains(path) {
                tab.conflict_paths.push(path.clone());
            }
        }
        tab.resolved_conflicts = previous_resolved
            .into_iter()
            .filter(|(path, _)| !tab.conflicts.contains(path))
            .collect();
        tab.conflict_drafts = if clear_drafts {
            Default::default()
        } else {
            previous_drafts
                .into_iter()
                .filter(|(path, _)| tab.conflicts.contains(path))
                .collect()
        };
        tab.conflict_index = selected_path
            .as_ref()
            .and_then(|selected| tab.conflict_paths.iter().position(|path| path == selected))
            .or_else(|| {
                tab.conflict_paths
                    .iter()
                    .position(|path| tab.conflicts.contains(path))
            })
            .unwrap_or(0);
    }
    sync_conflict_editor(tab, open_conflicts);
    return Ok(());
}

fn sync_conflict_editor(tab: &mut Tab, open_conflicts: bool) {
    let selected_path = tab
        .conflict_paths
        .get(tab.conflict_index)
        .filter(|selected| tab.conflicts.contains(selected))
        .cloned();
    let was_open = tab.conflict_open;
    if tab.operation == git::Operation::None {
        tab.conflict_open = false;
        tab.conflict_paths.clear();
        tab.resolved_conflicts.clear();
        tab.conflict_drafts.clear();
        tab.conflict_index = 0;
        tab.conflict_text.clear();
        tab.conflict_sides = None;
        tab.conflict_binary = false;
        return;
    }
    for path in &tab.conflicts {
        tab.resolved_conflicts.remove(path);
        if !tab.conflict_paths.contains(path) {
            tab.conflict_paths.push(path.clone());
        }
    }
    tab.conflict_index = selected_path
        .as_ref()
        .and_then(|selected| tab.conflict_paths.iter().position(|path| path == selected))
        .or_else(|| {
            tab.conflict_paths
                .iter()
                .position(|path| tab.conflicts.contains(path))
        })
        .unwrap_or(0);
    tab.conflict_open = was_open || open_conflicts;
    load_conflict_text(tab);
}

fn conflict_text(bytes: &[u8]) -> Option<String> {
    if bytes.contains(&0) {
        return None;
    }
    return String::from_utf8(bytes.to_vec()).ok();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::load_tab;
    use crate::test_support::TempRepo;
    use git::{LogFilter, Repo};

    #[test]
    fn conflict_view_takes_a_side_and_continues_the_merge() {
        let repo = TempRepo::new("conflict-view");
        let path = repo.path.display().to_string();
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&repo.path)
                .args(args)
                .status()
                .expect("git");
            return status.success();
        };
        run(&["checkout", "-q", "-b", "side"]);
        std::fs::write(repo.path.join("a.txt"), "side\n").expect("write");
        run(&["commit", "-q", "-am", "side"]);
        run(&["checkout", "-q", "main"]);
        std::fs::write(repo.path.join("a.txt"), "main\n").expect("write");
        run(&["commit", "-q", "-am", "main"]);
        assert!(!run(&["merge", "side"]), "合并应产生冲突");

        let mut tab = load_tab(&path, &LogFilter::default());
        open_conflict(&mut tab);
        assert!(tab.conflict_open);
        assert_eq!(tab.conflict_index, 0);
        assert!(
            domain::conflict::has_markers(tab.conflict_text.as_str()),
            "文件里应有冲突标记"
        );

        // 采用对方一侧 → 写回并标记解决。
        let merged = conflict_take_side(&mut tab, domain::conflict::Side::Theirs);
        assert!(!domain::conflict::has_markers(merged.as_str()));
        mark_conflict_resolved(&mut tab, merged.as_str()).expect("mark resolved");
        assert!(tab.conflicts.is_empty(), "标记后不应再有冲突");
        assert_eq!(tab.conflict_paths, vec!["a.txt".to_string()]);
        assert_eq!(conflict_result(&tab), "side\n");
        assert!(tab.resolved_conflicts.contains_key("a.txt"));
        assert!(tab.conflict_open, "全部解决后保留视图以继续当前操作");
        assert_eq!(
            std::fs::read_to_string(repo.path.join("a.txt")).expect("read"),
            "side\n"
        );

        // 继续合并。
        continue_operation(&mut tab).expect("continue");
        assert_eq!(tab.operation, git::Operation::None, "合并应已结束");
    }

    #[test]
    fn merge_conflict_is_reported_and_resolvable() {
        let repo = TempRepo::new("conflict");
        let path = repo.path.display().to_string();
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&repo.path)
                .args(args)
                .status()
                .expect("git");
            return status.success();
        };
        run(&["checkout", "-q", "-b", "side"]);
        std::fs::write(repo.path.join("a.txt"), "side\n").expect("write");
        run(&["commit", "-q", "-am", "side"]);
        run(&["checkout", "-q", "main"]);
        std::fs::write(repo.path.join("a.txt"), "main\n").expect("write");
        run(&["commit", "-q", "-am", "main"]);
        assert!(!run(&["merge", "side"]), "合并应产生冲突");

        let mut tab = load_tab(&path, &LogFilter::default());
        assert_eq!(tab.operation, git::Operation::Merge);
        assert_eq!(tab.conflicts, vec!["a.txt".to_string()]);

        resolve_conflict(&mut tab, "a.txt", true);
        assert!(tab.conflicts.is_empty(), "解决后不应再有冲突");
        assert_eq!(tab.conflict_paths, vec!["a.txt".to_string()]);
        assert_eq!(conflict_result(&tab), "main\n");
        assert_eq!(
            std::fs::read_to_string(repo.path.join("a.txt")).expect("read"),
            "main\n"
        );

        abort_operation(&mut tab, git::Operation::Merge).expect("abort merge");
        assert_eq!(tab.operation, git::Operation::None);
        assert!(tab.changes.is_empty());
    }

    #[test]
    fn rebase_continue_refreshes_the_next_conflict_round() {
        let repo = TempRepo::new("rebase-next-conflict");
        let path = repo.path.display().to_string();
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&repo.path)
                .args(args)
                .status()
                .expect("git");
            return status.success();
        };

        std::fs::write(repo.path.join("b.txt"), "base b\n").expect("write base b");
        assert!(run(&["add", "b.txt"]));
        assert!(run(&["commit", "-q", "-m", "base b"]));

        assert!(run(&["checkout", "-q", "-b", "feature"]));
        std::fs::write(repo.path.join("a.txt"), "feature a\n").expect("write feature a");
        assert!(run(&["commit", "-q", "-am", "feature a"]));
        std::fs::write(repo.path.join("b.txt"), "feature b\n").expect("write feature b");
        assert!(run(&["commit", "-q", "-am", "feature b"]));

        assert!(run(&["checkout", "-q", "main"]));
        std::fs::write(repo.path.join("a.txt"), "main a\n").expect("write main a");
        std::fs::write(repo.path.join("b.txt"), "main b\n").expect("write main b");
        assert!(run(&["commit", "-q", "-am", "main edits"]));
        assert!(run(&["checkout", "-q", "feature"]));
        assert!(
            Repo::open(&path)
                .expect("open repo")
                .rebase_onto("main")
                .is_err(),
            "首轮 rebase 应在 a.txt 冲突"
        );

        let mut tab = load_tab(&path, &LogFilter::default());
        assert_eq!(tab.operation, git::Operation::Rebase);
        assert_eq!(tab.conflicts, vec!["a.txt".to_string()]);
        open_conflict(&mut tab);
        Repo::open(&path)
            .expect("open repo")
            .resolve_with("a.txt", false)
            .expect("resolve first commit from feature");

        assert!(matches!(
            continue_operation(&mut tab),
            Err(ContinueError::UnresolvedConflicts)
        ));
        assert_eq!(tab.operation, git::Operation::Rebase);
        assert_eq!(tab.conflicts, vec!["b.txt".to_string()]);
        assert!(tab.conflict_open, "下一轮冲突应留在冲突视图中");
        assert_eq!(
            tab.conflict_paths
                .get(tab.conflict_index)
                .map(String::as_str),
            Some("b.txt")
        );
        assert!(tab.conflict_text.contains("<<<<<<<"));

        abort_operation(&mut tab, git::Operation::Rebase).expect("abort remaining rebase");
        assert_eq!(tab.operation, git::Operation::None);
        assert!(tab.conflicts.is_empty());
    }

    #[test]
    fn continue_and_abort_refresh_when_the_repository_operation_ended() {
        let repo = TempRepo::new("stale-operation");
        let path = repo.path.display().to_string();
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&repo.path)
                .args(args)
                .status()
                .expect("git");
            return status.success();
        };
        assert!(run(&["checkout", "-q", "-b", "side"]));
        std::fs::write(repo.path.join("a.txt"), "side\n").expect("write side");
        assert!(run(&["commit", "-q", "-am", "side"]));
        assert!(run(&["checkout", "-q", "main"]));
        std::fs::write(repo.path.join("a.txt"), "main\n").expect("write main");
        assert!(run(&["commit", "-q", "-am", "main"]));
        assert!(
            Repo::open(&path).expect("open repo").merge("side").is_err(),
            "merge 应产生冲突"
        );

        let mut tab = load_tab(&path, &LogFilter::default());
        open_conflict(&mut tab);
        Repo::open(&path)
            .expect("open repo")
            .abort_operation(git::Operation::Merge)
            .expect("external abort");

        assert!(matches!(
            continue_operation(&mut tab),
            Err(ContinueError::OperationChanged)
        ));
        assert_eq!(tab.operation, git::Operation::None);
        assert!(!tab.conflict_open);
        assert!(tab.conflict_text.is_empty());

        tab.operation = git::Operation::Merge;
        assert!(matches!(
            abort_operation(&mut tab, git::Operation::Merge),
            Err(AbortError::NoOperation)
        ));
        assert_eq!(tab.operation, git::Operation::None);
        assert!(tab.changes.is_empty());
    }
}
