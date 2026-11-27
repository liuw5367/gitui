//! 会话层：仓库标签页（`Tab`）的结构、状态与装载/刷新/初始化这类会话级动作。

use crate::features::changes::ops::{
    apply_changelist_assignments, load_changes_checked, refresh_changes,
};
use crate::features::history::ops::{
    CommitSelectionData, load_commit_selection_from_repo, select_commit,
};
use domain::{Commit, DiffStat, StatusEntry};
use git::{GitExecutable, LogFilter, Refs, Repo};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TAB_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn next_tab_id() -> u64 {
    return NEXT_TAB_ID.fetch_add(1, Ordering::Relaxed);
}

/// 仓库打不开的原因（决定横幅给哪几个动作）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum RepoProblem {
    /// 正常。
    #[default]
    None,
    /// 路径不存在或不可读。
    NotFound,
    /// 目录在，但不是 Git 仓库。
    NotARepository,
}

/// 尚未标记解决的冲突结果。删除侧作为明确的选择保存，不能与空文件混为一谈。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ConflictDraft {
    Text(String),
    Binary(Vec<u8>),
    Delete,
}

/// 一个仓库标签页的全部界面状态。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CommitFeedbackKind {
    #[default]
    Hooks,
    Commit,
    PushRunning,
    Push,
}

/// 一条串行操作链当前进行到的步骤。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WorkflowProgress {
    /// 当前步骤，从 1 开始。
    pub(crate) step: usize,
    pub(crate) total: usize,
    /// 整条操作链仍在运行时保持为真，包括步骤切换期间。
    pub(crate) running: bool,
}

impl WorkflowProgress {
    pub(crate) fn label(self) -> Option<String> {
        if self.total <= 1 || self.step == 0 {
            return None;
        }
        return Some(format!("{}/{}", self.step, self.total));
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct StashDiff {
    pub(crate) oid: String,
    pub(crate) label: String,
    pub(crate) patch: domain::diff::Patch,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Tab {
    /// 运行期标签身份。重排和关闭确认使用它，不依赖可变位置或路径。
    pub(crate) id: u64,
    /// 每次装载请求的身份。关闭后同路径重开时，旧任务不能覆盖新标签。
    pub(crate) load_id: u64,
    pub(crate) path: String,
    /// 该仓库当前使用的 Git executable；在途操作的 Repo 会保留自己的快照。
    pub(crate) git_executable: GitExecutable,
    pub(crate) name: String,
    /// 仓库打不开的原因；正常时为 `None`。
    pub(crate) problem: RepoProblem,
    pub(crate) branch: String,
    pub(crate) ahead: u32,
    pub(crate) behind: u32,
    /// 读取 HEAD 的真实提交 OID；分支名相同也能识别外部提交或 reset。
    pub(crate) head_oid: Option<String>,
    /// 所有非符号引用及其 OID；检测非当前分支推进后再决定是否读取 History。
    pub(crate) ref_oids: BTreeMap<String, String>,
    /// 刚创建的本地引用及其指向的提交，用于增量更新 History 徽标。
    pub(crate) new_history_refs: BTreeMap<String, String>,
    pub(crate) commits: Vec<Commit>,
    /// Ancestors retained to reconnect graph lanes after non-branch filters hide commits.
    pub(crate) graph_commits: Vec<Commit>,
    /// 普通 History 浏览只显示提交图；右键菜单可临时切到提交补丁。
    pub(crate) show_commit_diff: bool,
    pub(crate) refs: Refs,
    pub(crate) remote_tags: BTreeMap<String, Vec<git::RemoteTag>>,
    /// 侧栏分组与引用树的折叠键，只在对应仓库标签的运行期保存。
    pub(crate) sidebar_collapsed: BTreeSet<String>,
    pub(crate) status: Vec<StatusEntry>,
    /// 历史里出现过的作者（按最近提交顺序）。
    pub(crate) authors: Vec<git::Author>,
    /// 当前生效的筛选条件。
    pub(crate) filter: LogFilter,
    /// 选中的提交下标；空仓库时为 `None`。
    pub(crate) selected: Option<usize>,
    /// 当前显式显示的提交或比较补丁；导出 patch 与复制用 `patch.raw`（原始文本）。
    pub(crate) patch: domain::diff::Patch,
    /// 选中的提交改动的文件统计。
    pub(crate) files: Vec<DiffStat>,
    /// 选中的文件下标（`files`），用于单文件 diff。
    pub(crate) selected_file: Option<usize>,
    /// 单文件补丁：(路径, 补丁)。原始文本在 `Patch::raw` 里。
    pub(crate) file_patch: Option<(String, domain::diff::Patch)>,
    /// 多选对比的提交下标（升序无关，按点击顺序）。
    pub(crate) compare: Vec<usize>,
    /// 对比模式：开启后点击提交是加入/移出对比集合。
    pub(crate) comparing: bool,
    /// 工作区与暂存区的变更（设计板 10 的列表）。
    pub(crate) changes: Vec<ChangeEntry>,
    /// 当前正在检查的贮藏补丁；仅用于 Refs 侧栏的 Show Diff。
    pub(crate) stash_diff: Option<StashDiff>,
    /// 选中的变更下标。
    pub(crate) selected_change: Option<usize>,
    /// 选中变更的 diff（加载期解析好，渲染期不再解析）。
    pub(crate) change_diff: domain::diff::Patch,
    /// 选中变更每个 diff hunk 是否仍包含暂存改动。
    pub(crate) change_hunk_staged: Vec<bool>,
    /// 变更清单（本地存储，按仓库分组）。
    pub(crate) changelists: Vec<store::Changelist>,
    /// 当前清单名；`None` 表示默认清单。
    pub(crate) active_changelist: Option<String>,
    /// 进行中的多步操作（合并/变基/拣选/回滚）。
    pub(crate) operation: git::Operation,
    /// 冲突文件路径。
    pub(crate) conflicts: Vec<String>,
    /// 当前多步操作期间出现过的全部冲突文件，包含已经解决的路径。
    pub(crate) conflict_paths: Vec<String>,
    /// 已解决文件的阶段快照与最终结果，供 Files 列表回看。
    pub(crate) resolved_conflicts: BTreeMap<String, ResolvedConflict>,
    /// 最近一次同步（fetch/pull/push）的结果；`None` 表示还没同步过。
    pub(crate) sync_message: Option<String>,
    /// 最近一次同步是否失败。
    pub(crate) sync_failed: bool,
    /// 选中提交的完整元信息（Info 页）。
    pub(crate) details: Option<git::CommitDetails>,
    /// 选中文件的 blame 行。
    pub(crate) blame: Vec<git::BlameLine>,
    /// 当前 blame 的文件路径（重命名以前会指向历史路径）。
    pub(crate) blame_path: String,
    /// blame 的逻辑 revision；`HEAD` 表示从工作区打开。
    pub(crate) blame_revision: String,
    /// 打开 blame 前所在的主区，历史 revision 切换后仍按原入口返回。
    pub(crate) blame_return_mode: Option<crate::Mode>,
    /// 当前聚焦的文件历史提交，点 blame 行时更新但不改代码 revision。
    pub(crate) blame_focus_sha: Option<String>,
    /// 当前点选的 blame 行；只高亮这一行，提交聚焦由 `blame_focus_sha` 表示。
    pub(crate) blame_selected_line: Option<u32>,
    /// 是否在 blame 页内显示当前 revision 的文件 patch。
    pub(crate) blame_show_diff: bool,
    pub(crate) blame_patch: domain::diff::Patch,
    /// 当前 blame 失败时的可读原因。
    pub(crate) blame_error: Option<String>,
    /// 文件历史项含该提交时实际存在的路径。
    pub(crate) file_history: Vec<git::FileHistoryEntry>,
    /// 对比引用的起点（`None` 表示未开启 Compare refs）。
    pub(crate) compare_base: Option<String>,
    /// 对比引用的终点。
    pub(crate) compare_target: Option<String>,
    /// 对比 tree 时是否采用 merge-base 到右端的三点语义。
    pub(crate) compare_three_dot: bool,
    /// 对比右端引用与当前工作区（包括暂存和未暂存的 tracked 变更）。
    pub(crate) compare_worktree: bool,
    /// 提交说明的两段文本。
    pub(crate) summary: String,
    pub(crate) description: String,
    /// 最近一次提交或提交后推送的反馈；`None` 表示还没有结果。
    pub(crate) hook_log: Option<String>,
    /// 最近一次提交或提交后推送是否成功。
    pub(crate) hook_ok: bool,
    /// 反馈属于钩子、提交、提交后推送进行中或推送结果。
    pub(crate) commit_feedback_kind: CommitFeedbackKind,
    /// 提交区自动推送期间的两步进度。
    pub(crate) commit_feedback_progress: Option<WorkflowProgress>,
    /// 提交选项：成功后 amend、sign-off 与 no-verify 归位。
    pub(crate) amend: bool,
    pub(crate) signoff: bool,
    pub(crate) no_verify: bool,
    pub(crate) push_after_commit: bool,
    /// 提交后推送时是否允许使用 `--force-with-lease`。
    pub(crate) force_push_after_commit: bool,
    /// 钩子完整日志弹层是否打开。
    pub(crate) hook_open: bool,
    /// 仓库数据是否已完成首次装载；会话恢复时可先创建延迟装载的标签。
    pub(crate) loaded: bool,
    /// 正在后台读取（设计板 16 的 loading 状态）。
    pub(crate) loading: bool,
    /// HEAD 形态：分支 / 游离 / 尚未提交（设计板 16）。
    pub(crate) head: git::HeadState,
    /// 最近一次同步失败的归类（offline / auth / rejected），猜不出来时为空。
    pub(crate) sync_fault: Option<domain::NetworkFault>,
    /// 各远端的 URL（`remote -v`），用于展示与「复制 URL」。
    pub(crate) remote_urls: Vec<(String, String)>,
    /// 冲突视图：当前查看的冲突文件下标与是否打开。
    pub(crate) conflict_index: usize,
    pub(crate) conflict_open: bool,
    /// 冲突文件当前内容（含标记）。
    pub(crate) conflict_text: String,
    /// 当前冲突路径在 index stage 1/2/3 中的原始内容。
    pub(crate) conflict_sides: Option<domain::conflict::ConflictSides>,
    /// 当前工作区冲突内容是否为二进制，二进制 Result 必须先选择一侧。
    pub(crate) conflict_binary: bool,
    /// 按仓库路径保存的未解决结果，切换文件或关闭视图不会丢失。
    pub(crate) conflict_drafts: BTreeMap<String, ConflictDraft>,
    /// 打开失败时的错误详情；成功时为 `None`。
    pub(crate) error: Option<String>,
}

/// 已解决冲突的只读快照；只在多步 Git 操作仍进行时保留。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedConflict {
    pub(crate) sides: Option<domain::conflict::ConflictSides>,
    pub(crate) result: ConflictDraft,
}

/// 一条工作区变更。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ChangeEntry {
    pub(crate) path: String,
    /// 暂存区状态字符（`M`/`A`/`D`/`R`/`?`）。
    pub(crate) index: char,
    /// 工作区状态字符。
    pub(crate) worktree: char,
    /// 是否已在暂存区。
    pub(crate) staged: bool,
    /// 是否为未跟踪文件。
    pub(crate) untracked: bool,
    pub(crate) additions: u32,
    pub(crate) deletions: u32,
    /// 所属变更清单；`None` 表示默认清单。
    pub(crate) changelist: Option<String>,
}

impl ChangeEntry {
    pub(crate) fn has_unstaged_changes(&self) -> bool {
        return self.untracked || self.worktree != ' ';
    }

