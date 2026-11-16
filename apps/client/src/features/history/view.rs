//! 历史表：`.hist` 256 高，含工具条（32）、筛选浮层、列标题（23）与 28 高的提交行。
//!
//! 行内是设计稿的四列：图线 64、说明（含引用徽章）、作者 96、时间 58。工具条上的
//! 芯片显示当前筛选条件；点筛选图标打开浮层（设计板 05）。对比模式下点击提交是
//! 加入/移出对比集合（设计板 06）。
//!
//! 入口 `history` 收本功能的状态与用例；改写与创建初始提交走 `cmd.commit`，
//! 拣选/回滚/重置走 `cmd.recovery`，新建分支/标签的表单与复制走 `features::search`
//! 与 `shared::os`。

use super::commands::HistoryCommands;
use super::state::HistoryState;
use crate::design::{self, metrics};
use crate::features::commit::commands::CommitCommands;
use crate::features::refs::commands::RefsCommands;
use crate::filters::When;
use crate::i18n;
use crate::app::session::Tab;
use crate::ui::badge::kbd;
use crate::features::search::commands::SearchCommands;
use nsh_core::element::desktop::MenuItem;
use nsh_core::prelude::*;

const AUTHOR_W: f32 = 96.0;
const DATE_W: f32 = 58.0;
const GRAPH_W: f32 = 64.0;

/// 浮层宽度（设计板 `.filter-pop`）。
const POP_W: f32 = 268.0;

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
    let commits = tab.commits.clone();
    let selected = tab.selected;
    let compare = tab.compare.clone();
    // 视图回调要 `Arc<dyn Fn>`，这里把本功能的命令包一层。
    let select: std::sync::Arc<dyn Fn(usize)> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move |index: usize| cmd.select_commit(index))
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
            let needle = query.get().to_lowercase();
            let mut rows: Vec<(usize, domain::Commit)> = Vec::new();
            for (index, commit) in commits.iter().enumerate() {
                if needle.is_empty()
                    || commit.summary.to_lowercase().contains(&needle)
                    || commit.author.to_lowercase().contains(&needle)
                    || commit.short.contains(&needle)
                    || commit
                        .refs
                        .iter()
                        .any(|reference| reference.to_lowercase().contains(&needle))
                {
                    rows.push((index, commit.clone()));
                }
            }
            return rows;
        })
    };
    let branches: Vec<String> = tab
        .refs
        .branches
        .iter()
        .map(|branch| branch.name.clone())
        .collect();
    let authors = tab.authors.clone();
    let language_value = language;
    let filter_open = state.filter_open;
    let compare_mode = state.compare_mode;
    // 筛选浮层在 `dynamic` 里渲染，参数要能反复取用，因此这里各留一份克隆。
    let filter_state = state.clone();
    let filter_cmd = cmd.clone();

    return view()
        .w_full()
        .h(px(metrics::HISTORY_H))
        .flex_shrink_0()
        .flex_col()
        .bg(move || design::content())
        .border_b_1()
        .border_color(move || design::line())
        .child(hist_bar(language, state, cmd, &tab))
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
                .child(view().w(px(GRAPH_W - 8.0)))
                .child(column_header(language, "history.description").flex_1())
                .child(column_header(language, "history.author").w(px(AUTHOR_W)))
                .child(column_header(language, "history.date").w(px(DATE_W))),
        )
        .child(
            view().w_full().flex_1().min_h_0().child(
                scroll().w_full().h_full().child(
                    view()
                        .w_full()
                        .flex_col()
                        .child(dynamic(move || -> AnyElement {
                            // 后台还在读：先画骨架行，避免看到「空仓库」的假象。
                            if loading {
                                let mut skeleton: Vec<AnyElement> = Vec::new();
                                for index in 0..8 {
                                    skeleton.push(skeleton_row(index));
                                }
                                return view()
                                    .w_full()
                                    .flex_col()
                                    .child(skeleton)
                                    .into_element();
                            }
                            if visible.get().is_empty() {
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
                                            let key = if total == 0 {
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
                            let rows_snapshot = visible.get();
                            let rows: Vec<Vec<String>> = rows_snapshot
                                .iter()
                                .map(|(index, _commit)| vec![index.to_string()])
                                .collect();
                            let commits_for_render = rows_snapshot.clone();
                            let compare_for_render = compare_for_rows.clone();
                            let select_for_press = select.clone();
                            let toggle_for_press = toggle_compare.clone();
                            let compare_mode_for_press = compare_mode;
                            let menu_select = select.clone();
                            let menu_search = row_search.clone();
                            let menu_commit = commit_for_rows.clone();
                            let menu_squash = squash_range.clone();
                            let menu_refs = refs_for_menu.clone();
                            let menu_recovery = recovery_for_menu.clone();
                            let commits_for_press = rows_snapshot.clone();
                            let commits_for_menu = rows_snapshot.clone();
                            return crate::ui::table::table(crate::ui::table::TableSpec {
                                columns: vec![crate::ui::table::Column {
                                    key: "commit",
                                    title: None,
                                    // 单列自绘整行：宽度要给足，否则图线/摘要/作者/时间
                                    // 会被裁在 120pt 的默认列宽里看不出内容。
                                    width: Some(1200.0),
                                    fixed: false,
                                }],
                                rows,
                                selected,
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
                                    let is_merge = commit.parents.len() > 1;
                                    let has_refs = !commit.refs.is_empty();
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
                                        .child(graph_cell(is_selected || in_compare, is_merge))
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
                                                            this.child(ref_chips(commit.refs.clone()))
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
                                                let timestamp = commit.date.parse().unwrap_or(0);
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
                                    let items = commit_menu(
                                        language_value,
                                        &commit,
                                        {
                                            let select = menu_select.clone();
                                            std::sync::Arc::new(move || select(index))
                                        },
                                        &menu_search,
                                        &menu_recovery,
                                        menu_squash.clone(),
                                        &menu_refs,
                                        &menu_commit,
                                    );
                                    let _ = nsh_core::element::desktop::popup_menu_at(
                                        items,
                                        nsh_core::Point {
                                            x: Points(x),
                                            y: Points(y),
                                        },
                                    );
                                })),
                            })
                            .into_element();
                        })),
                ),
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
        .child(
            view()
                .w_full()
                .h(px(0.0))
                .child(dynamic(move || -> AnyElement {
                    if !filter_open.get() {
                        return view().w_full().h(px(0.0)).into_element();
                    }
                    return filter_pop(
                        language_value,
                        branches.clone(),
                        authors.clone(),
                        filter_state.clone(),
                        filter_cmd.clone(),
                    )
                    .into_element();
                })),
        );
}

