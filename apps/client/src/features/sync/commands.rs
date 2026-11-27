//! 同步的用例：打开/关闭面板、切换选项、执行 fetch/pull/push 与清掉失败状态。
//!
//! 方法体由原 `Actions` 里的闭包原样搬来，只把捕获的局部变量换成 `self.*`；
//! 这里只调本功能的 `ops` 并更新状态，不写布局。

use super::ops;
use super::state::{SyncDialogState, SyncState, SyncSummary, SyncTaskState};
use crate::app::session::{Sync, SyncOptions, WorkflowProgress};
use crate::features::SessionHandle;
use nsh_core::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

enum SyncWorkerEvent {
    Progress(String),
    Finished {
        result: Option<Result<String, domain::GitError>>,
        summary: Option<SyncSummary>,
        output: String,
        cancelled: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SyncPresentation {
    Dialog,
    CommitFeedback,
}

enum SyncBefore {
    Fetch(Option<std::collections::BTreeMap<String, String>>),
    Pull {
        branch: String,
        upstream: Option<(String, String)>,
        head: Option<String>,
    },
    Push,
}

fn sync_before(repo: &git::Repo, kind: Sync) -> SyncBefore {
    return match kind {
        Sync::Fetch => SyncBefore::Fetch(repo.sync_ref_snapshot().ok()),
        Sync::Pull => SyncBefore::Pull {
            branch: repo.abbrev_branch().unwrap_or_else(|_| "HEAD".to_string()),
            upstream: repo.upstream_ref(),
            head: repo.head_oid(),
        },
        Sync::Push => SyncBefore::Push,
    };
}

fn completion_summary(
    repo: &git::Repo,
    kind: Sync,
    remote: &str,
    options: &SyncOptions,
    before: SyncBefore,
    succeeded: bool,
    status_output: &str,
    status_complete: bool,
) -> Option<SyncSummary> {
    match (kind, before) {
        (Sync::Fetch, SyncBefore::Fetch(before)) => {
            let Some(before) = before else {
                return succeeded.then(|| SyncSummary::Unknown {
                    kind,
                    target: if options.fetch.all_remotes {
                        "*".to_string()
                    } else {
                        remote.to_string()
                    },
                });
            };
            let Some(after) = repo.sync_ref_snapshot().ok() else {
                return succeeded.then(|| SyncSummary::Unknown {
                    kind,
                    target: if options.fetch.all_remotes {
                        "*".to_string()
                    } else {
                        remote.to_string()
                    },
                });
            };
            let names = before
                .keys()
                .chain(after.keys())
                .collect::<std::collections::BTreeSet<_>>();
            let mut updated_refs = 0usize;
            let mut deleted_refs = 0usize;
            let mut commits = std::collections::HashSet::new();
            let mut count_known = true;
            for name in names {
                let old = before.get(name);
                let new = after.get(name);
                if old == new {
                    continue;
                }
                match (old, new) {
                    (Some(_), None) => deleted_refs += 1,
                    (_, Some(new)) => {
                        updated_refs += 1;
                        match repo.commit_oids_between(old.map(String::as_str), new) {
                            Ok(oids) => commits.extend(oids),
                            Err(_) => count_known = false,
                        }
                    }
                    (None, None) => {}
                }
            }
            if !succeeded && updated_refs == 0 && deleted_refs == 0 {
                return None;
            }
            return Some(SyncSummary::Fetch {
                target: if options.fetch.all_remotes {
                    "*".to_string()
                } else {
                    remote.to_string()
                },
                updated_refs,
                deleted_refs,
                commits: count_known.then_some(commits.len()),
            });
        }
        (
            Sync::Pull,
            SyncBefore::Pull {
                branch,
                upstream,
                head,
            },
        ) if succeeded => {
            let new_upstream = repo.upstream_ref();
            let new_head = repo.head_oid();
            let commits = match (upstream, new_upstream, head, new_head) {
                (Some((old_ref, old_oid)), Some((new_ref, new_oid)), Some(_), Some(head_oid))
                    if old_ref == new_ref && repo.is_ancestor(&new_oid, &head_oid).ok()? =>
                {
                    Some(
                        repo.commit_oids_between(Some(old_oid.as_str()), new_oid.as_str())
                            .ok()?
                            .len(),
                    )
                }
                _ => None,
            };
            return Some(SyncSummary::Pull {
                target: branch,
                commits,
            });
        }
        (Sync::Push, SyncBefore::Push) => {
            if !status_complete {
                return succeeded.then(|| SyncSummary::Unknown {
                    kind,
                    target: remote.to_string(),
                });
            }
            let updates = git::parse_push_porcelain(status_output);
            if updates.is_empty() {
                return succeeded.then(|| SyncSummary::Unknown {
                    kind,
                    target: remote.to_string(),
                });
            }
            let mut updated_refs = 0usize;
            let mut deleted_refs = 0usize;
            let mut rejected_refs = 0usize;
            let mut targets = std::collections::BTreeSet::new();
            let mut commits = std::collections::HashSet::new();
            let mut count_known = true;
            let mut counted_an_update = false;
            for update in &updates {
                targets.insert(update.destination.clone());
                match update.status {
                    git::PushRefStatus::Updated
                    | git::PushRefStatus::Created
                    | git::PushRefStatus::Forced => {
                        updated_refs += 1;
                        counted_an_update = true;
                        let endpoints =
                            update.summary.split_whitespace().next().and_then(|value| {
                                value.split_once("...").or_else(|| value.split_once(".."))
                            });
                        let range = if let Some((old, new)) = endpoints {
                            repo.resolve_commit_oid(old)
                                .ok()
                                .zip(repo.resolve_commit_oid(new).ok())
                                .map(|(old, new)| (Some(old), new))
                        } else if update.status == git::PushRefStatus::Created {
                            update
                                .source
                                .as_deref()
                                .and_then(|source| repo.resolve_commit_oid(source).ok())
                                .map(|new| (None, new))
                        } else {
                            None
                        };
                        match range {
                            Some((old, new)) => {
                                match repo.commit_oids_between(old.as_deref(), new.as_str()) {
                                    Ok(oids) => commits.extend(oids),
                                    Err(_) => count_known = false,
                                }
                            }
                            None => count_known = false,
                        }
                    }
                    git::PushRefStatus::Deleted => deleted_refs += 1,
                    git::PushRefStatus::Rejected => rejected_refs += 1,
                    git::PushRefStatus::UpToDate => {}
                }
            }
            if !succeeded && updated_refs == 0 && deleted_refs == 0 && rejected_refs == 0 {
                return None;
            }
            let target = if targets.len() == 1 && !options.push.all_branches && !options.push.tags {
                let reference = targets.first().map(String::as_str).unwrap_or_default();
                format!("{remote}/{}", short_remote_ref(reference))
            } else {
                remote.to_string()
            };
            return Some(SyncSummary::Push {
                target,
                updated_refs,
                deleted_refs,
                commits: (count_known && counted_an_update).then_some(commits.len()),
                rejected_refs,
            });
        }
        _ => {}
    }
    return None;
}

fn short_remote_ref(reference: &str) -> &str {
    return reference
        .strip_prefix("refs/heads/")
        .or_else(|| reference.strip_prefix("refs/tags/"))
        .unwrap_or(reference);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::sync::state::SyncSummary;
    use crate::test_support::TempRepo;
    use std::path::Path;

    struct RemoteFixture {
        repo: TempRepo,
        remote: std::path::PathBuf,
        other: std::path::PathBuf,
    }

    impl RemoteFixture {
        fn new(name: &str) -> Self {
            let repo = TempRepo::new(name);
            let base = repo.path.parent().expect("temp parent");
            let suffix = repo
                .path
                .file_name()
                .expect("temp repo name")
                .to_string_lossy();
            let remote = base.join(format!("{suffix}-remote.git"));
            let other = base.join(format!("{suffix}-other"));
            git(
                &repo.path,
                &["init", "--bare", "-q", remote.to_str().expect("utf8")],
            );
            git(
                &repo.path,
                &["remote", "add", "origin", remote.to_str().expect("utf8")],
            );
            git(&repo.path, &["push", "-q", "-u", "origin", "main"]);
            git(
                Path::new("."),
                &[
                    "clone",
                    "-q",
                    remote.to_str().expect("utf8"),
                    other.to_str().expect("utf8"),
                ],
            );
            git(&other, &["config", "user.name", "Other"]);
            git(&other, &["config", "user.email", "other@example.com"]);
            return Self {
                repo,
                remote,
                other,
            };
        }

        fn other_commit(&self, message: &str) {
            let file = self.other.join("remote.txt");
            let contents = std::fs::read_to_string(&file).unwrap_or_default();
            std::fs::write(file, format!("{contents}{message}\n")).expect("write remote file");
            git(&self.other, &["add", "-A"]);
            git(&self.other, &["commit", "-q", "-m", message]);
            git(&self.other, &["push", "-q", "origin", "main"]);
        }
    }

    impl Drop for RemoteFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.other);
            let _ = std::fs::remove_dir_all(&self.remote);
        }
    }

    fn git(directory: &Path, args: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(directory)
            .args(args)
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {:?}: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        return String::from_utf8_lossy(&output.stdout).trim().to_string();
    }

    fn finish_run(run: &mut git::SyncRun) -> Result<String, domain::GitError> {
        for _ in 0..1000 {
            if let Some(result) = run.try_finish() {
                return result;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("local Git sync did not finish");
    }

    #[test]
    fn fetch_summary_counts_changed_refs_and_deduplicates_commits() {
        let fixture = RemoteFixture::new("sync-fetch-summary");
        fixture.other_commit("remote-1");
        git(
            &fixture.other,
            &["push", "-q", "origin", "main:refs/heads/topic"],
        );
        git(&fixture.other, &["tag", "-a", "v1", "-m", "v1"]);
        git(&fixture.other, &["push", "-q", "origin", "v1"]);
        let repo = git::Repo::open(fixture.repo.path.to_str().expect("repo path")).expect("repo");
        let options = SyncOptions::default();
        let before = sync_before(&repo, Sync::Fetch);
        let mut run = repo
            .start_fetch_with("origin", &options.fetch)
            .expect("start fetch");
        let result = finish_run(&mut run);
        let summary = completion_summary(
            &repo,
            Sync::Fetch,
            "origin",
            &options,
            before,
            result.is_ok(),
            run.status_output().as_str(),
            !run.status_output_truncated(),
        );

        assert!(result.is_ok());
        assert_eq!(
            summary,
            Some(SyncSummary::Fetch {
                target: "origin".to_string(),
                updated_refs: 3,
                deleted_refs: 0,
                commits: Some(2),
            })
        );

        let before = sync_before(&repo, Sync::Fetch);
        let mut run = repo
            .start_fetch_with("origin", &options.fetch)
            .expect("start unchanged fetch");
        let result = finish_run(&mut run);
        let unchanged = completion_summary(
            &repo,
            Sync::Fetch,
            "origin",
            &options,
            before,
            result.is_ok(),
            run.status_output().as_str(),
            !run.status_output_truncated(),
        );
        assert_eq!(
            unchanged,
            Some(SyncSummary::Fetch {
                target: "origin".to_string(),
                updated_refs: 0,
                deleted_refs: 0,
                commits: Some(0),
            })
        );

        git(
            &fixture.other,
            &["push", "-q", "origin", "--delete", "topic"],
        );
        let before = sync_before(&repo, Sync::Fetch);
        let mut prune_options = options.clone();
        prune_options.fetch.prune = true;
        let mut run = repo
            .start_fetch_with("origin", &prune_options.fetch)
            .expect("start pruned fetch");
        let result = finish_run(&mut run);
        let pruned = completion_summary(
            &repo,
            Sync::Fetch,
            "origin",
            &prune_options,
            before,
            result.is_ok(),
            run.status_output().as_str(),
            !run.status_output_truncated(),
        );
        assert!(result.is_ok());
        assert_eq!(
            pruned,
            Some(SyncSummary::Fetch {
                target: "origin".to_string(),
                updated_refs: 0,
                deleted_refs: 1,
                commits: Some(0),
            })
        );
    }

    #[test]
    fn pull_summary_uses_upstream_delta_that_reached_head() {
        let fixture = RemoteFixture::new("sync-pull-summary");
        fixture.other_commit("remote-1");
        let repo = git::Repo::open(fixture.repo.path.to_str().expect("repo path")).expect("repo");
        let before = sync_before(&repo, Sync::Pull);
        let options = SyncOptions::default();
        let mut run = repo.start_pull_with(&options.pull).expect("start pull");
        let result = finish_run(&mut run);
        let summary = completion_summary(
            &repo,
            Sync::Pull,
            "origin",
            &options,
            before,
            result.is_ok(),
            run.status_output().as_str(),
            !run.status_output_truncated(),
        );

        assert!(result.is_ok());
        assert_eq!(
            summary,
            Some(SyncSummary::Pull {
                target: "main".to_string(),
                commits: Some(1),
            })
        );
    }

    #[test]
    fn push_summary_reports_commits_and_rejected_refs() {
        let fixture = RemoteFixture::new("sync-push-summary");
        let repo = git::Repo::open(fixture.repo.path.to_str().expect("repo path")).expect("repo");
        std::fs::write(fixture.repo.path.join("local.txt"), "local\n").expect("write local file");
        git(&fixture.repo.path, &["add", "-A"]);
        git(&fixture.repo.path, &["commit", "-q", "-m", "local-1"]);
        let options = SyncOptions::default();
        let before = sync_before(&repo, Sync::Push);
        let mut run = repo
            .start_push_with(Some("origin"), &options.push)
            .expect("start push");
        let result = finish_run(&mut run);
        let summary = completion_summary(
            &repo,
            Sync::Push,
            "origin",
            &options,
            before,
            result.is_ok(),
            run.status_output().as_str(),
            !run.status_output_truncated(),
        );
        assert!(result.is_ok());
        assert_eq!(
            summary,
            Some(SyncSummary::Push {
                target: "origin/main".to_string(),
                updated_refs: 1,
                deleted_refs: 0,
                commits: Some(1),
                rejected_refs: 0,
            })
        );

        let before = sync_before(&repo, Sync::Push);
        let mut run = repo
            .start_push_with(Some("origin"), &options.push)
            .expect("start unchanged push");
        let result = finish_run(&mut run);
        let unchanged = completion_summary(
            &repo,
            Sync::Push,
            "origin",
            &options,
            before,
            result.is_ok(),
            run.status_output().as_str(),
            !run.status_output_truncated(),
        );
        assert!(result.is_ok());
        assert_eq!(
            unchanged,
            Some(SyncSummary::Push {
                target: "origin/main".to_string(),
                updated_refs: 0,
                deleted_refs: 0,
                commits: None,
                rejected_refs: 0,
            })
        );

        git(&fixture.other, &["pull", "-q", "--ff-only"]);
        fixture.other_commit("remote-1");
        std::fs::write(fixture.repo.path.join("local-2.txt"), "local\n")
            .expect("write second file");
        git(&fixture.repo.path, &["add", "-A"]);
        git(&fixture.repo.path, &["commit", "-q", "-m", "local-2"]);
        let before = sync_before(&repo, Sync::Push);
        let mut run = repo
            .start_push_with(Some("origin"), &options.push)
            .expect("start rejected push");
        let result = finish_run(&mut run);
        let summary = completion_summary(
            &repo,
            Sync::Push,
            "origin",
            &options,
            before,
            result.is_ok(),
            run.status_output().as_str(),
            !run.status_output_truncated(),
        );
        assert!(result.is_err());
        assert_eq!(
            summary,
            Some(SyncSummary::Push {
                target: "origin/main".to_string(),
                updated_refs: 0,
                deleted_refs: 0,
                commits: None,
                rejected_refs: 1,
            })
        );
    }
}

#[derive(Clone)]
pub(crate) struct SyncCommands {
    pub(crate) state: SyncState,
    pub(crate) session: SessionHandle,
    /// 确认框与结果提示要按当前语言取。
    pub(crate) language: Signal<SharedString>,
    /// 状态栏短提示（非快进回退的拉取结果）。
    pub(crate) notice: Signal<Option<String>>,
    pub(crate) askpass: crate::askpass::AskPassClient,
}

impl SyncCommands {
    fn active_tab_id(&self) -> Option<u64> {
        return self
            .session
            .tabs
            .get_untracked()
            .get(self.session.active.get_untracked())
            .map(|tab| tab.id);
    }

    fn default_push_target(&self, tab_id: Option<u64>) -> Option<(String, String, String)> {
        let tabs = self.session.tabs.get_untracked();
        let tab = tabs.iter().find(|tab| Some(tab.id) == tab_id)?;
        let (reference, _) = tab.open_repo().ok()?.upstream_ref()?;
        let tracking = tab
            .refs
            .remote_branches
            .iter()
            .find(|branch| reference == format!("refs/remotes/{}", branch.full_name()))?;
        Some((
            tracking.remote.clone(),
            tab.branch.clone(),
            tracking.name.clone(),
        ))
    }

    /// 打开同步面板；已经打开时只切 tab，保留当前选项（原 `open_sync`）。
    pub(crate) fn open_sync(&self, kind: Sync) {
        let sync_state = self.state.dialog;
        let active_tab_id = self.active_tab_id();
        if let Some(task) = self.state.task.get_untracked()
            && (task.result.is_none()
                || (task.show_dialog
                    && Some(task.target_tab_id) == active_tab_id
                    && task.kind == kind))
        {
            let running = task.result.is_none();
            sync_state.set(Some(SyncDialogState {
                kind: task.kind,
                target_tab_id: Some(task.target_tab_id),
                target_path: Some(task.path),
                remote: task.remote,
                options: task.options,
                result: task.result,
                summary: task.summary,
                running,
            }));
            return;
        }
        let Some(mut state) = sync_state.get_untracked() else {
            let mut state = SyncDialogState::new(kind, active_tab_id, None);
            state.target_path = active_tab_id.and_then(|id| {
                self.session
                    .tabs
                    .get_untracked()
                    .into_iter()
                    .find(|tab| tab.id == id)
                    .map(|tab| tab.path)
            });
            if kind == Sync::Push
                && let Some((remote, source, target)) = self.default_push_target(active_tab_id)
            {
                state.remote = Some(remote);
                state.options.push.branches = Some((source, target));
            }
            sync_state.set(Some(state));
            return;
        };
        state.kind = kind;
        state.result = None;
        state.summary = None;
        sync_state.set(Some(state));
    }

    /// 从远端节点打开同步面板，并锁定仓库与远端目标。
    pub(crate) fn open_sync_for_remote(&self, tab_id: u64, kind: Sync, remote: String) {
        let sync_state = self.state.dialog;
        if let Some(task) = self.state.task.get_untracked()
            && (task.result.is_none()
                || (task.show_dialog
                    && task.target_tab_id == tab_id
                    && task.kind == kind
                    && task.remote.as_deref() == Some(remote.as_str())))
        {
            let running = task.result.is_none();
            sync_state.set(Some(SyncDialogState {
                kind: task.kind,
                target_tab_id: Some(task.target_tab_id),
                target_path: Some(task.path),
                remote: task.remote,
                options: task.options,
                result: task.result,
                summary: task.summary,
                running,
            }));
            return;
        }
        let mut state = sync_state
            .get_untracked()
            .unwrap_or_else(|| SyncDialogState::new(kind, Some(tab_id), Some(remote.clone())));
        state.kind = kind;
        state.target_tab_id = Some(tab_id);
        state.target_path = self
            .session
            .tabs
            .get_untracked()
            .into_iter()
            .find(|tab| tab.id == tab_id)
            .map(|tab| tab.path);
        state.remote = Some(remote);
        state.options.fetch.all_remotes = false;
        state.result = None;
        state.summary = None;
        sync_state.set(Some(state));
    }

    pub(crate) fn open_push_for_branch(&self, tab_id: u64, branch: String) {
        let target = self.session.tabs.with_untracked(|tabs| {
            tabs.iter().find(|tab| tab.id == tab_id).and_then(|tab| {
                let remote = tab
                    .refs
                    .remotes
                    .iter()
                    .find(|name| name.as_str() == "origin")
                    .or_else(|| tab.refs.remotes.first())?
                    .clone();
                let destination = tab
                    .refs
                    .remote_branches
                    .iter()
                    .any(|item| item.remote == remote && item.name == branch);
                Some((
                    remote,
                    if destination {
                        branch.clone()
                    } else {
                        String::new()
                    },
                ))
            })
        });
        let Some((remote, destination)) = target else {
            return;
        };
        self.open_sync_for_remote(tab_id, Sync::Push, remote);
        self.state.dialog.update(|dialog| {
            if let Some(state) = dialog.as_mut().filter(|state| !state.running) {
                state.options.push.all_branches = false;
                state.options.push.branches = Some((branch, destination));
                state.result = None;
                state.summary = None;
            }
        });
    }

    /// 选项开关（原 `set_sync_option`）：键名固定，策略三项互斥。
    pub(crate) fn set_sync_option(&self, key: &'static str, value: bool) {
        let sync_state = self.state.dialog;
        let Some(mut state) = sync_state.get_untracked() else {
            return;
        };
        if state.running {
            return;
        }
        match key {
            "prune" => state.options.fetch.prune = value,
            "fetchTags" => state.options.fetch.tags = value,
            "allRemotes" => state.options.fetch.all_remotes = value,
            "autostash" => state.options.pull.autostash = value,
            "autostashPop" => state.options.pull.autostash_pop = value,
            "rebaseLocal" => state.options.pull.rebase_local = value,
            "pushTags" => state.options.push.tags = value,
            "forceLease" => state.options.push.force_with_lease = value,
            "allBranches" => state.options.push.all_branches = value,
            "strategyMerge" => state.options.pull.strategy = git::PullStrategy::Merge,
            "strategyRebase" => state.options.pull.strategy = git::PullStrategy::Rebase,
            "strategyFfOnly" => state.options.pull.strategy = git::PullStrategy::FfOnly,
            _ => return,
        }
        sync_state.set(Some(state));
    }

    /// 执行同步动作，并把结果留在面板里（原 `run_sync`）。
    ///
    /// 推送被远端拒绝（非快进）时不自动强推，而是弹「拉取变基 / 拉取合并 / 取消」。
    pub(crate) fn run_sync_to(
        &self,
        kind: Sync,
        branch_pair: Option<(String, String)>,
        new_target: String,
    ) {
        let Some(mut state) = self.state.dialog.get_untracked() else {
            return;
        };
        let Some(target_tab_id) = state.target_tab_id.or_else(|| self.active_tab_id()) else {
            return;
        };
        if kind == Sync::Push {
            state.options.push.branches = branch_pair;
            if state.remote.is_none() {
                state.remote = self.session.tabs.with_untracked(|tabs| {
                    tabs.iter()
                        .find(|tab| tab.id == target_tab_id)
                        .and_then(|tab| tab.refs.remotes.first().cloned())
                });
            }
        }
        if kind == Sync::Push
            && !state.options.push.all_branches
            && let Some((source, destination)) = &mut state.options.push.branches
        {
            if destination.is_empty() {
                *destination = new_target.trim().to_string();
            }
            if source.is_empty() || destination.is_empty() {
                return;
            }
        }
        self.start_sync_with(
            kind,
            target_tab_id,
            state.remote,
            state.options,
            None,
            false,
            SyncPresentation::Dialog,
            None,
        );
    }

    /// 提交成功后直接推送当前仓库；Force 复选框覆盖本次推送的同步面板选项。
    pub(crate) fn run_push_after_commit(&self, tab_id: u64, force_with_lease: bool) -> bool {
        let state = self
            .state
            .dialog
            .get_untracked()
            .filter(|state| state.target_tab_id == Some(tab_id));
        let (remote, mut options) = state
            .map(|state| (state.remote, state.options))
            .unwrap_or_default();
        options.push.branches = None;
        options.push.force_with_lease = force_with_lease;
        return self.start_sync_with(
            Sync::Push,
            tab_id,
            remote,
            options,
            None,
            false,
            SyncPresentation::CommitFeedback,
            None,
        );
    }

    fn start_sync_with(
        &self,
        kind: Sync,
        target_tab_id: u64,
        remote: Option<String>,
        options: SyncOptions,
        follow_up_push: Option<SyncOptions>,
        retry_push: bool,
        presentation: SyncPresentation,
        workflow_progress: Option<WorkflowProgress>,
    ) -> bool {
        if self
            .state
            .task
            .get_untracked()
            .is_some_and(|task| task.result.is_none())
        {
            return false;
        }

        let tabs = self.session.tabs;
        let Some(target) = tabs
            .get_untracked()
            .into_iter()
            .find(|tab| tab.id == target_tab_id)
        else {
            if presentation == SyncPresentation::Dialog
                && let Some(mut dialog) = self.state.dialog.get_untracked()
            {
                dialog.result = Some(Err(crate::i18n::t(
                    self.language.get_untracked().as_str(),
                    "form.targetClosed",
                )));
                dialog.summary = None;
                dialog.running = false;
                self.state.dialog.set(Some(dialog));
            }
            return false;
        };

        let path = target.path.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        if presentation == SyncPresentation::Dialog {
            self.state.dialog.set(Some(SyncDialogState {
                kind,
                target_tab_id: Some(target_tab_id),
                target_path: Some(path.clone()),
                remote: remote.clone(),
                options: options.clone(),
                result: None,
                summary: None,
                running: true,
            }));
        }
        self.state.task.set(Some(SyncTaskState {
            kind,
            target_tab_id,
            path: path.clone(),
            remote: remote.clone(),
            options: options.clone(),
            progress: String::new(),
            result: None,
            summary: None,
            cancelled: false,
            show_dialog: presentation == SyncPresentation::Dialog,
            workflow_progress,
            follow_up_push: follow_up_push.clone(),
            retry_push,
            cancel: cancel.clone(),
        }));

        let (sender, receiver) = mpsc::channel();
        let worker_cancel = cancel;
        let worker_path = path.clone();
        let worker_remote = remote.clone();
        let worker_options = options.clone();
        let git_executable = target.git_executable.clone();
        let askpass_client = self.askpass.clone();
        std::thread::spawn(move || {
            let askpass = match crate::askpass::AskPassSession::new(git_executable, askpass_client)
            {
                Ok(session) => session,
                Err(error) => {
                    let _ = sender.send(SyncWorkerEvent::Finished {
                        output: error.to_string(),
                        result: Some(Err(domain::GitError::Spawn {
                            program: "AskPass".to_string(),
                            reason: error.to_string(),
                        })),
                        summary: None,
                        cancelled: false,
                    });
                    return;
                }
            };
            let repo =
                match git::Repo::open_with_executable(worker_path.as_str(), askpass.executable()) {
                    Ok(repo) => repo,
                    Err(error) => {
                        let _ = sender.send(SyncWorkerEvent::Finished {
                            output: error.detail(),
                            result: Some(Err(error)),
                            summary: None,
                            cancelled: false,
                        });
                        return;
                    }
                };
            let remote_name = worker_remote
                .clone()
                .or_else(|| {
                    repo.refs()
                        .ok()
                        .and_then(|refs| refs.remotes.first().cloned())
                })
                .unwrap_or_else(|| "origin".to_string());
            let before = sync_before(&repo, kind);
            let started = match kind {
                Sync::Fetch => repo.start_fetch_with(remote_name.as_str(), &worker_options.fetch),
                Sync::Pull => repo.start_pull_with(&worker_options.pull),
                Sync::Push => {
                    repo.start_push_with(Some(remote_name.as_str()), &worker_options.push)
                }
            };
            let mut run = match started {
                Ok(run) => run,
                Err(error) => {
                    let _ = sender.send(SyncWorkerEvent::Finished {
                        output: error.detail(),
                        result: Some(Err(error)),
                        summary: None,
                        cancelled: false,
                    });
                    return;
                }
            };
            let mut last = String::new();
            loop {
                if let Some(result) = run.try_finish() {
                    let cancelled = askpass.was_cancelled();
                    let output = run.progress();
                    let status_output = run.status_output();
                    let status_complete = !run.status_output_truncated();
                    let summary = completion_summary(
                        &repo,
                        kind,
                        remote_name.as_str(),
                        &worker_options,
                        before,
                        result.is_ok(),
                        status_output.as_str(),
                        status_complete,
                    );
                    let _ = sender.send(SyncWorkerEvent::Finished {
                        result: if cancelled { None } else { Some(result) },
                        summary: if cancelled { None } else { summary },
                        output,
                        cancelled,
                    });
                    return;
                }
                if worker_cancel.load(Ordering::Acquire) {
                    let output = run.cancel();
                    let _ = sender.send(SyncWorkerEvent::Finished {
                        result: None,
                        summary: None,
                        output,
                        cancelled: true,
                    });
                    return;
                }
                let progress = run.progress();
                if progress != last {
                    last = progress.clone();
                    let _ = sender.send(SyncWorkerEvent::Progress(progress));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        });

        let task_signal = self.state.task;
        let dialog_signal = self.state.dialog;
        let language = self.language;
        let notice = self.notice;
        let commands = self.clone();
        // 回调当前 owner 已销毁；监听器跨越弹窗生命周期，写回前仍按 tab id 和路径核对目标。
        Owner::detached().run(|| {
            spawn(async move {
                loop {
                    match receiver.try_recv() {
                        Ok(SyncWorkerEvent::Progress(progress)) => {
                            if let Some(mut task) = task_signal.get_untracked() {
                                if task.target_tab_id == target_tab_id && task.path == path {
                                    task.progress = progress;
                                    task_signal.set(Some(task));
                                }
                            }
                        }
                        Ok(SyncWorkerEvent::Finished {
                            result,
                            summary,
                            output,
                            cancelled,
                        }) => {
                            let language_now = language.get_untracked();
                            let cancelled_key = match kind {
                                Sync::Fetch => "sync.cancelledFetch",
                                Sync::Pull => "sync.cancelledPull",
                                Sync::Push => "sync.cancelledPush",
                            };
                            let cancelled_text =
                                crate::i18n::t(language_now.as_str(), cancelled_key);
                            let localized_failure = result.as_ref().and_then(|result| {
                                result.as_ref().err().map(|error| {
                                    crate::features::welcome::commands::localized_error(
                                        language_now.as_str(),
                                        error,
                                    )
                                })
                            });
                            let mut tabs_now = tabs.get_untracked();
                            let target = tabs_now
                                .iter_mut()
                                .find(|tab| tab.id == target_tab_id && tab.path == path);
                            let mut rejected = false;
                            let outcome = if let Some(target) = target {
                                if cancelled {
                                    ops::finish_sync_cancelled(target, cancelled_text.clone());
                                } else if let Some(result) = result {
                                    ops::finish_sync_result(target, kind, result);
                                }
                                if let Some(message) = localized_failure.as_ref() {
                                    target.sync_message = Some(message.clone());
                                }
                                rejected = kind == Sync::Push
                                    && target.sync_failed
                                    && target.sync_fault == Some(domain::NetworkFault::Rejected);
                                let outcome =
                                    match (&target.sync_message, target.sync_failed, cancelled) {
                                        (_, _, true) => Err(cancelled_text.clone()),
                                        (Some(message), true, false) => Err(message.clone()),
                                        (Some(message), false, false) => Ok(message.clone()),
                                        (None, _, false) => Ok(String::new()),
                                    };
                                if presentation == SyncPresentation::CommitFeedback
                                    && kind == Sync::Push
                                {
                                    let detail = summary
                                        .as_ref()
                                        .map(|summary| {
                                            super::view::sync_summary_text(
                                                language_now.as_str(),
                                                summary,
                                                outcome.is_err(),
                                            )
                                        })
                                        .unwrap_or_else(|| match &outcome {
                                            Ok(message) | Err(message) => message.clone(),
                                        });
                                    target.commit_feedback_kind =
                                        crate::app::session::CommitFeedbackKind::Push;
                                    target.hook_ok = outcome.is_ok();
                                    target.commit_feedback_progress =
                                        target.commit_feedback_progress.map(|mut progress| {
                                            progress.step = progress.total;
                                            progress.running = false;
                                            progress
                                        });
                                    target.hook_log = Some(if output.trim().is_empty() {
                                        detail
                                    } else {
                                        format!("{detail}\n\n{}", output.trim())
                                    });
                                }
                                outcome
                            } else if cancelled {
                                Err(cancelled_text.clone())
                            } else if let Some(message) = localized_failure {
                                Err(message)
                            } else {
                                match result {
                                    Some(Ok(message)) => Ok(message),
                                    Some(Err(error)) => Err(error.detail()),
                                    None => Ok(String::new()),
                                }
                            };
                            let workflow_continues =
                                follow_up_push.is_some() && !cancelled && outcome.is_ok();
                            tabs.set(tabs_now);

                            if let Some(mut task) = task_signal.get_untracked() {
                                if task.target_tab_id == target_tab_id && task.path == path {
                                    task.progress = if output.trim().is_empty() {
                                        task.progress
                                    } else {
                                        output
                                    };
                                    task.result = Some(outcome.clone());
                                    task.summary = summary.clone();
                                    task.cancelled = cancelled;
                                    if let Some(mut progress) = task.workflow_progress {
                                        progress.running = workflow_continues;
                                        task.workflow_progress = Some(progress);
                                    }
                                    task_signal.set(Some(task));
                                }
                            }
                            if let Some(mut dialog) = dialog_signal.get_untracked()
                                && dialog.target_tab_id == Some(target_tab_id)
                                && dialog.kind == kind
                                && (presentation == SyncPresentation::Dialog || dialog.running)
                            {
                                dialog.running = false;
                                dialog.result = Some(outcome.clone());
                                dialog.summary = summary;
                                dialog_signal.set(Some(dialog));
                            }

                            if let Some(push_options) = follow_up_push {
                                if !cancelled && outcome.is_ok() {
                                    commands.start_sync_with(
                                        Sync::Push,
                                        target_tab_id,
                                        remote,
                                        push_options,
                                        None,
                                        true,
                                        SyncPresentation::Dialog,
                                        Some(WorkflowProgress {
                                            step: 3,
                                            total: 3,
                                            running: true,
                                        }),
                                    );
                                } else if let Err(message) = outcome {
                                    crate::interactions::notify(notice, message);
                                }
                                return;
                            }
                            if retry_push {
                                if let Err(message) = outcome {
                                    crate::interactions::notify(notice, message);
                                } else {
                                    crate::interactions::notify(
                                        notice,
                                        crate::i18n::t(
                                            language.get_untracked().as_str(),
                                            "toast.pushed",
                                        ),
                                    );
                                }
                                return;
                            }
                            if !rejected || options.push.branches.is_some() {
                                return;
                            }

                            let lang = language.get_untracked();
                            let pending = crate::ui::dialog::alert_with_actions(
                                crate::i18n::t(lang.as_str(), "confirm.nonFastForwardTitle"),
                                crate::i18n::t(lang.as_str(), "confirm.nonFastForwardMessage"),
                                vec![
                                    (
                                        "rebase",
                                        crate::i18n::t(
                                            lang.as_str(),
                                            "confirm.nonFastForwardRebase",
                                        ),
                                    ),
                                    (
                                        "merge",
                                        crate::i18n::t(
                                            lang.as_str(),
                                            "confirm.nonFastForwardMerge",
                                        ),
                                    ),
                                ],
                                None,
                                crate::i18n::t(lang.as_str(), "common.cancel"),
                            );
                            let Ok(answer) = pending.await else {
                                return;
                            };
                            let Some(action) = answer.action else {
                                return;
                            };
                            if action.as_str() == "cancel" {
                                return;
                            }
                            let strategy = if action.as_str() == "rebase" {
                                git::PullStrategy::Rebase
                            } else {
                                git::PullStrategy::Merge
                            };
                            let mut pull_options = options.clone();
                            pull_options.pull.strategy = strategy;
                            commands.start_sync_with(
                                Sync::Pull,
                                target_tab_id,
                                remote,
                                pull_options,
                                Some(options),
                                false,
                                SyncPresentation::Dialog,
                                Some(WorkflowProgress {
                                    step: 2,
                                    total: 3,
                                    running: true,
                                }),
                            );
                            return;
                        }
                        Err(mpsc::TryRecvError::Disconnected) => return,
                        Err(mpsc::TryRecvError::Empty) => timer(Duration::from_millis(100)).await,
                    }
                }
            });
        });
        return true;
    }

    pub(crate) fn cancel_sync(&self) {
        let task = self.state.task.get_untracked();
        if let Some(task) = task
            && task.result.is_none()
        {
            task.cancel.store(true, Ordering::Release);
        }
    }

    /// 关闭同步面板（原 `close_sync`）。
    pub(crate) fn close_sync(&self) {
        let sync_state = self.state.dialog;
        sync_state.set(None);
    }

    /// 清掉同步失败状态（原 `clear_sync_fault`，重试前使用）。
    pub(crate) fn clear_sync_fault(&self) {
        let tabs = self.session.tabs;
        let active = self.session.active;
        let current = active.get_untracked();
        let mut list = tabs.get_untracked();
        if let Some(tab) = list.get_mut(current) {
            ops::clear_sync_fault(tab);
            tabs.set(list);
        }
    }
}
