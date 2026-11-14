//! Window titlebar: repository tabs and local status.

use crate::model::Loaded;
use crate::design;
use nsh_core::prelude::*;
use std::sync::Arc;

pub(crate) fn titlebar(
    tabs: Signal<Vec<Loaded>>,
    active: Signal<usize>,
    activate: Arc<dyn Fn(usize)>,
) -> impl IntoElement {
    view()
        .h(px(44.0))
        .flex_row()
        .items_center()
        .gap_2()
        .px_3()
        .bg(move || design::titlebar())
        .border_b_1()
        .border_color(move || design::line())
        .child(
            text("gitui")
                .text_size(px(13.0))
                .text_color(move || design::text()),
        )
        .child(
            view()
                .flex_1()
                .min_w_0()
                .child(crate::sections::tabs(tabs, active, activate)),
        )
        .child(
            text("local")
                .text_size(px(10.5))
                .text_color(move || design::text3()),
        )
}
