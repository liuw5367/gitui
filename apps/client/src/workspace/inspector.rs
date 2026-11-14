//! 提交文件面板（设计板 18 右侧 `.insp`）：列出选中提交改动的文件，点一行看该文件
//! 在提交里的补丁。
//!
//! 宽度 320；顶部是提交摘要与元信息，中间是文件列表，底部是复制路径与在编辑器打开
//! （P2 只有复制路径可用）。

use crate::design::{self, metrics};
use crate::i18n;
use crate::model::Tab;
use crate::widgets;
use crate::Actions;
use nsh_core::prelude::*;

/// 面板宽度（设计板 `.insp`）。
pub(crate) const WIDTH: f32 = 320.0;

pub(crate) fn inspector(
    language: Signal<SharedString>,
    tab: Tab,
    actions: Actions,
) -> impl IntoElement {
    let select_file = actions.select_file.clone();
    let commit = tab.selected_commit().cloned();
    let files = tab.files.clone();
    let selected = tab.selected_file;
    let now = store::now_unix();
    let summary = commit
        .as_ref()
        .map(|commit| commit.summary.clone())
        .unwrap_or_default();
    let author = commit
        .as_ref()
        .map(|commit| commit.author.clone())
        .unwrap_or_default();
    let short = commit
        .as_ref()
        .map(|commit| commit.short.clone())
        .unwrap_or_default();
    let timestamp = commit
        .as_ref()
        .and_then(|commit| commit.date.parse::<i64>().ok())
        .unwrap_or(0);
    return view()
        .w(px(WIDTH))
        .h_full()
        .flex_shrink_0()
        .flex_col()
        .bg(move || design::panel())
        .border_l_1()
        .border_color(move || design::line())
        .child(
            view()
                .w_full()
                .px(px(12.0))
                .py(px(10.0))
                .flex_col()
                .gap(px(3.0))
                .border_b_1()
                .border_color(move || design::line())
                .child(
                    text(summary)
                        .text_size(px(13.0))
                        .font_weight(FontWeight(600))
                        .text_color(move || design::text())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .child(
                    text(format!(
                        "{} · {} · {}",
                        author,
                        short,
                        i18n::relative_time(language.get_untracked().as_str(), timestamp, now)
                    ))
                    .text_size(px(9.5))
                    .font_family("SF Mono")
                    .text_color(move || design::text3()),
                )
                .child(
                    view()
                        .h(px(18.0))
                        .px(px(5.0))
                        .items_center()
                        .rounded(px(3.0))
                        .bg(move || design::accent_soft())
                        .child(
                            text(move || {
                                i18n::with_arguments(
                                    language.get().as_str(),
                                    "files.atCommit",
                                    &[("short", short.clone())],
                                )
                            })
                            .text_size(px(9.5))
                            .text_color(move || design::accent()),
                        ),
                ),
        )
        .child(
            view()
                .w_full()
                .h(px(26.0))
                .flex_row()
                .items_center()
                .gap(px(10.0))
                .px(px(12.0))
                .border_b_1()
                .border_color(move || design::line_soft())
                .child(
                    text(move || i18n::t(language.get().as_str(), "files.changes"))
                        .text_size(px(10.5))
                        .font_weight(FontWeight(600))
                        .text_color(move || design::text()),
                )
                .child(
                    text(move || i18n::t(language.get().as_str(), "files.title"))
                        .text_size(px(10.5))
                        .text_color(move || design::text3()),
                )
                .child(
                    text(move || i18n::t(language.get().as_str(), "files.info"))
                        .text_size(px(10.5))
                        .text_color(move || design::text3()),
                ),
        )
        .child(
            view().w_full().flex_1().min_h_0().child(
                scroll().w_full().h_full().child(
                    view()
                        .w_full()
                        .py(px(4.0))
                        .px(px(6.0))
                        .flex_col()
                        .child(if files.is_empty() {
                            view()
                                .w_full()
                                .h(px(40.0))
                                .items_center()
                                .justify_center()
                                .child(
                                    text(move || {
                                        i18n::t(language.get().as_str(), "files.empty")
                                    })
                                    .text_size(px(10.5))
                                    .text_color(move || design::text3()),
                                )
                                .into_element()
                        } else {
                            let mut rows: Vec<AnyElement> = Vec::new();
                            for (index, file) in files.iter().enumerate() {
                                let select_file = select_file.clone();
                                let path = file.path.clone();
                                let additions = file.additions;
                                let deletions = file.deletions;
                                let is_selected = selected == Some(index);
                                rows.push(
                                    view()
                                        .w_full()
                                        .h(px(24.0))
                                        .px(px(6.0))
                                        .flex_row()
                                        .items_center()
                                        .gap(px(6.0))
                                        .rounded(px(4.0))
                                        .bg(move || {
                                            if is_selected {
                                                design::selected()
                                            } else {
                                                AdaptiveColor::TRANSPARENT
                                            }
                                        })
                                        .hover(|style| style.bg(move || design::hover()))
                                        .on_press(move || select_file(index))
                                        .child(widgets::icon("file", 11.0, design::text3))
                                        .child(
                                            text(path)
                                                .flex_1()
                                                .text_size(px(10.5))
                                                .font_family("SF Mono")
                                                .text_color(move || design::text())
                                                .text_overflow(nsh_core::EllipsizeMode::Tail),
                                        )
                                        .child(
                                            text(format!("+{}", additions))
                                                .text_size(px(9.5))
                                                .text_color(move || design::add_fg()),
                                        )
                                        .child(
                                            text(format!("−{}", deletions))
                                                .text_size(px(9.5))
                                                .text_color(move || design::del_fg()),
                                        )
                                        .into_element(),
                                );
                            }
                            view().w_full().flex_col().child(rows).into_element()
                        }),
                ),
            ),
        )
        .child(
            view()
                .w_full()
                .h(px(34.0))
                .px(px(12.0))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .border_t_1()
                .border_color(move || design::line())
                .child(
                    view()
                        .h(px(metrics::BUTTON_SMALL_H))
                        .px(px(8.0))
                        .items_center()
                        .rounded(px(5.0))
                        .border_1()
                        .border_color(move || design::btn_border())
                        .bg(move || design::btn())
                        .opacity(0.45)
                        .child(
                            text(move || i18n::t(language.get().as_str(), "files.copyPath"))
                                .text_size(px(11.0))
                                .text_color(move || design::text()),
                        ),
                )
                .child(view().flex_1())
                .child(
                    text(move || i18n::t(language.get().as_str(), "files.openInEditor"))
                        .text_size(px(10.0))
                        .text_color(move || design::text3()),
                ),
        );
}
