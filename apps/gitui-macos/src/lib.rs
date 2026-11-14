//! gitui M1 — NSH + Rust, read-only Git browser.
//!
//! Slice 1: open the current repository, show status, recent history and the
//! selected commit's patch. No staging, commit, push or settings.

mod model;
mod sections;
mod widgets;

use gitui_git::{Branch, Repo, StatusEntry};
use model::{load, load_recent, load_session, save_recent, save_session, short_name};
use sections::{diff_view, history, statusbar};
use widgets::{conflict_pane, header_button, mini_button};
use nsh_base::{editor, input, theme};
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

    let header = view()
        .h(px(46.0))
        .flex_row()
        .items_center()
        .gap_3()
        .px_3()
        .bg(move || theme().background)
        .child(
            text("gitui · local")
                .text_size(px(13.0))
                .text_color(move || theme().foreground),
        )
        .child(
            input()
                .value(path_input)
                .on_change(move |text: &str| path_input.set(text.to_string()))
                .placeholder("Repository path")
                .text_size(px(11.0))
                .w(px(420.0)),
        )
        .child(
            text(move || format!("{} ↑{} ↓{}", branch.get(), ahead.get(), behind.get()))
                .text_size(px(10.0))
                .text_color(move || theme().primary),
        )
        .child(view().flex_1())
        .child(header_button("Open", true, reload))
        .child(header_button("Close tab", false, close_active))
        .child(header_button("Fetch", false, {
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
        }))
        .child(header_button("Pull", false, {
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
        }))
        .child(header_button("Push", false, {
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
        }));

    let operation_banner = view()
        .h(move || {
            if operation.get().is_some() {
                px(28.0)
            } else {
                px(0.0)
            }
        })
        .flex_row()
        .items_center()
        .gap_2()
        .px_3()
        .bg(move || theme().primary)
        .child(
            text(move || {
                operation
                    .get()
                    .map(|op| format!("{} in progress", op.label()))
                    .unwrap_or_default()
            })
            .text_size(px(10.0))
            .text_color(move || theme().primary_foreground),
        )
        .child(view().flex_1())
        .child(header_button("Continue", true, {
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
        }))
        .child(header_button("Abort", false, {
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
        }));

    let tab_bar = crate::sections::tabs(tabs, active, activate_tabs);

    let recent_bar = view()
        .h(px(28.0))
        .flex_row()
        .items_center()
        .gap_2()
        .px_3()
        .bg(move || theme().background)
        .child(
            text("Recent")
                .text_size(px(9.0))
                .text_color(move || theme().foreground),
        )
        .child(each(
            recent,
            |path: &SharedString| path.clone(),
            move |path, _| {
                let reload_recent = reload_for_recent.clone();
                view()
                    .h(px(20.0))
                    .px_2()
                    .rounded(px(4.0))
                    .bg(move || theme().primary)
                    .on_press(move || {
                        path_input.set(path.get().to_string());
                        reload_recent();
                    })
                    .child(
                        text(move || short_name(&path.get().to_string()))
                            .text_size(px(9.0))
                            .text_color(move || theme().primary_foreground),
                    )
            },
        ));

    let commit_bar = view()
        .h(px(38.0))
        .flex_row()
        .items_center()
        .gap_2()
        .px_3()
        .bg(move || theme().background)
        .child(
            input()
                .value(message)
                .on_change(move |text: &str| message.set(text.to_string()))
                .placeholder("Commit message")
                .flex_1(),
        )
        .child(header_button("Commit", true, {
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
        }))
        .child(header_button("No hooks", false, {
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
        }))
        .child(header_button("Amend", false, {
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
        }));



    let conflict_panel = view()
        .h(move || {
            if conflict_stages.get().is_some() {
                px(250.0)
            } else {
                px(0.0)
            }
        })
        .flex_col()
        .gap_2()
        .p_2()
        .bg(move || theme().background)
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    text(move || {
                        let total = conflicts.get().len();
                        let current = conflicts
                            .get()
                            .iter()
                            .position(|path| path == &conflict_path.get())
                            .map(|index| index + 1)
                            .unwrap_or(0);
                        format!("Conflict · {} · {} of {}", conflict_path.get(), current, total)
                    })
                    .text_size(px(10.0))
                    .text_color(|| rgb(0xff3b30)),
                )
                .child(view().flex_1())
                .child(header_button("Accept Ours", false, move || {
                    if let Some(stages) = conflict_stages.get_untracked().as_ref() {
                        conflict_result.set(stages.ours.clone());
                    }
                }))
                .child(header_button("Accept Theirs", false, move || {
                    if let Some(stages) = conflict_stages.get_untracked().as_ref() {
                        conflict_result.set(stages.theirs.clone());
                    }
                }))
                .child(header_button("Accept Both", false, move || {
                    if let Some(stages) = conflict_stages.get_untracked().as_ref() {
                        conflict_result.set(format!("{}\n{}", stages.ours, stages.theirs));
                    }
                }))
                .child(header_button("Mark Resolved", true, {
                    let reload_conflict = reload_for_conflict.clone();
                    move || {
                        match Repo::open(repo_path.get_untracked().to_string().as_str())
                            .and_then(|repo| {
                                repo.resolve(
                                    conflict_path.get_untracked().to_string().as_str(),
                                    conflict_result.get_untracked().to_string().as_str(),
                                )
                            })
                        {
                            Ok(()) => {
                                error.set(SharedString::from(""));
                                reload_conflict();
                            }
                            Err(err) => error.set(SharedString::from(err.to_string())),
                        }
                    }
                })),
        )
        .child(
            view()
                .flex_1()
                .flex_row()
                .gap_2()
                .child(conflict_pane("Base", move || {
                    conflict_stages
                        .get()
                        .as_ref()
                        .map(|stages| stages.base.clone())
                        .unwrap_or_default()
                }))
                .child(conflict_pane("Ours", move || {
                    conflict_stages
                        .get()
                        .as_ref()
                        .map(|stages| stages.ours.clone())
                        .unwrap_or_default()
                }))
                .child(conflict_pane("Theirs", move || {
                    conflict_stages
                        .get()
                        .as_ref()
                        .map(|stages| stages.theirs.clone())
                        .unwrap_or_default()
                }))
                .child(
                    view()
                        .flex_1()
                        .flex_col()
                        .gap_1()
                        .child(
                            text("Result")
                                .text_size(px(9.0))
                                .text_color(move || theme().primary),
                        )
                        .child(
                            editor()
                                .value(move || Some(SharedString::from(conflict_result.get())))
                                .on_change(move |value: &SharedString| {
                                    conflict_result.set(value.to_string())
                                })
                                .font_size(10.0)
                                .line_height(15.0)
                                .flex_1(),
                        ),
                ),
        );

    let status_pane = view()
        .w(px(250.0))
        .flex_col()
        .gap_2()
        .p_2()
        .child(
            text("Working tree")
                .text_size(px(11.0))
                .text_color(move || theme().foreground),
        )
        .child(
            text(move || format!("{} staged · {} unstaged", staged.get(), unstaged.get()))
                .text_size(px(10.0))
                .text_color(move || theme().foreground),
        )
        .child(
            text(move || branch.get())
                .text_size(px(10.0))
                .text_color(move || theme().primary),
        )
        .child(
            view()
                .flex_row()
                .gap_2()
                .child(header_button("Stage All", true, {
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
                }))
                .child(header_button("Unstage All", false, {
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
                })),
        )
        .child(
            text("Branches")
                .text_size(px(9.0))
                .text_color(move || theme().foreground),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_1()
                .child(
                    input()
                        .value(branch_name)
                        .on_change(move |text: &str| branch_name.set(text.to_string()))
                        .placeholder("new-branch")
                        .text_size(px(9.0))
                        .flex_1(),
                )
                .child(mini_button("New", true, {
                    let reload_branch = reload_for_branch.clone();
                    move || {
                        let name = branch_name.get_untracked().trim().to_string();
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
                })),
        )
        .child(each(
            branches,
            |branch: &Branch| branch.name.clone(),
            move |branch, _| {
                let name = branch.get().name.clone();
                let current = branch.get().current;
                let reload_branch = reload_for_branch.clone();
                view()
                    .h(px(18.0))
                    .px_1()
                    .rounded(px(3.0))
                    .bg(move || {
                        if branch.get().current {
                            theme().primary
                        } else {
                            theme().background
                        }
                    })
                    .on_press(move || {
                        if current {
                            return;
                        }
                        match Repo::open(repo_path.get_untracked().to_string().as_str())
                            .and_then(|repo| repo.checkout(&name))
                        {
                            Ok(_) => {
                                error.set(SharedString::from(""));
                                reload_branch();
                            }
                            Err(err) => error.set(SharedString::from(err.to_string())),
                        }
                    })
                    .child(
                        text(move || branch.get().name)
                            .text_size(px(9.0))
                            .text_color(move || {
                                if branch.get().current {
                                    theme().primary_foreground
                                } else {
                                    theme().foreground
                                }
                            }),
                    )
            },
        ))
        .child(
            text(move || {
                if conflicts.get().is_empty() {
                    String::new()
                } else {
                    "Conflicts".to_string()
                }
            })
            .text_size(px(9.0))
            .text_color(move || theme().foreground),
        )
        .child(each(
            conflicts,
            |path: &String| path.clone(),
            move |path, _| {
                view()
                    .h(px(18.0))
                    .px_1()
                    .rounded(px(3.0))
                    .bg(move || {
                        if conflict_path.get() == path.get() {
                            theme().primary
                        } else {
                            theme().background
                        }
                    })
                    .on_press(move || {
                        let selected = path.get();
                        match Repo::open(repo_path.get_untracked().to_string().as_str())
                            .and_then(|repo| repo.conflict_stages(&selected))
                        {
                            Ok(stages) => {
                                conflict_path.set(selected.clone());
                                conflict_result.set(stages.ours.clone());
                                conflict_stages.set(Some(stages));
                                error.set(SharedString::from(""));
                            }
                            Err(err) => error.set(SharedString::from(err.to_string())),
                        }
                    })
                    .child(
                        text(move || path.get())
                            .text_size(px(9.0))
                            .text_color(move || {
                                if conflict_path.get() == path.get() {
                                    rgb(0xffffff)
                                } else {
                                    rgb(0xff3b30)
                                }
                            }),
                    )
            },
        ))
        .child(
            text(move || error.get())
                .text_size(px(9.0))
                .text_color(|| rgb(0xff3b30)),
        )
        .child(
            text(move || {
                if status_entries.get().is_empty() {
                    "Clean working tree".to_string()
                } else {
                    String::new()
                }
            })
            .text_size(px(9.0))
            .text_color(move || theme().foreground),
        )
        .child(each(
            status_entries,
            |entry: &StatusEntry| entry.path.clone(),
            move |entry, _| {
                let path = entry.get().path.clone();
                let staged = entry.get().is_staged();
                let unstaged = entry.get().is_unstaged();
                let reload_entry = reload_for_entry.clone();
                let mut row = view()
                    .h(px(36.0))
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .child(
                        text(move || short_name(&entry.get().path))
                            .text_size(px(9.0))
                            .text_color(move || theme().foreground)
                            .flex_1(),
                    );
                if unstaged {
                    let path = path.clone();
                    let reload = reload_entry.clone();
                    row = row.child(mini_button("Stage", true, move || {
                        match Repo::open(repo_path.get_untracked().to_string().as_str())
                            .and_then(|repo| repo.stage_path(&path))
                        {
                            Ok(()) => {
                                error.set(SharedString::from(""));
                                reload();
                            }
                            Err(err) => error.set(SharedString::from(err.to_string())),
                        }
                    }));
                }
                if staged {
                    let path = path.clone();
                    let reload = reload_entry.clone();
                    row = row.child(mini_button("Unstage", false, move || {
                        match Repo::open(repo_path.get_untracked().to_string().as_str())
                            .and_then(|repo| repo.unstage_path(&path))
                        {
                            Ok(()) => {
                                error.set(SharedString::from(""));
                                reload();
                            }
                            Err(err) => error.set(SharedString::from(err.to_string())),
                        }
                    }));
                }
                row
            },
        ));

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
