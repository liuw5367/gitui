//! 历史表：占满中栏可用高度，含工具条、筛选浮层、列标题与可滚动的提交行。
//!
//! 行内是设计稿的四列：图线 64、说明（含引用徽章）、作者 96、时间 58。工具条上的
//! 芯片显示当前筛选条件；点筛选图标打开浮层（设计板 05）。对比模式下点击提交是
//! 加入/移出对比集合（设计板 06）。
//!
//! 入口 `history` 收本功能的状态与用例；改写与创建初始提交走 `cmd.commit`，
//! 拣选/回滚/重置走 `cmd.recovery`，新建分支/标签的表单与复制走 `features::search`
//! 与 `shared::os`。

use super::commands::HistoryCommands;
use super::graph::{self, GraphRow};
use super::ops;
use super::state::HistoryState;
use crate::app::session::Tab;
use crate::design::{self, metrics};
use crate::features::commit::commands::CommitCommands;
use crate::features::refs::commands::RefsCommands;
use crate::features::search::commands::SearchCommands;
use crate::filters::When;
use crate::i18n;
use crate::ui::badge::kbd;
use nsh_core::element::desktop::MenuItem;
use nsh_core::prelude::*;

const AUTHOR_W: f32 = 96.0;
const DATE_W: f32 = 58.0;
const GRAPH_LANE_W: f32 = 10.0;

/// 浮层宽度（设计板 `.filter-pop`）。
const POP_W: f32 = 432.0;
const QUICK_FILTER_W: f32 = 320.0;

#[derive(Clone)]
struct FilterBranch {
    label: String,
    reference: String,
    remote: bool,
}

#[derive(Clone)]
struct CompareRef {
    label: String,
    value: String,
}

