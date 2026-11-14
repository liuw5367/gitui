//! Command palette.

use crate::design;
use nsh_base::input;
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct PaletteCommand {
    pub(crate) label: &'static str,
    pub(crate) hint: &'static str,
    pub(crate) action: Arc<dyn Fn()>,
}

pub(crate) fn palette_panel(
    open: Signal<bool>,
    query: Signal<String>,
    commands: Vec<PaletteCommand>,
    close: Arc<dyn Fn()>,
) -> impl IntoElement {
    view()
        .absolute()
        .inset(px(0.0))
        .w_full()
        .h_full()
        .z_index(move || if open.get() { 50 } else { -1 })
        .items_start()
        .justify_center()
        .pt(px(90.0))
        .bg(move || {
            if open.get() {
                AdaptiveColor::from(rgb(0x000000)).opacity(0.24)
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .child(
            view()
                .w(px(560.0))
                .h(px(420.0))
                .flex_col()
                .rounded(px(10.0))
                .bg(move || design::content())
                .border_1()
                .border_color(move || design::line())
                .overflow_hidden()
                .child(
                    input()
                        .value(query)
                        .on_change(move |text: &str| query.set(text.to_string()))
                        .placeholder("Type a command…")
                        .text_size(px(12.0)),
                )
                .child({
                    let mut list = view().flex_1().min_h_0().flex_col().p_1();
                    for command in commands {
                        let label = command.label;
                        let hint = command.hint;
                        let action = command.action.clone();
                        let close = close.clone();
                        let matches = move || {
                            let q = query.get().to_lowercase();
                            q.is_empty()
                                || label.to_lowercase().contains(&q)
                                || hint.to_lowercase().contains(&q)
                        };
                        list = list.child(
                            view()
                                .h(move || if matches() { px(30.0) } else { px(0.0) })
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .px_2()
                                .rounded(px(5.0))
                                .on_press(move || {
                                    action();
                                    close();
                                })
                                .child(
                                    text(label)
                                        .text_size(px(11.0))
                                        .text_color(move || design::text()),
                                )
                                .child(view().flex_1())
                                .child(
                                    text(hint)
                                        .text_size(px(9.0))
                                        .text_color(move || design::text3()),
                                ),
                        );
                    }
                    list
                }),
        )
}
