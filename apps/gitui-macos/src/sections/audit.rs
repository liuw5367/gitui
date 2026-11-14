//! Agent audit log viewer.

use crate::design;
use crate::widgets::header_button;
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct AuditActions {
    pub(crate) close: Arc<dyn Fn()>,
    pub(crate) refresh: Arc<dyn Fn()>,
}

pub(crate) fn audit_panel(
    open: Signal<bool>,
    lines: Signal<Vec<String>>,
    error: Signal<SharedString>,
    actions: AuditActions,
) -> impl IntoElement {
    view()
        .absolute()
        .inset(px(0.0))
        .w_full()
        .h_full()
        .z_index(move || if open.get() { 49 } else { -1 })
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
                .w(px(760.0))
                .h(px(540.0))
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
                            text("Agent audit log")
                                .flex_1()
                                .text_size(px(13.0))
                                .text_color(move || design::text()),
                        )
                        .child(header_button("Refresh", false, {
                            let action = actions.refresh.clone();
                            move || action()
                        }))
                        .child(header_button("Close", false, {
                            let action = actions.close.clone();
                            move || action()
                        })),
                )
                .child(
                    text(move || error.get())
                        .px_3()
                        .text_size(px(9.5))
                        .text_color(move || design::danger()),
                )
                .child(
                    view()
                        .flex_1()
                        .min_h_0()
                        .flex_col()
                        .p_3()
                        .gap_1()
                        .child(each(
                            lines,
                            |line: &String| line.clone(),
                            |line, _| {
                                view().child(
                                    text(move || line.get())
                                        .text_size(px(9.0))
                                        .text_color(move || design::text2()),
                                )
                            },
                        )),
                ),
        )
}
