//! Read-only selected-commit diff.

use nsh_base::{editor, theme};
use nsh_core::prelude::*;

pub(crate) fn diff_view(selected: Signal<SharedString>, diff: Signal<SharedString>) -> impl IntoElement {
    view()
        .flex_1()
        .flex_col()
        .gap_2()
        .p_2()
        .child(
            text(move || format!("Diff · {}", selected.get()))
                .text_size(px(10.0))
                .text_color(move || theme().foreground),
        )
        .child(
            editor()
                .value(move || Some(diff.get()))
                .read_only(true)
                .language("diff")
                .font_size(11.0)
                .line_height(17.0)
                .flex_1(),
        )
}
