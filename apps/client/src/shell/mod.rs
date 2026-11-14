//! 工作区外壳：标题栏（仓库标签页）、工具栏与状态栏。
//!
//! 尺寸取自设计稿：`.titlebar.with-tabs` 44、`.toolbar.v2` 44、`.statusbar` 24。
//! P1 中 Fetch/Pull/Push、Branch/Stash、Agent 还没有对应的写路径或同步实现，
//! 这些按钮按禁用态渲染，不伪装成可用。

use crate::design::{self, metrics};
use crate::i18n;
use crate::model::Tab;
use crate::widgets;
use crate::Actions;
use nsh_base::input;
use nsh_core::prelude::*;

/// 标题栏：左侧留出系统窗口按钮，其后是仓库标签页与状态。
pub(crate) fn titlebar(
    language: Signal<SharedString>,
    tabs: Signal<Vec<Tab>>,
    active: Signal<usize>,
    actions: Actions,
) -> impl IntoElement {
    let (left, right) = crate::window_insets();
    let activate = actions.activate_tab.clone();
    let close = actions.close_tab.clone();
    let open = actions.choose_folder.clone();
    return view()
        .w_full()
        .h(px(metrics::TITLEBAR_H))
        .flex_row()
        .items_center()
        .gap(px(4.0))
        .pl(px(left + 10.0))
        .pr(px(right + 10.0))
        .bg(move || design::titlebar())
        .border_b_1()
        .border_color(move || design::line())
        .window_drag(true)
        .child(
            view()
                .flex_1()
                .min_w_0()
                .h_full()
                .flex_row()
                .items_center()
                .gap(px(4.0))
                .child(each(
                    tabs,
                    |tab: &Tab| tab.path.clone(),
                    move |tab, index| {
                        let activate = activate.clone();
                        let close = close.clone();
                        let is_active = move || active.get() == index.get();
                        view()
                            .h(px(metrics::RTAB_H))
                            .px(px(9.0))
                            .flex_row()
                            .items_center()
                            .gap(px(7.0))
                            .rounded(px(metrics::RADIUS_CONTROL))
                            .border_1()
                            .border_color(move || {
                                if is_active() {
                                    design::line()
                                } else {
                                    AdaptiveColor::TRANSPARENT
                                }
                            })
                            .bg(move || {
                                if is_active() {
                                    design::content()
                                } else {
                                    AdaptiveColor::TRANSPARENT
                                }
                            })
                            .hover(|style| style.bg(move || design::hover()))
                            .on_press(move || activate(index.get()))
                            .child(
                                text(move || tab.get().label())
                                    .text_size(px(11.5))
                                    .text_color(move || {
                                        if is_active() {
                                            design::text()
                                        } else {
                                            design::text2()
                                        }
                                    })
                                    .text_overflow(nsh_core::EllipsizeMode::Tail),
                            )
                            .child(
                                view()
                                    .w(px(14.0))
                                    .h(px(14.0))
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(4.0))
                                    .hover(|style| style.bg(move || design::line_soft()))
                                    .on_press(move || close(index.get()))
                                    .child(
                                        text("×")
                                            .text_size(px(11.0))
                                            .text_color(move || design::text3()),
                                    ),
                            )
                    },
                ))
                .child(
                    view()
                        .w(px(27.0))
                        .h(px(metrics::RTAB_H))
                        .items_center()
                        .justify_center()
                        .rounded(px(metrics::RADIUS_CONTROL))
                        .hover(|style| style.bg(move || design::hover()))
                        .on_press({
                            let open = open.clone();
                            move || open()
                        })
                        .child(widgets::icon("plus", 12.0, design::text2)),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .flex_shrink_0()
                .child(
                    view()
                        .w(px(6.0))
                        .h(px(6.0))
                        .rounded(px(3.0))
                        .bg(move || design::ok()),
                )
                .child(
                    text(move || {
                        let list = tabs.get();
                        let index = active.get();
                        match list.get(index) {
                            Some(tab) => {
                                let mut text = tab.branch.clone();
                                if tab.ahead > 0 {
                                    text.push_str(&format!(" ↑{}", tab.ahead));
                                }
                                if tab.behind > 0 {
                                    text.push_str(&format!(" ↓{}", tab.behind));
                                }
                                return text;
                            }
                            None => String::new(),
                        }
                    })
                    .text_size(px(11.0))
                    .text_color(move || design::text3()),
                )
                .child(
                    view()
                        .w(px(26.0))
                        .h(px(24.0))
                        .items_center()
                        .justify_center()
                        .rounded(px(metrics::RADIUS_CONTROL))
                        .hover(|style| style.bg(move || design::hover()))
                        .on_press({
                            let toggle = actions.toggle_language.clone();
                            move || toggle()
                        })
                        .child(
                            text(move || {
                                if language.get().as_str() == i18n::ZH_HANS_LANG {
                                    "EN".to_string()
                                } else {
                                    "中".to_string()
                                }
                            })
                            .text_size(px(11.0))
                            .text_color(move || design::text2()),
                        ),
                )
                .child(widgets::icon("gear", 14.0, design::text2)),
        );
}

