//! Source-list sidebar: working tree, branches, tags, remotes, stashes, conflicts.

use crate::design;
use crate::model::short_name;
use crate::sections::stashes::StashActions;
use crate::widgets::mini_button;
use gitui_git::{Branch, Remote, Stash, StatusEntry, Tag};
use nsh_base::input;
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct StatusActions {
    pub(crate) stage_all: Arc<dyn Fn()>,
    pub(crate) unstage_all: Arc<dyn Fn()>,
    pub(crate) stage_path: Arc<dyn Fn(String)>,
    pub(crate) unstage_path: Arc<dyn Fn(String)>,
    pub(crate) checkout: Arc<dyn Fn(String)>,
    pub(crate) create_branch: Arc<dyn Fn(String)>,
    pub(crate) delete_branch: Arc<dyn Fn(String)>,
    pub(crate) add_remote: Arc<dyn Fn(String, String)>,
    pub(crate) remove_remote: Arc<dyn Fn(String)>,
    pub(crate) select_file: Arc<dyn Fn(String)>,
    pub(crate) rename_branch: Arc<dyn Fn(String)>,
    pub(crate) merge_branch: Arc<dyn Fn(String)>,
    pub(crate) rebase_branch: Arc<dyn Fn(String)>,
    pub(crate) create_tag: Arc<dyn Fn(String)>,
    pub(crate) delete_tag: Arc<dyn Fn(String)>,
    pub(crate) select_conflict: Arc<dyn Fn(String)>,
}

fn side_head(label: &'static str, count: impl Fn() -> usize + 'static) -> impl IntoElement {
    view()
        .h(px(27.0))
        .flex_row()
        .items_center()
        .gap_1()
        .px_1()
        .mt_1()
        .child(
            text(label)
                .text_size(px(10.5))
                .text_color(move || design::text3()),
        )
        .child(view().flex_1())
        .child(
            text(move || count().to_string())
                .text_size(px(10.5))
                .text_color(move || design::text3()),
        )
}

