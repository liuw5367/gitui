//! 本模块负责与远端打交道：取回、拉取、推送，以及克隆与初始化。

use domain::GitError;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::exec::{GitExecutable, MAX_DETAIL, truncate};
use crate::parse::summarise;
use crate::query::Repo;

#[cfg(test)]
use crate::exec::run_git;

/// 取回选项（设计板 28）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FetchOptions {
    /// 删掉远端已删除的跟踪分支。
    pub prune: bool,
    /// 同时取回标签。
    pub tags: bool,
    /// 对所有远端执行。
    pub all_remotes: bool,
}

/// 拉取时如何整合上游（设计板 29 的三选）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PullStrategy {
    /// `--no-rebase`：产生合并提交。
    #[default]
    Merge,
    /// `--rebase`：把本地提交移到上游之上。
    Rebase,
    /// `--ff-only`：只允许快进，否则失败。
    FfOnly,
}

impl PullStrategy {
    pub fn id(self) -> &'static str {
        return match self {
            PullStrategy::Merge => "merge",
            PullStrategy::Rebase => "rebase",
            PullStrategy::FfOnly => "ff-only",
        };
    }

    pub fn from_id(id: &str) -> PullStrategy {
        return match id {
            "rebase" => PullStrategy::Rebase,
            "ff-only" => PullStrategy::FfOnly,
            _ => PullStrategy::Merge,
        };
    }

    fn flag(self) -> &'static str {
        return match self {
            PullStrategy::Merge => "--no-rebase",
            PullStrategy::Rebase => "--rebase",
            PullStrategy::FfOnly => "--ff-only",
        };
    }
}

/// 拉取选项（设计板 29）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PullOptions {
    pub strategy: PullStrategy,
    /// 拉取前自动贮藏、完成后恢复。
    pub autostash: bool,
    /// 变基时把本地所有提交一起移动。
    pub rebase_local: bool,
}

/// 推送选项（设计板 15）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PushOptions {
    /// 一并推送标签。
    pub tags: bool,
    /// `--force-with-lease`：远端没被别人改动才允许覆盖。
    pub force_with_lease: bool,
    /// 推送所有分支。
    pub all_branches: bool,
    /// 将指定的本地分支推送到指定的远端分支。
    pub branches: Option<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteTag {
    pub name: String,
    pub oid: String,
    pub target: String,
}

const MAX_SYNC_OUTPUT: usize = 256 * 1024;

/// A single status row emitted by `git push --porcelain`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PushRefUpdate {
    pub status: PushRefStatus,
    pub source: Option<String>,
    pub destination: String,
    /// Git's endpoint summary, such as `abcd123..def456` or `[new branch]`.
    pub summary: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PushRefStatus {
    Updated,
    Created,
    Deleted,
    Forced,
    UpToDate,
    Rejected,
}

/// Parse machine-readable status rows while ignoring the human progress and `To …` lines.
pub fn parse_push_porcelain(output: &str) -> Vec<PushRefUpdate> {
    let mut updates = Vec::new();
    for line in output.lines() {
        let Some((marker, fields)) =
            line.split_at_checked(line.chars().next().map(char::len_utf8).unwrap_or(0))
        else {
            continue;
        };
        let status = match marker {
            " " => PushRefStatus::Updated,
            "*" => PushRefStatus::Created,
            "-" => PushRefStatus::Deleted,
            "+" => PushRefStatus::Forced,
            "=" => PushRefStatus::UpToDate,
            "!" => PushRefStatus::Rejected,
            _ => continue,
        };
        let mut parts = fields.split_whitespace();
        let Some(refspec) = parts.next() else {
            continue;
        };
        let (source, destination) = match refspec.split_once(':') {
            Some((source, destination)) => (
                (!source.is_empty()).then(|| source.to_string()),
                destination.to_string(),
            ),
            None => continue,
        };
        if destination.is_empty() {
            continue;
        }
        let summary = parts.collect::<Vec<_>>().join(" ");
        updates.push(PushRefUpdate {
            status,
            source,
            destination,
            summary,
        });
    }
    return updates;
}

pub struct SyncRun {
    child: Child,
    program: String,
    process_group: Option<u32>,
    args: String,
    output: Arc<Mutex<String>>,
    stdout: Arc<Mutex<String>>,
    stdout_truncated: Arc<AtomicBool>,
    stop_readers: Arc<AtomicBool>,
    readers: Vec<JoinHandle<()>>,
}

impl SyncRun {
    fn start(
        executable: &GitExecutable,
        directory: &Path,
        args: &[String],
    ) -> Result<SyncRun, GitError> {
        let mut command = executable.command();
        command.arg("-C").arg(directory).args(args);
        return Self::spawn_with_program(command, args, executable.path().display().to_string());
    }

    #[cfg(test)]
    fn spawn(command: Command, args: &[String]) -> Result<SyncRun, GitError> {
        return Self::spawn_with_program(command, args, "git".to_string());
    }

    fn spawn_with_program(
        mut command: Command,
        args: &[String],
        program: String,
    ) -> Result<SyncRun, GitError> {
        let process_group = configure_sync_process_group(&mut command);
        let mut child = command
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| GitError::Spawn {
                program: program.clone(),
                reason: error.to_string(),
            })?;
        let output = Arc::new(Mutex::new(String::new()));
        let status_output = Arc::new(Mutex::new(String::new()));
        let status_truncated = Arc::new(AtomicBool::new(false));
        let stop_readers = Arc::new(AtomicBool::new(false));
        let mut readers = Vec::new();
        if let Some(pipe) = child.stdout.take() {
            readers.push(read_sync_output(
                pipe,
                output.clone(),
                Some(status_output.clone()),
                Some(status_truncated.clone()),
                stop_readers.clone(),
            ));
        }
        if let Some(pipe) = child.stderr.take() {
            readers.push(read_sync_output(
                pipe,
                output.clone(),
                None,
                None,
                stop_readers.clone(),
            ));
        }
        return Ok(SyncRun {
            process_group: process_group.then(|| child.id()),
            child,
            program,
            args: args.join(" "),
            output,
            stdout: status_output,
            stdout_truncated: status_truncated,
            stop_readers,
            readers,
        });
    }

    pub fn progress(&self) -> String {
        return self
            .output
            .lock()
            .map(|output| output.clone())
            .unwrap_or_default();
    }

    /// Machine-readable command output from stdout (stderr remains in `progress()`).
    pub fn status_output(&self) -> String {
        return self
            .stdout
            .lock()
            .map(|output| output.clone())
            .unwrap_or_default();
    }

    /// Whether the bounded machine-readable output buffer dropped any prefix.
    pub fn status_output_truncated(&self) -> bool {
        return self.stdout_truncated.load(Ordering::Acquire);
    }

    pub fn try_finish(&mut self) -> Option<Result<String, GitError>> {
        let status = match self.child.try_wait() {
            Ok(status) => status,
            Err(error) => {
                return Some(Err(GitError::Spawn {
                    program: self.program.clone(),
                    reason: error.to_string(),
                }));
            }
        }?;
        self.terminate_remaining_processes();
        self.stop_readers.store(true, Ordering::Release);
        self.join_readers();
        let output = self.progress();
        if status.success() {
            return Some(Ok(summarise(&output)));
        }
        return Some(Err(GitError::CommandFailed {
            program: self.program.clone(),
            args: self.args.clone(),
            code: status.code(),
            detail: crate::exec::truncate(output.trim(), MAX_DETAIL),
        }));
    }

    pub fn cancel(&mut self) -> String {
        self.terminate_remaining_processes();
        let _ = self.child.wait();
        self.stop_readers.store(true, Ordering::Release);
        self.join_readers();
        return self.progress();
    }

    fn terminate_remaining_processes(&mut self) {
        if let Some(process_group) = self.process_group.take() {
            terminate_sync_process_group(process_group);
        }
        let _ = self.child.kill();
    }

    fn join_readers(&mut self) {
        for reader in self.readers.drain(..) {
            let _ = reader.join();
        }
    }
}

