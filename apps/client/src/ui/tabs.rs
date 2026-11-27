//! 页签条：面板内的互斥视图切换（Blame Changes / History、同步 Fetch / Pull / Push）。
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
///
/// 页签条**按内容取宽**（`flex_shrink(1.0)`，不 `flex_grow`）：`+` 要紧跟在最后一个
/// 页签右侧，而不是被推到标题栏最右端；页签多到放不下时才收缩并横向滚动。
pub(crate) fn repository_tabs(
    tabs: Signal<Vec<RepositoryTab>>,
    active: Signal<usize>,
    session: crate::features::SessionHandle,
    language: Signal<SharedString>,
) -> impl IntoElement {
    let widths = signal(Vec::<(u64, f32)>::new());
    let drop_boundary = signal(None::<usize>);
    let tab_count = computed(move || tabs.with(|list| list.len()));
    return scroll()
        .horizontal()
        .scrollbar(false)
        .flex_shrink(1.0)
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
                        let tab_ref = element_ref();
                        // 选中态先立即更新；等本帧完成布局，再把对应页签滚入视口。
                        // 按 ID 复核可避免快速连续切换时，过期的滚动任务移动到旧页签。
                        effect(move || {
                            if active.get() != index.get() {
                                return;
                            }
                            let tab_ref = tab_ref;
                            let active = active;
                            let tabs = tabs;
                            spawn(async move {
                                next_frame().await;
                                let still_active = tabs.with_untracked(|list| {
                                    list.get(active.get_untracked())
                                        .is_some_and(|tab| tab.id == id)
                                });
                                if still_active {
                                    tab_ref.reveal(
                                        nsh_core::RevealOptions::default()
                                            .horizontal(nsh_core::ScrollAlignment::Nearest)
                                            .animated(false),
                                    );
                                }
                            });
                        });
                        let activate = session.clone();
                        let close = session.clone();
                        let context_session = session.clone();
                        let move_tabs = session.clone();
                        let width_state = widths;
                        let drop_boundary_for_item = drop_boundary;
                        let start_x = signal(0.0_f32);
                        // 页签自身的悬停：× 的显隐跟着它走。设计稿的要求是「活动页签
                        // 平时不显示 ×、悬停才出现」，而 `on_mouse_enter` 在「指针进入
                        // 该视图或其内部视图」时都会触发，所以整块页签悬停即可，不必
                        // 精确悬停到 14pt 的 × 上（它的常态不透明度就是 0）。
                        let hovered = signal(false);
                        let current = move || active.get() == index.get();
                        let close_label = i18n::with_arguments(
                            language.get_untracked().as_str(),
                            "shell.closeRepository",
                            &[("name", tab.get_untracked().label())],
                        );
                        view()
                            .relative()
                            .element_ref(&tab_ref)
                            .h(px(27.0))
                            .max_w(px(178.0))
                            .min_w(px(72.0))
                            .pl(px(9.0))
                            .pr(px(4.0))
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
                            .on_mouse_enter(move |_: &MouseEvent| hovered.set(true))
                            .on_mouse_leave(move |_: &MouseEvent| hovered.set(false))
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
                                drop_boundary.set(None);
                            })
                            .on_responder_move({
                                let tabs = tabs;
                                let width_state = width_state;
                                let drop_boundary = drop_boundary;
                                move |event: &ResponderEvent| {
                                    let order: Vec<u64> =
                                        tabs.get_untracked().iter().map(|tab| tab.id).collect();
                                    let Some(source) =
                                        order.iter().position(|candidate| *candidate == id)
                                    else {
                                        drop_boundary.set(None);
                                        return;
                                    };
                                    let measured = width_state.get_untracked();
                                    let destination = tab_drop_destination(
                                        source,
                                        start_x.get_untracked(),
                                        event.gesture.delta.x.0,
                                        &order,
                                        &measured,
                                    );
                                    let boundary = tab_drop_boundary(source, destination);
                                    if drop_boundary.get_untracked() != boundary {
                                        drop_boundary.set(boundary);
                                    }
                                }
                            })
                            .on_responder_release(move |event: &ResponderEvent| {
                                let order: Vec<u64> =
                                    tabs.get_untracked().iter().map(|tab| tab.id).collect();
                                let Some(source) =
                                    order.iter().position(|candidate| *candidate == id)
                                else {
                                    drop_boundary.set(None);
                                    return;
                                };
                                let measured = width_state.get_untracked();
                                let destination = tab_drop_destination(
                                    source,
                                    start_x.get_untracked(),
                                    event.gesture.delta.x.0,
                                    &order,
                                    &measured,
                                );
                                drop_boundary.set(None);
                                move_tabs.move_tab(id, destination);
                            })
                            .on_responder_terminate(move |_: &ResponderEvent| {
                                drop_boundary.set(None);
                            })
                            .on_press(move || activate.activate_tab_id(id))
                            .on_context_menu(move |event: &ContextMenuEvent| {
                                let current_tabs = tabs.get_untracked();
                                let Some(position) =
                                    current_tabs.iter().position(|tab| tab.id == id)
                                else {
                                    return;
                                };
                                let count = current_tabs.len();
                                let lang = language.get_untracked();
                                let mut items = vec![
                                    crate::ui::menu::action(
                                        i18n::t(lang.as_str(), "menu.closeTab"),
                                        true,
                                        {
                                            let session = context_session.clone();
                                            move || session.close_tab_id(id)
                                        },
                                    ),
                                    crate::ui::menu::divider(),
                                    crate::ui::menu::action(
                                        i18n::t(lang.as_str(), "menu.copyPath"),
                                        true,
                                        {
                                            let tabs = tabs;
                                            let notice = context_session.notice;
                                            let language = language;
                                            move || {
                                                let path = tabs.with_untracked(|list| {
                                                    list.iter()
                                                        .find(|tab| tab.id == id)
                                                        .map(|tab| tab.path.clone())
                                                });
                                                if let Some(path) = path {
                                                    crate::shared::os::copy_text(
                                                        notice, language, path,
                                                    );
                                                }
                                            }
                                        },
                                    ),
                                    crate::ui::menu::action(
                                        i18n::t(lang.as_str(), "menu.reveal"),
                                        true,
                                        {
                                            let tabs = tabs;
                                            let notice = context_session.notice;
                                            let language = language;
                                            move || {
                                                let path = tabs.with_untracked(|list| {
                                                    list.iter()
                                                        .find(|tab| tab.id == id)
                                                        .map(|tab| tab.path.clone())
                                                });
                                                if let Some(path) = path {
                                                    crate::shared::os::reveal_path(
                                                        notice, language, path,
                                                    );
                                                }
                                            }
                                        },
                                    ),
                                    crate::ui::menu::divider(),
                                    crate::ui::menu::action(
                                        i18n::t(lang.as_str(), "menu.closeTabsRight"),
                                        position + 1 < count,
                                        {
                                            let session = context_session.clone();
                                            move || session.close_tabs_right_of(id)
                                        },
                                    ),
                                    crate::ui::menu::action(
                                        i18n::t(lang.as_str(), "menu.closeTabsLeft"),
                                        position > 0,
                                        {
                                            let session = context_session.clone();
                                            move || session.close_tabs_left_of(id)
                                        },
                                    ),
                                    crate::ui::menu::divider(),
                                ];
                                let tabs_for_all = tabs;
                                let session = context_session.clone();
                                items.push(crate::ui::menu::action(
                                    i18n::t(lang.as_str(), "menu.closeAllTabs"),
                                    true,
                                    move || {
                                        let anchor_exists = tabs_for_all.with_untracked(|list| {
                                            list.iter().any(|tab| tab.id == id)
                                        });
                                        if anchor_exists {
                                            session.close_all_tabs();
                                        }
                                    },
                                ));
                                crate::ui::menu::popup_at(items, event.page_position);
                            })
                            // 活动页签的状态点：绿色 `ok` token。未选中的页签不画点
                            // （以前活动页签用的是 `accent`——那是极浅的选中底色，
                            // 白底上看起来就是灰点，等于没有状态指示）。
                            .child(dynamic({
                                let current = current.clone();
                                move || -> AnyElement {
                                    if current() {
                                        return view()
                                            .w(px(6.0))
                                            .h(px(6.0))
                                            .flex_shrink_0()
                                            .rounded(px(3.0))
                                            .bg(move || design::ok())
                                            .into_element();
                                    }
                                    return view().w(px(0.0)).h(px(0.0)).into_element();
                                }
                            }))
                            .child(
                                text(move || tab.get().label())
                                    .text_size(px(ui::BODY))
                                    .line_height(px(ui::BODY_LINE))
                                    .number_of_lines(1)
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
                                    // 悬停整块页签即显示 ×（`on_mouse_enter` 在指针进入
                                    // 该视图或其内部视图时触发，不必精确命中 14pt 的 ×）；
                                    // 其余状态：活动页签 0.35、非活动页签 0.55。活动页签
                                    // 一开始是画板的 0（完全不可见），但那样一旦指针事件
                                    // 不达（例如窗口刚激活、指针没有移动），关闭入口就彻底
                                    // 消失——留 0.35 既有状态层次又不会「找不到 ×」。
                                    .opacity(move || {
                                        if hovered.get() {
                                            1.0_f32
                                        } else if current() {
                                            0.35_f32
                                        } else {
                                            0.55_f32
                                        }
                                    })
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
                            .child(dynamic(move || -> AnyElement {
                                let boundary = drop_boundary_for_item.get();
                                let position = index.get();
                                let count = tab_count.get();
                                let at_start = boundary == Some(position);
                                let at_end = position + 1 == count && boundary == Some(count);
                                if !at_start && !at_end {
                                    return view().w(px(0.0)).h(px(0.0)).into_element();
                                }
                                let marker = view()
                                    .absolute()
                                    .top(px(4.0))
                                    .w(px(2.0))
                                    .h(px(19.0))
                                    .rounded(px(1.0))
                                    .z_index(2)
                                    .bg(move || design::token("primary-bg"));
                                return if at_start {
                                    marker.left(px(0.0)).into_element()
                                } else {
                                    marker.right(px(0.0)).into_element()
                                };
                            }))
                    },
                )),
        );
}

