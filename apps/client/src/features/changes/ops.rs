//! 变更页的操作：装载/刷新变更、单文件 diff、按 hunk 暂存与清单管理。

use crate::app::session::{ChangeEntry, Tab};
use domain::{GitError, StatusEntry};
use git::Repo;

/// 读取状态与行数统计；刷新路径用 `Result` 保留上次快照，避免查询失败伪装成干净仓库。
pub(crate) fn load_changes_checked(
    repo: &Repo,
) -> Result<(Vec<StatusEntry>, Vec<ChangeEntry>), GitError> {
    let status = repo.status()?;
    let entries = load_changes_for_status(repo, &status)?;
    return Ok((status, entries));
}

fn load_changes_for_status(
    repo: &Repo,
    status: &[StatusEntry],
) -> Result<Vec<ChangeEntry>, GitError> {
    let stats = repo.numstat_worktree_from_head()?;
    let mut entries: Vec<ChangeEntry> = Vec::new();
    for entry in status {
        let staged = entry.is_staged();
        let untracked = entry.index == '?' && entry.worktree == '?';
        let stat = stats.iter().find(|stat| stat.path == entry.path);
        entries.push(ChangeEntry {
            path: entry.path.clone(),
            index: entry.index,
            worktree: entry.worktree,
            staged,
            untracked,
            additions: stat.map(|stat| stat.additions).unwrap_or(0),
            deletions: stat.map(|stat| stat.deletions).unwrap_or(0),
            changelist: None,
        });
    }
    // 暂存的排在前面，组内按路径排序，与设计板的 Staged / Unstaged 两段一致。
    entries.sort_by(|left, right| {
        right
            .staged
            .cmp(&left.staged)
            .then_with(|| left.path.cmp(&right.path))
    });
    return Ok(entries);
}

/// 刷新变更列表（暂存、取消暂存、丢弃之后调用），保留选中项；无选择时不自动打开文件。
pub(crate) fn refresh_changes(tab: &mut Tab) {
    return refresh_changes_for_path(tab, None);
}