/// `use<>`：返回的元素只持有克隆出来的信号，不借用入参（edition 2024 的
/// `impl Trait` 默认捕获入参生命周期，这里显式声明不捕获）。
pub(crate) fn history(
    state: &HistoryState,
    cmd: &HistoryCommands,
    tab: Tab,
    search: &SearchCommands,
    refs: &RefsCommands,
) -> impl IntoElement + use<> {
    let language = cmd.language;
    let tab_id = tab.id;
    let commits = tab.commits.clone();
    let selected = tab.selected;
    let compare_refs_empty =
        tab.compare_base.is_some() && tab.compare_target.is_some() && tab.commits.is_empty();
    let graph_rows = graph::layout_filtered(&commits, &tab.graph_commits);
    let graph_width = graph_rows
        .iter()
        .map(|row| row.lane_count)
        .max()
        .unwrap_or(1) as f32
        * GRAPH_LANE_W;
    let compare = tab.compare.clone();
    // 视图回调要 `Arc<dyn Fn>`，这里把本功能的命令包一层。
    let select: std::sync::Arc<dyn Fn(usize)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |index: usize| cmd.select_commit(index))
    };
    let open_commit_diff: std::sync::Arc<dyn Fn(usize)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |index: usize| cmd.show_commit_diff(index))
    };
    let toggle_compare: std::sync::Arc<dyn Fn(usize)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |index: usize| cmd.toggle_compare(index))
    };
    let row_search = search.clone();
    // 提交功能的用例：右键菜单里的「改写说明 / 合并提交」与多选操作栏的「合并」。
    let commit_for_rows = cmd.commit.clone();
    let commit_for_bar = cmd.commit.clone();
    // 右键菜单里的「检出 / 合并」来自引用管理，克隆一份进 `'static` 回调。
    let refs_for_menu = refs.clone();
    // 拣选/回滚/重置归恢复功能，同样克隆一份进 `'static` 回调。
    let recovery_for_menu = cmd.recovery.clone();
    // 空仓库与「筛选后无结果」要区分：前者要给创建初始提交的入口。
    let loading = tab.loading;
    let unborn = matches!(tab.head, git::HeadState::Unborn { .. });
    let branch_for_empty = match &tab.head {
        git::HeadState::Unborn { branch } => branch.clone(),
        _ => tab.branch.clone(),
    };
    let initial_commit = {
        let commit = cmd.commit.clone();
        std::sync::Arc::new(move || commit.create_initial_commit())
    };
    // 合并提交的范围：取对比选区（至少两个），从旧到新列出来。
    let squash_range: Option<(String, String, Vec<(String, String)>)> =
        tab.compare_range().map(|(newest, oldest)| {
            let mut commits: Vec<(String, String)> = Vec::new();
            for index in oldest..=newest {
                if let Some(commit) = tab.commits.get(index) {
                    commits.push((commit.short.clone(), commit.summary.clone()));
                }
            }
            // `compare_range` 返回 (新, 旧)；squash 需要 (旧, 新)。
            let from = tab
                .commits
                .get(oldest)
                .map(|commit| commit.sha.clone())
                .unwrap_or_default();
            let to = tab
                .commits
                .get(newest)
                .map(|commit| commit.sha.clone())
                .unwrap_or_default();
            return (from, to, commits);
        });
    let can_squash = squash_range
        .as_ref()
        .map(|(_, _, commits)| commits.len() >= 2)
        .unwrap_or(false);
    let squash_range = squash_range.filter(|_| can_squash);
    let squash_range_for_bar = squash_range.clone();
    // 「清空选择」复用对比模式的取消：把 compare 集合清掉。
    let clear_selection: std::sync::Arc<dyn Fn()> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move || cmd.set_compare_mode(false))
    };
    let total = commits.len();
    let now = store::now_unix();
    let query = state.query;
    let visible = {
        let commits = commits.clone();
        computed(move || {
            return ops::visible_commit_indices(&commits, query.get().as_str(), selected)
                .into_iter()
                .filter_map(|index| commits.get(index).map(|commit| (index, commit.clone())))
                .collect::<Vec<_>>();
        })
    };
    let mut branches = tab
        .refs
        .branches
        .iter()
        .map(|branch| FilterBranch {
            label: branch.name.clone(),
            reference: format!("refs/heads/{}", branch.name),
            remote: false,
        })
        .collect::<Vec<_>>();
    branches.extend(tab.refs.remote_branches.iter().map(|branch| FilterBranch {
        label: branch.full_name(),
        reference: format!("refs/remotes/{}/{}", branch.remote, branch.name),
        remote: true,
    }));
    branches.sort_by(|left, right| left.label.cmp(&right.label));
    branches.dedup_by(|left, right| left.reference == right.reference);
    let authors = tab.authors.clone();
    let language_value = language;
    let filter_open = state.filter_open;
    let filter_position = state.filter_position;
    let filter_menu = state.filter_menu;
    let compare_mode = state.compare_mode;
    // 筛选浮层在 `dynamic` 里渲染，参数要能反复取用，因此这里各留一份克隆。
    let filter_state = state.clone();
    let filter_cmd = cmd.clone();
    let graph_ancestors = tab.graph_commits.clone();
    let table_ref = state.table_ref;
    let scroll_request = state.scroll_request;

    return view()
        .w_full()
        .flex_1()
        .min_h_0()
        .relative()
        .flex_col()
        .bg(move || design::content())
        .border_b_1()
        .border_color(move || design::line())
        .child(hist_bar(
            language,
            state,
            cmd,
            &tab,
            filter_position,
            filter_menu,
        ))
        .child(
            view()
                .w_full()
                .h(px(metrics::COMMIT_COLS_H))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .px(px(8.0))
                .border_b_1()
                .border_color(move || design::line_soft())
                .child(view().w(px(graph_width)))
                .child(column_header(language, "history.description").flex_1())
                .child(column_header(language, "history.author").w(px(AUTHOR_W)))
                .child(column_header(language, "history.date").w(px(DATE_W))),
        )
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .child(
                    view()
                        .w_full()
                        .h_full()
                        .child(dynamic(move || -> AnyElement {
                            let visible_rows = visible.get();
                            log::debug!(
                                target: "gitui::history",
                                "event=history_view.render tab={} loading={} commits={} visible={} graph_commits={} selected={} query_active={}",
                                tab_id,
                                loading,
                                total,
                                visible_rows.len(),
                                graph_ancestors.len(),
                                selected.is_some(),
                                !query.get().is_empty(),
                            );
                            // 后台还在读：先画骨架行，避免看到「空仓库」的假象。
                            if loading {
                                let mut skeleton: Vec<AnyElement> = Vec::new();
                                for index in 0..8 {
                                    skeleton.push(skeleton_row(index));
                                }
                                return view().w_full().flex_col().child(skeleton).into_element();
                            }
                            if visible_rows.is_empty() {
                                let branch_for_empty = branch_for_empty.clone();
                                let mut empty = view()
                                    .w_full()
                                    .h(px(96.0))
                                    .items_center()
                                    .justify_center()
                                    .flex_col()
                                    .gap(px(8.0))
                                    .child(
                                        text(move || {
                                            let lang = language_value.get();
                                            let lang = lang.as_str();
                                            if unborn {
                                                return i18n::with_arguments(
                                                    lang,
                                                    "state.unborn",
                                                    &[("branch", branch_for_empty.clone())],
                                                );
                                            }
                                            let key = if compare_refs_empty {
                                                "compare.noCommits"
                                            } else if total == 0 {
                                                "history.empty"
                                            } else {
                                                "history.noMatch"
                                            };
                                            return i18n::t(lang, key);
                                        })
                                        .text_size(px(crate::ui::BODY))
                                        .text_color(move || design::text3()),
                                    );
                                if unborn {
                                    let initial_commit = initial_commit.clone();
                                    empty = empty.child(crate::ui::button::button(
                                        {
                                            let language_value = language_value;
                                            move || {
                                                i18n::t(
                                                    language_value.get().as_str(),
                                                    "state.createInitialCommit",
                                                )
                                            }
                                        },
                                        crate::ui::button::ButtonVariant::Secondary,
                                        crate::ui::button::ButtonSize::Small,
                                        false,
                                        move || initial_commit(),
                                    ));
                                }
                                return empty.into_element();
                            }
                            let select = select.clone();
                            let toggle_compare = toggle_compare.clone();
                            let row_search = row_search.clone();
                            let squash_range = squash_range.clone();
                            let compare_for_rows = compare.clone();
                            // 表格：单列，整行交给 render 自绘——图线、引用徽章、对比
                            // 强调色都要保留，表格只负责虚拟化、行高与键盘导航。
                            // `visible` 是 Memo，先取一份快照供 render 与回调按索引取行。
                            let rows_snapshot = visible_rows;
                            let rows: Vec<Vec<String>> = rows_snapshot
                                .iter()
                                .map(|(index, _commit)| vec![index.to_string()])
                                .collect();
                            let commits_for_render = rows_snapshot.clone();
                            let compare_for_render = compare_for_rows.clone();
                            let visible_commits = rows_snapshot
                                .iter()
                                .map(|(_, commit)| commit.clone())
                                .collect::<Vec<_>>();
                            let graphs_for_render =
                                graph::layout_filtered(&visible_commits, &graph_ancestors);
                            let select_for_press = select.clone();
                            let toggle_for_press = toggle_compare.clone();
                            let compare_mode_for_press = compare_mode;
                            let open_diff_for_menu = open_commit_diff.clone();
                            let select_for_context = select.clone();
                            let menu_search = row_search.clone();
                            let menu_commit = commit_for_rows.clone();
                            let menu_squash = squash_range.clone();
                            let menu_refs = refs_for_menu.clone();
                            let menu_recovery = recovery_for_menu.clone();
                            let commits_for_press = rows_snapshot.clone();
                            let commits_for_menu = rows_snapshot.clone();
                            let visible_selected = rows_snapshot
                                .iter()
                                .position(|(index, _)| selected == Some(*index));
                            return crate::ui::table::table_with_scroll(
                                crate::ui::table::TableSpec {
                                    columns: vec![crate::ui::table::Column {
                                        key: "commit",
                                        title: None,
                                        // 单列自绘整行：宽度要给足，否则图线/摘要/作者/时间
                                        // 会被裁在 120pt 的默认列宽里看不出内容。
                                        width: Some(1200.0),
                                        fixed: false,
                                    }],
                                    rows,
                                    selected: visible_selected,
                                    row_height: Some(metrics::COMMIT_ROW_H),
                                    on_select: Box::new(move |_index| {}),
                                    render: Some(Box::new(move |row, _key, _text| {
                                        let Some((index, commit)) =
                                            commits_for_render.get(row as usize).cloned()
                                        else {
                                            return view().w_full().h_full().into_element();
                                        };
                                        let is_selected = selected == Some(index);
                                        let in_compare = compare_for_render.contains(&index);
                                        let has_refs = !commit.refs.is_empty();
                                        let graph_row = graphs_for_render
                                            .get(row as usize)
                                            .cloned()
                                            .unwrap_or_default();
                                        return view()
                                            .w_full()
                                            .h(px(metrics::COMMIT_ROW_H))
                                            .flex_row()
                                            .items_center()
                                            .gap(px(8.0))
                                            .px(px(8.0))
                                            .bg(move || {
                                                if in_compare {
                                                    design::accent_soft()
                                                } else {
                                                    AdaptiveColor::TRANSPARENT
                                                }
                                            })
                                            .child(graph_cell(
                                                graph_row,
                                                graph_width,
                                                is_selected || in_compare,
                                            ))
                                            .child(
                                                view()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .flex_row()
                                                    .items_center()
                                                    .gap(px(6.0))
                                                    .child(
                                                        text(commit.summary.clone())
                                                            .text_size(px(crate::ui::BODY))
                                                            .text_color(move || design::text())
                                                            .text_overflow(
                                                                nsh_core::EllipsizeMode::Tail,
                                                            ),
                                                    )
                                                    .child(
                                                        view()
                                                            .h(px(16.0))
                                                            .flex_row()
                                                            .items_center()
                                                            .gap(px(4.0))
                                                            .when(has_refs, |this| {
                                                                this.child(ref_chips(
                                                                    commit.refs.clone(),
                                                                ))
                                                            }),
                                                    ),
                                            )
                                            .child(
                                                text(commit.author.clone())
                                                    .w(px(AUTHOR_W))
                                                    .text_size(px(crate::ui::SECONDARY))
                                                    .text_color(move || design::text2())
                                                    .text_overflow(nsh_core::EllipsizeMode::Tail),
                                            )
                                            .child(
                                                text({
                                                    let timestamp =
                                                        commit.date.parse().unwrap_or(0);
                                                    crate::dates::format(
                                                        language_value.get().as_str(),
                                                        timestamp,
                                                        now,
                                                    )
                                                })
                                                .w(px(DATE_W))
                                                .text_size(px(crate::ui::SECONDARY))
                                                .text_color(move || design::text3()),
                                            )
                                            .into_element();
                                    })),
                                    on_row_press: Some(Box::new(move |row, platform| {
                                        let Some((index, _)) =
                                            commits_for_press.get(row as usize).cloned()
                                        else {
                                            return;
                                        };
                                        // 对比模式下点击是加入/移出集合；否则普通点击选中，
                                        // ⌘ 点击也加入对比。
                                        if compare_mode_for_press.get_untracked() || platform {
                                            toggle_for_press(index);
                                        } else {
                                            select_for_press(index);
                                        }
                                    })),
                                    on_context_menu: Some(Box::new(move |row, x, y| {
                                        let Some((index, commit)) =
                                            commits_for_menu.get(row as usize).cloned()
                                        else {
                                            return;
                                        };
                                        select_for_context(index);
                                        let items = commit_menu(
                                            language_value,
                                            &commit,
                                            {
                                                let open_diff = open_diff_for_menu.clone();
                                                std::sync::Arc::new(move || open_diff(index))
                                            },
                                            &menu_search,
                                            &menu_recovery,
                                            menu_squash.clone(),
                                            &menu_refs,
                                            &menu_commit,
                                        );
                                        crate::ui::menu::popup_at(
                                            items,
                                            nsh_core::Point {
                                                x: Points(x),
                                                y: Points(y),
                                            },
                                        );
                                    })),
                                },
                                table_ref,
                                scroll_request,
                            )
                            .into_element();
                        })),
                ),
        )
        // 06 板的多选操作栏：选中两个以上提交时出现。
        .child(dynamic(move || -> AnyElement {
            if !can_squash {
                return view().w_full().h(px(0.0)).into_element();
            }
            // 这个动态块是 `Fn`，每次渲染都取一份新的克隆。
            let open_squash = {
                let commit = commit_for_bar.clone();
                std::sync::Arc::new(
                    move |from: String, to: String, commits: Vec<(String, String)>| {
                        commit.open_squash(from, to, commits)
                    },
                )
            };
            let range = squash_range_for_bar.clone();
            let range_for_text = range.clone();
            let clear = clear_selection.clone();
            return view()
                .w_full()
                .h(px(metrics::HIST_BAR_H))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .px(px(10.0))
                .border_b_1()
                .border_color(move || design::line())
                .bg(move || design::accent_soft())
                .child(
                    text(move || {
                        i18n::with_arguments(
                            language_value.get().as_str(),
                            "compare.selected",
                            &[(
                                "count",
                                range_for_text
                                    .as_ref()
                                    .map(|(_, _, commits)| commits.len())
                                    .unwrap_or(0)
                                    .to_string(),
                            )],
                        )
                    })
                    .text_size(px(crate::ui::SECONDARY))
                    .line_height(px(crate::ui::SECONDARY_LINE))
                    .text_color(move || design::accent()),
                )
                .child(view().flex_1())
                .child(crate::ui::button::button(
                    move || i18n::t(language_value.get().as_str(), "rewrite.squashTitle"),
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    {
                        let open_squash = open_squash.clone();
                        let range = range.clone();
                        move || {
                            if let Some((from, to, commits)) = range.clone() {
                                open_squash(from, to, commits);
                            }
                        }
                    },
                ))
                .child(crate::ui::button::button(
                    move || i18n::t(language_value.get().as_str(), "compare.clear"),
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    {
                        let clear = clear.clone();
                        move || clear()
                    },
                ))
                .into_element();
        }))
        .child(dynamic(move || -> AnyElement {
            let advanced = filter_open.get();
            let menu = filter_menu.get();
            if !advanced && menu.is_none() {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            }
            let (left, top) = filter_position.get();
            let title_key = match menu {
                Some(crate::features::history::state::FilterMenu::Branches) => "filter.branch",
                Some(crate::features::history::state::FilterMenu::Authors) => "filter.author",
                None => "filter.title",
            };
            let panel = if advanced {
                filter_pop(
                    language_value,
                    branches.clone(),
                    authors.clone(),
                    filter_state.clone(),
                    filter_cmd.clone(),
                )
                .into_element()
            } else {
                quick_filter_pop(
                    language_value,
                    menu.unwrap_or(crate::features::history::state::FilterMenu::Branches),
                    branches.clone(),
                    authors.clone(),
                    filter_state.clone(),
                    filter_cmd.clone(),
                )
                .into_element()
            };
            let close_state = filter_state.clone();
            return crate::ui::dialog::popover_overlay(
                move || i18n::t(language_value.get().as_str(), title_key),
                80,
                view().w_full().h_full().relative().child(
                    view()
                        .absolute()
                        .left(px(left))
                        .top(px(top))
                        .z_index(1)
                        .child(crate::ui::dialog::modality(panel)),
                ),
                move || {
                    close_state.filter_open.set(false);
                    close_state.filter_menu.set(None);
                },
            );
        }));
}

