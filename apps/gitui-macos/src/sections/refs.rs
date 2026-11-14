//! Refs manager sheet.

use crate::design;
use crate::widgets::{header_button, mini_button};
use crate::RefsTab;
use gitui_git::{Branch, Tag};
use nsh_base::input;
use nsh_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct RefsActions {
    pub(crate) close: Arc<dyn Fn()>,
    pub(crate) checkout: Arc<dyn Fn(String)>,
    pub(crate) delete_branch: Arc<dyn Fn(String)>,
    pub(crate) branch_name: Arc<dyn Fn()>,
    pub(crate) create_tag: Arc<dyn Fn()>,
    pub(crate) delete_tag: Arc<dyn Fn()>,
}

fn nav_item(
    label: &'static str,
    target: RefsTab,
    tab: Signal<RefsTab>,
    on_tab: Arc<dyn Fn(RefsTab)>,
) -> impl IntoElement {
    view()
        .h(px(28.0))
        .px_2()
        .rounded(px(6.0))
        .items_center()
        .bg(move || {
            if tab.get() == target {
                design::primary()
            } else {
                design::sidebar()
            }
        })
        .on_press(move || on_tab(target))
        .child(
            text(label)
                .text_size(px(11.0))
                .text_color(move || {
                    if tab.get() == target {
                        design::primary_foreground()
                    } else {
                        design::text()
                    }
                }),
        )
}

pub(crate) fn refs_panel(
    open: Signal<bool>,
    tab: Signal<RefsTab>,
    branches: Signal<Vec<Branch>>,
    tags: Signal<Vec<Tag>>,
    branch_name: Signal<String>,
    tag_name: Signal<String>,
    actions: RefsActions,
) -> impl IntoElement {
    view()
        .absolute()
        .inset(px(0.0))
        .w_full()
        .h_full()
        .z_index(move || if open.get() { 47 } else { -1 })
        .items_center()
        .justify_center()
        .p(px(24.0))
        .bg(move || {
            if open.get() {
                AdaptiveColor::from(rgb(0x000000)).opacity(0.28)
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .child(
            view()
                .w(px(760.0))
                .h(px(540.0))
                .flex_row()
                .rounded(px(12.0))
                .bg(move || design::content())
                .border_1()
                .border_color(move || design::line())
                .overflow_hidden()
                .child(
                    view()
                        .w(px(190.0))
                        .flex_col()
                        .gap_1()
                        .p_2()
                        .bg(move || design::sidebar())
                        .border_r_1()
                        .border_color(move || design::line())
                        .child(
                            text("Refs")
                                .text_size(px(14.0))
                                .text_color(move || design::text()),
                        )
                        .child(view().h(px(8.0)))
                        .child(nav_item(
                            "Branches",
                            RefsTab::Branches,
                            tab,
                            Arc::new(move |next| tab.set(next)),
                        ))
                        .child(nav_item(
                            "Tags",
                            RefsTab::Tags,
                            tab,
                            Arc::new(move |next| tab.set(next)),
                        ))
                        .child(view().flex_1())
                        .child(header_button("Close", false, {
                            let action = actions.close.clone();
                            move || action()
                        })),
                )
                .child(
                    view()
                        .flex_1()
                        .min_w_0()
                        .flex_col()
                        .p_3()
                        .gap_2()
                        .child(
                            view()
                                .h(move || {
                                    if tab.get() == RefsTab::Branches {
                                        px(420.0)
                                    } else {
                                        px(0.0)
                                    }
                                })
                                .flex_col()
                                .gap_2()
                                .child(
                                    text("Branches")
                                        .text_size(px(14.0))
                                        .text_color(move || design::text()),
                                )
                                .child(
                                    view()
                                        .flex_row()
                                        .gap_1()
                                        .child(
                                            input()
                                                .value(branch_name)
                                                .on_change(move |text: &str| {
                                                    branch_name.set(text.to_string())
                                                })
                                                .placeholder("branch name")
                                                .text_size(px(11.0))
                                                .flex_1(),
                                        )
                                        .child(mini_button("Create", true, {
                                            let action = actions.branch_name.clone();
                                            move || action()
                                        })),
                                )
                                .child(each(
                                    branches,
                                    |branch: &Branch| branch.name.clone(),
                                    move |branch, _| {
                                        let name = branch.get().name.clone();
                                        let checkout = actions.checkout.clone();
                                        let delete = actions.delete_branch.clone();
                                        view()
                                            .h(px(28.0))
                                            .flex_row()
                                            .items_center()
                                            .gap_2()
                                            .px_1()
                                            .rounded(px(5.0))
                                            .bg(move || {
                                                if branch.get().current {
                                                    design::selected()
                                                } else {
                                                    design::content()
                                                }
                                            })
                                            .child(
                                                text(move || branch.get().name)
                                                    .flex_1()
                                                    .text_size(px(11.0))
                                                    .text_color(move || design::text()),
                                            )
                                            .child(mini_button("Checkout", false, {
                                                let name = name.clone();
                                                move || checkout(name.clone())
                                            }))
                                            .child(mini_button("Delete", false, {
                                                let name = name.clone();
                                                move || delete(name.clone())
                                            }))
                                    },
                                )),
                        )
                        .child(
                            view()
                                .h(move || {
                                    if tab.get() == RefsTab::Tags {
                                        px(420.0)
                                    } else {
                                        px(0.0)
                                    }
                                })
                                .flex_col()
                                .gap_2()
                                .child(
                                    text("Tags")
                                        .text_size(px(14.0))
                                        .text_color(move || design::text()),
                                )
                                .child(
                                    view()
                                        .flex_row()
                                        .gap_1()
                                        .child(
                                            input()
                                                .value(tag_name)
                                                .on_change(move |text: &str| {
                                                    tag_name.set(text.to_string())
                                                })
                                                .placeholder("tag name")
                                                .text_size(px(11.0))
                                                .flex_1(),
                                        )
                                        .child(mini_button("Create", true, {
                                            let action = actions.create_tag.clone();
                                            move || action()
                                        }))
                                        .child(mini_button("Delete", false, {
                                            let action = actions.delete_tag.clone();
                                            move || action()
                                        })),
                                )
                                .child(each(
                                    tags,
                                    |tag: &Tag| tag.name.clone(),
                                    |tag, _| {
                                        view().h(px(28.0)).child(
                                            text(move || tag.get().name)
                                                .text_size(px(11.0))
                                                .text_color(move || design::text()),
                                        )
                                    },
                                )),
                        ),
                ),
        )
}
