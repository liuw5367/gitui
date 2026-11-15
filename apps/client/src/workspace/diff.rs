//! diff 视图：顶部工具栏（导出、复制、三个显示开关）+ 正文。
//!
//! 正文自 P2 起交给 `ui::diff_surface`（包装 `nsh_base::DiffView`），等宽 12 / 行高
//! 18、行号与正负行配色都由它负责。本文件只剩工具栏与开关。
//!
//! 补丁解析在 `domain::diff`。

use crate::design::{self, metrics};
use crate::i18n;
use crate::widgets;
use domain::diff::Patch;
use nsh_core::prelude::*;


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
    let patch_for_view = patch_text.clone();
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
                        .text_size(px(crate::ui::CODE))
                        
                        .text_color(move || design::text())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .child(
                    text(label)
                        .text_size(px(crate::ui::CODE))
                        
                        .text_color(move || design::text3()),
                )
                .child(
                    text(format!("+{} −{}", additions, deletions))
                        .text_size(px(crate::ui::CODE))
                        
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
                dynamic(move || -> AnyElement {
                    if patch.lines.is_empty() {
                        return empty_state(language).into_element();
                    }
                    // 渲染交给 nsh 的 DiffView：分栏/统一、折叠、软换行、行号与
                    // 指示条都由它提供，这里只传 patch 与两个显示开关。
                    let layout = if split_view {
                        crate::ui::diff_surface::Layout::Split
                    } else {
                        crate::ui::diff_surface::Layout::Unified
                    };
                    return crate::ui::diff_surface::diff_surface(
                        SharedString::from(patch_for_view.clone()),
                        layout,
                        options.wrap,
                        None,
                    )
                    .into_element();
                }),
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




/// 空补丁提示（无改动或读取失败）。
fn empty_state(language: Signal<SharedString>) -> impl IntoElement {
    return view()
        .w_full()
        .h(px(60.0))
        .items_center()
        .justify_center()
        .child(
            text(move || i18n::t(language.get().as_str(), "history.diffEmpty"))
                .text_size(px(crate::ui::SECONDARY))
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
    // Unified / Split 二选一：选中的用 Secondary 表达，未选中用 Ghost。
    let variant = if active {
        crate::ui::button::ButtonVariant::Secondary
    } else {
        crate::ui::button::ButtonVariant::Ghost
    };
    return crate::ui::button::button(
        move || i18n::t(language.get().as_str(), key),
        variant,
        crate::ui::button::ButtonSize::Small,
        false,
        on_press,
    );
}