fn hist_bar(
    language: Signal<SharedString>,
    state: &HistoryState,
    cmd: &HistoryCommands,
    tab: &Tab,
) -> impl IntoElement + use<> {
    let filter_reference = state.reference;
    let filter_author = state.author;
    let filter_when = state.when;
    let filter_open = state.filter_open;
    let compare_mode = state.compare_mode;
    let branch = tab.branch.clone();
    let total = tab.commits.len();
    // Compare refs 的可选项：当前分支 + 本地分支 + 远程 + 标签。
    let mut refs_list: Vec<String> = Vec::new();
    for branch in &tab.refs.branches {
        refs_list.push(branch.name.clone());
    }
    for remote in &tab.refs.remotes {
        refs_list.push(remote.clone());
    }
    for tag in &tab.refs.tags {
        refs_list.push(tag.clone());
    }
    let compare_pair = (tab.compare_base.clone(), tab.compare_target.clone());
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
    let clear: std::sync::Arc<dyn Fn()> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move || cmd.clear_filter())
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
        .child(chip(
            move || true,
            {
                let branch = branch.clone();
                move || {
                    let reference = filter_reference.get();
                    if reference.is_empty() {
                        branch.clone()
                    } else {
                        reference
                    }
                }
            },
            design::branch_bg,
            design::branch_fg,
        ))
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
        .child(chip(
            move || When::from_key(filter_when.get().as_str()) != When::All,
            move || {
                i18n::t(
                    language.get().as_str(),
                    When::from_key(filter_when.get().as_str()).key(),
                )
            },
            design::panel2,
            design::text2,
        ))
        .child(chip(
            move || !filter_author.get().is_empty(),
            move || filter_author.get(),
            design::panel2,
            design::text2,
        ))
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
        .child(compare_refs(language, refs_list, compare_pair, set_compare_refs))
        .child(
            crate::ui::button::button(
                move || i18n::t(language.get().as_str(), "filter.clear"),
                crate::ui::button::ButtonVariant::Ghost,
                crate::ui::button::ButtonSize::Small,
                false,
                move || clear(),
            ),
        )
        .child(widgets_icon(filter_open));
}

fn widgets_icon(filter_open: Signal<bool>) -> impl IntoElement {
    return crate::ui::button::icon_button(
        "filter",
        move || "Filter".to_string(),
        crate::ui::button::ButtonSize::Small,
        false,
        move || filter_open.set(!filter_open.get_untracked()),
    );
}

/// 工具条上的筛选芯片：`visible` 为假时不占位。
fn chip(
    visible: impl Fn() -> bool + 'static,
    label: impl Fn() -> String + 'static,
    background: fn() -> AdaptiveColor,
    foreground: fn() -> AdaptiveColor,
) -> impl IntoElement {
    return view()
        .h(move || if visible() { px(18.0) } else { px(0.0) })
        .px(px(6.0))
        .items_center()
        .rounded(px(4.0))
        .bg(move || background())
        .child(
            text(move || label())
                .text_size(px(crate::ui::SECONDARY))
                .text_color(move || foreground()),
        );
}

