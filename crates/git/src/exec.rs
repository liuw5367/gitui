//! 本模块负责起 Git 进程：命令拼装、stdout/stderr 收集、输出截断与展示上限。

use domain::GitError;
use std::path::Path;
use std::process::Command;

/// 单条命令的输出上限，避免异常输出占满内存；超出的部分截断。
const MAX_OUTPUT: usize = 8 * 1024 * 1024;
/// 展示在错误里的 stderr 长度上限。
pub(crate) const MAX_DETAIL: usize = 4096;

/// 在 `directory` 里运行 `git args...`，返回 stdout。
pub fn run_git(directory: &Path, args: &[&str]) -> Result<Vec<u8>, GitError> {
    let program = "git";
    let mut command = Command::new(program);
    command
        .arg("-C")
        .arg(directory)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0");
    let output = command.output().map_err(|error| GitError::Spawn {
        program: program.to_string(),
        reason: error.to_string(),
    })?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = truncate(&detail, MAX_DETAIL);
        let joined = args.join(" ");
        // `rev-parse --show-toplevel` 在非仓库目录失败属于正常探测，调用方
        // 需要区分“不是仓库”和“命令出错”，因此这里保留原始错误。
        return Err(GitError::CommandFailed {
            program: program.to_string(),
            args: joined,
            code: output.status.code(),
            detail,
        });
    }
    let mut stdout = output.stdout;
    stdout.truncate(MAX_OUTPUT);
    return Ok(stdout);
}

/// 运行 Git 并把 stdout 与 stderr 合并返回。
///
/// 同步命令（fetch/pull/push）的进度与结果都写在 stderr，只读 stdout 会得到空串。
/// 带额外环境变量的合并输出版本（继续操作要关掉编辑器）。
pub(crate) fn run_git_combined_with_env(
    directory: &Path,
    args: &[&str],
    envs: &[(&str, &str)],
) -> Result<String, GitError> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(directory)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0");
    for (key, value) in envs {
        command.env(key, value);
    }
    let output = command.output().map_err(|error| GitError::Spawn {
        program: "git".to_string(),
        reason: error.to_string(),
    })?;
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !stderr.trim().is_empty() {
        text.push_str(stderr.as_str());
    }
    if !output.status.success() {
        return Err(GitError::CommandFailed {
            program: "git".to_string(),
            args: args.join(" "),
            code: output.status.code(),
            detail: truncate(text.trim(), MAX_DETAIL),
        });
    }
    return Ok(text);
}

pub(crate) fn run_git_combined(directory: &Path, args: &[&str]) -> Result<String, GitError> {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(directory)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0");
    let output = command.output().map_err(|error| GitError::Spawn {
        program: "git".to_string(),
        reason: error.to_string(),
    })?;
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !stderr.trim().is_empty() {
        text.push_str(stderr.as_str());
    }
    if !output.status.success() {
        return Err(GitError::CommandFailed {
            program: "git".to_string(),
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
