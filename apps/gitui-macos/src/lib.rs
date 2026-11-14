//! gitui M1 — NSH + Rust, read-only Git browser.
//!
//! Slice 1: open the current repository, show status, recent history and the
//! selected commit's patch. No staging, commit, push or settings.

mod audit;
mod design;
mod model;
mod sections;
mod widgets;

use gitui_agent::{AgentPermissions, AgentPlan, LocalPlanner, PlanStep, Planner};
use gitui_git::{BlameLine, ChangedFile, Commit, CommitDetails, PushPreview, ReflogEntry, Repo, StatusEntry};
use model::{load, load_recent, load_session, save_recent, save_session};
use sections::{agent_panel, audit_panel, changes_list, commit_info_panel, diff_view, history, inspect_panel, inspector, palette_panel, push_panel, refs_panel, reflog_panel, settings_panel, statusbar, sync_panel, titlebar, toolbar, AgentActions, AuditActions, ChangesActions, CommitActions, CommitInfoActions, ConflictActions, DiffActions, HeaderActions, HunkActions, InspectActions, PaletteCommand, PushActions, RefsActions, ReflogActions, StashActions, StatusActions, SyncActions, SettingsActions};
use nsh_base::theme;
use nsh_core::desktop::{TitleBarOptions, TitleBarStyle, WindowAttributes};
use nsh_core::element::desktop;
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    History,
    Changes,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SyncKind {
    Fetch,
    Pull,
    Push,
}

impl SyncKind {

    pub(crate) fn label(self) -> &'static str {
        match self {
            SyncKind::Fetch => "Fetch",
            SyncKind::Pull => "Pull",
            SyncKind::Push => "Push",
        }
    }
}

fn split_entries(entries: &[StatusEntry]) -> (Vec<StatusEntry>, Vec<StatusEntry>) {
    let staged = entries.iter().filter(|entry| entry.is_staged()).cloned().collect();
    let unstaged = entries.iter().filter(|entry| entry.is_unstaged()).cloned().collect();
    (staged, unstaged)
}

