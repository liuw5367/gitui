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

fn command_matches(command: &PaletteCommand, query: &str) -> bool {
    let q = query.to_lowercase();
    q.is_empty()
        || command.label.to_lowercase().contains(&q)
        || command.hint.to_lowercase().contains(&q)
}

pub(crate) fn palette_panel(
    open: Signal<bool>,
    query: Signal<String>,
    commands: Vec<PaletteCommand>,
    close: Arc<dyn Fn()>,
) -> impl IntoElement {
    let selected = signal(0usize);
    let key_commands = commands.clone();
    let key_query = query;
    let key_open = open;
    let key_selected = selected;
    let key_close = close.clone();
    on_key("down", {
        let commands = key_commands.clone();
        move |_| {
            if !key_open.get() {
                return;
            }
            let q = key_query.get();
            let len = commands.len();
            if len == 0 {
                return;
            }
            let mut next = (key_selected.get() + 1) % len;
            for _ in 0..len {
                if command_matches(&commands[next], &q) {
                    key_selected.set(next);
                    return;
                }
                next = (next + 1) % len;
            }
        }
    });
    on_key("up", {
        let commands = key_commands.clone();
        move |_| {
            if !key_open.get() {
                return;
            }
            let q = key_query.get();
            let len = commands.len();
            if len == 0 {
                return;
            }
            let mut next = (key_selected.get() + len - 1) % len;
            for _ in 0..len {
                if command_matches(&commands[next], &q) {
                    key_selected.set(next);
                    return;
                }
                next = (next + len - 1) % len;
            }
        }
    });
    on_key("enter", {
        let commands = key_commands.clone();
        move |_| {
            if !key_open.get() {
                return;
            }
            let q = key_query.get();
            if let Some(command) = commands.get(key_selected.get()) {
                if command_matches(command, &q) {
                    (command.action)();
                    key_close();
                }
            }
        }
    });
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
                        .on_change(move |text: &str| {
                            query.set(text.to_string());
                            selected.set(0);
                        })
                        .placeholder("Type a command…")
                        .text_size(px(12.0)),
                )
                .child({
                    let mut list = view().flex_1().min_h_0().flex_col().p_1();
                    for (index, command) in commands.into_iter().enumerate() {
                        let label = command.label;
                        let hint = command.hint;
                        let action = command.action.clone();
                        let close = close.clone();
                        let matches = move || command_matches(&command, &query.get());
                        let click_action = action.clone();
                        let click_close = close.clone();
                        list = list.child(
                            view()
                                .h(move || if matches() { px(30.0) } else { px(0.0) })
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .px_2()
                                .rounded(px(5.0))
                                .bg(move || {
                                    if selected.get() == index {
                                        design::selected()
                                    } else {
                                        design::content()
                                    }
                                })
                                .on_press(move || {
                                    selected.set(index);
                                    click_action();
                                    click_close();
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
