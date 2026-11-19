//! 合并与冲突的操作：冲突视图的浏览、按侧解决、标记已解决与继续/放弃操作。

use crate::app::session::{ConflictDraft, Tab, reload_after_history_change};
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
    Git(String),
}

/// 打开冲突视图，并载入第一个冲突文件的内容。
pub(crate) fn open_conflict(tab: &mut Tab) {
    tab.conflict_open = true;
    tab.conflict_index = 0;
    load_conflict_text(tab);
}

/// 切换到第 `index` 个冲突文件。
pub(crate) fn select_conflict(tab: &mut Tab, index: usize) {
    if index >= tab.conflicts.len() {
        return;
    }
    tab.conflict_index = index;
    load_conflict_text(tab);
}

/// 前一个/后一个冲突文件（到头就停住，不回绕，避免误操作）。
pub(crate) fn step_conflict(tab: &mut Tab, forward: bool) {
    if tab.conflicts.is_empty() {
        return;
    }
    let next = if forward {
        (tab.conflict_index + 1).min(tab.conflicts.len() - 1)
    } else {
        tab.conflict_index.saturating_sub(1)
    };
    select_conflict(tab, next);
}

fn load_conflict_text(tab: &mut Tab) {
    let Some(path) = tab.conflicts.get(tab.conflict_index).cloned() else {
        tab.conflict_text.clear();
        tab.conflict_sides = None;
        tab.conflict_binary = false;
        return;
    };
    let repo = match Repo::open(tab.path.as_str()) {
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

/// 当前文件的结果值来自未解决草稿，初次打开时才来自工作区文件。
pub(crate) fn conflict_result(tab: &Tab) -> String {
    let Some(path) = tab.conflicts.get(tab.conflict_index) else {
        return String::new();
    };
    return match tab.conflict_drafts.get(path) {
        Some(ConflictDraft::Text(value)) => value.clone(),
        Some(ConflictDraft::Binary(_) | ConflictDraft::Delete) => String::new(),
        None => tab.conflict_text.clone(),
    };
}

/// 立即把编辑器变化写入当前仓库标签的路径草稿。
pub(crate) fn update_conflict_result(tab: &mut Tab, value: String) {
    let Some(path) = tab.conflicts.get(tab.conflict_index).cloned() else {
        return;
    };
    tab.conflict_drafts.insert(path, ConflictDraft::Text(value));
}

/// 把当前冲突文件的某一侧整体写进结果。
pub(crate) fn conflict_take_side(tab: &mut Tab, side: domain::conflict::Side) -> String {
    let Some(path) = tab.conflicts.get(tab.conflict_index).cloned() else {
        return String::new();
    };
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
    let Some(path) = tab.conflicts.get(tab.conflict_index).cloned() else {
        return Ok(());
    };
    let repo =
        Repo::open(tab.path.as_str()).map_err(|error| MarkResolvedError::Git(error.detail()))?;
    match tab.conflict_drafts.get(&path) {
        Some(ConflictDraft::Binary(bytes)) => repo.resolve_conflict_bytes(path.as_str(), bytes),
        Some(ConflictDraft::Delete) => repo.resolve_conflict_delete(path.as_str()),
        Some(ConflictDraft::Text(_)) | None => {
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
    refresh_changes(tab);
    if tab.conflict_index >= tab.conflicts.len() {
        tab.conflict_index = tab.conflicts.len().saturating_sub(1);
    }
    load_conflict_text(tab);
    return Ok(());
}

/// 继续被冲突打断的操作；成功后刷新整个 tab。
pub(crate) fn continue_operation(tab: &mut Tab) -> Result<String, ContinueError> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| ContinueError::Git(error.detail()))?;
    tab.operation = repo.operation();
    tab.conflicts = repo
        .conflicts()
        .map_err(|error| ContinueError::Git(error.detail()))?;
    if !tab.conflicts.is_empty() {
        return Err(ContinueError::UnresolvedConflicts);
    }
    if !tab.conflict_drafts.is_empty() {
        return Err(ContinueError::PendingDrafts);
    }
    if tab.operation == git::Operation::None {
        return Err(ContinueError::NoOperation);
    }
    let summary = repo
        .continue_operation(tab.operation)
        .map_err(|error| ContinueError::Git(error.detail()))?;
    let branch = reload_after_history_change(tab).map_err(ContinueError::Git)?;
    return Ok(if summary.trim().is_empty() {
        branch
    } else {
        summary
    });
}

/// 用一侧内容解决冲突（`ours` 为真取当前分支）。
pub(crate) fn resolve_conflict(tab: &mut Tab, path: &str, ours: bool) {
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    match repo.resolve_with(path, ours) {
        Ok(()) => {
            tab.conflict_drafts.remove(path);
            refresh_changes(tab);
            if let Some(index) = tab.conflicts.iter().position(|item| item == path) {
                tab.conflict_index = index;
                load_conflict_text(tab);
            }
        }
        Err(error) => tab.error = Some(error.detail()),
    }
}

/// 放弃进行中的操作，回到干净工作区。
pub(crate) fn abort_operation(tab: &mut Tab) {
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    match repo.abort_operation(tab.operation) {
        Ok(()) => {
            tab.conflict_drafts.clear();
            tab.conflict_open = false;
            tab.conflict_index = 0;
            tab.conflict_text.clear();
            tab.conflict_sides = None;
            tab.conflict_binary = false;
            refresh_changes(tab);
        }
        Err(error) => tab.error = Some(error.detail()),
    }
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
    use git::LogFilter;

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
        assert_eq!(
            std::fs::read_to_string(repo.path.join("a.txt")).expect("read"),
            "main\n"
        );

        abort_operation(&mut tab);
        assert_eq!(tab.operation, git::Operation::None);
        assert!(tab.changes.is_empty());
    }
}
