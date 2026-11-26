//! 本模块负责起 Git 进程：命令拼装、stdout/stderr 收集、输出截断与展示上限。

use domain::GitError;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Git 可执行文件。应用显式选择路径；库测试与兼容入口默认使用 PATH 中的 `git`。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitExecutable {
    path: PathBuf,
    env: Vec<(OsString, OsString)>,
}

impl GitExecutable {
    /// 使用进程启动环境 PATH 中的 `git`。
    pub fn system() -> Self {
        return Self {
            path: PathBuf::from("git"),
            env: Vec::new(),
        };
    }

    /// 使用设置页选择的可执行文件路径。
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        return Self {
            path: path.into(),
            env: Vec::new(),
        };
    }

    /// 当前配置的程序路径。
    pub fn path(&self) -> &Path {
        return &self.path;
    }

    /// 为此操作启动的 Git 进程及其子进程附加环境变量。
    pub fn with_env(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        let key = key.as_ref().to_os_string();
        if let Some((_, current)) = self.env.iter_mut().find(|(name, _)| name == &key) {
            *current = value.as_ref().to_os_string();
        } else {
            self.env.push((key, value.as_ref().to_os_string()));
        }
        return self;
    }

    /// 创建一个使用此 executable 的 Git 命令。
    pub fn command(&self) -> Command {
        let mut command = Command::new(&self.path);
        for (key, value) in &self.env {
            command.env(key, value);
        }
        return command;
    }
}

impl Default for GitExecutable {
    fn default() -> Self {
        return Self::system();
    }
}

/// 单条命令的输出上限，避免异常输出占满内存；超出的部分截断。
const MAX_OUTPUT: usize = 8 * 1024 * 1024;
/// 展示在错误里的 stderr 长度上限。
pub(crate) const MAX_DETAIL: usize = 4096;

/// 在 `directory` 里运行 `git args...`，返回 stdout。
pub fn run_git(directory: &Path, args: &[&str]) -> Result<Vec<u8>, GitError> {
    return run_git_with(&GitExecutable::system(), directory, args);
}

/// 在 `directory` 里运行指定 Git 并返回 stdout。
pub(crate) fn run_git_with(
    executable: &GitExecutable,
    directory: &Path,
    args: &[&str],
) -> Result<Vec<u8>, GitError> {
    return run_git_inner(executable, directory, args, true, None);
}

/// 运行允许某个非零退出码的 Git 命令；`diff --no-index` 找到差异时会返回 1。
pub(crate) fn run_git_allow_exit_code_with(
    executable: &GitExecutable,
    directory: &Path,
    args: &[&str],
    allowed_exit_code: i32,
) -> Result<Vec<u8>, GitError> {
    return run_git_inner(executable, directory, args, true, Some(allowed_exit_code));
}

pub(crate) fn run_git_full_with(
    executable: &GitExecutable,
    directory: &Path,
    args: &[&str],
) -> Result<Vec<u8>, GitError> {
    return run_git_inner(executable, directory, args, false, None);
}

fn run_git_inner(
    executable: &GitExecutable,
    directory: &Path,
    args: &[&str],
    limit_stdout: bool,
    allowed_exit_code: Option<i32>,
) -> Result<Vec<u8>, GitError> {
    let program = executable.path().display().to_string();
    let mut command = executable.command();
    command
        .arg("-C")
        .arg(directory)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0");
    let output = command.output().map_err(|error| GitError::Spawn {
        program: program.clone(),
        reason: error.to_string(),
    })?;
    if !output.status.success()
        && (output.status.code() != allowed_exit_code || !output.stderr.is_empty())
    {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = truncate(&detail, MAX_DETAIL);
        let joined = args.join(" ");
        // `rev-parse --show-toplevel` 在非仓库目录失败属于正常探测，调用方
        // 需要区分“不是仓库”和“命令出错”，因此这里保留原始错误。
        return Err(GitError::CommandFailed {
            program,
            args: joined,
            code: output.status.code(),
            detail,
        });
    }
    let mut stdout = output.stdout;
    if limit_stdout {
        stdout.truncate(MAX_OUTPUT);
    }
    return Ok(stdout);
}

pub(crate) fn run_git_combined_with_executable_env(
    executable: &GitExecutable,
    directory: &Path,
    args: &[&str],
    envs: &[(&str, &str)],
) -> Result<String, GitError> {
    let program = executable.path().display().to_string();
    let mut command = executable.command();
    command
        .arg("-C")
        .arg(directory)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0");
    for (key, value) in envs {
        command.env(key, value);
    }
    let output = command.output().map_err(|error| GitError::Spawn {
        program: program.clone(),
        reason: error.to_string(),
    })?;
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !stderr.trim().is_empty() {
        text.push_str(stderr.as_str());
    }
    if !output.status.success() {
        return Err(GitError::CommandFailed {
            program,
            args: args.join(" "),
            code: output.status.code(),
            detail: truncate(text.trim(), MAX_DETAIL),
        });
    }
    return Ok(text);
}

pub(crate) fn run_git_combined_with_executable(
    executable: &GitExecutable,
    directory: &Path,
    args: &[&str],
) -> Result<String, GitError> {
    let program = executable.path().display().to_string();
    let mut command = executable.command();
    command
        .arg("-C")
        .arg(directory)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0");
    let output = command.output().map_err(|error| GitError::Spawn {
        program: program.clone(),
        reason: error.to_string(),
    })?;
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !stderr.trim().is_empty() {
        text.push_str(stderr.as_str());
    }
    if !output.status.success() {
        return Err(GitError::CommandFailed {
            program,
            args: args.join(" "),
            code: output.status.code(),
            detail: truncate(text.trim(), MAX_DETAIL),
        });
    }
    return Ok(text);
}

/// reflog 主题里动作与说明之间的分隔符（`": "`）。
pub(crate) fn portable_separator() -> &'static str {
    return ": ";
}

pub(crate) fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let kept: String = text.chars().take(limit).collect();
    return format!("{}…", kept);
}
