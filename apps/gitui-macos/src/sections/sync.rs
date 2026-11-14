//! Sync sheet with Fetch / Pull / Push tabs, based on board 15.

use crate::design;
use crate::widgets::header_button;
use crate::SyncKind;
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct SyncActions {
    pub(crate) select: Arc<dyn Fn(SyncKind)>,
    pub(crate) fetch: Arc<dyn Fn()>,
    pub(crate) pull: Arc<dyn Fn()>,
    pub(crate) push: Arc<dyn Fn()>,
    pub(crate) close: Arc<dyn Fn()>,
}

fn run_button(kind: Signal<SyncKind>, actions: SyncActions) -> impl IntoElement {
    view()
        .h(px(26.0))
        .px_3()
        .rounded(px(6.0))
        .items_center()
        .justify_center()
        .bg(move || design::primary())
        .on_press(move || match kind.get_untracked() {
            SyncKind::Fetch => (actions.fetch)(),
            SyncKind::Pull => (actions.pull)(),
            SyncKind::Push => (actions.push)(),
        })
        .child(
            text(move || kind.get().label())
                .text_size(px(11.0))
                .text_color(move || design::primary_foreground()),
        )
}

fn tab(
    label: &'static str,
    target: SyncKind,
    kind: Signal<SyncKind>,
    select: Arc<dyn Fn(SyncKind)>,
) -> impl IntoElement {
    view()
        .h(px(22.0))
        .px_2()
        .rounded(px(4.0))
        .bg(move || {
            if kind.get() == target {
                design::content()
            } else {
                design::panel2()
            }
        })
        .on_press(move || select(target))
        .child(
            text(label)
                .text_size(px(10.5))
                .text_color(move || {
                    if kind.get() == target {
                        design::text()
                    } else {
                        design::text2()
                    }
                }),
        )
}

pub(crate) fn sync_panel(
    open: Signal<bool>,
    kind: Signal<SyncKind>,
    branch: Signal<SharedString>,
    ahead: Signal<usize>,
    behind: Signal<usize>,
    log: Signal<SharedString>,
    error: Signal<SharedString>,
    actions: SyncActions,
) -> impl IntoElement {
    let auth_required = move || {
        let message = error.get().to_lowercase();
        message.contains("auth")
            || message.contains("username")
            || message.contains("permission denied")
            || message.contains("terminal prompts disabled")
    };
    view()
        .absolute()
        .inset(px(0.0))
        .w_full()
        .h_full()
        .z_index(move || if open.get() { 30 } else { -1 })
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
                .w(px(800.0))
                .h(px(520.0))
                .flex_col()
                .rounded(px(12.0))
                .bg(move || design::content())
                .border_1()
                .border_color(move || design::line())
                .overflow_hidden()
                .child(
                    view()
                        .h(px(46.0))
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .bg(move || design::panel())
                        .border_b_1()
                        .border_color(move || design::line())
                        .child(
                            text("Sync")
                                .text_size(px(13.0))
                                .text_color(move || design::text()),
                        )
                        .child(
                            view()
                                .h(px(26.0))
                                .flex_row()
                                .items_center()
                                .gap_1()
                                .p_1()
                                .rounded(px(6.0))
                                .bg(move || design::panel2())
                                .child(tab("Fetch", SyncKind::Fetch, kind, actions.select.clone()))
                                .child(tab("Pull", SyncKind::Pull, kind, actions.select.clone()))
                                .child(tab("Push", SyncKind::Push, kind, actions.select.clone())),
                        )
                        .child(view().flex_1())
                        .child(header_button("Close", false, {
                            let action = actions.close.clone();
                            move || action()
                        })),
                )
                .child(
                    view()
                        .flex_1()
                        .min_h_0()
                        .flex_row()
                        .child(
                            view()
                                .flex_1()
                                .flex_col()
                                .gap_2()
                                .p_3()
                                .child(
                                    text(move || kind.get().label().to_uppercase())
                                        .text_size(px(9.0))
                                        .text_color(move || design::text3()),
                                )
                                .child(
                                    text(move || {
                                        format!(
                                            "{} → origin/{}",
                                            branch.get(),
                                            branch.get()
                                        )
                                    })
                                    .text_size(px(12.0))
                                    .text_color(move || design::text()),
                                )
                                .child(
                                    text(move || {
                                        format!("{} ahead · {} behind", ahead.get(), behind.get())
                                    })
                                    .text_size(px(10.0))
                                    .text_color(move || design::text3()),
                                )
                                .child(
                                    view()
                                        .h(px(1.0))
                                        .bg(move || design::line_soft()),
                                )
                                .child(
                                    text("Remote  origin")
                                        .text_size(px(10.0))
                                        .text_color(move || design::text2()),
                                )
                                .child(
                                    text("Credentials are handled by Git / SSH.")
                                        .text_size(px(10.0))
                                        .text_color(move || design::text3()),
                                )
                                .child(
                                    text(move || {
                                        if auth_required() {
                                            "Authentication required".to_string()
                                        } else if error.get().is_empty() {
                                            "Ready · system Git credentials".to_string()
                                        } else {
                                            error.get().to_string()
                                        }
                                    })
                                    .text_size(px(10.0))
                                    .text_color(move || {
                                        if auth_required() {
                                            design::danger()
                                        } else {
                                            design::token("ok")
                                        }
                                    }),
                                )
                                .child(view().flex_1())
                                .child(
                                    view()
                                        .flex_row()
                                        .gap_2()
                                        .child(run_button(kind, actions.clone()))
                                        .child(header_button("Cancel", false, {
                                            let action = actions.close.clone();
                                            move || action()
                                        })),
                                ),
                        )
                        .child(
                            view()
                                .flex_1()
                                .flex_col()
                                .gap_2()
                                .p_3()
                                .bg(move || design::panel())
                                .border_l_1()
                                .border_color(move || design::line())
                                .child(
                                    text("Activity")
                                        .text_size(px(9.0))
                                        .text_color(move || design::text3()),
                                )
                                .child(
                                    text(move || {
                                        if log.get().is_empty() {
                                            "No activity yet.".to_string()
                                        } else {
                                            log.get().to_string()
                                        }
                                    })
                                    .text_size(px(9.0))
                                    .text_color(move || design::text2()),
                                )
                                .child(view().flex_1())
                                .child(
                                    text("You can close this sheet after the operation.")
                                        .text_size(px(9.0))
                                        .text_color(move || design::text3()),
                                ),
                        ),
                )
                .child(
                    view()
                        .h(px(46.0))
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .bg(move || design::panel())
                        .border_t_1()
                        .border_color(move || design::line())
                        .child(
                            text(move || {
                                format!("{} ahead · {} behind", ahead.get(), behind.get())
                            })
                            .text_size(px(10.0))
                            .text_color(move || design::text2()),
                        )
                        .child(view().flex_1())
                        .child(header_button("Close", false, {
                            let action = actions.close.clone();
                            move || action()
                        })),
                ),
        )
}
