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
    /// 提交面板的三个开关（设计板 10）。成功后归位。
    pub amend: bool,
    pub signoff: bool,
    pub push_after_commit: bool,
    /// 钩子完整日志弹层是否打开。
    pub hook_open: bool,
    /// 正在后台读取（设计板 16 的 loading 状态）。
    pub loading: bool,
    /// HEAD 形态：分支 / 游离 / 尚未提交（设计板 16）。
    pub head: git::HeadState,
    /// 最近一次同步失败的归类（offline / auth / rejected），猜不出来时为空。
    pub sync_fault: Option<domain::NetworkFault>,
    /// 各远端的 URL（`remote -v`），用于展示与「复制 URL」。
    pub remote_urls: Vec<(String, String)>,
    /// 冲突视图：当前查看的冲突文件下标与是否打开。
    pub conflict_index: usize,
    pub conflict_open: bool,
    /// 冲突文件当前内容（含标记）。
    pub conflict_text: String,
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

/// 文件树的两项显示偏好（设计板 23）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TreeOptions {
    /// 合并空目录：只含子目录、自己不含文件的层级不单独占一行。
    pub merge_empty_dirs: bool,
    /// 压缩单子目录：`a/b/c` 这种链式目录合并成一行显示。
    pub compact_single_child: bool,
}

impl TreeOptions {
    pub fn from_settings(settings: &store::Settings) -> TreeOptions {
        return TreeOptions {
            merge_empty_dirs: settings.merge_empty_dirs,
            compact_single_child: settings.compact_single_child,
        };
    }

    /// 目录行是否要显示。
    ///
    /// `merge_empty_dirs` 打开时，只有子目录、自己没有直接文件的层级不占行——
    /// 这正是「merge empty directories」要的效果：不把 `src` 这种纯容器画成一行。
    pub fn shows_directory(self, direct_files: usize) -> bool {
        if !self.merge_empty_dirs {
            return true;
        }
        return direct_files > 0;
    }

    /// 把文件名所属目录链压缩成一行显示用的标签。
    ///
    /// `dirs` 是所有出现过的目录（完整路径）。`compact_single_child` 打开时，
    /// 如果 `a` 下只有 `b`、`b` 下只有 `c`，标签就是 `a/b/c`。
    pub fn compact_label(self, dirs: &[String], path: &str) -> String {
        if path.is_empty() || !self.compact_single_child {
            return path.to_string();
        }
        let mut label = String::new();
        let mut prefix: String;
        for part in path.split('/') {
            if label.is_empty() {
                label.push_str(part);
            } else {
                label.push('/');
                label.push_str(part);
            }
            prefix = label.clone();
            // 该前缀下的直接子目录个数。
            let child_prefix = format!("{prefix}/");
            let mut children: Vec<&String> = dirs
                .iter()
                .filter(|candidate| candidate.starts_with(child_prefix.as_str()))
                .filter(|candidate| !candidate[child_prefix.len()..].contains('/'))
                .collect();
            children.sort();
            children.dedup();
            if children.len() != 1 {
                break;
            }
        }
        return label;
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
    let remote_urls = repo.remote_urls().unwrap_or_default();
    let head = repo.head_state();
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
        head,
        remote_urls,
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

/// 按 SHA 选中一个提交（reflog 的「在历史中显示」）。
///
/// 当前筛选可能把该提交过滤掉了；找不到时先清空关键字再找一次，仍找不到返回
/// `false`，由界面提示而不是悄悄失败。
pub fn select_by_sha(tab: &mut Tab, sha: &str) -> bool {
    if let Some(index) = tab.commits.iter().position(|commit| commit.sha == sha) {
        select_commit(tab, index);
        return true;
    }
    let filter = LogFilter::default();
    if let Ok(commits) = Repo::open(tab.path.as_str()).and_then(|repo| {
        repo.log_filtered(200, &filter)
            .map_err(|error| error)
    }) {
        tab.commits = commits;
        tab.filter = filter;
    }
    if let Some(index) = tab.commits.iter().position(|commit| commit.sha == sha) {
        select_commit(tab, index);
        return true;
    }
    return false;
}

/// 生成一个「正在加载」的占位标签页：只有路径与名称，列表都是空的。
pub fn placeholder_tab(path: &str, filter: &LogFilter) -> Tab {
    return Tab {
        path: path.to_string(),
        name: last_segment(path),
        loading: true,
        filter: filter.clone(),
        branch: "…".to_string(),
        ..Tab::default()
    };
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
    tab.remote_urls = repo.remote_urls().unwrap_or_default();
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
/// 切到某个提交（detached HEAD）。调用方先确认，因为离开当前分支容易误提交。
pub fn checkout_commit(tab: &mut Tab, sha: &str) -> Result<String, String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.checkout_branch(sha).map_err(|error| error.detail())?;
    return reload_after_history_change(tab);
}

/// 拣选一个提交到当前分支；冲突时进入冲突态，由冲突界面接手。
pub fn cherry_pick(tab: &mut Tab, sha: &str) -> Result<String, String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    let outcome = repo.cherry_pick(sha);
    let summary = reload_after_history_change(tab)?;
    match outcome {
        Ok(()) => return Ok(summary),
        Err(error) => return Err(error.detail()),
    }
}

/// 反向提交一个提交；冲突处理同上。
pub fn revert_commit(tab: &mut Tab, sha: &str) -> Result<String, String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    let outcome = repo.revert(sha);
    let summary = reload_after_history_change(tab)?;
    match outcome {
        Ok(()) => return Ok(summary),
        Err(error) => return Err(error.detail()),
    }
}