fn hist_bar(
    language: Signal<SharedString>,
    state: &HistoryState,
    cmd: &HistoryCommands,
    tab: &Tab,
    filter_position: Signal<(f32, f32)>,
    filter_menu: Signal<Option<crate::features::history::state::FilterMenu>>,
) -> impl IntoElement + use<> {
    let filter_open = state.filter_open;
    let compare_mode = state.compare_mode;
    let total = tab.commits.len();
    let applied = tab.filter.clone();
    let applied_branches = if applied.references.is_empty() {
        applied.reference.clone().into_iter().collect::<Vec<_>>()
    } else {
        applied.references.clone()
    };
    let has_dates = applied.since.is_some() || applied.until.is_some();
    let has_refine = applied.path.is_some() || !applied.message.is_empty();
    let filter_active =
        has_dates || has_refine || !applied_branches.is_empty() || !applied.authors.is_empty();
    // Compare refs 提供所有本地分支、远端跟踪分支与 tag；value 使用限定 ref 避免同名歧义。
    let mut refs_list: Vec<CompareRef> = Vec::new();
    let language_value = language.get_untracked();
    for branch in &tab.refs.branches {
        refs_list.push(CompareRef {
            label: i18n::with_arguments(
                language_value.as_str(),
                "compare.localBranch",
                &[("name", branch.name.clone())],
            ),
            value: format!("refs/heads/{}", branch.name),
        });
    }
    for branch in &tab.refs.remote_branches {
        refs_list.push(CompareRef {
            label: i18n::with_arguments(
                language_value.as_str(),
                "compare.remoteBranch",
                &[("name", branch.full_name())],
            ),
            value: format!("refs/remotes/{}", branch.full_name()),
        });
    }
    for tag in &tab.refs.tags {
        refs_list.push(CompareRef {
            label: i18n::with_arguments(
                language_value.as_str(),
                "compare.tagRef",
                &[("name", tag.clone())],
            ),
            value: format!("refs/tags/{tag}"),
        });
    }
    for stash in &tab.refs.stashes {
        refs_list.push(CompareRef {
            label: i18n::with_arguments(
                language_value.as_str(),
                "compare.stashRef",
                &[("name", stash.summary.clone())],
            ),
            value: stash.oid.clone(),
        });
    }
    let compare_pair = (tab.compare_base.clone(), tab.compare_target.clone());
    let compare_three_dot = tab.compare_three_dot;
    let compare_worktree = tab.compare_worktree;
    let branch_selection = applied_branches.clone();
    let branch_active = !branch_selection.is_empty();
    let branch_filter_menu = filter_menu;
    let branch_filter_open = filter_open;
    let branch_filter_position = filter_position;
    let branch_filter = crate::ui::menu::menu_button_tuned_at(
        move || {
            if branch_selection.is_empty() {
                return i18n::t(language.get().as_str(), "filter.allBranches");
            }
            return i18n::with_arguments(
                language.get().as_str(),
                "filter.branchCount",
                &[("count", branch_selection.len().to_string())],
            );
        },
        move || {
            branch_active
                || branch_filter_menu.get()
                    == Some(crate::features::history::state::FilterMenu::Branches)
        },
        Some("branch"),
        move |position| {
            if branch_filter_menu.get_untracked()
                == Some(crate::features::history::state::FilterMenu::Branches)
            {
                branch_filter_menu.set(None);
                return;
            }
            branch_filter_open.set(false);
            branch_filter_position.set(filter_popover_position(position, QUICK_FILTER_W));
            branch_filter_menu.set(Some(crate::features::history::state::FilterMenu::Branches));
        },
    );
    let author_selection = applied.authors.clone();
    let author_active = !author_selection.is_empty();
    let author_filter_menu = filter_menu;
    let author_filter_open = filter_open;
    let author_filter_position = filter_position;
    let author_filter = crate::ui::menu::menu_button_tuned_at(
        move || {
            if author_selection.is_empty() {
                return i18n::t(language.get().as_str(), "history.allAuthors");
            }
            return i18n::with_arguments(
                language.get().as_str(),
                "filter.authorCount",
                &[("count", author_selection.len().to_string())],
            );
        },
        move || {
            author_active
                || author_filter_menu.get()
                    == Some(crate::features::history::state::FilterMenu::Authors)
        },
        Some("chev-down"),
        move |position| {
            if author_filter_menu.get_untracked()
                == Some(crate::features::history::state::FilterMenu::Authors)
            {
                author_filter_menu.set(None);
                return;
            }
            author_filter_open.set(false);
            author_filter_position.set(filter_popover_position(position, QUICK_FILTER_W));
            author_filter_menu.set(Some(crate::features::history::state::FilterMenu::Authors));
        },
    );
    // 视图回调要 `Arc<dyn Fn>`，这里把本功能的命令包一层。
    let set_compare_refs: std::sync::Arc<dyn Fn(Option<String>, Option<String>)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |base: Option<String>, target: Option<String>| {
            cmd.set_compare_refs(base, target)
        })
    };
    let set_compare_mode: std::sync::Arc<dyn Fn(bool)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |enabled: bool| cmd.set_compare_mode(enabled))
    };
    let set_compare_three_dot: std::sync::Arc<dyn Fn(bool)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |enabled| cmd.set_compare_three_dot(enabled))
    };
    let set_compare_worktree: std::sync::Arc<dyn Fn(bool)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |enabled| cmd.set_compare_worktree(enabled))
    };
    return view()
        .w_full()
        .h(px(metrics::HIST_BAR_H))
        .flex_row()
        .items_center()
        .gap(px(7.0))
        .px(px(10.0))
        .border_b_1()
        .border_color(move || design::line_soft())
        .child(
            text(move || i18n::t(language.get().as_str(), "shell.history"))
                .text_size(px(crate::ui::BODY))
                .font_weight(FontWeight(600))
                .text_color(move || design::text()),
        )
        .child(branch_filter)
        .child(
            text(move || {
                i18n::with_arguments(
                    language.get().as_str(),
                    "history.commits",
                    &[("count", total.to_string())],
                )
            })
            .text_size(px(crate::ui::SECONDARY))
            .text_color(move || design::text3()),
        )
        .child(author_filter)
        .child(view().flex_1())
        .child({
            let variant = if compare_mode.get() {
                crate::ui::button::ButtonVariant::Secondary
            } else {
                crate::ui::button::ButtonVariant::Ghost
            };
            crate::ui::button::button(
                move || i18n::t(language.get().as_str(), "compare.mode"),
                variant,
                crate::ui::button::ButtonSize::Small,
                false,
                {
                    let set_compare_mode = set_compare_mode.clone();
                    move || set_compare_mode(!compare_mode.get_untracked())
                },
            )
        })
        .child(compare_refs_menu(
            language,
            refs_list,
            compare_pair,
            compare_three_dot,
            compare_worktree,
            set_compare_refs,
            set_compare_three_dot,
            set_compare_worktree,
        ))
        .child(widgets_icon(
            language,
            filter_open,
            filter_position,
            filter_menu,
            filter_active,
        ));
}

