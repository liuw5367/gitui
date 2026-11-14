//! Bottom status bar.

use crate::design;
use gitui_git::Commit;
use nsh_core::prelude::*;

pub(crate) fn statusbar(commits: Signal<Vec<Commit>>) -> impl IntoElement {
    view()
        .h(px(24.0))
        .flex_row()
        .items_center()
        .gap_3()
        .px_3()
        .bg(move || design::statusbar())
        .border_t_1()
        .border_color(move || design::line())
        .child(
            text("gitui · local")
                .text_size(px(10.5))
                .text_color(move || design::text3()),
        )
        .child(
            text(move || format!("{} commits", commits.get().len()))
                .text_size(px(10.5))
                .text_color(move || design::text3()),
        )
        .child(view().flex_1())
        .child(
            text("M4")
                .text_size(px(10.5))
                .text_color(move || design::text2()),
        )
}
