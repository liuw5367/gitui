//! 侧栏：Branches / Remotes / Tags / Stashes / Submodules / Worktrees。
//!
//! 行高 25、分组标题 27，均来自设计稿；计数与条目都来自真实仓库，没有内容的分组
//! 显示 0 而不是伪造行。
//!
//! 侧栏数据是打开标签页时的快照：仓库变化由上层重新装载标签页触发重建。

use crate::design::{self, metrics};
use crate::i18n;
use crate::model::Tab;
use crate::ui;
use crate::widgets;
use nsh_core::prelude::*;

pub(crate) fn sidebar(
    language: Signal<SharedString>,
    tab: Tab,
    actions: crate::Actions,
) -> impl IntoElement {
    let refs = tab.refs;
    let mut groups: Vec<AnyElement> = Vec::new();

    let checkout = actions.checkout_branch.clone();
    let delete = actions.delete_branch.clone();
    let new_branch = actions.checkout_new_branch.clone();
    let merge = actions.merge_reference.clone();
    let branch_rows: Vec<AnyElement> = refs
        .branches
        .iter()
        .map(|branch| {
            branch_row(
                language,
                branch.name.clone(),
                branch.current,
                branch.ahead,
                branch.behind,
                checkout.clone(),
                delete.clone(),
                new_branch.clone(),
                merge.clone(),
            )
        })
        .collect();
    groups.push(group(
        language,
        "sidebar.branches",
        refs.branches.len(),
        branch_rows,
    ));

    let remote_rows: Vec<AnyElement> = refs
        .remotes
        .iter()
        .map(|remote| plain_row(remote.clone(), "globe", design::text2))
        .collect();
    groups.push(group(
        language,
        "sidebar.remotes",
        refs.remotes.len(),
        remote_rows,
    ));

    let tag_rows: Vec<AnyElement> = refs
        .tags
        .iter()
        .map(|tag| plain_row(tag.clone(), "tag", design::text2))
        .collect();
    groups.push(group(language, "sidebar.tags", refs.tags.len(), tag_rows));

    let stash_rows: Vec<AnyElement> = refs
        .stashes
        .iter()
        .map(|stash| plain_row(stash.summary.clone(), "archive", design::text2))
        .collect();
    groups.push(group(
        language,
        "sidebar.stashes",
        refs.stashes.len(),
        stash_rows,
    ));

    let submodule_rows: Vec<AnyElement> = refs
        .submodules
        .iter()
        .map(|path| plain_row(path.clone(), "folder", design::text2))
        .collect();
    groups.push(group(
        language,
        "sidebar.submodules",
        refs.submodules.len(),
        submodule_rows,
    ));

    let worktree_rows: Vec<AnyElement> = refs
        .worktrees
        .iter()
        .map(|path| plain_row(short_path(path), "folder", design::text2))
        .collect();
    groups.push(group(
        language,
        "sidebar.worktrees",
        refs.worktrees.len(),
        worktree_rows,
    ));

    return view()
        .w(px(metrics::SIDEBAR_W))
        .h_full()
        .flex_shrink_0()
        .flex_col()
        .bg(move || design::sidebar())
        .border_r_1()
        .border_color(move || design::line())
        .child(
            scroll().w_full().h_full().child(
                view()
                    .w_full()
                    .pt(px(metrics::SIDE_SCROLL_PAD.0))
                    .pb(px(metrics::SIDE_SCROLL_PAD.2))
                    .px(px(metrics::SIDE_SCROLL_PAD.1))
                    .flex_col()
                    .child(groups),
            ),
        );
}

/// 分组：标题行（箭头、名称、计数）加该组的行。
fn group(
    language: Signal<SharedString>,
    key: &'static str,
    count: usize,
    rows: Vec<AnyElement>,
) -> AnyElement {
    return view()
        .w_full()
        .flex_col()
        .child(
            view()
                .w_full()
                .h(px(metrics::SIDE_HEAD_H))
                .mt(px(8.0))
                .px(px(6.0))
                .flex_row()
                .items_center()
                .gap(px(5.0))
                .child(widgets::icon("chev-down", 10.0, design::text3))
                .child(
                    text(move || i18n::t(language.get().as_str(), key))
                        .text_size(px(crate::ui::SECONDARY))
                        .font_weight(FontWeight(600))
                        .text_color(move || design::text3()),
                )
                .child(view().flex_1())
                .child(
                    text(count.to_string())
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(move || design::text3()),
                ),
        )
        .child(rows)
        .into_element();
}