fn configure_sync_process_group(command: &mut Command) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
        return true;
    }
    #[cfg(not(unix))]
    {
        let _ = command;
        return false;
    }
}

#[cfg(unix)]
fn terminate_sync_process_group(process_group: u32) {
    let process_group = process_group as libc::pid_t;
    unsafe {
        let _ = libc::kill(-process_group, libc::SIGKILL);
    }
}

#[cfg(not(unix))]
fn terminate_sync_process_group(_process_group: u32) {}

#[cfg(unix)]
fn read_sync_output<R>(
    pipe: R,
    output: Arc<Mutex<String>>,
    status_output: Option<Arc<Mutex<String>>>,
    status_truncated: Option<Arc<AtomicBool>>,
    stop: Arc<AtomicBool>,
) -> JoinHandle<()>
where
    R: std::io::Read + std::os::fd::AsRawFd + Send + 'static,
{
    return std::thread::spawn(move || {
        let fd = pipe.as_raw_fd();
        let mut pipe = pipe;
        let mut chunk = [0u8; 1024];
        let mut drained_after_stop = 0usize;
        loop {
            let stopping = stop.load(Ordering::Acquire);
            if !sync_output_ready(fd, if stopping { 0 } else { 100 }) {
                if stopping {
                    break;
                }
                continue;
            }
            let read = match std::io::Read::read(&mut pipe, &mut chunk) {
                Ok(0) => break,
                Ok(read) => read,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if stopping {
                        break;
                    }
                    continue;
                }
                Err(_) => break,
            };
            let _ = append_sync_output(&output, &chunk[..read]);
            if let Some(status_output) = &status_output {
                let truncated = append_sync_output(status_output, &chunk[..read]);
                if truncated && let Some(status_truncated) = &status_truncated {
                    status_truncated.store(true, Ordering::Release);
                }
            }
            if stopping {
                drained_after_stop = drained_after_stop.saturating_add(read);
                if drained_after_stop >= MAX_SYNC_OUTPUT {
                    break;
                }
            }
        }
    });
}

#[cfg(unix)]
fn sync_output_ready(fd: std::os::fd::RawFd, timeout_ms: i32) -> bool {
    let mut descriptor = libc::pollfd {
        fd,
        events: libc::POLLIN | libc::POLLHUP | libc::POLLERR,
        revents: 0,
    };
    loop {
        let result = unsafe { libc::poll(&mut descriptor, 1, timeout_ms) };
        if result > 0 {
            return true;
        }
        if result == 0 {
            return false;
        }
        if std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
            return false;
        }
    }
}

#[cfg(not(unix))]
fn read_sync_output<R>(
    pipe: R,
    output: Arc<Mutex<String>>,
    status_output: Option<Arc<Mutex<String>>>,
    status_truncated: Option<Arc<AtomicBool>>,
    stop: Arc<AtomicBool>,
) -> JoinHandle<()>
where
    R: std::io::Read + Send + 'static,
{
    return std::thread::spawn(move || {
        let mut pipe = pipe;
        let mut chunk = [0u8; 1024];
        loop {
            if stop.load(Ordering::Acquire) {
                break;
            }
            let read = match std::io::Read::read(&mut pipe, &mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(read) => read,
            };
            let _ = append_sync_output(&output, &chunk[..read]);
            if let Some(status_output) = &status_output {
                let truncated = append_sync_output(status_output, &chunk[..read]);
                if truncated && let Some(status_truncated) = &status_truncated {
                    status_truncated.store(true, Ordering::Release);
                }
            }
        }
    });
}

fn append_sync_output(output: &Arc<Mutex<String>>, bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes).replace('\r', "\n");
    let Ok(mut output) = output.lock() else {
        return true;
    };
    output.push_str(&text);
    if output.len() <= MAX_SYNC_OUTPUT {
        return false;
    }
    let excess = output.len() - MAX_SYNC_OUTPUT;
    let boundary = output
        .char_indices()
        .find(|(index, _)| *index >= excess)
        .map(|(index, _)| index)
        .unwrap_or(output.len());
    output.drain(..boundary);
    return true;
}

impl Repo {
    /// Capture local remote-tracking refs and tags before/after fetch.
    pub fn sync_ref_snapshot(&self) -> Result<BTreeMap<String, String>, GitError> {
        let output = self.run_git(&[
            "for-each-ref",
            "--format=%(refname)%1f%(objectname)%1f%(symref)",
            "refs/remotes",
            "refs/tags",
        ])?;
        let mut refs = BTreeMap::new();
        for line in String::from_utf8_lossy(&output).lines() {
            let Some((name, details)) = line.split_once('\u{1f}') else {
                continue;
            };
            let Some((oid, symbolic_target)) = details.split_once('\u{1f}') else {
                continue;
            };
            if !symbolic_target.is_empty() {
                continue;
            }
            if !name.is_empty() && !oid.is_empty() {
                refs.insert(name.to_string(), oid.to_string());
            }
        }
        return Ok(refs);
    }

    /// Current upstream full ref name and commit OID, if the branch has a readable upstream.
    pub fn upstream_ref(&self) -> Option<(String, String)> {
        let name = self
            .run_git(&["rev-parse", "--symbolic-full-name", "@{upstream}"])
            .ok()
            .map(|output| String::from_utf8_lossy(&output).trim().to_string())
            .filter(|name| !name.is_empty())?;
        let oid = self.resolve_commit_oid(name.as_str()).ok()?;
        return Some((name, oid));
    }

    /// Current HEAD commit OID, if one exists.
    pub fn head_oid(&self) -> Option<String> {
        return self.resolve_commit_oid("HEAD").ok();
    }

    /// Resolve a ref or abbreviated OID to a commit OID.
    pub fn resolve_commit_oid(&self, reference: &str) -> Result<String, GitError> {
        let revision = format!("{reference}^{{commit}}");
        let output = self.run_git(&[
            "rev-parse",
            "--verify",
            "--end-of-options",
            revision.as_str(),
        ])?;
        return Ok(String::from_utf8_lossy(&output).trim().to_string());
    }

    /// Commit OIDs reachable from `target` but not from `base`; `None` means no base ref.
    pub fn commit_oids_between(
        &self,
        base: Option<&str>,
        target: &str,
    ) -> Result<Vec<String>, GitError> {
        let mut args = vec!["rev-list", "--end-of-options", target];
        let exclusion = base.map(|base| format!("^{base}"));
        if let Some(exclusion) = exclusion.as_deref() {
            args.push(exclusion);
        }
        return self.lines(&args);
    }

    /// 从远程取回引用（`git fetch --prune <remote>`）。
    pub fn fetch(&self, remote: &str) -> Result<String, GitError> {
        return self.fetch_with(
            remote,
            &FetchOptions {
                prune: true,
                ..FetchOptions::default()
            },
        );
    }

    /// 带选项的取回：剪枝、标签、全部远端。
    pub fn fetch_with(&self, remote: &str, options: &FetchOptions) -> Result<String, GitError> {
        let mut args: Vec<&str> = vec!["fetch"];
        if options.prune {
            args.push("--prune");
        }
        if options.tags {
            args.push("--tags");
        }
        if options.all_remotes {
            args.push("--all");
        } else {
            args.push(remote);
        }
        let output = self.run_git_combined(&args)?;
        return Ok(summarise(&output));
    }

