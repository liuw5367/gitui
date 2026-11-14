//! Changes list for the Changes mode.

use crate::design;
use crate::widgets::mini_button;
use crate::Mode;
use gitui_git::StatusEntry;
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct ChangesActions {
    pub(crate) stage_all: Arc<dyn Fn()>,
    pub(crate) unstage_all: Arc<dyn Fn()>,
    pub(crate) stage_path: Arc<dyn Fn(String)>,
    pub(crate) unstage_path: Arc<dyn Fn(String)>,
    pub(crate) select_file: Arc<dyn Fn(String)>,
}

fn section_head(
    label: &'static str,
    count: impl Fn() -> usize + 'static,
) -> impl IntoElement {
    view()
        .h(px(24.0))
        .flex_row()
        .items_center()
        .gap_2()
        .px_2()
        .bg(move || design::panel2())
        .border_b_1()
        .border_color(move || design::line_soft())
        .child(
            text(label)
                .text_size(px(10.0))
                .text_color(move || design::text2()),
        )
        .child(
            text(move || count().to_string())
                .flex_1()
                .text_size(px(10.0))
                .text_color(move || design::text3()),
        )
}

fn file_row(
    entry: StatusEntry,
    staged: bool,
    selected_file: Signal<String>,
    actions: ChangesActions,
) -> impl IntoElement {
    let path = entry.path.clone();
    let select_path = path.clone();
    let status = format!("{}{}", entry.x, entry.y);
    let action = if staged {
        actions.unstage_path.clone()
    } else {
        actions.stage_path.clone()
    };
    let label = if staged { "Unstage" } else { "Stage" };
    let primary = !staged;
    let action_path = path.clone();
    let path_text = path.clone();
    view()
        .h(px(32.0))
        .flex_row()
        .items_center()
        .gap_2()
        .px_2()
        .border_b_1()
        .border_color(move || design::line_soft())
        .bg(move || {
            if selected_file.get() == path {
                design::selected()
            } else {
                design::content()
            }
        })
        .on_press(move || {
            let select = actions.select_file.clone();
            select(select_path.clone());
        })
        .child(
            text(move || path_text.clone())
                .flex_1()
                .text_size(px(10.5))
                .text_color(move || design::text()),
        )
        .child(
            text(status)
                .w(px(42.0))
                .text_size(px(9.5))
                .text_color(move || design::text3()),
        )
        .child(mini_button(label, primary, move || action(action_path.clone())))
}

fn changes_section(
    label: &'static str,
    entries: Signal<Vec<StatusEntry>>,
    staged: bool,
    selected_file: Signal<String>,
    actions: ChangesActions,
) -> impl IntoElement {
    view().flex_col().child(section_head(label, move || entries.get().len())).child(each(
        entries,
        |entry: &StatusEntry| entry.path.clone(),
        move |entry, _| file_row(entry.get(), staged, selected_file, actions.clone()),
    ))
}

pub(crate) fn changes_list(
    mode: Signal<Mode>,
    focus: Signal<bool>,
    staged_entries: Signal<Vec<StatusEntry>>,
    unstaged_entries: Signal<Vec<StatusEntry>>,
    selected_file: Signal<String>,
    actions: ChangesActions,
) -> impl IntoElement {
    view()
        .h(move || {
            if !focus.get() && mode.get() == Mode::Changes {
                px(342.0)
            } else {
                px(0.0)
            }
        })
        .w_full()
        .flex_col()
        .bg(move || design::content())
        .border_b_1()
        .border_color(move || design::line())
        .child(
            view()
                .h(px(32.0))
                .flex_row()
                .items_center()
                .gap_2()
                .px_2()
                .border_b_1()
                .border_color(move || design::line_soft())
                .child(
                    text("Changes")
                        .text_size(px(12.0))
                        .text_color(move || design::text()),
                )
                .child(view().flex_1())
                .child(
                    text(move || {
                        format!(
                            "{} staged · {} unstaged",
                            staged_entries.get().len(),
                            unstaged_entries.get().len()
                        )
                    })
                    .text_size(px(10.0))
                    .text_color(move || design::text3()),
                )
                .child(mini_button("Stage all", true, {
                    let action = actions.stage_all.clone();
                    move || action()
                }))
                .child(mini_button("Unstage all", false, {
                    let action = actions.unstage_all.clone();
                    move || action()
                })),
        )
        .child(
            view()
                .h(px(23.0))
                .flex_row()
                .items_center()
                .gap_2()
                .px_2()
                .border_b_1()
                .border_color(move || design::line_soft())
                .child(view().w(px(18.0)))
                .child(
                    text("Path")
                        .flex_1()
                        .text_size(px(10.0))
                        .text_color(move || design::text3()),
                )
                .child(
                    text("Status")
                        .w(px(42.0))
                        .text_size(px(10.0))
                        .text_color(move || design::text3()),
                )
                .child(view().w(px(62.0))),
        )
        .child(
            view()
                .flex_1()
                .min_h_0()
                .flex_col()
                .child(changes_section(
                    "Staged",
                    staged_entries,
                    true,
                    selected_file,
                    actions.clone(),
                ))
                .child(changes_section(
                    "Unstaged",
                    unstaged_entries,
                    false,
                    selected_file,
                    actions,
                )),
        )
}
