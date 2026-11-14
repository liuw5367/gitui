//! Right commit inspector for the Changes mode.

use crate::design;
use crate::sections::CommitActions;
use crate::widgets::header_button;
use crate::Mode;
use gitui_git::StatusEntry;
use nsh_base::input;
use nsh_core::prelude::*;

pub(crate) fn inspector(
    mode: Signal<Mode>,
    branch: Signal<SharedString>,
    staged: Signal<usize>,
    unstaged: Signal<usize>,
    staged_entries: Signal<Vec<StatusEntry>>,
    message: Signal<String>,
    commit_log: Signal<SharedString>,
    actions: CommitActions,
) -> impl IntoElement {
    view()
        .w(move || {
            if mode.get() == Mode::Changes {
                px(360.0)
            } else {
                px(0.0)
            }
        })
        .flex_col()
        .min_h_0()
        .bg(move || design::panel())
        .border_l_1()
        .border_color(move || design::line())
        .child(
            view()
                .h(px(33.0))
                .flex_row()
                .items_center()
                .gap_3()
                .px_3()
                .border_b_1()
                .border_color(move || design::line())
                .child(
                    text("Commit")
                        .text_size(px(11.5))
                        .text_color(move || design::text()),
                )
                .child(
                    text("Agent")
                        .text_size(px(11.5))
                        .text_color(move || design::text3()),
                ),
        )
        .child(
            view()
                .flex_1()
                .min_h_0()
                .flex_col()
                .gap_2()
                .p_2()
                .child(
                    text(move || format!("Commit to {}", branch.get()))
                        .text_size(px(11.0))
                        .text_color(move || design::text2()),
                )
                .child(
                    text(move || format!("{} staged · {} unstaged", staged.get(), unstaged.get()))
                        .text_size(px(10.0))
                        .text_color(move || design::text3()),
                )
                .child(
                    text("Summary")
                        .text_size(px(10.5))
                        .text_color(move || design::text2()),
                )
                .child(
                    input()
                        .value(message)
                        .on_change(move |text: &str| message.set(text.to_string()))
                        .placeholder("Commit summary")
                        .text_size(px(11.0)),
                )
                .child(
                    text(move || format!("Staged files · {}", staged_entries.get().len()))
                        .text_size(px(10.0))
                        .text_color(move || design::text3()),
                )
                .child(each(
                    staged_entries,
                    |entry: &StatusEntry| entry.path.clone(),
                    |entry, _| {
                        view().h(px(22.0)).child(
                            text(move || entry.get().path)
                                .text_size(px(10.0))
                                .text_color(move || design::text()),
                        )
                    },
                ))
                .child(view().flex_1())
                .child(
                    text(move || {
                        if commit_log.get().is_empty() {
                            String::new()
                        } else {
                            commit_log.get().to_string()
                        }
                    })
                    .text_size(px(9.0))
                    .text_color(move || design::text3()),
                )
                .child(
                    view()
                        .flex_row()
                        .gap_1()
                        .child(header_button("Commit", true, {
                            let action = actions.commit.clone();
                            move || action()
                        }))
                        .child(header_button("No hooks", false, {
                            let action = actions.no_hooks.clone();
                            move || action()
                        }))
                        .child(header_button("Amend", false, {
                            let action = actions.amend.clone();
                            move || action()
                        })),
                ),
        )
}