fn widgets_icon(
    language: Signal<SharedString>,
    filter_open: Signal<bool>,
    filter_position: Signal<(f32, f32)>,
    filter_menu: Signal<Option<crate::features::history::state::FilterMenu>>,
    filter_active: bool,
) -> impl IntoElement {
    let foreground = std::rc::Rc::new(move || {
        if filter_open.get() || filter_active {
            design::accent()
        } else {
            design::text2()
        }
    });
    return crate::ui::button::icon_button_tuned_at(
        "filter",
        move || i18n::t(language.get().as_str(), "history.filter"),
        crate::ui::button::ButtonSize::Small,
        Some(crate::ui::icon::IconSize::Button),
        Some(foreground),
        false,
        move |position| {
            if filter_open.get_untracked() {
                filter_open.set(false);
                return;
            }
            // 过滤器靠近触发按钮右对齐展开，避免遮住工具栏与提交列表入口。
            filter_menu.set(None);
            filter_position.set(filter_popover_position(position, POP_W));
            filter_open.set(true);
        },
    );
}

fn filter_popover_position(position: Point<Points>, width: f32) -> (f32, f32) {
    let left = (position.x.0 - width + 12.0).max(8.0);
    return (left, position.y.0 + 12.0);
}

fn column_header(language: Signal<SharedString>, key: &'static str) -> nsh_core::element::Text {
    return text(move || i18n::t(language.get().as_str(), key))
        .text_size(px(crate::ui::SECONDARY))
        .font_weight(FontWeight(600))
        .text_color(move || design::text3());
}

