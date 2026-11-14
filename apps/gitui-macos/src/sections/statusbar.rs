//! Bottom status bar.

use gitui_git::Commit;
use nsh_base::theme;
use nsh_core::prelude::*;

pub(crate) fn statusbar(commits: Signal<Vec<Commit>>) -> impl IntoElement {
    view()
        .h(px(24.0))
        .flex_row()
        .items_center()
        .gap_3()
        .px_3()
        .bg(move || theme().background)
        .child(
            text("NSH + Rust · local")
                .text_size(px(9.0))
                .text_color(move || theme().foreground),
        )
        .child(
            text(move || format!("{} commits", commits.get().len()))
                .text_size(px(9.0))
                .text_color(move || theme().foreground),
        )
        .child(view().flex_1())
        .child(
            text("M1/M2")
                .text_size(px(9.0))
                .text_color(move || theme().primary),
        )
}
