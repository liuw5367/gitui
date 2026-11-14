//! Working tree, branches and conflicts sidebar.

use gitui_git::{Branch, StatusEntry};
use nsh_base::{input, theme};
use nsh_core::prelude::*;
use std::sync::Arc;

use crate::model::short_name;
use crate::widgets::{header_button, mini_button};

#[derive(Clone)]
pub(crate) struct StatusActions {
    pub(crate) stage_all: Arc<dyn Fn()>,
    pub(crate) unstage_all: Arc<dyn Fn()>,
    pub(crate) stage_path: Arc<dyn Fn(String)>,
    pub(crate) unstage_path: Arc<dyn Fn(String)>,
    pub(crate) checkout: Arc<dyn Fn(String)>,
    pub(crate) create_branch: Arc<dyn Fn(String)>,
    pub(crate) select_conflict: Arc<dyn Fn(String)>,
}

pub(crate) fn status_pane(
    branch: Signal<SharedString>,
    staged: Signal<usize>,
    unstaged: Signal<usize>,
    status_entries: Signal<Vec<StatusEntry>>,
    branches: Signal<Vec<Branch>>,
    conflicts: Signal<Vec<String>>,
    conflict_path: Signal<String>,
    error: Signal<SharedString>,
    branch_name: Signal<String>,
    actions: StatusActions,
) -> impl IntoElement {
    view()
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
                    let action = actions.stage_all.clone();
                    move || action()
                }))
                .child(header_button("Unstage All", false, {
                    let action = actions.unstage_all.clone();
                    move || action()
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
                    let action = actions.create_branch.clone();
                    move || action(branch_name.get_untracked().trim().to_string())
                })),
        )
        .child(each(
            branches,
            |branch: &Branch| branch.name.clone(),
            move |branch, _| {
                let name = branch.get().name.clone();
                let current = branch.get().current;
                let action = actions.checkout.clone();
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
                        if !current {
                            action(name.clone());
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
                let selected = path.get();
                let action = actions.select_conflict.clone();
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
                    .on_press(move || action(selected.clone()))
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
                    let action = actions.stage_path.clone();
                    row = row.child(mini_button("Stage", true, move || action(path.clone())));
                }
                if staged {
                    let path = path.clone();
                    let action = actions.unstage_path.clone();
                    row = row.child(mini_button("Unstage", false, move || action(path.clone())));
                }
                row
            },
        ))
}