fn graph_cell(graph: GraphRow, width: f32, highlight: bool) -> impl IntoElement {
    let row_height = metrics::COMMIT_ROW_H;
    let mut cell = view()
        .relative()
        .w(px(width.max(GRAPH_LANE_W)))
        .h(px(row_height));
    for lane in graph.active_lanes {
        let x = lane as f32 * GRAPH_LANE_W + GRAPH_LANE_W / 2.0 - 0.5;
        cell = cell.child(
            view()
                .absolute()
                .left(px(x))
                .top(px(0.0))
                .w(px(1.0))
                .h_full()
                .bg(move || design::graph_color(lane)),
        );
    }
    for parent_lane in graph.parent_lanes {
        let start = graph.node_lane.min(parent_lane);
        let end = graph.node_lane.max(parent_lane);
        let left = start as f32 * GRAPH_LANE_W + GRAPH_LANE_W / 2.0;
        let width = (end - start) as f32 * GRAPH_LANE_W;
        let color = graph.node_lane;
        cell = cell.child(
            view()
                .absolute()
                .left(px(left))
                .top(px(row_height / 2.0 - 0.5))
                .w(px(width))
                .h(px(1.0))
                .bg(move || design::graph_color(color)),
        );
    }
    let node_left = graph.node_lane as f32 * GRAPH_LANE_W + GRAPH_LANE_W / 2.0 - 3.5;
    return cell.child(
        view()
            .absolute()
            .left(px(node_left))
            .top(px(row_height / 2.0 - 3.5))
            .w(px(7.0))
            .h(px(7.0))
            .rounded(px(4.0))
            .bg(move || {
                if highlight {
                    design::accent()
                } else {
                    design::graph_color(graph.node_lane)
                }
            }),
    );
}

/// 筛选浮层（设计板 05）：分支、作者、时间三组，底部应用与清除。
/// 加载骨架的一行：宽窄不等的灰条，模拟「提交说明 / 作者 / 时间」。
fn skeleton_row(index: usize) -> AnyElement {
    // 宽度按行号错开，看起来像真实数据的长度差异。
    let summary_width = 180.0 + ((index * 37) % 160) as f32;
    return view()
        .w_full()
        .h(px(metrics::COMMIT_ROW_H))
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .px(px(8.0))
        .border_b_1()
        .border_color(move || design::line_soft())
        .child(
            view()
                .w(px(20.0))
                .h(px(8.0))
                .rounded(px(4.0))
                .bg(move || design::panel2()),
        )
        .child(
            view()
                .w(px(summary_width))
                .h(px(8.0))
                .rounded(px(4.0))
                .bg(move || design::panel2()),
        )
        .child(view().flex_1())
        .child(
            view()
                .w(px(56.0))
                .h(px(8.0))
                .rounded(px(4.0))
                .bg(move || design::panel2()),
        )
        .into_element();
}

/// 提交行的右键菜单（设计板 21）。
///
/// 顺序：查看 → 引用类（检出/新建分支/新建标签）→ 改写类（拣选/回滚/合并）→
/// 重置 → 复制。重置放在最后并用分隔线隔开，因为它会移动当前分支。
fn commit_menu(
    language: Signal<SharedString>,
    commit: &domain::Commit,
    open: std::sync::Arc<dyn Fn()>,
    search: &SearchCommands,
    recovery: &crate::features::recovery::commands::RecoveryCommands,
    squash_range: Option<(String, String, Vec<(String, String)>)>,
    refs: &RefsCommands,
    commit_cmd: &CommitCommands,
) -> Vec<MenuItem> {
    let lang = language.get_untracked();
    let lang = lang.as_str();
    let short = commit.short.as_str();
    let summary = commit.summary.as_str();
    // 范围只有在至少两个提交时才给出来，因此「有范围」等价于「可以合并」。
    let can_squash = squash_range.is_some();
    let sha = short.to_string();
    let checkout: std::sync::Arc<dyn Fn(String)> = {
        let refs = refs.clone();
        std::sync::Arc::new(move |sha: String| refs.checkout_commit(sha))
    };
    let cherry: std::sync::Arc<dyn Fn(String)> = {
        let recovery = recovery.clone();
        std::sync::Arc::new(move |sha: String| recovery.cherry_pick(sha))
    };
    let revert: std::sync::Arc<dyn Fn(String)> = {
        let recovery = recovery.clone();
        std::sync::Arc::new(move |sha: String| recovery.revert_commit(sha))
    };
    let merge: std::sync::Arc<dyn Fn(String)> = {
        let refs = refs.clone();
        std::sync::Arc::new(move |sha: String| refs.merge_reference(sha))
    };
    // 表单与复制都要进 `'static` 回调，克隆一份本功能的句柄。
    let search_for_form = search.clone();
    let search_for_copy = search.clone();
    let open_reword = {
        let commit_cmd = commit_cmd.clone();
        std::sync::Arc::new(move |sha: String, short: String, summary: String| {
            commit_cmd.open_reword(sha, short, summary)
        })
    };
    let open_squash = {
        let commit_cmd = commit_cmd.clone();
        std::sync::Arc::new(
            move |from: String, to: String, commits: Vec<(String, String)>| {
                commit_cmd.open_squash(from, to, commits)
            },
        )
    };

    let mut items = vec![
        MenuItem::new(i18n::t(lang, "menu.openDiff")).on_select({
            let open = open.clone();
            move || open()
        }),
        MenuItem::separator(),
        MenuItem::new(i18n::t(lang, "menu.checkout")).on_select({
            let checkout = checkout.clone();
            let sha = sha.clone();
            move || checkout(sha.clone())
        }),
        MenuItem::new(i18n::t(lang, "form.newBranch")).on_select({
            let search = search_for_form.clone();
            let sha = sha.clone();
            move || search.open_form_from(crate::features::search::FormKind::NewBranch, sha.clone())
        }),
        MenuItem::new(i18n::t(lang, "form.newTag")).on_select({
            let search = search_for_form.clone();
            let sha = sha.clone();
            move || search.open_form_from(crate::features::search::FormKind::NewTag, sha.clone())
        }),
        MenuItem::separator(),
        MenuItem::new(i18n::t(lang, "menu.cherryPick")).on_select({
            let cherry = cherry.clone();
            let sha = sha.clone();
            move || cherry(sha.clone())
        }),
        MenuItem::new(i18n::t(lang, "menu.revert")).on_select({
            let revert = revert.clone();
            let sha = sha.clone();
            move || revert(sha.clone())
        }),
        MenuItem::new(i18n::t(lang, "menu.mergeIntoCurrent")).on_select({
            let merge = merge.clone();
            let sha = sha.clone();
            move || {
                let _ = merge(sha.clone());
            }
        }),
        MenuItem::new(i18n::t(lang, "rewrite.editMessage")).on_select({
            let open_reword = open_reword.clone();
            let sha = sha.clone();
            let summary = summary.to_string();
            let short = short.to_string();
            move || open_reword(sha.clone(), short.clone(), summary.clone())
        }),
        MenuItem::new(i18n::t(lang, "rewrite.squashTitle"))
            .disabled(!can_squash)
            .on_select({
                let open_squash = open_squash.clone();
                let range = squash_range.clone();
                move || {
                    if let Some((from, to, commits)) = range.clone() {
                        open_squash(from, to, commits);
                    }
                }
            }),
        MenuItem::separator(),
        MenuItem::new(i18n::t(lang, "branch.resetSoft")).on_select({
            let recovery = recovery.clone();
            let sha = sha.clone();
            move || recovery.reset_to(sha.clone(), git::ResetMode::Soft)
        }),
        MenuItem::new(i18n::t(lang, "branch.resetMixed")).on_select({
            let recovery = recovery.clone();
            let sha = sha.clone();
            move || recovery.reset_to(sha.clone(), git::ResetMode::Mixed)
        }),
        MenuItem::new(i18n::t(lang, "branch.resetHard")).on_select({
            let recovery = recovery.clone();
            let sha = sha.clone();
            move || recovery.reset_to(sha.clone(), git::ResetMode::Hard)
        }),
        MenuItem::separator(),
        MenuItem::new(i18n::t(lang, "menu.copySha")).on_select({
            let search = search_for_copy.clone();
            let sha = sha.clone();
            move || {
                crate::shared::os::copy_text(search.session.notice, search.language, sha.clone())
            }
        }),
        MenuItem::new(i18n::t(lang, "menu.copyMessage")).on_select({
            let search = search_for_copy.clone();
            let summary = summary.to_string();
            move || {
                crate::shared::os::copy_text(
                    search.session.notice,
                    search.language,
                    summary.clone(),
                )
            }
        }),
    ];
    items.shrink_to_fit();
    return items;
}

