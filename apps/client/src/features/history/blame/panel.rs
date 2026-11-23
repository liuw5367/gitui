//! Board 19 右侧文件历史检查器与路径动作。

use crate::app::session::Tab;
use crate::design;
use crate::features::history::commands::HistoryCommands;
use crate::i18n;
use crate::ui;
use nsh_core::prelude::*;

pub(crate) fn blame_history_panel(
    language: Signal<SharedString>,
    tab: Tab,
    cmd: HistoryCommands,
    settings_state: Signal<store::Settings>,
) -> impl IntoElement {
    let now = store::now_unix();
    let path = tab.blame_path.clone();
    let root = tab.path.clone();
    let copy_path = format!("{}/{}", root.trim_end_matches('/'), path);
    let line_count = tab.file_history.len();
    let changed_count = if tab.blame_revision == "HEAD" {
        tab.changes.len()
    } else {
        tab.files.len()
    };
    let selected_sha = tab.blame_focus_sha.clone();
    let mut rows = Vec::<AnyElement>::new();
    for entry in tab.file_history.iter().take(50) {
        let sha = entry.commit.sha.clone();
        let summary = entry.commit.summary.clone();
        let short = entry.commit.short.clone();
        let author = entry.commit.author.clone();
        let timestamp = entry.commit.date.parse::<i64>().unwrap_or(0);
        rows.push(
            ui::row::row_sized(
                move || summary.clone(),
                Some((
                    Box::new(move || {
                        format!(
                            "{short} · {author} · {}",
                            crate::dates::format_reactive(
                                settings_state,
                                language.get().as_str(),
                                timestamp,
                                now,
                            )
                        )
                    }),
                    true,
                )),
                None,
                vec![],
                selected_sha.as_deref() == Some(entry.commit.sha.as_str()),
                false,
                40.0,
                {
                    let cmd = cmd.clone();
                    move || cmd.select_blame_history(sha.clone())
                },
            )
            .into_element(),
        );
    }
    if rows.is_empty() {
        rows.push(
            view()
                .w_full()
                .flex_1()
                .items_center()
                .justify_center()
                .px(px(ui::GAP))
                .child(
                    text(move || i18n::t(language.get().as_str(), "blame.noHistory"))
                        .text_size(px(ui::SECONDARY))
                        .text_color(move || design::text3()),
                )
                .into_element(),
        );
    }
    let mode_cmd = cmd.clone();
    let notice = cmd.notice;

    return view()
        .w_full()
        .h_full()
        .min_w_0()
        .flex_col()
        .bg(move || design::panel())
        .child(
            view()
                .w_full()
                .h(px(ui::ROW + ui::GAP_SM * 2.0))
                .px(px(ui::GAP_LG))
                .py(px(ui::GAP_SM))
                .flex_row()
                .items_center()
                .flex_shrink_0()
                .child(ui::tabs::tabs(
                    vec![
                        ui::tabs::TabItem {
                            id: "changes",
                            label: Box::new({
                                let language = language;
                                move || {
                                    i18n::with_arguments(
                                        language.get().as_str(),
                                        "blame.changesCount",
                                        &[("count", changed_count.to_string())],
                                    )
                                }
                            }),
                        },
                        ui::tabs::TabItem {
                            id: "history",
                            label: Box::new({
                                let language = language;
                                move || i18n::t(language.get().as_str(), "shell.history")
                            }),
                        },
                    ],
                    "history",
                    move |id| {
                        if id == "changes" {
                            mode_cmd.return_from_blame();
                        }
                    },
                )),
        )
        .child(
            view()
                .w_full()
                .h(px(ui::ROW_COMPACT))
                .px(px(ui::GAP))
                .flex_shrink_0()
                .flex_row()
                .items_center()
                .gap(px(ui::GAP_SM))
                .border_b_1()
                .border_color(move || design::line_soft())
                .child(ui::icon::icon_view(
                    "file",
                    ui::icon::IconSize::Compact,
                    design::text3,
                ))
                .child(
                    text(path)
                        .flex_1()
                        .min_w_0()
                        .font_family(ui::MONO)
                        .text_size(px(ui::META))
                        .text_color(move || design::text2())
                        .text_overflow(nsh_core::EllipsizeMode::Head),
                )
                .child(ui::badge::count(move || {
                    i18n::with_arguments(
                        language.get().as_str(),
                        "blame.commits",
                        &[("count", line_count.to_string())],
                    )
                })),
        )
        .child(
            view().w_full().flex_1().min_h_0().child(
                scroll().w_full().h_full().child(
                    view()
                        .w_full()
                        .px(px(ui::GAP))
                        .py(px(ui::GAP_XS))
                        .flex_col()
                        .gap(px(ui::GAP_XS))
                        .children(rows),
                ),
            ),
        )
        .child(
            view()
                .w_full()
                .h(px(ui::CONTROL_LARGE + ui::GAP))
                .px(px(ui::GAP))
                .flex_shrink_0()
                .flex_row()
                .items_center()
                .gap(px(ui::GAP_SM))
                .border_t_1()
                .border_color(move || design::line())
                .child(ui::button::button(
                    {
                        let language = language;
                        move || i18n::t(language.get().as_str(), "files.openInEditor")
                    },
                    ui::button::ButtonVariant::Secondary,
                    ui::button::ButtonSize::Small,
                    path_is_empty(&copy_path),
                    {
                        let copy_path = copy_path.clone();
                        move || crate::shared::os::open_editor(notice, language, copy_path.clone())
                    },
                ))
                .child(view().flex_1())
                .child(ui::button::button(
                    {
                        let language = language;
                        move || i18n::t(language.get().as_str(), "files.copyPath")
                    },
                    ui::button::ButtonVariant::Secondary,
                    ui::button::ButtonSize::Small,
                    path_is_empty(&copy_path),
                    move || crate::shared::os::copy_text(notice, language, copy_path.clone()),
                )),
        )
        .into_element();
}

fn path_is_empty(path: &str) -> bool {
    path.ends_with('/')
}
