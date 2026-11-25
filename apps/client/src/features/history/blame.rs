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

const BLAME_META_W: f32 = 220.0;
const BLAME_SHA_W: f32 = 56.0;

pub(crate) fn blame_view(
    language: Signal<SharedString>,
    tab: Tab,
    cmd: HistoryCommands,
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
        "event=blame_view.render revision={} extension={} mode={} lines={} history_entries={} patch_bytes={} patch_files={} patch_hunks={} blame_source=diffview",
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
    let additions = tab
        .blame_patch
        .files
        .iter()
        .map(|file| file.additions)
        .sum::<u32>();
    let deletions = tab
        .blame_patch
        .files
        .iter()
        .map(|file| file.deletions)
        .sum::<u32>();
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
                .number_of_lines(1)
                .text_overflow(nsh_core::EllipsizeMode::Head),
        )
        .child({
            let revision = if tab.blame_revision == "HEAD" {
                tab.head_oid
                    .clone()
                    .unwrap_or_else(|| tab.blame_revision.clone())
            } else {
                tab.blame_revision.clone()
            };
            let label = revision_label(revision.as_str());
            let copy_notice = cmd.session.notice;
            let tooltip = i18n::t(language.get().as_str(), "history.copySha");
            view()
                .flex_shrink_0()
                .tooltip(ui::button::text_tooltip(tooltip))
                .child(ui::button::button(
                    move || label.clone(),
                    ui::button::ButtonVariant::Ghost,
                    ui::button::ButtonSize::Small,
                    false,
                    move || crate::shared::os::copy_text(copy_notice, language, revision.clone()),
                ))
        })
        .child(dynamic(move || -> AnyElement {
            let language_now = language.get();
            let mut summary = view()
                .flex_row()
                .flex_shrink_0()
                .items_center()
                .gap(px(ui::GAP_SM));
            if selected_mode == "diff" {
                summary = summary
                    .child(
                        text(format!("+{additions}"))
                            .font_family(ui::MONO)
                            .text_size(px(ui::META))
                            .text_color(move || design::add_fg()),
                    )
                    .child(
                        text(format!("−{deletions}"))
                            .font_family(ui::MONO)
                            .text_size(px(ui::META))
                            .text_color(move || design::del_fg()),
                    );
            }
            return summary
                .child(
                    text(i18n::with_arguments(
                        language_now.as_str(),
                        "blame.lines",
                        &[("count", line_count.to_string())],
                    ))
                    .text_size(px(ui::META))
                    .text_color(move || design::text3()),
                )
                .into_element();
        }))
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

fn blame_lines(cmd: &HistoryCommands, tab: &Tab) -> AnyElement {
    let source = tab
        .blame
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let blame_rows = std::sync::Arc::new(
        tab.blame
            .iter()
            .map(|line| (line.short.clone(), line.author.clone()))
            .collect::<Vec<_>>(),
    );
    let attributions = std::sync::Arc::new(
        tab.blame
            .iter()
            .map(|line| (line.sha.clone(), line.line))
            .collect::<Vec<_>>(),
    );
    let selected_line = tab.blame_selected_line;
    let handle = crate::ui::diff_surface::DiffHandle::new();
    let metadata_ref = element_ref();
    let last_synced_scroll = std::rc::Rc::new(std::cell::Cell::new(0.0_f32));
    let sidebar_rows = blame_rows.clone();
    let sidebar_scroll = last_synced_scroll.clone();
    let sidebar_handle = handle.clone();
    let sidebar = virtual_list(blame_rows.len(), move |at| {
        let (sha, author) = &sidebar_rows[at.index()];
        let selected = selected_line == Some(at.index() as u32 + 1);
        return view()
            .w_full()
            .h(px(ui::CODE_LINE))
            .flex_shrink_0()
            .px(px(ui::GAP_SM))
            .flex_row()
            .items_center()
            .gap(px(ui::GAP_SM))
            .bg(move || {
                if selected {
                    design::line_soft()
                } else {
                    design::panel()
                }
            })
            .child(
                text(sha.clone())
                    .w(px(BLAME_SHA_W))
                    .font_family(ui::MONO)
                    .text_size(px(ui::META))
                    .text_color(move || design::text3()),
            )
            .child(
                text(author.clone())
                    .flex_1()
                    .min_w_0()
                    .text_size(px(ui::META))
                    .text_color(move || design::text2())
                    .number_of_lines(1)
                    .text_overflow(nsh_core::EllipsizeMode::Tail),
            );
    })
    .estimated_item_size(Points(ui::CODE_LINE))
    .scrollbar(false)
    .element_ref(&metadata_ref)
    .on_scroll(move |event: &ScrollEvent| {
        let offset = event.offset.y;
        if (offset.0 - sidebar_scroll.get()).abs() <= 0.5 {
            return;
        }
        sidebar_scroll.set(offset.0);
        sidebar_handle.set_scroll_offset(offset);
    })
    .w(px(BLAME_META_W))
    .h_full()
    .flex_shrink_0()
    .border_r_1()
    .border_color(move || design::line())
    .bg(move || design::panel());
    let code_scroll = last_synced_scroll.clone();
    let code_metadata_ref = metadata_ref;
    let code = crate::ui::diff_surface::source_file(
        SharedString::from(source),
        SharedString::from(tab.blame_path.clone()),
        selected_line.map(|line| line.saturating_sub(1)),
        &handle,
        {
            let cmd = cmd.clone();
            move |index| {
                if let Some((sha, line)) = attributions.get(index as usize) {
                    cmd.focus_blame_commit(sha.clone(), *line);
                }
            }
        },
        move |offset| {
            if (offset.0 - code_scroll.get()).abs() <= 0.5 {
                return;
            }
            code_scroll.set(offset.0);
            let mut point = Point::default();
            point.y = offset;
            code_metadata_ref.scroll_to(point, false);
        },
    );
    return view()
        .w_full()
        .h_full()
        .min_w_0()
        .min_h_0()
        .flex_row()
        .child(sidebar)
        .child(view().flex_1().min_w_0().h_full().child(code))
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