fn quick_filter_pop(
    language: Signal<SharedString>,
    kind: crate::features::history::state::FilterMenu,
    branches: Vec<FilterBranch>,
    authors: Vec<git::Author>,
    state: HistoryState,
    cmd: HistoryCommands,
) -> impl IntoElement + use<> {
    let draft = state.draft;
    let close = state.filter_menu;
    let (title_key, empty_key) = match kind {
        crate::features::history::state::FilterMenu::Branches => {
            ("filter.branch", "filter.allBranches")
        }
        crate::features::history::state::FilterMenu::Authors => {
            ("filter.author", "filter.allAuthors")
        }
    };
    let mut rows = Vec::new();
    match kind {
        crate::features::history::state::FilterMenu::Branches => {
            rows.push(filter_choice_row(
                language,
                empty_key,
                std::sync::Arc::new(move || draft.get().references.is_empty()),
                {
                    let cmd = cmd.clone();
                    std::sync::Arc::new(move || cmd.clear_branch_filter())
                },
            ));
            for branch in branches {
                let reference = branch.reference.clone();
                let selected = reference.clone();
                let label = if branch.remote {
                    format!(
                        "{} · {}",
                        branch.label,
                        i18n::t(language.get_untracked().as_str(), "filter.remote")
                    )
                } else {
                    branch.label
                };
                let checked =
                    std::sync::Arc::new(move || draft.get().references.contains(&selected));
                let toggle = cmd.clone();
                rows.push(
                    crate::ui::check::check_inline(
                        move || label.clone(),
                        move || checked(),
                        26.0,
                        14.0,
                        false,
                        move |_| toggle.toggle_branch_filter(reference.clone()),
                    )
                    .into_element(),
                );
            }
        }
        crate::features::history::state::FilterMenu::Authors => {
            rows.push(filter_choice_row(
                language,
                empty_key,
                std::sync::Arc::new(move || draft.get().authors.is_empty()),
                {
                    let cmd = cmd.clone();
                    std::sync::Arc::new(move || cmd.clear_author_filter())
                },
            ));
            for author in authors {
                let email = author.email.clone();
                let selected = email.clone();
                let label = format!("{} <{}>", author.name, email);
                let checked = std::sync::Arc::new(move || {
                    draft.get().authors.iter().any(|item| item == &selected)
                });
                let toggle = cmd.clone();
                rows.push(
                    crate::ui::check::check_inline(
                        move || label.clone(),
                        move || checked(),
                        26.0,
                        14.0,
                        false,
                        move |_| toggle.toggle_author_filter(email.clone()),
                    )
                    .into_element(),
                );
            }
        }
    }

    return view()
        .w(px(QUICK_FILTER_W))
        .p(px(10.0))
        .flex_col()
        .gap(px(crate::ui::GAP_XS))
        .rounded(px(metrics::RADIUS_PANEL))
        .border_1()
        .border_color(move || design::line_strong())
        .bg(move || design::token("popover"))
        .shadow_lg()
        .child(
            view()
                .w_full()
                .h(px(crate::ui::ROW_COMPACT))
                .flex_row()
                .items_center()
                .gap(px(crate::ui::GAP_SM))
                .child(
                    text(move || i18n::t(language.get().as_str(), title_key))
                        .flex_1()
                        .text_size(px(crate::ui::BODY))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || design::text()),
                )
                .child(kbd("esc"))
                .child(crate::ui::button::icon_button(
                    "x",
                    {
                        let language = language;
                        move || i18n::t(language.get().as_str(), "shell.close")
                    },
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || close.set(None),
                )),
        )
        .child(
            scroll()
                .w_full()
                .h(px(288.0))
                .child(view().w_full().flex_col().children(rows)),
        )
        .into_element();
}

