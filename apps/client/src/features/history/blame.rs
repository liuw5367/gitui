//! 单文件 Blame / File History（设计板 19）。
//!
//! Blame 占用完整中栏，文件版本历史放在右侧检查器。选历史行切换当前 blame revision；
//! 选源码行只聚焦右侧对应提交，保持当前文件版本不变。

use super::commands::HistoryCommands;
use crate::app::session::Tab;
use crate::design::{self, metrics};
use crate::i18n;
use crate::ui;
use nsh_core::prelude::*;

pub(crate) fn blame_view(
    language: Signal<SharedString>,
    tab: Tab,
    cmd: HistoryCommands,
    settings_state: Signal<store::Settings>,
) -> impl IntoElement {
    let selected_mode = if tab.blame_show_diff { "diff" } else { "blame" };
    let extension = std::path::Path::new(tab.blame_path.as_str())
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("");
    let patch_hunks = tab
        .blame_patch
        .lines
        .iter()
        .filter(|line| line.kind == domain::diff::DiffLineKind::Hunk)
        .count();
    log::debug!(
        target: "gitui::history",
        "event=blame_view.render revision={} extension={} mode={} lines={} history_entries={} patch_bytes={} patch_files={} patch_hunks={} blame_rows_highlight=not_applied",
        tab.blame_revision.chars().take(7).collect::<String>(),
        extension,
        selected_mode,
        tab.blame.len(),
        tab.file_history.len(),
        tab.blame_patch.raw.len(),
        tab.blame_patch.files.len(),
        patch_hunks,
    );
    let back_key = if tab.blame_return_mode == Some(crate::Mode::Changes) {
        "blame.backToChanges"
    } else {
        "history.backToList"
    };
    let line_count = tab.blame.len();
    let last_change = tab
        .file_history
        .first()
        .and_then(|entry| entry.commit.date.parse::<i64>().ok());
    let segment_cmd = cmd.clone();
    let mut header = view()
        .w_full()
        .h(px(metrics::DIFF_RETURN_H))
        .flex_shrink_0()
        .px(px(ui::GAP_LG))
        .flex_row()
        .items_center()
        .gap(px(ui::GAP))
        .border_b_1()
        .border_color(move || design::line())
        .bg(move || design::panel())
        .child(ui::button::button_icon(
            move || i18n::t(language.get().as_str(), back_key),
            "chev-left",
            ui::button::ButtonVariant::Secondary,
            ui::button::ButtonSize::Small,
            false,
            {
                let cmd = cmd.clone();
                move || cmd.return_from_blame()
            },
        ))
        .child(
            text(tab.blame_path.clone())
                .flex_1()
                .min_w_0()
                .font_family(ui::MONO)
                .text_size(px(ui::CODE))
                .text_color(move || design::text())
                .text_overflow(nsh_core::EllipsizeMode::Head),
        )
        .child(ui::badge::badge(
            {
                let revision = revision_label(tab.blame_revision.as_str());
                move || revision.clone()
            },
            ui::badge::Tone::Neutral,
        ))
        .child(dynamic(move || -> AnyElement {
            let cmd = segment_cmd.clone();
            ui::segmented::segmented(
                vec![
                    ui::segmented::Segment {
                        id: "blame",
                        label: Box::new({
                            let language = language;
                            move || i18n::t(language.get().as_str(), "blame.blame")
                        }),
                    },
                    ui::segmented::Segment {
                        id: "diff",
                        label: Box::new({
                            let language = language;
                            move || i18n::t(language.get().as_str(), "blame.diff")
                        }),
                    },
                ],
                selected_mode,
                {
                    let cmd = cmd.clone();
                    move |id| cmd.toggle_blame_diff(id == "diff")
                },
            )
            .into_element()
        }))
        .child(
            text(move || {
                let language_now = language.get();
                if let Some(timestamp) = last_change {
                    i18n::with_arguments(
                        language_now.as_str(),
                        "blame.lastChange",
                        &[
                            ("count", line_count.to_string()),
                            (
                                "date",
                                crate::dates::format_reactive(
                                    settings_state,
                                    language_now.as_str(),
                                    timestamp,
                                    store::now_unix(),
                                ),
                            ),
                        ],
                    )
                } else {
                    i18n::with_arguments(
                        language_now.as_str(),
                        "blame.lines",
                        &[("count", line_count.to_string())],
                    )
                }
            })
            .text_size(px(ui::META))
            .text_color(move || design::text3()),
        );

    let controls_cmd = cmd.clone();
    let raw_patch = tab.blame_patch.raw.clone();
    let show_diff = tab.blame_show_diff;
    header = header.child(dynamic(move || -> AnyElement {
        if !show_diff {
            return view().into_element();
        }
        let options = controls_cmd.diff.state.options.get();
        let split = controls_cmd.diff.state.split.get();
        diff_controls(&controls_cmd, language, options, split, raw_patch.clone())
    }));

    let body_cmd = cmd.clone();
    let body_tab = tab.clone();
    let body = dynamic(move || -> AnyElement {
        if body_tab.blame_error.is_some() {
            return centered_message(i18n::t(language.get().as_str(), "blame.error"));
        }
        if body_tab.blame_show_diff {
            return blame_diff(language, &body_cmd, &body_tab);
        }
        if body_tab.blame.is_empty() {
            return centered_message(i18n::t(language.get().as_str(), "blame.empty"));
        }
        return blame_lines(&body_cmd, &body_tab);
    });

    return view()
        .w_full()
        .h_full()
        .min_w_0()
        .min_h_0()
        .flex_col()
        .bg(move || design::content())
        .child(header)
        .child(view().w_full().flex_1().min_h_0().child(body));
}

