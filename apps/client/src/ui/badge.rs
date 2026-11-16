//! 徽章、键帽与计数：行内的小型标记。
//!
//! 全部按 11pt 起排，底部圆角 4，颜色只表达语义（中性、强调、成功、警告、危险），
//! 不承载状态之外的信息。

use crate::design;
use crate::ui;
use nsh_core::prelude::*;

/// 徽章语义。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tone {
    Neutral,
    Accent,
    Success,
    Warning,
    Danger,
}

impl Tone {
    fn background(self) -> AdaptiveColor {
        return match self {
            Tone::Neutral => design::line_soft(),
            Tone::Accent => design::accent_soft(),
            Tone::Success => design::add_bg(),
            Tone::Warning => design::token("hunk-bg"),
            Tone::Danger => design::del_bg(),
        };
    }

    fn foreground(self) -> AdaptiveColor {
        return match self {
            Tone::Neutral => design::text2(),
            Tone::Accent => design::accent(),
            Tone::Success => design::add_fg(),
            Tone::Warning => design::warn(),
            Tone::Danger => design::del_fg(),
        };
    }
}

/// 徽章：高 18、左右 6、圆角 4。
pub(crate) fn badge(caption: impl Fn() -> String + 'static, tone: Tone) -> impl IntoElement {
    return view()
        .h(px(18.0))
        .px(px(6.0))
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .bg(move || tone.background())
        .child(
            text(move || caption())
                .text_size(px(ui::META))
                .line_height(px(18.0))
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || tone.foreground()),
        );
}

/// 纯计数（括号或行尾数字）：11pt、三级色、不占背景。
pub(crate) fn count(value: impl Fn() -> String + 'static) -> impl IntoElement {
    return text(move || value())
        .text_size(px(ui::META))
        .line_height(px(ui::SECONDARY_LINE))
        .text_color(move || design::text3());
}

/// 分支 chip：高 16、圆角 4、等宽 11pt，只显示分支名。
pub(crate) fn branch_chip(caption: impl Fn() -> String + 'static) -> impl IntoElement {
    return view()
        .h(px(16.0))
        .px(px(5.0))
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .border_1()
        .border_color(move || design::line())
        .bg(move || design::panel())
        .child(
            text(move || caption())
                .text_size(px(ui::META))
                .line_height(px(16.0))
                .font_family(ui::MONO)
                .text_color(move || design::text2()),
        );
}

/// 键帽：高 18、圆角 4、1px 描边，11pt。
pub(crate) fn kbd(caption: impl Into<SharedString>) -> impl IntoElement {
    let caption = caption.into();
    return view()
        .h(px(18.0))
        .px(px(5.0))
        .items_center()
        .justify_center()
        .rounded(px(4.0))
        .border_1()
        .border_color(move || design::line())
        .bg(move || design::panel())
        .child(
            text(caption)
                .text_size(px(ui::META))
                .line_height(px(18.0))
                .text_color(move || design::text2()),
        );
}
