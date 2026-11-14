//! diff 视图：`.diffbar` 38 高，正文 SF Mono 11/19，行号与正负行配色。
//!
//! 补丁解析在 `domain::diff`；这里只负责排版与配色。

use crate::design::{self, metrics};
use crate::i18n;
use crate::widgets;
use domain::diff::{DiffLine, DiffLineKind, Patch};
use nsh_core::prelude::*;

/// 行号栏宽度（旧、新各一列）。
const GUTTER_W: f32 = 34.0;

/// diff 的三个显示开关（设计板 04 的 fold / wrap / whitespace）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DiffOptions {
    /// 折叠未改动的大段上下文。
    pub fold: bool,
    /// 长行换行而不是横向截断。
    pub wrap: bool,
    /// 把行尾空格与制表符显示成可见符号。
    pub whitespace: bool,
}

impl DiffOptions {
    /// 折叠时保留的上下文行数。
    pub const FOLD_CONTEXT: usize = 3;
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn diff(
    language: Signal<SharedString>,
    label: String,
    patch_text: String,
    copy_text: std::sync::Arc<dyn Fn(String)>,
    options: DiffOptions,
    toggle_option: std::sync::Arc<dyn Fn(&'static str)>,
    export_patch: std::sync::Arc<dyn Fn()>,
    split_view: bool,
    set_split_mode: std::sync::Arc<dyn Fn(bool)>,
) -> impl IntoElement {
    let patch = Patch::parse(patch_text.as_str());
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
                    text(label)
                        .text_size(px(9.5))
                        .font_family("SF Mono")
                        .text_color(move || design::text3()),
                )
                .child(
                    text(format!("+{} −{}", additions, deletions))
                        .text_size(px(9.5))
                        .font_family("SF Mono")
                        .text_color(move || design::text3()),
                )
                .child(view().flex_1())
                .child(fold_toggle(language, options, toggle_option.clone()))
                .child(wrap_toggle(language, options, toggle_option.clone()))
                .child(space_toggle(language, options, toggle_option.clone()))
                .child(widgets::small_button(
                    {
                        let language = language;
                        move || i18n::t(language.get().as_str(), "diff.export")
                    },
                    move || export_patch(),
                ))
                .child(mode_toggle(language, split_view, set_split_mode.clone()))
                .child(crate::widgets::icon_button(
                    "copy",
                    12.0,
                    design::text3,
                    move || copy_text(patch_text.clone()),
                )),
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
                            // 并排：左右两栏、按行对齐（设计稿无规格，推断实现）。
                            if split_view {
                                let rows: Vec<AnyElement> = domain::diff::split_rows(
                                    &patch.lines,
                                )
                                .into_iter()
                                .map(|row| split_row(row, options))
                                .collect();
                                return view().w_full().flex_col().child(rows).into_element();
                            }
                            if options.fold {
                                let rows: Vec<AnyElement> = fold_lines(&patch.lines)
                                    .into_iter()
                                    .map(|row| match row {
                                        FoldRow::Line(line) => diff_row(&line, options),
                                        FoldRow::Gap(count) => {
                                            gap_row(language, count)
                                        }
                                    })
                                    .collect();
                                return view().w_full().flex_col().child(rows).into_element();
                            }
                            let rows: Vec<AnyElement> = patch
                                .lines
                                .iter()
                                .map(|line| diff_row(line, options))
                                .collect();
                            return view().w_full().flex_col().child(rows).into_element();
                        })),
                ),
            ),
        );
}

