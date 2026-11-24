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
    let header = view()
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
    return crate::shared::diff::diff_panel(
        &cmd.diff.state,
        &cmd.diff,
        language,
        patch.clone(),
        patch,
        Some("history.diffEmpty"),
        None,
        Some({
            let cmd = cmd.clone();
            std::sync::Arc::new(move |ignore| cmd.reload_blame_diff(ignore))
        }),
        None,
    );
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