    pub fn start_fetch_with(
        &self,
        remote: &str,
        options: &FetchOptions,
    ) -> Result<SyncRun, GitError> {
        let mut args = vec!["fetch".to_string(), "--progress".to_string()];
        if options.prune {
            args.push("--prune".to_string());
        }
        if options.tags {
            args.push("--tags".to_string());
        }
        if options.all_remotes {
            args.push("--all".to_string());
        } else {
            args.push(remote.to_string());
        }
        return SyncRun::start(&self.executable, &self.root, &args);
    }

    /// 上游新增的提交数（`HEAD..@{u}`）；没有上游时为 0。
    pub fn upstream_incoming(&self) -> u32 {
        let output = match self.run_git(&["rev-list", "--count", "HEAD..@{upstream}"]) {
            Ok(output) => output,
            Err(_) => return 0,
        };
        return String::from_utf8_lossy(&output).trim().parse().unwrap_or(0);
    }

    /// 拉取当前分支的上游（`git pull --ff-only`）。
    ///
    /// 只用快进：会产生合并或冲突的拉取交给后续阶段的操作流程，避免在同步里隐式
    /// 生成合并提交。
    pub fn pull(&self) -> Result<String, GitError> {
        return self.pull_with(&PullOptions::default());
    }

    /// 带选项的拉取：整合方式、自动贮藏、整体变基。
    pub fn pull_with(&self, options: &PullOptions) -> Result<String, GitError> {
        let mut args: Vec<&str> = vec!["pull", options.strategy.flag(), "--no-edit"];
        if options.autostash {
            args.push("--autostash");
        }
        if options.rebase_local && options.strategy == PullStrategy::Rebase {
            args.push("--rebase-merges");
        }
        let output = self.run_git_combined(&args)?;
        return Ok(summarise(&output));
    }

    pub fn start_pull_with(&self, options: &PullOptions) -> Result<SyncRun, GitError> {
        let mut args = vec![
            "pull".to_string(),
            "--progress".to_string(),
            "--no-edit".to_string(),
        ];
        args.push(options.strategy.flag().to_string());
        if options.autostash {
            args.push("--autostash".to_string());
        }
        if options.rebase_local && options.strategy == PullStrategy::Rebase {
            args.push("--rebase-merges".to_string());
        }
        return SyncRun::start(&self.executable, &self.root, &args);
    }

    /// 从指定远端分支拉取并合并到当前分支。
    pub fn pull_remote_branch(&self, remote: &str, branch: &str) -> Result<String, GitError> {
        let output =
            self.run_git_combined(&["pull", "--no-rebase", "--no-edit", remote, branch])?;
        return Ok(summarise(&output));
    }

    /// 从服务器删除一条分支，不使用强制推送。
    pub fn delete_remote_branch(&self, remote: &str, branch: &str) -> Result<String, GitError> {
        let output = self.run_git_combined(&["push", remote, "--delete", branch])?;
        return Ok(summarise(&output));
    }

    /// 推送当前分支到上游；没有上游时用 `-u <remote> <branch>` 建立。
    pub fn push(&self, remote: Option<&str>) -> Result<String, GitError> {
        return self.push_with(remote, &PushOptions::default());
    }

    /// 带选项的推送：标签、`--force-with-lease`、全部分支。
    pub fn push_with(
        &self,
        remote: Option<&str>,
        options: &PushOptions,
    ) -> Result<String, GitError> {
        let args = self.push_args(remote, options)?;
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let output = self.run_git_combined(&refs)?;
        return Ok(summarise(&output));
    }

    pub fn start_push_with(
        &self,
        remote: Option<&str>,
        options: &PushOptions,
    ) -> Result<SyncRun, GitError> {
        let mut args = self.push_args(remote, options)?;
        args.insert(1, "--porcelain".to_string());
        args.insert(2, "--progress".to_string());
        return SyncRun::start(&self.executable, &self.root, &args);
    }

    fn push_args(
        &self,
        remote: Option<&str>,
        options: &PushOptions,
    ) -> Result<Vec<String>, GitError> {
        if !options.all_branches
            && let Some((source, target)) = &options.branches
        {
            self.run_git(&["check-ref-format", &format!("refs/heads/{source}")])?;
            self.run_git(&["check-ref-format", &format!("refs/heads/{target}")])?;
            let mut args = vec!["push".to_string()];
            if options.force_with_lease {
                args.push("--force-with-lease".to_string());
            }
            if options.tags {
                args.push("--tags".to_string());
            }
            args.extend([
                "--".to_string(),
                remote.unwrap_or("origin").to_string(),
                format!("refs/heads/{source}:refs/heads/{target}"),
            ]);
            return Ok(args);
        }
        let branch = self.abbrev_branch()?;
        let has_upstream = self
            .run_git(&[
                "rev-parse",
                "--abbrev-ref",
                "--symbolic-full-name",
                "@{upstream}",
            ])
            .is_ok();
        let mut args: Vec<String> = vec!["push".to_string()];
        if options.force_with_lease {
            args.push("--force-with-lease".to_string());
        }
        if options.tags {
            args.push("--tags".to_string());
        }
        if options.all_branches {
            args.push("--all".to_string());
            if let Some(remote) = remote {
                args.extend(["--".to_string(), remote.to_string()]);
            }
        }
        if options.all_branches || options.tags {
            // 已经指定了范围，不再追加分支名。
        } else {
            match (has_upstream, remote) {
                (true, _) => {}
                (false, Some(remote)) => {
                    args.push("-u".to_string());
                    args.push(remote.to_string());
                    args.push(branch);
                }
                (false, None) => {
                    args.push("-u".to_string());
                    args.push("origin".to_string());
                    args.push(branch);
                }
            }
        }
        return Ok(args);
    }

    /// Read remote tags without importing them into the local tag namespace.
    pub fn remote_tags(&self, remote: &str) -> Result<Vec<RemoteTag>, GitError> {
        let output = self.run_git(&["ls-remote", "--tags", "--", remote])?;
        let output = String::from_utf8_lossy(&output);
        let mut tags = std::collections::BTreeMap::new();
        let mut peeled = std::collections::BTreeMap::new();
        for line in output.lines() {
            let Some((oid, reference)) = line.split_once('\t') else {
                continue;
            };
            let Some(name) = reference.strip_prefix("refs/tags/") else {
                continue;
            };
            if let Some(name) = name.strip_suffix("^{}") {
                peeled.insert(name, oid);
            } else {
                tags.insert(name, oid);
            }
        }
        Ok(tags
            .into_iter()
            .map(|(name, oid)| RemoteTag {
                name: name.to_string(),
                oid: oid.to_string(),
                target: peeled.get(name).copied().unwrap_or(oid).to_string(),
            })
            .collect())
    }

    /// Delete exactly the advertised tag, failing if it moved since confirmation.
    pub fn delete_remote_tag(&self, remote: &str, tag: &RemoteTag) -> Result<(), GitError> {
        let reference = format!("refs/tags/{}", tag.name);
        self.run_git(&["check-ref-format", &reference])?;
        self.run_git(&[
            "push",
            &format!("--force-with-lease={reference}:{}", tag.oid),
            "--",
            remote,
            &format!(":{reference}"),
        ])?;
        Ok(())
    }

    /// 推送一个标签（`git push <remote> <tag>`）。
    pub fn push_tag(&self, tag: &str, remote: Option<&str>) -> Result<String, GitError> {
        let remote = remote.unwrap_or("origin");
        let reference = format!("refs/tags/{tag}");
        let output = self.run_git_combined(&["push", "--", remote, &reference])?;
        return Ok(summarise(&output));
    }