pub(crate) fn status_pane(
    branch: Signal<SharedString>,
    staged: Signal<usize>,
    unstaged: Signal<usize>,
    status_entries: Signal<Vec<StatusEntry>>,
    branches: Signal<Vec<Branch>>,
    tags: Signal<Vec<Tag>>,
    remotes: Signal<Vec<Remote>>,
    stashes: Signal<Vec<Stash>>,
    conflicts: Signal<Vec<String>>,
    conflict_path: Signal<String>,
    error: Signal<SharedString>,
    branch_name: Signal<String>,
    tag_name: Signal<String>,
    remote_name: Signal<String>,
    remote_url: Signal<String>,
    actions: StatusActions,
    stash_actions: StashActions,
) -> impl IntoElement {
    let delete_armed = signal(false);
    let tag_delete_armed = signal(false);
    let remote_delete_armed = signal(false);

    view()
        .w(px(232.0))
        .flex_col()
        .gap_1()
        .p_2()
        .bg(move || design::sidebar())
        .border_r_1()
        .border_color(move || design::line())
        .child(
            text("Working tree")
                .text_size(px(11.0))
                .text_color(move || design::text()),
        )
        .child(
            text(move || format!("{} staged · {} unstaged", staged.get(), unstaged.get()))
                .text_size(px(10.5))
                .text_color(move || design::text3()),
        )
        .child(
            text(move || branch.get())
                .text_size(px(12.0))
                .text_color(move || design::text2()),
        )
        .child(
            view()
                .flex_row()
                .gap_1()
                .child(mini_button("Stage All", true, {
                    let action = actions.stage_all.clone();
                    move || action()
                }))
                .child(mini_button("Unstage All", false, {
                    let action = actions.unstage_all.clone();
                    move || action()
                })),
        )
        .child(side_head("Branches", move || branches.get().len()))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_1()
                .child(
                    input()
                        .value(branch_name)
                        .on_change(move |text: &str| branch_name.set(text.to_string()))
                        .placeholder("branch-name")
                        .text_size(px(10.5))
                        .flex_1(),
                )
                .child(mini_button("New", true, {
                    let action = actions.create_branch.clone();
                    move || action(branch_name.get_untracked().trim().to_string())
                })),
        )
        .child(
            view()
                .flex_row()
                .gap_1()
                .child(mini_button("Delete", false, {
                    let action = actions.delete_branch.clone();
                    move || {
                        let name = branch_name.get_untracked().trim().to_string();
                        if delete_armed.get_untracked() {
                            delete_armed.set(false);
                            action(name);
                        } else {
                            delete_armed.set(true);
                        }
                    }
                }))
                .child(mini_button("Rename", false, {
                    let action = actions.rename_branch.clone();
                    move || action(branch_name.get_untracked().trim().to_string())
                }))
                .child(mini_button("Merge", false, {
                    let action = actions.merge_branch.clone();
                    move || action(branch_name.get_untracked().trim().to_string())
                }))
                .child(mini_button("Rebase", false, {
                    let action = actions.rebase_branch.clone();
                    move || action(branch_name.get_untracked().trim().to_string())
                })),
        )
        .child(
            text(move || {
                if delete_armed.get() {
                    "Delete again to confirm".to_string()
                } else {
                    String::new()
                }
            })
            .text_size(px(10.0))
            .text_color(move || design::danger()),
        )
        .child(each(
            branches,
            |branch: &Branch| branch.name.clone(),
            move |branch, _| {
                let name = branch.get().name.clone();
                let current = branch.get().current;
                let action = actions.checkout.clone();
                view()
                    .h(px(26.0))
                    .px_1()
                    .rounded(px(6.0))
                    .bg(move || {
                        if branch.get().current {
                            design::primary()
                        } else {
                            design::sidebar()
                        }
                    })
                    .on_press(move || {
                        if !current {
                            action(name.clone());
                        }
                    })
                    .child(
                        text(move || branch.get().name)
                            .text_size(px(12.0))
                            .text_color(move || {
                                if branch.get().current {
                                    design::primary_foreground()
                                } else {
                                    design::text()
                                }
                            }),
                    )
            },
        ))
        .child(side_head("Tags", move || tags.get().len()))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_1()
                .child(
                    input()
                        .value(tag_name)
                        .on_change(move |text: &str| tag_name.set(text.to_string()))
                        .placeholder("tag-name")
                        .text_size(px(10.5))
                        .flex_1(),
                )
                .child(mini_button("Create", true, {
                    let action = actions.create_tag.clone();
                    move || action(tag_name.get_untracked().trim().to_string())
                }))
                .child(mini_button("Delete", false, {
                    let action = actions.delete_tag.clone();
                    move || {
                        let name = tag_name.get_untracked().trim().to_string();
                        if tag_delete_armed.get_untracked() {
                            tag_delete_armed.set(false);
                            action(name);
                        } else {
                            tag_delete_armed.set(true);
                        }
                    }
                })),
        )
        .child(
            text(move || {
                if tag_delete_armed.get() {
                    "Delete tag again to confirm".to_string()
                } else {
                    String::new()
                }
            })
            .text_size(px(10.0))
            .text_color(move || design::danger()),
        )
        .child(each(
            tags,
            |tag: &Tag| tag.name.clone(),
            |tag, _| {
                view()
                    .h(px(26.0))
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .px_1()
                    .child(
                        view()
                            .w(px(7.0))
                            .h(px(7.0))
                            .rounded(px(2.0))
                            .bg(move || design::warn()),
                    )
                    .child(
                        text(move || tag.get().name)
                            .text_size(px(12.0))
                            .text_color(move || design::text()),
                    )
            },
        ))
        .child(side_head("Remotes", move || remotes.get().len()))
        .child(
            view()
                .flex_col()
                .gap_1()
                .child(
                    input()
                        .value(remote_name)
                        .on_change(move |text: &str| remote_name.set(text.to_string()))
                        .placeholder("remote")
                        .text_size(px(10.5)),
                )
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .child(
                            input()
                                .value(remote_url)
                                .on_change(move |text: &str| remote_url.set(text.to_string()))
                                .placeholder("url")
                                .text_size(px(10.5))
                                .flex_1(),
                        )
                        .child(mini_button("Add", true, {
                            let action = actions.add_remote.clone();
                            move || {
                                action(
                                    remote_name.get_untracked().trim().to_string(),
                                    remote_url.get_untracked().trim().to_string(),
                                )
                            }
                        })),
                ),
        )
        .child(each(
            remotes,
            |remote: &Remote| remote.name.clone(),
            move |remote, _| {
                let name = remote.get().name.clone();
                let action = actions.remove_remote.clone();
                view()
                    .flex_col()
                    .gap_0()
                    .px_1()
                    .child(
                        view()
                            .h(px(22.0))
                            .flex_row()
                            .items_center()
                            .gap_1()
                            .child(
                                text(move || remote.get().name)
                                    .text_size(px(12.0))
                                    .text_color(move || design::text()),
                            )
                            .child(view().flex_1())
                            .child(mini_button("Remove", false, move || {
                                if remote_delete_armed.get_untracked() {
                                    remote_delete_armed.set(false);
                                    action(name.clone());
                                } else {
                                    remote_delete_armed.set(true);
                                }
                            })),
                    )
                    .child(
                        text(move || remote.get().url)
                            .text_size(px(10.0))
                            .text_color(move || design::text3()),
                    )
            },
        ))
        .child(side_head("Stashes", move || stashes.get().len()))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_1()
                .child(
                    text("Work in progress")
                        .text_size(px(10.0))
                        .text_color(move || design::text3()),
                )
                .child(view().flex_1())
                .child(mini_button("Stash", true, {
                    let action = stash_actions.push.clone();
                    move || action()
                })),
        )
        .child(each(
            stashes,
            |stash: &Stash| stash.selector.clone(),
            move |stash, _| {
                let selector = stash.get().selector.clone();
                let apply_selector = selector.clone();
                let pop_selector = selector.clone();
                let drop_selector = selector;
                let apply = stash_actions.apply.clone();
                let pop = stash_actions.pop.clone();
                let drop = stash_actions.drop.clone();
                view()
                    .flex_col()
                    .gap_1()
                    .px_1()
                    .py_1()
                    .child(
                        text(move || stash.get().message)
                            .text_size(px(10.5))
                            .text_color(move || design::text2()),
                    )
                    .child(
                        view()
                            .flex_row()
                            .gap_1()
                            .child(mini_button("Apply", true, move || apply(apply_selector.clone())))
                            .child(mini_button("Pop", false, move || pop(pop_selector.clone())))
                            .child(mini_button("Drop", false, move || drop(drop_selector.clone()))),
                    )
            },
        ))
        .child(side_head("Conflicts", move || conflicts.get().len()))
        .child(each(
            conflicts,
            |path: &String| path.clone(),
            move |path, _| {
                let selected = path.get();
                let action = actions.select_conflict.clone();
                view()
                    .h(px(26.0))
                    .px_1()
                    .rounded(px(6.0))
                    .bg(move || {
                        if conflict_path.get() == path.get() {
                            design::primary()
                        } else {
                            design::sidebar()
                        }
                    })
                    .on_press(move || action(selected.clone()))
                    .child(
                        text(move || path.get())
                            .text_size(px(10.5))
                            .text_color(move || {
                                if conflict_path.get() == path.get() {
                                    design::primary_foreground()
                                } else {
                                    design::danger()
                                }
                            }),
                    )
            },
        ))
        .child(
            text(move || error.get())
                .text_size(px(10.0))
                .text_color(move || design::danger()),
        )
        .child(
            text(move || {
                if status_entries.get().is_empty() {
                    "Clean working tree".to_string()
                } else {
                    String::new()
                }
            })
            .text_size(px(10.5))
            .text_color(move || design::text3()),
        )
        .child(each(
            status_entries,
            |entry: &StatusEntry| entry.path.clone(),
            move |entry, _| {
                let path = entry.get().path.clone();
                let staged = entry.get().is_staged();
                let unstaged = entry.get().is_unstaged();
                let select = actions.select_file.clone();
                let select_path = path.clone();
                let mut row = view()
                    .h(px(28.0))
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .on_press(move || select(select_path.clone()))
                    .child(
                        text(move || short_name(&entry.get().path))
                            .text_size(px(11.0))
                            .text_color(move || design::text())
                            .flex_1(),
                    );
                if unstaged {
                    let action = actions.stage_path.clone();
                    let stage_path = path.clone();
                    row = row.child(mini_button("Stage", true, move || action(stage_path.clone())));
                }
                if staged {
                    let action = actions.unstage_path.clone();
                    let unstage_path = path;
                    row = row.child(mini_button("Unstage", false, move || action(unstage_path.clone())));
                }
                row
            },
        ))
}
