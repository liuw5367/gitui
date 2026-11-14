//! Push preview and confirmation.

use crate::design;
use crate::widgets::header_button;
use gitui_git::{Commit, PushPreview};
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct PushActions {
    pub(crate) close: Arc<dyn Fn()>,
    pub(crate) confirm: Arc<dyn Fn()>,
}

pub(crate) fn push_panel(
    open: Signal<bool>,
    preview: Signal<Option<PushPreview>>,
    commits: Signal<Vec<Commit>>,
    error: Signal<SharedString>,
    actions: PushActions,
) -> impl IntoElement {
    view()
        .absolute()
        .inset(px(0.0))
        .w_full()
        .h_full()
        .z_index(move || if open.get() { 52 } else { -1 })
        .items_center()
        .justify_center()
        .p(px(24.0))
        .bg(move || {
            if open.get() {
                AdaptiveColor::from(rgb(0x000000)).opacity(0.28)
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .child(
            view()
                .w(px(640.0))
                .h(px(460.0))
                .flex_col()
                .rounded(px(12.0))
                .bg(move || design::content())
                .border_1()
                .border_color(move || design::line())
                .overflow_hidden()
                .child(
                    view()
                        .h(px(44.0))
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .bg(move || design::panel())
                        .border_b_1()
                        .border_color(move || design::line())
                        .child(
                            text("Push")
                                .text_size(px(13.0))
                                .text_color(move || design::text()),
                        )
                        .child(
                            text(move || {
                                preview
                                    .get()
                                    .map(|preview| {
                                        format!(
                                            "{} · {} commit(s)",
                                            preview.upstream,
                                            preview.commits.len()
                                        )
                                    })
                                    .unwrap_or_else(|| "no upstream".to_string())
                            })
                            .flex_1()
                            .text_size(px(9.5))
                            .text_color(move || design::text3()),
                        )
                        .child(header_button("Cancel", false, {
                            let action = actions.close.clone();
                            move || action()
                        })),
                )
                .child(
                    view()
                        .flex_1()
                        .min_h_0()
                        .flex_col()
                        .gap_2()
                        .p_3()
                        .child(
                            text("Commits to push")
                                .text_size(px(10.0))
                                .text_color(move || design::text3()),
                        )
                        .child(
                            view()
                                .flex_1()
                                .min_h_0()
                                .flex_col()
                                .child(each(
                                    commits,
                                    |commit: &Commit| commit.sha.clone(),
                                    |commit, _| {
                                        view()
                                            .h(px(30.0))
                                            .flex_row()
                                            .items_center()
                                            .gap_2()
                                            .border_b_1()
                                            .border_color(move || design::line_soft())
                                            .child(
                                                text(move || commit.get().short)
                                                    .w(px(60.0))
                                                    .text_size(px(9.5))
                                                    .text_color(move || design::text3()),
                                            )
                                            .child(
                                                text(move || commit.get().summary)
                                                    .flex_1()
                                                    .text_size(px(10.5))
                                                    .text_color(move || design::text()),
                                            )
                                    },
                                )),
                        )
                        .child(
                            text(move || error.get())
                                .text_size(px(9.5))
                                .text_color(move || design::danger()),
                        )
                        .child(
                            text("This pushes HEAD to the configured upstream. No force.")
                                .text_size(px(9.0))
                                .text_color(move || design::text3()),
                        )
                        .child(
                            view()
                                .flex_row()
                                .justify_end()
                                .child(header_button("Push", true, {
                                    let action = actions.confirm.clone();
                                    move || action()
                                })),
                        ),
                ),
        )
}