    /// 把指定本地分支推送到远端的同名分支，不要求它当前已检出。
    pub fn push_branch(&self, remote: &str, branch: &str) -> Result<String, GitError> {
        let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
        let output = self.run_git_combined(&["push", remote, refspec.as_str()])?;
        return Ok(summarise(&output));
    }

    /// 把一段补丁应用到暂存区（`git apply --cached`），用于 hunk / 行级暂存。
    ///
    /// `reverse` 为真时反向应用，即「取消暂存」。补丁不合法时 `git apply` 会拒绝，
    /// 这里把它的原始输出带回去，不做静默重试。
    pub fn apply_cached(&self, patch: &str, reverse: bool) -> Result<(), GitError> {
        if patch.trim().is_empty() {
            return Err(GitError::CommandFailed {
                program: "git".to_string(),
                args: "apply --cached".to_string(),
                code: None,
                detail: "补丁为空".to_string(),
            });
        }
        let directory = std::env::temp_dir();
        let stamp = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        );
        let path = directory.join(format!("gitclient-apply-{stamp}.patch"));
        std::fs::write(&path, patch.as_bytes()).map_err(|error| GitError::Unreadable {
            path: path.display().to_string(),
            reason: error.to_string(),
        })?;
        let mut args: Vec<&str> = vec!["apply", "--cached", "--unidiff-zero"];
        if reverse {
            args.push("--reverse");
        }
        args.push(path.to_str().unwrap_or_default());
        let result = self.run_git(&args);
        let _ = std::fs::remove_file(&path);
        result?;
        return Ok(());
    }
}

/// 在 `directory` 初始化一个空仓库（`git init -b main`）。
pub fn init_repository(directory: &str) -> Result<(), GitError> {
    return init_repository_with_executable(directory, &GitExecutable::system());
}

pub fn init_repository_with_executable(
    directory: &str,
    executable: &GitExecutable,
) -> Result<(), GitError> {
    return init_branch(directory, "main", executable);
}

/// 用指定分支初始化；分支为空时回退 `main`。
fn init_branch(directory: &str, branch: &str, executable: &GitExecutable) -> Result<(), GitError> {
    let branch = if branch.trim().is_empty() {
        "main"
    } else {
        branch.trim()
    };
    let path = Path::new(directory);
    std::fs::create_dir_all(path).map_err(|error| GitError::Unreadable {
        path: path.display().to_string(),
        reason: error.to_string(),
    })?;
    crate::exec::run_git_with(
        executable,
        Path::new("."),
        &["init", "-q", "-b", branch, directory],
    )?;
    return Ok(());
}

/// 初始化的附带文件与分支（设计板 03 的 Init 表单）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitOptions {
    /// 生成 `.gitignore`（通用最小集合，不绑定具体语言）。
    pub gitignore: bool,
    /// 生成 `README.md`。
    pub readme: bool,
    /// 生成 MIT `LICENSE`。
    pub license: bool,
    /// README 标题；为空时用目录名。
    pub name: String,
    /// 初始分支；为空时用 `main`。
    pub branch: String,
}

impl Default for InitOptions {
    fn default() -> Self {
        return Self {
            gitignore: false,
            readme: true,
            license: false,
            name: String::new(),
            branch: "main".to_string(),
        };
    }
}

/// 带附带文件的初始化；目录非空时只初始化、不追加文件。
pub fn init_with(directory: &str, options: &InitOptions) -> Result<(), GitError> {
    return init_with_executable(directory, options, &GitExecutable::system());
}

pub fn init_with_executable(
    directory: &str,
    options: &InitOptions,
    executable: &GitExecutable,
) -> Result<(), GitError> {
    init_branch(directory, options.branch.as_str(), executable)?;
    let root = Path::new(directory);
    let name = match options.name.trim() {
        "" => root
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "repository".to_string()),
        name => name.to_string(),
    };
    if options.gitignore && !root.join(".gitignore").exists() {
        std::fs::write(root.join(".gitignore"), DEFAULT_GITIGNORE).map_err(|error| {
            GitError::Unreadable {
                path: root.join(".gitignore").display().to_string(),
                reason: error.to_string(),
            }
        })?;
    }
    if options.readme && !root.join("README.md").exists() {
        std::fs::write(root.join("README.md"), format!("# {name}\n")).map_err(|error| {
            GitError::Unreadable {
                path: root.join("README.md").display().to_string(),
                reason: error.to_string(),
            }
        })?;
    }
    if options.license && !root.join("LICENSE").exists() {
        let holder = license_holder(root, name.as_str(), executable);
        let year = utc_year();
        std::fs::write(
            root.join("LICENSE"),
            format!("MIT License\n\nCopyright (c) {year} {holder}\n\n{MIT_BODY}\n"),
        )
        .map_err(|error| GitError::Unreadable {
            path: root.join("LICENSE").display().to_string(),
            reason: error.to_string(),
        })?;
    }
    return Ok(());
}

/// LICENSE 的版权人：优先 Git 用户名，其次邮箱，最后用仓库作者。
fn license_holder(root: &Path, name: &str, executable: &GitExecutable) -> String {
    for key in ["user.name", "user.email"] {
        if let Ok(bytes) = crate::exec::run_git_with(executable, root, &["config", "--get", key]) {
            let value = String::from_utf8_lossy(&bytes).trim().to_string();
            if !value.is_empty() {
                return value;
            }
        }
    }
    return format!("The {name} authors");
}

/// 当前 UTC 年份；系统时间不可用时回退 1970。
fn utc_year() -> i64 {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    // Howard Hinnant 的 civil_from_days，只取年份。
    let days = seconds.div_euclid(86_400) + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let doe = days - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = mp + if mp < 10 { 3 } else { -9 };
    return year + if month <= 2 { 1 } else { 0 };
}

/// 生成的 `.gitignore` 内容：通用最小集合（系统垃圾、编辑器目录、日志与环境文件）。
///
/// 不写具体语言或框架的条目：初始化面板只提供一个通用勾选项，替用户决定技术栈
/// 会在错误的时候写出错误的忽略规则。
const DEFAULT_GITIGNORE: &str =
    "\n# macOS\n.DS_Store\n\n# Editors\n.idea/\n.vscode/\n\n# Logs\n*.log\n\n# Environment\n.env\n";

/// MIT 许可正文；年份与版权人由调用处补在前两行。
const MIT_BODY: &str = "Permission is hereby granted, free of charge, to any person obtaining a copy\nof this software and associated documentation files (the \"Software\"), to deal\nin the Software without restriction, including without limitation the rights\nto use, copy, modify, merge, publish, distribute, sublicense, and/or sell\ncopies of the Software, and to permit persons to whom the Software is\nfurnished to do so, subject to the following conditions:\n\nThe above copyright notice and this permission notice shall be included in all\ncopies or substantial portions of the Software.\n\nTHE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR\nIMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,\nFITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE\nAUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER\nLIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,\nOUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE\nSOFTWARE.";

pub fn clone(url: &str, destination: &str) -> Result<(), GitError> {
    return clone_with_executable(
        url,
        destination,
        &CloneOptions::default(),
        &GitExecutable::system(),
    );
}

/// 克隆选项（设计板 03 的 Clone 表单）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CloneOptions {
    /// 只拉取指定分支（`--branch`）。
    pub branch: Option<String>,
    /// 浅克隆深度（`--depth`）；`0` 表示完整克隆。
    pub depth: u32,
    /// 递归拉取子模块（`--recurse-submodules`）。
    pub recurse_submodules: bool,
}

