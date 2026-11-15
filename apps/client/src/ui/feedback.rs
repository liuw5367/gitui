//! 提示条与进度：跨屏复用的反馈件。
//!
//! 设计稿里「操作状态条 / 同步结果 / 登录提示」是三套外观，这里收成一条带语义的
//! 横幅，状态用底色 + 文案共同表达（不只靠颜色）。

use crate::design::{self, metrics};
use crate::ui;
use nsh_core::prelude::*;

/// 横幅语义。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BannerTone {
    Info,
    Warning,
    Danger,
    /// 进行中的操作（合并/变基/拣选/回滚）。
    Accent,
}

impl BannerTone {
    fn background(self) -> AdaptiveColor {
        return match self {
            BannerTone::Info => design::panel(),
            BannerTone::Warning => design::token("hunk-bg"),
            BannerTone::Danger => design::del_bg(),
            BannerTone::Accent => design::token("tag-bg"),
        };
    }

    fn foreground(self) -> AdaptiveColor {
        return match self {
            BannerTone::Info => design::text2(),
            BannerTone::Warning => design::warn(),
            BannerTone::Danger => design::del_fg(),
            BannerTone::Accent => design::text(),
        };
    }
}

/// 横幅：一行文案 + 可选动作，高 32，占满宽度。
pub(crate) fn banner(
    tone: BannerTone,
    message: impl Fn() -> String + 'static,
    actions: Vec<AnyElement>,
) -> impl IntoElement {
    return banner_sized(tone, 32.0, None, message, actions);
}

/// 同 [`banner`]，可指定高度与前置图标——工作区三条横幅是 24/26/28 且都带图标。
pub(crate) fn banner_sized(
    tone: BannerTone,
    height: f32,
    icon: Option<&'static str>,
    message: impl Fn() -> String + 'static,
    actions: Vec<AnyElement>,
) -> impl IntoElement {
    let mut row = view()
        .w_full()
        .h(px(height))
        .flex_row()
        .items_center()
        .gap(px(ui::GAP))
        .px(px(ui::GAP_LG))
        .overflow_hidden()
        .bg(move || tone.background());
    if let Some(name) = icon {
        row = row.child(crate::ui::icon::icon_view(
            name,
            crate::ui::icon::IconSize::Button,
            move || tone.foreground(),
        ));
    }
    row = row.child(
        text(move || message())
            .flex_1()
            .min_w_0()
            .text_size(px(ui::SECONDARY))
            .line_height(px(ui::SECONDARY_LINE))
            .text_color(move || tone.foreground()),
    );
    for action in actions {
        row = row.child(action);
    }
    return row;
}

/// 进度条：高 4、圆角 2，`value` 为 0–1；`None` 表示不确定进度。
pub(crate) fn progress(value: Option<f32>) -> impl IntoElement {
    let mut bar = nsh_base::progress()
        .h(px(4.0))
        .w_full()
        .rounded(px(2.0))
        .track_style(move |style| style.h(px(4.0)).rounded(px(2.0)).bg(move || design::line_soft()))
        .indicator_style(move |style| {
            style
                .h(px(4.0))
                .rounded(px(2.0))
                .bg(move || design::token("primary-bg"))
        });
    bar = match value {
        Some(fraction) => bar.value(fraction),
        None => bar.indeterminate(true),
    };
    return view()
        .w_full()
        .h(px(4.0))
        .rounded(px(2.0))
        .overflow_hidden()
        .child(bar);
}

/// 卡片：面板的默认容器（圆角 10 + 1px 描边）。
pub(crate) fn card(children: impl IntoElement) -> impl IntoElement {
    return view()
        .w_full()
        .rounded(px(metrics::RADIUS_PANEL + 2.0))
        .border_1()
        .border_color(move || design::line())
        .bg(move || design::panel())
        .child(children);
}
