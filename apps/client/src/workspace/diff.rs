//! diff 视图：`.diffbar` 38 高，正文 SF Mono 11/19，行号与正负行配色。
//!
//! 补丁解析在 `domain::diff`；这里只负责排版与配色。

use crate::design::{self, metrics};
use crate::i18n;
use crate::model::Tab;
use crate::widgets;
use domain::diff::{DiffLine, DiffLineKind, Patch};
use nsh_core::prelude::*;

/// 行号栏宽度（旧、新各一列）。
const GUTTER_W: f32 = 34.0;

pub(crate) fn diff(language: Signal<SharedString>, tab: Tab) -> impl IntoElement {
    let patch = Patch::parse(tab.patch.as_str());
    let additions: u32 = patch.files.iter().map(|file| file.additions).sum();
    let deletions: u32 = patch.files.iter().map(|file| file.deletions).sum();
    let file_label = patch
        .files
        .first()
        .map(|file| file.path.clone())
        .unwrap_or_default();
    return view()
        .w_full()
        .flex_1()
        .min_h_0()
        .flex_col()
        .bg(move || design::content())
        .child(
            view()
                .w_full()
                .h(px(metrics::DIFFBAR_H))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .px(px(11.0))
                .border_b_1()
                .border_color(move || design::line())
                .child(widgets::icon("diff", 12.0, design::text3))
                .child(
                    text(file_label)
                        .text_size(px(11.0))
                        .font_family("SF Mono")
                        .text_color(move || design::text())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .child(
                    text(format!("+{} −{}", additions, deletions))
                        .text_size(px(9.5))
                        .font_family("SF Mono")
                        .text_color(move || design::text3()),
                )
                .child(view().flex_1())
                .child(mode_toggle(language))
                .child(widgets::icon("copy", 12.0, design::text3)),
        )
        .child(
            view().w_full().flex_1().min_h_0().child(
                scroll().w_full().h_full().child(
                    view()
                        .w_full()
                        .flex_col()
                        .child(dynamic(move || -> AnyElement {
                            if patch.lines.is_empty() {
                                return empty_state(language).into_element();
                            }
                            let rows: Vec<AnyElement> =
                                patch.lines.iter().map(diff_row).collect();
                            return view().w_full().flex_col().child(rows).into_element();
                        })),
                ),
            ),
        );
}

/// 空补丁提示（无改动或读取失败）。
fn empty_state(language: Signal<SharedString>) -> impl IntoElement {
    return view()
        .w_full()
        .h(px(60.0))
        .items_center()
        .justify_center()
        .child(
            text(move || i18n::t(language.get().as_str(), "history.diffEmpty"))
                .text_size(px(11.0))
                .text_color(move || design::text3()),
        );
}

/// 合并/并排切换：P1 只有合并视图，并排标为未实现（灰显，不响应）。
fn mode_toggle(language: Signal<SharedString>) -> impl IntoElement {
    return view()
        .h(px(20.0))
        .flex_row()
        .items_center()
        .gap(px(2.0))
        .p(px(2.0))
        .rounded(px(5.0))
        .border_1()
        .border_color(move || design::line_soft())
        .bg(move || design::panel2())
        .child(
            view()
                .h(px(16.0))
                .px(px(6.0))
                .items_center()
                .rounded(px(4.0))
                .bg(move || design::content())
                .child(
                    text(move || i18n::t(language.get().as_str(), "history.unified"))
                        .text_size(px(9.5))
                        .text_color(move || design::text()),
                ),
        )
        .child(
            view()
                .h(px(16.0))
                .px(px(6.0))
                .items_center()
                .opacity(0.5)
                .child(
                    text(move || i18n::t(language.get().as_str(), "history.split"))
                        .text_size(px(9.5))
                        .text_color(move || design::text3()),
                ),
        );
}

/// 一行补丁的界面元素。
fn diff_row(line: &DiffLine) -> AnyElement {
    let (background, foreground) = match line.kind {
        DiffLineKind::Add => (design::add_bg(), design::add_fg()),
        DiffLineKind::Remove => (design::del_bg(), design::del_fg()),
        DiffLineKind::Hunk => (design::panel2(), design::text2()),
        DiffLineKind::Meta => (design::content(), design::text3()),
        DiffLineKind::Context => (design::content(), design::text()),
    };
    let old = line.old.map(|value| value.to_string()).unwrap_or_default();
    let new = line.new.map(|value| value.to_string()).unwrap_or_default();
    let content = line.text.clone();
    return view()
        .w_full()
        .h(px(metrics::DIFF_LINE_H))
        .flex_row()
        .items_center()
        .bg(move || background)
        .child(
            text(old)
                .w(px(GUTTER_W))
                .flex_shrink_0()
                .pl(px(6.0))
                .text_size(px(metrics::DIFF_FONT))
                .line_height(px(metrics::DIFF_LINE_H))
                .font_family("SF Mono")
                .text_color(move || design::text3()),
        )
        .child(
            text(new)
                .w(px(GUTTER_W))
                .flex_shrink_0()
                .pl(px(6.0))
                .text_size(px(metrics::DIFF_FONT))
                .line_height(px(metrics::DIFF_LINE_H))
                .font_family("SF Mono")
                .text_color(move || design::text3()),
        )
        .child(
            text(content)
                .flex_1()
                .text_size(px(metrics::DIFF_FONT))
                .line_height(px(metrics::DIFF_LINE_H))
                .font_family("SF Mono")
                .text_color(move || foreground)
                .text_overflow(nsh_core::EllipsizeMode::Clip),
        )
        .into_element();
}