fn column_header(language: Signal<SharedString>, key: &'static str) -> nsh_core::element::Text {
    return text(move || i18n::t(language.get().as_str(), key))
        .text_size(px(crate::ui::SECONDARY))
        .font_weight(FontWeight(600))
        .text_color(move || design::text3());
}

fn graph_cell(highlight: bool, is_merge: bool) -> impl IntoElement {
    return view()
        .w(px(GRAPH_W - 8.0))
        .h_full()
        .flex_row()
        .items_center()
        .gap(px(4.0))
        .child(
            view()
                .w(px(1.0))
                .h_full()
                .bg(move || design::line_strong()),
        )
        .child(
            view()
                .w(px(7.0))
                .h(px(7.0))
                .rounded(px(4.0))
                .bg(move || {
                    if highlight {
                        design::accent()
                    } else {
                        design::line_strong()
                    }
                }),
        )
        .child(
            view()
                .w(px(10.0))
                .h(px(1.0))
                .bg(move || {
                    if is_merge {
                        design::line_strong()
                    } else {
                        AdaptiveColor::TRANSPARENT
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
                crate::shared::os::copy_text(
                    search.session.notice,
                    search.language,
                    sha.clone(),
                )
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

fn filter_pop(
    language: Signal<SharedString>,
    branches: Vec<String>,
    authors: Vec<String>,
    state: HistoryState,
    cmd: HistoryCommands,
) -> impl IntoElement + use<> {
    let filter_reference = state.reference;
    let filter_author = state.author;
    let filter_when = state.when;
    // 视图回调要 `Arc<dyn Fn>`，这里把本功能的命令包一层。
    let apply: std::sync::Arc<dyn Fn()> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move || cmd.apply_filter())
    };
    let clear: std::sync::Arc<dyn Fn()> = {
        let cmd = cmd.clone();
        std::sync::Arc::new(move || cmd.clear_filter())
    };
    let mut branch_rows: Vec<AnyElement> = Vec::new();
    branch_rows.push(choice_row(
        language,
        "filter.allBranches".to_string(),
        move || filter_reference.get().is_empty(),
        {
            let filter_reference = filter_reference;
            move || filter_reference.set(String::new())
        },
    ));
    for branch in branches.iter().take(8) {
        let name = branch.clone();
        let value = branch.clone();
        branch_rows.push(choice_row(
            language,
            name,
            move || filter_reference.get() == value,
            {
                let filter_reference = filter_reference;
                let value = branch.clone();
                move || filter_reference.set(value.clone())
            },
        ));
    }

    let mut author_rows: Vec<AnyElement> = Vec::new();
    author_rows.push(choice_row(
        language,
        i18n::t(language.get_untracked().as_str(), "filter.allAuthors"),
        move || filter_author.get().is_empty(),
        {
            let filter_author = filter_author;
            move || filter_author.set(String::new())
        },
    ));
    for author in authors.iter().take(8) {
        let label = author.clone();
        let value = author.clone();
        author_rows.push(choice_row(
            language,
            label,
            move || filter_author.get() == value,
            {
                let filter_author = filter_author;
                let value = author.clone();
                move || filter_author.set(value.clone())
            },
        ));
    }

    let when_rows: Vec<AnyElement> = When::ALL
        .iter()
        .map(|when| {
            let when = *when;
            choice_row(
                language,
                i18n::t(language.get_untracked().as_str(), when.key()),
                move || When::from_key(filter_when.get().as_str()) == when,
                {
                    let filter_when = filter_when;
                    move || filter_when.set(SharedString::from(when.id()))
                },
            )
        })
        .collect();

    return view()
        .w(px(POP_W))
        .p(px(6.0))
        .flex_col()
        .gap(px(2.0))
        .rounded(px(metrics::RADIUS_PANEL))
        .border_1()
        .border_color(move || design::line())
        .bg(move || design::token("popover"))
        .child(section_label(language, "filter.branch"))
        .child(branch_rows)
        .child(section_label(language, "filter.author"))
        .child(author_rows)
        .child(section_label(language, "filter.when"))
        .child(when_rows)
        .child(
            view()
                .w_full()
                .mt(px(4.0))
                .pt(px(6.0))
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
                    move || i18n::t(language.get().as_str(), "filter.clear"),
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    {
                        let clear = clear.clone();
                        move || clear()
                    },
                ))
                .child(view().flex_1())
                .child(kbd("esc")),
        );
}

fn section_label(language: Signal<SharedString>, key: &'static str) -> impl IntoElement {
    return crate::ui::section::section_label(move || i18n::t(language.get().as_str(), key));
}

fn choice_row(
    language: Signal<SharedString>,
    label: String,
    selected: impl Fn() -> bool + 'static,
    on_press: impl Fn() + 'static,
) -> AnyElement {
    let _ = language;
    let label = label;
    // 选中的 chip 用 Secondary 表达，未选中用 Ghost。
    let variant = if selected() {
        crate::ui::button::ButtonVariant::Secondary
    } else {
        crate::ui::button::ButtonVariant::Ghost
    };
    return crate::ui::button::button(
        move || label.clone(),
        variant,
        crate::ui::button::ButtonSize::Small,
        false,
        on_press,
    )
    .into_element();
}

/// 引用徽章：本地分支、远程、标签各用一组配色。
fn ref_chips(refs: Vec<String>) -> impl IntoElement {
    let mut chips: Vec<AnyElement> = Vec::new();
    for reference in refs.into_iter().take(2) {
        let is_tag = reference.starts_with("tag:");
        let is_remote = reference.contains('/') && !is_tag;
        let label = reference
            .trim_start_matches("HEAD -> ")
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

/// Compare refs 控件（设计板 07）：起点、终点与交换；未选择时提示从哪些引用里挑。
fn compare_refs(
    language: Signal<SharedString>,
    refs: Vec<String>,
    pair: (Option<String>, Option<String>),
    set_compare_refs: std::sync::Arc<dyn Fn(Option<String>, Option<String>)>,
) -> impl IntoElement {
    let (base, target) = pair;
    let base_value = base.clone();
    let target_value = target.clone();
    let has_pair = base.is_some() && target.is_some();
    let swap_base = base.clone();
    let swap_target = target.clone();
    let clear_refs = set_compare_refs.clone();

    // 起点、终点各用一个横向的引用列表选择：点一下循环到下一个（引用数量通常不多）。
    let mut base_picker: Vec<AnyElement> = Vec::new();
    let mut target_picker: Vec<AnyElement> = Vec::new();
    for reference in refs.iter().take(6) {
        let pick_base = set_compare_refs.clone();
        let pick_target = set_compare_refs.clone();
        let base_name = reference.clone();
        let target_name = reference.clone();
        let base_label = reference.clone();
        let target_label = reference.clone();
        let base_target = target_value.clone();
        let target_base = base_value.clone();
        let is_base = base_value.as_deref() == Some(reference.as_str());
        let is_target = target_value.as_deref() == Some(reference.as_str());
        base_picker.push(
            crate::ui::button::button(
                move || base_label.clone(),
                if is_base {
                    crate::ui::button::ButtonVariant::Secondary
                } else {
                    crate::ui::button::ButtonVariant::Ghost
                },
                crate::ui::button::ButtonSize::Small,
                false,
                move || pick_base(Some(base_name.clone()), base_target.clone()),
            )
            .into_element(),
        );
        target_picker.push(
            crate::ui::button::button(
                move || target_label.clone(),
                if is_target {
                    crate::ui::button::ButtonVariant::Secondary
                } else {
                    crate::ui::button::ButtonVariant::Ghost
                },
                crate::ui::button::ButtonSize::Small,
                false,
                move || pick_target(target_base.clone(), Some(target_name.clone())),
            )
            .into_element(),
        );
    }

    return view()
        .flex_row()
        .items_center()
        .gap(px(4.0))
        .child(
            text(move || {
                i18n::t(language.get().as_str(), if has_pair { "compare.range" } else { "compare.pick" })
            })
            .text_size(px(crate::ui::SECONDARY))
            .text_color(move || design::text3()),
        )
        .child(
            view()
                .h(px(18.0))
                .px(px(4.0))
                .flex_row()
                .items_center()
                .gap(px(2.0))
                .rounded(px(4.0))
                .border_1()
                .border_color(move || design::line_soft())
                .child(base_picker),
        )
        .child(
            text("→")
                .text_size(px(crate::ui::SECONDARY))
                .text_color(move || design::text3()),
        )
        .child(
            view()
                .h(px(18.0))
                .px(px(4.0))
                .flex_row()
                .items_center()
                .gap(px(2.0))
                .rounded(px(4.0))
                .border_1()
                .border_color(move || design::line_soft())
                .child(target_picker),
        )
        .child(crate::ui::button::button(
            move || i18n::t(language.get().as_str(), "compare.swap"),
            crate::ui::button::ButtonVariant::Ghost,
            crate::ui::button::ButtonSize::Small,
            false,
            move || set_compare_refs(swap_target.clone(), swap_base.clone()),
        ))
        .child(crate::ui::button::button(
            move || i18n::t(language.get().as_str(), "compare.clear"),
            crate::ui::button::ButtonVariant::Ghost,
            crate::ui::button::ButtonSize::Small,
            false,
            move || clear_refs(None, None),
        ));
}