fn revision_label(revision: &str) -> String {
    if revision == "HEAD" {
        return "HEAD".to_string();
    }
    return revision.chars().take(7).collect();
}

fn diff_controls(
    cmd: &HistoryCommands,
    language: Signal<SharedString>,
    options: crate::shared::diff::DiffOptions,
    split: bool,
    patch: String,
) -> AnyElement {
    let cmd = cmd.clone();
    let layout_cmd = cmd.clone();
    let mut controls = view()
        .h_full()
        .flex_row()
        .items_center()
        .gap(px(ui::GAP_SM))
        .child(toolbar_separator())
        .child(ui::segmented::segmented(
            vec![
                ui::segmented::Segment {
                    id: "split",
                    label: Box::new({
                        let language = language;
                        move || i18n::t(language.get().as_str(), "history.split")
                    }),
                },
                ui::segmented::Segment {
                    id: "unified",
                    label: Box::new({
                        let language = language;
                        move || i18n::t(language.get().as_str(), "history.unified")
                    }),
                },
            ],
            if split { "split" } else { "unified" },
            move |id| layout_cmd.diff.set_diff_split(id == "split"),
        ));
    let fold_active = options.fold;
    controls = controls
        .child(crate::ui::button::icon_button(
            if fold_active { "fold" } else { "unfold" },
            {
                let language = language;
                move || {
                    let key = if fold_active {
                        "diff.unfold"
                    } else {
                        "diff.fold"
                    };
                    i18n::t(language.get().as_str(), key)
                }
            },
            ui::button::ButtonSize::Small,
            false,
            {
                let cmd = cmd.clone();
                move || cmd.diff.toggle_diff_option("fold")
            },
        ))
        .child(crate::ui::button::icon_button(
            "wrap",
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "diff.wrap")
            },
            ui::button::ButtonSize::Small,
            false,
            {
                let cmd = cmd.clone();
                move || cmd.diff.toggle_diff_option("wrap")
            },
        ))
        .child(crate::ui::button::icon_button(
            "copy",
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "diff.copy")
            },
            ui::button::ButtonSize::Small,
            patch.trim().is_empty(),
            {
                let notice = cmd.notice;
                move || crate::shared::os::copy_text(notice, language, patch.clone())
            },
        ));
    return controls.into_element();
}

fn blame_lines(cmd: &HistoryCommands, tab: &Tab) -> AnyElement {
    let rows = tab
        .blame
        .iter()
        .map(|line| {
            let sha = line.sha.clone();
            ui::row::blame_line(
                line.short.clone(),
                line.author.clone(),
                line.line,
                line.text.clone(),
                tab.blame_selected_line == Some(line.line),
                {
                    let cmd = cmd.clone();
                    let line = line.line;
                    move || cmd.focus_blame_commit(sha.clone(), line)
                },
            )
            .into_element()
        })
        .collect::<Vec<_>>();
    return scroll()
        .w_full()
        .h_full()
        .child(view().w_full().flex_col().children(rows))
        .into_element();
}

fn blame_diff(language: Signal<SharedString>, cmd: &HistoryCommands, tab: &Tab) -> AnyElement {
    let patch = tab.blame_patch.clone();
    if patch.raw.trim().is_empty() {
        return centered_message(i18n::t(language.get().as_str(), "history.diffEmpty"))
            .into_element();
    }
    if !patch
        .lines
        .iter()
        .any(|line| line.kind == domain::diff::DiffLineKind::Hunk)
    {
        return centered_message(i18n::t(language.get().as_str(), "diff.noTextChanges"))
            .into_element();
    }
    let handle = crate::ui::diff_surface::DiffHandle::new();
    let options = cmd.diff.state.options.get();
    let layout = if cmd.diff.state.split.get() {
        crate::ui::diff_surface::Layout::Split
    } else {
        crate::ui::diff_surface::Layout::Unified
    };
    let notice = cmd.notice;
    let on_hunk_copy = crate::shared::diff::copy_hunk_handler(patch.raw.as_str(), notice, language);
    return crate::ui::diff_surface::diff_surface(
        SharedString::from(patch.raw),
        layout,
        options.wrap,
        !options.fold,
        &handle,
        language,
        on_hunk_copy,
    )
    .into_element();
}

fn centered_message(message: String) -> AnyElement {
    return view()
        .w_full()
        .h_full()
        .items_center()
        .justify_center()
        .px(px(ui::GAP_LG))
        .child(
            text(message)
                .text_size(px(ui::SECONDARY))
                .text_color(move || design::text3()),
        )
        .into_element();
}

pub(crate) mod panel;
pub(crate) use panel::blame_history_panel;

fn toolbar_separator() -> impl IntoElement {
    return view()
        .w(px(1.0))
        .h(px(ui::CONTROL_SMALL))
        .bg(move || design::line_soft());
}