    pub(crate) fn is_fully_staged(&self) -> bool {
        return self.staged && !self.has_unstaged_changes();
    }

    pub(crate) fn is_partially_staged(&self) -> bool {
        return self.staged && self.has_unstaged_changes();
    }

    /// 设计稿里显示的状态字母。
    pub(crate) fn status_letter(&self) -> char {
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
}

/// 文件树的两项显示偏好（设计板 23）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TreeOptions {
    /// 合并空目录：只含子目录、自己不含文件的层级不单独占一行。
    pub(crate) merge_empty_dirs: bool,
    /// 压缩单子目录：`a/b/c` 这种链式目录合并成一行显示。
    pub(crate) compact_single_child: bool,
}

impl TreeOptions {
    pub(crate) fn from_settings(settings: &store::Settings) -> TreeOptions {
        return TreeOptions {
            merge_empty_dirs: settings.merge_empty_dirs,
            compact_single_child: settings.compact_single_child,
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
    pub(crate) fn open_repo(&self) -> Result<Repo, domain::GitError> {
        return Repo::open_with_executable(self.path.as_str(), &self.git_executable);
    }

    /// 当前选中的提交。
    pub(crate) fn selected_commit(&self) -> Option<&Commit> {
        return self.selected.and_then(|index| self.commits.get(index));
    }

    /// 名字前的短标识：仓库名。
    pub(crate) fn label(&self) -> String {
        return if self.name.is_empty() {
            self.path.clone()
        } else {
            self.name.clone()
        };
    }

    /// 对比选区的提交索引：`(较新, 较旧)`，列表按新到旧排列。
    pub(crate) fn compare_range(&self) -> Option<(usize, usize)> {
        if self.compare.len() < 2 {
            return None;
        }
        let mut indices = self.compare.clone();
        indices.sort_unstable();
        return Some((indices[0], indices[indices.len() - 1]));
    }
}

/// 打开 `path` 并按 `filter` 装载数据；失败时返回带 `error` 的空标签页。
#[cfg(test)]
pub(crate) fn load_tab(path: &str, filter: &LogFilter) -> Tab {
    return load_tab_with_executable(path, filter, &GitExecutable::system());
}

pub(crate) fn load_tab_with_executable(
    path: &str,
    filter: &LogFilter,
    executable: &GitExecutable,
) -> Tab {
    let started = std::time::Instant::now();
    log::debug!(
        target: "gitui::history",
        "event=repo_load.start query_active={} author_filter={} author_count={} reference_filter={} reference_count={} path_filter={} message_filter={} date_filter={}",
        !filter.query.is_empty(),
        filter.author.is_some(),
        filter.authors.len(),
        filter.reference.is_some(),
        filter.references.len(),
        filter.path.is_some(),
        !filter.message.is_empty(),
        filter.since.is_some() || filter.until.is_some(),
    );
    let repo = match Repo::open_with_executable(path, executable) {
        Ok(repo) => repo,
        Err(error) => {
            log::warn!(
                target: "gitui::history",
                "event=repo_load.open_failed reason={}",
                error.summary()
            );
            let problem = match &error {
                domain::GitError::NotARepository { .. } => RepoProblem::NotARepository,
                domain::GitError::Unreadable { .. } => RepoProblem::NotFound,
                _ => RepoProblem::None,
            };
            return Tab {
                id: next_tab_id(),
                load_id: next_tab_id(),
                path: path.to_string(),
                name: last_segment(path),
                filter: filter.clone(),
                git_executable: executable.clone(),
                error: Some(error.detail()),
                problem,
                loaded: true,
                ..Tab::default()
            };
        }
    };
    let summary = repo.summary().unwrap_or_default();
    let log_started = std::time::Instant::now();
    let mut commits = repo
        .log_history(filter)
        .inspect(|commits| {
            log::debug!(
                target: "gitui::history",
                "event=repo_load.history_ok commits={} elapsed_ms={}",
                commits.len(),
                log_started.elapsed().as_millis(),
            );
        })
        .inspect_err(|error| {
            log::warn!(
                target: "gitui::history",
                "event=repo_load.history_failed reason={} elapsed_ms={}",
                error.summary(),
                log_started.elapsed().as_millis(),
            );
        })
        .unwrap_or_default();
    let graph_commits = if filter.query.is_empty()
        && filter.author.is_none()
        && filter.authors.is_empty()
        && filter.since.is_none()
        && filter.until.is_none()
        && filter.path.is_none()
        && filter.message.is_empty()
    {
        commits.clone()
    } else {
        let graph_started = std::time::Instant::now();
        repo.log_history_graph(filter)
            .inspect(|graph_commits| {
                log::debug!(
                    target: "gitui::history",
                    "event=repo_load.graph_ok commits={} elapsed_ms={}",
                    graph_commits.len(),
                    graph_started.elapsed().as_millis(),
                );
            })
            .inspect_err(|error| {
                log::warn!(
                    target: "gitui::history",
                    "event=repo_load.graph_failed reason={} elapsed_ms={}",
                    error.summary(),
                    graph_started.elapsed().as_millis(),
                );
            })
            .unwrap_or_else(|_| commits.clone())
    };
    let refs = repo.refs().unwrap_or_default();
    let ref_oids = repo.ref_oid_snapshot().unwrap_or_default();
    let branch_graph_commits = if filter.reference.is_some() || !filter.references.is_empty() {
        repo.log_all().unwrap_or_else(|_| graph_commits.clone())
    } else {
        graph_commits.clone()
    };
    git::decorate_commit_branch_history(&mut commits, &branch_graph_commits, &refs);
    let (status, changes) = load_changes_checked(&repo).unwrap_or_default();
    let authors = repo.authors_with_email().unwrap_or_default();
    let operation = repo.operation();
    let conflicts = repo.conflicts().unwrap_or_default();
    let remote_urls = repo.remote_urls().unwrap_or_default();
    let head = repo.head_state();
    let sidebar_collapsed = initial_sidebar_collapsed(&refs);
    // 存储键统一用仓库根路径：入参可能是符号链接形式（macOS 的 /tmp → /private/tmp），
    // 与写入时用的 `repo.root()` 不一致会读不到清单。
    let changelists = store::load_changelists(
        &store::default_dir(crate::brand::STORAGE_DIR),
        repo.root().display().to_string().as_str(),
    );
    let mut tab = Tab {
        id: next_tab_id(),
        load_id: next_tab_id(),
        path: repo.root().display().to_string(),
        git_executable: executable.clone(),
        name: summary.name,
        branch: summary.branch,
        ahead: summary.ahead,
        behind: summary.behind,
        head_oid: repo.head_oid(),
        ref_oids,
        commits,
        graph_commits,
        refs,
        sidebar_collapsed,
        status,
        authors,
        changes,
        operation,
        conflicts,
        head,
        remote_urls,
        changelists,
        filter: filter.clone(),
        loaded: true,
        ..Tab::default()
    };
    apply_changelist_assignments(&mut tab);
    if tab.operation != git::Operation::None {
        tab.conflict_paths = tab.conflicts.clone();
    }
    if !tab.commits.is_empty() {
        select_commit(&mut tab, 0);
    }
    log::debug!(
        target: "gitui::history",
        "event=repo_load.complete commits={} graph_commits={} refs={} authors={} changes={} elapsed_ms={}",
        tab.commits.len(),
        tab.graph_commits.len(),
        tab.refs.branches.len() + tab.refs.remote_branches.len() + tab.refs.tags.len(),
        tab.authors.len(),
        tab.changes.len(),
        started.elapsed().as_millis(),
    );
    return tab;
}

/// 新加载的仓库标签默认折叠内部引用组与 Tags；外层 Branches、Remotes 保持展开。
fn initial_sidebar_collapsed(refs: &Refs) -> BTreeSet<String> {
    let branch_names = refs
        .branches
        .iter()
        .map(|branch| branch.name.as_str())
        .collect::<BTreeSet<_>>();
    let mut collapsed = BTreeSet::from(["group:tags".to_string()]);
    for branch in &refs.branches {
        let mut prefix = String::new();
        let mut segments = branch.name.split('/').peekable();
        while let Some(segment) = segments.next() {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(segment);
            // 前缀本身若是实际分支，会渲染为分支行而非可折叠组。
            if segments.peek().is_some() && !branch_names.contains(prefix.as_str()) {
                collapsed.insert(format!("branch-prefix:{prefix}"));
            }
        }
    }
    if refs.remote_info.is_empty() {
        for remote in &refs.remotes {
            collapsed.insert(format!("remote:{remote}"));
        }
    } else {
        for remote in &refs.remote_info {
            collapsed.insert(format!("remote:{}", remote.name));
        }
    }
    for branch in &refs.remote_branches {
        let mut prefix = String::new();
        let segments = branch.name.split('/').collect::<Vec<_>>();
        for segment in segments.iter().take(segments.len().saturating_sub(1)) {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(segment);
            collapsed.insert(format!("remote-prefix:{}:{prefix}", branch.remote));
        }
    }
    return collapsed;
}

/// 生成一个尚未启动装载任务的占位标签页：只有路径与名称，列表都是空的。
/// `loading` 只在后台任务实际启动时设置，避免占位页被误判成已有任务。
pub(crate) fn placeholder_tab(path: &str, filter: &LogFilter) -> Tab {
    return Tab {
        id: next_tab_id(),
        load_id: next_tab_id(),
        path: path.to_string(),
        name: last_segment(path),
        loading: false,
        filter: filter.clone(),
        branch: "…".to_string(),
        ..Tab::default()
    };
}

/// 会话恢复时先保留标签身份与路径，首次激活后再在后台读取仓库。
pub(crate) fn deferred_tab(path: &str, filter: &LogFilter) -> Tab {
    let mut tab = placeholder_tab(path, filter);
    tab.loading = false;
    return tab;
}

/// 重新读一遍同一个仓库（初始化后、从别处改过仓库后用）。
pub(crate) fn reload(tab: &mut Tab) {
    let id = tab.id;
    let load_id = next_tab_id();
    let sidebar_collapsed = tab.sidebar_collapsed.clone();
    let remote_tags = std::mem::take(&mut tab.remote_tags);
    let path = tab.path.clone();
    let filter = tab.filter.clone();
    let executable = tab.git_executable.clone();
    *tab = load_tab_with_executable(path.as_str(), &filter, &executable);
    tab.id = id;
    tab.load_id = load_id;
    tab.sidebar_collapsed = sidebar_collapsed;
    tab.remote_tags = remote_tags;
}

/// 后台仓库校验的结果；只含 Git 数据，不包含提交草稿、筛选与布局状态。
pub(crate) struct RefreshSnapshot {
    summary: domain::RepoSummary,
    status: Vec<StatusEntry>,
    changes: Vec<ChangeEntry>,
    selected_change: Option<(Option<usize>, domain::diff::Patch, Vec<bool>, String)>,
    operation: git::Operation,
    conflicts: Vec<String>,
    conflict_editor: Option<ConflictEditorRefresh>,
    refs: Refs,
    ref_oids: BTreeMap<String, String>,
    head: git::HeadState,
    head_oid: Option<String>,
    remote_urls: Vec<(String, String)>,
    history: Option<HistoryRefresh>,
    blame: Option<BlameRefresh>,
}

/// 刷新所需的轻量输入，不复制标签中缓存的完整提交历史、diff 和 blame 数据。
#[derive(Clone)]
pub(crate) struct RefreshInput {
    path: String,
    git_executable: GitExecutable,
    changelists: Vec<store::Changelist>,
    selected_change_path: Option<String>,
    selected_change_index: Option<usize>,
    refs: Refs,
    ref_oids: BTreeMap<String, String>,
    head: git::HeadState,
    head_oid: Option<String>,
    filter: LogFilter,
    selected_commit_sha: Option<String>,
    selected_index: Option<usize>,
    compare_shas: Vec<String>,
    show_commit_diff: bool,
    operation: git::Operation,
    conflict_open: bool,
    selected_conflict_path: Option<String>,
    blame_path: String,
    blame_revision: String,
    blame_return_mode: Option<crate::Mode>,
    blame_focus_sha: Option<String>,
    blame_selected_line: Option<u32>,
}

impl RefreshInput {
    pub(crate) fn from_tab(tab: &Tab) -> RefreshInput {
        let selected_change_path = tab
            .selected_change
            .and_then(|index| tab.changes.get(index))
            .map(|change| change.path.clone());
        let selected_commit_sha = tab.selected_commit().map(|commit| commit.sha.clone());
        let compare_shas = tab
            .compare
            .iter()
            .filter_map(|index| tab.commits.get(*index).map(|commit| commit.sha.clone()))
            .collect();
        let selected_conflict_path = tab
            .conflict_paths
            .get(tab.conflict_index)
            .filter(|path| {
                tab.conflicts.contains(path) || tab.resolved_conflicts.contains_key(*path)
            })
            .cloned();
        return RefreshInput {
            path: tab.path.clone(),
            git_executable: tab.git_executable.clone(),
            changelists: tab.changelists.clone(),
            selected_change_path,
            selected_change_index: tab.selected_change,
            refs: tab.refs.clone(),
            ref_oids: tab.ref_oids.clone(),
            head: tab.head.clone(),
            head_oid: tab.head_oid.clone(),
            filter: tab.filter.clone(),
            selected_commit_sha,
            selected_index: tab.selected,
            compare_shas,
            show_commit_diff: tab.show_commit_diff,
            operation: tab.operation,
            conflict_open: tab.conflict_open,
            selected_conflict_path,
            blame_path: tab.blame_path.clone(),
            blame_revision: tab.blame_revision.clone(),
            blame_return_mode: tab.blame_return_mode,
            blame_focus_sha: tab.blame_focus_sha.clone(),
            blame_selected_line: tab.blame_selected_line,
        };
    }
}

struct HistoryRefresh {
    commits: Vec<Commit>,
    graph_commits: Vec<Commit>,
    authors: Vec<git::Author>,
    selected: Option<usize>,
    selection: Option<CommitSelectionData>,
    compare: Vec<usize>,
    show_commit_patch: Option<domain::diff::Patch>,
}

struct BlameRefresh {
    blame: Vec<git::BlameLine>,
    blame_path: String,
    blame_revision: String,
    blame_return_mode: Option<crate::Mode>,
    blame_focus_sha: Option<String>,
    blame_selected_line: Option<u32>,
    blame_patch: domain::diff::Patch,
    blame_error: Option<String>,
    file_history: Vec<git::FileHistoryEntry>,
}

struct ConflictEditorRefresh {
    path: String,
    text: String,
    sides: Option<domain::conflict::ConflictSides>,
    binary: bool,
}

/// 在后台读取当前仓库状态。任一必要查询失败时保留整个上次成功快照。
pub(crate) fn read_refresh_snapshot(
    input: &RefreshInput,
    mode: crate::Mode,
) -> Result<RefreshSnapshot, String> {
    let repo = Repo::open_with_executable(input.path.as_str(), &input.git_executable)
        .map_err(|error| error.detail())?;
    let summary = repo.summary().map_err(|error| error.detail())?;
    let refs = repo.refs().map_err(|error| error.detail())?;
    let ref_oids = repo.ref_oid_snapshot().map_err(|error| error.detail())?;
    let (status, mut changes) = load_changes_checked(&repo).map_err(|error| error.detail())?;
    for change in &mut changes {
        change.changelist = input
            .changelists
            .iter()
            .find(|list| list.paths.iter().any(|path| path == &change.path))
            .map(|list| list.name.clone());
    }
    let selected_change = (mode == crate::Mode::Changes)
        .then_some(input.selected_change_index)
        .flatten()
        .map(|old_index| -> Result<_, String> {
            let index = input
                .selected_change_path
                .as_deref()
                .and_then(|path| changes.iter().position(|change| change.path == path))
                .or_else(|| {
                    (!changes.is_empty()).then_some(old_index.min(changes.len().saturating_sub(1)))
                });
            let (patch, staged_hunks, staged_patch) =
                match index.and_then(|index| changes.get(index)) {
                    Some(change) => {
                        crate::features::changes::ops::read_change_diff(&repo, change, false)
                            .map_err(|error| error.detail())?
                    }
                    _ => (domain::diff::Patch::default(), Vec::new(), String::new()),
                };
            return Ok((index, patch, staged_hunks, staged_patch));
        });
    let selected_change = match selected_change {
        Some(result) => Some(result?),
        None => None,
    };
    let operation = repo.operation();
    let conflicts = repo.conflicts().map_err(|error| error.detail())?;
    let conflict_editor = refreshed_conflict_path(input, operation, &conflicts)
        .map(|path| -> Result<_, String> {
            let worktree = repo
                .conflict_worktree_content(path.as_str())
                .map_err(|error| error.detail())?;
            let sides = repo
                .conflict_sides(path.as_str())
                .map_err(|error| error.detail())?;
            let binary = worktree
                .as_deref()
                .is_some_and(|bytes| bytes.contains(&0) || std::str::from_utf8(bytes).is_err());
            let text = worktree
                .as_deref()
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .unwrap_or_default()
                .to_string();
            return Ok(ConflictEditorRefresh {
                path: path.to_string(),
                text,
                sides: Some(sides),
                binary,
            });
        })
        .transpose()?;
    let head = repo.head_state();
    let head_oid = repo.head_oid();
    let remote_urls = repo.remote_urls().map_err(|error| error.detail())?;
    let history = if head != input.head
        || head_oid != input.head_oid
        || refs != input.refs
        || ref_oids != input.ref_oids
    {
        Some(read_history_refresh(input, &repo, &refs)?)
    } else {
        None
    };
    let blame = if mode == crate::Mode::Blame
        && input.blame_revision == "HEAD"
        && !input.blame_path.is_empty()
    {
        let mut refreshed = Tab {
            path: input.path.clone(),
            git_executable: input.git_executable.clone(),
            ..Tab::default()
        };
        crate::features::history::ops::open_blame(
            &mut refreshed,
            input.blame_path.as_str(),
            "HEAD",
            input.blame_return_mode.unwrap_or(crate::Mode::Changes),
        );
        if let Some(focused) = input.blame_focus_sha.as_ref().filter(|sha| {
            refreshed
                .file_history
                .iter()
                .any(|entry| &entry.commit.sha == *sha)
        }) {
            refreshed.blame_focus_sha = Some(focused.clone());
        }
        if let Some(line) = input
            .blame_selected_line
            .filter(|line| *line > 0 && *line as usize <= refreshed.blame.len())
        {
            refreshed.blame_selected_line = Some(line);
        }
        Some(BlameRefresh {
            blame: refreshed.blame,
            blame_path: refreshed.blame_path,
            blame_revision: refreshed.blame_revision,
            blame_return_mode: refreshed.blame_return_mode,
            blame_focus_sha: refreshed.blame_focus_sha,
            blame_selected_line: refreshed.blame_selected_line,
            blame_patch: refreshed.blame_patch,
            blame_error: refreshed.blame_error,
            file_history: refreshed.file_history,
        })
    } else {
        None
    };
    return Ok(RefreshSnapshot {
        summary,
        status,
        changes,
        selected_change,
        operation,
        conflicts,
        conflict_editor,
        refs,
        ref_oids,
        head,
        head_oid,
        remote_urls,
        history,
        blame,
    });
}

/// 把刷新时自动清掉的 hunk 暂存状态同步回 Git index。
pub(crate) fn reconcile_refresh_snapshot(
    input: &RefreshInput,
    mode: crate::Mode,
    snapshot: RefreshSnapshot,
) -> Result<RefreshSnapshot, String> {
    if mode != crate::Mode::Changes {
        return Ok(snapshot);
    }
    let Some(path) = input.selected_change_path.as_deref() else {
        return Ok(snapshot);
    };
    let Some((Some(index), patch, _, staged_patch)) = snapshot.selected_change.as_ref() else {
        return Ok(snapshot);
    };
    if snapshot
        .changes
        .get(*index)
        .is_none_or(|change| change.path != path || change.untracked)
    {
        return Ok(snapshot);
    }
    let repo = Repo::open_with_executable(input.path.as_str(), &input.git_executable)
        .map_err(|error| error.detail())?;
    let changed = crate::features::changes::ops::unstage_hunks_missing_from_worktree(
        &repo,
        patch.raw.as_str(),
        staged_patch.as_str(),
    )
    .map_err(|error| error.detail())?;
    if !changed {
        return Ok(snapshot);
    }
    return read_refresh_snapshot(input, mode);
}

fn refreshed_conflict_path(
    input: &RefreshInput,
    operation: git::Operation,
    conflicts: &[String],
) -> Option<String> {
    if !input.conflict_open || operation == git::Operation::None {
        return None;
    }
    if operation != input.operation {
        return conflicts.first().cloned();
    }
    return input
        .selected_conflict_path
        .clone()
        .or_else(|| conflicts.first().cloned());
}

fn read_history_refresh(
    input: &RefreshInput,
    repo: &Repo,
    refs: &Refs,
) -> Result<HistoryRefresh, String> {
    let mut commits = repo
        .log_history(&input.filter)
        .map_err(|error| error.detail())?;
    let graph_commits = if input.filter.query.is_empty()
        && input.filter.author.is_none()
        && input.filter.authors.is_empty()
        && input.filter.since.is_none()
        && input.filter.until.is_none()
        && input.filter.path.is_none()
        && input.filter.message.is_empty()
    {
        commits.clone()
    } else {
        repo.log_history_graph(&input.filter)
            .map_err(|error| error.detail())?
    };
    let branch_graph_commits =
        if input.filter.reference.is_some() || !input.filter.references.is_empty() {
            repo.log_all().map_err(|error| error.detail())?
        } else {
            graph_commits.clone()
        };
    git::decorate_commit_branch_history(&mut commits, &branch_graph_commits, refs);

    let previous_selected_sha = input.selected_commit_sha.clone();
    let selected = previous_selected_sha
        .as_ref()
        .and_then(|sha| commits.iter().position(|commit| &commit.sha == sha))
        .or_else(|| {
            (!commits.is_empty()).then(|| {
                input
                    .selected_index
                    .unwrap_or_default()
                    .min(commits.len().saturating_sub(1))
            })
        });
    let next_selected_sha =
        selected.and_then(|index| commits.get(index).map(|commit| commit.sha.clone()));
    let selection = selected
        .and_then(|index| commits.get(index))
        .map(|commit| load_commit_selection_from_repo(repo, &commit.sha, &commit.short))
        .transpose()?;
    let compare = input
        .compare_shas
        .iter()
        .filter_map(|sha| commits.iter().position(|commit| commit.sha == *sha))
        .collect();
    let show_commit_patch = if input.show_commit_diff && previous_selected_sha != next_selected_sha
    {
        Some(match next_selected_sha.as_deref() {
            Some(sha) => repo.patch(sha).map_err(|error| error.detail())?,
            None => domain::diff::Patch::default(),
        })
    } else {
        None
    };
    let authors = repo.authors_with_email().map_err(|error| error.detail())?;
    return Ok(HistoryRefresh {
        commits,
        graph_commits,
        authors,
        selected,
        selection,
        compare,
        show_commit_patch,
    });
}

/// 只替换变化的仓库字段，保留草稿、筛选、折叠和其余交互状态。
pub(crate) fn apply_refresh_snapshot(tab: &mut Tab, snapshot: RefreshSnapshot) -> bool {
    let mut changed = false;
    changed |= replace_if_changed(&mut tab.name, snapshot.summary.name);
    changed |= replace_if_changed(&mut tab.branch, snapshot.summary.branch);
    changed |= replace_if_changed(&mut tab.ahead, snapshot.summary.ahead);
    changed |= replace_if_changed(&mut tab.behind, snapshot.summary.behind);
    changed |= replace_if_changed(&mut tab.status, snapshot.status);
    changed |= replace_if_changed(&mut tab.changes, snapshot.changes);
    changed |= replace_if_changed(&mut tab.refs, snapshot.refs);
    changed |= replace_if_changed(&mut tab.ref_oids, snapshot.ref_oids);
    changed |= replace_if_changed(&mut tab.head, snapshot.head);
    changed |= replace_if_changed(&mut tab.head_oid, snapshot.head_oid);
    changed |= replace_if_changed(&mut tab.remote_urls, snapshot.remote_urls);
    if let Some((selected, patch, staged_hunks, _)) = snapshot.selected_change {
        changed |= replace_if_changed(&mut tab.selected_change, selected);
        changed |= replace_if_changed(&mut tab.change_diff, patch);
        changed |= replace_if_changed(&mut tab.change_hunk_staged, staged_hunks);
    }
    changed |= apply_operation_refresh(tab, snapshot.operation, snapshot.conflicts);
    if let Some(editor) = snapshot.conflict_editor
        && tab.conflict_open
        && tab.operation != git::Operation::None
        && tab.conflict_paths.get(tab.conflict_index) == Some(&editor.path)
        && !tab.conflict_drafts.contains_key(&editor.path)
        && !tab.resolved_conflicts.contains_key(&editor.path)
    {
        changed |= replace_if_changed(&mut tab.conflict_text, editor.text);
        changed |= replace_if_changed(&mut tab.conflict_sides, editor.sides);
        changed |= replace_if_changed(&mut tab.conflict_binary, editor.binary);
    }
    if let Some(history) = snapshot.history {
        changed |= apply_history_refresh(tab, history);
    }
    if let Some(blame) = snapshot.blame {
        changed |= apply_blame_refresh(tab, blame);
    }
    return changed;
}

fn apply_history_refresh(tab: &mut Tab, history: HistoryRefresh) -> bool {
    let old_selected_sha = tab.selected_commit().map(|commit| commit.sha.clone());
    let next_selected_sha = history
        .selected
        .and_then(|index| history.commits.get(index).map(|commit| commit.sha.clone()));
    let selection_changed = old_selected_sha != next_selected_sha;
    let old_file_path = tab
        .selected_file
        .and_then(|index| tab.files.get(index).map(|file| file.path.clone()));
    let mut changed = false;
    changed |= replace_if_changed(&mut tab.commits, history.commits);
    changed |= replace_if_changed(&mut tab.graph_commits, history.graph_commits);
    changed |= replace_if_changed(&mut tab.authors, history.authors);
    changed |= replace_if_changed(&mut tab.selected, history.selected);
    changed |= replace_if_changed(&mut tab.compare, history.compare);
    if let Some(selection) = history.selection {
        changed |= replace_if_changed(&mut tab.files, selection.files);
        changed |= replace_if_changed(&mut tab.details, selection.details);
    }
    if selection_changed {
        changed |= replace_if_changed(&mut tab.selected_file, None);
        changed |= replace_if_changed(&mut tab.file_patch, None);
    } else if let Some(path) = old_file_path {
        let selected_file = tab.files.iter().position(|file| file.path == path);
        changed |= replace_if_changed(&mut tab.selected_file, selected_file);
        if selected_file.is_none() {
            changed |= replace_if_changed(&mut tab.file_patch, None);
        }
    }
    if let Some(patch) = history.show_commit_patch {
        changed |= replace_if_changed(&mut tab.patch, patch);
    }
    if tab.selected.is_none() && tab.show_commit_diff {
        tab.show_commit_diff = false;
        changed = true;
    }
    return changed;
}

fn apply_operation_refresh(
    tab: &mut Tab,
    operation: git::Operation,
    conflicts: Vec<String>,
) -> bool {
    let operation_changed = tab.operation != operation;
    let conflicts_changed = tab.conflicts != conflicts;
    if !operation_changed && !conflicts_changed {
        return false;
    }
    let selected_path = tab.conflict_paths.get(tab.conflict_index).cloned();
    tab.operation = operation;
    tab.conflicts = conflicts;
    if operation == git::Operation::None || operation_changed {
        tab.conflict_paths = tab.conflicts.clone();
        tab.resolved_conflicts.clear();
        tab.conflict_drafts.clear();
        tab.conflict_index = 0;
        if operation == git::Operation::None {
            tab.conflict_open = false;
            tab.conflict_text.clear();
            tab.conflict_sides = None;
            tab.conflict_binary = false;
        }
        return true;
    }
    for path in &tab.conflicts {
        tab.resolved_conflicts.remove(path);
        if !tab.conflict_paths.contains(path) {
            tab.conflict_paths.push(path.clone());
        }
    }
    tab.resolved_conflicts
        .retain(|path, _| !tab.conflicts.contains(path));
    tab.conflict_drafts
        .retain(|path, _| tab.conflicts.contains(path));
    tab.conflict_index = selected_path
        .as_ref()
        .and_then(|selected| tab.conflict_paths.iter().position(|path| path == selected))
        .or_else(|| {
            tab.conflict_paths
                .iter()
                .position(|path| tab.conflicts.contains(path))
        })
        .unwrap_or(0);
    return true;
}

fn apply_blame_refresh(tab: &mut Tab, blame: BlameRefresh) -> bool {
    let mut changed = false;
    changed |= replace_if_changed(&mut tab.blame, blame.blame);
    changed |= replace_if_changed(&mut tab.blame_path, blame.blame_path);
    changed |= replace_if_changed(&mut tab.blame_revision, blame.blame_revision);
    changed |= replace_if_changed(&mut tab.blame_return_mode, blame.blame_return_mode);
    changed |= replace_if_changed(&mut tab.blame_focus_sha, blame.blame_focus_sha);
    changed |= replace_if_changed(&mut tab.blame_selected_line, blame.blame_selected_line);
    changed |= replace_if_changed(&mut tab.blame_patch, blame.blame_patch);
    changed |= replace_if_changed(&mut tab.blame_error, blame.blame_error);
    changed |= replace_if_changed(&mut tab.file_history, blame.file_history);
    return changed;
}

fn replace_if_changed<T: PartialEq>(target: &mut T, next: T) -> bool {
    if *target == next {
        return false;
    }
    *target = next;
    return true;
}

/// 在标签页指向的目录里初始化仓库（设计板 16 的「不是仓库」动作）。
pub(crate) fn init_here(tab: &mut Tab) -> Result<(), String> {
    git::init_with_executable(
        tab.path.as_str(),
        &git::InitOptions::default(),
        &tab.git_executable,
    )
    .map_err(|error| error.detail())?;
    reload(tab);
    return Ok(());
}

/// 路径的上一级；没有上一级时返回空串。
pub(crate) fn parent_path(path: &str) -> String {
    return std::path::Path::new(path)
        .parent()
        .map(|parent| parent.display().to_string())
        .unwrap_or_default();
}

/// 历史被改写或切走后，把列表、引用与操作状态一起刷新。
pub(crate) fn reload_after_history_change(tab: &mut Tab) -> Result<String, String> {
    refresh_changes(tab);
    let filter = tab.filter.clone();
    let refreshed = load_tab_with_executable(tab.path.as_str(), &filter, &tab.git_executable);
    tab.commits = refreshed.commits;
    tab.refs = refreshed.refs;
    tab.branch = refreshed.branch.clone();
    tab.head = refreshed.head.clone();
    tab.head_oid = refreshed.head_oid.clone();
    tab.operation = refreshed.operation;
    tab.conflicts = refreshed.conflicts;
    tab.conflict_paths = if refreshed.operation == git::Operation::None {
        Vec::new()
    } else {
        tab.conflicts.clone()
    };
    tab.resolved_conflicts.clear();
    tab.ahead = refreshed.ahead;
    tab.behind = refreshed.behind;
    tab.selected = None;
    tab.conflict_index = 0;
    tab.conflict_open = false;
    tab.conflict_text.clear();
    tab.conflict_sides = None;
    tab.conflict_binary = false;
    tab.conflict_drafts.clear();
    if !tab.commits.is_empty() {
        select_commit(tab, 0);
    }
    return Ok(refreshed.branch);
}

fn last_segment(path: &str) -> String {
    return path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_string();
}

/// 同步操作。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Sync {
    Fetch,
    Pull,
    Push,
}

impl Sync {
    /// 界面上显示的动作名（同时用于文案键 `sync.<id>`）。
    pub(crate) fn id(self) -> &'static str {
        return match self {
            Sync::Fetch => "fetch",
            Sync::Pull => "pull",
            Sync::Push => "push",
        };
    }
}

/// 同步选项：三种动作共用一份，节省一个入参。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SyncOptions {
    pub(crate) fetch: git::FetchOptions,
    pub(crate) pull: git::PullOptions,
    pub(crate) push: git::PushOptions,
}

