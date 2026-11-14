//! gitui M1 — NSH + Rust, read-only Git browser.
//!
//! Slice 1: open the current repository, show status, recent history and the
//! selected commit's patch. No staging, commit, push or settings.

mod model;
mod sections;
mod widgets;

use gitui_git::Repo;
use model::{load, load_recent, load_session, save_recent, save_session};
use sections::{diff_view, header, history, statusbar, CommitActions, ConflictActions, HeaderActions, StatusActions};
use nsh_base::theme;
use nsh_core::prelude::*;
use std::sync::Arc;

#[native_module("gitui/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}

#[component]
pub fn App() -> impl IntoElement {
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
    let branch = signal(SharedString::from(active_tab.branch.clone()));
    let operation = signal(active_tab.operation);
    let conflicts = signal(active_tab.conflicts.clone());
    let branches = signal(active_tab.branches.clone());
    let tags = signal(active_tab.tags.clone());
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
        status_entries.set(loaded.status_entries);
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
        status_entries.set(tab.status_entries);
        commits.set(tab.commits);
        selected.set(tab.selected);
        diff.set(tab.diff);
        error.set(tab.error);
        save_session(active.get_untracked(), tabs.get_untracked().as_slice());
    };
    let activate_for_close = activate.clone();
    let activate_tabs: Arc<dyn Fn(usize)> = Arc::new(move |index| activate(index));
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
    let header = header(
        path_input,
        branch,
        ahead,
        behind,
        HeaderActions {
            open: open_action,
            close: close_action,
            fetch: fetch_action,
            pull: pull_action,
            push: push_action,
        },
    );

    let continue_action = Arc::new({
        let reload_merge = reload_for_merge.clone();
        move || match Repo::open(repo_path.get_untracked().to_string().as_str())
            .and_then(|repo| repo.merge_continue())
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
            .and_then(|repo| repo.merge_abort())
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
    let operation_banner =
        crate::sections::operation_banner(operation, continue_action, abort_action);

    let tab_bar = crate::sections::tabs(tabs, active, activate_tabs);

    let open_recent = Arc::new(move |path: String| {
        path_input.set(path);
        reload_for_recent();
    });
    let recent_bar = crate::sections::recent_bar(recent, open_recent);

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
    let commit_bar = crate::sections::commit_bar(
        message,
        CommitActions {
            commit: commit_action,
            no_hooks: no_hooks_action,
            amend: amend_action,
        },
    );

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
        },
    );

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
    let status_pane = crate::sections::status_pane(
        branch,
        staged,
        unstaged,
        status_entries,
        branches,
        tags,
        conflicts,
        conflict_path,
        error,
        branch_name,
        tag_name,
        StatusActions {
            stage_all: stage_all_action,
            unstage_all: unstage_all_action,
            stage_path: stage_path_action,
            unstage_path: unstage_path_action,
            checkout: checkout_action,
            create_branch: create_branch_action,
            delete_branch: delete_branch_action,
            create_tag: create_tag_action,
            delete_tag: delete_tag_action,
            select_conflict: select_conflict_action,
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
    let history = history(commits, selected, select_commit);

    let diff_pane = diff_view(selected, diff);

    let main = view()
        .flex_1()
        .flex_row()
        .gap_2()
        .p_2()
        .bg(move || theme().background)
        .child(status_pane)
        .child(history)
        .child(diff_pane);

    let statusbar = statusbar(commits);

    let commit_log_panel = view()
        .h(move || {
            if commit_log.get().is_empty() {
                px(0.0)
            } else {
                px(120.0)
            }
        })
        .flex_col()
        .p_2()
        .bg(move || theme().background)
        .child(
            text(move || commit_log.get())
                .text_size(px(9.0))
                .text_color(move || theme().foreground),
        );

    view()
        .flex_1()
        .bg(move || theme().background)
        .text_color(move || theme().foreground)
        .child(header)
        .child(operation_banner)
        .child(tab_bar)
        .child(recent_bar)
        .child(commit_bar)
        .child(conflict_panel)
        .child(main)
        .child(commit_log_panel)
        .child(statusbar)
}