/// 历史被改写或切走后，把列表、引用与操作状态一起刷新。
fn reload_after_history_change(tab: &mut Tab) -> Result<String, String> {
    refresh_changes(tab);
    let filter = tab.filter.clone();
    let refreshed = load_tab(tab.path.as_str(), &filter);
    tab.commits = refreshed.commits;
    tab.refs = refreshed.refs;
    tab.branch = refreshed.branch.clone();
    tab.operation = refreshed.operation;
    tab.conflicts = refreshed.conflicts;
    tab.ahead = refreshed.ahead;
    tab.behind = refreshed.behind;
    tab.selected = None;
    if !tab.commits.is_empty() {
        select_commit(tab, 0);
    }
    return Ok(refreshed.branch);
}

/// 按 hunk 或单行暂存/取消暂存当前选中的变更。
///
/// `line_index` 为 `None` 时整段 hunk；`reverse` 为真表示从暂存区取回。
/// 补丁来源是 `tab.change_diff`（当前文件相对暂存区的 diff）。
pub fn apply_piece(
    tab: &mut Tab,
    hunk_index: usize,
    line_index: Option<usize>,
    reverse: bool,
) -> Result<(), String> {
    let sections = domain::stage::Sections::parse(tab.change_diff.as_str());
    let patch = match line_index {
        Some(line) => sections.line_patch(0, hunk_index, line),
        None => sections.hunk_patch(0, hunk_index),
    }
    .ok_or_else(|| "这一段没有可暂存的内容".to_string())?;
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.apply_cached(patch.as_str(), reverse)
        .map_err(|error| error.detail())?;
    refresh_changes(tab);
    // 重新装载当前选中文件的 diff，让刚改的片段立刻反映出来。
    let selected = tab.selected_change;
    let filter = tab.filter.clone();
    let refreshed = load_tab(tab.path.as_str(), &filter);
    tab.commits = refreshed.commits;
    tab.changes = refreshed.changes;
    tab.status = refreshed.status;
    if let Some(index) = selected {
        if index < tab.changes.len() {
            load_change_diff(tab, index);
        } else {
            tab.selected_change = None;
            tab.change_diff.clear();
        }
    }
    return Ok(());
}

/// 空仓库里创建第一个提交（`--allow-empty`，只为了让分支有起点）。
pub fn create_initial_commit(tab: &mut Tab, summary: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    let options = git::CommitOptions {
        allow_empty: true,
        ..git::CommitOptions::default()
    };
    repo.commit_with(summary, "", &options)
        .map_err(|error| error.detail())?;
    let filter = tab.filter.clone();
    let refreshed = load_tab(tab.path.as_str(), &filter);
    tab.commits = refreshed.commits;
    tab.head = refreshed.head;
    tab.branch = refreshed.branch;
    tab.refs = refreshed.refs;
    tab.remote_urls = refreshed.remote_urls;
    tab.status = refreshed.status;
    tab.changes = refreshed.changes;
    return Ok(());
}

