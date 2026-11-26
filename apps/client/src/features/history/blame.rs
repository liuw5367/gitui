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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BlameSnapshot {
    tab_id: u64,
    path: String,
    revision: String,
    head_oid: Option<String>,
    return_mode: Option<crate::Mode>,
    show_diff: bool,
    has_error: bool,
    lines: Vec<git::BlameLine>,
    patch: Option<domain::diff::Patch>,
}

pub(crate) fn snapshot(tab: &Tab) -> BlameSnapshot {
    BlameSnapshot {
        tab_id: tab.id,
        path: tab.blame_path.clone(),
        revision: tab.blame_revision.clone(),
        head_oid: (tab.blame_revision == "HEAD")
            .then(|| tab.head_oid.clone())
            .flatten(),
        return_mode: tab.blame_return_mode,
        show_diff: tab.blame_show_diff,
        has_error: tab.blame_error.is_some(),
        lines: tab.blame.clone(),
        patch: tab.blame_show_diff.then(|| tab.blame_patch.clone()),
    }
}

pub(crate) fn blame_view(
    language: Signal<SharedString>,
    tab: BlameSnapshot,
    selected_line: Memo<Option<u32>>,
    cmd: HistoryCommands,
) -> impl IntoElement {
    let selected_mode = if tab.show_diff { "diff" } else { "blame" };
    let extension = std::path::Path::new(tab.path.as_str())
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("");
    log::debug!(
        target: "gitui::history",
        "event=blame_view.render revision={} extension={} mode={} lines={} patch_bytes={} blame_source=diffview",
        tab.revision.chars().take(7).collect::<String>(),
        extension,
        selected_mode,
        tab.lines.len(),
        tab.patch.as_ref().map_or(0, |patch| patch.raw.len()),
    );
    let back_key = if tab.return_mode == Some(crate::Mode::Changes) {
        "blame.backToChanges"
    } else {
        "history.backToList"
    };
    let line_count = tab.lines.len();
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
            text(tab.path.clone())
                .flex_1()
                .min_w_0()
                .font_family(ui::MONO)
                .text_size(px(ui::CODE))
                .text_color(move || design::text())
                .number_of_lines(1)
                .text_overflow(nsh_core::EllipsizeMode::Head),
        )
        .child({
            let revision = if tab.revision == "HEAD" {
                tab.head_oid.clone().unwrap_or_else(|| tab.revision.clone())
            } else {
                tab.revision.clone()
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
        .child(
            text(move || {
                let language_now = language.get();
                i18n::with_arguments(
                    language_now.as_str(),
                    "blame.lines",
                    &[("count", line_count.to_string())],
                )
            })
            .flex_shrink_0()
            .text_size(px(ui::META))
            .text_color(move || design::text3()),
        )
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

    let body = if tab.has_error {
        dynamic(move || centered_message(i18n::t(language.get().as_str(), "blame.error")))
            .into_element()
    } else if let Some(patch) = tab.patch {
        blame_diff(language, &cmd, patch)
    } else if tab.lines.is_empty() {
        dynamic(move || centered_message(i18n::t(language.get().as_str(), "blame.empty")))
            .into_element()
    } else {
        blame_lines(&cmd, &tab, selected_line)
    };

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlameScrollSide {
    Metadata,
    Source,
}

#[derive(Default)]
struct BlameScroll {
    source: std::cell::Cell<Option<BlameScrollSide>>,
    offset: std::cell::Cell<Points>,
}

impl BlameScroll {
    fn follow(&self, side: BlameScrollSide, offset: Points) -> bool {
        let follows =
            self.source.get() == Some(side) && (offset - self.offset.get()).abs() > px(0.5);
        if follows {
            self.offset.set(offset);
        }
        log::trace!(target: "gitui::history::scroll",
            "blame side={side:?} source={:?} offset={offset:?} write={follows}", self.source.get());
        follows
    }
}

fn blame_lines(
    cmd: &HistoryCommands,
    tab: &BlameSnapshot,
    selected_line: Memo<Option<u32>>,
) -> AnyElement {
    let source = tab
        .lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let blame_rows = std::sync::Arc::new(
        tab.lines
            .iter()
            .map(|line| (line.short.clone(), line.author.clone()))
            .collect::<Vec<_>>(),
    );
    let attributions = std::sync::Arc::new(
        tab.lines
            .iter()
            .map(|line| (line.sha.clone(), line.line))
            .collect::<Vec<_>>(),
    );
    let handle = crate::ui::diff_surface::DiffHandle::new();
    let metadata_ref = element_ref();
    let scroll = std::rc::Rc::new(BlameScroll::default());
    let sidebar_rows = blame_rows.clone();
    let sidebar_scroll = scroll.clone();
    let sidebar_drag = scroll.clone();
    let sidebar_wheel = scroll.clone();
    let code_input = scroll.clone();
    let sidebar_handle = handle.clone();
    let sidebar = virtual_list(blame_rows.len(), move |at| {
        let (sha, author) = &sidebar_rows[at.index()];
        let line = at.index() as u32 + 1;
        return view()
            .w_full()
            .h(px(ui::CODE_LINE))
            .flex_shrink_0()
            .px(px(ui::GAP_SM))
            .flex_row()
            .items_center()
            .gap(px(ui::GAP_SM))
            .bg(move || {
                if selected_line.get() == Some(line) {
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
    .on_scroll_drag_start(move |_: &Point<Points>| {
        sidebar_drag.source.set(Some(BlameScrollSide::Metadata));
    })
    .on_wheel(move |event: &nsh_core::WheelEvent| {
        let delta = event.delta.points(px(1.0));
        if delta.y.abs() >= delta.x.abs() && delta.y != Points::ZERO {
            sidebar_wheel.source.set(Some(BlameScrollSide::Metadata));
        }
    })
    .on_scroll(move |event: &ScrollEvent| {
        let limit = (event.content_size.height - event.size.height).max(Points::ZERO);
        let offset = event.offset.y.clamp(Points::ZERO, limit);
        if sidebar_scroll.follow(BlameScrollSide::Metadata, offset) {
            sidebar_handle.set_scroll_offset(offset);
        }
    })
    .w(px(BLAME_META_W))
    .h_full()
    .flex_shrink_0()
    .border_r_1()
    .border_color(move || design::line())
    .bg(move || design::panel());
    let code_metadata_ref = metadata_ref;
    let code = crate::ui::diff_surface::source_file(
        SharedString::from(source),
        SharedString::from(tab.path.clone()),
        selected_line,
        &handle,
        {
            let cmd = cmd.clone();
            move |index| {
                if let Some((sha, line)) = attributions.get(index as usize) {
                    cmd.focus_blame_commit(sha.clone(), *line);
                }
            }
        },
        move || code_input.source.set(Some(BlameScrollSide::Source)),
        move |offset| {
            if scroll.follow(BlameScrollSide::Source, offset) {
                let mut point = Point::default();
                point.y = offset;
                code_metadata_ref.scroll_to(point, false);
            }
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

fn blame_diff(
    language: Signal<SharedString>,
    cmd: &HistoryCommands,
    patch: domain::diff::Patch,
) -> AnyElement {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blame_scroll_ignores_clamped_and_delayed_follower_reports() {
        use BlameScrollSide::{Metadata, Source};

        let scroll = BlameScroll::default();
        assert!(!scroll.follow(Source, px(100.0)));

        scroll.source.set(Some(Source));
        assert!(scroll.follow(Source, px(100.0)));
        assert!(!scroll.follow(Metadata, px(95.0)));
        assert!(!scroll.follow(Metadata, px(40.0)));

        scroll.source.set(Some(Metadata));
        assert!(scroll.follow(Metadata, px(30.0)));
        assert!(!scroll.follow(Source, px(100.0)));
        assert_eq!(scroll.offset.get(), px(30.0));
    }

    #[test]
    fn only_blame_content_changes_invalidate_the_center_view() {
        let mut tab = Tab {
            blame_revision: "HEAD".into(),
            blame_path: "src/lib.rs".into(),
            ..Tab::default()
        };
        let initial = snapshot(&tab);

        tab.blame_selected_line = Some(20);
        tab.blame_focus_sha = Some("selected-commit".into());
        tab.ahead = 3;
        assert_eq!(snapshot(&tab), initial);

        tab.head_oid = Some("new-head".into());
        let new_head = snapshot(&tab);
        assert_ne!(new_head, initial);

        tab.blame_show_diff = true;
        let diff = snapshot(&tab);
        assert_ne!(diff, new_head);
        tab.blame_patch.raw = "updated patch".into();
        assert_ne!(snapshot(&tab), diff);

        tab.blame_show_diff = false;
        tab.blame.push(git::BlameLine::default());
        assert_ne!(snapshot(&tab), new_head);
    }
}
