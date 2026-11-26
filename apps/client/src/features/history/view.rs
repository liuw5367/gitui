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
use nsh_core::element::desktop::MenuItem;
use nsh_core::prelude::*;

const AUTHOR_MIN_W: f32 = 80.0;
const AUTHOR_MAX_W: f32 = 320.0;
const DATE_MIN_W: f32 = 80.0;
const DATE_MAX_W: f32 = 250.0;
const GRAPH_LANE_W: f32 = 10.0;
const GRAPH_NODE_D: f32 = 7.0;

/// 浮层宽度（设计板 `.filter-pop`）。
const POP_W: f32 = 432.0;
const QUICK_FILTER_W: f32 = 320.0;
/// 比较引用窗口按引用名与两列端点操作布置。
const COMPARE_DIALOG_W: f32 = 640.0;
const COMPARE_REF_LIST_H: f32 = 350.0;
const COMPARE_REF_ACTION_W: f32 = 88.0;

#[derive(Clone)]
struct FilterBranch {
    label: String,
    reference: String,
    remote: bool,
}

#[derive(Clone)]
struct CompareRef {
    label_key: &'static str,
    name: String,
    value: String,
}

fn compare_refs_for_tab(tab: &Tab) -> Vec<CompareRef> {
    let mut refs = Vec::new();
    for branch in &tab.refs.branches {
        refs.push(CompareRef {
            label_key: "compare.localBranch",
            name: branch.name.clone(),
            value: format!("refs/heads/{}", branch.name),
        });
    }
    for branch in &tab.refs.remote_branches {
        refs.push(CompareRef {
            label_key: "compare.remoteBranch",
            name: branch.full_name(),
            value: format!("refs/remotes/{}", branch.full_name()),
        });
    }
    for tag in &tab.refs.tags {
        refs.push(CompareRef {
            label_key: "compare.tagRef",
            name: tag.clone(),
            value: format!("refs/tags/{tag}"),
        });
    }
    for stash in &tab.refs.stashes {
        refs.push(CompareRef {
            label_key: "compare.stashRef",
            name: stash.summary.clone(),
            value: stash.oid.clone(),
        });
    }
    return refs;
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum CommitRefKind {
    Head,
    Branch,
    Tag,
    Stash,
}

#[derive(Clone)]
struct CommitRefChip {
    label: String,
    kind: CommitRefKind,
    remote: bool,
}

/// `use<>`：返回的元素只持有克隆出来的信号，不借用入参（edition 2024 的
/// `impl Trait` 默认捕获入参生命周期，这里显式声明不捕获）。
pub(crate) fn history(
    state: &HistoryState,
    cmd: &HistoryCommands,
    tab: Tab,
    search: &SearchCommands,
    refs: &RefsCommands,
    selected_commit: Memo<Option<(u64, String)>>,
    settings_state: Signal<store::Settings>,
) -> impl IntoElement + use<> {
    let language = cmd.language;
    let inspector_open = search.right_panel_open;
    let tab_id = tab.id;
    let commits = tab.commits.clone();
    let remote_branch_names = std::sync::Arc::new(
        tab.refs
            .remote_branches
            .iter()
            .map(|branch| (branch.full_name(), branch.name.clone()))
            .collect::<std::collections::HashMap<_, _>>(),
    );
    let compare_refs_empty =
        tab.compare_base.is_some() && tab.compare_target.is_some() && tab.commits.is_empty();
    let graph_rows = graph::layout_filtered(&commits, &tab.graph_commits);
    let graph_lane_width = graph_rows
        .iter()
        .map(|row| row.lane_count)
        .max()
        .unwrap_or(1) as f32
        * GRAPH_LANE_W;
    let table_width = signal(0.0_f32);
    let graph_width = computed(move || {
        let max_width = table_width.get() * 0.30;
        if max_width <= 0.0 {
            return GRAPH_LANE_W;
        }
        return graph_lane_width.min(max_width);
    });
    let compare = tab.compare.clone();
    // 视图回调要 `Arc<dyn Fn>`，这里把本功能的命令包一层。
    let select: std::sync::Arc<dyn Fn(usize)> = {
        let cmd = cmd.clone();
        let tab_id = tab_id;
        let commits = commits.clone();
        std::sync::Arc::new(move |index: usize| {
            if let Some(commit) = commits.get(index) {
                cmd.request_commit_selection(tab_id, commit.sha.clone());
            }
        })
    };
    let open_commit_diff: std::sync::Arc<dyn Fn(usize)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |index: usize| cmd.show_commit_diff(index))
    };
    let toggle_compare: std::sync::Arc<dyn Fn(usize)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |index: usize| cmd.toggle_compare(index))
    };
    let toggle_multi_commit: std::sync::Arc<dyn Fn(usize)> = {
        let cmd = cmd.clone();
        let commits = commits.clone();
        std::sync::Arc::new(move |index: usize| {
            if let Some(commit) = commits.get(index) {
                cmd.toggle_multi_commit_selection(tab_id, commit.sha.clone());
            }
        })
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
    // 合并提交的范围：取对比选区（至少两个）；提交列表按新到旧排列。
    let squash_range = tab.compare_range().and_then(|(newest, oldest)| {
        squash_range_for_indices(&commits, (newest..=oldest).collect())
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
    let fast_selection = state.fast_selection;
    let multi_commit_selection = state.multi_commit_selection;
    let visible = {
        let commits = commits.clone();
        computed(move || {
            return ops::visible_commit_indices(&commits, query.get().as_str(), None)
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
    let compare_refs = compare_refs_for_tab(&tab);
    let filter_open = state.filter_open;
    let filter_tab_id = state.filter_tab_id;
    let filter_position = state.filter_position;
    let filter_menu = state.filter_menu;
    let compare_mode = state.compare_mode;
    let author_column_width = state.author_column_width;
    let date_column_width = state.date_column_width;
    // 筛选浮层在 `dynamic` 里渲染，参数要能反复取用，因此这里各留一份克隆。
    let filter_state = state.clone();
    let filter_cmd = cmd.clone();
    let compare_refs_state = state.clone();
    let compare_refs_cmd = cmd.clone();
    let graph_ancestors = tab.graph_commits.clone();
    let table_ref = state.table_ref;
    let scroll_request = state.scroll_request;

    return view()
        .on_layout(move |bounds: &Bounds<Points>| {
            let width = bounds.size.width.0;
            if (table_width.get_untracked() - width).abs() > 0.5 {
                table_width.set(width);
            }
        })
        .w_full()
        .flex_1()
        .min_h_0()
        .relative()
        .flex_col()
        .bg(move || design::content())
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
                .child(view().w(move || px(graph_width.get())))
                .child(column_header(language, "history.description").flex_1())
                .child(resizable_column_header(
                    language,
                    "history.author",
                    "history.resizeAuthor",
                    author_column_width,
                    AUTHOR_MIN_W,
                    AUTHOR_MAX_W,
                    true,
                ))
                .child(resizable_column_header(
                    language,
                    "history.date",
                    "history.resizeDate",
                    date_column_width,
                    DATE_MIN_W,
                    DATE_MAX_W,
                    false,
                )),
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
                                selected_commit.get_untracked().is_some(),
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
                            let toggle_multi_commit = toggle_multi_commit.clone();
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
                            let remote_branches_for_render = remote_branch_names.clone();
                            let compare_for_render = compare_for_rows.clone();
                            let fast_selection = fast_selection;
                            let multi_selection_for_render = multi_commit_selection;
                            let visible_commits = rows_snapshot
                                .iter()
                                .map(|(_, commit)| commit.clone())
                                .collect::<Vec<_>>();
                            let graphs_for_render =
                                graph::layout_filtered(&visible_commits, &graph_ancestors);
                            let select_for_press = select.clone();
                            let toggle_for_press = toggle_compare.clone();
                            let toggle_multi_for_press = toggle_multi_commit.clone();
                            let compare_mode_for_press = compare_mode;
                            let open_diff_for_menu = open_commit_diff.clone();
                            let select_for_context = select.clone();
                            let menu_search = row_search.clone();
                            let menu_commit = commit_for_rows.clone();
                            let compare_squash_for_menu = squash_range.clone();
                            let compare_selection_for_menu = tab.compare.clone();
                            let multi_selection_for_menu = multi_commit_selection;
                            let all_commits_for_menu = commits.clone();
                            let menu_refs = refs_for_menu.clone();
                            let menu_recovery = recovery_for_menu.clone();
                            let commits_for_press = rows_snapshot.clone();
                            let commits_for_menu = rows_snapshot.clone();
                            let selected_sha = selected_commit
                                .get_untracked()
                                .filter(|(selected_tab, _)| *selected_tab == tab_id)
                                .map(|(_, sha)| sha);
                            let visible_selected = rows_snapshot.iter().position(
                                |(_, commit)| selected_sha.as_ref() == Some(&commit.sha),
                            );
                            return crate::ui::table::table_with_scroll(
                                crate::ui::table::TableSpec {
                                    columns: vec![crate::ui::table::Column {
                                        key: "commit",
                                        title: None,
                                        // 单列随中栏宽度伸缩，保留右侧作者与日期。
                                        width: None,
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
                                        let selected_commit_for_row = selected_commit;
                                        let multi_selection_for_row = multi_selection_for_render;
                                        let fast_selection_for_row = fast_selection;
                                        let row_sha = commit.sha.clone();
                                        let row_tab_id = tab_id;
                                        let row_selected = computed(move || {
                                            let fast = fast_selection_for_row.get();
                                            if fast.as_ref().is_some_and(|(tab, _)| *tab == row_tab_id) {
                                                return fast_selection_matches(row_tab_id, &row_sha, fast);
                                            }
                                            selected_commit_for_row.get().as_ref().is_some_and(
                                                |(tab, sha)| *tab == row_tab_id && sha == &row_sha,
                                            ) || multi_selection_for_row.get().as_ref().is_some_and(
                                                |selection| selection.tab_id == row_tab_id && selection.shas.contains(&row_sha),
                                            )
                                        });
                                        let selected_now: std::sync::Arc<dyn Fn() -> bool> =
                                            std::sync::Arc::new(move || row_selected.get());
                                        let in_compare = compare_for_render.contains(&index);
                                        let has_refs = !commit.refs.is_empty();
                                        let row_selected_for_bg = selected_now.clone();
                                        let row_selected_for_hover = selected_now.clone();
                                        let row_selected_for_graph = selected_now;
                                        let graph_row = graphs_for_render
                                            .get(row as usize)
                                            .cloned()
                                            .unwrap_or_default();
                                        let remote_branches = remote_branches_for_render.clone();
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
                                                } else if row_selected_for_bg() {
                                                    design::selected()
                                                } else {
                                                    AdaptiveColor::TRANSPARENT
                                                }
                                            })
                                            .hover(move |style| {
                                                if in_compare || row_selected_for_hover() {
                                                    style
                                                } else {
                                                    style.bg(move || design::hover())
                                                }
                                            })
                                            .child(graph_cell(
                                                graph_row,
                                                graph_width,
                                                move || in_compare || row_selected_for_graph(),
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
                                                            .flex_1()
                                                            .min_w_0()
                                                            .text_size(px(crate::ui::BODY))
                                                            .text_color(move || design::text())
                                                            .number_of_lines(1)
                                                            .text_overflow(
                                                                nsh_core::EllipsizeMode::Tail,
                                                            ),
                                                    )
                                                    .child(
                                                        view()
                                                            .h(px(16.0))
                                                            .min_w_0()
                                                            .flex_row()
                                                            .items_center()
                                                            .gap(px(4.0))
                                                            .when(has_refs, |this| {
                                                                this.child(ref_chips(
                                                                    commit.refs.clone(),
                                                                    remote_branches,
                                                                ))
                                                            }),
                                                    ),
                                            )
                                            .child(
                                                text(commit.author.clone())
                                                    .w(move || px(author_column_width.get()))
                                                    .min_w_0()
                                                    .flex_shrink_0()
                                                    .pl(px(8.0))
                                                    .number_of_lines(1)
                                                    .text_size(px(crate::ui::SECONDARY))
                                                    .text_color(move || design::text2())
                                                    .text_overflow(nsh_core::EllipsizeMode::Tail),
                                            )
                                            .child(
                                                text(move || {
                                                    let timestamp =
                                                        commit.date.parse().unwrap_or(0);
                                                    crate::dates::format_reactive(
                                                        settings_state,
                                                        language_value.get().as_str(),
                                                        timestamp,
                                                        now,
                                                    )
                                                })
                                                .w(move || px(date_column_width.get()))
                                                .min_w_0()
                                                .flex_shrink_0()
                                                .number_of_lines(1)
                                                .text_size(px(crate::ui::SECONDARY))
                                                .text_color(move || design::text3())
                                                .text_overflow(nsh_core::EllipsizeMode::Tail),
                                            )
                                            .into_element();
                                    })),
                                    on_row_press: Some(Box::new(move |row, platform| {
                                        let Some((index, _)) =
                                            commits_for_press.get(row as usize).cloned()
                                        else {
                                            return;
                                        };
                                        if platform {
                                            inspector_open.set(true);
                                            toggle_multi_for_press(index);
                                        } else if compare_mode_for_press.get_untracked() {
                                            toggle_for_press(index);
                                        } else {
                                            inspector_open.set(true);
                                            select_for_press(index);
                                        }
                                    })),
                                    on_context_menu: Some(Box::new(move |row, x, y| {
                                        let Some((index, commit)) =
                                            commits_for_menu.get(row as usize).cloned()
                                        else {
                                            return;
                                        };
                                        let multi_selection = multi_selection_for_menu
                                            .get_untracked()
                                            .filter(|selection| selection.tab_id == tab_id);
                                        let selected_by_multi = multi_selection
                                            .as_ref()
                                            .is_some_and(|selection| {
                                                selection.shas.contains(&commit.sha)
                                            });
                                        let selected_by_compare =
                                            compare_selection_for_menu.contains(&index);
                                        let keep_selection =
                                            selected_by_multi || selected_by_compare;
                                        let context_squash_range = if selected_by_multi {
                                            multi_selection.as_ref().and_then(|selection| {
                                                squash_range_for_shas(
                                                    &all_commits_for_menu,
                                                    &selection.shas,
                                                )
                                            })
                                        } else if selected_by_compare {
                                            compare_squash_for_menu.clone()
                                        } else {
                                            None
                                        };
                                        if !keep_selection {
                                            select_for_context(index);
                                        }
                                        let items = commit_menu(
                                            language_value,
                                            &commit,
                                            {
                                                let open_diff = open_diff_for_menu.clone();
                                                std::sync::Arc::new(move || open_diff(index))
                                            },
                                            &menu_search,
                                            &menu_recovery,
                                            context_squash_range,
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
            let menu_matches_tab = filter_tab_id.get() == Some(tab_id);
            if !menu_matches_tab || (!advanced && menu.is_none()) {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            }
            let position = filter_position.get();
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
            let content = if let Some((left, top)) = position {
                view().w_full().h_full().relative().child(
                    view()
                        .absolute()
                        .left(px(left))
                        .top(px(top))
                        .z_index(1)
                        .child(crate::ui::dialog::modality(panel)),
                )
                .into_element()
            } else {
                crate::ui::dialog::modality(panel).into_element()
            };
            return crate::ui::dialog::popover_overlay(
                move || i18n::t(language_value.get().as_str(), title_key),
                80,
                content,
                move || {
                    close_state.filter_open.set(false);
                    close_state.filter_menu.set(None);
                },
            );
        }))
        .child(dynamic(move || -> AnyElement {
            if !compare_refs_state.compare_refs_open.get() {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            }
            return compare_refs_dialog(
                language_value,
                compare_refs.clone(),
                compare_refs_state.clone(),
                compare_refs_cmd.clone(),
            );
        }))
        // 选中项变化只同步 DataTable 的活动行；History 列表和 graph snapshot 保持不变。
        .child(dynamic(move || -> AnyElement {
            let rows = visible.get();
            let selected = selected_commit.get();
            if let Some((selected_tab, sha)) = selected
                && selected_tab == tab_id
                && let Some(row) = rows
                    .iter()
                    .position(|(_, commit)| commit.sha == sha)
            {
                crate::ui::table::select_row(table_ref, row as u32);
            }
            return view().w(px(0.0)).h(px(0.0)).into_element();
        }));
}

fn hist_bar(
    language: Signal<SharedString>,
    state: &HistoryState,
    cmd: &HistoryCommands,
    tab: &Tab,
    filter_position: Signal<Option<(f32, f32)>>,
    filter_menu: Signal<Option<crate::features::history::state::FilterMenu>>,
) -> impl IntoElement + use<> {
    let query = state.query;
    let query_ref = state.query_ref;
    let filter_open = state.filter_open;
    let filter_tab_id = state.filter_tab_id;
    let compare_refs_open = state.compare_refs_open;
    let compare_refs_query = state.compare_refs_query;
    let compare_refs_draft = state.compare_refs_draft;
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
    let compare_pair = (tab.compare_base.clone(), tab.compare_target.clone());
    let compare_three_dot = tab.compare_three_dot;
    let compare_worktree = tab.compare_worktree;
    let branch_selection = applied_branches.clone();
    let branch_active = !branch_selection.is_empty();
    let branch_filter_menu = filter_menu;
    let branch_filter_open = filter_open;
    let branch_filter_position = filter_position;
    let branch_filter_tab_id = state.filter_tab_id;
    let tab_id = tab.id;
    let branch_filter_tab_for_active = branch_filter_tab_id;
    let branch_tab_id = tab_id;
    let branch_cmd = cmd.clone();
    let branch_filter = crate::ui::menu::menu_button_anchored_at(
        move || {
            if branch_selection.is_empty() {
                return i18n::t(language.get().as_str(), "filter.branch");
            }
            return i18n::with_arguments(
                language.get().as_str(),
                "filter.branchCount",
                &[("count", branch_selection.len().to_string())],
            );
        },
        move || {
            branch_active
                || (branch_filter_menu.get()
                    == Some(crate::features::history::state::FilterMenu::Branches)
                    && branch_filter_tab_for_active.get() == Some(branch_tab_id))
        },
        Some("chev-down"),
        move |position, local_position| {
            if branch_filter_menu.get_untracked()
                == Some(crate::features::history::state::FilterMenu::Branches)
                && branch_filter_tab_id.get_untracked() == Some(tab_id)
            {
                branch_filter_menu.set(None);
                return;
            }
            branch_cmd.begin_quick_filter(tab_id);
            branch_filter_open.set(false);
            branch_filter_position.set(Some(filter_popover_position(
                position,
                local_position,
                QUICK_FILTER_W,
                QUICK_FILTER_W,
            )));
            branch_filter_menu.set(Some(crate::features::history::state::FilterMenu::Branches));
        },
    );
    let author_selection = applied.authors.clone();
    let author_active = !author_selection.is_empty();
    let author_filter_menu = filter_menu;
    let author_filter_open = filter_open;
    let author_filter_position = filter_position;
    let author_filter_tab_id = state.filter_tab_id;
    let author_filter_tab_for_active = author_filter_tab_id;
    let author_tab_id = tab_id;
    let author_cmd = cmd.clone();
    let author_filter = crate::ui::menu::menu_button_anchored_at(
        move || {
            if author_selection.is_empty() {
                return i18n::t(language.get().as_str(), "filter.author");
            }
            return i18n::with_arguments(
                language.get().as_str(),
                "filter.authorCount",
                &[("count", author_selection.len().to_string())],
            );
        },
        move || {
            author_active
                || (author_filter_menu.get()
                    == Some(crate::features::history::state::FilterMenu::Authors)
                    && author_filter_tab_for_active.get() == Some(author_tab_id))
        },
        Some("chev-down"),
        move |position, local_position| {
            if author_filter_menu.get_untracked()
                == Some(crate::features::history::state::FilterMenu::Authors)
                && author_filter_tab_id.get_untracked() == Some(tab_id)
            {
                author_filter_menu.set(None);
                return;
            }
            author_cmd.begin_quick_filter(tab_id);
            author_filter_open.set(false);
            author_filter_position.set(Some(filter_popover_position(
                position,
                local_position,
                QUICK_FILTER_W,
                QUICK_FILTER_W,
            )));
            author_filter_menu.set(Some(crate::features::history::state::FilterMenu::Authors));
        },
    );
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
            view()
                .h(px(crate::ui::CONTROL_SMALL))
                .flex_row()
                .items_center()
                .child(
                    text(move || i18n::t(language.get().as_str(), "shell.history"))
                        .text_size(px(crate::ui::BODY))
                        .line_height(px(crate::ui::BODY_LINE))
                        .font_weight(FontWeight(600))
                        .text_color(move || design::text()),
                ),
        )
        .child(
            view()
                .h(px(crate::ui::CONTROL_SMALL))
                .flex_row()
                .items_center()
                .child(
                    text(move || {
                        i18n::with_arguments(
                            language.get().as_str(),
                            "history.commits",
                            &[("count", total.to_string())],
                        )
                    })
                    .text_size(px(crate::ui::SECONDARY))
                    .line_height(px(crate::ui::SECONDARY_LINE))
                    .text_color(move || design::text3()),
                ),
        )
        // 快速搜索与已应用筛选独立；输入立即筛选提交，清空按钮只在有内容时出现。
        .child(
            view().w(px(128.0)).child(
                crate::ui::text_input::search_field_clearable_ref(
                    query,
                    move || i18n::t(language.get().as_str(), "history.filter"),
                    move || i18n::t(language.get().as_str(), "history.clear"),
                    move |value| query.set(value.to_string()),
                    &query_ref,
                )
                .h(px(crate::ui::CONTROL_SMALL)),
            ),
        )
        .child(branch_filter)
        .child(author_filter)
        .child(widgets_icon(
            language,
            filter_open,
            filter_position,
            filter_menu,
            filter_active,
            filter_tab_id,
            tab_id,
            cmd.clone(),
        ))
        .child(view().flex_1())
        .child({
            let compare_refs_active =
                (compare_pair.0.is_some() && compare_pair.1.is_some()) || compare_worktree;
            let variant = if compare_refs_active {
                crate::ui::button::ButtonVariant::Secondary
            } else {
                crate::ui::button::ButtonVariant::Ghost
            };
            crate::ui::button::button_icon(
                move || i18n::t(language.get().as_str(), "compare.mode"),
                "diff",
                variant,
                crate::ui::button::ButtonSize::Toolbar,
                false,
                move || {
                    compare_refs_query.set(String::new());
                    compare_refs_draft.set(crate::features::history::state::CompareRefsDraft {
                        base: compare_pair.0.clone(),
                        target: compare_pair.1.clone(),
                        three_dot: compare_three_dot,
                        worktree: compare_worktree,
                    });
                    filter_open.set(false);
                    filter_menu.set(None);
                    compare_refs_open.set(true);
                },
            )
        });
}

fn widgets_icon(
    language: Signal<SharedString>,
    filter_open: Signal<bool>,
    filter_position: Signal<Option<(f32, f32)>>,
    filter_menu: Signal<Option<crate::features::history::state::FilterMenu>>,
    filter_active: bool,
    filter_tab_id: Signal<Option<u64>>,
    tab_id: u64,
    cmd: HistoryCommands,
) -> impl IntoElement {
    let foreground = std::rc::Rc::new(move || {
        if (filter_open.get() && filter_tab_id.get() == Some(tab_id)) || filter_active {
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
        move |position, local_position| {
            if filter_open.get_untracked() && filter_tab_id.get_untracked() == Some(tab_id) {
                filter_open.set(false);
                return;
            }
            // 完整过滤浮层在筛选按钮下方右对齐。
            cmd.begin_filter_for_tab(tab_id);
            filter_menu.set(None);
            filter_position.set(Some(filter_popover_position(
                position,
                local_position,
                24.0,
                POP_W,
            )));
            filter_open.set(true);
        },
    );
}

fn filter_popover_position(
    page_position: Point<Points>,
    local_position: Point<Points>,
    trigger_width: f32,
    width: f32,
) -> (f32, f32) {
    let trigger_left = page_position.x.0 - local_position.x.0;
    let trigger_top = page_position.y.0 - local_position.y.0;
    let left = (trigger_left + trigger_width - width).max(8.0);
    return (left, trigger_top + crate::ui::CONTROL_SMALL + 4.0);
}

fn column_header(language: Signal<SharedString>, key: &'static str) -> nsh_core::element::Text {
    return text(move || i18n::t(language.get().as_str(), key))
        .number_of_lines(1)
        .text_size(px(crate::ui::SECONDARY))
        .font_weight(FontWeight(600))
        .text_color(move || design::text3());
}

fn resizable_column_header(
    language: Signal<SharedString>,
    title_key: &'static str,
    resize_label_key: &'static str,
    width: Signal<f32>,
    min_width: f32,
    max_width: f32,
    leading_divider: bool,
) -> impl IntoElement {
    let drag_start = signal(width.get_untracked());
    let update_width: std::rc::Rc<dyn Fn(&Points)> = std::rc::Rc::new(move |offset: &Points| {
        width.set(super::state::resized_column_width(
            drag_start.get_untracked(),
            offset.0,
            min_width,
            max_width,
        ));
    });
    let update_on_move = update_width.clone();
    let update_on_end = update_width;
    let mut header = view()
        .relative()
        .w(move || px(width.get()))
        .h_full()
        .flex_row()
        .items_center()
        .flex_shrink_0()
        .child(column_header(language, title_key).w_full().text_center());
    if leading_divider {
        header = header
            .border_l_1()
            .border_color(move || design::line_soft());
    }
    return header.child(crate::ui::table::column_resize_handle(
        SharedString::from(i18n::t(language.get_untracked().as_str(), resize_label_key)),
        move || drag_start.set(width.get_untracked()),
        move |offset| update_on_move(offset),
        move |offset| update_on_end(offset),
    ));
}

fn graph_cell(
    graph: GraphRow,
    width: Memo<f32>,
    highlight: impl Fn() -> bool + 'static,
) -> impl IntoElement {
    let row_height = metrics::COMMIT_ROW_H;
    let middle = row_height / 2.0;
    let node_radius = GRAPH_NODE_D / 2.0;
    let mut cell = view()
        .relative()
        .w(move || px(width.get().max(GRAPH_LANE_W)))
        .h(px(row_height))
        .overflow_hidden();
    for lane in graph.top_lanes {
        let x = lane as f32 * GRAPH_LANE_W + GRAPH_LANE_W / 2.0 - 0.5;
        let bottom = if lane == graph.node_lane {
            middle - node_radius
        } else {
            middle
        };
        if bottom <= 0.0 {
            continue;
        }
        cell = cell.child(
            view()
                .absolute()
                .left(px(x))
                .top(px(0.0))
                .w(px(1.0))
                .h(px(bottom))
                .bg(move || design::graph_color(lane)),
        );
    }
    for lane in graph.bottom_lanes {
        let x = lane as f32 * GRAPH_LANE_W + GRAPH_LANE_W / 2.0 - 0.5;
        let top = if lane == graph.node_lane {
            middle + node_radius
        } else {
            middle
        };
        if top >= row_height {
            continue;
        }
        cell = cell.child(
            view()
                .absolute()
                .left(px(x))
                .top(px(top))
                .w(px(1.0))
                .h(px(row_height - top))
                .bg(move || design::graph_color(lane)),
        );
    }
    for parent_lane in graph.parent_lanes {
        let node_center = graph.node_lane as f32 * GRAPH_LANE_W + GRAPH_LANE_W / 2.0;
        let parent_center = parent_lane as f32 * GRAPH_LANE_W + GRAPH_LANE_W / 2.0;
        let left = if parent_lane < graph.node_lane {
            parent_center
        } else {
            node_center + node_radius
        };
        let right = if parent_lane < graph.node_lane {
            node_center - node_radius
        } else {
            parent_center
        };
        let width = (right - left).max(0.0);
        let color = graph.node_lane;
        cell = cell.child(
            view()
                .absolute()
                .left(px(left))
                .top(px(middle - 0.5))
                .w(px(width))
                .h(px(1.0))
                .bg(move || design::graph_color(color)),
        );
    }
    let node_left = graph.node_lane as f32 * GRAPH_LANE_W + GRAPH_LANE_W / 2.0 - node_radius;
    return cell.child(
        view()
            .absolute()
            .left(px(node_left))
            .top(px(middle - node_radius))
            .w(px(GRAPH_NODE_D))
            .h(px(GRAPH_NODE_D))
            .rounded(px(node_radius + 0.5))
            .bg(move || {
                if highlight() {
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

/// 按连续提交 SHA 生成可 Squash 的端点范围；不连续选择会被拒绝。
fn squash_range_for_shas(
    commits: &[domain::Commit],
    shas: &[String],
) -> Option<(String, String, Vec<(String, String)>)> {
    if shas.len() < 2 {
        return None;
    }
    let indices = shas
        .iter()
        .map(|sha| commits.iter().position(|commit| &commit.sha == sha))
        .collect::<Option<Vec<_>>>()?;
    return squash_range_for_indices(commits, indices);
}

fn squash_range_for_indices(
    commits: &[domain::Commit],
    mut indices: Vec<usize>,
) -> Option<(String, String, Vec<(String, String)>)> {
    indices.sort_unstable();
    indices.dedup();
    if indices.len() < 2 {
        return None;
    }
    let newest = indices[0];
    let oldest = *indices.last()?;
    // Squash 会应用整个端点区间；不连续的 ⌘ 多选不能隐式包含未选中的提交。
    if oldest - newest + 1 != indices.len() {
        return None;
    }
    let from = commits.get(oldest)?.sha.clone();
    let to = commits.get(newest)?.sha.clone();
    let selected = (newest..=oldest)
        .map(|index| {
            commits
                .get(index)
                .map(|commit| (commit.short.clone(), commit.summary.clone()))
        })
        .collect::<Option<Vec<_>>>()?;
    return Some((from, to, selected));
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
    let filter_menu = state.filter_menu;
    let filter_open = state.filter_open;
    let query = signal(String::new());
    let choices: Vec<(String, String)> = match kind {
        super::state::FilterMenu::Branches => branches
            .into_iter()
            .map(|b| {
                (
                    b.reference,
                    if b.remote {
                        format!(
                            "{} · {}",
                            i18n::t(language.get_untracked().as_str(), "filter.remote"),
                            b.label
                        )
                    } else {
                        b.label
                    },
                )
            })
            .collect(),
        super::state::FilterMenu::Authors => authors
            .into_iter()
            .map(|a| (a.email.clone(), format!("{} <{}>", a.name, a.email)))
            .collect(),
    };
    let visible_choices = computed(move || {
        let search = query.get().trim().to_lowercase();
        choices
            .iter()
            .filter(|(_, label)| label.to_lowercase().contains(&search))
            .cloned()
            .collect::<Vec<_>>()
    });
    let reset = cmd.clone();
    let apply = cmd.clone();
    let toggle = cmd.clone();
    view()
        .w(px(QUICK_FILTER_W))
        .p(px(10.0))
        .flex_col()
        .gap(px(crate::ui::GAP))
        .rounded(px(metrics::RADIUS_PANEL))
        .border_1()
        .border_color(design::line_strong)
        .bg(|| design::token("popover"))
        .shadow_lg()
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .gap(px(crate::ui::GAP_SM))
                .child(
                    view()
                        .flex_1()
                        .min_w_0()
                        .child(crate::ui::text_input::text_input(
                            query,
                            move || i18n::t(language.get().as_str(), "filter.search"),
                            false,
                            move |value| query.set(value.to_string()),
                        )),
                )
                .child(crate::ui::button::icon_button(
                    "x",
                    move || i18n::t(language.get().as_str(), "shell.close"),
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || {
                        filter_open.set(false);
                        filter_menu.set(None);
                    },
                )),
        )
        .child(
            scroll().w_full().h(px(288.0)).child(
                view()
                    .w_full()
                    .flex_col()
                    .gap(px(crate::ui::GAP_XS))
                    .child(when(
                        move || visible_choices.with(|choices| choices.is_empty()),
                        move || {
                            text(move || i18n::t(language.get().as_str(), "filter.noResults"))
                                .text_size(px(crate::ui::SECONDARY))
                                .text_color(design::text3)
                        },
                    ))
                    .child(each(
                        move || visible_choices.get(),
                        |(id, _): &(String, String)| id.clone(),
                        move |choice, _| {
                            let toggle = toggle.clone();
                            let selected = computed(move || {
                                choice.with(|(id, _)| {
                                    draft.with(|draft| match kind {
                                        super::state::FilterMenu::Branches => {
                                            draft.references.contains(id)
                                        }
                                        super::state::FilterMenu::Authors => {
                                            draft.authors.contains(id)
                                        }
                                    })
                                })
                            });
                            dynamic(move || {
                                let (id, label) = choice.get();
                                let toggle = toggle.clone();
                                let is_selected = selected.get();
                                let leading =
                                    Some(crate::ui::check::indicator(is_selected).into_element());
                                let on_press = move || match kind {
                                    super::state::FilterMenu::Branches => {
                                        toggle.toggle_branch_filter(id.clone())
                                    }
                                    super::state::FilterMenu::Authors => {
                                        toggle.toggle_author_filter(id.clone())
                                    }
                                };
                                if kind == super::state::FilterMenu::Branches {
                                    crate::ui::row::row_head_ellipsized(
                                        move || label.clone(),
                                        None,
                                        leading,
                                        vec![],
                                        is_selected,
                                        false,
                                        on_press,
                                    )
                                    .into_element()
                                } else {
                                    crate::ui::row::row(
                                        move || label.clone(),
                                        None,
                                        leading,
                                        vec![],
                                        is_selected,
                                        false,
                                        on_press,
                                    )
                                    .into_element()
                                }
                            })
                        },
                    )),
            ),
        )
        .child(
            view()
                .w_full()
                .pt(px(crate::ui::GAP))
                .border_t_1()
                .border_color(design::line)
                .flex_row()
                .justify_end()
                .gap(px(crate::ui::GAP_SM))
                .child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "filter.reset"),
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || {
                        match kind {
                            super::state::FilterMenu::Branches => reset.clear_branch_filter(),
                            super::state::FilterMenu::Authors => reset.clear_author_filter(),
                        }
                        reset.apply_quick_filter();
                    },
                ))
                .child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "filter.apply"),
                    crate::ui::button::ButtonVariant::Primary,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || apply.apply_quick_filter(),
                )),
        )
}

fn filter_pop(
    language: Signal<SharedString>,
    _branches: Vec<FilterBranch>,
    _authors: Vec<git::Author>,
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
        .child(section_label(language, "filter.whenLabel"))
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
                .child(view().flex_1())
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
                .child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "filter.apply"),
                    crate::ui::button::ButtonVariant::Primary,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    {
                        let apply = apply.clone();
                        move || apply()
                    },
                )),
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
fn ref_chips(
    refs: Vec<String>,
    remote_branches: std::sync::Arc<std::collections::HashMap<String, String>>,
) -> impl IntoElement {
    let mut chips: Vec<AnyElement> = Vec::new();
    let mut references = Vec::<CommitRefChip>::new();
    let mut branch_indices = std::collections::HashMap::<String, usize>::new();
    let mut seen_tags = std::collections::HashSet::<String>::new();
    let mut seen_stashes = std::collections::HashSet::<String>::new();
    let mut has_head = false;
    for reference in refs {
        let label = if let Some(target) = reference.strip_prefix("HEAD -> ") {
            if !has_head {
                has_head = true;
                references.push(CommitRefChip {
                    label: String::from("HEAD"),
                    kind: CommitRefKind::Head,
                    remote: false,
                });
            }
            target.to_string()
        } else if reference == "HEAD" {
            if !has_head {
                has_head = true;
                references.push(CommitRefChip {
                    label: String::from("HEAD"),
                    kind: CommitRefKind::Head,
                    remote: false,
                });
            }
            continue;
        } else {
            reference
        };
        if let Some(label) = label.strip_prefix("tag: ") {
            if seen_tags.insert(label.to_string()) {
                references.push(CommitRefChip {
                    label: label.to_string(),
                    kind: CommitRefKind::Tag,
                    remote: false,
                });
            }
            continue;
        }
        if let Some(label) = label.strip_prefix("stash: ") {
            if seen_stashes.insert(label.to_string()) {
                references.push(CommitRefChip {
                    label: label.to_string(),
                    kind: CommitRefKind::Stash,
                    remote: false,
                });
            }
            continue;
        }

        let remote_name = remote_branches.get(&label);
        let remote = remote_name.is_some();
        let branch_name = remote_name.cloned().unwrap_or_else(|| label.clone());
        if let Some(existing) = branch_indices
            .get(&branch_name)
            .and_then(|index| references.get_mut(*index))
        {
            existing.remote |= remote;
            // 本地分支名优先显示；多个远端别名仍合并为同一个徽章和图标。
            if !remote {
                existing.label = branch_name;
            }
            continue;
        }
        branch_indices.insert(branch_name.clone(), references.len());
        let chip = CommitRefChip {
            label: if remote { label } else { branch_name.clone() },
            kind: CommitRefKind::Branch,
            remote,
        };
        references.push(chip);
    }

    references.sort_by_key(|chip| match chip.kind {
        CommitRefKind::Head => 0,
        CommitRefKind::Branch => 1,
        CommitRefKind::Tag => 2,
        CommitRefKind::Stash => 3,
    });
    for chip in references.into_iter().take(3) {
        let is_head = chip.kind == CommitRefKind::Head;
        let is_branch = chip.kind == CommitRefKind::Branch;
        let is_tag = chip.kind == CommitRefKind::Tag;
        let is_stash = chip.kind == CommitRefKind::Stash;
        let is_remote = chip.remote;
        let branch_color = branch_color(&chip.label);
        let branch_ink = branch_badge_ink(branch_color);
        let branch_wash = branch_ink.opacity(0.12);
        let tooltip = crate::ui::button::text_tooltip(chip.label.clone());
        let label = chip.label;
        let mut badge = view()
            .h(px(16.0))
            .max_w(px(140.0))
            .min_w_0()
            .px(px(4.0))
            .flex_row()
            .items_center()
            .justify_center()
            .gap(px(2.0))
            .rounded(px(3.0))
            .border_1()
            .border_color(move || {
                if is_head {
                    design::warn()
                } else if is_tag {
                    design::tag_border()
                } else if is_branch {
                    branch_ink
                } else {
                    design::line_soft()
                }
            })
            .bg(move || {
                if is_head || is_tag {
                    design::tag_bg()
                } else if is_branch {
                    branch_wash
                } else if is_stash || is_remote {
                    design::panel2()
                } else {
                    design::branch_bg()
                }
            });
        if is_remote {
            badge = badge.child(crate::ui::icon::icon_view(
                "globe",
                crate::ui::icon::IconSize::Compact,
                move || branch_ink,
            ));
        }
        chips.push(
            badge
                .child(
                    text(label)
                        .flex_1()
                        .min_w_0()
                        .number_of_lines(1)
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(move || {
                            if is_head {
                                design::warn()
                            } else if is_tag {
                                design::tag_fg()
                            } else if is_branch {
                                branch_ink
                            } else if is_stash || is_remote {
                                design::remote_fg()
                            } else {
                                design::branch_fg()
                            }
                        })
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .tooltip(tooltip)
                .into_element(),
        );
    }
    return view()
        .min_w_0()
        .flex_row()
        .items_center()
        .gap(px(4.0))
        .child(chips);
}

/// 稳定地把分支名映射到图线使用的四种设计颜色。
fn branch_color(name: &str) -> usize {
    let hash = name.bytes().fold(2_166_136_261_u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(16_777_619)
    });
    return hash as usize % 4;
}

fn branch_badge_ink(index: usize) -> AdaptiveColor {
    return softened_branch_badge_ink(design::graph_color(index));
}

fn softened_branch_badge_ink(color: AdaptiveColor) -> AdaptiveColor {
    return color.map(|color| color.lighten(0.18));
}

fn fast_selection_matches(tab_id: u64, sha: &str, selection: Option<(u64, String)>) -> bool {
    return selection
        .is_some_and(|(selected_tab, selected_sha)| selected_tab == tab_id && selected_sha == sha);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[component]
    fn RefChipsFixture() -> impl IntoElement {
        let refs = [
            "HEAD -> main",
            "main",
            "origin/main",
            "upstream/main",
            "tag: v1.0",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let remotes = std::collections::HashMap::from([
            (String::from("origin/main"), String::from("main")),
            (String::from("upstream/main"), String::from("main")),
        ]);
        return ref_chips(refs, std::sync::Arc::new(remotes));
    }

    #[test]
    fn current_commit_has_a_visible_head_badge() {
        let mut app = nsh_testing::Headless::start(nsh_engine::EngineConfig {
            viewport: size(px(400.0), px(120.0)),
            ..nsh_engine::EngineConfig::new(nsh_core::element::Root::of::<RefChipsFixture>())
        })
        .unwrap();
        app.first_render().unwrap();
        app.wait_for("HEAD").unwrap();
        app.wait_for("v1.0").unwrap();
        let main_count = app
            .screen()
            .layout_nodes()
            .iter()
            .filter(|node| {
                node.text
                    .as_ref()
                    .is_some_and(|text| text.text.text == "main")
            })
            .count();
        assert_eq!(
            main_count, 1,
            "local and remote refs share one branch badge"
        );
    }

    #[test]
    fn branch_colors_are_stable_and_vary_across_names() {
        let names = [
            "main",
            "develop",
            "feature/history",
            "fix/diff-scroll",
            "release/2.0",
            "origin/main",
            "topic/rebase",
        ];
        let colors = names
            .iter()
            .map(|name| branch_color(name))
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(
            branch_color("feature/history"),
            branch_color("feature/history")
        );
        assert!(colors.len() > 1);
        assert!(colors.iter().all(|color| *color < 4));
    }

    #[test]
    fn branch_badge_ink_is_lighter_than_graph_lines() {
        let source = AdaptiveColor::rgb(0x2E9E6B, 0x6FCC9D);
        let badge = softened_branch_badge_ink(source);
        assert!(badge.light.l > source.light.l);
        assert!(badge.dark.l > source.dark.l);
    }

    #[test]
    fn fast_selection_is_scoped_to_its_tab_and_commit() {
        let selection = Some((12, String::from("commit-a")));
        assert!(fast_selection_matches(12, "commit-a", selection.clone()));
        assert!(!fast_selection_matches(13, "commit-a", selection.clone()));
        assert!(!fast_selection_matches(12, "commit-b", selection));
        assert!(!fast_selection_matches(12, "commit-a", None));
    }
}

/// 可搜索的 Compare refs 窗口：端点与比较选项先保存在草稿，Done 时一次应用。
fn compare_refs_dialog(
    language: Signal<SharedString>,
    refs: Vec<CompareRef>,
    state: HistoryState,
    cmd: HistoryCommands,
) -> AnyElement {
    let query = state.compare_refs_query;
    let draft = state.compare_refs_draft;
    let open = state.compare_refs_open;
    let results_refs = refs;
    let results_draft = draft;
    let results_query = query;
    let search = crate::ui::text_input::text_input(
        query,
        move || i18n::t(language.get().as_str(), "compare.searchRefs"),
        false,
        move |value| query.set(value.to_string()),
    );
    let rows = dynamic(move || -> AnyElement {
        let search = results_query.get().trim().to_lowercase();
        let locale = language.get();
        let mut rows = Vec::new();
        for reference in &results_refs {
            let label = i18n::with_arguments(
                locale.as_str(),
                reference.label_key,
                &[("name", reference.name.clone())],
            );
            if !label.to_lowercase().contains(&search)
                && !reference.name.to_lowercase().contains(&search)
                && !reference.value.to_lowercase().contains(&search)
            {
                continue;
            }
            let base_reference = reference.clone();
            let target_reference = reference.clone();
            let row_draft = results_draft;
            let row_draft_for_target = row_draft;
            rows.push(
                view()
                    .w_full()
                    .h(px(crate::ui::ROW + 2.0))
                    .flex_row()
                    .items_center()
                    .gap(px(crate::ui::GAP_SM))
                    .px(px(crate::ui::GAP_SM))
                    .border_b_1()
                    .border_color(move || design::line_soft())
                    .child(
                        text(move || label.clone())
                            .flex_1()
                            .min_w_0()
                            .text_size(px(crate::ui::BODY))
                            .text_color(move || design::text())
                            .number_of_lines(1)
                            .text_overflow(nsh_core::EllipsizeMode::Tail),
                    )
                    .child(
                        view()
                            .w(px(COMPARE_REF_ACTION_W))
                            .flex_row()
                            .justify_center()
                            .child(dynamic(move || -> AnyElement {
                                let selected = row_draft.get().base.as_deref()
                                    == Some(base_reference.value.as_str());
                                let value = base_reference.value.clone();
                                crate::ui::button::button(
                                    move || i18n::t(language.get().as_str(), "compare.base"),
                                    if selected {
                                        crate::ui::button::ButtonVariant::Primary
                                    } else {
                                        crate::ui::button::ButtonVariant::Ghost
                                    },
                                    crate::ui::button::ButtonSize::Small,
                                    false,
                                    move || {
                                        row_draft.update(|selection| {
                                            selection.base = Some(value.clone())
                                        })
                                    },
                                )
                                .into_element()
                            })),
                    )
                    .child(
                        view()
                            .w(px(COMPARE_REF_ACTION_W))
                            .flex_row()
                            .justify_center()
                            .child(dynamic(move || -> AnyElement {
                                let selected = row_draft_for_target.get().target.as_deref()
                                    == Some(target_reference.value.as_str());
                                let value = target_reference.value.clone();
                                crate::ui::button::button(
                                    move || i18n::t(language.get().as_str(), "compare.target"),
                                    if selected {
                                        crate::ui::button::ButtonVariant::Primary
                                    } else {
                                        crate::ui::button::ButtonVariant::Ghost
                                    },
                                    crate::ui::button::ButtonSize::Small,
                                    false,
                                    move || {
                                        row_draft_for_target.update(|selection| {
                                            selection.target = Some(value.clone())
                                        })
                                    },
                                )
                                .into_element()
                            })),
                    )
                    .into_element(),
            );
        }
        if rows.is_empty() {
            return view()
                .w_full()
                .h(px(crate::ui::ROW * 3.0))
                .flex_row()
                .items_center()
                .justify_center()
                .child(
                    text(move || i18n::t(language.get().as_str(), "compare.noRefs"))
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(move || design::text3()),
                )
                .into_element();
        }
        return view().w_full().flex_col().children(rows).into_element();
    });
    let worktree = dynamic(move || -> AnyElement {
        let selection = draft.get();
        return crate::ui::button::button(
            move || i18n::t(language.get().as_str(), "compare.withWorkingTree"),
            if selection.worktree {
                crate::ui::button::ButtonVariant::Secondary
            } else {
                crate::ui::button::ButtonVariant::Ghost
            },
            crate::ui::button::ButtonSize::Regular,
            selection.target.is_none(),
            move || draft.update(|value| value.worktree = !value.worktree),
        )
        .into_element();
    });
    let mode_buttons = view()
        .flex_row()
        .items_center()
        .gap(px(crate::ui::GAP_XS))
        .child(dynamic(move || -> AnyElement {
            let selected = !draft.get().three_dot;
            return crate::ui::button::button(
                move || i18n::t(language.get().as_str(), "compare.twoDot"),
                if selected {
                    crate::ui::button::ButtonVariant::Secondary
                } else {
                    crate::ui::button::ButtonVariant::Ghost
                },
                crate::ui::button::ButtonSize::Regular,
                false,
                move || draft.update(|value| value.three_dot = false),
            )
            .into_element();
        }))
        .child(dynamic(move || -> AnyElement {
            let selection = draft.get();
            return crate::ui::button::button(
                move || i18n::t(language.get().as_str(), "compare.threeDot"),
                if selection.three_dot {
                    crate::ui::button::ButtonVariant::Secondary
                } else {
                    crate::ui::button::ButtonVariant::Ghost
                },
                crate::ui::button::ButtonSize::Regular,
                selection.worktree,
                move || draft.update(|value| value.three_dot = true),
            )
            .into_element();
        }));
    let body = view()
        .w_full()
        .flex_col()
        .min_h_0()
        .gap(px(crate::ui::GAP))
        .child(search)
        .child(
            view()
                .w_full()
                .h(px(crate::ui::ROW))
                .flex_row()
                .items_center()
                .gap(px(crate::ui::GAP_SM))
                .px(px(crate::ui::GAP_SM))
                .child(
                    text(move || i18n::t(language.get().as_str(), "compare.reference"))
                        .flex_1()
                        .text_size(px(crate::ui::SECONDARY))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(move || design::text3()),
                )
                .child(
                    view()
                        .w(px(COMPARE_REF_ACTION_W))
                        .flex_row()
                        .justify_center()
                        .child(
                            text(move || i18n::t(language.get().as_str(), "compare.base"))
                                .text_size(px(crate::ui::SECONDARY))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(move || design::text3()),
                        ),
                )
                .child(
                    view()
                        .w(px(COMPARE_REF_ACTION_W))
                        .flex_row()
                        .justify_center()
                        .child(
                            text(move || i18n::t(language.get().as_str(), "compare.target"))
                                .text_size(px(crate::ui::SECONDARY))
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(move || design::text3()),
                        ),
                ),
        )
        .child(scroll().w_full().h(px(COMPARE_REF_LIST_H)).child(rows))
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .gap(px(crate::ui::GAP_SM))
                .child(mode_buttons)
                .child(view().flex_1())
                .child(worktree),
        );
    let footer_cmd = cmd.clone();
    let dismiss_open = open;
    let done_open = open;
    let swap_draft = draft;
    let clear_draft = draft;
    let footer = vec![
        dynamic(move || -> AnyElement {
            let selection = swap_draft.get();
            return crate::ui::button::button(
                move || i18n::t(language.get().as_str(), "compare.swap"),
                crate::ui::button::ButtonVariant::Secondary,
                crate::ui::button::ButtonSize::Regular,
                selection.base.is_none() || selection.target.is_none() || selection.worktree,
                move || {
                    swap_draft.update(|selection| {
                        std::mem::swap(&mut selection.base, &mut selection.target);
                    })
                },
            )
            .into_element();
        }),
        dynamic(move || -> AnyElement {
            let selection = clear_draft.get();
            return crate::ui::button::button(
                move || i18n::t(language.get().as_str(), "compare.clear"),
                crate::ui::button::ButtonVariant::Secondary,
                crate::ui::button::ButtonSize::Regular,
                selection.base.is_none() && selection.target.is_none() && !selection.worktree,
                move || {
                    clear_draft.update(|selection| {
                        selection.base = None;
                        selection.target = None;
                        selection.worktree = false;
                    })
                },
            )
            .into_element();
        }),
        crate::ui::button::button(
            move || i18n::t(language.get().as_str(), "compare.done"),
            crate::ui::button::ButtonVariant::Primary,
            crate::ui::button::ButtonSize::Regular,
            false,
            move || {
                let selection = draft.get_untracked();
                footer_cmd.apply_compare_refs(
                    selection.base,
                    selection.target,
                    selection.three_dot,
                    selection.worktree,
                );
                done_open.set(false);
            },
        )
        .into_element(),
    ];
    return crate::ui::dialog::dialog_with_close(
        true,
        COMPARE_DIALOG_W,
        move || i18n::t(language.get().as_str(), "compare.dialogTitle"),
        move || i18n::t(language.get().as_str(), "shell.close"),
        body,
        footer,
        move || dismiss_open.set(false),
    );
}