/// 游离头时回到默认分支（本地没有就用远端同名分支建一个）。
pub fn checkout_default_branch(tab: &mut Tab) -> Result<(), String> {
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

/// 清掉同步失败的状态（重试或用户主动关闭横幅）。
pub fn clear_sync_fault(tab: &mut Tab) {
    tab.sync_fault = None;
}

/// 改写提交说明。
pub fn reword_commit(tab: &mut Tab, sha: &str, message: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.reword_commit(sha, message).map_err(|error| error.detail())?;
    reload_after_history_change(tab)?;
    return Ok(());
}

/// 合并连续的一段提交。
pub fn squash_commits(
    tab: &mut Tab,
    from: &str,
    to: &str,
    message: &str,
) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.squash_commits(from, to, message)
        .map_err(|error| error.detail())?;
    reload_after_history_change(tab)?;
    return Ok(());
}

/// 当前是否能改写历史；`Err` 里是给用户看的原因。
pub fn rewrite_blocker(tab: &Tab) -> Option<String> {
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => return Some(error.detail()),
    };
    return repo.can_rewrite().err();
}

/// 打开冲突视图，并载入第一个冲突文件的内容。
pub fn open_conflict(tab: &mut Tab) {
    tab.conflict_open = true;
    tab.conflict_index = 0;
    load_conflict_text(tab);
}

/// 切换到第 `index` 个冲突文件。
pub fn select_conflict(tab: &mut Tab, index: usize) {
    if index >= tab.conflicts.len() {
        return;
    }
    tab.conflict_index = index;
    load_conflict_text(tab);
}

/// 前一个/后一个冲突文件（到头就停住，不回绕，避免误操作）。
pub fn step_conflict(tab: &mut Tab, forward: bool) {
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
        return;
    };
    tab.conflict_text = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo.conflict_content(path.as_str()),
        Err(error) => {
            tab.error = Some(error.detail());
            String::new()
        }
    };
}

/// 把当前冲突文件的某一侧整体写进结果。
pub fn conflict_take_side(tab: &mut Tab, side: domain::conflict::Side) -> String {
    return domain::conflict::merge(tab.conflict_text.as_str(), side);
}

/// 标记当前冲突文件为已解决：写入结果并 `git add`。
pub fn mark_conflict_resolved(tab: &mut Tab, resolved: &str) -> Result<(), String> {
    let Some(path) = tab.conflicts.get(tab.conflict_index).cloned() else {
        return Ok(());
    };
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.resolve_conflict(path.as_str(), resolved)
        .map_err(|error| error.detail())?;
    tab.conflicts = repo.conflicts().map_err(|error| error.detail())?;
    tab.conflict_text = repo.conflict_content(path.as_str());
    if tab.conflict_index >= tab.conflicts.len() {
        tab.conflict_index = tab.conflicts.len().saturating_sub(1);
    }
    refresh_changes(tab);
    if tab.conflicts.is_empty() {
        tab.conflict_open = false;
    }
    return Ok(());
}

/// 继续被冲突打断的操作；成功后刷新整个 tab。
pub fn continue_operation(tab: &mut Tab) -> Result<String, String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    let summary = repo
        .continue_operation(tab.operation)
        .map_err(|error| error.detail())?;
    let branch = reload_after_history_change(tab)?;
    if !tab.conflicts.is_empty() {
        return Err("仍有未解决的冲突".to_string());
    }
    return Ok(if summary.trim().is_empty() {
        branch
    } else {
        summary
    });
}

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
    refresh_refs(tab, &repo);
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
    refresh_refs(tab, &repo);
    return Ok(());
}

/// 添加远程。
pub fn add_remote(tab: &mut Tab, name: &str, url: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.add_remote(name, url).map_err(|error| error.detail())?;
    refresh_refs(tab, &repo);
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
    refresh_refs(tab, &repo);
    return Ok(());
}

/// 删除标签。
/// 刷新引用与其附属信息（远端 URL）。所有改引用的动作都走这里。
fn refresh_refs(tab: &mut Tab, repo: &Repo) {
    tab.refs = repo.refs().unwrap_or_default();
    tab.remote_urls = repo.remote_urls().unwrap_or_default();
}

