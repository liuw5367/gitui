//! 单文件的 blame + 文件历史（板 19）。
//!
//! 左侧是文件历史（`.fhist`），右侧是 blame 主体：`.blamebar` 一行摘要 + 按钮，
//! `.blamecode` 逐行显示「提交 / 作者 | 行号 | 代码」。
//!
//! 这个视图不占工具栏的分段——画板的入口是 `.blamebar` 里的 Files 按钮，从检查器
//! 的 Blame 页或文件行进来。

use crate::design::{self, metrics};
use crate::i18n;
use crate::model::Tab;
use crate::ui;
use nsh_core::prelude::*;

/// 左栏宽：与画板的 `.fhist` 一致。
const HISTORY_W: f32 = 260.0;
/// 行高：与 diff 正文相同的 18。
const ROW_H: f32 = 18.0;

pub(crate) fn blame_view(
    language: Signal<SharedString>,
    tab: Tab,
    on_copy: std::sync::Arc<dyn Fn(String)>,
) -> impl IntoElement {
    return view()
        .w_full()
        .flex_1()
        .min_h_0()
        .flex_row()
        .child(history_side(language, tab.clone()))
        .child(pane(language, tab, on_copy));
}

/// 左栏：文件历史列表。
fn history_side(language: Signal<SharedString>, tab: Tab) -> impl IntoElement {
    let mut rows: Vec<AnyElement> = Vec::new();
    for commit in tab.file_history.iter().take(40) {
        let short = commit.short.clone();
        let message = commit.summary.clone();
        let author = commit.author.clone();
        rows.push(
            view()
                .w_full()
                .h(px(44.0))
                .px(px(ui::GAP))
                .flex_col()
                .justify_center()
                .gap(px(2.0))
                .child(
                    text(message)
                        .w_full()
                        .text_size(px(ui::SECONDARY))
                        .line_height(px(ui::SECONDARY_LINE))
                        .text_color(move || design::text())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .child(
                    text(move || format!("{short} · {author}"))
                        .text_size(px(ui::META))
                        .text_color(move || design::text3()),
                )
                .into_element(),
        );
    }
    return view()
        .w(px(HISTORY_W))
        .h_full()
        .flex_shrink_0()
        .flex_col()
        .bg(move || design::panel())
        .border_r_1()
        .border_color(move || design::line())
        .child(
            view()
                .w_full()
                .h(px(ui::ROW))
                .flex_shrink_0()
                .px(px(ui::GAP))
                .flex_row()
                .items_center()
                .border_b_1()
                .border_color(move || design::line_soft())
                .child(
                    text(move || i18n::t(language.get().as_str(), "blame.history"))
                        .text_size(px(ui::SECONDARY))
                        .font_weight(FontWeight(600))
                        .text_color(move || design::text2()),
                ),
        )
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .child(scroll().w_full().h_full().child(
                    view().w_full().flex_col().children(rows),
                )),
        );
}

/// 右栏：`.blamebar` + `.blamecode`。
fn pane(
    language: Signal<SharedString>,
    tab: Tab,
    on_copy: std::sync::Arc<dyn Fn(String)>,
) -> impl IntoElement {
    let path = tab.blame_path.clone();
    let count = tab.blame.len();
    let copy_source = tab.blame_text();
    let mut rows: Vec<AnyElement> = Vec::new();
    for line in &tab.blame {
        let short = line.short.clone();
        let author = line.author.clone();
        let number = line.line.to_string();
        let text_value = line.text.clone();
        rows.push(
            view()
                .w_full()
                .h(px(ROW_H))
                .flex_row()
                .items_center()
                .child(
                    view()
                        .w(px(150.0))
                        .flex_shrink_0()
                        .flex_row()
                        .items_center()
                        .gap(px(ui::GAP_SM))
                        .pl(px(ui::GAP))
                        .child(
                            text(short)
                                .text_size(px(ui::CODE))
                                .line_height(px(ui::CODE_LINE))
                                .text_color(move || design::text3()),
                        )
                        .child(
                            text(author)
                                .text_size(px(ui::SECONDARY))
                                .line_height(px(ui::SECONDARY_LINE))
                                .text_color(move || design::text2())
                                .text_overflow(nsh_core::EllipsizeMode::Tail),
                        ),
                )
                .child(
                    text(number)
                        .w(px(44.0))
                        .flex_shrink_0()
                        .text_size(px(ui::CODE))
                        .line_height(px(ui::CODE_LINE))
                        .text_color(move || design::text3()),
                )
                .child(
                    text(text_value)
                        .flex_1()
                        .text_size(px(ui::CODE))
                        .line_height(px(ui::CODE_LINE))
                        .text_color(move || design::text()),
                )
                .into_element(),
        );
    }
    return view()
        .flex_1()
        .min_w_0()
        .h_full()
        .flex_col()
        .child(
            view()
                .w_full()
                .h(px(metrics::TOOLBAR_H))
                .flex_shrink_0()
                .px(px(ui::GAP_LG))
                .flex_row()
                .items_center()
                .gap(px(ui::GAP))
                .border_b_1()
                .border_color(move || design::line())
                .child(ui::badge::badge(
                    move || i18n::t(language.get().as_str(), "files.blame"),
                    ui::badge::Tone::Accent,
                ))
                .child(
                    text(path)
                        .text_size(px(ui::SECONDARY))
                        .text_color(move || design::text()),
                )
                .child(view().flex_1())
                .child(
                    text(format!("{count} lines"))
                        .text_size(px(ui::META))
                        .text_color(move || design::text3()),
                )
                .child(ui::button::button(
                    move || i18n::t(language.get().as_str(), "common.copy"),
                    ui::button::ButtonVariant::Secondary,
                    ui::button::ButtonSize::Small,
                    false,
                    move || on_copy(copy_source.clone()),
                )),
        )
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .child(scroll().w_full().h_full().child(
                    view().w_full().flex_col().children(rows),
                )),
        );
}
