//! 引用管理功能的 Signal 集合：页签、reflog 过滤与当前选中记录。
//!
//! 只放属于本功能、且不跨功能的信号；分支、远程、标签、贮藏、工作树与 reflog
//! 条目都是每个仓库各自的数据，挂在 `Tab` 上，仍留在 `Tab` 里。
//!
use nsh_core::prelude::*;

/// Refs 主区的两个页签。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RefsTab {
    Refs,
    Reflog,
}

impl RefsTab {
    /// 页签文案的 i18n 键。
    pub(crate) fn key(self) -> &'static str {
        return match self {
            RefsTab::Refs => "refs.title",
            RefsTab::Reflog => "reflog.title",
        };
    }
}

/// Reflog 行的快照身份。同一 SHA 可能因多次引用移动出现多次，不能只按 SHA 选行。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReflogEntryKey {
    pub(crate) tab_id: u64,
    pub(crate) path: String,
    /// 本次仓库 reflog 快照中的原始位置，不受过滤后可见位置影响。
    pub(crate) index: usize,
    pub(crate) sha: String,
    pub(crate) action: String,
    pub(crate) summary: String,
    pub(crate) timestamp: i64,
}

impl ReflogEntryKey {
    pub(crate) fn new(tab_id: u64, path: String, index: usize, entry: &git::ReflogEntry) -> Self {
        return Self {
            tab_id,
            path,
            index,
            sha: entry.sha.clone(),
            action: entry.action.clone(),
            summary: entry.summary.clone(),
            timestamp: entry.timestamp,
        };
    }

    pub(crate) fn matches_entry(
        &self,
        tab_id: u64,
        path: &str,
        index: usize,
        entry: &git::ReflogEntry,
    ) -> bool {
        return self == &Self::new(tab_id, path.to_string(), index, entry);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReflogDetailState {
    Loading(ReflogEntryKey),
    Loaded {
        selection: ReflogEntryKey,
        details: git::CommitDetails,
        files: Vec<domain::DiffStat>,
    },
    Failed(ReflogEntryKey),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReflogFileDiffStatus {
    Loading,
    Loaded {
        patch: domain::diff::Patch,
        raw_patch: domain::diff::Patch,
    },
    Failed,
}

/// The selected file diff in the center column. Its identity includes the exact reflog row,
/// because the same commit can appear at multiple positions in a reflog snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReflogFileDiffState {
    pub(crate) selection: ReflogEntryKey,
    pub(crate) path: String,
    pub(crate) files: Vec<domain::DiffStat>,
    pub(crate) status: ReflogFileDiffStatus,
}

#[derive(Clone)]
pub(crate) struct RefsState {
    /// Refs / Reflog 的页签选择，工具栏直达入口与中栏页签共用这份状态。
    pub(crate) selected_tab: Signal<RefsTab>,
    /// Reflog 的过滤关键字；同时匹配动作、说明与短 sha，空串不过滤。
    pub(crate) reflog_query: Signal<String>,
    /// 当前选择按原始 Reflog 快照行区分，而非只按 SHA。
    pub(crate) reflog_selection: Signal<Option<ReflogEntryKey>>,
    /// 右栏提交详情只读加载状态。
    pub(crate) reflog_detail: Signal<Option<ReflogDetailState>>,
    /// 中栏当前打开的 Reflog 文件差异。
    pub(crate) reflog_file_diff: Signal<Option<ReflogFileDiffState>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflog_rows_with_same_sha_keep_distinct_identity() {
        let entry = git::ReflogEntry {
            short: "abc1234".to_string(),
            sha: "abc1234567890".to_string(),
            action: "reset".to_string(),
            summary: "moving to HEAD~1".to_string(),
            timestamp: 1_700_000_000,
        };
        let first = ReflogEntryKey::new(7, "/repo".to_string(), 0, &entry);
        let second = ReflogEntryKey::new(7, "/repo".to_string(), 1, &entry);

        assert_ne!(first, second);
        assert!(first.matches_entry(7, "/repo", 0, &entry));
        assert!(!first.matches_entry(8, "/repo", 0, &entry));
        assert!(!first.matches_entry(7, "/other", 0, &entry));
    }
}