fn branch_row(
    language: Signal<SharedString>,
    name: String,
    current: bool,
    ahead: u32,
    behind: u32,
    checkout: std::sync::Arc<dyn Fn(String)>,
    delete: std::sync::Arc<dyn Fn(String)>,
    new_branch: std::sync::Arc<dyn Fn(String, String)>,
    merge: std::sync::Arc<dyn Fn(String)>,
) -> AnyElement {
    let checkout_name = name.clone();
    let delete_name = name.clone();
    let new_branch_name = name.clone();
    let merge_name = name.clone();
    let is_current = current;
    return view()
        .w_full()
        .h(px(metrics::SIDE_ROW_H))
        .px(px(7.0))
        .flex_row()
        .items_center()
        .gap(px(7.0))
        .rounded(px(metrics::RADIUS_CONTROL))
        .bg(move || {
            if current {
                design::selected()
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .hover(|style| style.bg(move || design::hover()))
        // 右键：切换分支 / 删除分支（当前分支的两项都禁用）。
        .on_context_menu(move |event: &nsh_core::ContextMenuEvent| {
            let checkout = checkout.clone();
            let delete = delete.clone();
            let new_branch = new_branch.clone();
            let merge = merge.clone();
            let checkout_name = checkout_name.clone();
            let delete_name = delete_name.clone();
            let new_branch_name = new_branch_name.clone();
            let merge_name = merge_name.clone();
            let items = vec![
                ui::menu::action(
                    i18n::t(language.get().as_str(), "branch.checkout"),
                    !is_current,
                    move || checkout(checkout_name.clone()),
                ),
                ui::menu::action(
                    i18n::t(language.get().as_str(), "branch.checkoutAsNew"),
                    true,
                    move || new_branch(new_branch_name.clone(), new_branch_name.clone()),
                ),
                ui::menu::divider(),
                ui::menu::action(
                    i18n::t(language.get().as_str(), "branch.mergeInto"),
                    !is_current,
                    move || merge(merge_name.clone()),
                ),
                ui::menu::divider(),
                ui::menu::danger(
                    i18n::t(language.get().as_str(), "branch.delete"),
                    !is_current,
                    move || delete(delete_name.clone()),
                ),
            ];
            let _ = nsh_core::element::desktop::popup_menu_at(items, event.page_position);
        })
        .child(widgets::icon(
            "branch",
            12.0,
            if current { design::accent } else { design::text3 },
        ))
        .child(
            text(name)
                .text_size(px(crate::ui::BODY))
                .text_color(move || design::text())
                .flex_1()
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        )
        .child(
            text(move || {
                let mut track = String::new();
                if ahead > 0 {
                    track.push_str(&format!("↑{}", ahead));
                }
                if behind > 0 {
                    track.push_str(&format!("↓{}", behind));
                }
                return track;
            })
            .text_size(px(crate::ui::SECONDARY))
            .text_color(move || design::text3()),
        )
        .into_element();
}

fn plain_row(label: String, icon: &'static str, color: fn() -> AdaptiveColor) -> AnyElement {
    return view()
        .w_full()
        .h(px(metrics::SIDE_ROW_H))
        .px(px(7.0))
        .flex_row()
        .items_center()
        .gap(px(7.0))
        .rounded(px(metrics::RADIUS_CONTROL))
        .hover(|style| style.bg(move || design::hover()))
        .child(widgets::icon(icon, 11.0, color))
        .child(
            text(label)
                .text_size(px(crate::ui::BODY))
                .text_color(move || design::text2())
                .flex_1()
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        )
        .into_element();
}

fn short_path(path: &str) -> String {
    return path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_string();
}
