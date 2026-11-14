//! Window titlebar: repository tabs and local status.

use crate::design;
use crate::model::Loaded;
use nsh_core::element::desktop;
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
        .window_drag(true)
        .child(
            view()
                .w(move || px(desktop::title_bar().left.0 + 8.0))
                .h(px(1.0)),
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
