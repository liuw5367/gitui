//! gitui M1 — NSH + Rust, read-only Git browser.
//!
//! Slice 1: open the current repository, show status, recent history and the
//! selected commit's patch. No staging, commit, push or settings.

use gitui_git::{Commit, Repo};
use nsh_base::{editor, input, theme};
use nsh_core::prelude::*;
use std::path::PathBuf;

struct Loaded {
    path: String,
    branch: String,
    staged: usize,
    unstaged: usize,
    status_lines: Vec<SharedString>,
    commits: Vec<Commit>,
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
            Loaded {
                path: repo.root().display().to_string(),
                branch,
                staged: status.staged,
                unstaged: status.unstaged,
                status_lines: status.lines.into_iter().map(SharedString::from).collect(),
                commits,
                diff: SharedString::from(diff),
                error: SharedString::from(""),
            }
        }
        Err(error) => Loaded {
            path: path.to_string(),
            branch: "—".to_string(),
            staged: 0,
            unstaged: 0,
            status_lines: Vec::new(),
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
    let first_sha = initial
        .commits
        .first()
        .map(|commit| commit.sha.clone())
        .unwrap_or_default();

    let path_input = signal(initial.path.clone());
    let mut saved_recent = load_recent();
    if !saved_recent.iter().any(|path| path == &initial.path) {
        saved_recent.insert(0, initial.path.clone());
    }
    saved_recent.truncate(5);
    let recent = signal(
        saved_recent
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    );
    save_recent(recent.get_untracked().as_slice());
    let repo_path = signal(SharedString::from(initial.path));
    let branch = signal(SharedString::from(initial.branch));
    let staged = signal(initial.staged);
    let unstaged = signal(initial.unstaged);
    let status_lines = signal(initial.status_lines);
    let commits = signal(initial.commits);
    let selected = signal(SharedString::from(first_sha));
    let diff = signal(initial.diff);
    let error = signal(initial.error);

    let reload = move || {
        let requested = path_input.get_untracked();
        let loaded = load(&requested);
        let first = loaded
            .commits
            .first()
            .map(|commit| commit.sha.clone())
            .unwrap_or_default();
        repo_path.set(SharedString::from(loaded.path.clone()));
        path_input.set(loaded.path.clone());
        branch.set(SharedString::from(loaded.branch));
        staged.set(loaded.staged);
        unstaged.set(loaded.unstaged);
        status_lines.set(loaded.status_lines);
        commits.set(loaded.commits);
        selected.set(SharedString::from(first));
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
    };

    let reload_for_recent = reload.clone();

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
        .child(header_button("Open", true, reload));

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
            text(move || error.get())
                .text_size(px(9.0))
                .text_color(|| rgb(0xff3b30)),
        )
        .child(
            text(move || {
                if status_lines.get().is_empty() {
                    "Clean working tree".to_string()
                } else {
                    String::new()
                }
            })
            .text_size(px(9.0))
            .text_color(move || theme().foreground),
        )
        .child(each(
            status_lines,
            |line: &SharedString| line.clone(),
            |line, _| {
                view().h(px(18.0)).child(
                    text(move || line.get())
                        .text_size(px(9.5))
                        .text_color(move || theme().foreground),
                )
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
                                    diff.set(SharedString::from(patch));
                                    error.set(SharedString::from(""));
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

    view()
        .flex_1()
        .bg(move || theme().background)
        .text_color(move || theme().foreground)
        .child(header)
        .child(recent_bar)
        .child(main)
        .child(statusbar)
}
