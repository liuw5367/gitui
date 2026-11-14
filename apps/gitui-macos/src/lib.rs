//! gitui M1 — NSH + Rust, read-only Git browser.
//!
//! Slice 1: open the current repository, show status, recent history and the
//! selected commit's patch. No staging, commit, push or settings.

use gitui_git::{Commit, Repo, StatusEntry};
use nsh_base::{editor, input, theme};
use nsh_core::prelude::*;
use std::path::PathBuf;

#[derive(Clone, PartialEq)]
struct Loaded {
    path: String,
    branch: String,
    staged: usize,
    unstaged: usize,
    status_lines: Vec<SharedString>,
    status_entries: Vec<StatusEntry>,
    commits: Vec<Commit>,
    selected: SharedString,
    diff: SharedString,
    error: SharedString,
}

fn load(path: &str) -> Loaded {
    match Repo::open(path) {
        Ok(repo) => {
            let status = repo.status().unwrap_or_default();
            let commits = repo.log(15).unwrap_or_default();
            let diff = commits
                .first()
                .and_then(|commit| repo.diff(&commit.sha).ok())
                .unwrap_or_default();
            let branch = if status.branch.is_empty() {
                "HEAD".to_string()
            } else {
                status.branch
            };
            let selected = commits
                .first()
                .map(|commit| commit.sha.clone())
                .unwrap_or_default();
            Loaded {
                path: repo.root().display().to_string(),
                branch,
                staged: status.staged,
                unstaged: status.unstaged,
                status_lines: status.lines.into_iter().map(SharedString::from).collect(),
                status_entries: status.entries,
                commits,
                selected: SharedString::from(selected),
                diff: SharedString::from(diff),
                error: SharedString::from(""),
            }
        }
        Err(error) => Loaded {
            path: path.to_string(),
            branch: "—".to_string(),
            selected: SharedString::from(""),
            staged: 0,
            unstaged: 0,
            status_lines: Vec::new(),
            status_entries: Vec::new(),
            commits: Vec::new(),
            diff: SharedString::from(format!("Could not open repository:\n{error}")),
            error: SharedString::from(error.to_string()),
        },
    }
}