/// 带选项的克隆。目录非空时直接报错，不覆盖已有内容。
pub fn clone_with(url: &str, destination: &str, options: &CloneOptions) -> Result<(), GitError> {
    return clone_with_executable(url, destination, options, &GitExecutable::system());
}

pub fn clone_with_executable(
    url: &str,
    destination: &str,
    options: &CloneOptions,
    executable: &GitExecutable,
) -> Result<(), GitError> {
    let destination_path = Path::new(destination);
    if destination_path.exists() {
        let empty = std::fs::read_dir(destination_path)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false);
        if !empty {
            return Err(GitError::CommandFailed {
                program: "git".to_string(),
                args: "clone".to_string(),
                code: None,
                detail: format!("目标目录不是空目录：{destination}"),
            });
        }
    }
    if let Some(parent) = destination_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|error| GitError::Unreadable {
                path: parent.display().to_string(),
                reason: error.to_string(),
            })?;
        }
    }
    let destination_text = destination_path.display().to_string();
    let depth = options.depth.to_string();
    let mut args: Vec<&str> = vec!["clone"];
    if let Some(branch) = options.branch.as_deref()
        && !branch.trim().is_empty()
    {
        args.push("--branch");
        args.push(branch);
    }
    if options.depth > 0 {
        args.push("--depth");
        args.push(depth.as_str());
    }
    if options.recurse_submodules {
        args.push("--recurse-submodules");
    }
    args.push(url);
    args.push(destination_text.as_str());
    crate::exec::run_git_with(executable, Path::new("."), &args)?;
    return Ok(());
}

/// 一次正在进行的克隆：可轮询结果、读进度、随时取消。
///
/// `git clone` 的进度写在 stderr，所以这里用读线程把 stderr 收进共享缓冲，
/// 调用方（界面）在主线程轮询 [`CloneRun::progress`]，不需要阻塞界面。
pub struct CloneRun {
    child: std::process::Child,
    program: String,
    stderr: std::sync::Arc<std::sync::Mutex<String>>,
    reader: Option<std::thread::JoinHandle<()>>,
    destination: std::path::PathBuf,
}

impl CloneRun {
    /// 最近一行输出（git 的进度行）；没有输出时为空串。
    pub fn progress(&self) -> String {
        let text = self
            .stderr
            .lock()
            .map(|buffer| buffer.clone())
            .unwrap_or_default();
        return text
            .lines()
            .last()
            .map(|line| line.trim().to_string())
            .unwrap_or_default();
    }

    /// 轮询：还在跑返回 `None`，结束后返回克隆结果。
    pub fn try_finish(&mut self) -> Option<Result<(), GitError>> {
        let status = match self.child.try_wait() {
            Ok(status) => status,
            Err(error) => {
                return Some(Err(GitError::Spawn {
                    program: self.program.clone(),
                    reason: error.to_string(),
                }));
            }
        };
        let status = status?;
        // 进程结束后读线程会读到 EOF，join 它把最后几行收完。
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        if status.success() {
            return Some(Ok(()));
        }
        let detail = self
            .stderr
            .lock()
            .map(|buffer| buffer.clone())
            .unwrap_or_default();
        let detail = truncate(detail.trim(), MAX_DETAIL);
        return Some(Err(GitError::CommandFailed {
            program: self.program.clone(),
            args: "clone".to_string(),
            code: status.code(),
            detail,
        }));
    }

    /// 取消：杀掉子进程、等它退出，并删掉这次创建的目标目录。
    ///
    /// 只删除「本次克隆创建」的目录；用户给的目录如果原本就存在（空目录）也一并
    /// 清空，因为里面只可能是这次没写完的产物。
    pub fn cancel(&mut self) -> Result<(), GitError> {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        let removed = std::fs::remove_dir_all(&self.destination);
        if let Err(error) = removed
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(GitError::Unreadable {
                path: self.destination.display().to_string(),
                reason: error.to_string(),
            });
        }
        return Ok(());
    }
}

/// 启动一次可取消的克隆；目录非空时直接报错，不覆盖已有内容。
pub fn start_clone(
    url: &str,
    destination: &str,
    options: &CloneOptions,
) -> Result<CloneRun, GitError> {
    return start_clone_with_executable(url, destination, options, &GitExecutable::system());
}

