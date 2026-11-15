//! 引用与追溯（设计板 09 / 30 / 17）：Refs 表与 Reflog 列表。
//!
//! 两张表共用同一个右侧主区，通过顶部页签切换：
//! - Refs：分支、远程、标签、贮藏、工作树的完整列表（名称、上游、追踪、说明），
//!   行高 28，与设计稿的 `.refs` 表一致；
//! - Reflog：最近的引用移动，提供「撤销到此处」（`reset --hard`，需确认）。
//!
//! 写操作（新建分支/标签、删除、添加远程、贮藏）需要输入与确认流程，本阶段只读 +
//! 撤销，其余按「尚未实现」标注。
use crate::design::{self, metrics};
use crate::i18n;
use crate::model::Tab;
use crate::Actions;
use crate::ui;
use nsh_core::element::desktop::MenuItem;
use nsh_core::prelude::*;
/// 页签。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RefsTab {
    Refs,
    Reflog,
}
impl RefsTab {
    fn key(self) -> &'static str {
        return match self {
            RefsTab::Refs => "refs.title",
            RefsTab::Reflog => "reflog.title",
        };
    }
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn refs_view(
    language: Signal<SharedString>,
    tab: Tab,
    reflog_query: Signal<String>,
    selected_tab: Signal<RefsTab>,
    set_tab: std::sync::Arc<dyn Fn(RefsTab)>,
    actions: Actions,
) -> impl IntoElement {
    let now = store::now_unix();
    return view()
        .w_full()
        .h_full()
        .flex_col()
        .bg(move || design::content())
        .child(
            view()
                .w_full()
                .h(px(metrics::HIST_BAR_H))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .px(px(12.0))
                .border_b_1()
                .border_color(move || design::line())
                .child(
                    text(move || i18n::t(language.get().as_str(), "refs.title"))
                        .text_size(px(crate::ui::TITLE))
                        .font_weight(FontWeight(600))
                        .line_height(px(16.0))
                        .text_color(move || design::text()),
                )
                .child(view().flex_1())
                .child(create_button(
                    language,
                    "form.newBranch",
                    {
                        let open_form = actions.open_form.clone();
                        move || open_form(crate::screens::FormKind::NewBranch)
                    },
                ))
                .child(create_button(
                    language,
                    "form.newTag",
                    {
                        let open_form = actions.open_form.clone();
                        move || open_form(crate::screens::FormKind::NewTag)
                    },
                ))
                .child(create_button(
                    language,
                    "form.addRemote",
                    {
                        let open_form = actions.open_form.clone();
                        move || open_form(crate::screens::FormKind::AddRemote)
                    },
                ))
                .child(create_button(
                    language,
                    "form.newStash",
                    {
                        let open_form = actions.open_form.clone();
                        move || open_form(crate::screens::FormKind::NewStash)
                    },
                ))
                .child(tab_button(
                    language,
                    RefsTab::Refs,
                    selected_tab,
                    set_tab.clone(),
                ))
                .child(tab_button(
                    language,
                    RefsTab::Reflog,
                    selected_tab,
                    set_tab.clone(),
                )),
        )
        .child(dynamic(move || -> AnyElement {
            match selected_tab.get() {
                RefsTab::Refs => refs_table(language, tab.clone(), actions.clone()).into_element(),
                RefsTab::Reflog => {
                    reflog_list(
                        language,
                        tab.clone(),
                        now,
                        reflog_query,
                        actions.clone(),
                    )
                    .into_element()
                }
            }
        }));
}
fn tab_button(
    language: Signal<SharedString>,
    which: RefsTab,
    selected: Signal<RefsTab>,
    set_selected: std::sync::Arc<dyn Fn(RefsTab)>,
) -> impl IntoElement {
    // 页签：选中的用 Secondary 表达，未选中用 Ghost。
    let variant = if selected.get_untracked() == which {
        crate::ui::button::ButtonVariant::Secondary
    } else {
        crate::ui::button::ButtonVariant::Ghost
    };
    return crate::ui::button::button(
        move || i18n::t(language.get().as_str(), which.key()),
        variant,
        crate::ui::button::ButtonSize::Small,
        false,
        move || set_selected(which),
    );
}
fn refs_table(language: Signal<SharedString>, tab: Tab, actions: Actions) -> impl IntoElement {
    let mut sections: Vec<AnyElement> = Vec::new();
    // 分支：检出、检出为新本地分支、比较、合并、重命名、删除、复制。
    let mut branch_rows: Vec<AnyElement> = Vec::new();
    for (index, branch) in tab.refs.branches.iter().enumerate() {
        let name = branch.name.clone();
        let track = if branch.ahead == 0 && branch.behind == 0 {
            String::new()
        } else {
            format!("↑{} ↓{}", branch.ahead, branch.behind)
        };
        let note = if branch.current {
            i18n::t(language.get_untracked().as_str(), "shell.branch.current")
        } else {
            String::new()
        };
        let is_current = branch.current;
        let checkout = actions.checkout_branch.clone();
        let checkout_new = actions.checkout_new_branch.clone();
        let merge = actions.merge_reference.clone();
        let compare = actions.set_compare_refs.clone();
        let open_form = actions.open_form_from.clone();
        let delete = actions.delete_branch.clone();
        let copy_text = actions.copy_text.clone();
        let name_for_menu = name.clone();
        let items = vec![
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "branch.checkout"),
                !is_current,
                {
                    let checkout = checkout.clone();
                    let name = name_for_menu.clone();
                    move || checkout(name.clone())
                },
            ),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "branch.checkoutAsNew"),
                true,
                {
                    let checkout_new = checkout_new.clone();
                    let name = name_for_menu.clone();
                    move || checkout_new(name.clone(), name.clone())
                },
            ),
            ui::menu::divider(),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.openCompare"),
                true,
                {
                    let compare = compare.clone();
                    let name = name_for_menu.clone();
                    move || compare(None, Some(name.clone()))
                },
            ),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "branch.mergeInto"),
                !is_current,
                {
                    let merge = merge.clone();
                    let name = name_for_menu.clone();
                    move || {
                        let _ = merge(name.clone());
                    }
                },
            ),
            ui::menu::divider(),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "branch.rename"),
                true,
                {
                    let open_form = open_form.clone();
                    let name = name_for_menu.clone();
                    move || open_form(crate::screens::FormKind::RenameBranch, name.clone())
                },
            ),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.copyName"),
                true,
                {
                    let copy_text = copy_text.clone();
                    let name = name_for_menu.clone();
                    move || copy_text(name.clone())
                },
            ),
            ui::menu::divider(),
            ui::menu::danger(
                i18n::t(language.get_untracked().as_str(), "branch.delete"),
                !is_current,
                {
                    let delete = delete.clone();
                    let name = name_for_menu;
                    move || delete(name.clone())
                },
            ),
        ];
        branch_rows.push(menu_row(
            language,
            name,
            note,
            track,
            design::branch_fg,
            items,
            index,
        ));
    }
    // 远程：编辑 URL、取回、移除、复制。
    let mut remote_rows: Vec<AnyElement> = Vec::new();
    for (index, remote) in tab.refs.remotes.iter().enumerate() {
        let name = remote.clone();
        let drop_remote = actions.drop_remote.clone();
        let fetch = actions.open_sync.clone();
        let copy_text = actions.copy_text.clone();
        let edit_form = actions.open_form_from.clone();
        let url = tab
            .remote_urls
            .iter()
            .find(|(remote_name, _)| remote_name == &name)
            .map(|(_, url)| url.clone())
            .unwrap_or_default();
        let items = vec![
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.editRemoteUrl"),
                true,
                {
                    let edit_form = edit_form.clone();
                    let name = name.clone();
                    move || {
                        edit_form(crate::screens::FormKind::EditRemote, name.clone());
                    }
                },
            ),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "sync.fetch"),
                true,
                {
                    let fetch = fetch.clone();
                    move || fetch(crate::model::Sync::Fetch)
                },
            ),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.copyUrl"),
                true,
                {
                    let copy_text = copy_text.clone();
                    let url = url.clone();
                    move || copy_text(url.clone())
                },
            ),
            ui::menu::divider(),
            ui::menu::danger(
                i18n::t(language.get_untracked().as_str(), "form.dropRemote"),
                true,
                {
                    let drop_remote = drop_remote.clone();
                    let name = name.clone();
                    move || drop_remote(name.clone())
                },
            ),
        ];
        remote_rows.push(menu_row(
            language,
            name,
            url,
            String::new(),
            design::text2,
            items,
            index,
        ));
    }
    // 标签：推送、删除、复制。
    let mut tag_rows: Vec<AnyElement> = Vec::new();
    for (index, tag) in tab.refs.tags.iter().enumerate() {
        let name = tag.clone();
        let push = actions.push_tag.clone();
        let drop_tag = actions.drop_tag.clone();
        let copy_text = actions.copy_text.clone();
        let items = vec![
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.pushTag"),
                true,
                {
                    let push = push.clone();
                    let name = name.clone();
                    move || push(name.clone())
                },
            ),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.copyName"),
                true,
                {
                    let copy_text = copy_text.clone();
                    let name = name.clone();
                    move || copy_text(name.clone())
                },
            ),
            ui::menu::divider(),
            ui::menu::danger(
                i18n::t(language.get_untracked().as_str(), "form.dropTag"),
                true,
                {
                    let drop_tag = drop_tag.clone();
                    let name = name.clone();
                    move || drop_tag(name.clone())
                },
            ),
        ];
        tag_rows.push(menu_row(
            language,
            name,
            String::new(),
            String::new(),
            design::tag_fg,
            items,
            index,
        ));
    }
    // 贮藏：应用、弹出、丢弃、复制引用。
    let mut stash_rows: Vec<AnyElement> = Vec::new();
    for (index, stash) in tab.refs.stashes.iter().enumerate() {
        let reference = stash.reference.clone();
        let summary = stash.summary.clone();
        let apply = actions.apply_stash.clone();
        let pop = actions.pop_stash.clone();
        let drop_stash = actions.drop_stash.clone();
        let copy_text = actions.copy_text.clone();
        let items = vec![
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.stashApply"),
                true,
                {
                    let apply = apply.clone();
                    let reference = reference.clone();
                    move || apply(reference.clone())
                },
            ),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.stashPop"),
                true,
                {
                    let pop = pop.clone();
                    let reference = reference.clone();
                    move || pop(reference.clone())
                },
            ),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.copyName"),
                true,
                {
                    let copy_text = copy_text.clone();
                    let reference = reference.clone();
                    move || copy_text(reference.clone())
                },
            ),
            ui::menu::divider(),
            ui::menu::danger(
                i18n::t(language.get_untracked().as_str(), "form.dropStash"),
                true,
                {
                    let drop_stash = drop_stash.clone();
                    let reference = reference.clone();
                    move || drop_stash(reference.clone())
                },
            ),
        ];
        stash_rows.push(menu_row(
            language,
            reference,
            summary,
            String::new(),
            design::text2,
            items,
            index,
        ));
    }
    // 工作树：在 Finder 中显示、复制路径。
    let mut worktree_rows: Vec<AnyElement> = Vec::new();
    for (index, path) in tab.refs.worktrees.iter().enumerate() {
        let reveal = actions.reveal_path.clone();
        let copy_text = actions.copy_text.clone();
        let path_for_menu = path.clone();
        let items = vec![
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.reveal"),
                true,
                {
                    let reveal = reveal.clone();
                    let path = path_for_menu.clone();
                    move || reveal(path.clone())
                },
            ),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "menu.copyPath"),
                true,
                {
                let copy_text = copy_text.clone();
                let path = path_for_menu.clone();
                move || copy_text(path.clone())
            }),
        ];
        worktree_rows.push(menu_row(
            language,
            path.clone(),
            String::new(),
            String::new(),
            design::text2,
            items,
            index,
        ));
    }
    sections.push(section(
        language,
        "sidebar.branches",
        tab.refs.branches.len(),
        branch_rows,
    ));
    sections.push(section(
        language,
        "sidebar.remotes",
        tab.refs.remotes.len(),
        remote_rows,
    ));
    sections.push(section(
        language,
        "sidebar.tags",
        tab.refs.tags.len(),
        tag_rows,
    ));
    sections.push(section(
        language,
        "sidebar.stashes",
        tab.refs.stashes.len(),
        stash_rows,
    ));
    sections.push(section(
        language,
        "sidebar.worktrees",
        tab.refs.worktrees.len(),
        worktree_rows,
    ));
    return view().w_full().h_full().child(
        scroll().w_full().h_full().child(
            view()
                .w_full()
                .py(px(8.0))
                .px(px(12.0))
                .flex_col()
                .child(
                    text(move || i18n::t(language.get().as_str(), "refs.writeHint"))
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(crate::ui::SECONDARY_LINE))
                        .text_color(move || design::text3()),
                )
                .child(sections),
        ),
    );
}
fn section(
    language: Signal<SharedString>,
    key: &'static str,
    count: usize,
    rows: Vec<AnyElement>,
) -> AnyElement {
    return view()
        .w_full()
        .mt(px(10.0))
        .flex_col()
        .child(
            view()
                .w_full()
                .h(px(24.0))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(move || i18n::t(language.get().as_str(), key))
                        .text_size(px(crate::ui::SECONDARY))
                        .font_weight(FontWeight(600))
                        .line_height(px(14.0))
                        .text_color(move || design::text2()),
                )
                .child(
                    text(count.to_string())
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(move || design::text3()),
                ),
        )
        .child(rows)
        .into_element();
}
fn reflog_list(
    language: Signal<SharedString>,
    tab: Tab,
    now: i64,
    query: Signal<String>,
    actions: Actions,
) -> impl IntoElement {
    let undo = actions.undo_reflog.clone();
    let needle = query.get_untracked().to_lowercase();
    let mut rows: Vec<AnyElement> = Vec::new();
    for (index, entry) in tab.reflog.iter().enumerate() {
        // 关键字同时匹配动作与说明，空关键字不过滤。
        if !needle.is_empty()
            && !entry.action.to_lowercase().contains(&needle)
            && !entry.summary.to_lowercase().contains(&needle)
            && !entry.short.contains(&needle)
        {
            continue;
        }
        let undo = undo.clone();
        let short = entry.short.clone();
        let action = entry.action.clone();
        let summary = entry.summary.clone();
        let timestamp = entry.timestamp;
        rows.push(
            view()
                .w_full()
                .h(px(28.0))
                .px(px(6.0))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .rounded(px(4.0))
                .hover(|style| style.bg(move || design::hover()))
                .child(
                    text(short)
                        .w(px(64.0))
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(14.0))
                        .text_color(move || design::text3()),
                )
                .child(
                    text(action)
                        .w(px(80.0))
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(crate::ui::SECONDARY_LINE))
                        .text_color(move || design::accent()),
                )
                .child(
                    text(summary)
                        .flex_1()
                        .text_size(px(crate::ui::BODY))
                        .line_height(px(14.0))
                        .text_color(move || design::text())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .child(
                    text(move || {
                        crate::dates::format(language.get().as_str(), timestamp, now)
                    })
                    .text_size(px(crate::ui::SECONDARY))
                    .text_color(move || design::text3()),
                )
                .child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "reflog.undo"),
                    crate::ui::button::ButtonVariant::Ghost,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || undo(index),
                ))
                .on_context_menu({
                    let undo = actions.undo_reflog.clone();
                    let open_form = actions.open_form_from.clone();
                    let show = actions.show_in_history.clone();
                    let copy_text = actions.copy_text.clone();
                    let sha = entry.sha.clone();
                    let short = entry.short.clone();
                    move |event: &nsh_core::ContextMenuEvent| {
                        let lang = language.get_untracked();
                        let lang = lang.as_str();
                        let items = vec![
                            ui::menu::action(
                                i18n::t(lang, "reflog.undo"),
                                true,
                                {
                                    let undo = undo.clone();
                                    move || undo(index)
                                },
                            ),
                            ui::menu::action(
                                i18n::t(lang, "menu.newBranchFromHere"),
                                true,
                                {
                                let open_form = open_form.clone();
                                let sha = sha.clone();
                                move || {
                                    open_form(
                                        crate::screens::FormKind::NewBranch,
                                        sha.clone(),
                                    )
                                }
                            }),
                            ui::menu::action(
                                i18n::t(lang, "menu.showInHistory"),
                                true,
                                {
                                let show = show.clone();
                                let sha = sha.clone();
                                move || show(sha.clone())
                            }),
                            ui::menu::divider(),
                            ui::menu::action(
                                i18n::t(lang, "menu.copySha"),
                                true,
                                {
                                let copy_text = copy_text.clone();
                                let sha = sha.clone();
                                move || copy_text(sha.clone())
                            }),
                            ui::menu::action(
                                i18n::t(lang, "menu.copyName"),
                                true,
                                {
                                let copy_text = copy_text.clone();
                                let short = short.clone();
                                move || copy_text(short.clone())
                            }),
                        ];
                        let _ = nsh_core::element::desktop::popup_menu_at(
                            items,
                            event.page_position,
                        );
                    }
                })
                .into_element(),
        );
    }
    return view().w_full().h_full().child(
        scroll().w_full().h_full().child(
            view()
                .w_full()
                .py(px(8.0))
                .px(px(12.0))
                .flex_col()
                .child(
                    view()
                        .w_full()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            text(move || i18n::t(language.get().as_str(), "reflog.hint"))
                                .text_size(px(crate::ui::SECONDARY))
                                .line_height(px(crate::ui::SECONDARY_LINE))
                                .text_color(move || design::text3()),
                        )
                        .child(view().flex_1())
                        .child(
                            view()
                                .w(px(180.0))
                                .h(px(metrics::SEARCH_H))
                                .px(px(7.0))
                                .items_center()
                                .rounded(px(5.0))
                                .border_1()
                                .border_color(move || design::field_border())
                                .bg(move || design::field())
                                .child(
                                    nsh_base::input()
                                        .value(query)
                                        .on_change(move |text: &str| query.set(text.to_string()))
                                        .placeholder(move || {
                                            i18n::t(
                                                language.get().as_str(),
                                                "reflog.filterHint",
                                            )
                                        })
                                        .text_size(px(crate::ui::SECONDARY))
                                        .flex_1(),
                                ),
                        ),
                )
                .child(view().w_full().h(px(1.0)).bg(move || design::line()))
                .child(rows),
        ),
    );
}
/// 创建入口按钮（工具条右侧）。
fn create_button(
    language: Signal<SharedString>,
    key: &'static str,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    return crate::ui::button::button(
        move || i18n::t(language.get().as_str(), key),
        crate::ui::button::ButtonVariant::Ghost,
        crate::ui::button::ButtonSize::Small,
        false,
        on_press,
    );
}
/// 行内的删除动作。
/// 一行带右键菜单的引用。`items` 由各分段自己组装，顺序即菜单顺序。
#[allow(clippy::too_many_arguments)]
fn menu_row(
    language: Signal<SharedString>,
    name: String,
    note: String,
    track: String,
    color: fn() -> AdaptiveColor,
    items: Vec<MenuItem>,
    _index: usize,
) -> AnyElement {
    return view()
        .w_full()
        .h(px(28.0))
        .px(px(6.0))
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .rounded(px(4.0))
        .hover(|style| style.bg(move || design::hover()))
        .on_context_menu(move |event: &nsh_core::ContextMenuEvent| {
            let _ = nsh_core::element::desktop::popup_menu_at(
                items.clone(),
                event.page_position,
            );
        })
        .child(
            text(name)
                .w(px(220.0))
                .text_size(px(crate::ui::BODY))
                .line_height(px(15.0))
                .text_color(move || color())
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        )
        .child(
            text(note)
                .flex_1()
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3())
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        )
        .child(
            text(track)
                .text_size(px(crate::ui::SECONDARY))
                .text_color(move || design::text3()),
        )
        .child(
            text(move || i18n::t(language.get().as_str(), "common.menuHint"))
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        )
        .into_element();
}
