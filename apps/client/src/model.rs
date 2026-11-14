//! 应用状态与数据装载：把 `git` crate 的读取结果整理成界面直接渲染的形状。
//!
//! 只读阶段：打开仓库后读取摘要、提交列表、引用集合、首个提交的补丁与文件统计。
//! 筛选条件、对比选择与单文件补丁都由这里维护，界面只读结果。
//!
//! Git 命令在主线程上执行——本地只读命令在毫秒级；加入写路径与同步后，这些调用
//! 会移到后台任务，接口保持不变。

use domain::{Commit, DiffStat, StatusEntry};
use git::{LogFilter, Refs, Repo};

/// 一个仓库标签页的全部界面状态。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tab {
    pub path: String,
    pub name: String,
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub commits: Vec<Commit>,
    pub refs: Refs,
    pub status: Vec<StatusEntry>,
    /// 历史里出现过的作者（按最近提交顺序）。
    pub authors: Vec<String>,
    /// 当前生效的筛选条件。
    pub filter: LogFilter,
    /// 选中的提交下标；空仓库时为 `None`。
    pub selected: Option<usize>,
    /// 选中的提交的补丁文本。
    pub patch: String,
    /// 选中的提交改动的文件统计。
    pub files: Vec<DiffStat>,
    /// 选中的文件下标（`files`），用于单文件 diff。
    pub selected_file: Option<usize>,
    /// 单文件补丁：(路径, 补丁文本)。
    pub file_patch: Option<(String, String)>,
    /// 多选对比的提交下标（升序无关，按点击顺序）。
    pub compare: Vec<usize>,
    /// 对比模式：开启后点击提交是加入/移出对比集合。
    pub comparing: bool,
    /// 工作区与暂存区的变更（设计板 10 的列表）。
    pub changes: Vec<ChangeEntry>,
    /// 选中的变更下标。
    pub selected_change: Option<usize>,
    /// 选中变更的 diff 文本。
    pub change_diff: String,
    /// 变更清单（本地存储，按仓库分组）。
    pub changelists: Vec<store::Changelist>,
    /// 当前清单名；`None` 表示默认清单。
    pub active_changelist: Option<String>,
    /// 进行中的多步操作（合并/变基/拣选/回滚）。
    pub operation: git::Operation,
    /// 冲突文件路径。
    pub conflicts: Vec<String>,
    /// 最近一次同步（fetch/pull/push）的结果；`None` 表示还没同步过。
    pub sync_message: Option<String>,
    /// 最近一次同步是否失败。
    pub sync_failed: bool,
    /// reflog（最近的在前）。
    pub reflog: Vec<git::ReflogEntry>,
    /// 选中提交的完整元信息（Info 页）。
    pub details: Option<git::CommitDetails>,
    /// 选中文件的 blame 行。
    pub blame: Vec<git::BlameLine>,
    /// 选中文件的历史。
    pub file_history: Vec<Commit>,
    /// 对比引用的起点（`None` 表示未开启 Compare refs）。
    pub compare_base: Option<String>,
    /// 对比引用的终点。
    pub compare_target: Option<String>,
    /// 提交说明的两段文本。
    pub summary: String,
    pub description: String,
    /// 最近一次提交尝试的钩子输出；`None` 表示还没提交过。
    pub hook_log: Option<String>,
    /// 最近一次提交是否成功。
    pub hook_ok: bool,
    /// 打开失败时的错误详情；成功时为 `None`。
    pub error: Option<String>,
}

/// 一条工作区变更。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChangeEntry {
    pub path: String,
    /// 暂存区状态字符（`M`/`A`/`D`/`R`/`?`）。
    pub index: char,
    /// 工作区状态字符。
    pub worktree: char,
    /// 是否已在暂存区。
    pub staged: bool,
    /// 是否为未跟踪文件。
    pub untracked: bool,
    pub additions: u32,
    pub deletions: u32,
    /// 所属变更清单；`None` 表示默认清单。
    pub changelist: Option<String>,
}

impl ChangeEntry {
    /// 设计稿里显示的状态字母。
    pub fn status_letter(&self) -> char {
        if self.untracked {
            return '?';
        }
        if self.staged && self.worktree != ' ' {
            return 'M';
        }
        if self.staged {
            return normalise_status(self.index);
        }
        return normalise_status(self.worktree);
    }

    /// 目录部分（`src/ui/CommitPanel.tsx` → `src/ui`），没有目录时为空串。
    pub fn directory(&self) -> &str {
        return match self.path.rsplit_once('/') {
            Some((directory, _)) => directory,
            None => "",
        };
    }
}