/// 推送标签到首个远端。
pub fn push_tag(tab: &mut Tab, name: &str) -> Result<String, String> {
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
pub fn apply_stash(tab: &mut Tab, reference: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.stash_apply(reference).map_err(|error| error.detail())?;
    tab.refs = repo.refs().unwrap_or_default();
    refresh_changes(tab);
    return Ok(());
}

/// 弹出贮藏：取出并删除条目。
pub fn pop_stash(tab: &mut Tab, reference: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    // `git stash pop` 只取最近一条；指定引用时用 `apply` + `drop` 组合，语义一致。
    repo.stash_apply(reference).map_err(|error| error.detail())?;
    repo.stash_drop(reference).map_err(|error| error.detail())?;
    tab.refs = repo.refs().unwrap_or_default();
    refresh_changes(tab);
    return Ok(());
}

/// 重命名分支。
pub fn rename_branch(tab: &mut Tab, old: &str, new: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.rename_branch(old, new).map_err(|error| error.detail())?;
    tab.refs = repo.refs().unwrap_or_default();
    tab.branch = repo.abbrev_branch().unwrap_or_default();
    return Ok(());
}

/// 仓库级提交身份（`user.name` / `user.email`）。
pub fn identity(tab: &Tab) -> (String, String) {
    let Ok(repo) = Repo::open(tab.path.as_str()) else {
        return (String::new(), String::new());
    };
    let name = repo.config_get_local("user.name").unwrap_or_default();
    let email = repo.config_get_local("user.email").unwrap_or_default();
    return (name, email);
}

/// 写入仓库级提交身份。
pub fn set_identity(tab: &Tab, name: &str, email: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.config_set("user.name", name)
        .map_err(|error| error.detail())?;
    repo.config_set("user.email", email)
        .map_err(|error| error.detail())?;
    return Ok(());
}

/// 清掉仓库级身份，回到全局配置。
pub fn clear_identity(tab: &Tab) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.config_unset("user.name").map_err(|error| error.detail())?;
    repo.config_unset("user.email").map_err(|error| error.detail())?;
    return Ok(());
}

/// 改写远端的 URL。
pub fn edit_remote(tab: &mut Tab, name: &str, url: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.set_remote_url(name, url).map_err(|error| error.detail())?;
    refresh_refs(tab, &repo);
    return Ok(());
}

pub fn drop_tag(tab: &mut Tab, name: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.delete_tag(name).map_err(|error| error.detail())?;
    refresh_refs(tab, &repo);
    return Ok(());
}

/// 移除远程。
pub fn drop_remote(tab: &mut Tab, name: &str) -> Result<(), String> {
    let repo = Repo::open(tab.path.as_str()).map_err(|error| error.detail())?;
    repo.remove_remote(name).map_err(|error| error.detail())?;
    refresh_refs(tab, &repo);
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
/// 同步选项：三种动作共用一份，节省一个入参。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SyncOptions {
    pub fetch: git::FetchOptions,
    pub pull: git::PullOptions,
    pub push: git::PushOptions,
}

impl SyncOptions {
    /// 同步动作的选项是否含破坏性含义（强推）。
    pub fn is_destructive(&self, kind: Sync) -> bool {
        return kind == Sync::Push && self.push.force_with_lease;
    }
}

pub fn sync(tab: &mut Tab, kind: Sync) {
    return sync_with(tab, kind, &SyncOptions::default());
}