fn filter_pop(
    language: Signal<SharedString>,
    branches: Vec<FilterBranch>,
    authors: Vec<git::Author>,
    state: HistoryState,
    cmd: HistoryCommands,
) -> impl IntoElement + use<> {
    let draft = state.draft;
    let close_filter = state.filter_open;
    // 视图回调要 `Arc<dyn Fn>`，这里把本功能的命令包一层。
    let apply: std::sync::Arc<dyn Fn()> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move || cmd.apply_filter())
    };
    let clear: std::sync::Arc<dyn Fn()> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move || cmd.clear_filter())
    };
    let all_branches = std::sync::Arc::new(move || draft.update(|value| value.references.clear()));
    let branch_values = branches
        .iter()
        .map(|branch| branch.reference.clone())
        .collect::<Vec<_>>();
    let select_all_branches = std::sync::Arc::new(move || {
        draft.update(|value| value.references = branch_values.clone());
    });
    let mut branch_rows = vec![filter_choice_row(
        language,
        "filter.allBranches",
        std::sync::Arc::new(move || draft.get().references.is_empty()),
        all_branches,
    )];
    for branch in &branches {
        let branch_value = branch.reference.clone();
        let selected_value = branch_value.clone();
        let label = if branch.remote {
            format!(
                "{} · {}",
                branch.label,
                i18n::t(language.get_untracked().as_str(), "filter.remote")
            )
        } else {
            branch.label.clone()
        };
        let checked = std::sync::Arc::new(move || {
            draft
                .get()
                .references
                .iter()
                .any(|reference| reference == &selected_value)
        });
        branch_rows.push(
            view()
                .w_full()
                .h(px(24.0))
                .flex_row()
                .items_center()
                .child(crate::ui::check::check_inline(
                    move || label.clone(),
                    move || checked(),
                    24.0,
                    14.0,
                    false,
                    move |checked| {
                        draft.update(|value| {
                            if checked {
                                if !value.references.contains(&branch_value) {
                                    value.references.push(branch_value.clone());
                                }
                            } else {
                                value.references.retain(|item| item != &branch_value);
                            }
                        });
                    },
                ))
                .into_element(),
        );
    }

    let all_authors = std::sync::Arc::new(move || draft.update(|value| value.authors.clear()));
    let mut author_rows = vec![filter_choice_row(
        language,
        "filter.allAuthors",
        std::sync::Arc::new(move || draft.get().authors.is_empty()),
        all_authors,
    )];
    for author in &authors {
        let email = author.email.clone();
        let selected_email = email.clone();
        let label = format!("{} <{}>", author.name, author.email);
        let checked = std::sync::Arc::new(move || {
            draft
                .get()
                .authors
                .iter()
                .any(|selected| selected == &selected_email)
        });
        author_rows.push(
            crate::ui::check::check_inline(
                move || label.clone(),
                move || checked(),
                24.0,
                14.0,
                false,
                move |checked| {
                    draft.update(|value| {
                        if checked {
                            if !value.authors.contains(&email) {
                                value.authors.push(email.clone());
                            }
                        } else {
                            value.authors.retain(|item| item != &email);
                        }
                    });
                },
            )
            .into_element(),
        );
    }

    let when_rows = When::ALL
        .iter()
        .map(|when| {
            let when = *when;
            filter_choice_row(
                language,
                when.key(),
                std::sync::Arc::new(move || draft.get().when == when),
                std::sync::Arc::new(move || draft.update(|value| value.when = when)),
            )
        })
        .collect::<Vec<_>>();

    let branch_select_all = crate::ui::button::button(
        move || i18n::t(language.get().as_str(), "filter.selectAll"),
        crate::ui::button::ButtonVariant::Ghost,
        crate::ui::button::ButtonSize::Small,
        false,
        move || select_all_branches(),
    );
    let custom_dates =
        {
            let draft = draft;
            dynamic(move || -> AnyElement {
                if draft.get().when != When::Custom {
                    return view().h(px(0.0)).into_element();
                }
                let since_get = move || draft.get().since.clone();
                let since_set = move |value: String| {
                    draft.update(|state| state.since = value);
                };
                let until_get = move || draft.get().until.clone();
                let until_set = move |value: String| {
                    draft.update(|state| state.until = value);
                };
                return view()
                    .w_full()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .child(view().flex_1().min_w_0().child(
                        crate::ui::text_input::text_input_bound(
                            since_get,
                            move || i18n::t(language.get().as_str(), "filter.startDate"),
                            false,
                            since_set,
                        ),
                    ))
                    .child(text("→").text_color(move || design::text3()))
                    .child(view().flex_1().min_w_0().child(
                        crate::ui::text_input::text_input_bound(
                            until_get,
                            move || i18n::t(language.get().as_str(), "filter.endDate"),
                            false,
                            until_set,
                        ),
                    ))
                    .into_element();
            })
        };
    let path_input = crate::ui::text_input::text_input_bound(
        move || draft.get().path.clone(),
        move || i18n::t(language.get().as_str(), "filter.pathPlaceholder"),
        false,
        move |value| draft.update(|state| state.path = value),
    );
    let message_input = crate::ui::text_input::text_input_bound(
        move || draft.get().message.clone(),
        move || i18n::t(language.get().as_str(), "filter.messagePlaceholder"),
        false,
        move |value| draft.update(|state| state.message = value),
    );

    return view()
        .w(px(POP_W))
        .max_h(px(560.0))
        .p(px(12.0))
        .flex_col()
        .gap(px(6.0))
        .rounded(px(metrics::RADIUS_PANEL))
        .border_1()
        .border_color(move || design::line_strong())
        .bg(move || design::token("popover"))
        .shadow_lg()
        .child(
            view()
                .w_full()
                .h(px(crate::ui::ROW_COMPACT))
                .flex_row()
                .items_center()
                .gap(px(crate::ui::GAP_SM))
                .child(
                    text(move || i18n::t(language.get().as_str(), "filter.title"))
                        .flex_1()
                        .text_size(px(crate::ui::BODY))
                        .line_height(px(crate::ui::ROW_COMPACT))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || design::text()),
                )
                .child(kbd("esc"))
                .child(crate::ui::button::icon_button(
                    "x",
                    {
                        let language = language;
                        move || i18n::t(language.get().as_str(), "shell.close")
                    },
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || close_filter.set(false),
                )),
        )
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .child(
                    view()
                        .flex_1()
                        .child(section_label(language, "filter.branch")),
                )
                .child(branch_select_all),
        )
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    view()
                        .w(px(120.0))
                        .h(px(24.0))
                        .flex_shrink_0()
                        .child(branch_rows.remove(0)),
                )
                .child(
                    scroll()
                        .w_full()
                        .h(px(120.0))
                        .child(view().w_full().flex_col().child(branch_rows)),
                ),
        )
        .child(section_label(language, "filter.author"))
        .child(
            scroll()
                .w_full()
                .h(px(96.0))
                .child(view().w_full().flex_col().child(author_rows)),
        )
        .child(section_label(language, "filter.when"))
        .child(
            view()
                .w_full()
                .flex_row()
                .flex_wrap()
                .gap(px(4.0))
                .child(when_rows),
        )
        .child(custom_dates)
        .child(section_label(language, "filter.path"))
        .child(path_input)
        .child(section_label(language, "filter.message"))
        .child(message_input)
        .child(
            view()
                .w_full()
                .mt(px(2.0))
                .pt(px(8.0))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .border_t_1()
                .border_color(move || design::line())
                .child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "filter.apply"),
                    crate::ui::button::ButtonVariant::Primary,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    {
                        let apply = apply.clone();
                        move || apply()
                    },
                ))
                .child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "filter.reset"),
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    {
                        let clear = clear.clone();
                        move || clear()
                    },
                ))
                .child(view().flex_1()),
        );
}

