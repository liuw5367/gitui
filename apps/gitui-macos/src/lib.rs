//! gitui M1 — NSH + Rust, read-only Git browser.
//!
//! Slice 1: open the current repository, show status, recent history and the
//! selected commit's patch. No staging, commit, push or settings.

use gitui_git::{Commit, Repo};
use nsh_base::{editor, theme};
use nsh_core::prelude::*;

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
        let loaded = load(repo_path.get_untracked().to_string().as_str());
        let first = loaded
            .commits
            .first()
            .map(|commit| commit.sha.clone())
            .unwrap_or_default();
        repo_path.set(SharedString::from(loaded.path));
        branch.set(SharedString::from(loaded.branch));
        staged.set(loaded.staged);
        unstaged.set(loaded.unstaged);
        status_lines.set(loaded.status_lines);
        commits.set(loaded.commits);
        selected.set(SharedString::from(first));
        diff.set(loaded.diff);
        error.set(loaded.error);
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
            text(move || repo_path.get())
                .text_size(px(10.0))
                .text_color(move || theme().foreground),
        )
        .child(
            text(move || branch.get())
                .text_size(px(10.0))
                .text_color(move || theme().primary),
        )
        .child(view().flex_1())
        .child(header_button("Reload", true, reload));

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
        .child(main)
        .child(statusbar)
}
