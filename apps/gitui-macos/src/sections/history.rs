//! Commit history list.

use gitui_git::Commit;
use nsh_base::theme;
use nsh_core::prelude::*;
use std::sync::Arc;

pub(crate) fn history(
    commits: Signal<Vec<Commit>>,
    selected: Signal<SharedString>,
    on_select: Arc<dyn Fn(String)>,
) -> impl IntoElement {
    view()
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
                let on_select = on_select.clone();
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
                    .on_press(move || on_select(sha.clone()))
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
        ))
}
