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
                        .text_size(px(12.5))
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
                RefsTab::Reflog => reflog_list(language, tab.clone(), now, actions.clone())
                    .into_element(),
            }
        }));
}

fn tab_button(
    language: Signal<SharedString>,
    which: RefsTab,
    selected: Signal<RefsTab>,
    set_selected: std::sync::Arc<dyn Fn(RefsTab)>,
) -> impl IntoElement {
    return view()
        .h(px(20.0))
        .px(px(8.0))
        .items_center()
        .rounded(px(4.0))
        .bg(move || {
            if selected.get() == which {
                design::selected()
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .on_press(move || set_selected(which))
        .child(
            text(move || i18n::t(language.get().as_str(), which.key()))
                .text_size(px(11.0))
                .line_height(px(14.0))
                .text_color(move || {
                    if selected.get() == which {
                        design::text()
                    } else {
                        design::text2()
                    }
                }),
        );
}

fn refs_table(
    language: Signal<SharedString>,
    tab: Tab,
    actions: Actions,
) -> impl IntoElement {
    let drop_tag = actions.drop_tag.clone();
    let drop_remote = actions.drop_remote.clone();
    let drop_stash = actions.drop_stash.clone();
    let mut sections: Vec<AnyElement> = Vec::new();

    let branch_rows: Vec<AnyElement> = tab
        .refs
        .branches
        .iter()
        .map(|branch| {
            let track = if branch.ahead == 0 && branch.behind == 0 {
                String::new()
            } else {
                format!("↑{} ↓{}", branch.ahead, branch.behind)
            };
            return ref_row(
                branch.name.clone(),
                if branch.current {
                    "shell.branch.current".to_string()
                } else {
                    String::new()
                },
                track,
                design::branch_fg,
            );
        })
        .collect();
    sections.push(section(
        language,
        "sidebar.branches",
        tab.refs.branches.len(),
        branch_rows,
    ));

    let remote_rows: Vec<AnyElement> = tab
        .refs
        .remotes
        .iter()
        .map(|remote| {
            let drop_remote = drop_remote.clone();
            let name = remote.clone();
            return ref_row_with_menu(
                language,
                remote.clone(),
                String::new(),
                String::new(),
                design::text2,
                MenuAction {
                    label_key: "form.dropRemote",
                    name: name.clone(),
                    run: Box::new(move || drop_remote(name.clone())),
                },
            );
        })
        .collect();
    sections.push(section(
        language,
        "sidebar.remotes",
        tab.refs.remotes.len(),
        remote_rows,
    ));

    let tag_rows: Vec<AnyElement> = tab
        .refs
        .tags
        .iter()
        .map(|tag| {
            let drop_tag = drop_tag.clone();
            let name = tag.clone();
            ref_row_with_menu(
                language,
                tag.clone(),
                String::new(),
                String::new(),
                design::tag_fg,
                MenuAction {
                    label_key: "form.dropTag",
                    name: name.clone(),
                    run: Box::new(move || drop_tag(name.clone())),
                },
            )
        })
        .collect();
    sections.push(section(
        language,
        "sidebar.tags",
        tab.refs.tags.len(),
        tag_rows,
    ));

    let stash_rows: Vec<AnyElement> = tab
        .refs
        .stashes
        .iter()
        .map(|stash| {
            let drop_stash = drop_stash.clone();
            let reference = stash.reference.clone();
            return ref_row_with_menu(
                language,
                stash.reference.clone(),
                stash.summary.clone(),
                String::new(),
                design::text2,
                MenuAction {
                    label_key: "form.dropStash",
                    name: reference.clone(),
                    run: Box::new(move || drop_stash(reference.clone())),
                },
            );
        })
        .collect();
    sections.push(section(
        language,
        "sidebar.stashes",
        tab.refs.stashes.len(),
        stash_rows,
    ));

    let worktree_rows: Vec<AnyElement> = tab
        .refs
        .worktrees
        .iter()
        .map(|path| ref_row(path.clone(), String::new(), String::new(), design::text2))
        .collect();
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
                        .text_size(px(10.0))
                        .line_height(px(14.0))
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
                        .text_size(px(10.5))
                        .font_weight(FontWeight(600))
                        .line_height(px(14.0))
                        .text_color(move || design::text2()),
                )
                .child(
                    text(count.to_string())
                        .text_size(px(9.5))
                        .text_color(move || design::text3()),
                ),
        )
        .child(rows)
        .into_element();
}

fn ref_row(name: String, note: String, track: String, color: fn() -> AdaptiveColor) -> AnyElement {
    return view()
        .w_full()
        .h(px(28.0))
        .px(px(6.0))
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .rounded(px(4.0))
        .hover(|style| style.bg(move || design::hover()))
        .child(
            text(name)
                .w(px(220.0))
                .text_size(px(11.5))
                .line_height(px(15.0))
                .text_color(move || color())
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        )
        .child(
            text(note)
                .flex_1()
                .text_size(px(10.0))
                .line_height(px(14.0))
                .text_color(move || design::text3())
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        )
        .child(
            text(track)
                .text_size(px(9.5))
                .text_color(move || design::text3()),
        )
        .into_element();
}

fn reflog_list(
    language: Signal<SharedString>,
    tab: Tab,
    now: i64,
    actions: Actions,
) -> impl IntoElement {
    let undo = actions.undo_reflog.clone();
    let mut rows: Vec<AnyElement> = Vec::new();
    for (index, entry) in tab.reflog.iter().enumerate() {
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
                        .text_size(px(10.0))
                        .font_family("SF Mono")
                        .line_height(px(14.0))
                        .text_color(move || design::text3()),
                )
                .child(
                    text(action)
                        .w(px(80.0))
                        .text_size(px(10.5))
                        .line_height(px(14.0))
                        .text_color(move || design::accent()),
                )
                .child(
                    text(summary)
                        .flex_1()
                        .text_size(px(11.0))
                        .line_height(px(14.0))
                        .text_color(move || design::text())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .child(
                    text(move || {
                        i18n::relative_time(language.get().as_str(), timestamp, now)
                    })
                    .text_size(px(9.5))
                    .text_color(move || design::text3()),
                )
                .child(
                    view()
                        .h(px(18.0))
                        .px(px(6.0))
                        .items_center()
                        .rounded(px(4.0))
                        .hover(|style| style.bg(move || design::panel()))
                        .on_press(move || undo(index))
                        .child(
                            text(move || i18n::t(language.get().as_str(), "reflog.undo"))
                                .text_size(px(9.5))
                                .line_height(px(12.0))
                                .text_color(move || design::text2()),
                        ),
                )
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
                    text(move || i18n::t(language.get().as_str(), "reflog.hint"))
                        .text_size(px(10.0))
                        .line_height(px(14.0))
                        .text_color(move || design::text3()),
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
    return view()
        .h(px(18.0))
        .px(px(6.0))
        .items_center()
        .rounded(px(4.0))
        .border_1()
        .border_color(move || design::line_soft())
        .hover(|style| style.bg(move || design::hover()))
        .on_press(on_press)
        .child(
            text(move || i18n::t(language.get().as_str(), key))
                .text_size(px(9.5))
                .line_height(px(12.0))
                .text_color(move || design::text2()),
        );
}

/// 行内的删除动作。
struct MenuAction {
    label_key: &'static str,
    name: String,
    run: Box<dyn Fn()>,
}

/// 带右键删除的引用行（标签、远程、贮藏）。
fn ref_row_with_menu(
    language: Signal<SharedString>,
    name: String,
    note: String,
    track: String,
    color: fn() -> AdaptiveColor,
    action: MenuAction,
) -> AnyElement {
    let label = format!("{} {}", i18n::t(language.get_untracked().as_str(), action.label_key), action.name);
    let run = std::sync::Arc::new(action.run);
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
            let run = run.clone();
            let items = vec![
                MenuItem::new(i18n::t(language.get().as_str(), "form.drop"))
                    .on_select(move || run()),
            ];
            let _ = nsh_core::element::desktop::popup_menu_at(items, event.page_position);
        })
        .child(
            text(name)
                .w(px(220.0))
                .text_size(px(11.5))
                .line_height(px(15.0))
                .text_color(move || color())
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        )
        .child(
            text(note)
                .flex_1()
                .text_size(px(10.0))
                .line_height(px(14.0))
                .text_color(move || design::text3())
                .text_overflow(nsh_core::EllipsizeMode::Tail),
        )
        .child(
            text(track)
                .text_size(px(9.5))
                .text_color(move || design::text3()),
        )
        .child(
            text(label)
                .text_size(px(9.0))
                .line_height(px(12.0))
                .text_color(move || design::text3()),
        )
        .into_element();
}