/// 工具栏：仓库操作、搜索、History/Changes 切换与 Agent 入口。
pub(crate) fn toolbar(
    language: Signal<SharedString>,
    history_query: Signal<String>,
    change_count: impl Fn() -> u32 + 'static,
) -> impl IntoElement {
    return view()
        .w_full()
        .h(px(metrics::TOOLBAR_H))
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .px(px(10.0))
        .bg(move || design::bar())
        .border_b_1()
        .border_color(move || design::line())
        .child(widgets::icon_button(
            "sidebar",
            14.0,
            design::text2,
            || {},
        ))
        .child(widgets::separator())
        .child(disabled_tool(language, "refresh", "shell.fetch"))
        .child(disabled_tool(language, "down", "shell.pull"))
        .child(disabled_tool(language, "up", "shell.push"))
        .child(widgets::separator())
        .child(disabled_tool(language, "branch", "shell.branch"))
        .child(disabled_tool(language, "archive", "shell.stash"))
        .child(view().flex_1())
        .child(
            view()
                .w(px(metrics::SEARCH_W))
                .h(px(metrics::SEARCH_H))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .px(px(8.0))
                .rounded(px(metrics::RADIUS_CONTROL))
                .border_1()
                .border_color(move || design::field_border())
                .bg(move || design::field())
                .child(widgets::icon("search", 12.0, design::text3))
                .child(
                    input()
                        .value(history_query)
                        .on_change(move |value: &str| history_query.set(value.to_string()))
                        .placeholder(move || i18n::t(language.get().as_str(), "history.filter"))
                        .text_size(px(12.0))
                        .flex_1(),
                )
                .child(widgets::kbd("⌘F")),
        )
        .child(widgets::separator())
        .child(
            view()
                .h(px(26.0))
                .flex_row()
                .items_center()
                .gap(px(2.0))
                .p(px(2.0))
                .rounded(px(7.0))
                .border_1()
                .border_color(move || design::line_soft())
                .bg(move || design::panel2())
                .child(
                    view()
                        .h(px(22.0))
                        .px(px(10.0))
                        .items_center()
                        .rounded(px(5.0))
                        .bg(move || design::content())
                        .child(
                            text(move || i18n::t(language.get().as_str(), "shell.history"))
                                .text_size(px(11.5))
                                .text_color(move || design::text()),
                        ),
                )
                .child(
                    view()
                        .h(px(22.0))
                        .px(px(10.0))
                        .flex_row()
                        .items_center()
                        .gap(px(5.0))
                        .child(
                            text(move || i18n::t(language.get().as_str(), "shell.changes"))
                                .text_size(px(11.5))
                                .text_color(move || design::text2()),
                        )
                        .child(
                            text(move || change_count().to_string())
                                .text_size(px(9.5))
                                .text_color(move || design::text3()),
                        ),
                ),
        )
        .child(disabled_tool(language, "sparkle", "shell.agent"))
        .child(widgets::icon("gear", 14.0, design::text2));
}

/// 状态栏：分支、操作状态、变更数量与编码信息。
pub(crate) fn statusbar(language: Signal<SharedString>, tab: Memo<Option<Tab>>) -> impl IntoElement {
    return view()
        .w_full()
        .h(px(metrics::STATUS_H))
        .flex_row()
        .items_center()
        .gap(px(9.0))
        .px(px(12.0))
        .bg(move || design::status())
        .border_t_1()
        .border_color(move || design::line())
        .child(widgets::icon("branch", 11.0, design::text3))
        .child(
            text(move || {
                return tab
                    .get()
                    .map(|tab| tab.branch)
                    .unwrap_or_default();
            })
            .text_size(px(10.5))
            .text_color(move || design::text3()),
        )
        .child(view().flex_1())
        .child(
            text(move || {
                let count = tab
                    .get()
                    .map(|tab| tab.status.len())
                    .unwrap_or(0);
                return i18n::with_arguments(
                    language.get().as_str(),
                    "history.commits",
                    &[("count", count.to_string())],
                );
            })
            .text_size(px(10.5))
            .text_color(move || design::text3()),
        )
        .child(
            text("UTF-8")
                .text_size(px(10.5))
                .text_color(move || design::text3()),
        )
        .child(
            text("LF")
                .text_size(px(10.5))
                .text_color(move || design::text3()),
        );
}

/// 尚未接入的动作：按钮按设计稿渲染，但处于禁用态（不响应按压）。
fn disabled_tool(
    language: Signal<SharedString>,
    icon: &'static str,
    key: &'static str,
) -> impl IntoElement {
    return view()
        .h(px(metrics::TBTN_H))
        .px(px(9.0))
        .flex_row()
        .items_center()
        .gap(px(5.0))
        .rounded(px(metrics::RADIUS_CONTROL))
        .opacity(0.45)
        .child(widgets::icon(icon, 13.0, design::text2))
        .child(
            text(move || i18n::t(language.get().as_str(), key))
                .text_size(px(12.0))
                .text_color(move || design::text()),
        );
}