pub fn start_clone_with_executable(
    url: &str,
    destination: &str,
    options: &CloneOptions,
    executable: &GitExecutable,
) -> Result<CloneRun, GitError> {
    let destination_path = Path::new(destination);
    if destination_path.exists() {
        let empty = std::fs::read_dir(destination_path)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false);
        if !empty {
            return Err(GitError::CommandFailed {
                program: "git".to_string(),
                args: "clone".to_string(),
                code: None,
                detail: format!("目标目录不是空目录：{destination}"),
            });
        }
    }
    if let Some(parent) = destination_path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent).map_err(|error| GitError::Unreadable {
            path: parent.display().to_string(),
            reason: error.to_string(),
        })?;
    }
    let destination_text = destination_path.display().to_string();
    let depth = options.depth.to_string();
    let mut args: Vec<&str> = vec!["clone", "--progress"];
    if let Some(branch) = options.branch.as_deref()
        && !branch.trim().is_empty()
    {
        args.push("--branch");
        args.push(branch);
    }
    if options.depth > 0 {
        args.push("--depth");
        args.push(depth.as_str());
    }
    if options.recurse_submodules {
        args.push("--recurse-submodules");
    }
    args.push(url);
    args.push(destination_text.as_str());
    let program = executable.path().display().to_string();
    let mut command = executable.command();
    command
        .arg("-C")
        .arg(".")
        .args(&args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null());
    let mut child = command.spawn().map_err(|error| GitError::Spawn {
        program: program.clone(),
        reason: error.to_string(),
    })?;
    let stderr = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let reader = child.stderr.take().map(|pipe| {
        let buffer = stderr.clone();
        std::thread::spawn(move || {
            use std::io::Read;
            let mut reader = std::io::BufReader::new(pipe);
            let mut chunk = [0u8; 512];
            loop {
                match reader.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => {
                        // 进度里含 `\r`，按行与回车都切开，只保留可读文本。
                        let text = String::from_utf8_lossy(&chunk[..read]).to_string();
                        if let Ok(mut buffer) = buffer.lock() {
                            buffer.push_str(text.replace('\r', "\n").as_str());
                            if buffer.len() > MAX_DETAIL * 4 {
                                let keep = buffer.len() - MAX_DETAIL * 2;
                                *buffer = buffer[keep..].to_string();
                            }
                        }
                    }
                }
            }
        })
    });
    return Ok(CloneRun {
        child,
        program,
        stderr,
        reader,
        destination: destination_path.to_path_buf(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::testkit::*;

    #[test]
    fn parses_push_porcelain_rows_and_ignores_human_lines() {
        let output = concat!(
            "To /remote.git\n",
            "  refs/heads/main:refs/heads/main abc1234..def5678\n",
            "* refs/heads/topic:refs/heads/topic [new branch]\n",
            "+ refs/heads/main:refs/heads/main abc1234...def5678 (forced update)\n",
            "- :refs/heads/old [deleted]\n",
            "= refs/heads/main:refs/heads/main [up to date]\n",
            "! refs/heads/blocked:refs/heads/blocked [rejected] (non-fast-forward)\n",
            "Done\n",
            "error: rejected\n",
        );

        let updates = parse_push_porcelain(output);

        assert_eq!(updates.len(), 6);
        assert_eq!(updates[0].status, PushRefStatus::Updated);
        assert_eq!(updates[0].source.as_deref(), Some("refs/heads/main"));
        assert_eq!(updates[0].destination, "refs/heads/main");
        assert_eq!(updates[1].status, PushRefStatus::Created);
        assert_eq!(updates[2].status, PushRefStatus::Forced);
        assert_eq!(updates[3].status, PushRefStatus::Deleted);
        assert_eq!(updates[3].source, None);
        assert_eq!(updates[4].status, PushRefStatus::UpToDate);
        assert_eq!(updates[5].status, PushRefStatus::Rejected);
    }

    #[cfg(unix)]
    #[test]
    fn sync_run_keeps_machine_output_separate_from_progress() {
        let mut command = Command::new("sh");
        command.args(["-c", "printf 'status-row\\n'; printf 'progress-row\\n' >&2"]);
        let mut run = SyncRun::spawn(command, &[]).expect("start fake Git");
        let mut result = None;
        while result.is_none() {
            result = run.try_finish();
            if result.is_none() {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        assert!(result.expect("finished").is_ok());
        assert_eq!(run.status_output(), "status-row\n");
        let output = run.progress();
        assert!(output.contains("status-row"));
        assert!(output.contains("progress-row"));
    }

    #[cfg(unix)]
    #[test]
    fn sync_run_reports_truncated_machine_output() {
        let mut command = Command::new("python3");
        command.args(["-c", "import sys; sys.stdout.write('x' * 300000)"]);
        let mut run = SyncRun::spawn(command, &[]).expect("start fake Git");
        let mut result = None;
        while result.is_none() {
            result = run.try_finish();
            if result.is_none() {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        assert!(result.expect("finished").is_ok());
        assert!(run.status_output_truncated());
        assert!(run.status_output().len() <= MAX_SYNC_OUTPUT);
    }

    #[test]
    fn push_and_fetch_report_results() {
        let (guard, path, _remote_guard, _remote) = with_remote("sync");
        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "two\n").expect("write");
        repo.stage_all().expect("stage");
        repo.commit("second", "").expect("commit");

        let pushed = repo.push(None).expect("push");
        assert!(!pushed.is_empty(), "推送应有输出");
        assert_eq!(repo.ahead_behind(), (0, 0));

        // 再取一次远端：没有新提交时也应有可展示的输出或空串，不应报错。
        repo.fetch("origin").expect("fetch");
        assert_eq!(repo.summary().expect("summary").ahead, 0);
        drop(guard);
    }

    #[test]
    fn pushes_a_named_branch_without_checking_it_out_again() {
        let (_guard, path, _remote_guard, remote) = with_remote("push-branch");
        let repo = Repo::open(&path).expect("open");
        repo.create_branch("topic/one", Some("main"))
            .expect("branch");
        repo.push_branch("origin", "topic/one")
            .expect("push branch");

        let remote_head = run_git(
            Path::new(remote.as_str()),
            &["rev-parse", "refs/heads/topic/one"],
        )
        .expect("remote branch");
        let local_head =
            run_git(repo.root(), &["rev-parse", "refs/heads/topic/one"]).expect("local branch");
        assert_eq!(remote_head, local_head);
        assert_eq!(repo.abbrev_branch().expect("current branch"), "main");
    }

    #[test]
    fn pull_fast_forwards_from_the_remote() {
        let (_guard, path, _remote_guard, remote) = with_remote("pull");
        // 用第二个克隆向远程推一个新提交。
        let other_dir = TempDir::new("pull-other");
        let other = other_dir.path().display().to_string();
        run_git(
            Path::new("."),
            &["clone", "-q", remote.as_str(), other.as_str()],
        )
        .expect("clone");
        run_git(Path::new(other.as_str()), &["config", "user.name", "Other"]).expect("name");
        run_git(
            Path::new(other.as_str()),
            &["config", "user.email", "o@example.com"],
        )
        .expect("mail");
        std::fs::write(Path::new(other.as_str()).join("b.txt"), "b\n").expect("write");
        run_git(Path::new(other.as_str()), &["add", "-A"]).expect("add");
        run_git(
            Path::new(other.as_str()),
            &["commit", "-q", "-m", "remote work"],
        )
        .expect("commit");
        run_git(Path::new(other.as_str()), &["push", "-q", "origin", "main"]).expect("push");

        let repo = Repo::open(&path).expect("open");
        repo.fetch("origin").expect("fetch");
        assert_eq!(repo.ahead_behind(), (0, 1), "应落后一个提交");
        repo.pull().expect("pull");
        assert_eq!(repo.ahead_behind(), (0, 0));
        assert_eq!(
            std::fs::read_to_string(repo.root().join("b.txt")).expect("read"),
            "b\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn sync_worker_does_not_wait_forever_for_output_after_git_exits() {
        let helper = TempDir::new("sync-output-child");
        let pid_file = helper.path().join("child.pid");
        let script = helper.path().join("fake-git");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\nsleep 60 &\necho $! > '{}'\nexit 0\n",
                pid_file.display()
            ),
        )
        .expect("write fake Git");
        let mut permissions = std::fs::metadata(&script).expect("metadata").permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o755);
        std::fs::set_permissions(&script, permissions).expect("make fake Git executable");

        let mut run = SyncRun::spawn(Command::new(&script), &[]).expect("start fake Git");
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            loop {
                if run.try_finish().is_some() {
                    let _ = sender.send(());
                    return;
                }
                if worker_cancel.load(std::sync::atomic::Ordering::Acquire) {
                    run.cancel();
                    let _ = sender.send(());
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });

        for _ in 0..200 {
            if pid_file.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(pid_file.exists(), "fake Git child should start");
        cancel.store(true, std::sync::atomic::Ordering::Release);
        let outcome = receiver.recv_timeout(std::time::Duration::from_millis(500));
        if outcome.is_err() {
            let pid = std::fs::read_to_string(&pid_file).expect("read child pid");
            let _ = std::process::Command::new("/bin/kill")
                .args(["-KILL", pid.trim()])
                .status();
            let _ = receiver.recv_timeout(std::time::Duration::from_secs(2));
        }
        assert!(
            outcome.is_ok(),
            "worker should not block joining output readers"
        );
    }

    #[cfg(unix)]
    #[test]
    fn sync_cancel_terminates_descendants_holding_output_pipes() {
        let helper = TempDir::new("sync-cancel-child");
        let pid_file = helper.path().join("child.pid");
        let script = helper.path().join("fake-git");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\nsleep 60 &\necho $! > '{}'\nwait\n",
                pid_file.display()
            ),
        )
        .expect("write fake Git");
        let mut permissions = std::fs::metadata(&script).expect("metadata").permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o755);
        std::fs::set_permissions(&script, permissions).expect("make fake Git executable");

        let run = SyncRun::spawn(Command::new(&script), &[]).expect("start fake Git");
        for _ in 0..200 {
            if pid_file.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(pid_file.exists(), "fake Git child should start");

        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut run = run;
            run.cancel();
            let _ = sender.send(());
        });
        let outcome = receiver.recv_timeout(std::time::Duration::from_millis(500));
        if outcome.is_err() {
            let pid = std::fs::read_to_string(&pid_file).expect("read child pid");
            let _ = std::process::Command::new("/bin/kill")
                .args(["-KILL", pid.trim()])
                .status();
            let _ = receiver.recv_timeout(std::time::Duration::from_secs(2));
        }
        assert!(
            outcome.is_ok(),
            "cancel should close inherited output pipes"
        );
    }

    #[cfg(unix)]
    #[test]
    fn sync_finish_does_not_wait_for_a_detached_output_descendant() {
        let helper = TempDir::new("sync-detached-output-child");
        let pid_file = helper.path().join("child.pid");
        let script = helper.path().join("fake-git");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\npython3 -c 'import os,sys,time; os.setsid(); open(sys.argv[1], chr(119)).write(str(os.getpid())); time.sleep(60)' '{}' &\nwhile [ ! -f '{}' ]; do sleep 0.01; done\nexit 0\n",
                pid_file.display(),
                pid_file.display()
            ),
        )
        .expect("write fake Git");
        let mut permissions = std::fs::metadata(&script).expect("metadata").permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o755);
        std::fs::set_permissions(&script, permissions).expect("make fake Git executable");

        let mut run = SyncRun::spawn(Command::new(&script), &[]).expect("start fake Git");
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            loop {
                if run.try_finish().is_some() {
                    let _ = sender.send(());
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });

        for _ in 0..200 {
            if pid_file.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(pid_file.exists(), "detached Git child should start");
        let outcome = receiver.recv_timeout(std::time::Duration::from_millis(500));
        let pid = std::fs::read_to_string(&pid_file).expect("read child pid");
        let _ = std::process::Command::new("/bin/kill")
            .args(["-KILL", pid.trim()])
            .status();
        if outcome.is_err() {
            let _ = receiver.recv_timeout(std::time::Duration::from_secs(2));
        }
        assert!(
            outcome.is_ok(),
            "try_finish should not block waiting for detached output descendants"
        );
    }

    #[cfg(unix)]
    #[test]
    fn sync_cancel_stops_readers_for_a_detached_output_descendant() {
        let helper = TempDir::new("sync-cancel-detached-output-child");
        let pid_file = helper.path().join("child.pid");
        let script = helper.path().join("fake-git");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\npython3 -c 'import os,sys,time; os.setsid(); open(sys.argv[1], chr(119)).write(str(os.getpid())); time.sleep(60)' '{}' &\nwhile [ ! -f '{}' ]; do sleep 0.01; done\nwait\n",
                pid_file.display(),
                pid_file.display()
            ),
        )
        .expect("write fake Git");
        let mut permissions = std::fs::metadata(&script).expect("metadata").permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o755);
        std::fs::set_permissions(&script, permissions).expect("make fake Git executable");

        let run = SyncRun::spawn(Command::new(&script), &[]).expect("start fake Git");
        for _ in 0..200 {
            if pid_file.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(pid_file.exists(), "detached Git child should start");

        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut run = run;
            run.cancel();
            let _ = sender.send(());
        });
        let outcome = receiver.recv_timeout(std::time::Duration::from_millis(500));
        let pid = std::fs::read_to_string(&pid_file).expect("read child pid");
        let _ = std::process::Command::new("/bin/kill")
            .args(["-KILL", pid.trim()])
            .status();
        if outcome.is_err() {
            let _ = receiver.recv_timeout(std::time::Duration::from_secs(2));
        }
        assert!(
            outcome.is_ok(),
            "cancel should not join readers held by detached descendants"
        );
    }

    #[test]
    fn pull_remote_branch_uses_the_requested_remote_ref() {
        let (_guard, path, _remote_guard, remote) = with_remote("pull-specific");
        let other_dir = TempDir::new("pull-specific-other");
        let other = other_dir.path().display().to_string();
        run_git(
            Path::new("."),
            &["clone", "-q", remote.as_str(), other.as_str()],
        )
        .expect("clone");
        run_git(Path::new(other.as_str()), &["config", "user.name", "Other"]).expect("name");
        run_git(
            Path::new(other.as_str()),
            &["config", "user.email", "other@example.com"],
        )
        .expect("email");
        std::fs::write(
            Path::new(other.as_str()).join("remote.txt"),
            "from server\n",
        )
        .expect("write");
        run_git(Path::new(other.as_str()), &["add", "-A"]).expect("add");
        run_git(
            Path::new(other.as_str()),
            &["commit", "-q", "-m", "remote update"],
        )
        .expect("commit");
        run_git(Path::new(other.as_str()), &["push", "-q", "origin", "main"]).expect("push");

        let repo = Repo::open(&path).expect("open");
        repo.pull_remote_branch("origin", "main")
            .expect("pull requested branch");
        assert_eq!(
            std::fs::read_to_string(repo.root().join("remote.txt")).expect("read"),
            "from server\n"
        );
    }

    #[test]
    fn delete_remote_branch_removes_server_ref_without_force() {
        let (_guard, path, _remote_guard, remote) = with_remote("delete-remote-branch");
        let repo = Repo::open(&path).expect("open");
        repo.create_branch("topic/remove", Some("main"))
            .expect("branch");
        repo.push_branch("origin", "topic/remove")
            .expect("push branch");

        repo.delete_remote_branch("origin", "topic/remove")
            .expect("delete remote branch");
        let result = run_git(
            Path::new(remote.as_str()),
            &["show-ref", "--verify", "--quiet", "refs/heads/topic/remove"],
        );
        assert!(result.is_err(), "远端分支应已删除");
        assert!(
            repo.refs()
                .expect("refs")
                .branches
                .iter()
                .any(|branch| branch.name == "topic/remove")
        );
    }

    #[test]
    fn push_failure_keeps_the_rejection_text() {
        let (_guard, path, _remote_guard, remote) = with_remote("pushfail");
        // 让远程前进，本地推送会被拒（non-fast-forward）。
        let other_dir = TempDir::new("pushfail-other");
        let other = other_dir.path().display().to_string();
        run_git(
            Path::new("."),
            &["clone", "-q", remote.as_str(), other.as_str()],
        )
        .expect("clone");
        run_git(Path::new(other.as_str()), &["config", "user.name", "Other"]).expect("name");
        run_git(
            Path::new(other.as_str()),
            &["config", "user.email", "o@example.com"],
        )
        .expect("mail");
        std::fs::write(Path::new(other.as_str()).join("c.txt"), "c\n").expect("write");
        run_git(Path::new(other.as_str()), &["add", "-A"]).expect("add");
        run_git(Path::new(other.as_str()), &["commit", "-q", "-m", "remote"]).expect("commit");
        run_git(Path::new(other.as_str()), &["push", "-q", "origin", "main"]).expect("push");

        let repo = Repo::open(&path).expect("open");
        std::fs::write(repo.root().join("a.txt"), "local\n").expect("write");
        repo.stage_all().expect("stage");
        repo.commit("local", "").expect("commit");
        let error = repo.push(None).expect_err("push should be rejected");
        let detail = error.detail();
        assert!(
            detail.to_lowercase().contains("reject")
                || detail.to_lowercase().contains("fetch first")
                || detail.to_lowercase().contains("non-fast-forward"),
            "推送拒绝的原因应在详情里：{}",
            detail
        );
    }

    #[test]
    fn sync_options_map_to_the_right_flags() {
        assert_eq!(PullStrategy::default(), PullStrategy::Merge);
        assert_eq!(PullStrategy::Rebase.flag(), "--rebase");
        assert_eq!(PullStrategy::FfOnly.flag(), "--ff-only");
        assert_eq!(PullStrategy::Merge.flag(), "--no-rebase");
        assert_eq!(PullStrategy::from_id("rebase"), PullStrategy::Rebase);
        assert_eq!(PullStrategy::from_id("ff-only"), PullStrategy::FfOnly);
        assert_eq!(PullStrategy::from_id("nonsense"), PullStrategy::Merge);
        assert_eq!(PullStrategy::Rebase.id(), "rebase");

        let options = FetchOptions {
            prune: true,
            tags: true,
            all_remotes: true,
        };
        assert!(options.all_remotes, "全部远端时不追加单个 remote 名");
        assert!(!PushOptions::default().force_with_lease, "强推默认关闭");
        assert!(!FetchOptions::default().prune, "取回默认不剪枝");
        assert!(!PullOptions::default().autostash);
    }

    #[test]
    fn push_with_force_lease_is_refused_when_the_remote_moved() {
        // with_remote 返回 (工作区守卫, 工作区路径, 远端守卫, 远端路径)。
        let (work_guard, work_path, remote_guard, remote_path) = with_remote("force-lease");
        let _ = (&work_guard, &remote_guard);
        let repo = Repo::open(&work_path).expect("open");
        // 另一个人推进远端。
        let other = TempDir::new("force-lease-other");
        run_git(other.path(), &["clone", "-q", remote_path.as_str(), "."]).expect("clone");
        run_git(other.path(), &["config", "user.name", "Other"]).expect("name");
        run_git(other.path(), &["config", "user.email", "o@example.com"]).expect("email");
        std::fs::write(other.path().join("other.txt"), "x\n").expect("write");
        run_git(other.path(), &["add", "-A"]).expect("add");
        run_git(other.path(), &["commit", "-q", "-m", "other"]).expect("commit");
        run_git(other.path(), &["push", "-q", "origin", "HEAD"]).expect("push");

        // 本地再提交，然后强推：lease 不成立，应被拒绝。
        std::fs::write(repo.root().join("mine.txt"), "y\n").expect("write");
        repo.stage_all().expect("stage");
        repo.commit("mine", "").expect("commit");
        let forced = repo.push_with(
            Some("origin"),
            &PushOptions {
                force_with_lease: true,
                ..PushOptions::default()
            },
        );
        assert!(forced.is_err(), "远端已前进时 --force-with-lease 必须失败");
    }

    #[test]
    fn clone_options_produce_the_expected_arguments() {
        // 选项到参数的映射：完整克隆不带额外参数。
        let defaults = CloneOptions::default();
        assert_eq!(defaults.depth, 0);
        assert!(!defaults.recurse_submodules);
        assert!(defaults.branch.is_none());

        // 真的克隆一次：浅克隆 + 指定分支 + 子模块开关都走通。
        let (work_guard, work_path, _remote_guard, remote_path) = with_remote("clone-opts");
        let target = TempDir::new("clone-opts-target");
        let destination = target.path().join("copy");
        let options = CloneOptions {
            branch: Some("main".to_string()),
            depth: 1,
            recurse_submodules: false,
        };
        clone_with(
            remote_path.as_str(),
            destination.display().to_string().as_str(),
            &options,
        )
        .expect("clone");
        assert!(destination.join("a.txt").exists(), "浅克隆应拿到文件");
        let repo = Repo::open(destination.display().to_string().as_str()).expect("open");
        assert_eq!(repo.abbrev_branch().expect("branch"), "main");
        // 浅克隆只有一个提交。
        assert_eq!(repo.log(10, "").expect("log").len(), 1);
        let _ = (&work_guard, &work_path);
    }

    #[test]
    fn clone_refuses_a_non_empty_directory() {
        let (_guard, path) = fixture("clone-dirty", &[("a.txt", "one\n")], "base");
        let error = clone("https://example.com/x.git", path.as_str());
        assert!(error.is_err(), "非空目录必须拒绝");
        let detail = error.unwrap_err().detail();
        assert!(detail.contains("不是空目录"), "错误应说明原因：{detail}");
    }

    #[test]
    fn init_with_branch_name_and_license() {
        let dir = TempDir::new("init-branch");
        let target = dir.path().join("custom");
        let target_text = target.display().to_string();
        init_with(
            target_text.as_str(),
            &InitOptions {
                gitignore: true,
                readme: true,
                license: true,
                name: "My Project".to_string(),
                branch: "trunk".to_string(),
            },
        )
        .expect("init");
        let repo = Repo::open(target_text.as_str()).expect("open");
        let branch = match repo.head_state() {
            crate::query::HeadState::Branch(branch) => branch,
            crate::query::HeadState::Unborn { branch } => branch,
            other => panic!("unexpected head: {other:?}"),
        };
        assert_eq!(branch, "trunk");
        let readme = std::fs::read_to_string(target.join("README.md")).expect("readme");
        assert!(readme.starts_with("# My Project"), "README: {readme}");
        let license = std::fs::read_to_string(target.join("LICENSE")).expect("license");
        assert!(license.starts_with("MIT License"));
        assert!(license.contains("Copyright (c)"));
        let ignore = std::fs::read_to_string(target.join(".gitignore")).expect("ignore");
        assert!(ignore.contains(".DS_Store"));
        assert!(
            !ignore.contains("DerivedData/"),
            "不再内置 Xcode/Swift 条目"
        );
    }

    #[test]
    fn init_with_writes_the_requested_files() {
        let dir = TempDir::new("init-files");
        let target = dir.path().join("fresh");
        let target_text = target.display().to_string();
        init_with(
            target_text.as_str(),
            &InitOptions {
                gitignore: true,
                readme: true,
                ..Default::default()
            },
        )
        .expect("init");
        assert!(target.join(".gitignore").exists());
        assert!(target.join("README.md").exists());
        let readme = std::fs::read_to_string(target.join("README.md")).expect("read");
        assert!(readme.starts_with("# fresh"), "README 应带仓库名：{readme}");

        // 已存在的文件不覆盖。
        std::fs::write(target.join("README.md"), "# mine\n").expect("write");
        init_with(
            target_text.as_str(),
            &InitOptions {
                gitignore: false,
                readme: true,
                ..Default::default()
            },
        )
        .expect("re-init");
        assert_eq!(
            std::fs::read_to_string(target.join("README.md")).expect("read"),
            "# mine\n"
        );
    }

    #[test]
    fn init_repository_creates_a_repo_on_main() {
        let base = std::env::temp_dir().join(format!(
            "git-init-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        let target = base.join("fresh");
        init_repository(&target.display().to_string()).expect("init");
        let repo = Repo::open(&target.display().to_string()).expect("open");
        assert_eq!(repo.abbrev_branch().unwrap_or_default(), "main");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn cancellable_clone_finishes_and_reports_progress() {
        let (guard, path) = fixture("clone-run", &[("a.txt", "one\n")], "first");
        let target = TempDir::new("clone-run-target");
        let destination = target.path().join("copy");
        let destination_text = destination.display().to_string();
        let mut run = start_clone(
            path.as_str(),
            destination_text.as_str(),
            &CloneOptions::default(),
        )
        .expect("start clone");
        // 本地仓库克隆很快：轮询到结束，最多等 10 秒（避免慢机器上偶发失败）。
        let mut outcome = None;
        for _ in 0..200 {
            if let Some(result) = run.try_finish() {
                outcome = Some(result);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let result = outcome.expect("克隆应在超时前结束");
        result.expect("克隆应成功");
        assert!(destination.join(".git").exists(), "克隆产物应存在");
        assert_eq!(
            std::fs::read_to_string(destination.join("a.txt")).expect("read"),
            "one\n"
        );
        let _ = guard;
    }

    #[test]
    fn cancelling_a_clone_removes_the_half_written_directory() {
        let (guard, path) = fixture("clone-cancel", &[("a.txt", "one\n")], "first");
        let target = TempDir::new("clone-cancel-target");
        let destination = target.path().join("gone");
        let destination_text = destination.display().to_string();
        let mut run = start_clone(
            path.as_str(),
            destination_text.as_str(),
            &CloneOptions::default(),
        )
        .expect("start clone");
        run.cancel().expect("cancel");
        assert!(!destination.exists(), "取消后不应留下半成品目录");
        let _ = guard;
    }
}