fn normalise_status(status: char) -> char {
    return match status {
        ' ' | '?' => '?',
        'M' | 'T' => 'M',
        'A' => 'A',
        'D' => 'D',
        'R' => 'R',
        'C' => 'C',
        'U' => 'U',
        _ => 'M',
    };
}

impl Tab {
    /// 当前选中的提交。
    pub fn selected_commit(&self) -> Option<&Commit> {
        return self.selected.and_then(|index| self.commits.get(index));
    }

    /// 名字前的短标识：仓库名。
    pub fn label(&self) -> String {
        return if self.name.is_empty() {
            self.path.clone()
        } else {
            self.name.clone()
        };
    }

    /// 对比区间的起止提交：`(from, to)`，按历史位置排序。
    pub fn compare_range(&self) -> Option<(usize, usize)> {
        if self.compare.len() < 2 {
            return None;
        }
        let mut indices = self.compare.clone();
        indices.sort_unstable();
        return Some((indices[0], indices[indices.len() - 1]));
    }
}

/// 打开 `path` 并按 `filter` 装载数据；失败时返回带 `error` 的空标签页。
pub fn load_tab(path: &str, filter: &LogFilter) -> Tab {
    let repo = match Repo::open(path) {
        Ok(repo) => repo,
        Err(error) => {
            return Tab {
                path: path.to_string(),
                name: last_segment(path),
                filter: filter.clone(),
                error: Some(error.detail()),
                ..Tab::default()
            };
        }
    };
    let summary = repo.summary().unwrap_or_default();
    let commits = repo.log_filtered(200, filter).unwrap_or_default();
    let refs = repo.refs().unwrap_or_default();
    let status = repo.status().unwrap_or_default();
    let authors = repo.authors().unwrap_or_default();
    let changes = load_changes(&repo);
    let operation = repo.operation();
    let reflog = repo.reflog(100).unwrap_or_default();
    let conflicts = repo.conflicts().unwrap_or_default();
    // 存储键统一用仓库根路径：入参可能是符号链接形式（macOS 的 /tmp → /private/tmp），
    // 与写入时用的 `repo.root()` 不一致会读不到清单。
    let changelists = store::load_changelists(
        &store::default_dir(crate::brand::STORAGE_DIR),
        repo.root().display().to_string().as_str(),
    );
    let mut tab = Tab {
        path: repo.root().display().to_string(),
        name: summary.name,
        branch: summary.branch,
        ahead: summary.ahead,
        behind: summary.behind,
        commits,
        refs,
        status,
        authors,
        changes,
        operation,
        conflicts,
        reflog,
        changelists,
        filter: filter.clone(),
        ..Tab::default()
    };
    if !tab.commits.is_empty() {
        select_commit(&mut tab, 0);
    }
    return tab;
}

/// 选中第 `index` 个提交，读取补丁与文件统计，并清掉单文件选择。
pub fn select_commit(tab: &mut Tab, index: usize) {
    if index >= tab.commits.len() {
        return;
    }
    let sha = tab.commits[index].sha.clone();
    tab.selected = Some(index);
    tab.selected_file = None;
    tab.file_patch = None;
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    tab.patch = repo.patch(&sha).unwrap_or_default();
    tab.files = repo.numstat(&sha).unwrap_or_default();
    tab.details = repo.commit_details(&sha).ok();
    tab.blame.clear();
    tab.file_history.clear();
    tab.error = None;
}

/// 选中某个文件并读取它在当前提交里的补丁。
pub fn select_file(tab: &mut Tab, index: usize) {
    let Some(commit_index) = tab.selected else {
        return;
    };
    let Some(file) = tab.files.get(index) else {
        return;
    };
    let Some(commit) = tab.commits.get(commit_index) else {
        return;
    };
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    match repo.patch_for_path(&commit.sha, file.path.as_str()) {
        Ok(patch) => {
            tab.selected_file = Some(index);
            tab.file_patch = Some((file.path.clone(), patch));
            // blame 与文件历史只读，失败时留空而不是报错（例如文件在新提交里不存在）。
            tab.blame = repo.blame(file.path.as_str()).unwrap_or_default();
            tab.file_history = repo
                .file_history(file.path.as_str(), 50)
                .unwrap_or_default();
        }
        Err(error) => tab.error = Some(error.detail()),
    }
}

/// 把第 `index` 个提交加入或移出对比集合。
pub fn toggle_compare(tab: &mut Tab, index: usize) {
    if let Some(position) = tab.compare.iter().position(|value| *value == index) {
        tab.compare.remove(position);
    } else {
        tab.compare.push(index);
        tab.compare.sort_unstable();
    }
}

