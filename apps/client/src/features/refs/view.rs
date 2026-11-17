//! 引用管理（设计板 09 / 30 / 17）：左侧侧栏 + 右侧 Refs 表与 Reflog 列表。
//!
//! 侧栏是 Branches / Remotes / Tags / Stashes / Submodules / Worktrees 六组，行高 25、
//! 分组标题 27；Refs 表是同一批数据的完整列表（名称、上游、追踪、说明），行高 28，
//! 与设计稿的 `.refs` 表一致；Reflog 是最近的引用移动，提供「撤销到此处」
//! （`reset --hard`，需确认）。
//!
//! 入口 `refs_sidebar` 画侧栏、`refs_view` 画主区，两者都收本功能的状态与用例；
//! 新建分支/标签的表单归 `features::search`，复制与在 Finder 中显示走 `shared::os`。

use super::commands::RefsCommands;
use super::state::{RefsState, RefsTab};
use crate::app::session::Tab;
use crate::design::{self, metrics};
use crate::features::search::commands::SearchCommands;
use crate::i18n;
use crate::ui;
use nsh_core::element::desktop::MenuItem;
use nsh_core::prelude::*;

pub(crate) fn refs_sidebar(
    _state: &RefsState,
    cmd: &RefsCommands,
    language: Signal<SharedString>,
    tab: Tab,
) -> impl IntoElement + use<> {
    let refs = tab.refs;
    let mut groups: Vec<AnyElement> = Vec::new();

    // 右键菜单的回调要 `Arc<dyn Fn>`，这里把本功能的用例包一层。
    let checkout: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |name: String| cmd.checkout_branch(name))
    };
    let delete: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |name: String| cmd.delete_branch(name))
    };
    let new_branch: std::sync::Arc<dyn Fn(String, String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |name: String, from: String| cmd.checkout_new_branch(name, from))
    };
    let merge: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |name: String| cmd.merge_reference(name))
    };
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
                .child(crate::ui::icon::icon_view(
                    "chev-down",
                    crate::ui::icon::IconSize::Compact,
                    design::text3,
                ))
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
            crate::ui::menu::popup_at(items, event.page_position);
        })
        .child(crate::ui::icon::icon_view(
            "branch",
            crate::ui::icon::IconSize::Button,
            if current {
                design::accent
            } else {
                design::text3
            },
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
        .child(crate::ui::icon::icon_view(
            icon,
            crate::ui::icon::IconSize::Compact,
            color,
        ))
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

pub(crate) fn refs_view(
    state: &RefsState,
    cmd: &RefsCommands,
    language: Signal<SharedString>,
    tab: Tab,
    search: &SearchCommands,
    selected_tab: Signal<RefsTab>,
) -> impl IntoElement + use<> {
    let now = store::now_unix();
    let reflog_query = state.reflog_query;
    // 表格在 `dynamic` 里重建，先克隆一份用例，闭包才拿得到 `'static` 数据。
    let table_cmd = cmd.clone();
    let table_search = search.clone();
    // 新建表单归搜索功能：这里只把入口包成按钮要的回调。
    let open_form: std::sync::Arc<dyn Fn(crate::features::search::FormKind)> = {
        let search = search.clone();
        std::sync::Arc::new(move |kind: crate::features::search::FormKind| search.open_form(kind))
    };
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
                .child(create_button(language, "form.newBranch", {
                    let open_form = open_form.clone();
                    move || open_form(crate::features::search::FormKind::NewBranch)
                }))
                .child(create_button(language, "form.newTag", {
                    let open_form = open_form.clone();
                    move || open_form(crate::features::search::FormKind::NewTag)
                }))
                .child(create_button(language, "form.addRemote", {
                    let open_form = open_form.clone();
                    move || open_form(crate::features::search::FormKind::AddRemote)
                }))
                .child(create_button(language, "form.newStash", {
                    let open_form = open_form.clone();
                    move || open_form(crate::features::search::FormKind::NewStash)
                }))
                .child(tab_button(language, RefsTab::Refs, selected_tab))
                .child(tab_button(language, RefsTab::Reflog, selected_tab)),
        )
        .child(dynamic(move || -> AnyElement {
            match selected_tab.get() {
                RefsTab::Refs => {
                    refs_table(language, tab.clone(), &table_search, &table_cmd).into_element()
                }
                RefsTab::Reflog => {
                    // 「撤销」归恢复功能（`cmd.recovery`），「在历史中显示」归提交
                    // 历史（`cmd.history`）：refs 只是这两个入口的宿主。
                    reflog_list(
                        language,
                        tab.clone(),
                        now,
                        reflog_query,
                        &table_cmd,
                        &table_search,
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
        move || selected.set(which),
    );
}
fn refs_table(
    language: Signal<SharedString>,
    tab: Tab,
    search: &SearchCommands,
    cmd: &RefsCommands,
) -> impl IntoElement {
    // 复制与在 Finder 中显示走 `shared::os`，表单入口走搜索功能；都包成回调供
    // 右键菜单使用。
    let notice = cmd.notice;
    let copy_text: std::sync::Arc<dyn Fn(String)> = {
        let language = language;
        std::sync::Arc::new(move |text: String| {
            crate::shared::os::copy_text(notice, language, text)
        })
    };
    let reveal_path: std::sync::Arc<dyn Fn(String)> = {
        let language = language;
        std::sync::Arc::new(move |path: String| {
            crate::shared::os::reveal_path(notice, language, path)
        })
    };
    let open_form_from: std::sync::Arc<dyn Fn(crate::features::search::FormKind, String)> = {
        let search = search.clone();
        std::sync::Arc::new(
            move |kind: crate::features::search::FormKind, from: String| {
                search.open_form_from(kind, from)
            },
        )
    };
    // 「比较」与各段引用行的写动作要 `Arc<dyn Fn>`，这里把用例包成回调。
    let compare_refs: std::sync::Arc<dyn Fn(Option<String>, Option<String>)> = {
        let history = cmd.history.clone();
        std::sync::Arc::new(move |base: Option<String>, target: Option<String>| {
            history.set_compare_refs(base, target)
        })
    };
    let checkout: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |name: String| cmd.checkout_branch(name))
    };
    let checkout_new: std::sync::Arc<dyn Fn(String, String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |name: String, from: String| cmd.checkout_new_branch(name, from))
    };
    let merge: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |name: String| cmd.merge_reference(name))
    };
    let delete: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |name: String| cmd.delete_branch(name))
    };
    let drop_remote: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |name: String| cmd.drop_remote(name))
    };
    let push_tag: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |name: String| cmd.push_tag(name))
    };
    let drop_tag: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |name: String| cmd.drop_tag(name))
    };
    let apply_stash: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |reference: String| cmd.apply_stash(reference))
    };
    let pop_stash: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |reference: String| cmd.pop_stash(reference))
    };
    let drop_stash: std::sync::Arc<dyn Fn(String)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |reference: String| cmd.drop_stash(reference))
    };
    // 远程行的「取回」打开同步面板：那是同步功能的入口，走它的用例。
    let fetch: std::sync::Arc<dyn Fn(crate::app::session::Sync)> = {
        let sync = cmd.sync.clone();
        std::sync::Arc::new(move |kind: crate::app::session::Sync| sync.open_sync(kind))
    };
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
        let checkout = checkout.clone();
        let checkout_new = checkout_new.clone();
        let merge = merge.clone();
        let compare = compare_refs.clone();
        let open_form = open_form_from.clone();
        let delete = delete.clone();
        let copy_text = copy_text.clone();
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
                    move || {
                        open_form(
                            crate::features::search::FormKind::RenameBranch,
                            name.clone(),
                        )
                    }
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
        let drop_remote = drop_remote.clone();
        let fetch = fetch.clone();
        let copy_text = copy_text.clone();
        let edit_form = open_form_from.clone();
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
                        edit_form(crate::features::search::FormKind::EditRemote, name.clone());
                    }
                },
            ),
            ui::menu::action(
                i18n::t(language.get_untracked().as_str(), "sync.fetch"),
                true,
                {
                    let fetch = fetch.clone();
                    move || fetch(crate::app::session::Sync::Fetch)
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
        let push = push_tag.clone();
        let drop_tag = drop_tag.clone();
        let copy_text = copy_text.clone();
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
        let apply = apply_stash.clone();
        let pop = pop_stash.clone();
        let drop_stash = drop_stash.clone();
        let copy_text = copy_text.clone();
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
        let reveal = reveal_path.clone();
        let copy_text = copy_text.clone();
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
                },
            ),
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
    cmd: &RefsCommands,
    search: &SearchCommands,
) -> impl IntoElement {
    let recovery = cmd.recovery.clone();
    let show_in_history = cmd.history.clone();
    let notice = cmd.notice;
    let open_form_from: std::sync::Arc<dyn Fn(crate::features::search::FormKind, String)> = {
        let search = search.clone();
        std::sync::Arc::new(
            move |kind: crate::features::search::FormKind, from: String| {
                search.open_form_from(kind, from)
            },
        )
    };
    let copy_text_fn: std::sync::Arc<dyn Fn(String)> = {
        let language = language;
        std::sync::Arc::new(move |text: String| {
            crate::shared::os::copy_text(notice, language, text)
        })
    };
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
        let undo = recovery.clone();
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
                    text(move || crate::dates::format(language.get().as_str(), timestamp, now))
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(move || design::text3()),
                )
                .child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "reflog.undo"),
                    crate::ui::button::ButtonVariant::Ghost,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || undo.undo_reflog(index),
                ))
                .on_context_menu({
                    let undo = recovery.clone();
                    let open_form = open_form_from.clone();
                    let show = show_in_history.clone();
                    let copy_text = copy_text_fn.clone();
                    let sha = entry.sha.clone();
                    let short = entry.short.clone();
                    move |event: &nsh_core::ContextMenuEvent| {
                        let lang = language.get_untracked();
                        let lang = lang.as_str();
                        let items = vec![
                            ui::menu::action(i18n::t(lang, "reflog.undo"), true, {
                                let undo = undo.clone();
                                move || undo.undo_reflog(index)
                            }),
                            ui::menu::action(i18n::t(lang, "menu.newBranchFromHere"), true, {
                                let open_form = open_form.clone();
                                let sha = sha.clone();
                                move || {
                                    open_form(
                                        crate::features::search::FormKind::NewBranch,
                                        sha.clone(),
                                    )
                                }
                            }),
                            ui::menu::action(i18n::t(lang, "menu.showInHistory"), true, {
                                let show = show.clone();
                                let sha = sha.clone();
                                move || show.show_in_history(sha.clone())
                            }),
                            ui::menu::divider(),
                            ui::menu::action(i18n::t(lang, "menu.copySha"), true, {
                                let copy_text = copy_text.clone();
                                let sha = sha.clone();
                                move || copy_text(sha.clone())
                            }),
                            ui::menu::action(i18n::t(lang, "menu.copyName"), true, {
                                let copy_text = copy_text.clone();
                                let short = short.clone();
                                move || copy_text(short.clone())
                            }),
                        ];
                        crate::ui::menu::popup_at(items, event.page_position);
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
                                .child(crate::ui::text_input::search_field(
                                    query,
                                    move || i18n::t(language.get().as_str(), "reflog.filterHint"),
                                    None,
                                    move |text| query.set(text.to_string()),
                                )),
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
            crate::ui::menu::popup_at(items.clone(), event.page_position);
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
