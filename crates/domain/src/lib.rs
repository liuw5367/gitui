//! 领域类型与错误分类：UI 与 Git 执行层共用的最小集合。
//!
//! 这里只放与界面、进程都无关的数据结构与失败类别；命令拼装和输出解析在
//! `gitui-git`，UI 状态在应用 crate。

pub mod conflict;
pub mod diff;
pub mod stage;

/// 失败类别。UI 按类别选择呈现方式（内联错误、banner、表单错误），
/// 不解析错误文案。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitError {
    /// 系统找不到 `git` 可执行文件。
    GitMissing,
    /// 路径存在但不是 Git 仓库（没有 `.git`，也不是 bare 仓库）。
    NotARepository { path: String },
    /// 仓库路径不存在或不可读。
    Unreadable { path: String, reason: String },
    /// Git 命令以非零状态退出；`detail` 已截断，供界面展示。
    CommandFailed {
        program: String,
        args: String,
        code: Option<i32>,
        detail: String,
    },
    /// 无法启动 Git 进程。
    Spawn { program: String, reason: String },
    /// 钩子拒绝了这次操作（提交、推送），`log` 是钩子与 Git 的输出。
    HookFailed { log: String },
    /// 输出与预期格式不符。
    Parse { context: String, detail: String },
}

/// 网络类失败的三种可恢复形态。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkFault {
    /// 需要登录或换 SSH。
    AuthRequired,
    /// 连不上远端。
    Offline,
    /// 远端拒绝了这次推送（需要先拉取）。
    Rejected,
}

impl GitError {
    /// 面向用户的一句话说明；具体命令与 stderr 由界面放在详情里。
    pub fn summary(&self) -> String {
        let summary = match self {
            GitError::GitMissing => "找不到 git 命令".to_string(),
            GitError::NotARepository { .. } => "不是 Git 仓库".to_string(),
            GitError::Unreadable { .. } => "无法读取仓库路径".to_string(),
            GitError::CommandFailed { .. } => "Git 命令执行失败".to_string(),
            GitError::Spawn { .. } => "无法启动 Git 进程".to_string(),
            GitError::HookFailed { .. } => "钩子拒绝了这次操作".to_string(),
            GitError::Parse { .. } => "无法解析 Git 输出".to_string(),
        };
        return summary;
    }

    /// 可用于详情区或日志的诊断文本。
    pub fn detail(&self) -> String {
        match self {
            GitError::GitMissing => {
                return "请安装 Git，或在设置中指定 git 可执行文件路径".to_string();
            }
            GitError::NotARepository { path } => {
                return format!("{} 不是 Git 仓库", path);
            }
            GitError::Unreadable { path, reason } => {
                return format!("无法读取 {}：{}", path, reason);
            }
            GitError::CommandFailed {
                program,
                args,
                code,
                detail,
            } => {
                let code = code.map(|code| code.to_string()).unwrap_or_default();
                return format!("{} {}（退出码 {}）：{}", program, args, code, detail);
            }
            GitError::Spawn { program, reason } => {
                return format!("无法启动 {}：{}", program, reason);
            }
            GitError::HookFailed { log } => {
                return log.clone();
            }
            GitError::Parse { context, detail } => {
                return format!("解析 {} 失败：{}", context, detail);
            }
        }
    }
}

impl std::fmt::Display for GitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(formatter, "{}", self.detail());
    }
}

impl std::error::Error for GitError {}

/// 一次提交。`date` 是 `git log --date=relative` 的相对时间文本。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Commit {
    pub sha: String,
    pub short: String,
    pub summary: String,
    pub author: String,
    /// Author email, used as a stable filter identity when names collide.
    pub author_email: String,
    /// Full commit message, including the body.
    pub message: String,
    pub date: String,
    /// `%D` 解析后的引用名：`main`、`tag: v1.4.0`、`origin/main`。
    pub refs: Vec<String>,
    /// 指向该提交的所有本地与远端分支，含经过该提交历史的后代分支。
    pub branches: Vec<String>,
    /// 父提交的完整 SHA；多于一个表示合并提交（图线据此画分叉）。
    pub parents: Vec<String>,
}

/// 工作区或 index 中的一个变更文件。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StatusEntry {
    pub path: String,
    /// index 状态字符（`M`、`A`、`D`、`R`…）；` ` 表示未暂存。
    pub index: char,
    /// 工作区状态字符。
    pub worktree: char,
}

impl StatusEntry {
    pub fn is_staged(&self) -> bool {
        return self.index != ' ' && self.index != '?';
    }

    pub fn is_unstaged(&self) -> bool {
        return self.worktree != ' ' || self.index == '?';
    }
}

/// 一个文件的 diff 统计。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiffStat {
    /// Git 文件变更类型：A、M、D、R、C、T。
    pub status: char,
    pub path: String,
    pub additions: u32,
    pub deletions: u32,
}

/// 仓库快照：侧栏与状态栏使用的最小信息。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RepoSummary {
    pub path: String,
    pub name: String,
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_failure_detail_keeps_program_and_exit_code() {
        let error = GitError::CommandFailed {
            program: "git".to_string(),
            args: "status".to_string(),
            code: Some(128),
            detail: "not a git repository".to_string(),
        };
        let detail = error.detail();
        assert!(detail.contains("git status"));
        assert!(detail.contains("128"));
        assert!(detail.contains("not a git repository"));
    }

    #[test]
    fn status_entry_classifies_index_and_worktree() {
        let staged = StatusEntry {
            path: "a.txt".to_string(),
            index: 'M',
            worktree: ' ',
        };
        assert!(staged.is_staged());
        assert!(!staged.is_unstaged());

        let untracked = StatusEntry {
            path: "b.txt".to_string(),
            index: '?',
            worktree: '?',
        };
        assert!(!untracked.is_staged());
        assert!(untracked.is_unstaged());
    }
}