impl SyncOptions {
    /// 同步动作的选项是否含破坏性含义（强推）。
    pub(crate) fn is_destructive(&self, kind: Sync) -> bool {
        return kind == Sync::Push && self.push.force_with_lease;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempRepo;

    #[test]
    fn workflow_progress_labels_only_multi_step_flows() {
        assert_eq!(
            WorkflowProgress {
                step: 2,
                total: 3,
                running: true,
            }
            .label()
            .as_deref(),
            Some("2/3")
        );
        assert_eq!(
            WorkflowProgress {
                step: 1,
                total: 1,
                running: false,
            }
            .label(),
            None
        );
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
        assert!(tab.sidebar_collapsed.contains("group:tags"));
        assert!(!tab.sidebar_collapsed.contains("group:branches"));
        assert!(!tab.sidebar_collapsed.contains("group:remotes"));
        assert!(tab.patch.lines.is_empty(), "提交列表不预载完整 patch");
        assert_eq!(tab.files.len(), 1);
        assert_eq!(tab.authors[0].name, "Test");
    }

    #[test]
    fn new_tabs_collapse_branch_groups_remotes_and_tags_by_default() {
        let refs = Refs {
            branches: vec![
                git::Branch {
                    name: "feature".to_string(),
                    ..git::Branch::default()
                },
                git::Branch {
                    name: "feature/ui".to_string(),
                    ..git::Branch::default()
                },
                git::Branch {
                    name: "feature/deep/leaf".to_string(),
                    ..git::Branch::default()
                },
                git::Branch {
                    name: "release/v1".to_string(),
                    ..git::Branch::default()
                },
            ],
            remote_branches: vec![git::RemoteBranch {
                remote: "origin".to_string(),
                name: "feature/deep/ui".to_string(),
                ..git::RemoteBranch::default()
            }],
            remote_info: vec![
                git::RemoteInfo {
                    name: "origin".to_string(),
                    ..git::RemoteInfo::default()
                },
                git::RemoteInfo {
                    name: "upstream".to_string(),
                    ..git::RemoteInfo::default()
                },
            ],
            ..Refs::default()
        };
        let collapsed = initial_sidebar_collapsed(&refs);
        assert_eq!(
            collapsed,
            BTreeSet::from([
                "branch-prefix:feature/deep".to_string(),
                "branch-prefix:release".to_string(),
                "group:tags".to_string(),
                "remote-prefix:origin:feature".to_string(),
                "remote-prefix:origin:feature/deep".to_string(),
                "remote:origin".to_string(),
                "remote:upstream".to_string(),
            ])
        );

        let refs = Refs {
            remotes: vec!["origin".to_string()],
            ..Refs::default()
        };
        assert!(initial_sidebar_collapsed(&refs).contains("remote:origin"));
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
    fn refresh_observes_external_commit_and_keeps_user_state() {
        let repo = TempRepo::new("external-commit-refresh");
        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        let old_head = tab.head_oid.clone().expect("initial HEAD");
        tab.summary = "unsaved commit title".to_string();
        tab.description = "unsaved commit body".to_string();
        tab.sidebar_collapsed.insert("user-fold".to_string());

        std::fs::write(repo.path.join("a.txt"), "two\n").expect("write");
        run_test_git(&repo.path, &["add", "-A"]);
        run_test_git(&repo.path, &["commit", "-q", "-m", "outside commit"]);

        let snapshot = read_refresh_snapshot(&RefreshInput::from_tab(&tab), crate::Mode::Changes)
            .expect("refresh snapshot");
        assert!(apply_refresh_snapshot(&mut tab, snapshot));
        assert!(tab.changes.is_empty(), "外部提交后工作区变更应清空");
        assert_ne!(tab.head_oid.as_deref(), Some(old_head.as_str()));
        assert_eq!(tab.commits[0].summary, "outside commit");
        assert_eq!(tab.summary, "unsaved commit title");
        assert_eq!(tab.description, "unsaved commit body");
        assert!(tab.sidebar_collapsed.contains("user-fold"));

        let unchanged = read_refresh_snapshot(&RefreshInput::from_tab(&tab), crate::Mode::Changes)
            .expect("second snapshot");
        assert!(!apply_refresh_snapshot(&mut tab, unchanged));
    }

    #[test]
    fn refresh_updates_selected_change_patch_when_status_is_unchanged() {
        let repo = TempRepo::new("external-diff-refresh");
        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        tab.selected_change = Some(0);
        std::fs::write(repo.path.join("a.txt"), "two\n").expect("write");

        let first = read_refresh_snapshot(&RefreshInput::from_tab(&tab), crate::Mode::Changes)
            .expect("first snapshot");
        assert!(apply_refresh_snapshot(&mut tab, first));
        assert!(tab.change_diff.raw.contains("two"));
        let status = tab.status.clone();
        let changes = tab.changes.clone();
        let old_patch = tab.change_diff.raw.clone();

        std::fs::write(repo.path.join("a.txt"), "three\n").expect("rewrite");
        let second = read_refresh_snapshot(&RefreshInput::from_tab(&tab), crate::Mode::Changes)
            .expect("second snapshot");
        assert!(apply_refresh_snapshot(&mut tab, second));
        assert_eq!(tab.status, status);
        assert_eq!(tab.changes, changes);
        assert_ne!(tab.change_diff.raw, old_patch);
        assert!(tab.change_diff.raw.contains("three"));
    }

    #[test]
    fn refresh_unstages_hunks_that_diffview_clears_after_conflicting_edits() {
        use crate::features::changes::ops::{apply_piece, load_change_diff};

        let (repo, mut changed, mut tab) = three_hunk_change("auto-unstage-adjacent-edit");
        apply_piece(&mut tab, 0, false).expect("stage first hunk");
        let input = RefreshInput::from_tab(&tab);
        changed[2] = "line3-adjacent-edit".to_string();
        std::fs::write(repo.path.join("a.txt"), format!("{}\n", changed.join("\n")))
            .expect("edit adjacent unstaged line");
        let snapshot =
            read_refresh_snapshot(&input, crate::Mode::Changes).expect("read adjacent edit");
        assert_eq!(
            snapshot.selected_change.as_ref().expect("selected diff").2,
            vec![true, false, false],
            "unchanged staged content should remain checked beside a new unstaged edit"
        );
        let snapshot = reconcile_refresh_snapshot(&input, crate::Mode::Changes, snapshot)
            .expect("reconcile adjacent edit");
        assert!(apply_refresh_snapshot(&mut tab, snapshot));
        let staged = tab
            .open_repo()
            .expect("repo")
            .staged_file_patch_from_head_with_whitespace("a.txt", false)
            .expect("staged patch");
        assert!(staged.contains("line2-staged"));

        let (repo, mut changed, mut tab) = three_hunk_change("auto-unstage-one");
        let index = tab.selected_change.expect("selected change");
        apply_piece(&mut tab, 0, false).expect("stage first hunk");

        changed[1] = "line2-edited-after-stage".to_string();
        std::fs::write(repo.path.join("a.txt"), format!("{}\n", changed.join("\n")))
            .expect("edit staged line");
        load_change_diff(&mut tab, index);
        assert_eq!(tab.selected_change, Some(index));
        assert!(
            !tab.changes[index].staged,
            "the file tree should become unchecked when the only staged hunk is cleared"
        );
        assert_eq!(tab.change_hunk_staged, vec![false, false, false]);
        let staged = tab
            .open_repo()
            .expect("repo")
            .staged_file_patch_from_head_with_whitespace("a.txt", false)
            .expect("staged patch");
        assert!(!staged.contains("line2-staged"));
        assert!(
            staged.is_empty(),
            "the index should no longer contain the hunk"
        );

        let (repo, mut changed, mut tab) = three_hunk_change("auto-unstage-partial");
        for hunk in 0..3 {
            apply_piece(&mut tab, hunk, false).expect("stage hunk");
        }
        let input = RefreshInput::from_tab(&tab);
        changed[13] = "line14-edited-after-stage".to_string();
        std::fs::write(repo.path.join("a.txt"), format!("{}\n", changed.join("\n")))
            .expect("edit middle staged line");

        let snapshot =
            read_refresh_snapshot(&input, crate::Mode::Changes).expect("read conflicted diff");
        assert_eq!(
            snapshot.selected_change.as_ref().expect("selected diff").2,
            vec![true, false, true],
            "DiffView should clear only the edited hunk"
        );
        let snapshot = reconcile_refresh_snapshot(&input, crate::Mode::Changes, snapshot)
            .expect("reconcile cleared hunk");
        assert!(apply_refresh_snapshot(&mut tab, snapshot));
        let entry = tab
            .changes
            .iter()
            .find(|entry| entry.path == "a.txt")
            .expect("file tree entry");
        assert!(
            entry.is_partially_staged(),
            "two staged hunks should leave a half selection"
        );
        assert_eq!(tab.change_hunk_staged, vec![true, false, true]);
        let staged = tab
            .open_repo()
            .expect("repo")
            .staged_file_patch_from_head_with_whitespace("a.txt", false)
            .expect("staged patch");
        assert!(staged.contains("line2-staged"));
        assert!(!staged.contains("line14-staged"));
        assert!(staged.contains("line26-staged"));
    }

    #[test]
    fn refresh_reads_history_when_a_non_current_ref_moves() {
        let repo = TempRepo::new("non-current-ref-refresh");
        run_test_git(&repo.path, &["branch", "side"]);
        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        let original_refs = tab.refs.clone();
        let original_head = tab.head_oid.clone();

        run_test_git(&repo.path, &["checkout", "-q", "side"]);
        std::fs::write(repo.path.join("a.txt"), "side\n").expect("write");
        run_test_git(&repo.path, &["add", "-A"]);
        run_test_git(&repo.path, &["commit", "-q", "-m", "side advancement"]);
        run_test_git(&repo.path, &["checkout", "-q", "main"]);

        let snapshot = read_refresh_snapshot(&RefreshInput::from_tab(&tab), crate::Mode::History)
            .expect("refresh snapshot");
        assert_eq!(snapshot.refs, original_refs);
        assert_eq!(snapshot.head_oid, original_head);
        assert!(
            snapshot.history.is_some(),
            "引用 OID 变化应触发 History 重读"
        );
        assert!(apply_refresh_snapshot(&mut tab, snapshot));
        assert!(
            tab.commits
                .iter()
                .any(|commit| commit.summary == "side advancement")
        );
    }

    #[test]
    fn refresh_tracks_external_merge_conflicts_and_abort() {
        let repo = TempRepo::new("external-merge-refresh");
        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        run_test_git(&repo.path, &["branch", "side"]);
        run_test_git(&repo.path, &["checkout", "-q", "side"]);
        std::fs::write(repo.path.join("a.txt"), "side\n").expect("write side");
        run_test_git(&repo.path, &["add", "-A"]);
        run_test_git(&repo.path, &["commit", "-q", "-m", "side change"]);
        run_test_git(&repo.path, &["checkout", "-q", "main"]);
        std::fs::write(repo.path.join("a.txt"), "main\n").expect("write main");
        run_test_git(&repo.path, &["add", "-A"]);
        run_test_git(&repo.path, &["commit", "-q", "-m", "main change"]);
        let merge = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo.path)
            .args(["merge", "--no-edit", "side"])
            .output()
            .expect("merge");
        assert!(!merge.status.success(), "merge should stop at the conflict");

        let snapshot = read_refresh_snapshot(&RefreshInput::from_tab(&tab), crate::Mode::Changes)
            .expect("merge snapshot");
        assert_eq!(snapshot.operation, git::Operation::Merge);
        assert_eq!(snapshot.conflicts, vec!["a.txt".to_string()]);
        assert!(apply_refresh_snapshot(&mut tab, snapshot));
        assert_eq!(tab.operation, git::Operation::Merge);
        assert_eq!(tab.conflicts, vec!["a.txt".to_string()]);

        run_test_git(&repo.path, &["merge", "--abort"]);
        let snapshot = read_refresh_snapshot(&RefreshInput::from_tab(&tab), crate::Mode::Changes)
            .expect("abort snapshot");
        assert!(apply_refresh_snapshot(&mut tab, snapshot));
        assert_eq!(tab.operation, git::Operation::None);
        assert!(tab.conflicts.is_empty());
        assert!(tab.conflict_paths.is_empty());
        assert!(tab.conflict_drafts.is_empty());
    }

    fn run_test_git(path: &std::path::Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(path)
            .args(args)
            .output()
            .expect("git");
        assert!(
            output.status.success(),
            "git {:?}: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn three_hunk_change(name: &str) -> (TempRepo, Vec<String>, Tab) {
        let repo = TempRepo::new(name);
        let base: Vec<String> = (1..=30).map(|index| format!("line{index}")).collect();
        std::fs::write(repo.path.join("a.txt"), format!("{}\n", base.join("\n")))
            .expect("write base file");
        run_test_git(&repo.path, &["add", "-A"]);
        run_test_git(&repo.path, &["commit", "-q", "-m", "multi-line base"]);

        let mut changed = base;
        changed[1] = "line2-staged".to_string();
        changed[13] = "line14-staged".to_string();
        changed[25] = "line26-staged".to_string();
        std::fs::write(repo.path.join("a.txt"), format!("{}\n", changed.join("\n")))
            .expect("write changed file");

        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        let index = tab
            .changes
            .iter()
            .position(|entry| entry.path == "a.txt")
            .expect("changed file");
        crate::features::changes::ops::load_change_diff(&mut tab, index);
        assert_eq!(
            domain::stage::Sections::parse(tab.change_diff.raw.as_str()).files[0]
                .hunks
                .len(),
            3,
            "fixture should have three hunks"
        );
        return (repo, changed, tab);
    }

    #[test]
    fn placeholder_tab_waits_for_a_load_task_and_has_no_data() {
        let tab = placeholder_tab("/tmp/whatever", &LogFilter::default());
        assert!(!tab.loading, "任务启动前占位标签页不应伪装成加载中");
        assert!(!tab.loaded, "占位标签页尚未装载仓库数据");
        assert_eq!(tab.name, "whatever");
        assert!(tab.commits.is_empty());
        assert!(tab.changes.is_empty());
        assert!(tab.error.is_none(), "占位不应显示错误面板");

        // 真的装载出来的标签页不是 loading。
        let repo = TempRepo::new("loading-flag");
        let loaded = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        assert!(!loaded.loading, "装载完成后应清掉 loading");
        assert!(loaded.loaded, "装载完成后应标记仓库数据可用");
    }

    #[test]
    fn deferred_tab_is_not_loading_until_activated() {
        let tab = deferred_tab("/tmp/whatever", &LogFilter::default());
        assert!(!tab.loading);
        assert!(!tab.loaded);
        assert_eq!(tab.path, "/tmp/whatever");
        assert!(tab.error.is_none());
    }

    #[test]
    fn a_plain_folder_reports_not_a_repository_and_can_be_initialized() {
        // 独立的临时目录：放在别的仓库里会被 git 解析成上级仓库，测不到「不是仓库」。
        let folder = std::env::temp_dir().join(format!(
            "nsh-app-plain-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&folder).expect("mkdir");
        let filter = LogFilter::default();
        let mut tab = load_tab(folder.display().to_string().as_str(), &filter);
        assert_eq!(tab.problem, RepoProblem::NotARepository);
        assert!(tab.error.is_some(), "打不开时仍要留着原因给界面");
        // 在同一个目录初始化后，标签页应变成可用仓库。
        init_here(&mut tab).expect("init here");
        assert_eq!(tab.problem, RepoProblem::None);
        assert!(tab.error.is_none());
        assert!(
            tab.branch == "main" || tab.branch == "master",
            "branch: {}",
            tab.branch
        );
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_missing_path_reports_not_found() {
        let root = TempRepo::new("missing");
        let gone = root.path.join("does-not-exist");
        let filter = LogFilter::default();
        let tab = load_tab(gone.display().to_string().as_str(), &filter);
        assert_eq!(tab.problem, RepoProblem::NotFound);
        assert_eq!(
            parent_path(gone.display().to_string().as_str()),
            root.path.display().to_string()
        );
    }
}