fn short_name(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

fn recent_file() -> Option<PathBuf> {
    let home = std::env::var_os("GITUI_HOME").or_else(|| std::env::var_os("HOME"))?;
    Some(PathBuf::from(home).join(".gitui").join("recent"))
}

fn load_recent() -> Vec<String> {
    let Some(file) = recent_file() else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(file) else {
        return Vec::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(5)
        .map(str::to_string)
        .collect()
}

fn save_recent(paths: &[SharedString]) {
    let Some(file) = recent_file() else {
        return;
    };
    if let Some(parent) = file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let text = paths
        .iter()
        .map(|path| path.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::write(file, text);
}

fn session_file() -> Option<PathBuf> {
    let home = std::env::var_os("GITUI_HOME").or_else(|| std::env::var_os("HOME"))?;
    Some(PathBuf::from(home).join(".gitui").join("session"))
}

fn load_session() -> (usize, Vec<String>) {
    let Some(file) = session_file() else {
        return (0, Vec::new());
    };
    let Ok(text) = std::fs::read_to_string(file) else {
        return (0, Vec::new());
    };
    let mut lines = text.lines();
    let active = lines
        .next()
        .and_then(|line| line.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let paths = lines
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(10)
        .map(str::to_string)
        .collect();
    (active, paths)
}

fn save_session(active: usize, tabs: &[Loaded]) {
    let Some(file) = session_file() else {
        return;
    };
    if let Some(parent) = file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut lines = vec![active.to_string()];
    lines.extend(tabs.iter().map(|tab| tab.path.clone()));
    let _ = std::fs::write(file, lines.join("\n"));
}

fn mini_button(
    label: &'static str,
    primary: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    view()
        .px_2()
        .py_1()
        .rounded(px(3.0))
        .bg(move || if primary { theme().primary } else { theme().background })
        .on_press(on_press)
        .child(
            text(label)
                .text_size(px(8.5))
                .text_color(move || {
                    if primary {
                        theme().primary_foreground
                    } else {
                        theme().primary
                    }
                }),
        )
}

fn header_button(
    label: &'static str,
    primary: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    view()
        .px_3()
        .py_2()
        .rounded(px(5.0))
        .bg(move || if primary { theme().primary } else { theme().background })
        .on_press(on_press)
        .child(
            text(label)
                .text_size(px(11.0))
                .text_color(move || {
                    if primary {
                        theme().primary_foreground
                    } else {
                        theme().primary
                    }
                }),
        )
}

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
    let activate = move |index: usize| {
        let Some(tab) = tabs.get_untracked().get(index).cloned() else {
            return;
        };
        active.set(index);
        repo_path.set(SharedString::from(tab.path.clone()));
        path_input.set(tab.path.clone());
        branch.set(SharedString::from(tab.branch));
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
    let activate_for_tabs = activate.clone();
    let activate_for_close = activate.clone();
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
            text("gitui · M1 read-only")
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
            text(move || branch.get())
                .text_size(px(10.0))
                .text_color(move || theme().primary),
        )
        .child(view().flex_1())
        .child(header_button("Open", true, reload))
        .child(header_button("Close tab", false, close_active));

    let tab_bar = view()
        .h(px(30.0))
        .flex_row()
        .items_center()
        .gap_1()
        .px_3()
        .bg(move || theme().background)
        .child(each(
            tabs,
            |tab: &Loaded| tab.path.clone(),
            move |tab, _| {
                let index = tab.get().path.clone();
                let activate = activate_for_tabs.clone();
                view()
                    .h(px(24.0))
                    .px_2()
                    .rounded(px(4.0))
                    .bg(move || {
                        let path = tab.get().path;
                        if active.get()
                            == tabs
                                .get_untracked()
                                .iter()
                                .position(|item| item.path == path)
                                .unwrap_or(usize::MAX)
                        {
                            theme().primary
                        } else {
                            theme().background
                        }
                    })
                    .on_press(move || {
                        let index = tabs
                            .get_untracked()
                            .iter()
                            .position(|item| item.path == index)
                            .unwrap_or(0);
                        activate(index);
                    })
                    .child(
                        text(move || short_name(&tab.get().path))
                            .text_size(px(10.0))
                            .text_color(move || {
                                let index = tabs
                                    .get_untracked()
                                    .iter()
                                    .position(|item| item.path == tab.get().path)
                                    .unwrap_or(usize::MAX);
                                if active.get() == index {
                                    theme().primary_foreground
                                } else {
                                    theme().foreground
                                }
                            }),
                    )
            },
        ));

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
        }));


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

    let history = view()
        .w(px(360.0))
        .flex_col()
        .gap_2()
        .p_2()
        .child(
            text("History")
                .text_size(px(11.0))
                .text_color(move || theme().foreground),
        )
        .child(each(
            commits,
            |commit: &Commit| commit.sha.clone(),
            move |commit, _| {
                let sha = commit.get().sha.clone();
                view()
                    .h(px(40.0))
                    .px_2()
                    .rounded(px(4.0))
                    .bg(move || {
                        if selected.get().to_string() == commit.get().sha {
                            theme().primary
                        } else {
                            theme().background
                        }
                    })
                    .on_press(move || {
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
                    })
                    .child(
                        text(move || commit.get().summary.clone())
                            .text_size(px(11.0))
                            .text_color(move || {
                                if selected.get().to_string() == commit.get().sha {
                                    theme().primary_foreground
                                } else {
                                    theme().foreground
                                }
                            }),
                    )
                    .child(
                        text(move || {
                            format!(
                                "{} · {} · {}",
                                commit.get().short,
                                commit.get().author,
                                commit.get().date
                            )
                        })
                        .text_size(px(9.0))
                        .text_color(move || {
                            if selected.get().to_string() == commit.get().sha {
                                theme().primary_foreground
                            } else {
                                theme().foreground
                            }
                        }),
                    )
            },
        ));

    let diff_pane = view()
        .flex_1()
        .flex_col()
        .gap_2()
        .p_2()
        .child(
            text(move || format!("Diff · {}", selected.get()))
                .text_size(px(10.0))
                .text_color(move || theme().foreground),
        )
        .child(
            editor()
                .value(move || Some(diff.get()))
                .read_only(true)
                .language("diff")
                .font_size(11.0)
                .line_height(17.0)
                .flex_1(),
        );

    let main = view()
        .flex_1()
        .flex_row()
        .gap_2()
        .p_2()
        .bg(move || theme().background)
        .child(status_pane)
        .child(history)
        .child(diff_pane);

    let statusbar = view()
        .h(px(24.0))
        .flex_row()
        .items_center()
        .gap_3()
        .px_3()
        .bg(move || theme().background)
        .child(
            text("NSH + Rust · read-only")
                .text_size(px(9.0))
                .text_color(move || theme().foreground),
        )
        .child(
            text(move || format!("{} commits", commits.get().len()))
                .text_size(px(9.0))
                .text_color(move || theme().foreground),
        )
        .child(view().flex_1())
        .child(
            text("M1 slice 1")
                .text_size(px(9.0))
                .text_color(move || theme().primary),
        );

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
        .child(tab_bar)
        .child(recent_bar)
        .child(commit_bar)
        .child(main)
        .child(commit_log_panel)
        .child(statusbar)
}
