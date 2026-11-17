//! 页签条：面板内的互斥视图切换（检查器 Files / Info / Blame、同步 Fetch / Pull / Push）。
//!
//! 行为来自 `nsh_base::tabs`/`tab`（每一页签自己处理按压与选中语义，键盘可进入），
//! 外观是系统页签：行高 28、选中项文字转正文色并在底部画 2pt 强调线。

use crate::design;
use crate::ui;
use crate::{app::session::Tab as RepositoryTab, i18n};
use nsh_core::prelude::*;

/// 一个页签。
pub(crate) struct TabItem {
    /// 稳定标识，用于比较。
    pub(crate) id: &'static str,
    /// 显示文案（已翻译）。
    pub(crate) label: Box<dyn Fn() -> String>,
}

/// 页签条。`selected` 是当前页签的 `id`。
pub(crate) fn tabs(
    items: Vec<TabItem>,
    selected: &'static str,
    on_select: impl Fn(&'static str) + 'static,
) -> impl IntoElement {
    let on_select = std::rc::Rc::new(on_select);
    let mut bar = nsh_base::tabs()
        .w_full()
        .h(px(ui::ROW))
        .flex_row()
        .items_end()
        .gap(px(ui::GAP_XL))
        .border_b_1()
        .border_color(move || design::line_soft());
    for item in items {
        let id = item.id;
        let label = item.label;
        let is_selected = id == selected;
        let press = on_select.clone();
        bar = bar.child(
            nsh_base::tab()
                .selected(is_selected)
                .h(px(ui::ROW))
                .flex_col()
                .justify_end()
                .gap(px(6.0))
                .focus_visible(move |style| {
                    style
                        .outline(px(ui::FOCUS_WIDTH))
                        .outline_color(move || ui::focus_color())
                })
                .on_press(move |_: &()| press(id))
                .child(
                    text(move || label())
                        .text_size(px(ui::BODY))
                        .line_height(px(ui::BODY_LINE))
                        .font_weight(if is_selected {
                            FontWeight::MEDIUM
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(move || {
                            if is_selected {
                                design::text()
                            } else {
                                design::text2()
                            }
                        }),
                )
                .child(view().w_full().h(px(2.0)).bg(move || {
                    if is_selected {
                        design::token("primary-bg")
                    } else {
                        AdaptiveColor::TRANSPARENT
                    }
                })),
        );
    }
    return bar;
}

/// 主窗口标题栏里的仓库页签，宽度与次序来自工作区设计板 04。
pub(crate) fn repository_tabs(
    tabs: Signal<Vec<RepositoryTab>>,
    active: Signal<usize>,
    session: crate::features::SessionHandle,
    language: Signal<SharedString>,
) -> impl IntoElement {
    let widths = signal(Vec::<(u64, f32)>::new());
    return scroll()
        .horizontal()
        .scrollbar(false)
        .flex_1()
        .min_w_0()
        .h_full()
        .child(
            view()
                .h_full()
                .flex_row()
                .items_center()
                .gap(px(4.0))
                .flex_shrink_0()
                .child(each(
                    tabs,
                    |tab: &RepositoryTab| tab.id,
                    move |tab, index| {
                        let id = tab.get_untracked().id;
                        let activate = session.clone();
                        let close = session.clone();
                        let move_tabs = session.clone();
                        let width_state = widths;
                        let start_x = signal(0.0_f32);
                        let current = move || active.get() == index.get();
                        let close_label = i18n::with_arguments(
                            language.get_untracked().as_str(),
                            "shell.closeRepository",
                            &[("name", tab.get_untracked().label())],
                        );
                        view()
                            .h(px(27.0))
                            .max_w(px(178.0))
                            .min_w(px(72.0))
                            .px(px(9.0))
                            .flex_row()
                            .items_center()
                            .gap(px(7.0))
                            .flex_shrink_0()
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(move || {
                                if current() {
                                    design::token("line-strong")
                                } else {
                                    AdaptiveColor::TRANSPARENT
                                }
                            })
                            .bg(move || {
                                if current() {
                                    design::content()
                                } else {
                                    AdaptiveColor::TRANSPARENT
                                }
                            })
                            .hover(|style| style.bg(move || design::hover()))
                            .on_layout(move |bounds: &Bounds<Points>| {
                                let mut measured = width_state.get_untracked();
                                if let Some((_, width)) =
                                    measured.iter_mut().find(|(key, _)| *key == id)
                                {
                                    *width = bounds.size.width.0;
                                } else {
                                    measured.push((id, bounds.size.width.0));
                                }
                                width_state.set(measured);
                            })
                            .claim_touches(TouchClaim::on_pan(PanActivation::horizontal(
                                px(8.0),
                                px(8.0),
                            )))
                            .on_responder_grant(move |event: &ResponderEvent| {
                                start_x.set(event.location.x.0);
                            })
                            .on_responder_release(move |event: &ResponderEvent| {
                                let order: Vec<u64> =
                                    tabs.get_untracked().iter().map(|tab| tab.id).collect();
                                let Some(source) =
                                    order.iter().position(|candidate| *candidate == id)
                                else {
                                    return;
                                };
                                let measured = width_state.get_untracked();
                                let width = |position: usize| {
                                    measured
                                        .iter()
                                        .find(|(key, _)| order.get(position) == Some(key))
                                        .map(|(_, width)| *width)
                                        .unwrap_or(120.0)
                                };
                                let delta = event.gesture.delta.x.0;
                                let origin = start_x.get_untracked();
                                let mut destination = source;
                                if delta > 0.0 {
                                    for target in source + 1..order.len() {
                                        let threshold = width(source) - origin
                                            + (source + 1..target).map(width).sum::<f32>()
                                            + width(target) / 2.0;
                                        if delta > threshold {
                                            destination = target;
                                        }
                                    }
                                } else if delta < 0.0 {
                                    for target in (0..source).rev() {
                                        let threshold = -(origin
                                            + (target + 1..source).map(width).sum::<f32>()
                                            + width(target) / 2.0);
                                        if delta < threshold {
                                            destination = target;
                                        }
                                    }
                                }
                                move_tabs.move_tab(id, destination);
                            })
                            .on_press(move || activate.activate_tab_id(id))
                            .child(dynamic({
                                let current = current.clone();
                                move || -> AnyElement {
                                    if current() {
                                        return view()
                                            .w(px(5.0))
                                            .h(px(5.0))
                                            .flex_shrink_0()
                                            .rounded(px(3.0))
                                            .bg(move || design::token("accent"))
                                            .into_element();
                                    }
                                    return view().w(px(0.0)).h(px(0.0)).into_element();
                                }
                            }))
                            .child(
                                text(move || tab.get().label())
                                    .text_size(px(ui::BODY))
                                    .line_height(px(ui::BODY_LINE))
                                    .flex_1()
                                    .min_w_0()
                                    .text_color(move || {
                                        if current() {
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
                                    .flex_shrink_0()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(4.0))
                                    .opacity(if current() { 0.0 } else { 0.55 })
                                    .hover(|style| {
                                        style.bg(move || design::line_soft()).opacity(1.0)
                                    })
                                    .focus_visible(|style| {
                                        style
                                            .outline(px(ui::FOCUS_WIDTH))
                                            .outline_color(move || ui::focus_color())
                                    })
                                    .label(SharedString::from(close_label))
                                    .on_press(move || close.close_tab_id(id))
                                    .child(
                                        text("×")
                                            .text_size(px(ui::SECONDARY))
                                            .line_height(px(ui::SECONDARY_LINE))
                                            .text_color(move || design::text3()),
                                    ),
                            )
                    },
                )),
        );
}
