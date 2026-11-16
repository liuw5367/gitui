//! 欢迎页的 Signal 集合：仓库搜索词与 Open / Clone / Init 三段表单的输入。
//!
//! 只放属于本功能、且不跨功能的信号；最近仓库、语言与屏幕状态是会话级的，
//! 走 `SessionHandle`。

use nsh_core::prelude::*;

/// 仓库管理页的三种入口（设计板 03 的分段控件）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RepoForm {
    Open,
    Clone,
    Init,
}

/// Init 表单的附带文件（设计板 03）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct InitFiles {
    pub(crate) gitignore: bool,
    pub(crate) readme: bool,
}

/// Clone 表单的选项（设计板 03）：分支、深度、子模块、克隆后打开。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CloneOptions {
    pub(crate) branch: String,
    /// 0 = 完整克隆。
    pub(crate) depth: u32,
    /// 用户自定义深度输入（空则用 `depth`）。
    pub(crate) depth_custom: String,
    pub(crate) recurse_submodules: bool,
    pub(crate) open_after_clone: bool,
}

#[derive(Clone)]
pub(crate) struct WelcomeState {
    /// 左栏仓库搜索词。
    pub(crate) query: Signal<String>,
    /// 克隆源地址。
    pub(crate) clone_url: Signal<String>,
    /// 克隆目标目录。
    pub(crate) clone_destination: Signal<String>,
    /// 克隆的附加选项。
    pub(crate) clone_options: Signal<CloneOptions>,
    /// 初始化时要附带的文件。
    pub(crate) init_files: Signal<InitFiles>,
    /// 初始化目标目录。
    pub(crate) init_path: Signal<String>,
    /// 表单错误（克隆/初始化共用一条）。
    pub(crate) clone_error: Signal<Option<String>>,
    /// 克隆是否进行中。
    pub(crate) cloning: Signal<bool>,
    /// 克隆进度（git 的 stderr 最后一行）。
    pub(crate) clone_progress: Signal<String>,
    /// 当前选中的入口分段。
    pub(crate) form: Signal<RepoForm>,
}
