//! S-6 harness: NSH + Rust only.
//!
//! Validates:
//! - three code editors side by side (Base / Result / Theirs)
//! - 10,000-line files, editable Result, save
//! - a long-running Rust task with progress and cancellation
//! - no TSX, no JS, no HTML UI.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use nsh_base::{editor, theme};
use nsh_core::prelude::*;

/// The app's Rust UI set. NSH's test/describe host requires this even for a
/// Rust-only app; it is Rust, not TSX.
#[native_module("gitui-s6/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}

/// Build a deterministic file for the harness.
fn make_text(lines: usize, variant: u8) -> SharedString {
    let mut text = String::with_capacity(lines * 48);
    for i in 0..lines {
        let marker = match (variant, i) {
            (1, 5000) => " /* ours */",
            (2, 5000) => " /* theirs */",
            _ => "",
        };
        text.push_str(&format!("fn line_{i:05}() -> u32 {{ {i} }}{marker}\n"));
    }
    SharedString::from(text)
}

fn header_button(
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

fn pane(
    title: &'static str,
    value: Signal<SharedString>,
    read_only: bool,
) -> impl IntoElement {
    view()
        .flex_1()
        .flex_col()
        .gap_2()
        .child(
            view()
                .h(px(24.0))
                .flex_row()
                .items_center()
                .gap_2()
                .child(text(title).text_size(px(11.0)).text_color(move || theme().foreground))
                .child(view().flex_1())
                .child(
                    text(if read_only { "read-only" } else { "editable" })
                        .text_size(px(9.0))
                        .text_color(move || theme().primary),
                ),
        )
        .child(
            editor()
                .value(move || Some(value.get()))
                .read_only(read_only)
                .language("rust")
                .font_size(12.0)
                .line_height(18.0)
                .flex_1(),
        )
}

fn file_row(label: &'static str, index: usize, selected: Signal<usize>) -> impl IntoElement {
    view()
        .h(px(28.0))
        .px_2()
        .rounded(px(5.0))
        .bg(move || {
            if selected.get() == index {
                theme().primary
            } else {
                theme().background
            }
        })
        .on_press(move || selected.set(index))
        .child(
            text(label)
                .text_size(px(11.0))
                .text_color(move || {
                    if selected.get() == index {
                        theme().primary_foreground
                    } else {
                        theme().foreground
                    }
                }),
        )
}

#[component]
pub fn App() -> impl IntoElement {
    let initial_lines = if std::env::var_os("S6_LOAD_10K").is_some() { 10_000 } else { 200 };
    let panes = std::env::var("S6_PANES").unwrap_or_else(|_| "3".to_owned());
    let pane_label = panes.clone();
    let base = signal(make_text(initial_lines, 0));
    let ours = signal(make_text(initial_lines, 1));
    let theirs = signal(make_text(initial_lines, 2));
    let result_text = signal(make_text(initial_lines, 1));
    let line_count = signal(initial_lines as u32);
    let file = signal(0_usize);
    let running = signal(false);
    let progress = signal(0_u32);
    let saved = signal(SharedString::from("Not saved"));
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_for_start = cancel_flag.clone();
    let cancel_for_cancel = cancel_flag.clone();
    let cancel_for_effect = cancel_flag.clone();

    // The task must be owned by an effect. A `spawn` created directly in an
    // on_press handler has no owner and does not run in NSH.
    effect(move || {
        if !running.get() {
            return;
        }
        let flag = cancel_for_effect.clone();
        let task = spawn(async move {
            for step in 0..=100_u32 {
                if flag.load(Ordering::SeqCst) {
                    saved.set(SharedString::from("Task cancelled"));
                    running.set(false);
                    return;
                }
                timer(Duration::from_millis(60)).await;
                progress.set(step);
            }
            saved.set(SharedString::from("Task done"));
            running.set(false);
        });
        cleanup(move || task.cancel());
    });

    let header = view()
        .h(px(46.0))
        .flex_row()
        .items_center()
        .gap_2()
        .px_3()
        .bg(move || theme().background)
        .child(
            text("gitui S-6 · NSH Rust harness")
                .text_size(px(13.0))
                .text_color(move || theme().foreground),
        )
        .child(
            text(move || {
                if running.get() {
                    format!("Task {}%", progress.get())
                } else {
                    "Idle".to_owned()
                }
            })
            .text_size(px(10.0))
            .text_color(move || theme().primary),
        )
        .child(view().flex_1())
        .child(header_button("Load 10k", false, move || {
            base.set(make_text(10_000, 0));
            ours.set(make_text(10_000, 1));
            theirs.set(make_text(10_000, 2));
            result_text.set(make_text(10_000, 1));
            line_count.set(10_000);
            saved.set(SharedString::from("10,000 lines loaded"));
        }))
        .child(header_button("Start task", false, move || {
            if running.get_untracked() {
                return;
            }
            cancel_for_start.store(false, Ordering::SeqCst);
            progress.set(0);
            saved.set(SharedString::from("Task started"));
            running.set(true);
        }))
        .child(header_button("Cancel", false, move || {
            cancel_for_cancel.store(true, Ordering::SeqCst);
            running.set(false);
            saved.set(SharedString::from("Task cancelled"));
        }))
        .child(header_button("Save result", true, move || {
            match std::fs::write("/tmp/gitui-s6-result.txt", result_text.get().to_string()) {
                Ok(()) => saved.set(SharedString::from("Saved /tmp/gitui-s6-result.txt")),
                Err(error) => saved.set(SharedString::from(format!("Save failed: {error}"))),
            }
        }));

    let sidebar = view()
        .w(px(200.0))
        .flex_col()
        .gap_2()
        .p_2()
        .child(
            text("Conflicted files")
                .text_size(px(10.0))
                .text_color(move || theme().foreground),
        )
        .child(file_row("src/merge.ts", 0, file))
        .child(file_row("src/ui/CommitPanel.tsx", 1, file))
        .child(file_row("src/git/index.rs", 2, file))
        .child(view().flex_1())
        .child(
            text(move || format!("10k lines · {pane_label} panes"))
                .text_size(px(9.0))
                .text_color(move || theme().primary),
        );

    let result_pane = view()
        .flex_1()
        .flex_col()
        .gap_2()
        .child(
            view()
                .h(px(24.0))
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    text("Result")
                        .text_size(px(11.0))
                        .text_color(move || theme().foreground),
                )
                .child(view().flex_1())
                .child(
                    text("editable · saved to /tmp")
                        .text_size(px(9.0))
                        .text_color(move || theme().primary),
                ),
        )
        .child(
            editor()
                .value(move || Some(result_text.get()))
                .language("rust")
                .font_size(12.0)
                .line_height(18.0)
                .on_change(move |value: &SharedString| {
                    result_text.set(value.clone());
                    line_count.set(value.to_string().lines().count() as u32);
                    saved.set(SharedString::from("Edited"));
                })
                .flex_1(),
        );

    let mut main = view()
        .flex_1()
        .flex_row()
        .gap_2()
        .p_2()
        .bg(move || theme().background)
        .child(sidebar);
    if panes != "1" {
        main = main.child(pane("Base", base, true));
    }
    main = main.child(result_pane);
    if panes == "3" {
        main = main.child(pane("Theirs", theirs, true));
    }

    let status = view()
        .h(px(26.0))
        .flex_row()
        .items_center()
        .gap_3()
        .px_3()
        .bg(move || theme().background)
        .child(
            text(move || format!("{} lines", line_count.get()))
                .text_size(px(10.0))
                .text_color(move || theme().foreground),
        )
        .child(
            text(move || {
                if running.get() {
                    format!("task {}%", progress.get())
                } else {
                    "idle".to_owned()
                }
            })
            .text_size(px(10.0))
            .text_color(move || theme().primary),
        )
        .child(
            text(move || saved.get())
                .text_size(px(10.0))
                .text_color(move || theme().foreground),
        )
        .child(view().flex_1())
        .child(
            text("NSH + Rust only · no TSX")
                .text_size(px(9.0))
                .text_color(move || theme().foreground),
        );

    view()
        .flex_1()
        .bg(move || theme().background)
        .text_color(move || theme().foreground)
        .child(header)
        .child(main)
        .child(status)
}