/// 一个图标开关（fold / wrap / whitespace），开启时用强调色。
fn diff_toggle(
    name: &'static str,
    hint_key: &'static str,
    active: bool,
    toggle: std::sync::Arc<dyn Fn(&'static str)>,
) -> impl IntoElement {
    return view()
        .h(px(20.0))
        .w(px(22.0))
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .bg(move || {
            if active {
                design::accent_soft()
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .hover(|style| style.bg(move || design::hover()))
        .on_press(move || toggle(hint_key))
        .child(widgets::icon(
            name,
            11.0,
            if active {
                design::accent
            } else {
                design::text3
            },
        ));
}

fn fold_toggle(
    language: Signal<SharedString>,
    options: DiffOptions,
    toggle: std::sync::Arc<dyn Fn(&'static str)>,
) -> impl IntoElement {
    let _ = language;
    return diff_toggle("fold", "fold", options.fold, toggle);
}

fn wrap_toggle(
    language: Signal<SharedString>,
    options: DiffOptions,
    toggle: std::sync::Arc<dyn Fn(&'static str)>,
) -> impl IntoElement {
    let _ = language;
    return diff_toggle("wrap", "wrap", options.wrap, toggle);
}

fn space_toggle(
    language: Signal<SharedString>,
    options: DiffOptions,
    toggle: std::sync::Arc<dyn Fn(&'static str)>,
) -> impl IntoElement {
    let _ = language;
    return diff_toggle("space", "whitespace", options.whitespace, toggle);
}

/// 把行尾空白画成可见符号（`·` 空格、`→` 制表）。
fn reveal_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '\t' => out.push_str("→   "),
            ' ' => out.push('·'),
            other => out.push(other),
        }
    }
    return out;
}

/// 折叠未改动的大段上下文：保留每一段改动前后的若干行，其余合成一条提示。
///
/// 只折叠 `Context` 行；`Hunk` 头与增删行一律保留，否则会看不清改了哪里。
fn fold_lines(lines: &[DiffLine]) -> Vec<FoldRow> {
    let mut rows: Vec<FoldRow> = Vec::new();
    let mut run: Vec<DiffLine> = Vec::new();
    let flush = |run: &mut Vec<DiffLine>, rows: &mut Vec<FoldRow>| {
        if run.is_empty() {
            return;
        }
        let lines = std::mem::take(run);
        if lines.len() <= DiffOptions::FOLD_CONTEXT * 2 + 1 {
            rows.extend(lines.into_iter().map(FoldRow::Line));
            return;
        }
        let head = DiffOptions::FOLD_CONTEXT;
        for line in &lines[..head] {
            rows.push(FoldRow::Line(line.clone()));
        }
        rows.push(FoldRow::Gap(lines.len() - head * 2));
        for line in &lines[lines.len() - head..] {
            rows.push(FoldRow::Line(line.clone()));
        }
    };
    for line in lines {
        if line.kind == DiffLineKind::Context {
            run.push(line.clone());
        } else {
            flush(&mut run, &mut rows);
            rows.push(FoldRow::Line(line.clone()));
        }
    }
    flush(&mut run, &mut rows);
    return rows;
}

/// 折叠后的一行：真实 diff 行或被省略的上下文。
enum FoldRow {
    Line(DiffLine),
    Gap(usize),
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

/// 合并 / 并排切换（设计板 04 的 Split / Unified）。
///
/// 设计稿没有并排 diff 的排版规格，这里按通用做法实现左右两栏、按行对齐，
/// 属于推断而非像素还原。
fn mode_toggle(
    language: Signal<SharedString>,
    split: bool,
    set_split: std::sync::Arc<dyn Fn(bool)>,
) -> impl IntoElement {
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
        .child(mode_button(
            language,
            "history.unified",
            !split,
            {
                let set_split = set_split.clone();
                move || set_split(false)
            },
        ))
        .child(mode_button(language, "history.split", split, move || set_split(true)));
}

fn mode_button(
    language: Signal<SharedString>,
    key: &'static str,
    active: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    return view()
        .h(px(16.0))
        .px(px(6.0))
        .items_center()
        .rounded(px(4.0))
        .bg(move || {
            if active {
                design::content()
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .on_press(on_press)
        .child(
            text(move || i18n::t(language.get().as_str(), key))
                .text_size(px(9.5))
                .text_color(move || {
                    if active {
                        design::text()
                    } else {
                        design::text3()
                    }
                }),
        );
}

/// 一行补丁的界面元素。
/// 并排视图的一行：左（旧）与右（新）各占一半，空侧留白。
fn split_row(row: domain::diff::SplitRow, options: DiffOptions) -> AnyElement {
    return view()
        .w_full()
        .h(px(metrics::DIFF_LINE_H))
        .flex_row()
        .items_center()
        .child(split_cell(row.left, options))
        .child(split_cell(row.right, options))
        .into_element();
}

fn split_cell(line: Option<DiffLine>, options: DiffOptions) -> AnyElement {
    let Some(line) = line else {
        // 空侧用更淡的底色，让「这一侧没有对应行」一眼可见。
        return view()
            .flex_1()
            .min_w_0()
            .h_full()
            .bg(move || design::panel2())
            .into_element();
    };
    let (background, foreground) = match line.kind {
        DiffLineKind::Add => (design::add_bg(), design::add_fg()),
        DiffLineKind::Remove => (design::del_bg(), design::del_fg()),
        DiffLineKind::Hunk => (design::panel2(), design::text2()),
        DiffLineKind::Meta => (design::content(), design::text3()),
        DiffLineKind::Context => (design::content(), design::text()),
    };
    let number = line
        .old
        .or(line.new)
        .map(|value| value.to_string())
        .unwrap_or_default();
    let content = if options.whitespace {
        reveal_whitespace(line.text.as_str())
    } else {
        line.text.clone()
    };
    return view()
        .flex_1()
        .min_w_0()
        .h_full()
        .flex_row()
        .items_center()
        .bg(move || background)
        .child(
            text(number)
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
                .when(!options.wrap, |this| {
                    this.text_overflow(nsh_core::EllipsizeMode::Clip)
                }),
        )
        .into_element();
}

fn gap_row(language: Signal<SharedString>, count: usize) -> AnyElement {
    return view()
        .w_full()
        .h(px(metrics::DIFF_LINE_H))
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .px(px(GUTTER_W * 2.0 + 6.0))
        .bg(move || design::panel2())
        .child(
            text(move || {
                i18n::with_arguments(
                    language.get().as_str(),
                    "diff.folded",
                    &[("count", count.to_string())],
                )
            })
            .text_size(px(9.5))
            .line_height(px(12.0))
            .text_color(move || design::text3()),
        )
        .into_element();
}

fn diff_row(line: &DiffLine, options: DiffOptions) -> AnyElement {
    let (background, foreground) = match line.kind {
        DiffLineKind::Add => (design::add_bg(), design::add_fg()),
        DiffLineKind::Remove => (design::del_bg(), design::del_fg()),
        DiffLineKind::Hunk => (design::panel2(), design::text2()),
        DiffLineKind::Meta => (design::content(), design::text3()),
        DiffLineKind::Context => (design::content(), design::text()),
    };
    let old = line.old.map(|value| value.to_string()).unwrap_or_default();
    let new = line.new.map(|value| value.to_string()).unwrap_or_default();
    // 显示空白字符时只改呈现，不改内容本身。
    let content = if options.whitespace {
        reveal_whitespace(line.text.as_str())
    } else {
        line.text.clone()
    };
    let wraps = options.wrap;
    return view()
        .w_full()
        .when(!wraps, |this| this.h(px(metrics::DIFF_LINE_H)))
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
                // 换行开着就不截断；行高由排版自动撑开。
                .when(!wraps, |this| this.text_overflow(nsh_core::EllipsizeMode::Clip)),
        )
        .into_element();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(kind: DiffLineKind, text: &str) -> DiffLine {
        return DiffLine {
            kind,
            old: None,
            new: None,
            text: text.to_string(),
        };
    }

    #[test]
    fn whitespace_becomes_visible_without_losing_content() {
        assert_eq!(reveal_whitespace("a b\t"), "a·b→   ");
        assert_eq!(reveal_whitespace("plain"), "plain");
    }

    #[test]
    fn folding_keeps_edges_and_collapses_the_middle() {
        let mut lines = vec![line(DiffLineKind::Hunk, "@@ -1,20 +1,20 @@")];
        // 12 行上下文：折叠规则保留头 3 与尾 3。
        for index in 0..12 {
            lines.push(line(DiffLineKind::Context, &format!("ctx {index}")));
        }
        lines.push(line(DiffLineKind::Add, "+new"));
        let rows = fold_lines(&lines);
        let gaps: Vec<usize> = rows
            .iter()
            .filter_map(|row| match row {
                FoldRow::Gap(count) => Some(*count),
                _ => None,
            })
            .collect();
        assert_eq!(gaps, vec![6], "12 行上下文折叠中间 6 行");
        // 增行与 hunk 头一律保留。
        assert!(rows.iter().any(|row| matches!(row, FoldRow::Line(line) if line.kind == DiffLineKind::Add)));
        assert!(rows.iter().any(|row| matches!(row, FoldRow::Line(line) if line.kind == DiffLineKind::Hunk)));
    }

    #[test]
    fn short_context_runs_are_not_folded() {
        let lines = vec![
            line(DiffLineKind::Context, "a"),
            line(DiffLineKind::Context, "b"),
            line(DiffLineKind::Add, "+c"),
        ];
        let rows = fold_lines(&lines);
        assert_eq!(rows.len(), 3);
        assert!(!rows.iter().any(|row| matches!(row, FoldRow::Gap(_))));
    }
}