/// 暂存指定文件后刷新清单；只在该文件就是当前 Diff 时重读补丁。
fn refresh_changes_for_path(tab: &mut Tab, changed_path: Option<&str>) {
    let repo = match tab.open_repo() {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    let previous = tab
        .selected_change
        .and_then(|index| tab.changes.get(index))
        .map(|entry| entry.path.clone());
    let Ok((status, changes)) = load_changes_checked(&repo) else {
        return;
    };
    tab.changes = changes;
    apply_changelist_assignments(tab);
    tab.status = status;
    tab.operation = repo.operation();
    tab.conflicts = repo.conflicts().unwrap_or_default();
    tab.remote_urls = repo.remote_urls().unwrap_or_default();
    tab.selected_change =
        previous.and_then(|path| tab.changes.iter().position(|entry| entry.path == path));
    if let Some(index) = tab.selected_change {
        if changed_path.is_none_or(|path| tab.changes[index].path == path) {
            load_change_diff(tab, index);
        }
    } else {
        tab.change_diff = domain::diff::Patch::default();
        tab.change_hunk_staged.clear();
    }
    tab.error = None;
}

/// 暂存区操作不会改变工作区补丁与行数，只刷新 Git 状态和当前 Diff 的暂存标记。
fn refresh_staging_state(tab: &mut Tab, ignore_whitespace: bool) -> Result<(), GitError> {
    let repo = tab.open_repo()?;
    let status = repo.status()?;
    let has_new_paths = status
        .iter()
        .any(|entry| !tab.changes.iter().any(|change| change.path == entry.path));
    let mut changes = if has_new_paths {
        load_changes_for_status(&repo, &status)?
    } else {
        tab.changes.clone()
    };
    for change in &mut changes {
        if let Some(entry) = status.iter().find(|entry| entry.path == change.path) {
            change.index = entry.index;
            change.worktree = entry.worktree;
            change.staged = entry.is_staged();
            change.untracked = entry.index == '?' && entry.worktree == '?';
        }
    }
    changes.retain(|change| status.iter().any(|entry| entry.path == change.path));
    changes.sort_by(|left, right| {
        right
            .staged
            .cmp(&left.staged)
            .then_with(|| left.path.cmp(&right.path))
    });

    let selected_path = tab
        .selected_change
        .and_then(|index| tab.changes.get(index))
        .map(|entry| entry.path.clone());
    tab.status = status;
    tab.changes = changes;
    apply_changelist_assignments(tab);
    tab.selected_change = selected_path
        .as_deref()
        .and_then(|path| tab.changes.iter().position(|entry| entry.path == path));

    if let Some(index) = tab.selected_change {
        let entry = &tab.changes[index];
        let hunks = if entry.untracked || !entry.staged {
            hunk_staging_state(tab.change_diff.raw.as_str(), "")
        } else if !entry.has_unstaged_changes() {
            // 文件已完整暂存时，工作区补丁的每个变更都已进入 index；无需再运行一次 git diff。
            hunk_staging_state(tab.change_diff.raw.as_str(), tab.change_diff.raw.as_str())
        } else {
            let staged_patch = repo.staged_file_patch_from_head_with_whitespace(
                entry.path.as_str(),
                ignore_whitespace,
            )?;
            hunk_staging_state(tab.change_diff.raw.as_str(), &staged_patch)
        };
        tab.change_hunk_staged = hunks;
    } else {
        tab.change_diff = domain::diff::Patch::default();
        tab.change_hunk_staged.clear();
    }
    tab.error = None;
    return Ok(());
}

/// 把持久化清单中的路径归属恢复到当前工作区变更行。
pub(crate) fn apply_changelist_assignments(tab: &mut Tab) {
    for entry in &mut tab.changes {
        entry.changelist = tab
            .changelists
            .iter()
            .find(|list| list.paths.iter().any(|path| path == &entry.path))
            .map(|list| list.name.clone());
    }
}

/// 选中一条变更并读取它的 diff。
pub(crate) fn load_change_diff(tab: &mut Tab, index: usize) {
    return load_change_diff_with_whitespace(tab, index, false);
}

pub(crate) fn load_change_diff_with_whitespace(
    tab: &mut Tab,
    index: usize,
    ignore_whitespace: bool,
) {
    let Some(entry) = tab.changes.get(index).cloned() else {
        return;
    };
    let repo = match tab.open_repo() {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    tab.selected_change = Some(index);
    match read_change_diff(&repo, &entry, ignore_whitespace) {
        Ok((patch, staged_hunks, staged_patch)) => {
            tab.change_diff = patch;
            tab.change_hunk_staged = staged_hunks;
            if !entry.untracked {
                // 选择或重读 Diff 时若发现 index 中的 staged 行已不在当前工作区补丁里，
                // 立即把该 index hunk 反向应用，并同步文件树状态。
                match unstage_hunks_missing_from_worktree(
                    &repo,
                    tab.change_diff.raw.as_str(),
                    staged_patch.as_str(),
                ) {
                    Ok(true) => {
                        if let Err(error) = refresh_staging_state(tab, ignore_whitespace) {
                            tab.error = Some(error.detail());
                        }
                    }
                    Ok(false) => {}
                    Err(error) => tab.error = Some(error.detail()),
                }
            }
        }
        Err(error) => {
            tab.change_diff = domain::diff::Patch::default();
            tab.change_hunk_staged.clear();
            tab.error = Some(error.detail());
        }
    }
}

/// 读取 HEAD 到工作区的完整 diff，并标记每个显示 hunk 是否含暂存改动。
pub(crate) fn read_change_diff(
    repo: &Repo,
    entry: &ChangeEntry,
    ignore_whitespace: bool,
) -> Result<(domain::diff::Patch, Vec<bool>, String), GitError> {
    let patch = if entry.untracked {
        repo.untracked_file_diff_with_whitespace(entry.path.as_str(), ignore_whitespace)?
    } else {
        repo.file_diff_from_head_with_whitespace(entry.path.as_str(), ignore_whitespace)?
    };
    let staged_patch = if entry.untracked {
        String::new()
    } else {
        repo.staged_file_patch_from_head_with_whitespace(entry.path.as_str(), ignore_whitespace)?
    };
    let staged_hunks = hunk_staging_state(patch.raw.as_str(), &staged_patch);
    return Ok((patch, staged_hunks, staged_patch));
}

/// 以变更类型、所在侧行号和正文匹配暂存补丁，忽略 Git 对上下文边界的重分组。
fn hunk_staging_state(worktree_patch: &str, staged_patch: &str) -> Vec<bool> {
    let worktree = domain::stage::Sections::parse(worktree_patch);
    let staged = domain::stage::Sections::parse(staged_patch);
    let staged_marks = staged
        .files
        .first()
        .map(|file| {
            file.hunks
                .iter()
                .flat_map(hunk_change_marks)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let Some(file) = worktree.files.first() else {
        return Vec::new();
    };
    let mut hunk_states = Vec::with_capacity(file.hunks.len());
    for hunk in &file.hunks {
        let marks = hunk_change_line_marks(hunk);
        // hunk 复选框只有二态：index 中对应的每个变更都仍出现在工作区补丁里才勾选。
        // 若 staged 内容被后续编辑替换，刷新路径会把这段 index 内容取消暂存。
        let changes = marks.iter().flatten().collect::<Vec<_>>();
        let staged_changes = staged_marks
            .iter()
            .filter(|(staged_kind, staged_line, _)| {
                changes
                    .iter()
                    .any(|(kind, line, _)| kind == staged_kind && line == staged_line)
            })
            .collect::<Vec<_>>();
        let mut available_changes = changes.clone();
        let all_staged_changes_match = !staged_changes.is_empty()
            && staged_changes.iter().all(|staged| {
                let Some(index) = available_changes.iter().position(|change| change == staged)
                else {
                    return false;
                };
                available_changes.remove(index);
                return true;
            });
        hunk_states.push(all_staged_changes_match);
    }
    return hunk_states;
}

/// 刷新时自动取消暂存已从工作区 hunk 消失或被改写的 index 片段。
///
/// 只处理 index 中已暂存、但在 HEAD 到工作区的当前补丁里已不存在的变更行。
pub(crate) fn unstage_hunks_missing_from_worktree(
    repo: &Repo,
    current_patch: &str,
    staged_patch: &str,
) -> Result<bool, GitError> {
    let current = domain::stage::Sections::parse(current_patch);
    let current_marks = current
        .files
        .first()
        .map(|file| {
            file.hunks
                .iter()
                .flat_map(hunk_change_marks)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let staged = domain::stage::Sections::parse(staged_patch);
    let Some(staged_file) = staged.files.first() else {
        return Ok(false);
    };

    let hunks_to_unstage = staged_file
        .hunks
        .iter()
        .enumerate()
        .filter(|(_, hunk)| {
            let mut available_marks = current_marks.clone();
            return hunk_change_marks(hunk).iter().any(|mark| {
                let Some(index) = available_marks.iter().position(|current| current == mark) else {
                    return true;
                };
                available_marks.remove(index);
                return false;
            });
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let Some(patch) = hunks_patch(staged_file, &hunks_to_unstage) else {
        return Ok(false);
    };
    repo.apply_cached(patch.as_str(), true)?;
    return Ok(true);
}

/// 变更类型、所在侧行号与正文；新增行用新侧行号，删除行用旧侧行号。
fn hunk_change_marks(hunk: &domain::stage::Hunk) -> Vec<(char, u32, String)> {
    return hunk_change_line_marks(hunk).into_iter().flatten().collect();
}

fn hunk_change_line_marks(hunk: &domain::stage::Hunk) -> Vec<Option<(char, u32, String)>> {
    let mut old_line = hunk.old_start;
    let mut new_line = hunk.new_start;
    let mut marks = Vec::new();
    for raw in &hunk.lines {
        match domain::stage::classify(raw) {
            Some(domain::stage::LineKind::Add) => {
                marks.push(Some(('+', new_line, raw[1..].to_string())));
                new_line += 1;
            }
            Some(domain::stage::LineKind::Remove) => {
                marks.push(Some(('-', old_line, raw[1..].to_string())));
                old_line += 1;
            }
            Some(domain::stage::LineKind::Context) => {
                marks.push(None);
                old_line += 1;
                new_line += 1;
            }
            None => marks.push(None),
        }
    }
    return marks;
}

/// 把一个变更文件放进某个清单（`None` 放回默认清单）。
pub(crate) fn move_to_changelist(tab: &mut Tab, path: &str, list: Option<&str>) {
    let store_dir = store::default_dir(crate::brand::STORAGE_DIR);
    let repo = tab.path.clone();
    let mut lists = store::load_changelists(&store_dir, repo.as_str());
    // 先从所有清单里移除，再按需要加入目标清单。
    for existing in lists.iter_mut() {
        existing.paths.retain(|candidate| candidate != path);
    }
    if let Some(name) = list {
        match lists.iter_mut().find(|existing| existing.name == name) {
            Some(existing) => existing.paths.push(path.to_string()),
            None => lists.push(store::Changelist {
                name: name.to_string(),
                paths: vec![path.to_string()],
            }),
        }
    }
    store::save_changelists(&store_dir, repo.as_str(), &lists);
    tab.changelists = lists;
    // 变更行上的清单归属跟着更新。
    for entry in tab.changes.iter_mut() {
        if entry.path == path {
            entry.changelist = list.map(|name| name.to_string());
        }
    }
}

pub(crate) fn delete_changelist(tab: &mut Tab, name: &str) {
    let store_dir = store::default_dir(crate::brand::STORAGE_DIR);
    let repo = tab.path.clone();
    let mut lists = store::load_changelists(&store_dir, repo.as_str());
    lists.retain(|list| list.name != name);
    for entry in tab.changes.iter_mut() {
        if entry.changelist.as_deref() == Some(name) {
            entry.changelist = None;
        }
    }
    if tab.active_changelist.as_deref() == Some(name) {
        tab.active_changelist = None;
    }
    store::save_changelists(&store_dir, repo.as_str(), &lists);
    tab.changelists = lists;
}

/// 重命名清单并同步更新路径归属与当前默认清单。
pub(crate) fn rename_changelist(tab: &mut Tab, old_name: &str, new_name: &str) -> bool {
    let new_name = new_name.trim();
    if new_name.is_empty()
        || (old_name != new_name && tab.changelists.iter().any(|list| list.name == new_name))
    {
        return false;
    }
    if old_name == new_name {
        return true;
    }

    let Some(list) = tab
        .changelists
        .iter_mut()
        .find(|list| list.name == old_name)
    else {
        return false;
    };
    list.name = new_name.to_string();
    for entry in &mut tab.changes {
        if entry.changelist.as_deref() == Some(old_name) {
            entry.changelist = Some(new_name.to_string());
        }
    }
    if tab.active_changelist.as_deref() == Some(old_name) {
        tab.active_changelist = Some(new_name.to_string());
    }

    let store_dir = store::default_dir(crate::brand::STORAGE_DIR);
    store::save_changelists(&store_dir, tab.path.as_str(), &tab.changelists);
    return true;
}

/// 新建清单（空清单也保留，便于先建后移）。
pub(crate) fn create_changelist(tab: &mut Tab, name: &str) {
    let store_dir = store::default_dir(crate::brand::STORAGE_DIR);
    let repo = tab.path.clone();
    let mut lists = store::load_changelists(&store_dir, repo.as_str());
    if lists.iter().any(|existing| existing.name == name) {
        return;
    }
    lists.push(store::Changelist {
        name: name.to_string(),
        paths: Vec::new(),
    });
    store::save_changelists(&store_dir, repo.as_str(), &lists);
    tab.changelists = lists;
}

/// 切换当前清单。
pub(crate) fn set_active_changelist(tab: &mut Tab, name: Option<String>) {
    tab.active_changelist = name;
}

/// 暂存或取消暂存某个清单里的全部文件。
pub(crate) fn stage_changelist(
    tab: &mut Tab,
    name: Option<&str>,
    stage: bool,
    ignore_whitespace: bool,
) {
    let paths = tab
        .changes
        .iter()
        .filter(|entry| entry.changelist.as_deref() == name)
        .filter(|entry| is_stageable(entry, stage))
        .map(|entry| entry.path.clone())
        .collect::<Vec<_>>();
    return stage_paths(tab, paths, stage, ignore_whitespace);
}

/// 暂存或取消暂存当前清单下某个目录的全部文件。
pub(crate) fn stage_directory(
    tab: &mut Tab,
    name: Option<&str>,
    directory: &str,
    stage: bool,
    ignore_whitespace: bool,
) {
    let prefix = format!("{directory}/");
    let paths = tab
        .changes
        .iter()
        .filter(|entry| entry.changelist.as_deref() == name)
        .filter(|entry| entry.path.starts_with(&prefix))
        .filter(|entry| is_stageable(entry, stage))
        .map(|entry| entry.path.clone())
        .collect::<Vec<_>>();
    return stage_paths(tab, paths, stage, ignore_whitespace);
}

fn is_stageable(entry: &ChangeEntry, stage: bool) -> bool {
    if stage {
        return entry.worktree != ' ' || entry.untracked;
    }
    return entry.index != ' ' && entry.index != '?';
}

fn stage_paths(tab: &mut Tab, paths: Vec<String>, stage: bool, ignore_whitespace: bool) {
    if paths.is_empty() {
        return;
    }
    let repo = match tab.open_repo() {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    let result = if stage {
        repo.stage(&paths)
    } else {
        repo.unstage(&paths)
    };
    match result {
        Ok(()) => {
            if let Err(error) = refresh_staging_state(tab, ignore_whitespace) {
                tab.error = Some(error.detail());
            }
        }
        Err(error) => tab.error = Some(error.detail()),
    }
}

/// 按 hunk 暂存/取消暂存当前选中的变更；`reverse` 为真表示从暂存区取回。
/// 补丁来源是 `tab.change_diff`（当前文件相对暂存区 diff 的解析结果，原文在 `raw`）。
pub(crate) fn apply_piece(tab: &mut Tab, hunk_index: usize, reverse: bool) -> Result<(), String> {
    return apply_piece_with_whitespace(tab, hunk_index, reverse, false);
}

pub(crate) fn apply_piece_with_whitespace(
    tab: &mut Tab,
    hunk_index: usize,
    reverse: bool,
    ignore_whitespace: bool,
) -> Result<(), String> {
    let sections = domain::stage::Sections::parse(tab.change_diff.raw.as_str());
    let patch = sections
        .hunk_patch(0, hunk_index)
        .ok_or_else(|| "这一段没有可暂存的内容".to_string())?;
    let target_hunk = sections
        .files
        .first()
        .and_then(|file| file.hunks.get(hunk_index))
        .ok_or_else(|| "这一段没有可暂存的内容".to_string())?;
    let path = tab
        .selected_change
        .and_then(|index| tab.changes.get(index))
        .map(|entry| entry.path.clone())
        .ok_or_else(|| "没有选中的变更文件".to_string())?;
    let repo = tab.open_repo().map_err(|error| error.detail())?;
    if let Err(error) = repo.apply_cached(patch.as_str(), reverse) {
        let changed = unstage_overlapping_hunks(&repo, path.as_str(), target_hunk)
            .map_err(|fallback| fallback.detail())?;
        if !changed {
            return Err(error.detail());
        }
    }
    refresh_staging_state(tab, ignore_whitespace).map_err(|error| error.detail())?;
    return Ok(());
}

/// 丢弃显示的 HEAD → 工作区 hunk，并移除暂存区内对应片段。
pub(crate) fn discard_hunk(
    tab: &mut Tab,
    path: &str,
    patch: &str,
    ignore_whitespace: bool,
) -> Result<(), String> {
    let sections = domain::stage::Sections::parse(patch);
    let target = sections
        .files
        .first()
        .and_then(|file| file.hunks.first())
        .ok_or_else(|| "这一段没有可丢弃的内容".to_string())?;
    let worktree_patch = sections
        .hunk_patch(0, 0)
        .ok_or_else(|| "这一段没有可丢弃的内容".to_string())?;
    let repo = tab.open_repo().map_err(|error| error.detail())?;
    let status = repo.status().map_err(|error| error.detail())?;
    let Some(entry) = status.iter().find(|entry| entry.path == path) else {
        return Err("文件已没有待丢弃的改动".to_string());
    };
    let staged_patch = if entry.index == '?' {
        String::new()
    } else {
        repo.staged_file_patch_from_head_with_whitespace(path, ignore_whitespace)
            .map_err(|error| error.detail())?
    };
    let staged_patch = staged_patch_for_hunk(target, staged_patch.as_str())?;

    repo.apply_worktree(worktree_patch.as_str(), true)
        .map_err(|error| error.detail())?;
    if let Some(staged_patch) = staged_patch {
        if let Err(error) = repo.apply_cached(staged_patch.as_str(), true) {
            let rollback = repo.apply_worktree(worktree_patch.as_str(), false);
            return Err(match rollback {
                Ok(()) => error.detail(),
                Err(rollback_error) => format!(
                    "{}；恢复工作区失败：{}",
                    error.detail(),
                    rollback_error.detail()
                ),
            });
        }
    }
    refresh_changes(tab);
    return Ok(());
}

fn staged_patch_for_hunk(
    target: &domain::stage::Hunk,
    staged_patch: &str,
) -> Result<Option<String>, String> {
    let staged = domain::stage::Sections::parse(staged_patch);
    let Some(file) = staged.files.first() else {
        return Ok(None);
    };
    let target_marks = hunk_change_marks(target);
    if target_marks.is_empty() {
        return Err("这一段没有可丢弃的内容".to_string());
    }
    let overlapping = file
        .hunks
        .iter()
        .enumerate()
        .filter(|(_, hunk)| {
            let marks = hunk_change_marks(hunk);
            target_marks
                .iter()
                .any(|target| marks.iter().any(|mark| mark == target))
        })
        .map(|(index, hunk)| (index, hunk))
        .collect::<Vec<_>>();
    for (_, hunk) in &overlapping {
        if hunk_change_marks(hunk)
            .iter()
            .any(|mark| !target_marks.contains(mark))
        {
            return Err("这一段与其他暂存改动交叠，无法安全单独丢弃".to_string());
        }
    }
    let indices = overlapping
        .into_iter()
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    return Ok(hunks_patch(file, &indices));
}

/// 补丁应用冲突时，只把 index 中与显示 hunk 重叠的 staged 片段反向应用。
fn unstage_overlapping_hunks(
    repo: &Repo,
    path: &str,
    target: &domain::stage::Hunk,
) -> Result<bool, GitError> {
    let staged = repo.staged_file_patch_from_head_with_whitespace(path, false)?;
    let sections = domain::stage::Sections::parse(staged.as_str());
    let Some(file) = sections.files.first() else {
        return Ok(false);
    };
    let target_marks = hunk_change_marks(target);
    let hunk_indices = file
        .hunks
        .iter()
        .enumerate()
        .filter(|(_, hunk)| {
            let staged_marks = hunk_change_marks(hunk);
            target_marks.iter().any(|(kind, line, _)| {
                staged_marks
                    .iter()
                    .any(|(staged_kind, staged_line, _)| kind == staged_kind && line == staged_line)
            })
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let Some(patch) = hunks_patch(file, &hunk_indices) else {
        return Ok(false);
    };
    repo.apply_cached(patch.as_str(), true)?;
    return Ok(true);
}

fn hunks_patch(file: &domain::stage::FileSection, indices: &[usize]) -> Option<String> {
    if indices.is_empty() {
        return None;
    }
    let mut lines = file.header.clone();
    for index in indices {
        let hunk = file.hunks.get(*index)?;
        lines.push(hunk.header.clone());
        lines.extend(hunk.lines.iter().cloned());
    }
    let mut patch = lines.join("\n");
    patch.push('\n');
    return Some(patch);
}

/// 暂存或取消暂存一条变更。
pub(crate) fn toggle_stage(tab: &mut Tab, index: usize, ignore_whitespace: bool) {
    let Some(entry) = tab.changes.get(index).cloned() else {
        return;
    };
    return set_stage_path(
        tab,
        entry.path.as_str(),
        !entry.is_fully_staged(),
        ignore_whitespace,
    );
}

/// 将文件切换到明确的全暂存/全未暂存状态。
pub(crate) fn set_stage_path(tab: &mut Tab, path: &str, stage: bool, ignore_whitespace: bool) {
    let Some(entry) = tab.changes.iter().find(|entry| entry.path == path).cloned() else {
        return;
    };
    if (stage && !entry.has_unstaged_changes()) || (!stage && !entry.staged) {
        return;
    }
    let repo = match tab.open_repo() {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    let paths = vec![entry.path.clone()];
    let result = if stage {
        repo.stage(&paths)
    } else {
        repo.unstage(&paths)
    };
    match result {
        Ok(()) => {
            if let Err(error) = refresh_staging_state(tab, ignore_whitespace) {
                tab.error = Some(error.detail());
            }
        }
        Err(error) => tab.error = Some(error.detail()),
    }
}

/// 丢弃一条变更的工作区改动（不可撤销，调用方负责确认）。
pub(crate) fn discard_change(tab: &mut Tab, index: usize) {
    let Some(entry) = tab.changes.get(index).cloned() else {
        return;
    };
    let repo = match tab.open_repo() {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    match repo.discard(&[entry.path.clone()]) {
        Ok(()) => refresh_changes(tab),
        Err(error) => tab.error = Some(error.detail()),
    }
}

/// 删除新的未跟踪文件或暂存区新增文件。
pub(crate) fn delete_new_file(tab: &mut Tab, path: &str) {
    let repo = match tab.open_repo() {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    match repo.delete_new_file(path) {
        Ok(()) => refresh_changes(tab),
        Err(error) => tab.error = Some(error.detail()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::load_tab;
    use crate::test_support::TempRepo;
    use git::LogFilter;

    fn run_test_git(path: &std::path::Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(path)
            .args(args)
            .status()
            .expect("git");
        assert!(status.success(), "git {args:?} failed");
    }

    #[test]
    fn changes_list_splits_staged_and_unstaged() {
        let repo = TempRepo::new("changes");
        let path = repo.path.display().to_string();
        std::fs::write(repo.path.join("a.txt"), "changed\n").expect("write");
        std::fs::write(repo.path.join("new.txt"), "new\n").expect("write");
        let mut tab = load_tab(&path, &LogFilter::default());
        assert_eq!(tab.changes.len(), 2);

        // 暂存 a.txt 后它应出现在 Staged 段。
        let index = tab
            .changes
            .iter()
            .position(|entry| entry.path == "a.txt")
            .expect("a.txt");
        toggle_stage(&mut tab, index, false);
        let a = tab
            .changes
            .iter()
            .find(|entry| entry.path == "a.txt")
            .expect("a.txt");
        assert!(a.staged);
        let new = tab
            .changes
            .iter()
            .find(|entry| entry.path == "new.txt")
            .expect("new.txt");
        assert!(!new.staged);
        assert!(new.untracked, "未跟踪文件应标记为 untracked");

        // 取消暂存后回到未暂存段。
        let index = tab
            .changes
            .iter()
            .position(|entry| entry.path == "a.txt")
            .expect("a.txt");
        toggle_stage(&mut tab, index, false);
        assert!(
            !tab.changes
                .iter()
                .find(|entry| entry.path == "a.txt")
                .expect("a.txt")
                .staged
        );
    }

    #[test]
    fn apply_piece_stages_one_hunk_of_the_selected_file() {
        let repo = TempRepo::new("piece");
        let lines: String = (1..=30).map(|index| format!("line{index}\n")).collect();
        std::fs::write(repo.path.join("f.txt"), lines).expect("write");
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo.path)
            .args(["add", "-A"])
            .status()
            .expect("git");
        assert!(status.success());
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo.path)
            .args(["commit", "-q", "-m", "base"])
            .status()
            .expect("git");
        assert!(status.success());

        // 两处远端改动，形成两个 hunk。
        let mut changed: Vec<String> = (1..=30).map(|index| format!("line{index}")).collect();
        changed[1] = "line2-changed".to_string();
        changed[27] = "line28-changed".to_string();
        std::fs::write(repo.path.join("f.txt"), format!("{}\n", changed.join("\n")))
            .expect("write");

        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        let index = tab
            .changes
            .iter()
            .position(|entry| entry.path == "f.txt")
            .expect("f.txt");
        load_change_diff(&mut tab, index);
        let sections = domain::stage::Sections::parse(tab.change_diff.raw.as_str());
        assert_eq!(sections.files[0].hunks.len(), 2, "应有两个 hunk");

        // 只暂存第二个 hunk。
        apply_piece(&mut tab, 1, false).expect("stage hunk");
        assert!(
            tab.changes
                .iter()
                .any(|entry| entry.path == "f.txt" && entry.staged),
            "文件应有一部分进了暂存区"
        );
        // 工作区仍保留第一个 hunk 的改动。
        let diff = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo.path)
            .args(["diff", "--", "f.txt"])
            .output()
            .expect("git");
        let diff = String::from_utf8_lossy(&diff.stdout).to_string();
        assert!(diff.contains("line2-changed"), "第一个 hunk 还在工作区");
    }

    #[test]
    fn unstage_conflicting_hunk_updates_file_tree_partial_state() {
        let repo = TempRepo::new("staged-hunk-adjacent-edit");
        let lines: String = (1..=30).map(|index| format!("line{index}\n")).collect();
        std::fs::write(repo.path.join("f.txt"), lines).expect("write");
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo.path)
            .args(["add", "-A"])
            .status()
            .expect("git");
        assert!(status.success());
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo.path)
            .args(["commit", "-q", "-m", "base"])
            .status()
            .expect("git");
        assert!(status.success());

        let mut changed: Vec<String> = (1..=30).map(|index| format!("line{index}")).collect();
        changed[1] = "line2-staged".to_string();
        changed[13] = "line14-staged".to_string();
        changed[25] = "line26-staged".to_string();
        std::fs::write(repo.path.join("f.txt"), format!("{}\n", changed.join("\n")))
            .expect("write");

        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        let index = tab
            .changes
            .iter()
            .position(|entry| entry.path == "f.txt")
            .expect("f.txt");
        load_change_diff(&mut tab, index);
        assert_eq!(
            domain::stage::Sections::parse(tab.change_diff.raw.as_str()).files[0]
                .hunks
                .len(),
            3,
            "测试文件应有三个 hunk"
        );
        apply_piece(&mut tab, 0, false).expect("stage hunk");
        assert_eq!(tab.change_hunk_staged, vec![true, false, false]);

        // 保留暂存行，同时改动其下一行，模拟切回 Git 客户端时的工作区状态。
        changed[2] = "line3-worktree-edit".to_string();
        std::fs::write(repo.path.join("f.txt"), format!("{}\n", changed.join("\n")))
            .expect("write");
        refresh_changes(&mut tab);

        let entry = tab
            .changes
            .iter()
            .find(|entry| entry.path == "f.txt")
            .expect("f.txt after refresh");
        assert!(entry.is_partially_staged(), "文件树应显示半选状态");
        assert!(!entry.is_fully_staged(), "文件树不应显示全选状态");
        assert_eq!(
            tab.change_hunk_staged,
            vec![true, false, false],
            "仍在 index 中的 hunk 应保持勾选"
        );

        // 取消冲突 hunk 后，index 应同步移除它，文件树恢复未选。
        apply_piece(&mut tab, 0, true).expect("unstage conflicting hunk");
        let entry = tab
            .changes
            .iter()
            .find(|entry| entry.path == "f.txt")
            .expect("f.txt after unstage");
        assert!(!entry.staged, "唯一暂存的 hunk 移除后文件应未选");
        assert!(!entry.is_partially_staged());
        assert_eq!(tab.change_hunk_staged, vec![false, false, false]);

        // 全部三个 hunk 暂存后再取消其中一个，文件树应落到半选状态。
        for hunk in 0..3 {
            apply_piece(&mut tab, hunk, false).expect("stage remaining hunk");
        }
        let staged_contents = std::fs::read_to_string(repo.path.join("f.txt")).expect("read");
        let mut edited_again = staged_contents
            .lines()
            .map(str::to_string)
            .collect::<Vec<_>>();
        edited_again[2] = "line3-another-worktree-edit".to_string();
        std::fs::write(
            repo.path.join("f.txt"),
            format!("{}\n", edited_again.join("\n")),
        )
        .expect("write");
        refresh_changes(&mut tab);

        let entry = tab
            .changes
            .iter()
            .find(|entry| entry.path == "f.txt")
            .expect("f.txt after partial unstage");
        assert!(
            entry.is_partially_staged(),
            "剩余两个暂存 hunk 应令文件半选"
        );
        assert_eq!(
            tab.change_hunk_staged,
            vec![false, true, true],
            "refresh should automatically clear only the edited hunk"
        );
    }

    #[test]
    fn discard_change_restores_the_file() {
        let repo = TempRepo::new("discard");
        let path = repo.path.display().to_string();
        std::fs::write(repo.path.join("a.txt"), "scribble\n").expect("write");
        let mut tab = load_tab(&path, &LogFilter::default());
        let index = tab
            .changes
            .iter()
            .position(|entry| entry.path == "a.txt")
            .expect("a.txt");
        discard_change(&mut tab, index);
        assert_eq!(
            std::fs::read_to_string(repo.path.join("a.txt")).expect("read"),
            "one\n"
        );
        assert!(tab.changes.is_empty());
    }

    #[test]
    fn discard_hunk_removes_only_the_selected_worktree_and_staged_changes() {
        let repo = TempRepo::new("discard-hunk");
        let lines: String = (1..=36).map(|index| format!("line{index}\n")).collect();
        std::fs::write(repo.path.join("a.txt"), lines).expect("write base");
        run_test_git(&repo.path, &["add", "-A"]);
        run_test_git(&repo.path, &["commit", "-q", "-m", "base"]);

        let mut changed: Vec<String> = (1..=36).map(|index| format!("line{index}")).collect();
        changed[1] = "line2-staged".to_string();
        changed[13] = "line14-unstaged".to_string();
        changed[27] = "line28-kept".to_string();
        std::fs::write(repo.path.join("a.txt"), format!("{}\n", changed.join("\n")))
            .expect("write changes");

        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        let index = tab
            .changes
            .iter()
            .position(|entry| entry.path == "a.txt")
            .expect("a.txt");
        load_change_diff(&mut tab, index);
        assert_eq!(tab.change_hunk_staged, vec![false, false, false]);
        apply_piece(&mut tab, 0, false).expect("stage first hunk");

        let second_hunk = domain::stage::Sections::parse(tab.change_diff.raw.as_str())
            .hunk_patch(0, 1)
            .expect("second hunk");
        discard_hunk(&mut tab, "a.txt", second_hunk.as_str(), false)
            .expect("discard unstaged middle hunk");
        let content = std::fs::read_to_string(repo.path.join("a.txt")).expect("read worktree");
        assert!(content.contains("line2-staged"));
        assert!(content.contains("line28-kept"));
        assert!(content.contains("line14\n"));
        assert_eq!(tab.change_hunk_staged, vec![true, false]);

        let first_hunk = domain::stage::Sections::parse(tab.change_diff.raw.as_str())
            .hunk_patch(0, 0)
            .expect("first hunk");
        discard_hunk(&mut tab, "a.txt", first_hunk.as_str(), false)
            .expect("discard staged first hunk");
        let content = std::fs::read_to_string(repo.path.join("a.txt")).expect("read worktree");
        assert!(content.contains("line2\n"));
        assert!(content.contains("line28-kept"));
        assert!(!content.contains("line2-staged"));
        let staged = tab
            .open_repo()
            .expect("repo")
            .staged_file_patch_from_head_with_whitespace("a.txt", false)
            .expect("staged patch");
        assert!(!staged.contains("line2-staged"));
        assert_eq!(tab.change_hunk_staged, vec![false]);
    }

    #[test]
    fn delete_new_file_refreshes_the_changes_tree() {
        let repo = TempRepo::new("delete-new-file");
        let path = repo.path.display().to_string();
        std::fs::write(repo.path.join("new.txt"), "new\n").expect("write");
        let mut tab = load_tab(&path, &LogFilter::default());
        assert!(tab.changes.iter().any(|entry| entry.path == "new.txt"));

        delete_new_file(&mut tab, "new.txt");

        assert!(!repo.path.join("new.txt").exists());
        assert!(!tab.changes.iter().any(|entry| entry.path == "new.txt"));
    }

    #[test]
    fn changelists_group_changes_and_keep_storage() {
        // 用独立的 APP_HOME 隔离存储，避免污染真实的应用支持目录。
        let home = std::env::temp_dir().join(format!(
            "nsh-app-changelists-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&home).expect("home");
        // SAFETY：测试进程内串行使用该变量，只影响本测试的存储目录。
        unsafe { std::env::set_var("APP_HOME", &home) };

        let repo = TempRepo::new("changelists");
        std::fs::write(repo.path.join("a.txt"), "changed\n").expect("write");
        std::fs::write(repo.path.join("b.txt"), "new\n").expect("write");
        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        assert_eq!(tab.changes.len(), 2);

        create_changelist(&mut tab, "Commit composer");
        assert_eq!(tab.changelists.len(), 1);
        assert!(tab.changelists[0].paths.is_empty());

        move_to_changelist(&mut tab, "a.txt", Some("Commit composer"));
        let list = tab
            .changelists
            .iter()
            .find(|list| list.name == "Commit composer")
            .expect("list");
        assert_eq!(list.paths, vec!["a.txt".to_string()]);
        let entry = tab
            .changes
            .iter()
            .find(|entry| entry.path == "a.txt")
            .expect("a.txt");
        assert_eq!(entry.changelist.as_deref(), Some("Commit composer"));

        // 重新读取：清单从磁盘恢复。
        let reloaded = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        assert!(
            reloaded
                .changelists
                .iter()
                .any(|list| list.name == "Commit composer"
                    && list.paths == vec!["a.txt".to_string()])
        );
        assert_eq!(
            reloaded
                .changes
                .iter()
                .find(|entry| entry.path == "a.txt")
                .and_then(|entry| entry.changelist.as_deref()),
            Some("Commit composer")
        );

        stage_changelist(&mut tab, Some("Commit composer"), true, false);
        let staged = tab
            .changes
            .iter()
            .find(|entry| entry.path == "a.txt")
            .expect("a.txt after staging");
        assert!(staged.staged);
        assert_eq!(staged.changelist.as_deref(), Some("Commit composer"));
        stage_changelist(&mut tab, Some("Commit composer"), false, false);
        let unstaged = tab
            .changes
            .iter()
            .find(|entry| entry.path == "a.txt")
            .expect("a.txt after unstaging");
        assert!(!unstaged.staged);
        assert_eq!(unstaged.changelist.as_deref(), Some("Commit composer"));

        // 移回默认清单：清单保留为空，等待用户显式删除。
        move_to_changelist(&mut tab, "a.txt", None);
        assert!(
            tab.changelists
                .iter()
                .any(|list| list.name == "Commit composer")
        );
        assert!(tab.changelists[0].paths.is_empty());
        let reloaded_empty = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        assert!(
            reloaded_empty
                .changelists
                .iter()
                .any(|list| { list.name == "Commit composer" && list.paths.is_empty() })
        );

        move_to_changelist(&mut tab, "a.txt", Some("Commit composer"));
        create_changelist(&mut tab, "Other");
        set_active_changelist(&mut tab, Some("Commit composer".to_string()));
        assert_eq!(tab.active_changelist.as_deref(), Some("Commit composer"));

        assert!(rename_changelist(
            &mut tab,
            "Commit composer",
            "Reviewed files"
        ));
        assert_eq!(tab.active_changelist.as_deref(), Some("Reviewed files"));
        assert!(tab.changelists.iter().any(|list| {
            list.name == "Reviewed files" && list.paths == vec!["a.txt".to_string()]
        }));
        assert_eq!(
            tab.changes
                .iter()
                .find(|entry| entry.path == "a.txt")
                .and_then(|entry| entry.changelist.as_deref()),
            Some("Reviewed files")
        );
        let reloaded_renamed = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        assert!(reloaded_renamed.changelists.iter().any(|list| {
            list.name == "Reviewed files" && list.paths == vec!["a.txt".to_string()]
        }));
        assert_eq!(
            reloaded_renamed
                .changes
                .iter()
                .find(|entry| entry.path == "a.txt")
                .and_then(|entry| entry.changelist.as_deref()),
            Some("Reviewed files")
        );
        assert!(!rename_changelist(&mut tab, "Reviewed files", ""));
        assert!(!rename_changelist(&mut tab, "Reviewed files", "Other"));

        // 显式删除清单时，未暂存变更回到默认清单，当前选择也回到默认清单。
        move_to_changelist(&mut tab, "a.txt", Some("Reviewed files"));
        delete_changelist(&mut tab, "Reviewed files");
        delete_changelist(&mut tab, "Other");
        assert!(tab.changelists.is_empty());
        assert!(tab.active_changelist.is_none());
        assert!(
            tab.changes
                .iter()
                .find(|entry| entry.path == "a.txt")
                .is_some_and(|entry| entry.changelist.is_none())
        );

        unsafe { std::env::remove_var("APP_HOME") };
        let _ = std::fs::remove_dir_all(&home);
    }
}