fn section_label(language: Signal<SharedString>, key: &'static str) -> impl IntoElement {
    return crate::ui::section::section_label(move || i18n::t(language.get().as_str(), key));
}

fn filter_choice_row(
    language: Signal<SharedString>,
    label_key: &'static str,
    selected: std::sync::Arc<dyn Fn() -> bool>,
    on_press: std::sync::Arc<dyn Fn()>,
) -> AnyElement {
    return dynamic(move || -> AnyElement {
        let variant = if selected() {
            crate::ui::button::ButtonVariant::Secondary
        } else {
            crate::ui::button::ButtonVariant::Ghost
        };
        let press = on_press.clone();
        return crate::ui::button::button(
            move || i18n::t(language.get().as_str(), label_key),
            variant,
            crate::ui::button::ButtonSize::Small,
            false,
            move || press(),
        )
        .into_element();
    })
    .into_element();
}

/// 引用徽章：本地分支、远程、标签各用一组配色。
fn ref_chips(refs: Vec<String>) -> impl IntoElement {
    let mut chips: Vec<AnyElement> = Vec::new();
    for reference in refs.into_iter().take(2) {
        let is_stash = reference.starts_with("stash: ");
        let is_tag = reference.starts_with("tag:");
        let is_remote = reference.contains('/') && !is_tag && !is_stash;
        let label = reference
            .trim_start_matches("HEAD -> ")
            .trim_start_matches("stash: ")
            .trim_start_matches("tag: ")
            .to_string();
        chips.push(
            view()
                .h(px(16.0))
                .px(px(4.0))
                .items_center()
                .rounded(px(3.0))
                .bg(move || {
                    if is_tag {
                        design::tag_bg()
                    } else if is_stash {
                        design::panel2()
                    } else if is_remote {
                        design::panel2()
                    } else {
                        design::branch_bg()
                    }
                })
                .child(
                    text(label)
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(move || {
                            if is_tag {
                                design::tag_fg()
                            } else if is_stash {
                                design::remote_fg()
                            } else if is_remote {
                                design::remote_fg()
                            } else {
                                design::branch_fg()
                            }
                        }),
                )
                .into_element(),
        );
    }
    return view().flex_row().items_center().gap(px(4.0)).child(chips);
}

/// Compare refs 下拉菜单：集中放入端点、模式、工作区比较、交换与清除动作。
fn compare_refs_menu(
    language: Signal<SharedString>,
    refs: Vec<CompareRef>,
    pair: (Option<String>, Option<String>),
    three_dot: bool,
    compare_worktree: bool,
    set_compare_refs: std::sync::Arc<dyn Fn(Option<String>, Option<String>)>,
    set_three_dot: std::sync::Arc<dyn Fn(bool)>,
    set_worktree: std::sync::Arc<dyn Fn(bool)>,
) -> impl IntoElement {
    let (base, target) = pair;
    let has_pair = base.is_some() && target.is_some();
    let label_key = if has_pair || compare_worktree {
        "compare.refsActive"
    } else {
        "compare.pick"
    };
    let menu_refs = refs.clone();
    let menu_base = base.clone();
    let menu_target = target.clone();
    let menu_set_refs = set_compare_refs.clone();
    let menu_set_dot = set_three_dot.clone();
    let menu_set_worktree = set_worktree.clone();

    return crate::ui::menu::menu_button_tuned(
        move || i18n::t(language.get().as_str(), label_key),
        has_pair || compare_worktree,
        Some("chev-down"),
        move || {
            let mut items = Vec::new();
            for reference in menu_refs.clone() {
                let value = reference.value.clone();
                let target = menu_target.clone();
                let selected = menu_base.as_deref() == Some(value.as_str());
                let action = menu_set_refs.clone();
                let label = format!(
                    "{} · {}",
                    i18n::t(language.get().as_str(), "compare.base"),
                    reference.label
                );
                items.push(crate::ui::menu::checkable(
                    label,
                    selected,
                    true,
                    move || action(Some(value.clone()), target.clone()),
                ));
            }
            items.push(crate::ui::menu::divider());
            for reference in menu_refs.clone() {
                let value = reference.value.clone();
                let base = menu_base.clone();
                let selected = menu_target.as_deref() == Some(value.as_str());
                let action = menu_set_refs.clone();
                let label = format!(
                    "{} · {}",
                    i18n::t(language.get().as_str(), "compare.target"),
                    reference.label
                );
                items.push(crate::ui::menu::checkable(
                    label,
                    selected,
                    true,
                    move || action(base.clone(), Some(value.clone())),
                ));
            }
            items.push(crate::ui::menu::divider());
            items.push(crate::ui::menu::checkable(
                i18n::t(language.get().as_str(), "compare.twoDot"),
                !three_dot,
                true,
                {
                    let action = menu_set_dot.clone();
                    move || action(false)
                },
            ));
            items.push(crate::ui::menu::checkable(
                i18n::t(language.get().as_str(), "compare.threeDot"),
                three_dot,
                !compare_worktree,
                {
                    let action = menu_set_dot.clone();
                    move || action(true)
                },
            ));
            items.push(crate::ui::menu::checkable(
                i18n::t(language.get().as_str(), "compare.withWorkingTree"),
                compare_worktree,
                menu_target.is_some(),
                {
                    let action = menu_set_worktree.clone();
                    move || action(!compare_worktree)
                },
            ));
            items.push(crate::ui::menu::action(
                i18n::t(language.get().as_str(), "compare.swap"),
                has_pair && !compare_worktree,
                {
                    let action = menu_set_refs.clone();
                    let base = menu_base.clone();
                    let target = menu_target.clone();
                    move || action(target.clone(), base.clone())
                },
            ));
            items.push(crate::ui::menu::divider());
            items.push(crate::ui::menu::action(
                i18n::t(language.get().as_str(), "compare.clear"),
                has_pair || menu_base.is_some() || menu_target.is_some() || compare_worktree,
                {
                    let action = menu_set_refs.clone();
                    move || action(None, None)
                },
            ));
            return items;
        },
    );
}