pub fn sync_with(tab: &mut Tab, kind: Sync, options: &SyncOptions) {
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
        Sync::Fetch => repo.fetch_with(remote.as_str(), &options.fetch),
        Sync::Pull => repo.pull_with(&options.pull),
        Sync::Push => repo.push_with(Some(remote.as_str()), &options.push),
    };
    match result {
        Ok(message) => {
            tab.sync_failed = false;
            tab.sync_fault = None;
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
            tab.sync_fault = domain::classify_network_error(error.detail().as_str());
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
    return commit_staged_with(tab, false);
}

/// 提交暂存区；`no_verify` 为真时跳过钩子（界面走强确认）。
///
/// 面板上的「改写上一条」与「sign-off」来自 `Tab` 的开关，成功后归位，
/// 避免下一次提交意外沿用。
pub fn commit_staged_with(tab: &mut Tab, no_verify: bool) {
    let summary = tab.summary.trim().to_string();
    if summary.is_empty() {
        tab.hook_log = Some("提交说明不能为空".to_string());
        tab.hook_ok = false;
        return;
    }
    let options = git::CommitOptions {
        amend: tab.amend,
        signoff: tab.signoff,
        no_verify,
        allow_empty: false,
    };
    let repo = match Repo::open(tab.path.as_str()) {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    match repo.commit_with(summary.as_str(), tab.description.as_str(), &options) {
        Ok(outcome) => {
            tab.hook_ok = true;
            tab.hook_log = Some(if outcome.log.is_empty() {
                format!("已提交 {}", &outcome.sha[..7.min(outcome.sha.len())])
            } else {
                outcome.log
            });
            tab.summary.clear();
            tab.description.clear();
            tab.amend = false;
            tab.signoff = false;
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
        // 先造提交，再装载 tab，避免读一次再整体覆盖。
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
        let mut tab = load_tab(&path, &LogFilter::default());
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
    fn amend_reuses_the_last_commit_and_resets_the_switch() {
        let repo = TempRepo::new("amend");
        let path = repo.path.display().to_string();
        let mut tab = load_tab(&path, &LogFilter::default());
        let before = tab.commits.len();

        // 只改说明、不动内容：amend 不需要暂存内容。
        tab.summary = "Rewritten".to_string();
        tab.amend = true;
        commit_staged(&mut tab);

        assert!(tab.hook_ok, "改写应成功：{:?}", tab.hook_log);
        assert_eq!(tab.commits.len(), before, "改写不新增提交");
        assert_eq!(tab.commits[0].summary, "Rewritten");
        assert!(!tab.amend, "开关成功后应归位");
    }

    #[test]
    fn commit_without_hooks_bypasses_a_failing_hook() {
        let repo = TempRepo::new("skip-hooks");
        let path = repo.path.display().to_string();
        let hook = repo.path.join(".git/hooks/pre-commit");
        std::fs::write(&hook, "#!/bin/sh\nexit 1\n").expect("hook");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
        }
        std::fs::write(repo.path.join("a.txt"), "changed\n").expect("write");
        let mut tab = load_tab(&path, &LogFilter::default());
        stage_all_or_none(&mut tab, true);
        tab.summary = "Blocked".to_string();

        commit_staged(&mut tab);
        assert!(!tab.hook_ok, "钩子应拦住普通提交");

        commit_staged_with(&mut tab, true);
        assert!(tab.hook_ok, "跳过钩子应成功：{:?}", tab.hook_log);
        assert_eq!(tab.commits[0].summary, "Blocked");
    }

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
        assert!(!tab.conflict_open, "全部解决后视图应关闭");
        assert_eq!(
            std::fs::read_to_string(repo.path.join("a.txt")).expect("read"),
            "side\n"
        );

        // 继续合并。
        continue_operation(&mut tab).expect("continue");
        assert_eq!(tab.operation, git::Operation::None, "合并应已结束");
    }

    #[test]
    fn select_by_sha_finds_the_commit_even_when_filtered_out() {
        let repo = TempRepo::new("select-sha");
        let path = repo.path.display().to_string();
        std::fs::write(repo.path.join("a.txt"), "second\n").expect("write");
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo.path)
            .args(["commit", "-q", "-am", "second"])
            .status()
            .expect("git");
        assert!(status.success());

        let mut tab = load_tab(&path, &LogFilter::default());
        assert_eq!(tab.commits.len(), 2);
        let oldest = tab.commits[1].sha.clone();

        // 用关键字把列表筛到只剩最新一条，再按 SHA 找最旧那条。
        tab.filter.query = "second".to_string();
        let filtered = load_tab(&path, &tab.filter);
        assert_eq!(filtered.commits.len(), 1, "筛选应只剩一条");
        let mut tab = filtered;
        assert!(select_by_sha(&mut tab, oldest.as_str()), "应能在清筛选后找到");
        assert_eq!(
            tab.commits.get(tab.selected.unwrap_or(0)).map(|c| c.sha.clone()),
            Some(oldest)
        );

        // 不存在的 SHA 返回 false，不改变选中。
        assert!(!select_by_sha(&mut tab, "0000000000000000000000000000000000000000"));
    }

    #[test]
    fn placeholder_tab_is_loading_and_has_no_data() {
        let tab = placeholder_tab("/tmp/whatever", &LogFilter::default());
        assert!(tab.loading, "占位标签页应标记为加载中");
        assert_eq!(tab.name, "whatever");
        assert!(tab.commits.is_empty());
        assert!(tab.changes.is_empty());
        assert!(tab.error.is_none(), "占位不应显示错误面板");

        // 真的装载出来的标签页不是 loading。
        let repo = TempRepo::new("loading-flag");
        let loaded = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        assert!(!loaded.loading, "装载完成后应清掉 loading");
    }

    #[test]
    fn tree_options_compact_single_child_chains() {
        let all = TreeOptions {
            merge_empty_dirs: true,
            compact_single_child: true,
        };
        // src 下只有 git，git 下只有 legacy → 整条链压成一行。
        let dirs = vec![
            "src".to_string(),
            "src/git".to_string(),
            "src/git/legacy".to_string(),
        ];
        assert_eq!(all.compact_label(&dirs, "src/git/legacy"), "src/git/legacy");
        // 一旦某层有多个子目录，就在那层断开。
        let dirs = vec![
            "src".to_string(),
            "src/git".to_string(),
            "src/ui".to_string(),
            "src/git/legacy".to_string(),
        ];
        assert_eq!(
            all.compact_label(&dirs, "src/git/legacy"),
            "src",
            "src 有两个子目录，压缩应停在 src"
        );

        // 关掉压缩时原样返回。
        let plain = TreeOptions {
            merge_empty_dirs: true,
            compact_single_child: false,
        };
        assert_eq!(plain.compact_label(&dirs, "src/git/legacy"), "src/git/legacy");
        // 根目录没有标签。
        assert_eq!(all.compact_label(&dirs, ""), "");
    }

    #[test]
    fn tree_options_decide_whether_a_directory_row_shows() {
        let merged = TreeOptions {
            merge_empty_dirs: true,
            compact_single_child: false,
        };
        assert!(merged.shows_directory(2), "有直接文件就显示");
        assert!(!merged.shows_directory(0), "纯容器目录不占行");

        let unmerged = TreeOptions {
            merge_empty_dirs: false,
            compact_single_child: false,
        };
        assert!(unmerged.shows_directory(0), "关闭合并时空目录也显示");
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
        std::fs::write(
            repo.path.join("f.txt"),
            format!("{}\n", changed.join("\n")),
        )
        .expect("write");

        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        let index = tab
            .changes
            .iter()
            .position(|entry| entry.path == "f.txt")
            .expect("f.txt");
        load_change_diff(&mut tab, index);
        let sections = domain::stage::Sections::parse(tab.change_diff.as_str());
        assert_eq!(sections.files[0].hunks.len(), 2, "应有两个 hunk");

        // 只暂存第二个 hunk。
        apply_piece(&mut tab, 1, None, false).expect("stage hunk");
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
    fn empty_repository_is_detected_and_can_get_a_first_commit() {
        let dir = std::env::temp_dir().join(format!(
            "nsh-app-unborn-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("dir");
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(&dir)
            .args(["init", "-q", "-b", "main"])
            .status()
            .expect("git init");
        assert!(status.success());

        let mut tab = load_tab(&dir.display().to_string(), &LogFilter::default());
        assert!(
            matches!(tab.head, git::HeadState::Unborn { ref branch } if branch == "main"),
            "新仓库应识别为未提交状态：{:?}",
            tab.head
        );
        assert!(tab.commits.is_empty());

        create_initial_commit(&mut tab, "Initial commit").expect("initial commit");
        assert_eq!(
            tab.head,
            git::HeadState::Branch("main".to_string()),
            "提交后应回到分支状态"
        );
        assert_eq!(tab.commits.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

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
    fn sync_failures_are_classified_for_the_banner() {
        // 直接验证归类函数接到界面上用的那条路径。
        assert_eq!(
            domain::classify_network_error("fatal: Authentication failed for 'https://x/'"),
            Some(domain::NetworkFault::AuthRequired)
        );
        let mut tab = Tab::default();
        assert_eq!(tab.sync_fault, None);
        tab.sync_fault = Some(domain::NetworkFault::Offline);
        clear_sync_fault(&mut tab);
        assert_eq!(tab.sync_fault, None);
    }

    #[test]
    fn rewrite_wrappers_reword_and_squash_through_the_model() {
        let repo = TempRepo::new("rewrite");
        let path = repo.path.display().to_string();
        // 造三个提交。
        for (file, message) in [("b.txt", "second"), ("c.txt", "third")] {
            std::fs::write(repo.path.join(file), "x\n").expect("write");
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
                .args(["commit", "-q", "-m", message])
                .status()
                .expect("git");
            assert!(status.success());
        }
        let mut tab = load_tab(&path, &LogFilter::default());
        assert_eq!(tab.commits.len(), 3);
        assert!(rewrite_blocker(&tab).is_none(), "干净工作区应可改写");

        // 改写中间那条的说明。
        let middle = tab.commits[1].sha.clone();
        reword_commit(&mut tab, middle.as_str(), "second (edited)").expect("reword");
        assert_eq!(tab.commits[1].summary, "second (edited)");
        assert_eq!(tab.commits.len(), 3, "改写不改变提交数");

        // 合并最近两个。
        let newest = tab.commits[0].sha.clone();
        let older = tab.commits[1].sha.clone();
        squash_commits(&mut tab, older.as_str(), newest.as_str(), "combined")
            .expect("squash");
        assert_eq!(tab.commits.len(), 2);
        assert_eq!(tab.commits[0].summary, "combined");
    }

    #[test]
    fn rewrite_blocker_reports_a_dirty_worktree() {
        let repo = TempRepo::new("rewrite-dirty");
        std::fs::write(repo.path.join("a.txt"), "dirty\n").expect("write");
        let tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        let blocker = rewrite_blocker(&tab).expect("脏工作区应被拦下");
        assert!(blocker.contains("未提交"), "原因应可读：{blocker}");
    }

    #[test]
    fn identity_reads_writes_and_clears_repo_config() {
        let repo = TempRepo::new("identity");
        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());

        // 测试仓库自带仓库级身份（fixture 里设的）。
        assert_eq!(
            identity(&tab),
            ("Test".to_string(), "test@example.com".to_string())
        );

        set_identity(&tab, "Ada", "ada@example.com").expect("set identity");
        assert_eq!(
            identity(&tab),
            ("Ada".to_string(), "ada@example.com".to_string())
        );

        // 清掉后回到「没有仓库级身份」，与全局配置无关。
        clear_identity(&tab).expect("clear identity");
        assert_eq!(identity(&tab), (String::new(), String::new()));

        // 再清一次不应报错（键已不存在）。
        clear_identity(&tab).expect("clear twice");
        tab.summary = "x".to_string();
    }

    #[test]
    fn refs_actions_rename_branch_and_apply_stash() {
        let repo = TempRepo::new("refs-actions");
        let path = repo.path.display().to_string();

        // 贮藏：改动先藏起来，列表里有条目；应用后改动回来且条目还在。
        std::fs::write(repo.path.join("a.txt"), "stashed\n").expect("write");
        let mut tab = load_tab(&path, &LogFilter::default());
        stash_changes(&mut tab, "wip").expect("stash");
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
        assert!(tab.refs.branches.iter().any(|branch| branch.name == "renamed"));

        // 远端 URL 列表与编辑。
        add_remote(&mut tab, "origin", "https://example.com/a.git").expect("add remote");
        assert!(tab.remote_urls.iter().any(|(name, _)| name == "origin"));
        edit_remote(&mut tab, "origin", "git@example.com:b.git").expect("edit remote");
        assert!(tab
            .remote_urls
            .iter()
            .any(|(_, url)| url == "git@example.com:b.git"));
    }

    #[test]
    fn cherry_pick_and_revert_round_trip_through_the_model() {
        let repo = TempRepo::new("pick-revert");
        let path = repo.path.display().to_string();
        std::fs::write(repo.path.join("b.txt"), "bee\n").expect("write");
        let mut tab = load_tab(&path, &LogFilter::default());
        stage_all_or_none(&mut tab, true);
        tab.summary = "add b".to_string();
        commit_staged(&mut tab);
        let picked = tab.commits[0].sha.clone();

        // 回到初始提交，再拣选回来。
        let base = tab.commits[1].sha.clone();
        reset_to(&mut tab, base.as_str(), git::ResetMode::Hard).expect("reset");
        assert!(!repo.path.join("b.txt").exists());

        cherry_pick(&mut tab, picked.as_str()).expect("cherry-pick");
        assert!(repo.path.join("b.txt").exists(), "拣选应带过来文件");
        assert_eq!(tab.commits[0].summary, "add b");

        let picked = tab.commits[0].sha.clone();
        revert_commit(&mut tab, picked.as_str()).expect("revert");
        assert!(!repo.path.join("b.txt").exists(), "回滚应删掉文件");
        assert!(tab.commits[0].summary.contains("Revert"));
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