#[native_module("gitui/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefsTab {
    Branches,
    Tags,
    Remotes,
    Stashes,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum InspectorTab {
    Commit,
    Agent,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum InspectTab {
    Compare,
    FileHistory,
    Blame,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsTab {
    General,
    Git,
    Agent,
    Credentials,
    Shortcuts,
    About,
}

#[component]
pub fn App() -> impl IntoElement {
    let tokens = nsh_base::theme_overrides(design::THEME_JSON)
        .expect("the design tokens parse");
    nsh_core::element::set_tokens(tokens);
    let mut title_bar = TitleBarOptions::default();
    title_bar.style = TitleBarStyle::Hidden;
    title_bar.height = Some(Points(44.0));
    let mut window = WindowAttributes::default();
    window.title_bar = Some(title_bar);
    desktop::main_window().set(window);
    let initial_path = std::env::var("GITUI_REPO").unwrap_or_else(|_| {
        std::env::current_dir()
            .unwrap_or_default()
            .display()
            .to_string()
    });
    let initial = load(&initial_path);
    let (saved_active, saved_paths) = load_session();
    let mut initial_tabs = if saved_paths.is_empty() {
        vec![initial.clone()]
    } else {
        saved_paths.iter().map(|path| load(path)).collect::<Vec<_>>()
    };
    if initial_tabs.is_empty() {
        initial_tabs.push(initial.clone());
    }
    let initial_active = saved_active.min(initial_tabs.len() - 1);
    let active_tab = initial_tabs[initial_active].clone();

    let path_input = signal(active_tab.path.clone());
    let recent_path = active_tab.path.clone();
    let mut saved_recent = load_recent();
    if !saved_recent.iter().any(|path| path == &recent_path) {
        saved_recent.insert(0, recent_path);
    }
    saved_recent.truncate(5);
    let recent = signal(
        saved_recent
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    );
    save_recent(recent.get_untracked().as_slice());

    let repo_path = signal(SharedString::from(active_tab.path.clone()));
    let reflog_open = signal(false);
    let reflog_entries = signal(Vec::<ReflogEntry>::new());
    let reflog_selected = signal(SharedString::from(""));
    let reflog_error = signal(SharedString::from(""));
    let commit_info_open = signal(false);
    let commit_info = signal(None::<CommitDetails>);
    let commit_files = signal(Vec::<ChangedFile>::new());
    let push_open = signal(false);
    let push_preview = signal(None::<PushPreview>);
    let push_commits = signal(Vec::<Commit>::new());
    let inspect_open = signal(false);
    let inspect_tab = signal(InspectTab::Compare);
    let inspect_from = signal(String::from("HEAD~1"));
    let inspect_to = signal(String::from("HEAD"));
    let inspect_path = signal(String::new());
    let inspect_diff = signal(SharedString::from(""));
    let inspect_commits = signal(Vec::<Commit>::new());
    let inspect_blame = signal(Vec::<BlameLine>::new());
    let inspect_selected = signal(SharedString::from(""));
    let inspect_error = signal(SharedString::from(""));
    let agent_open = signal(false);
    let agent_steps = signal(Vec::<PlanStep>::new());
    let agent_log = signal(SharedString::from(""));
    let agent_error = signal(SharedString::from(""));
    let agent_protected_ok = signal(false);
    let agent_prompt = signal(String::new());
    let audit_open = signal(false);
    let audit_lines = signal(Vec::<String>::new());
    let audit_error = signal(SharedString::from(""));
    let palette_open = signal(false);
    let palette_query = signal(String::new());
    let open_palette: Arc<dyn Fn()> = Arc::new(move || {
        palette_query.set(String::new());
        palette_open.set(true);
    });
    on_key("mod+k", {
        let open = open_palette.clone();
        move |_| open()
    });
    on_key("escape", move |_| palette_open.set(false));
    let open_audit: Arc<dyn Fn()> = Arc::new(move || {
        audit_lines.set(crate::audit::read_recent(200));
        audit_error.set(SharedString::from(""));
        audit_open.set(true);
    });
    let open_agent: Arc<dyn Fn()> = Arc::new(move || {
        agent_steps.set(Vec::new());
        agent_log.set(SharedString::from(""));
        agent_error.set(SharedString::from(""));
        agent_protected_ok.set(false);
        agent_prompt.set(String::new());
        agent_open.set(true);
    });
    let open_compare: Arc<dyn Fn()> = Arc::new(move || {
        inspect_tab.set(InspectTab::Compare);
        inspect_diff.set(SharedString::from(""));
        inspect_error.set(SharedString::from(""));
        inspect_open.set(true);
    });
    let open_file_history: Arc<dyn Fn()> = Arc::new(move || {
        inspect_tab.set(InspectTab::FileHistory);
        inspect_error.set(SharedString::from(""));
        inspect_open.set(true);
    });
    let branch = signal(SharedString::from(active_tab.branch.clone()));
    let operation = signal(active_tab.operation);
    let conflicts = signal(active_tab.conflicts.clone());
    let branches = signal(active_tab.branches.clone());
    let tags = signal(active_tab.tags.clone());
    let stashes = signal(active_tab.stashes.clone());
    let remotes = signal(active_tab.remotes.clone());
    let remote_name = signal(String::new());
    let remote_url = signal(String::new());
    let identity_name = signal(active_tab.identity.name.clone());
    let identity_email = signal(active_tab.identity.email.clone());
    let selected_file = signal(String::new());
    let hunk_indices = signal(Vec::<usize>::new());
    let staged_hunk_indices = signal(Vec::<usize>::new());
    let file_patch = signal(SharedString::from(""));
    let staged_patch = signal(SharedString::from(""));
    let tag_name = signal(String::new());
    let branch_name = signal(String::new());
    let conflict_path = signal(active_tab.conflict_path.clone());
    let conflict_stages = signal(active_tab.conflict_stages.clone());
    let conflict_result = signal(
        active_tab
            .conflict_stages
            .as_ref()
            .map(|stages| stages.ours.clone())
            .unwrap_or_default(),
    );
    let ahead = signal(active_tab.ahead);
    let behind = signal(active_tab.behind);
    let staged = signal(active_tab.staged);
    let unstaged = signal(active_tab.unstaged);
    let status_lines = signal(active_tab.status_lines.clone());
    let status_entries = signal(active_tab.status_entries.clone());
    let (initial_staged_entries, initial_unstaged_entries) =
        split_entries(&active_tab.status_entries);
    let staged_entries = signal(initial_staged_entries);
    let unstaged_entries = signal(initial_unstaged_entries);
    let mode = signal(Mode::History);
    let set_mode: Arc<dyn Fn(Mode)> = Arc::new(move |next| mode.set(next));
    let sync_open = signal(false);
    let sync_kind = signal(SyncKind::Fetch);
    let sync_log = signal(SharedString::from(""));
    let settings_open = signal(false);
    let settings_tab = signal(SettingsTab::General);
    let agent_permissions = signal(AgentPermissions::default());
    let inspector_tab = signal(InspectorTab::Commit);
    let refs_open = signal(false);
    let refs_tab = signal(RefsTab::Branches);
    let open_refs: Arc<dyn Fn()> = Arc::new(move || {
        refs_tab.set(RefsTab::Branches);
        refs_open.set(true);
    });
    let open_settings: Arc<dyn Fn()> = Arc::new(move || {
        settings_tab.set(SettingsTab::General);
        settings_open.set(true);
    });
    let open_sync: Arc<dyn Fn()> = Arc::new(move || {
        sync_kind.set(SyncKind::Fetch);
        sync_log.set(SharedString::from(""));
        sync_open.set(true);
    });
    let commits = signal(active_tab.commits.clone());
    let selected = signal(active_tab.selected.clone());
    let diff = signal(active_tab.diff.clone());
    let error = signal(active_tab.error.clone());
    let message = signal(String::new());
    let commit_log = signal(SharedString::from(""));
    let tabs = signal(initial_tabs);
    let active = signal(initial_active);
    save_session(active.get_untracked(), tabs.get_untracked().as_slice());

    let reload = move || {
        let requested = path_input.get_untracked();
        let loaded = load(&requested);
        let first = loaded.selected.clone();
        let target = tabs
            .get_untracked()
            .iter()
            .position(|tab| tab.path == loaded.path);
        match target {
            Some(index) => {
                tabs.update(|list| list[index] = loaded.clone());
                active.set(index);
            }
            None => {
                tabs.update(|list| list.push(loaded.clone()));
                active.set(tabs.get_untracked().len() - 1);
            }
        }
        repo_path.set(SharedString::from(loaded.path.clone()));
        path_input.set(loaded.path.clone());
        branch.set(SharedString::from(loaded.branch));
        operation.set(loaded.operation);
        conflicts.set(loaded.conflicts);
        branches.set(loaded.branches);
        tags.set(loaded.tags);
        stashes.set(loaded.stashes);
        remotes.set(loaded.remotes);
        identity_name.set(loaded.identity.name);
        identity_email.set(loaded.identity.email);
        conflict_path.set(loaded.conflict_path);
        conflict_stages.set(loaded.conflict_stages.clone());
        conflict_result.set(
            loaded
                .conflict_stages
                .as_ref()
                .map(|stages| stages.ours.clone())
                .unwrap_or_default(),
        );
        ahead.set(loaded.ahead);
        behind.set(loaded.behind);
        staged.set(loaded.staged);
        unstaged.set(loaded.unstaged);
        status_lines.set(loaded.status_lines);
        let (loaded_staged, loaded_unstaged) = split_entries(&loaded.status_entries);
        status_entries.set(loaded.status_entries);
        staged_entries.set(loaded_staged);
        unstaged_entries.set(loaded_unstaged);
        commits.set(loaded.commits);
        selected.set(first);
        diff.set(loaded.diff);
        error.set(loaded.error);
        recent.update(|list| {
            if let Some(index) = list.iter().position(|item| item.to_string() == loaded.path) {
                list.remove(index);
            }
            list.insert(0, SharedString::from(loaded.path));
            list.truncate(5);
            save_recent(list.as_slice());
        });
        save_session(active.get_untracked(), tabs.get_untracked().as_slice());
    };

    let reload_for_recent = reload.clone();
    let reload_for_stage = reload.clone();
    let reload_for_unstage = reload.clone();
    let reload_for_commit = reload.clone();
    let reload_for_entry = reload.clone();
    let reload_for_sync = reload.clone();
    let reload_for_merge = reload.clone();
    let reload_for_conflict = reload.clone();
    let reload_for_branch = reload.clone();
    let reload_for_stash = reload.clone();
    let activate = move |index: usize| {
        let Some(tab) = tabs.get_untracked().get(index).cloned() else {
            return;
        };
        active.set(index);
        repo_path.set(SharedString::from(tab.path.clone()));
        path_input.set(tab.path.clone());
        branch.set(SharedString::from(tab.branch));
        operation.set(tab.operation);
        conflicts.set(tab.conflicts);
        branches.set(tab.branches);
        tags.set(tab.tags);
        stashes.set(tab.stashes);
        remotes.set(tab.remotes);
        identity_name.set(tab.identity.name);
        identity_email.set(tab.identity.email);
        conflict_path.set(tab.conflict_path);
        conflict_stages.set(tab.conflict_stages.clone());
        conflict_result.set(
            tab.conflict_stages
                .as_ref()
                .map(|stages| stages.ours.clone())
                .unwrap_or_default(),
        );
        ahead.set(tab.ahead);
        behind.set(tab.behind);
        staged.set(tab.staged);
        unstaged.set(tab.unstaged);
        status_lines.set(tab.status_lines);
        let (tab_staged, tab_unstaged) = split_entries(&tab.status_entries);
        status_entries.set(tab.status_entries);
        staged_entries.set(tab_staged);
        unstaged_entries.set(tab_unstaged);
        commits.set(tab.commits);
        selected.set(tab.selected);
        diff.set(tab.diff);
        error.set(tab.error);
        save_session(active.get_untracked(), tabs.get_untracked().as_slice());
    };
    let activate_for_close = activate.clone();
    let activate_tabs: Arc<dyn Fn(usize)> = Arc::new(move |index| activate(index));
    let titlebar = titlebar(tabs, active, activate_tabs);
    let close_active = move || {
        let index = active.get_untracked();
        let len = tabs.get_untracked().len();
        if len <= 1 || index >= len {
            return;
        }
        tabs.update(|list| {
            list.remove(index);
        });
        let next = if index >= len - 1 { len - 2 } else { index };
        activate_for_close(next);
    };

    let open_action = Arc::new(move || reload());
    let close_action = Arc::new(move || close_active());
    let fetch_action = Arc::new({
        let reload_sync = reload_for_sync.clone();
        move || match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.fetch())
        {
            Ok(_) => {
                error.set(SharedString::from(""));
                commit_log.set(SharedString::from(""));
                reload_sync();
            }
            Err(err) => {
                error.set(SharedString::from(err.to_string()));
                commit_log.set(SharedString::from(err.to_string()));
            }
        }
    });
    let pull_action = Arc::new({
        let reload_sync = reload_for_sync.clone();
        move || match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.pull_ff_only())
        {
            Ok(_) => {
                error.set(SharedString::from(""));
                commit_log.set(SharedString::from(""));
                reload_sync();
            }
            Err(err) => {
                error.set(SharedString::from(err.to_string()));
                commit_log.set(SharedString::from(err.to_string()));
            }
        }
    });
    let push_action = Arc::new({
        let reload_sync = reload_for_sync.clone();
        move || match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.push())
        {
            Ok(_) => {
                error.set(SharedString::from(""));
                commit_log.set(SharedString::from(""));
                reload_sync();
            }
            Err(err) => {
                error.set(SharedString::from(err.to_string()));
                commit_log.set(SharedString::from(err.to_string()));
            }
        }
    });
    let open_push: Arc<dyn Fn()> = Arc::new(move || {
        sync_open.set(false);
        error.set(SharedString::from(""));
        match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.push_preview())
        {
            Ok(preview) => {
                push_commits.set(preview.commits.clone());
                push_preview.set(Some(preview));
                push_open.set(true);
            }
            Err(err) => {
                push_commits.set(Vec::new());
                push_preview.set(None);
                error.set(SharedString::from(err.to_string()));
                push_open.set(true);
            }
        }
    });
    let confirm_push: Arc<dyn Fn()> = Arc::new({
        let push = push_action.clone();
        move || {
            push();
            push_open.set(false);
        }
    });

    let palette_fetch = fetch_action.clone();
    let palette_pull = pull_action.clone();
    let toolbar = toolbar(
        path_input,
        branch,
        ahead,
        behind,
        HeaderActions {
            open: open_action,
            close: close_action,
            fetch: fetch_action,
            pull: pull_action,
            push: open_push.clone(),
        },
        mode,
        set_mode.clone(),
        open_sync.clone(),
        open_settings.clone(),
        open_refs.clone(),
        open_palette.clone(),
    );

    let sync_close: Arc<dyn Fn()> = Arc::new(move || sync_open.set(false));
    let sync_select: Arc<dyn Fn(SyncKind)> = Arc::new(move |next| {
        sync_kind.set(next);
        sync_log.set(SharedString::from(""));
    });
    let sync_fetch_action = Arc::new({
        let reload = reload_for_sync.clone();
        move || {
            sync_log.set(SharedString::from("Fetching origin…"));
            match Repo::open(repo_path.get_untracked().to_string().as_str())
                .and_then(|repo| repo.fetch())
            {
                Ok(output) => {
                    sync_log.set(SharedString::from(output));
                    error.set(SharedString::from(""));
                    reload();
                }
                Err(err) => {
                    sync_log.set(SharedString::from(err.to_string()));
                    error.set(SharedString::from(err.to_string()));
                }
            }
        }
    });
    let sync_pull_action = Arc::new({
        let reload = reload_for_sync.clone();
        move || {
            sync_log.set(SharedString::from("Pulling origin…"));
            match Repo::open(repo_path.get_untracked().to_string().as_str())
                .and_then(|repo| repo.pull_ff_only())
            {
                Ok(output) => {
                    sync_log.set(SharedString::from(output));
                    error.set(SharedString::from(""));
                    reload();
                }
                Err(err) => {
                    sync_log.set(SharedString::from(err.to_string()));
                    error.set(SharedString::from(err.to_string()));
                }
            }
        }
    });
    let open_reflog: Arc<dyn Fn()> = Arc::new({
        let repo_path = repo_path;
        let reflog_open = reflog_open;
        let reflog_entries = reflog_entries;
        let reflog_selected = reflog_selected;
        let reflog_error = reflog_error;
        move || {
            match Repo::open(repo_path.get_untracked().to_string().as_str())
                .and_then(|repo| repo.reflog(80))
            {
                Ok(entries) => {
                    reflog_selected.set(
                        entries
                            .first()
                            .map(|entry| SharedString::from(entry.sha.clone()))
                            .unwrap_or_default(),
                    );
                    reflog_entries.set(entries);
                    reflog_error.set(SharedString::from(""));
                    reflog_open.set(true);
                }
                Err(err) => reflog_error.set(SharedString::from(err.to_string())),
            }
        }
    });
    let close_reflog: Arc<dyn Fn()> = Arc::new(move || reflog_open.set(false));
    let reset_reflog: Arc<dyn Fn(String)> = Arc::new({
        let reload = reload_for_branch.clone();
        move |sha: String| {
            match Repo::open(repo_path.get_untracked().to_string().as_str())
                .and_then(|repo| repo.reset_hard(&sha))
            {
                Ok(_) => {
                    reflog_error.set(SharedString::from(""));
                    reflog_open.set(false);
                    reload();
                }
                Err(err) => reflog_error.set(SharedString::from(err.to_string())),
            }
        }
    });

    let continue_action = Arc::new({
        let reload_merge = reload_for_merge.clone();
        move || match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.continue_operation())
        {
            Ok(_) => {
                error.set(SharedString::from(""));
                commit_log.set(SharedString::from(""));
                reload_merge();
            }
            Err(err) => {
                error.set(SharedString::from(err.to_string()));
                commit_log.set(SharedString::from(err.to_string()));
            }
        }
    });
    let abort_action = Arc::new({
        let reload_merge = reload_for_merge.clone();
        move || match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.abort_operation())
        {
            Ok(_) => {
                error.set(SharedString::from(""));
                commit_log.set(SharedString::from(""));
                reload_merge();
            }
            Err(err) => {
                error.set(SharedString::from(err.to_string()));
                commit_log.set(SharedString::from(err.to_string()));
            }
        }
    });
    let skip_action = Arc::new({
        let reload_skip = reload_for_merge.clone();
        move || match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.skip_operation())
        {
            Ok(_) => {
                error.set(SharedString::from(""));
                reload_skip();
            }
            Err(err) => error.set(SharedString::from(err.to_string())),
        }
    });
    let operation_banner = crate::sections::operation_banner(
        operation,
        continue_action.clone(),
        skip_action.clone(),
        abort_action.clone(),
    );

    let open_recent = Arc::new(move |path: String| {
        path_input.set(path);
        reload_for_recent();
    });
    let commit_action = Arc::new({
        let reload_commit = reload_for_commit.clone();
        move || {
            let text = message.get_untracked().trim().to_string();
            if text.is_empty() {
                error.set(SharedString::from("Commit message is empty"));
                return;
            }
            match Repo::open(repo_path.get_untracked().to_string().as_str())
                .and_then(|repo| repo.commit(&text))
            {
                Ok(_) => {
                    message.set(String::new());
                    error.set(SharedString::from(""));
                    commit_log.set(SharedString::from(""));
                    reload_commit();
                }
                Err(err) => {
                    error.set(SharedString::from(err.to_string()));
                    commit_log.set(SharedString::from(err.to_string()));
                }
            }
        }
    });
    let no_hooks_action = Arc::new({
        let reload_no_verify = reload_for_commit.clone();
        move || {
            let text = message.get_untracked().trim().to_string();
            if text.is_empty() {
                error.set(SharedString::from("Commit message is empty"));
                return;
            }
            match Repo::open(repo_path.get_untracked().to_string().as_str())
                .and_then(|repo| repo.commit_no_verify(&text))
            {
                Ok(_) => {
                    message.set(String::new());
                    error.set(SharedString::from(""));
                    commit_log.set(SharedString::from(""));
                    reload_no_verify();
                }
                Err(err) => {
                    error.set(SharedString::from(err.to_string()));
                    commit_log.set(SharedString::from(err.to_string()));
                }
            }
        }
    });
    let amend_action = Arc::new({
        let reload_amend = reload_for_commit.clone();
        move || {
            let text = message.get_untracked().trim().to_string();
            if text.is_empty() {
                error.set(SharedString::from("Commit message is empty"));
                return;
            }
            match Repo::open(repo_path.get_untracked().to_string().as_str())
                .and_then(|repo| repo.amend(&text))
            {
                Ok(_) => {
                    message.set(String::new());
                    error.set(SharedString::from(""));
                    commit_log.set(SharedString::from(""));
                    reload_amend();
                }
                Err(err) => {
                    error.set(SharedString::from(err.to_string()));
                    commit_log.set(SharedString::from(err.to_string()));
                }
            }
        }
    });
    let palette_commit = commit_action.clone();
    let commit_actions = CommitActions {
        commit: commit_action,
        no_hooks: no_hooks_action,
        amend: amend_action,
    };

    let accept_ours_action = Arc::new(move || {
        if let Some(stages) = conflict_stages.get_untracked().as_ref() {
            conflict_result.set(stages.ours.clone());
        }
    });
    let accept_theirs_action = Arc::new(move || {
        if let Some(stages) = conflict_stages.get_untracked().as_ref() {
            conflict_result.set(stages.theirs.clone());
        }
    });
    let accept_both_action = Arc::new(move || {
        if let Some(stages) = conflict_stages.get_untracked().as_ref() {
            conflict_result.set(format!("{}\n{}", stages.ours, stages.theirs));
        }
    });
    let mark_resolved_action = Arc::new({
        let reload_conflict = reload_for_conflict.clone();
        move || {
            match Repo::open(repo_path.get_untracked().to_string().as_str()).and_then(|repo| {
                repo.resolve(
                    conflict_path.get_untracked().to_string().as_str(),
                    conflict_result.get_untracked().to_string().as_str(),
                )
            }) {
                Ok(()) => {
                    error.set(SharedString::from(""));
                    reload_conflict();
                }
                Err(err) => error.set(SharedString::from(err.to_string())),
            }
        }
    });
    let stage_all_action = Arc::new({
        let reload_stage = reload_for_stage.clone();
        move || match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.stage_all())
        {
            Ok(()) => {
                error.set(SharedString::from(""));
                reload_stage();
            }
            Err(err) => error.set(SharedString::from(err.to_string())),
        }
    });
    let unstage_all_action = Arc::new({
        let reload_unstage = reload_for_unstage.clone();
        move || match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.unstage_all())
        {
            Ok(()) => {
                error.set(SharedString::from(""));
                reload_unstage();
            }
            Err(err) => error.set(SharedString::from(err.to_string())),
        }
    });
    let stage_path_action = Arc::new({
        let reload_entry = reload_for_entry.clone();
        move |path: String| match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.stage_path(&path))
        {
            Ok(()) => {
                error.set(SharedString::from(""));
                reload_entry();
            }
            Err(err) => error.set(SharedString::from(err.to_string())),
        }
    });
    let unstage_path_action = Arc::new({
        let reload_entry = reload_for_entry.clone();
        move |path: String| match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.unstage_path(&path))
        {
            Ok(()) => {
                error.set(SharedString::from(""));
                reload_entry();
            }
            Err(err) => error.set(SharedString::from(err.to_string())),
        }
    });
    let checkout_action = Arc::new({
        let reload_branch = reload_for_branch.clone();
        move |name: String| match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.checkout(&name))
        {
            Ok(_) => {
                error.set(SharedString::from(""));
                reload_branch();
            }
            Err(err) => error.set(SharedString::from(err.to_string())),
        }
    });
    let create_branch_action = Arc::new({
        let reload_branch = reload_for_branch.clone();
        move |name: String| {
            if name.is_empty() {
                error.set(SharedString::from("Branch name is empty"));
                return;
            }
            match Repo::open(repo_path.get_untracked().to_string().as_str())
                .and_then(|repo| repo.create_branch(&name))
            {
                Ok(_) => {
                    branch_name.set(String::new());
                    error.set(SharedString::from(""));
                    reload_branch();
                }
                Err(err) => error.set(SharedString::from(err.to_string())),
            }
        }
    });
    let delete_branch_action = Arc::new({
        let reload_branch = reload_for_branch.clone();
        move |name: String| {
            if name.is_empty() {
                error.set(SharedString::from("Branch name is empty"));
                return;
            }
            match Repo::open(repo_path.get_untracked().to_string().as_str())
                .and_then(|repo| repo.delete_branch(&name))
            {
                Ok(_) => {
                    branch_name.set(String::new());
                    error.set(SharedString::from(""));
                    reload_branch();
                }
                Err(err) => error.set(SharedString::from(err.to_string())),
            }
        }
    });
    let create_tag_action = Arc::new({
        let reload_tag = reload_for_branch.clone();
        move |name: String| {
            if name.is_empty() {
                error.set(SharedString::from("Tag name is empty"));
                return;
            }
            match Repo::open(repo_path.get_untracked().to_string().as_str())
                .and_then(|repo| repo.create_tag(&name))
            {
                Ok(_) => {
                    tag_name.set(String::new());
                    error.set(SharedString::from(""));
                    reload_tag();
                }
                Err(err) => error.set(SharedString::from(err.to_string())),
            }
        }
    });
    let delete_tag_action = Arc::new({
        let reload_tag = reload_for_branch.clone();
        move |name: String| {
            if name.is_empty() {
                error.set(SharedString::from("Tag name is empty"));
                return;
            }
            match Repo::open(repo_path.get_untracked().to_string().as_str())
                .and_then(|repo| repo.delete_tag(&name))
            {
                Ok(_) => {
                    tag_name.set(String::new());
                    error.set(SharedString::from(""));
                    reload_tag();
                }
                Err(err) => error.set(SharedString::from(err.to_string())),
            }
        }
    });
    let stash_push_action = Arc::new({
        let reload_stash = reload_for_stash.clone();
        move || match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.stash_push(""))
        {
            Ok(_) => {
                error.set(SharedString::from(""));
                reload_stash();
            }
            Err(err) => error.set(SharedString::from(err.to_string())),
        }
    });
    let stash_apply_action = Arc::new({
        let reload_stash = reload_for_stash.clone();
        move |selector: String| match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.stash_apply(&selector))
        {
            Ok(_) => {
                error.set(SharedString::from(""));
                reload_stash();
            }
            Err(err) => error.set(SharedString::from(err.to_string())),
        }
    });
    let stash_pop_action = Arc::new({
        let reload_stash = reload_for_stash.clone();
        move |selector: String| match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.stash_pop(&selector))
        {
            Ok(_) => {
                error.set(SharedString::from(""));
                reload_stash();
            }
            Err(err) => error.set(SharedString::from(err.to_string())),
        }
    });
    let stash_drop_action = Arc::new({
        let reload_stash = reload_for_stash.clone();
        move |selector: String| match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.stash_drop(&selector))
        {
            Ok(_) => {
                error.set(SharedString::from(""));
                reload_stash();
            }
            Err(err) => error.set(SharedString::from(err.to_string())),
        }
    });
    let rename_branch_action = Arc::new({
        let reload_branch = reload_for_branch.clone();
        move |new_name: String| {
            if new_name.is_empty() {
                error.set(SharedString::from("Branch name is empty"));
                return;
            }
            let current = branch.get_untracked().to_string();
            match Repo::open(repo_path.get_untracked().to_string().as_str())
                .and_then(|repo| repo.rename_branch(&current, &new_name))
            {
                Ok(_) => {
                    branch_name.set(String::new());
                    error.set(SharedString::from(""));
                    reload_branch();
                }
                Err(err) => error.set(SharedString::from(err.to_string())),
            }
        }
    });
    let add_remote_action = Arc::new({
        let reload_remote = reload_for_branch.clone();
        move |name: String, url: String| {
            if name.is_empty() || url.is_empty() {
                error.set(SharedString::from("Remote name and URL are required"));
                return;
            }
            match Repo::open(repo_path.get_untracked().to_string().as_str())
                .and_then(|repo| repo.add_remote(&name, &url))
            {
                Ok(_) => {
                    remote_name.set(String::new());
                    remote_url.set(String::new());
                    error.set(SharedString::from(""));
                    reload_remote();
                }
                Err(err) => error.set(SharedString::from(err.to_string())),
            }
        }
    });
    let remove_remote_action = Arc::new({
        let reload_remote = reload_for_branch.clone();
        move |name: String| {
            if name.is_empty() {
                error.set(SharedString::from("Remote name is empty"));
                return;
            }
            match Repo::open(repo_path.get_untracked().to_string().as_str())
                .and_then(|repo| repo.remove_remote(&name))
            {
                Ok(_) => {
                    error.set(SharedString::from(""));
                    reload_remote();
                }
                Err(err) => error.set(SharedString::from(err.to_string())),
            }
        }
    });
    let select_file_action = Arc::new(move |path: String| {
        selected_file.set(path.clone());
        match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.diff_unstaged(&path))
        {
            Ok(patch) => {
                let count = patch.lines().filter(|line| line.starts_with("@@")).count();
                hunk_indices.set((0..count).collect());
                file_patch.set(SharedString::from(patch));
                if let Ok(repo) =
                    Repo::open(repo_path.get_untracked().to_string().as_str())
                {
                    if let Ok(staged) = repo.diff_staged(&path) {
                        let staged_count =
                            staged.lines().filter(|line| line.starts_with("@@")).count();
                        staged_hunk_indices.set((0..staged_count).collect());
                        staged_patch.set(SharedString::from(staged));
                    }
                }
                error.set(SharedString::from(""));
            }
            Err(err) => {
                hunk_indices.set(Vec::new());
                file_patch.set(SharedString::from(""));
                error.set(SharedString::from(err.to_string()));
            }
        }
    });
    let stage_hunk_action = Arc::new({
        let reload_hunk = reload_for_entry.clone();
        move |index: usize| {
            let path = selected_file.get_untracked().to_string();
            if path.is_empty() {
                return;
            }
            match Repo::open(repo_path.get_untracked().to_string().as_str())
                .and_then(|repo| repo.stage_hunk(&path, index))
            {
                Ok(()) => {
                    error.set(SharedString::from(""));
                    if let Ok(repo) = Repo::open(repo_path.get_untracked().to_string().as_str()) {
                        if let Ok(patch) = repo.diff_unstaged(&path) {
                            let count =
                                patch.lines().filter(|line| line.starts_with("@@")).count();
                            hunk_indices.set((0..count).collect());
                            file_patch.set(SharedString::from(patch));
                        }
                        if let Ok(staged) = repo.diff_staged(&path) {
                            let staged_count =
                                staged.lines().filter(|line| line.starts_with("@@")).count();
                            staged_hunk_indices.set((0..staged_count).collect());
                            staged_patch.set(SharedString::from(staged));
                        }
                    }
                    reload_hunk();
                }
                Err(err) => error.set(SharedString::from(err.to_string())),
            }
        }
    });
    let unstage_hunk_action = Arc::new({
        let reload_hunk = reload_for_entry.clone();
        move |index: usize| {
            let path = selected_file.get_untracked().to_string();
            if path.is_empty() {
                return;
            }
            match Repo::open(repo_path.get_untracked().to_string().as_str())
                .and_then(|repo| repo.unstage_hunk(&path, index))
            {
                Ok(()) => {
                    error.set(SharedString::from(""));
                    if let Ok(repo) = Repo::open(repo_path.get_untracked().to_string().as_str()) {
                        if let Ok(patch) = repo.diff_unstaged(&path) {
                            let count =
                                patch.lines().filter(|line| line.starts_with("@@")).count();
                            hunk_indices.set((0..count).collect());
                            file_patch.set(SharedString::from(patch));
                        }
                        if let Ok(staged) = repo.diff_staged(&path) {
                            let staged_count =
                                staged.lines().filter(|line| line.starts_with("@@")).count();
                            staged_hunk_indices.set((0..staged_count).collect());
                            staged_patch.set(SharedString::from(staged));
                        }
                    }
                    reload_hunk();
                }
                Err(err) => error.set(SharedString::from(err.to_string())),
            }
        }
    });
    let hunk_actions = HunkActions {
        stage: stage_hunk_action,
        unstage: unstage_hunk_action,
    };
    let save_identity_action = Arc::new({
        let reload_identity = reload_for_branch.clone();
        move || {
            let name = identity_name.get_untracked().trim().to_string();
            let email = identity_email.get_untracked().trim().to_string();
            match Repo::open(repo_path.get_untracked().to_string().as_str())
                .and_then(|repo| repo.set_identity(&name, &email))
            {
                Ok(()) => {
                    error.set(SharedString::from(""));
                    reload_identity();
                }
                Err(err) => error.set(SharedString::from(err.to_string())),
            }
        }
    });
    let close_settings: Arc<dyn Fn()> = Arc::new(move || settings_open.set(false));
    let settings_panel = settings_panel(
        settings_open,
        settings_tab,
        identity_name,
        identity_email,
        recent,
        agent_permissions,
        SettingsActions {
            close: close_settings,
            save_identity: save_identity_action,
            open_recent: Arc::new({
                let open = open_recent.clone();
                let settings_open = settings_open;
                move |path| {
                    settings_open.set(false);
                    open(path);
                }
            }),
            view_audit: open_audit.clone(),
        },
    );
    let merge_branch_action = Arc::new({
        let reload_merge = reload_for_branch.clone();
        move |name: String| {
            if name.is_empty() {
                error.set(SharedString::from("Branch name is empty"));
                return;
            }
            match Repo::open(repo_path.get_untracked().to_string().as_str())
                .and_then(|repo| repo.merge(&name))
            {
                Ok(_) => {
                    error.set(SharedString::from(""));
                    reload_merge();
                }
                Err(err) => {
                    error.set(SharedString::from(err.to_string()));
                    reload_merge();
                }
            }
        }
    });
    let rebase_branch_action = Arc::new({
        let reload_rebase = reload_for_branch.clone();
        move |name: String| {
            if name.is_empty() {
                error.set(SharedString::from("Branch name is empty"));
                return;
            }
            match Repo::open(repo_path.get_untracked().to_string().as_str())
                .and_then(|repo| repo.rebase(&name))
            {
                Ok(_) => {
                    error.set(SharedString::from(""));
                    reload_rebase();
                }
                Err(err) => {
                    error.set(SharedString::from(err.to_string()));
                    reload_rebase();
                }
            }
        }
    });
    let select_conflict_action = Arc::new(move |selected: String| {
        match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.conflict_stages(&selected))
        {
            Ok(stages) => {
                conflict_path.set(selected);
                conflict_result.set(stages.ours.clone());
                conflict_stages.set(Some(stages));
                error.set(SharedString::from(""));
            }
            Err(err) => error.set(SharedString::from(err.to_string())),
        }
    });
    let conflict_panel = crate::sections::conflict_panel(
        conflicts,
        conflict_path,
        conflict_stages,
        conflict_result,
        ConflictActions {
            accept_ours: accept_ours_action,
            accept_theirs: accept_theirs_action,
            accept_both: accept_both_action,
            mark_resolved: mark_resolved_action,
            select: select_conflict_action.clone(),
            continue_op: continue_action.clone(),
            abort: abort_action.clone(),
        },
    );

    let close_refs: Arc<dyn Fn()> = Arc::new(move || refs_open.set(false));
    let refs_panel = refs_panel(
        refs_open,
        refs_tab,
        branches,
        tags,
        branch_name,
        tag_name,
        remotes,
        remote_name,
        remote_url,
        stashes,
        RefsActions {
            close: close_refs,
            checkout: checkout_action.clone(),
            delete_branch: delete_branch_action.clone(),
            add_remote: add_remote_action.clone(),
            remove_remote: remove_remote_action.clone(),
            stash: StashActions {
                push: stash_push_action.clone(),
                apply: stash_apply_action.clone(),
                pop: stash_pop_action.clone(),
                drop: stash_drop_action.clone(),
            },
            branch_name: Arc::new({
                let action = create_branch_action.clone();
                move || action(branch_name.get_untracked().trim().to_string())
            }),
            create_tag: Arc::new({
                let action = create_tag_action.clone();
                move || action(tag_name.get_untracked().trim().to_string())
            }),
            delete_tag: Arc::new({
                let action = delete_tag_action.clone();
                move || action(tag_name.get_untracked().trim().to_string())
            }),
        },
    );
    let palette_stage_all = stage_all_action.clone();
    let changes_actions = ChangesActions {
        stage_all: stage_all_action.clone(),
        unstage_all: unstage_all_action.clone(),
        stage_path: stage_path_action.clone(),
        unstage_path: unstage_path_action.clone(),
        select_file: select_file_action.clone(),
    };
    let status_pane = crate::sections::status_pane(
        branch,
        staged,
        unstaged,
        status_entries,
        branches,
        tags,
        remotes,
        stashes,
        conflicts,
        conflict_path,
        error,
        branch_name,
        tag_name,
        remote_name,
        remote_url,
        StatusActions {
            stage_all: stage_all_action,
            unstage_all: unstage_all_action,
            stage_path: stage_path_action,
            unstage_path: unstage_path_action,
            checkout: checkout_action,
            create_branch: create_branch_action,
            delete_branch: delete_branch_action,
            add_remote: add_remote_action,
            remove_remote: remove_remote_action,
            rename_branch: rename_branch_action,
            merge_branch: merge_branch_action,
            rebase_branch: rebase_branch_action,
            select_file: select_file_action,
            create_tag: create_tag_action,
            delete_tag: delete_tag_action,
            select_conflict: select_conflict_action,
        },
        StashActions {
            push: stash_push_action,
            apply: stash_apply_action,
            pop: stash_pop_action,
            drop: stash_drop_action,
        },
    );

    let select_commit = Arc::new(move |sha: String| {
        selected.set(SharedString::from(sha.clone()));
        match Repo::open(repo_path.get_untracked().to_string().as_str()) {
            Ok(repo) => match repo.diff(&sha) {
                Ok(patch) => {
                    diff.set(SharedString::from(patch.clone()));
                    error.set(SharedString::from(""));
                    let index = active.get_untracked();
                    tabs.update(|list| {
                        if let Some(tab) = list.get_mut(index) {
                            tab.selected = SharedString::from(sha.clone());
                            tab.diff = SharedString::from(patch);
                        }
                    });
                }
                Err(err) => error.set(SharedString::from(err.to_string())),
            },
            Err(err) => error.set(SharedString::from(err.to_string())),
        }
    });
    let open_commit_info: Arc<dyn Fn()> = Arc::new(move || {
        let sha = selected.get_untracked().to_string();
        match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.commit_details(&sha))
        {
            Ok(details) => {
                commit_files.set(details.files.clone());
                commit_info.set(Some(details));
                commit_info_open.set(true);
            }
            Err(err) => error.set(SharedString::from(err.to_string())),
        }
    });
    let history = history(branch, commits, selected, select_commit, open_compare.clone(), mode);

    let cherry_pick_action = Arc::new({
        let reload_pick = reload_for_branch.clone();
        move |sha: String| match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.cherry_pick(&sha))
        {
            Ok(_) => {
                error.set(SharedString::from(""));
                reload_pick();
            }
            Err(err) => {
                error.set(SharedString::from(err.to_string()));
                reload_pick();
            }
        }
    });
    let revert_action = Arc::new({
        let reload_revert = reload_for_branch.clone();
        move |sha: String| match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.revert(&sha))
        {
            Ok(_) => {
                error.set(SharedString::from(""));
                reload_revert();
            }
            Err(err) => {
                error.set(SharedString::from(err.to_string()));
                reload_revert();
            }
        }
    });
    let diff_pane = diff_view(
        mode,
        selected,
        diff,
        selected_file,
        file_patch,
        staged_patch,
        hunk_indices,
        staged_hunk_indices,
        DiffActions {
            cherry_pick: cherry_pick_action,
            revert: revert_action,
        },
        hunk_actions,
    );

    let changes = changes_list(
        mode,
        staged_entries,
        unstaged_entries,
        selected_file,
        changes_actions,
    );
    let inspector = inspector(
        mode,
        inspector_tab,
        agent_permissions,
        branch,
        staged,
        unstaged,
        staged_entries,
        message,
        commit_log,
        commit_actions,
        open_agent.clone(),
    );
    let close_inspect: Arc<dyn Fn()> = Arc::new(move || inspect_open.set(false));
    let run_compare: Arc<dyn Fn()> = Arc::new(move || {
        match Repo::open(repo_path.get_untracked().to_string().as_str()).and_then(|repo| {
            repo.compare(
                inspect_from.get_untracked().trim(),
                inspect_to.get_untracked().trim(),
            )
        }) {
            Ok(patch) => {
                inspect_diff.set(SharedString::from(patch));
                inspect_error.set(SharedString::from(""));
            }
            Err(err) => inspect_error.set(SharedString::from(err.to_string())),
        }
    });
    let run_file_history: Arc<dyn Fn()> = Arc::new(move || {
        let path = inspect_path.get_untracked().trim().to_string();
        match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.file_history(&path, 80))
        {
            Ok(commits) => {
                let first = commits
                    .first()
                    .map(|commit| commit.sha.clone())
                    .unwrap_or_default();
                inspect_commits.set(commits);
                inspect_selected.set(SharedString::from(first.clone()));
                if first.is_empty() {
                    inspect_diff.set(SharedString::from(""));
                } else if let Ok(repo) =
                    Repo::open(repo_path.get_untracked().to_string().as_str())
                {
                    if let Ok(patch) = repo.diff(&first) {
                        inspect_diff.set(SharedString::from(patch));
                    }
                }
                inspect_error.set(SharedString::from(""));
            }
            Err(err) => inspect_error.set(SharedString::from(err.to_string())),
        }
    });
    let run_blame: Arc<dyn Fn()> = Arc::new(move || {
        let path = inspect_path.get_untracked().trim().to_string();
        match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.blame(&path))
        {
            Ok(lines) => {
                inspect_blame.set(lines);
                inspect_error.set(SharedString::from(""));
            }
            Err(err) => inspect_error.set(SharedString::from(err.to_string())),
        }
    });
    let load_inspect: Arc<dyn Fn()> = Arc::new({
        let history = run_file_history.clone();
        let blame = run_blame.clone();
        move || {
            if inspect_tab.get_untracked() == InspectTab::Blame {
                blame();
            } else {
                history();
            }
        }
    });
    let select_inspect_commit: Arc<dyn Fn(String)> = Arc::new(move |sha: String| {
        inspect_selected.set(SharedString::from(sha.clone()));
        match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.diff(&sha))
        {
            Ok(patch) => {
                inspect_diff.set(SharedString::from(patch));
                inspect_error.set(SharedString::from(""));
            }
            Err(err) => inspect_error.set(SharedString::from(err.to_string())),
        }
    });
    let inspect_panel = inspect_panel(
        inspect_open,
        inspect_tab,
        inspect_from,
        inspect_to,
        inspect_path,
        inspect_diff,
        inspect_commits,
        inspect_blame,
        inspect_selected,
        inspect_error,
        InspectActions {
            close: close_inspect,
            compare: run_compare,
            load: load_inspect,
            select_commit: select_inspect_commit,
        },
    );

    let close_agent: Arc<dyn Fn()> = Arc::new(move || agent_open.set(false));
    let add_stage: Arc<dyn Fn()> = Arc::new(move || {
        let mut steps = agent_steps.get_untracked();
        if !steps.iter().any(|step| matches!(step, PlanStep::StageAll)) {
            steps.push(PlanStep::StageAll);
            agent_steps.set(steps);
        }
    });
    let add_commit: Arc<dyn Fn()> = Arc::new(move || {
        let text = message.get_untracked().trim().to_string();
        if text.is_empty() {
            agent_error.set(SharedString::from("Commit message is empty"));
            return;
        }
        if staged.get_untracked() == 0 {
            agent_error.set(SharedString::from("No staged files"));
            return;
        }
        let mut steps = agent_steps.get_untracked();
        steps.push(PlanStep::Commit { message: text });
        agent_steps.set(steps);
        agent_error.set(SharedString::from(""));
    });
    let add_push: Arc<dyn Fn()> = Arc::new(move || {
        let mut steps = agent_steps.get_untracked();
        if !steps.iter().any(|step| matches!(step, PlanStep::Push)) {
            steps.push(PlanStep::Push);
            agent_steps.set(steps);
        }
    });
    let clear_agent: Arc<dyn Fn()> = Arc::new(move || {
        agent_steps.set(Vec::new());
        agent_error.set(SharedString::from(""));
        agent_log.set(SharedString::from(""));
    });
    let build_agent_plan: Arc<dyn Fn()> = Arc::new(move || {
        let prompt = agent_prompt.get_untracked();
        match LocalPlanner.plan(&prompt, message.get_untracked().trim()) {
            Ok(plan) => {
                agent_steps.set(plan.steps);
                agent_error.set(SharedString::from(""));
            }
            Err(err) => agent_error.set(SharedString::from(err.to_string())),
        }
    });
    let approve_agent: Arc<dyn Fn()> = Arc::new({
        let reload = reload_for_branch.clone();
        move || {
            let plan = AgentPlan {
                steps: agent_steps.get_untracked(),
            };
            let permissions = agent_permissions.get_untracked();
            if let Err(err) = plan.validate(&permissions) {
                audit::append(&format!("agent plan denied: {err}"), false);
                agent_error.set(SharedString::from(err.to_string()));
                return;
            }
            audit::append("agent plan approved", true);
            let repo = match Repo::open(repo_path.get_untracked().to_string().as_str()) {
                Ok(repo) => repo,
                Err(err) => {
                    agent_error.set(SharedString::from(err.to_string()));
                    return;
                }
            };
            let has_push = plan.steps.iter().any(|step| matches!(step, PlanStep::Push));
            let protected = matches!(branch.get_untracked().as_str(), "main" | "master");
            if has_push
                && protected
                && permissions.protected_requires_approval
                && !agent_protected_ok.get_untracked()
            {
                audit::append("agent push denied: protected branch not confirmed", false);
                agent_error.set(SharedString::from(
                    "Protected branch push needs extra confirmation",
                ));
                return;
            }
            for step in plan.steps {
                let step_name = match &step {
                    PlanStep::StageAll => "stage-all",
                    PlanStep::Commit { .. } => "commit",
                    PlanStep::Push => "push",
                };
                let result = match step {
                    PlanStep::StageAll => repo.stage_all().map(|_| "staged all".to_string()),
                    PlanStep::Commit { message } => repo.commit(&message),
                    PlanStep::Push => repo.push(),
                };
                match result {
                    Ok(output) => {
                        audit::append(
                            &format!("agent step {}: ok", step_name),
                            true,
                        );
                        let line = format!("• {output}");
                        agent_log.set(SharedString::from(line));
                    }
                    Err(err) => {
                        audit::append(
                            &format!("agent step {}: failed", step_name),
                            false,
                        );
                        agent_error.set(SharedString::from(err.to_string()));
                        return;
                    }
                }
            }
            agent_steps.set(Vec::new());
            agent_protected_ok.set(false);
            agent_open.set(false);
            agent_error.set(SharedString::from(""));
            reload();
        }
    });
    let agent_panel = agent_panel(
        agent_open,
        agent_permissions,
        branch,
        staged,
        agent_steps,
        agent_prompt,
        agent_protected_ok,
        agent_log,
        agent_error,
        AgentActions {
            close: close_agent,
            add_stage,
            add_commit,
            add_push,
            clear: clear_agent,
            build: build_agent_plan,
            confirm_protected: Arc::new(move || {
                agent_protected_ok.set(!agent_protected_ok.get_untracked())
            }),
            approve: approve_agent,
        },
    );

    let audit_panel = audit_panel(
        audit_open,
        audit_lines,
        audit_error,
        AuditActions {
            close: Arc::new(move || audit_open.set(false)),
            refresh: {
                let open_audit = open_audit.clone();
                Arc::new(move || open_audit())
            },
        },
    );
    let palette_close: Arc<dyn Fn()> = Arc::new(move || palette_open.set(false));
    let palette_commands = vec![
        PaletteCommand {
            label: "History mode",
            hint: "show commits",
            action: {
                let set = set_mode.clone();
                Arc::new(move || set(Mode::History))
            },
        },
        PaletteCommand {
            label: "Changes mode",
            hint: "show working tree",
            action: {
                let set = set_mode.clone();
                Arc::new(move || set(Mode::Changes))
            },
        },
        PaletteCommand {
            label: "Stage all",
            hint: "git add -A",
            action: palette_stage_all.clone(),
        },
        PaletteCommand {
            label: "Commit staged",
            hint: "commit with current summary",
            action: palette_commit.clone(),
        },
        PaletteCommand {
            label: "Fetch",
            hint: "origin",
            action: palette_fetch.clone(),
        },
        PaletteCommand {
            label: "Pull",
            hint: "fast-forward only",
            action: palette_pull.clone(),
        },
        PaletteCommand {
            label: "Push",
            hint: "review commits to push",
            action: open_push.clone(),
        },
        PaletteCommand {
            label: "Sync",
            hint: "fetch, pull, push",
            action: open_sync.clone(),
        },
        PaletteCommand {
            label: "Refs",
            hint: "branches, tags, remotes",
            action: open_refs.clone(),
        },
        PaletteCommand {
            label: "Reflog",
            hint: "recent HEAD movements",
            action: open_reflog.clone(),
        },
        PaletteCommand {
            label: "Compare",
            hint: "diff two refs",
            action: open_compare.clone(),
        },
        PaletteCommand {
            label: "File history",
            hint: "history and blame",
            action: open_file_history.clone(),
        },
        PaletteCommand {
            label: "Commit info",
            hint: "metadata and files",
            action: open_commit_info.clone(),
        },
        PaletteCommand {
            label: "Agent plan",
            hint: "local stage/commit/push",
            action: open_agent.clone(),
        },
        PaletteCommand {
            label: "Settings",
            hint: "preferences and permissions",
            action: open_settings.clone(),
        },
        PaletteCommand {
            label: "Audit log",
            hint: "agent activity",
            action: open_audit.clone(),
        },
    ];
    let palette_panel = palette_panel(palette_open, palette_query, palette_commands, palette_close);

    let commit_info_panel = commit_info_panel(
        commit_info_open,
        commit_info,
        commit_files,
        CommitInfoActions {
            close: Arc::new(move || commit_info_open.set(false)),
        },
    );
    let push_panel = push_panel(
        push_open,
        push_preview,
        push_commits,
        error,
        PushActions {
            close: Arc::new(move || push_open.set(false)),
            confirm: confirm_push,
        },
    );
    let reflog_panel = reflog_panel(
        reflog_open,
        reflog_entries,
        reflog_selected,
        reflog_error,
        ReflogActions {
            close: close_reflog,
            reset: reset_reflog,
        },
    );
    let sync_panel = sync_panel(
        sync_open,
        sync_kind,
        branch,
        ahead,
        behind,
        sync_log,
        error,
        SyncActions {
            select: sync_select,
            fetch: sync_fetch_action,
            pull: sync_pull_action,
            push: open_push.clone(),
            close: sync_close,
        },
    );
    let content = view()
        .flex_1()
        .min_w_0()
        .flex_col()
        .child(history)
        .child(changes)
        .child(diff_pane);
    let main = view()
        .flex_1()
        .flex_row()
        .gap_0()
        .p_0()
        .bg(move || theme().background)
        .child(status_pane)
        .child(content)
        .child(inspector);

    let statusbar = statusbar(commits);

    view()
        .flex_1()
        .relative()
        .bg(move || theme().background)
        .text_color(move || theme().foreground)
        .child(titlebar)
        .child(toolbar)
        .child(operation_banner)
        .child(conflict_panel)
        .child(main)
        .child(sync_panel)
        .child(reflog_panel)
        .child(inspect_panel)
        .child(refs_panel)
        .child(commit_info_panel)
        .child(push_panel)
        .child(agent_panel)
        .child(audit_panel)
        .child(palette_panel)
        .child(settings_panel)
        .child(statusbar)
}
