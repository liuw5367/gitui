//! Small presentational widgets shared by the app sections.

use nsh_base::{editor, theme};
use nsh_core::prelude::*;

pub(crate) fn mini_button(
    label: &'static str,
    primary: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    view()
        .px_2()
        .py_1()
        .rounded(px(3.0))
        .bg(move || if primary { theme().primary } else { theme().background })
        .on_press(on_press)
        .child(
            text(label)
                .text_size(px(8.5))
                .text_color(move || {
                    if primary {
                        theme().primary_foreground
                    } else {
                        theme().primary
                    }
                }),
        )
}

pub(crate) fn conflict_pane(title: &'static str, value: impl Fn() -> String + 'static) -> impl IntoElement {
    view()
        .flex_1()
        .flex_col()
        .gap_1()
        .child(
            text(title)
                .text_size(px(9.0))
                .text_color(move || theme().foreground),
        )
        .child(
            editor()
                .value(move || Some(SharedString::from(value())))
                .read_only(true)
                .font_size(10.0)
                .line_height(15.0)
                .flex_1(),
        )
}


pub(crate) fn header_button(
    label: &'static str,
    primary: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    view()
        .px_3()
        .py_2()
        .rounded(px(5.0))
        .bg(move || if primary { theme().primary } else { theme().background })
        .on_press(on_press)
        .child(
            text(label)
                .text_size(px(11.0))
                .text_color(move || {
                    if primary {
                        theme().primary_foreground
                    } else {
                        theme().primary
                    }
                }),
        )
}