fn tab_drop_destination(
    source: usize,
    origin: f32,
    delta: f32,
    order: &[u64],
    measured: &[(u64, f32)],
) -> usize {
    let width = |position: usize| {
        measured
            .iter()
            .find(|(key, _)| order.get(position) == Some(key))
            .map(|(_, width)| *width)
            .unwrap_or(120.0)
    };
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
            let threshold =
                -(origin + (target + 1..source).map(width).sum::<f32>() + width(target) / 2.0);
            if delta < threshold {
                destination = target;
            }
        }
    }
    return destination;
}

fn tab_drop_boundary(source: usize, destination: usize) -> Option<usize> {
    if destination == source {
        return None;
    }
    return Some(if destination < source {
        destination
    } else {
        destination + 1
    });
}

#[cfg(test)]
mod tests {
    use super::{tab_drop_boundary, tab_drop_destination};

    #[test]
    fn tab_drop_destination_tracks_crossed_tab_midpoints() {
        let order = [10, 20, 30, 40];
        let widths = [(10, 100.0), (20, 100.0), (30, 100.0), (40, 100.0)];
        assert_eq!(tab_drop_destination(1, 20.0, 129.0, &order, &widths), 1);
        assert_eq!(tab_drop_destination(1, 20.0, 131.0, &order, &widths), 2);
        assert_eq!(tab_drop_destination(2, 50.0, -101.0, &order, &widths), 1);
        assert_eq!(tab_drop_destination(2, 50.0, -199.0, &order, &widths), 1);
        assert_eq!(tab_drop_destination(2, 50.0, -201.0, &order, &widths), 0);
    }

    #[test]
    fn tab_drop_boundary_marks_the_insertion_side_of_the_target() {
        assert_eq!(tab_drop_boundary(1, 2), Some(3));
        assert_eq!(tab_drop_boundary(2, 1), Some(1));
        assert_eq!(tab_drop_boundary(1, 1), None);
    }
}
