//! 会话层：仓库标签页（`Tab`）的结构、状态与装载/刷新/初始化这类会话级动作。

use crate::features::changes::ops::{load_changes, refresh_changes};
use crate::features::history::ops::select_commit;
use domain::{Commit, DiffStat, StatusEntry};
use git::{LogFilter, Refs, Repo};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TAB_ID: AtomicU64 = AtomicU64::new(1);

fn next_tab_id() -> u64 {
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

/// 一个仓库标签页的全部界面状态。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Tab {
    /// 运行期标签身份。重排和关闭确认使用它，不依赖可变位置或路径。
    pub(crate) id: u64,
    /// 每次装载请求的身份。关闭后同路径重开时，旧任务不能覆盖新标签。
    pub(crate) load_id: u64,
    pub(crate) path: String,
    pub(crate) name: String,
    /// 仓库打不开的原因；正常时为 `None`。
    pub(crate) problem: RepoProblem,
    pub(crate) branch: String,
    pub(crate) ahead: u32,
    pub(crate) behind: u32,
    pub(crate) commits: Vec<Commit>,
    /// Ancestors retained to reconnect graph lanes after non-branch filters hide commits.
    pub(crate) graph_commits: Vec<Commit>,
    /// 普通 History 浏览只显示提交图；右键菜单可临时切到提交补丁。
    pub(crate) show_commit_diff: bool,
    pub(crate) refs: Refs,
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
    pub(crate) stash_diff: Option<(String, domain::diff::Patch)>,
    /// 选中的变更下标。
    pub(crate) selected_change: Option<usize>,
    /// 选中变更的 diff（加载期解析好，渲染期不再解析）。
    pub(crate) change_diff: domain::diff::Patch,
    /// 变更清单（本地存储，按仓库分组）。
    pub(crate) changelists: Vec<store::Changelist>,
    /// 当前清单名；`None` 表示默认清单。
    pub(crate) active_changelist: Option<String>,
    /// 进行中的多步操作（合并/变基/拣选/回滚）。
    pub(crate) operation: git::Operation,
    /// 冲突文件路径。
    pub(crate) conflicts: Vec<String>,
    /// 最近一次同步（fetch/pull/push）的结果；`None` 表示还没同步过。
    pub(crate) sync_message: Option<String>,
    /// 最近一次同步是否失败。
    pub(crate) sync_failed: bool,
    /// reflog（最近的在前）。
    pub(crate) reflog: Vec<git::ReflogEntry>,
    /// 选中提交的完整元信息（Info 页）。
    pub(crate) details: Option<git::CommitDetails>,
    /// 选中文件的 blame 行。
    pub(crate) blame: Vec<git::BlameLine>,
    /// 正在看的 blame 文件路径（板 19 的 `.blamebar` 要显示它）。
    pub(crate) blame_path: String,
    /// 选中文件的历史。
    pub(crate) file_history: Vec<Commit>,
    /// 对比引用的起点（`None` 表示未开启 Compare refs）。
    pub(crate) compare_base: Option<String>,
    /// 对比引用的终点。
    pub(crate) compare_target: Option<String>,
    /// 提交说明的两段文本。
    pub(crate) summary: String,
    pub(crate) description: String,
    /// 最近一次提交尝试的钩子输出；`None` 表示还没提交过。
    pub(crate) hook_log: Option<String>,
    /// 最近一次提交是否成功。
    pub(crate) hook_ok: bool,
    /// 提交面板的三个开关（设计板 10）。成功后归位。
    pub(crate) amend: bool,
    pub(crate) signoff: bool,
    pub(crate) push_after_commit: bool,
    /// 钩子完整日志弹层是否打开。
    pub(crate) hook_open: bool,
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
    /// 打开失败时的错误详情；成功时为 `None`。
    pub(crate) error: Option<String>,
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

    /// 目录部分（`src/ui/CommitPanel.tsx` → `src/ui`），没有目录时为空串。
    pub(crate) fn directory(&self) -> &str {
        return match self.path.rsplit_once('/') {
            Some((directory, _)) => directory,
            None => "",
        };
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

    /// 目录行是否要显示。
    ///
    /// `merge_empty_dirs` 打开时，只有子目录、自己没有直接文件的层级不占行——
    /// 这正是「merge empty directories」要的效果：不把 `src` 这种纯容器画成一行。
    pub(crate) fn shows_directory(self, direct_files: usize) -> bool {
        if !self.merge_empty_dirs {
            return true;
        }
        return direct_files > 0;
    }

    /// 把文件名所属目录链压缩成一行显示用的标签。
    ///
    /// `dirs` 是所有出现过的目录（完整路径）。`compact_single_child` 打开时，
    /// 如果 `a` 下只有 `b`、`b` 下只有 `c`，标签就是 `a/b/c`。
    pub(crate) fn compact_label(self, dirs: &[String], path: &str) -> String {
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
    /// blame 的纯文本（复制按钮用）：`短hash 作者 行号 代码`。
    pub(crate) fn blame_text(&self) -> String {
        let mut out = String::new();
        for line in &self.blame {
            out.push_str(&format!(
                "{} {} {} {}\n",
                line.short, line.author, line.line, line.text
            ));
        }
        return out;
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

    /// 对比区间的起止提交：`(from, to)`，按历史位置排序。
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
pub(crate) fn load_tab(path: &str, filter: &LogFilter) -> Tab {
    let repo = match Repo::open(path) {
        Ok(repo) => repo,
        Err(error) => {
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
                error: Some(error.detail()),
                problem,
                ..Tab::default()
            };
        }
    };
    let summary = repo.summary().unwrap_or_default();
    let commits = repo.log_history(filter).unwrap_or_default();
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
        repo.log_history_graph(filter)
            .unwrap_or_else(|_| commits.clone())
    };
    let refs = repo.refs().unwrap_or_default();
    let status = repo.status().unwrap_or_default();
    let authors = repo.authors_with_email().unwrap_or_default();
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
        id: next_tab_id(),
        load_id: next_tab_id(),
        path: repo.root().display().to_string(),
        name: summary.name,
        branch: summary.branch,
        ahead: summary.ahead,
        behind: summary.behind,
        commits,
        graph_commits,
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

/// 生成一个「正在加载」的占位标签页：只有路径与名称，列表都是空的。
pub(crate) fn placeholder_tab(path: &str, filter: &LogFilter) -> Tab {
    return Tab {
        id: next_tab_id(),
        load_id: next_tab_id(),
        path: path.to_string(),
        name: last_segment(path),
        loading: true,
        filter: filter.clone(),
        branch: "…".to_string(),
        ..Tab::default()
    };
}

/// 重新读一遍同一个仓库（初始化后、从别处改过仓库后用）。
pub(crate) fn reload(tab: &mut Tab) {
    let id = tab.id;
    let load_id = next_tab_id();
    let sidebar_collapsed = tab.sidebar_collapsed.clone();
    let path = tab.path.clone();
    let filter = tab.filter.clone();
    *tab = load_tab(path.as_str(), &filter);
    tab.id = id;
    tab.load_id = load_id;
    tab.sidebar_collapsed = sidebar_collapsed;
}

/// 在标签页指向的目录里初始化仓库（设计板 16 的「不是仓库」动作）。
pub(crate) fn init_here(tab: &mut Tab) -> Result<(), String> {
    git::init_with(tab.path.as_str(), &git::InitOptions::default())
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
    fn loading_a_repo_fills_the_tab_and_selects_the_first_commit() {
        let repo = TempRepo::new("load");
        let tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        assert!(tab.error.is_none());
        assert_eq!(tab.branch, "main");
        assert_eq!(tab.commits.len(), 1);
        assert_eq!(tab.selected, Some(0));
        assert_eq!(tab.selected_commit().expect("commit").summary, "first");
        assert!(!tab.refs.branches.is_empty());
        assert!(!tab.patch.lines.is_empty());
        assert_eq!(tab.files.len(), 1);
        assert_eq!(tab.authors[0].name, "Test");
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
        assert_eq!(
            plain.compact_label(&dirs, "src/git/legacy"),
            "src/git/legacy"
        );
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