/// 对比模式下要显示的补丁：区间 diff；不足两个提交时为 `None`。
pub fn compare_patch(tab: &Tab) -> Option<String> {
    let (from, to) = tab.compare_range()?;
    let from_sha = tab.commits.get(from)?.sha.clone();
    let to_sha = tab.commits.get(to)?.sha.clone();
    // 区间 diff 包含 `from` 之后的提交到 `to`，因此起点取 `from` 的父提交。
    let repo = Repo::open(tab.path.as_str()).ok()?;
    return repo.diff_between(&format!("{}^", from_sha), &to_sha).ok();
}

/// 读取工作区与暂存区的变更，并附上逐文件行数统计。
pub fn load_changes(repo: &Repo) -> Vec<ChangeEntry> {
    let status = repo.status().unwrap_or_default();
    let staged_stats = repo.numstat_section(true).unwrap_or_default();
    let worktree_stats = repo.numstat_section(false).unwrap_or_default();
    let mut entries: Vec<ChangeEntry> = Vec::new();
    for entry in status {
        let staged = entry.is_staged();
        let untracked = entry.index == '?' && entry.worktree == '?';
        let stats = if staged {
            staged_stats.iter().find(|stat| stat.path == entry.path)
        } else {
            worktree_stats.iter().find(|stat| stat.path == entry.path)
        };
        entries.push(ChangeEntry {
            path: entry.path.clone(),
            index: entry.index,
            worktree: entry.worktree,
            staged,
            untracked,
            additions: stats.map(|stat| stat.additions).unwrap_or(0),
            deletions: stats.map(|stat| stat.deletions).unwrap_or(0),
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
    return entries;
}

/// 刷新变更列表（暂存、取消暂存、丢弃之后调用），保留选中项。
pub fn refresh_changes(tab: &mut Tab) {
    let repo = match Repo::open(tab.path.as_str()) {
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
    tab.changes = load_changes(&repo);
    tab.status = repo.status().unwrap_or_default();
    tab.operation = repo.operation();
    tab.conflicts = repo.conflicts().unwrap_or_default();
    tab.selected_change = previous
        .and_then(|path| tab.changes.iter().position(|entry| entry.path == path))
        .or(if tab.changes.is_empty() { None } else { Some(0) });
    if let Some(index) = tab.selected_change {
        load_change_diff(tab, index);
    } else {
        tab.change_diff.clear();
    }
    tab.error = None;
}

/// 选中一条变更并读取它的 diff。
pub fn load_change_diff(tab: &mut Tab, index: usize) {
    let Some(entry) = tab.changes.get(index).cloned() else {
        return;
    };
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    tab.selected_change = Some(index);
    // 未跟踪文件没有 diff（git 不比较索引外的文件），留空由界面提示。
    tab.change_diff = if entry.untracked {
        String::new()
    } else {
        repo.file_diff(entry.path.as_str(), entry.staged)
            .unwrap_or_default()
    };
}

/// 切换分支并刷新标签页（历史、引用、变更都要跟着变）。
pub fn checkout_branch(tab: &mut Tab, branch: &str) {
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
pub fn delete_branch(tab: &mut Tab, branch: &str) {
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

/// 把一个变更文件放进某个清单（`None` 放回默认清单）。
pub fn move_to_changelist(tab: &mut Tab, path: &str, list: Option<&str>) {
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
    lists.retain(|existing| !existing.paths.is_empty());
    store::save_changelists(&store_dir, repo.as_str(), &lists);
    tab.changelists = lists;
    // 变更行上的清单归属跟着更新。
    for entry in tab.changes.iter_mut() {
        if entry.path == path {
            entry.changelist = list.map(|name| name.to_string());
        }
    }
}

/// 新建清单（空清单也保留，便于先建后移）。
pub fn create_changelist(tab: &mut Tab, name: &str) {
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
pub fn set_active_changelist(tab: &mut Tab, name: Option<String>) {
    tab.active_changelist = name;
}

/// 从引用新建并切换到本地分支。
pub fn checkout_new_branch(tab: &mut Tab, name: &str, from: &str) -> Result<(), String> {
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

/// 把某个引用合并进当前分支。
pub fn merge_reference(tab: &mut Tab, reference: &str) -> Result<String, String> {
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

/// 把当前分支重置到某个提交。
pub fn reset_to(tab: &mut Tab, sha: &str, mode: git::ResetMode) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.reset_to(sha, mode).map_err(|error| error.detail())?;
    refresh_changes(tab);
    let filter = tab.filter.clone();
    let refreshed = load_tab(tab.path.as_str(), &filter);
    tab.commits = refreshed.commits;
    tab.branch = refreshed.branch;
    tab.ahead = refreshed.ahead;
    tab.behind = refreshed.behind;
    return Ok(());
}

/// 新建分支；`from` 为空时基于当前 HEAD。
pub fn create_branch(tab: &mut Tab, name: &str, from: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    let from = if from.trim().is_empty() {
        None
    } else {
        Some(from.trim())
    };
    repo.create_branch(name, from).map_err(|error| error.detail())?;
    tab.refs = repo.refs().unwrap_or_default();
    return Ok(());
}

/// 新建标签。
pub fn create_tag(tab: &mut Tab, name: &str, from: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    let from = if from.trim().is_empty() {
        None
    } else {
        Some(from.trim())
    };
    repo.create_tag(name, from).map_err(|error| error.detail())?;
    tab.refs = repo.refs().unwrap_or_default();
    return Ok(());
}

/// 添加远程。
pub fn add_remote(tab: &mut Tab, name: &str, url: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.add_remote(name, url).map_err(|error| error.detail())?;
    tab.refs = repo.refs().unwrap_or_default();
    return Ok(());
}

/// 贮藏当前改动。
pub fn stash_changes(tab: &mut Tab, message: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.stash_push(message).map_err(|error| error.detail())?;
    tab.refs = repo.refs().unwrap_or_default();
    refresh_changes(tab);
    return Ok(());
}

/// 丢弃一条贮藏。
pub fn drop_stash(tab: &mut Tab, reference: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.stash_drop(reference).map_err(|error| error.detail())?;
    tab.refs = repo.refs().unwrap_or_default();
    return Ok(());
}

/// 删除标签。
pub fn drop_tag(tab: &mut Tab, name: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.delete_tag(name).map_err(|error| error.detail())?;
    tab.refs = repo.refs().unwrap_or_default();
    return Ok(());
}

/// 移除远程。
pub fn drop_remote(tab: &mut Tab, name: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.remove_remote(name).map_err(|error| error.detail())?;
    tab.refs = repo.refs().unwrap_or_default();
    return Ok(());
}

/// 设置对比引用的两端；任一端为空表示关闭对比。
pub fn set_compare_refs(tab: &mut Tab, base: Option<String>, target: Option<String>) {
    let (Some(base), Some(target)) = (base.clone(), target.clone()) else {
        // 任一端为空表示退出对比：把历史恢复到当前分支的完整列表。
        tab.compare_base = None;
        tab.compare_target = None;
        let filter = tab.filter.clone();
        let refreshed = load_tab(tab.path.as_str(), &filter);
        tab.commits = refreshed.commits;
        tab.selected = refreshed.selected;
        tab.patch = refreshed.patch;
        tab.files = refreshed.files;
        tab.details = refreshed.details;
        tab.blame = refreshed.blame;
        tab.file_history = refreshed.file_history;
        return;
    };
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    match repo.commits_between(base.as_str(), target.as_str()) {
        Ok(commits) => {
            tab.commits = commits;
            tab.compare_base = Some(base);
            tab.compare_target = Some(target);
            tab.selected = if tab.commits.is_empty() { None } else { Some(0) };
            if let Some(index) = tab.selected {
                select_commit(tab, index);
            }
            tab.error = None;
        }
        Err(error) => tab.error = Some(error.detail()),
    }
}

/// 把当前分支重置到某条 reflog 记录（不可撤销，调用方需确认）。
pub fn undo_to_reflog(tab: &mut Tab, index: usize) {
    let Some(entry) = tab.reflog.get(index).cloned() else {
        return;
    };
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    match repo.reset_hard(entry.sha.as_str()) {
        Ok(()) => {
            refresh_changes(tab);
            let filter = tab.filter.clone();
            let refreshed = load_tab(tab.path.as_str(), &filter);
            tab.commits = refreshed.commits;
            tab.branch = refreshed.branch;
            tab.ahead = refreshed.ahead;
            tab.behind = refreshed.behind;
            tab.reflog = refreshed.reflog;
        }
        Err(error) => tab.error = Some(error.detail()),
    }
}

/// 同步操作。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sync {
    Fetch,
    Pull,
    Push,
}

impl Sync {
    /// 界面上显示的动作名（同时用于文案键 `sync.<id>`）。
    pub fn id(self) -> &'static str {
        return match self {
            Sync::Fetch => "fetch",
            Sync::Pull => "pull",
            Sync::Push => "push",
        };
    }
}

/// 执行一次同步，并把结果留在标签页上（供操作条显示）。
pub fn sync(tab: &mut Tab, kind: Sync) {
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    let remote = repo
        .refs()
        .ok()
        .and_then(|refs| refs.remotes.first().cloned())
        .unwrap_or_else(|| "origin".to_string());
    let result = match kind {
        Sync::Fetch => repo.fetch(remote.as_str()),
        Sync::Pull => repo.pull(),
        Sync::Push => repo.push(Some(remote.as_str())),
    };
    match result {
        Ok(message) => {
            tab.sync_failed = false;
            tab.sync_message = Some(if message.trim().is_empty() {
                format!("{} 完成", kind.id())
            } else {
                message
            });
            refresh_changes(tab);
            // 领先/落后与历史都要跟着刷新。
            let filter = tab.filter.clone();
            let refreshed = load_tab(tab.path.as_str(), &filter);
            tab.branch = refreshed.branch;
            tab.ahead = refreshed.ahead;
            tab.behind = refreshed.behind;
            tab.commits = refreshed.commits;
            tab.refs = refreshed.refs;
            tab.changelists = refreshed.changelists;
        }
        Err(error) => {
            tab.sync_failed = true;
            tab.sync_message = Some(error.detail());
            refresh_changes(tab);
        }
    }
}

/// 用一侧内容解决冲突（`ours` 为真取当前分支）。
pub fn resolve_conflict(tab: &mut Tab, path: &str, ours: bool) {
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    match repo.resolve_with(path, ours) {
        Ok(()) => refresh_changes(tab),
        Err(error) => tab.error = Some(error.detail()),
    }
}

/// 放弃进行中的操作，回到干净工作区。
pub fn abort_operation(tab: &mut Tab) {
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    match repo.abort_operation(tab.operation) {
        Ok(()) => refresh_changes(tab),
        Err(error) => tab.error = Some(error.detail()),
    }
}

/// 暂存或取消暂存一条变更。
pub fn toggle_stage(tab: &mut Tab, index: usize) {
    let Some(entry) = tab.changes.get(index).cloned() else {
        return;
    };
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    let paths = vec![entry.path.clone()];
    let result = if entry.staged {
        repo.unstage(&paths)
    } else {
        repo.stage(&paths)
    };
    match result {
        Ok(()) => refresh_changes(tab),
        Err(error) => tab.error = Some(error.detail()),
    }
}

/// 暂存或取消暂存全部。
pub fn stage_all_or_none(tab: &mut Tab, stage: bool) {
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    let result = if stage {
        repo.stage_all()
    } else {
        repo.unstage_all()
    };
    match result {
        Ok(()) => refresh_changes(tab),
        Err(error) => tab.error = Some(error.detail()),
    }
}

/// 丢弃一条变更的工作区改动（不可撤销，调用方负责确认）。
pub fn discard_change(tab: &mut Tab, index: usize) {
    let Some(entry) = tab.changes.get(index).cloned() else {
        return;
    };
    let repo = match Repo::open(tab.path.as_str()) {
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

/// 提交暂存区；无论成败都保留钩子输出。
pub fn commit_staged(tab: &mut Tab) {
    let summary = tab.summary.trim().to_string();
    if summary.is_empty() {
        tab.hook_log = Some("提交说明不能为空".to_string());
        tab.hook_ok = false;
        return;
    }
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    match repo.commit(summary.as_str(), tab.description.as_str()) {
        Ok(outcome) => {
            tab.hook_ok = true;
            tab.hook_log = Some(if outcome.log.is_empty() {
                format!("已提交 {}", &outcome.sha[..7.min(outcome.sha.len())])
            } else {
                outcome.log
            });
            tab.summary.clear();
            tab.description.clear();
            refresh_changes(tab);
            // 提交后历史新增一条，重新读取以显示。
            let filter = tab.filter.clone();
            let refreshed = load_tab(tab.path.as_str(), &filter);
            tab.commits = refreshed.commits;
            tab.authors = refreshed.authors;
            tab.refs = refreshed.refs;
            tab.branch = refreshed.branch;
            tab.ahead = refreshed.ahead;
            tab.behind = refreshed.behind;
        }
        Err(error) => {
            tab.hook_ok = false;
            tab.hook_log = Some(error.detail());
        }
    }
}


/// 换一组筛选条件后重新装载。
pub fn reload_with_filter(tab: &Tab, filter: &LogFilter) -> Tab {
    let mut next = load_tab(tab.path.as_str(), filter);
    next.comparing = tab.comparing;
    return next;
}

fn last_segment(path: &str) -> String {
    return path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_string();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TempRepo {
        path: PathBuf,
    }

    impl TempRepo {
        fn new(name: &str) -> TempRepo {
            let unique = format!(
                "nsh-app-model-test-{}-{}-{}",
                name,
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_nanos())
                    .unwrap_or(0)
            );
            let path = std::env::temp_dir().join(unique);
            std::fs::create_dir_all(&path).expect("temp dir");
            let run = |args: &[&str]| {
                let status = std::process::Command::new("git")
                    .arg("-C")
                    .arg(&path)
                    .args(args)
                    .status()
                    .expect("git");
                assert!(status.success(), "git {:?}", args);
            };
            run(&["init", "-q", "-b", "main"]);
            run(&["config", "user.name", "Test"]);
            run(&["config", "user.email", "test@example.com"]);
            std::fs::write(path.join("a.txt"), "one\n").expect("write");
            run(&["add", "-A"]);
            run(&["commit", "-q", "-m", "first"]);
            return TempRepo { path };
        }
    }

    impl Drop for TempRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn loading_a_repo_fills_the_tab_and_selects_the_first_commit() {
        let repo = TempRepo::new("load");
        let tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        assert!(tab.error.is_none());
        assert_eq!(tab.branch, "main");
        assert_eq!(tab.commits.len(), 1);
        assert_eq!(tab.selected, Some(0));
        assert_eq!(tab.selected_commit().expect("commit").summary, "first");
        assert!(!tab.refs.branches.is_empty());
        assert!(!tab.patch.is_empty());
        assert_eq!(tab.files.len(), 1);
        assert_eq!(tab.authors, vec!["Test".to_string()]);
    }

    #[test]
    fn loading_a_non_repo_returns_an_error_tab() {
        let path = std::env::temp_dir().join(format!(
            "nsh-app-plain-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&path).expect("dir");
        let tab = load_tab(&path.display().to_string(), &LogFilter::default());
        let _ = std::fs::remove_dir_all(&path);
        assert!(tab.error.is_some());
        assert!(tab.commits.is_empty());
        assert_eq!(tab.selected, None);
    }

    #[test]
    fn keyword_filter_narrows_the_history() {
        let repo = TempRepo::new("filter");
        let path = repo.path.display().to_string();
        let all = load_tab(&path, &LogFilter::default());
        assert_eq!(all.commits.len(), 1);
        let filtered = load_tab(
            &path,
            &LogFilter {
                query: "no-such-summary".to_string(),
                ..LogFilter::default()
            },
        );
        assert!(filtered.commits.is_empty());
        assert_eq!(filtered.selected, None);
    }

    #[test]
    fn compare_range_sorts_and_needs_two_commits() {
        let repo = TempRepo::new("compare");
        let path = repo.path.display().to_string();
        let mut tab = load_tab(&path, &LogFilter::default());
        // 再提交两次，得到三个提交。
        for message in ["second", "third"] {
            std::fs::write(repo.path.join("a.txt"), format!("{message}\n")).expect("write");
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&repo.path)
                .args(["commit", "-q", "-am", message])
                .status()
                .expect("git");
            assert!(status.success());
        }
        tab = load_tab(&path, &LogFilter::default());
        assert_eq!(tab.commits.len(), 3);

        toggle_compare(&mut tab, 2);
        assert_eq!(tab.compare_range(), None, "一个提交不构成区间");
        toggle_compare(&mut tab, 0);
        assert_eq!(tab.compare_range(), Some((0, 2)));
        assert!(compare_patch(&tab).is_some(), "区间 diff 应可读出");

        toggle_compare(&mut tab, 0);
        assert_eq!(tab.compare_range(), None, "移出后不再成区间");
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
        toggle_stage(&mut tab, index);
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
        toggle_stage(&mut tab, index);
        assert!(!tab
            .changes
            .iter()
            .find(|entry| entry.path == "a.txt")
            .expect("a.txt")
            .staged);
    }

    #[test]
    fn commit_staged_creates_a_commit_and_clears_the_form() {
        let repo = TempRepo::new("commit");
        let path = repo.path.display().to_string();
        std::fs::write(repo.path.join("a.txt"), "second\n").expect("write");
        let mut tab = load_tab(&path, &LogFilter::default());
        stage_all_or_none(&mut tab, true);
        assert!(!tab.changes.is_empty());

        tab.summary = "Working tree and commit".to_string();
        tab.description = "Details.".to_string();
        commit_staged(&mut tab);

        assert!(tab.hook_ok, "提交应成功：{:?}", tab.hook_log);
        assert!(tab.summary.is_empty());
        assert!(tab.description.is_empty());
        assert!(tab.changes.is_empty(), "提交后工作区应干净");
        assert_eq!(
            tab.commits.first().map(|commit| commit.summary.as_str()),
            Some("Working tree and commit")
        );
    }

    #[test]
    fn commit_without_summary_reports_an_error() {
        let repo = TempRepo::new("commit-empty");
        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        std::fs::write(repo.path.join("a.txt"), "changed\n").expect("write");
        stage_all_or_none(&mut tab, true);
        commit_staged(&mut tab);
        assert!(!tab.hook_ok);
        assert!(tab.hook_log.is_some());
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

    #[test]
    fn sync_pushes_and_reports_clean_state() {
        // 建 bare 远程并让工作仓库跟踪它。
        let base = std::env::temp_dir().join(format!(
            "nsh-app-sync-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&base).expect("dir");
        let remote = base.join("remote.git");
        let repo_dir = base.join("local");
        let run = |directory: &std::path::Path, args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(directory)
                .args(args)
                .status()
                .expect("git");
            assert!(status.success(), "git {:?}", args);
        };
        run(&base, &["init", "-q", "--bare", remote.to_str().expect("utf8")]);
        run(&base, &["init", "-q", "-b", "main", repo_dir.to_str().expect("utf8")]);
        run(&repo_dir, &["config", "user.name", "Test"]);
        run(&repo_dir, &["config", "user.email", "test@example.com"]);
        std::fs::write(repo_dir.join("a.txt"), "one\n").expect("write");
        run(&repo_dir, &["add", "-A"]);
        run(&repo_dir, &["commit", "-q", "-m", "base"]);
        run(
            &repo_dir,
            &["remote", "add", "origin", remote.to_str().expect("utf8")],
        );
        run(&repo_dir, &["push", "-q", "-u", "origin", "main"]);

        let mut tab = load_tab(&repo_dir.display().to_string(), &LogFilter::default());
        assert_eq!((tab.ahead, tab.behind), (0, 0));

        // 本地再提交一次，推送后领先数回到 0，并留下同步信息。
        std::fs::write(repo_dir.join("a.txt"), "two\n").expect("write");
        stage_all_or_none(&mut tab, true);
        tab.summary = "second".to_string();
        commit_staged(&mut tab);
        assert_eq!(tab.ahead, 1, "提交后应领先远程一个提交");

        sync(&mut tab, Sync::Push);
        assert!(!tab.sync_failed, "推送应成功：{:?}", tab.sync_message);
        assert!(tab.sync_message.is_some());
        assert_eq!(tab.ahead, 0, "推送后不再领先");

        sync(&mut tab, Sync::Fetch);
        assert!(!tab.sync_failed);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn reflog_is_loaded_and_undo_resets_the_branch() {
        let repo = TempRepo::new("reflog");
        let path = repo.path.display().to_string();
        std::fs::write(repo.path.join("a.txt"), "two\n").expect("write");
        let mut tab = load_tab(&path, &LogFilter::default());
        stage_all_or_none(&mut tab, true);
        tab.summary = "second".to_string();
        commit_staged(&mut tab);

        // 提交后 reflog 顶部应是这次 commit。
        let mut tab = load_tab(&path, &LogFilter::default());
        assert!(!tab.reflog.is_empty());
        assert_eq!(tab.reflog[0].action, "commit");
        let newest = tab.commits[0].sha.clone();

        // 撤销到 reflog 里第一条之外的记录（即上一个提交）。
        let target = tab
            .reflog
            .iter()
            .position(|entry| entry.sha != newest)
            .expect("older entry");
        let target_sha = tab.reflog[target].sha.clone();
        undo_to_reflog(&mut tab, target);

        assert_eq!(tab.commits[0].sha, target_sha, "分支应回到目标提交");
        assert!(
            !tab.reflog.is_empty() && tab.reflog[0].action == "reset",
            "撤销本身也应留下 reflog 记录"
        );
    }

    #[test]
    fn compare_refs_narrows_history_to_the_range() {
        let repo = TempRepo::new("cmp");
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
        run(&["branch", "base"]);
        std::fs::write(repo.path.join("a.txt"), "two\n").expect("write");
        run(&["commit", "-q", "-am", "second"]);
        std::fs::write(repo.path.join("a.txt"), "three\n").expect("write");
        run(&["commit", "-q", "-am", "third"]);

        let mut tab = load_tab(&path, &LogFilter::default());
        assert_eq!(tab.commits.len(), 3);
        set_compare_refs(&mut tab, Some("base".to_string()), Some("main".to_string()));
        assert_eq!(tab.compare_base.as_deref(), Some("base"));
        assert_eq!(tab.compare_target.as_deref(), Some("main"));
        assert_eq!(tab.commits.len(), 2, "base..main 应只含两个提交");
        assert_eq!(tab.commits[0].summary, "third");
        // 选中项随之更新，详情与文件列表跟着刷新。
        assert!(tab.details.is_some());
        assert!(!tab.files.is_empty());

        // 传空表示退出对比，历史恢复。
        set_compare_refs(&mut tab, None, None);
        assert!(tab.compare_base.is_none());
        assert_eq!(tab.commits.len(), 3);
    }

    #[test]
    fn selecting_a_file_fills_blame_and_history() {
        let repo = TempRepo::new("blame");
        let path = repo.path.display().to_string();
        let mut tab = load_tab(&path, &LogFilter::default());
        assert!(!tab.files.is_empty());
        select_file(&mut tab, 0);
        assert!(!tab.blame.is_empty(), "选中文件后应有 blame 行");
        assert_eq!(tab.blame[0].text, "one");
        assert!(!tab.file_history.is_empty(), "选中文件后应有文件历史");
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
        assert!(tab
            .refs
            .branches
            .iter()
            .any(|branch| branch.name == "feature" && branch.current));
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
    fn ref_forms_create_branches_tags_and_remotes() {
        let repo = TempRepo::new("refwrite");
        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        let before = tab.refs.branches.len();

        create_branch(&mut tab, "feature/x", "").expect("branch");
        assert_eq!(tab.refs.branches.len(), before + 1);
        assert!(tab.refs.branches.iter().any(|branch| branch.name == "feature/x"));

        create_tag(&mut tab, "v0.0.1", "").expect("tag");
        assert_eq!(tab.refs.tags, vec!["v0.0.1".to_string()]);

        add_remote(&mut tab, "origin", "https://example.com/repo.git").expect("remote");
        assert_eq!(tab.refs.remotes, vec!["origin".to_string()]);

        // 空名字被 Git 拒绝，错误应回传给调用方。
        assert!(create_branch(&mut tab, "", "").is_err());
    }

    #[test]
    fn stash_changes_moves_worktree_content_away() {
        let repo = TempRepo::new("stashmodel");
        std::fs::write(repo.path.join("a.txt"), "changed\n").expect("write");
        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        assert!(!tab.changes.is_empty());

        stash_changes(&mut tab, "wip").expect("stash");
        assert!(tab.changes.is_empty(), "贮藏后变更列表应为空");
        assert_eq!(tab.refs.stashes.len(), 1);

        let reference = tab.refs.stashes[0].reference.clone();
        drop_stash(&mut tab, reference.as_str()).expect("drop");
        assert!(tab.refs.stashes.is_empty());
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
        assert!(reloaded
            .changelists
            .iter()
            .any(|list| list.name == "Commit composer"
                && list.paths == vec!["a.txt".to_string()]));

        // 移回默认清单：清单条目被清空。
        move_to_changelist(&mut tab, "a.txt", None);
        assert!(tab.changelists.iter().all(|list| list.paths.is_empty()));

        set_active_changelist(&mut tab, Some("Commit composer".to_string()));
        assert_eq!(tab.active_changelist.as_deref(), Some("Commit composer"));

        unsafe { std::env::remove_var("APP_HOME") };
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn selecting_a_file_loads_its_patch_only() {
        let repo = TempRepo::new("file");
        // 同时改动两个文件，第二个提交的 numstat 才有两行。
        std::fs::write(repo.path.join("a.txt"), "one\ntwo\n").expect("write");
        std::fs::write(repo.path.join("b.txt"), "x\n").expect("write");
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
            .args(["commit", "-q", "-m", "two files"])
            .status()
            .expect("git");
        assert!(status.success());
        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        assert_eq!(tab.files.len(), 2);
        select_file(&mut tab, 0);
        let (path, patch) = tab.file_patch.clone().expect("file patch");
        assert!(!path.is_empty());
        // 补丁只应包含所选文件，另一个文件不应出现。
        let other = if path == "a.txt" { "b.txt" } else { "a.txt" };
        assert!(!patch.contains(&format!("+++ b/{}", other)));
        assert_eq!(tab.selected_file, Some(0));
    }
}
