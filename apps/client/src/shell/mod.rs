//! 工作区外壳：标题栏（仓库标签页）、工具栏与状态栏。
//!
//! 尺寸取自设计稿：`.titlebar.with-tabs` 与 `.toolbar.v2` 使用紧凑 40pt，高度不改变 tabs。
//! 工具栏提供 Fetch/Pull/Push、Stash、Reflog、History/Changes 与 Agent 入口。

use crate::app::session::{RepoProblem, Sync, Tab};
use crate::design::{self, metrics};
use crate::features::search::commands::SearchCommands;
use crate::i18n;
use nsh_core::prelude::*;
use std::collections::BTreeMap;
use std::sync::Arc;

const BRANCH_PICKER_W: f32 = 320.0;
const BRANCH_PICKER_LIST_H: f32 = 288.0;

#[derive(Clone, Debug, PartialEq, Eq)]
struct BranchPickerChoice {
    /// 本地分支名，或带 remote 前缀的远端分支名。
    label: String,
    /// Git 命令使用的本地分支名。
    name: String,
    /// 供 History 导航与远端跟踪表单使用的完整 ref。
    reference: String,
    remote: Option<String>,
    current: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum BranchPickerGroupKind {
    Local,
    Remote(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BranchPickerGroup {
    kind: BranchPickerGroupKind,
    choices: Vec<BranchPickerChoice>,
}

#[derive(Clone, Copy)]
enum BranchPickerActionKind {
    Primary,
    History,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum BranchPickerAction {
    Checkout {
        tab_id: u64,
        branch: String,
    },
    TrackRemote {
        tab_id: u64,
        local_name: String,
        remote_reference: String,
    },
    ShowHistory {
        tab_id: u64,
        reference: String,
    },
}

fn branch_picker_groups(
    branches: &[git::Branch],
    remote_branches: &[git::RemoteBranch],
    current_branch: &str,
) -> Vec<BranchPickerGroup> {
    let mut groups = Vec::new();
    let mut local = branches
        .iter()
        .map(|branch| BranchPickerChoice {
            label: branch.name.clone(),
            name: branch.name.clone(),
            reference: format!("refs/heads/{}", branch.name),
            remote: None,
            current: branch.current || branch.name == current_branch,
        })
        .collect::<Vec<_>>();
    local.sort_by(|left, right| {
        right
            .current
            .cmp(&left.current)
            .then_with(|| left.name.cmp(&right.name))
    });
    if !local.is_empty() {
        groups.push(BranchPickerGroup {
            kind: BranchPickerGroupKind::Local,
            choices: local,
        });
    }

    let mut remotes = BTreeMap::<String, Vec<BranchPickerChoice>>::new();
    for branch in remote_branches {
        let full_name = branch.full_name();
        remotes
            .entry(branch.remote.clone())
            .or_default()
            .push(BranchPickerChoice {
                label: full_name.clone(),
                name: branch.name.clone(),
                reference: format!("refs/remotes/{full_name}"),
                remote: Some(branch.remote.clone()),
                current: false,
            });
    }
    for (remote, mut choices) in remotes {
        choices.sort_by(|left, right| left.name.cmp(&right.name));
        groups.push(BranchPickerGroup {
            kind: BranchPickerGroupKind::Remote(remote),
            choices,
        });
    }
    return groups;
}

fn filter_branch_picker_groups(
    groups: &[BranchPickerGroup],
    query: &str,
) -> Vec<BranchPickerGroup> {
    let query = query.trim().to_lowercase();
    return groups
        .iter()
        .filter_map(|group| {
            let choices = group
                .choices
                .iter()
                .filter(|choice| {
                    if query.is_empty() {
                        return true;
                    }
                    let full_name = choice.label.to_lowercase();
                    let leaf_name = choice.name.rsplit('/').next().unwrap_or(&choice.name);
                    full_name.contains(&query) || leaf_name.to_lowercase().contains(&query)
                })
                .cloned()
                .collect::<Vec<_>>();
            if choices.is_empty() {
                return None;
            }
            return Some(BranchPickerGroup {
                kind: group.kind.clone(),
                choices,
            });
        })
        .collect();
}

fn branch_picker_action(
    tab_id: u64,
    choice: &BranchPickerChoice,
    kind: BranchPickerActionKind,
) -> BranchPickerAction {
    if matches!(kind, BranchPickerActionKind::History) {
        return BranchPickerAction::ShowHistory {
            tab_id,
            reference: choice.reference.clone(),
        };
    }
    return match &choice.remote {
        Some(remote) => BranchPickerAction::TrackRemote {
            tab_id,
            local_name: choice.name.clone(),
            remote_reference: format!("{remote}/{}", choice.name),
        },
        None => BranchPickerAction::Checkout {
            tab_id,
            branch: choice.name.clone(),
        },
    };
}

fn run_branch_picker_action(
    action: BranchPickerAction,
    refs: &crate::features::refs::commands::RefsCommands,
) {
    match action {
        BranchPickerAction::Checkout { tab_id, branch } => refs.checkout_branch_at(tab_id, branch),
        BranchPickerAction::TrackRemote {
            tab_id,
            local_name,
            remote_reference,
        } => refs
            .search
            .open_tracking_branch_form(tab_id, local_name, remote_reference),
        BranchPickerAction::ShowHistory { tab_id, reference } => {
            refs.history.show_reference_at(tab_id, reference)
        }
    }
}

/// 标题栏：左侧留出系统窗口按钮，其后是仓库标签页与状态。
pub(crate) fn titlebar(
    language: Signal<SharedString>,
    tabs: Signal<Vec<Tab>>,
    active: Signal<usize>,
    session: crate::features::SessionHandle,
    settings: crate::features::settings::commands::SettingsCommands,
    refs: crate::features::refs::commands::RefsCommands,
) -> impl IntoElement {
    let auxiliary = session.auxiliary;
    let open = session;
    let repository_area = dynamic(move || -> AnyElement {
        if auxiliary {
            let title = tabs
                .get()
                .get(active.get())
                .map(|tab| tab.name.clone())
                .unwrap_or_default();
            return view()
                .flex_1()
                .min_w_0()
                .h_full()
                .flex_row()
                .items_center()
                .pl(px(8.0))
                .child(
                    text(title)
                        .text_size(px(crate::ui::BODY))
                        .font_weight(FontWeight(600))
                        .text_color(move || design::text()),
                )
                .into_element();
        }
        return view()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex_row()
            .items_center()
            .gap(px(4.0))
            // 这一整块是标题栏的拖动区：页签与按钮自己处理按压，其余位置拖动窗口。
            .window_drag(true)
            .child(crate::ui::tabs::repository_tabs(
                tabs,
                active,
                open.clone(),
                language,
            ))
            // `+` 紧跟在最后一个页签右侧；它右侧的空白是拖动区（见下方 spacer）。
            // 命中区比视觉尺寸大 4pt：标题栏里的 26×24 图标按钮不好点，实测用户按
            // 加号经常落在边缘的空处（那里只剩拖动区，点了没有反应）。
            .child(
                crate::ui::button::icon_button(
                    "plus",
                    move || i18n::t(language.get_untracked().as_str(), "shell.addTab"),
                    crate::ui::button::ButtonSize::Toolbar,
                    false,
                    {
                        let open = open.clone();
                        move || open.go_repo_manager()
                    },
                )
                // 只向右/上/下扩 4pt：向左扩会盖住页签末尾的 ×，反而更难关页签。
                .hit_slop(EdgeInsets {
                    left: px(0.0),
                    ..EdgeInsets::all(px(4.0))
                }),
            )
            // 页签与按钮之外的位置留给拖动窗口：这块空白没有任何处理按压的视图，
            // 显式标成拖动区（不依赖祖先的间接推断）。
            .child(view().flex_1().min_w_0().h_full().window_drag(true))
            .into_element();
    });
    return view()
        .w_full()
        .h(px(metrics::TITLEBAR_H))
        .flex_row()
        .items_center()
        .gap(px(4.0))
        .pl(move || px(crate::window_insets().0 + 10.0))
        .pr(move || px(crate::window_insets().1 + 10.0))
        .bg(move || design::titlebar())
        .border_b_1()
        .border_color(move || design::line())
        .window_drag(true)
        .child(repository_area)
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .flex_shrink_0()
                .child(branch_picker(language, tabs, active, refs))
                .child(
                    view()
                        .w(px(26.0))
                        .h(px(24.0))
                        .items_center()
                        .justify_center()
                        .rounded(px(metrics::RADIUS_CONTROL))
                        .hover(|style| style.bg(move || design::hover()))
                        .on_press({
                            let toggle = settings.clone();
                            move || toggle.toggle_language()
                        })
                        .child(
                            text(move || {
                                if language.get().as_str() == i18n::ZH_HANS_LANG {
                                    "EN".to_string()
                                } else {
                                    "中".to_string()
                                }
                            })
                            .text_size(px(crate::ui::SECONDARY))
                            .text_color(move || design::text2()),
                        ),
                ),
        );
}

fn branch_picker(
    language: Signal<SharedString>,
    tabs: Signal<Vec<Tab>>,
    active: Signal<usize>,
    refs: crate::features::refs::commands::RefsCommands,
) -> impl IntoElement + use<> {
    let open = signal(false);
    let query = signal(String::new());
    let groups = signal(Vec::<BranchPickerGroup>::new());
    let opened_tab_id = signal(None::<u64>);
    let trigger_ref = element_ref();
    let search_ref = element_ref();

    // 弹层绑定到打开时的仓库；活动标签页变化时关闭，避免旧列表操作新仓库。
    effect({
        let open = open;
        let opened_tab_id = opened_tab_id;
        let tabs = tabs;
        let active = active;
        let trigger_ref = trigger_ref;
        let search_ref = search_ref;
        move || {
            let current_tab_id = tabs.with(|list| list.get(active.get()).map(|tab| tab.id));
            if open.get() && opened_tab_id.get() != current_tab_id {
                trigger_ref.focus_back(search_ref);
                open.set(false);
            }
        }
    });

    let toggle = {
        let open = open;
        let query = query;
        let groups = groups;
        let opened_tab_id = opened_tab_id;
        move || {
            if open.get_untracked() {
                open.set(false);
                return;
            }
            let index = active.get_untracked();
            let Some(tab) = tabs.with_untracked(|list| list.get(index).cloned()) else {
                return;
            };
            opened_tab_id.set(Some(tab.id));
            groups.set(branch_picker_groups(
                &tab.refs.branches,
                &tab.refs.remote_branches,
                &tab.branch,
            ));
            query.set(String::new());
            open.set(true);
        }
    };

    return view()
        .flex_shrink_0()
        .child(
            view()
                .element_ref(&trigger_ref)
                .window_drag(false)
                .role(AccessibilityRole::Button)
                .label(move || {
                    let branch = tabs
                        .with(|list| list.get(active.get()).map(|tab| tab.branch.clone()))
                        .filter(|branch| !branch.is_empty());
                    SharedString::from(match branch {
                        Some(branch) => i18n::with_arguments(
                            language.get().as_str(),
                            "branch.selectCurrent",
                            &[("name", branch)],
                        ),
                        None => i18n::t(language.get().as_str(), "branch.select"),
                    })
                })
                .max_w(px(320.0))
                .h(px(28.0))
                .px(px(8.0))
                .flex_row()
                .items_center()
                .gap(px(crate::ui::GAP_SM))
                .rounded(px(metrics::RADIUS_CONTROL))
                .bg(move || {
                    if open.get() {
                        design::hover()
                    } else {
                        AdaptiveColor::TRANSPARENT
                    }
                })
                .hover(|style| style.bg(move || design::hover()))
                .active(|style| style.bg(move || design::line_soft()))
                .focus_visible(move |style| {
                    style
                        .outline(px(crate::ui::FOCUS_WIDTH))
                        .outline_color(move || crate::ui::focus_color())
                })
                .on_press(toggle)
                .child(
                    view()
                        .w(px(6.0))
                        .h(px(6.0))
                        .flex_shrink_0()
                        .rounded(px(3.0))
                        .bg(move || design::ok()),
                )
                .child(
                    text(move || {
                        tabs.with(|list| {
                            list.get(active.get())
                                .map(|tab| tab.branch.clone())
                                .unwrap_or_default()
                        })
                    })
                    .flex_1()
                    .min_w_0()
                    .number_of_lines(1)
                    .text_overflow(nsh_core::EllipsizeMode::Tail)
                    .text_size(px(crate::ui::SECONDARY))
                    .text_color(move || design::text3()),
                )
                .child(
                    text(move || {
                        tabs.with(|list| {
                            list.get(active.get()).map_or_else(String::new, |tab| {
                                let mut tracking = Vec::new();
                                if tab.ahead > 0 {
                                    tracking.push(format!("↑{}", tab.ahead));
                                }
                                if tab.behind > 0 {
                                    tracking.push(format!("↓{}", tab.behind));
                                }
                                tracking.join(" ")
                            })
                        })
                    })
                    .flex_shrink_0()
                    .text_size(px(crate::ui::SECONDARY))
                    .text_color(move || design::text3()),
                )
                .child(crate::ui::icon::icon_view(
                    "chev-down",
                    crate::ui::icon::IconSize::Compact,
                    design::text3,
                )),
        )
        .child(when(
            move || open.get(),
            move || {
                let Some(tab_id) = opened_tab_id.get() else {
                    return view().w(px(0.0)).h(px(0.0)).into_element();
                };
                branch_picker_overlay(
                    language,
                    open,
                    query,
                    groups,
                    search_ref,
                    trigger_ref,
                    tab_id,
                    refs.clone(),
                )
                .into_element()
            },
        ));
}

fn branch_picker_overlay(
    language: Signal<SharedString>,
    open: Signal<bool>,
    query: Signal<String>,
    groups: Signal<Vec<BranchPickerGroup>>,
    search_ref: ElementRef,
    trigger_ref: ElementRef,
    tab_id: u64,
    refs: crate::features::refs::commands::RefsCommands,
) -> impl IntoElement {
    let filtered = computed(move || filter_branch_picker_groups(&groups.get(), &query.get()));
    let outside_open = open;
    let escape_open = open;
    let escape_trigger = trigger_ref;
    let escape_search = search_ref;
    return overlay()
        .anchor(&trigger_ref)
        .placement(Placement::Bottom)
        .align(Align::End)
        .offset(px(4.0))
        .margin(px(8.0))
        .on_press_outside(move |_: &Point<Points>| outside_open.set(false))
        .on_cancel(move || {
            escape_trigger.focus_back(escape_search);
            escape_open.set(false);
        })
        .child(branch_picker_panel(
            language, query, search_ref, filtered, tab_id, open, refs,
        ));
}

fn branch_picker_panel(
    language: Signal<SharedString>,
    query: Signal<String>,
    search_ref: ElementRef,
    filtered: Memo<Vec<BranchPickerGroup>>,
    tab_id: u64,
    open: Signal<bool>,
    refs: crate::features::refs::commands::RefsCommands,
) -> impl IntoElement {
    return view()
        .w(px(BRANCH_PICKER_W))
        .p(px(10.0))
        .flex_col()
        .gap(px(crate::ui::GAP))
        .rounded(px(metrics::RADIUS_PANEL))
        .border_1()
        .border_color(design::line_strong)
        .bg(|| design::token("popover"))
        .shadow_lg()
        .child(crate::ui::text_input::text_input_auto_focus(
            query,
            move || i18n::t(language.get().as_str(), "branch.search"),
            false,
            search_ref,
            move |value| query.set(value.to_string()),
        ))
        .child(scroll().w_full().h(px(BRANCH_PICKER_LIST_H)).child(dynamic(
            move || -> AnyElement {
                let groups = filtered.get();
                if groups.is_empty() {
                    return text(move || i18n::t(language.get().as_str(), "branch.noMatches"))
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(design::text3)
                        .into_element();
                }
                let mut list = view().w_full().flex_col().gap(px(crate::ui::GAP_XS));
                let mut remote_section_shown = false;
                for group in groups {
                    match &group.kind {
                        BranchPickerGroupKind::Local => {
                            list = list.child(branch_picker_section_heading(
                                language,
                                "branch.localBranches",
                            ));
                        }
                        BranchPickerGroupKind::Remote(remote) => {
                            if !remote_section_shown {
                                list = list.child(branch_picker_section_heading(
                                    language,
                                    "branch.remoteBranches",
                                ));
                                remote_section_shown = true;
                            }
                            list = list.child(
                                text(remote.clone())
                                    .pt(px(crate::ui::GAP_XS))
                                    .text_size(px(crate::ui::SECONDARY))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(move || design::text3()),
                            );
                        }
                    }
                    for choice in group.choices {
                        list = list.child(branch_picker_row(
                            language,
                            tab_id,
                            choice,
                            open,
                            refs.clone(),
                        ));
                    }
                }
                return list.into_element();
            },
        )));
}

fn branch_picker_section_heading(
    language: Signal<SharedString>,
    key: &'static str,
) -> impl IntoElement {
    return text(move || i18n::t(language.get().as_str(), key))
        .pt(px(crate::ui::GAP_XS))
        .text_size(px(crate::ui::SECONDARY))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(move || design::text3());
}

fn branch_picker_row(
    language: Signal<SharedString>,
    tab_id: u64,
    choice: BranchPickerChoice,
    open: Signal<bool>,
    refs: crate::features::refs::commands::RefsCommands,
) -> AnyElement {
    let current = choice.current;
    let label = choice.label.clone();
    let primary = branch_picker_action(tab_id, &choice, BranchPickerActionKind::Primary);
    let history = branch_picker_action(tab_id, &choice, BranchPickerActionKind::History);
    let primary_refs = refs.clone();
    let main = view().flex_1().min_w_0().child(crate::ui::row::row_sized(
        move || label.clone(),
        None,
        current.then(|| {
            crate::ui::icon::icon_view("check", crate::ui::icon::IconSize::Compact, design::accent)
                .into_element()
        }),
        if current {
            vec![
                text(move || i18n::t(language.get().as_str(), "branch.current"))
                    .text_size(px(crate::ui::SECONDARY))
                    .text_color(move || design::accent())
                    .into_element(),
            ]
        } else {
            Vec::new()
        },
        current,
        current,
        crate::ui::ROW,
        move || {
            open.set(false);
            run_branch_picker_action(primary.clone(), &primary_refs);
        },
    ));
    let history_name = choice.label.clone();
    let history_refs = refs;
    return view()
        .w_full()
        .flex_row()
        .items_center()
        .gap(px(crate::ui::GAP_XS))
        .child(main)
        .child(crate::ui::button::icon_button(
            "clock",
            move || {
                i18n::with_arguments(
                    language.get().as_str(),
                    "branch.showInHistory",
                    &[("name", history_name.clone())],
                )
            },
            crate::ui::button::ButtonSize::Small,
            false,
            move || {
                open.set(false);
                run_branch_picker_action(history.clone(), &history_refs);
            },
        ))
        .into_element();
}

/// 工具栏：仓库操作、主区切换与 Agent 入口。
pub(crate) fn toolbar(
    language: Signal<SharedString>,
    change_count: impl Fn() -> u32 + 'static,
    mode: Signal<crate::Mode>,
    search: &SearchCommands,
    refs: &crate::features::refs::commands::RefsCommands,
    sync: std::sync::Arc<dyn Fn(Sync) -> ()>,
    sidebar_open: Signal<bool>,
    right_panel_open: Signal<bool>,
) -> impl IntoElement {
    // 工具栏的主区入口分别由搜索与引用功能的用例负责。
    let do_agent = {
        let search = search.clone();
        move || search.set_mode_agent()
    };
    let do_reflog = refs.clone();
    let do_stash = search.clone();
    let set_history = {
        let search = search.clone();
        std::sync::Arc::new(move || search.set_mode(crate::Mode::History))
    };
    let set_changes = {
        let search = search.clone();
        std::sync::Arc::new(move || search.set_mode(crate::Mode::Changes))
    };
    let do_fetch = sync.clone();
    let do_pull = sync.clone();
    let do_push = sync.clone();
    return view()
        .w_full()
        .h(px(metrics::TOOLBAR_H))
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .px(px(10.0))
        .bg(move || design::bar())
        .border_b_1()
        .border_color(move || design::line())
        .child(dynamic(move || -> AnyElement {
            let is_open = sidebar_open.get();
            let label_key = if is_open {
                "shell.hideSidebar"
            } else {
                "shell.showSidebar"
            };
            let color_state = sidebar_open;
            crate::ui::button::icon_button_tuned(
                "sidebar",
                move || i18n::t(language.get().as_str(), label_key),
                crate::ui::button::ButtonSize::Toolbar,
                Some(crate::ui::icon::IconSize::Toolbar),
                Some(std::rc::Rc::new(move || {
                    if color_state.get() {
                        design::accent()
                    } else {
                        design::text2()
                    }
                })),
                false,
                move || sidebar_open.set(!sidebar_open.get_untracked()),
            )
            .into_element()
        }))
        .child(crate::ui::v_separator())
        .child(tool_button(language, "refresh", "shell.fetch", move || {
            do_fetch(Sync::Fetch)
        }))
        .child(tool_button(language, "down", "shell.pull", move || {
            do_pull(Sync::Pull)
        }))
        .child(tool_button(language, "up", "shell.push", move || {
            do_push(Sync::Push)
        }))
        .child(crate::ui::v_separator())
        .child(tool_button(language, "stash", "shell.stash", move || {
            do_stash.open_form(crate::features::search::FormKind::NewStash)
        }))
        .child(tool_button(language, "clock", "shell.reflog", move || {
            do_reflog.open_reflog()
        }))
        .child(view().flex_1())
        .child(crate::ui::v_separator())
        .child(
            view()
                .h(px(26.0))
                .flex_row()
                .items_center()
                .gap(px(2.0))
                .p(px(2.0))
                .rounded(px(7.0))
                .border_1()
                .border_color(move || design::line_soft())
                .bg(move || design::panel2())
                .child(mode_tab(
                    language,
                    "shell.history",
                    Arc::new(move || {
                        matches!(mode.get(), crate::Mode::History | crate::Mode::Blame)
                    }),
                    move || set_history(),
                    None,
                ))
                .child(mode_tab(
                    language,
                    "shell.changes",
                    Arc::new(move || mode.get() == crate::Mode::Changes),
                    move || set_changes(),
                    Some(Arc::new(move || change_count())),
                )),
        )
        .child(crate::ui::button::icon_button_tip(
            "sparkle",
            move || i18n::t(language.get().as_str(), "shell.agent"),
            crate::ui::button::ButtonSize::Toolbar,
            false,
            move || i18n::t(language.get().as_str(), "shell.agent"),
            move || do_agent(),
        ))
        // 设计稿：Agent 右侧的 tb-icon 是右侧面板的显隐开关。
        .child(dynamic(move || -> AnyElement {
            let is_open = right_panel_open.get();
            let label_key = if is_open {
                "shell.hideRightPanel"
            } else {
                "shell.showRightPanel"
            };
            let color_state = right_panel_open;
            crate::ui::button::icon_button_tuned(
                "sidebar",
                move || i18n::t(language.get().as_str(), label_key),
                crate::ui::button::ButtonSize::Toolbar,
                Some(crate::ui::icon::IconSize::Toolbar),
                Some(std::rc::Rc::new(move || {
                    if color_state.get() {
                        design::accent()
                    } else {
                        design::text2()
                    }
                })),
                false,
                move || right_panel_open.set(!right_panel_open.get_untracked()),
            )
            .into_element()
        }));
}

/// 同步结果条：成功用面板底色，失败用危险色；没有结果时高度为 0。
/// 仓库状态横幅（设计板 16）：游离头、空仓库、需要登录、离线。
///
/// 只显示**真实检测到**的状态：`head` 来自 `git symbolic-ref` / `rev-parse`，
/// 网络状态来自同步失败的输出归类；猜不出来就不显示，不制造假状态。
#[allow(clippy::too_many_arguments)]
pub(crate) fn state_banner(
    language: Signal<SharedString>,
    tab: Memo<Option<Tab>>,
    active: Signal<usize>,
    session: crate::features::SessionHandle,
    settings: crate::features::settings::commands::SettingsCommands,
    refs: &crate::features::refs::commands::RefsCommands,
    sync: &crate::features::sync::commands::SyncCommands,
    commit: &crate::features::commit::commands::CommitCommands,
    search: &SearchCommands,
) -> impl IntoElement {
    let visible = move || {
        return tab.get().is_some_and(|tab| {
            if !tab.loaded {
                return false;
            }
            // 打不开的仓库优先级最高：其它状态在它上面都没有意义。
            tab.problem != RepoProblem::None
                || matches!(tab.head, git::HeadState::Detached { .. })
                || matches!(tab.head, git::HeadState::Unborn { .. })
                || tab.sync_fault.is_some()
        });
    };
    let create_branch: std::sync::Arc<dyn Fn(crate::features::search::FormKind, String)> = {
        let search = search.clone();
        std::sync::Arc::new(
            move |kind: crate::features::search::FormKind, from: String| {
                search.open_form_from(kind, from)
            },
        )
    };
    // 「回到 main」「关闭同步失败横幅」已归引用管理与同步功能的用例。
    let checkout_main: std::sync::Arc<dyn Fn()> = {
        let refs = refs.clone();
        std::sync::Arc::new(move || refs.checkout_default_branch())
    };
    let initial_commit: std::sync::Arc<dyn Fn()> = {
        let commit = commit.clone();
        std::sync::Arc::new(move || commit.create_initial_commit())
    };
    let open_settings = settings.clone();
    let dismiss: std::sync::Arc<dyn Fn()> = {
        let sync = sync.clone();
        std::sync::Arc::new(move || sync.clear_sync_fault())
    };

    return view()
        .w_full()
        .h(move || if visible() { px(26.0) } else { px(0.0) })
        .border_b(move || if visible() { px(1.0) } else { px(0.0) })
        .border_color(move || design::line())
        .overflow_hidden()
        .child(dynamic(move || -> AnyElement {
            let Some(current) = tab.get() else {
                return view().w_full().h(px(0.0)).into_element();
            };
            // 语言值克隆进各个闭包（`SharedString` 克隆很便宜），避免借用逃逸。
            let language_now = language.get();
            // 打不开的仓库：给出能直接补救的三个动作，而不是让用户看着报错。
            if current.problem != RepoProblem::None {
                let path = current.path.clone();
                let not_found = current.problem == RepoProblem::NotFound;
                let locate = session.clone();
                let init_here = session.clone();
                let open_parent = session.clone();
                let close = session.clone();
                let language_for_text = language_now.clone();
                // 打不开的仓库：图标 + 说明 + 三个补救动作。「打开上级目录」只在路径
                // 找不到时才有意义，所以单独判一次 not_found。
                let mut banner_actions: Vec<AnyElement> = vec![
                    crate::ui::button::button(
                        {
                            let language_now = language_now.clone();
                            move || {
                                i18n::t(
                                    language_now.as_str(),
                                    if not_found {
                                        "state.locate"
                                    } else {
                                        "state.initHere"
                                    },
                                )
                            }
                        },
                        crate::ui::button::ButtonVariant::Secondary,
                        crate::ui::button::ButtonSize::Small,
                        false,
                        move || {
                            if not_found {
                                locate.locate_repo()
                            } else {
                                init_here.init_here()
                            }
                        },
                    )
                    .into_element(),
                ];
                if !not_found {
                    banner_actions.push(
                        crate::ui::button::button(
                            {
                                let language_now = language_now.clone();
                                move || i18n::t(language_now.as_str(), "state.openParent")
                            },
                            crate::ui::button::ButtonVariant::Secondary,
                            crate::ui::button::ButtonSize::Small,
                            false,
                            move || open_parent.open_parent(),
                        )
                        .into_element(),
                    );
                }
                banner_actions.push(
                    crate::ui::button::button(
                        {
                            let language_now = language_now.clone();
                            move || i18n::t(language_now.as_str(), "common.close")
                        },
                        crate::ui::button::ButtonVariant::Secondary,
                        crate::ui::button::ButtonSize::Small,
                        false,
                        move || close.close_tab(active.get_untracked()),
                    )
                    .into_element(),
                );
                return crate::ui::feedback::banner_sized(
                    crate::ui::feedback::BannerTone::Neutral,
                    26.0,
                    Some("info"),
                    move || {
                        i18n::with_arguments(
                            language_for_text.as_str(),
                            if not_found {
                                "state.repoNotFound"
                            } else {
                                "state.notARepository"
                            },
                            &[("path", path.clone())],
                        )
                    },
                    banner_actions,
                )
                .into_element();
            }
            // 优先级：需要登录 > 离线 > 游离头 > 空仓库。
            if let Some(fault) = current.sync_fault {
                let message_key = match fault {
                    domain::NetworkFault::AuthRequired => "state.authRequired",
                    domain::NetworkFault::Offline => "state.offline",
                    domain::NetworkFault::Rejected => "state.rejected",
                };
                return crate::ui::feedback::banner_sized(
                    crate::ui::feedback::BannerTone::Neutral,
                    26.0,
                    Some("globe"),
                    {
                        let language_now = language_now.clone();
                        move || i18n::t(language_now.as_str(), message_key)
                    },
                    vec![
                        crate::ui::button::button(
                            {
                                let language_now = language_now.clone();
                                move || i18n::t(language_now.as_str(), "state.openCredentials")
                            },
                            crate::ui::button::ButtonVariant::Secondary,
                            crate::ui::button::ButtonSize::Small,
                            false,
                            {
                                let open_settings = open_settings.clone();
                                move || open_settings.go_settings()
                            },
                        )
                        .into_element(),
                        crate::ui::button::button(
                            {
                                let language_now = language_now.clone();
                                move || i18n::t(language_now.as_str(), "state.dismiss")
                            },
                            crate::ui::button::ButtonVariant::Secondary,
                            crate::ui::button::ButtonSize::Small,
                            false,
                            {
                                let dismiss = dismiss.clone();
                                move || dismiss()
                            },
                        )
                        .into_element(),
                    ],
                )
                .into_element();
            }
            match current.head.clone() {
                git::HeadState::Detached { sha } => {
                    let short: String = sha.chars().take(7).collect();
                    return crate::ui::feedback::banner_sized(
                        crate::ui::feedback::BannerTone::Highlight,
                        26.0,
                        Some("branch"),
                        {
                            let language_now = language_now.clone();
                            move || {
                                i18n::with_arguments(
                                    language_now.as_str(),
                                    "state.detached",
                                    &[("short", short.clone())],
                                )
                            }
                        },
                        vec![
                            crate::ui::button::button(
                                {
                                    let language_now = language_now.clone();
                                    move || i18n::t(language_now.as_str(), "state.createBranch")
                                },
                                crate::ui::button::ButtonVariant::Secondary,
                                crate::ui::button::ButtonSize::Small,
                                false,
                                {
                                    let create_branch = create_branch.clone();
                                    let sha = sha.clone();
                                    move || {
                                        create_branch(
                                            crate::features::search::FormKind::NewBranch,
                                            sha.clone(),
                                        )
                                    }
                                },
                            )
                            .into_element(),
                            crate::ui::button::button(
                                {
                                    let language_now = language_now.clone();
                                    move || i18n::t(language_now.as_str(), "state.checkoutMain")
                                },
                                crate::ui::button::ButtonVariant::Secondary,
                                crate::ui::button::ButtonSize::Small,
                                false,
                                {
                                    let checkout_main = checkout_main.clone();
                                    move || checkout_main()
                                },
                            )
                            .into_element(),
                        ],
                    )
                    .into_element();
                }
                git::HeadState::Unborn { branch } => {
                    return crate::ui::feedback::banner_sized(
                        crate::ui::feedback::BannerTone::Neutral,
                        26.0,
                        Some("plus"),
                        {
                            let language_now = language_now.clone();
                            move || {
                                i18n::with_arguments(
                                    language_now.as_str(),
                                    "state.unborn",
                                    &[("branch", branch.clone())],
                                )
                            }
                        },
                        vec![
                            crate::ui::button::button(
                                {
                                    let language_now = language_now.clone();
                                    move || {
                                        i18n::t(language_now.as_str(), "state.createInitialCommit")
                                    }
                                },
                                crate::ui::button::ButtonVariant::Secondary,
                                crate::ui::button::ButtonSize::Small,
                                false,
                                {
                                    let initial_commit = initial_commit.clone();
                                    move || initial_commit()
                                },
                            )
                            .into_element(),
                        ],
                    )
                    .into_element();
                }
                git::HeadState::Branch(_) => {
                    return view().w_full().h(px(0.0)).into_element();
                }
            }
        }));
}

/// 操作条（设计板 14 / 16 的 `.op-banner`）：合并/变基/拣选/回滚进行中时的提示
/// 与放弃入口。没有进行中的操作时高度为 0。
pub(crate) fn op_banner(
    language: Signal<SharedString>,
    tab: Memo<Option<Tab>>,
    merge: &crate::features::merge::commands::MergeCommands,
) -> impl IntoElement {
    let abort: std::sync::Arc<dyn Fn()> = {
        let merge = merge.clone();
        std::sync::Arc::new(move || merge.abort_operation())
    };
    let has_operation = move || {
        return !matches!(
            tab.get().map(|tab| tab.operation),
            Some(git::Operation::None) | None
        );
    };
    let abort_for_view = abort.clone();
    let open_conflicts: std::sync::Arc<dyn Fn()> = {
        let merge = merge.clone();
        std::sync::Arc::new(move || merge.open_conflict())
    };
    let continue_operation: std::sync::Arc<dyn Fn()> = {
        let merge = merge.clone();
        std::sync::Arc::new(move || merge.continue_op())
    };
    let open_conflicts_for_view = open_conflicts.clone();
    let continue_for_view = continue_operation.clone();
    // 无进行中的操作时整条横幅高度为 0；有操作时走 ui 层横幅（28 高、tag-bg、
    // 前置 info 图标），与另外两条横幅共用同一套外观。
    return view()
        .w_full()
        .h(move || if has_operation() { px(28.0) } else { px(0.0) })
        .overflow_hidden()
        .child(dynamic(move || -> AnyElement {
            if !has_operation() {
                return view().w_full().h(px(0.0)).into_element();
            }
            let abort = abort_for_view.clone();
            let conflict_count = tab.get().map(|tab| tab.conflicts.len()).unwrap_or(0);
            let has_pending_drafts = tab.get().is_some_and(|tab| !tab.conflict_drafts.is_empty());
            let action = if conflict_count > 0 {
                let open_conflicts = open_conflicts_for_view.clone();
                crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "conflicts.resolve"),
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || open_conflicts(),
                )
                .into_element()
            } else {
                let continue_operation = continue_for_view.clone();
                crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "operation.continue"),
                    crate::ui::button::ButtonVariant::Primary,
                    crate::ui::button::ButtonSize::Small,
                    has_pending_drafts,
                    move || continue_operation(),
                )
                .into_element()
            };
            return crate::ui::feedback::banner_sized(
                crate::ui::feedback::BannerTone::Accent,
                28.0,
                Some("info"),
                move || {
                    let operation = tab
                        .get()
                        .map(|tab| {
                            crate::features::merge::ops::operation_label(
                                language.get().as_str(),
                                tab.operation,
                            )
                        })
                        .unwrap_or_default();
                    let conflicts = tab.get().map(|tab| tab.conflicts.len()).unwrap_or(0);
                    i18n::with_arguments(
                        language.get().as_str(),
                        "operation.banner",
                        &[("operation", operation), ("files", conflicts.to_string())],
                    )
                },
                vec![
                    action,
                    crate::ui::button::button(
                        move || i18n::t(language.get().as_str(), "operation.abort"),
                        crate::ui::button::ButtonVariant::Secondary,
                        crate::ui::button::ButtonSize::Small,
                        false,
                        move || abort(),
                    )
                    .into_element(),
                ],
            )
            .into_element();
        }));
}
/// 状态栏：临时操作提示、变更数量与编码信息。
pub(crate) fn statusbar(
    language: Signal<SharedString>,
    tab: Memo<Option<Tab>>,
    notice: Signal<Option<String>>,
) -> impl IntoElement {
    return view()
        .w_full()
        .h(px(metrics::STATUS_H))
        .flex_row()
        .items_center()
        .gap(px(9.0))
        .px(px(12.0))
        .bg(move || design::status())
        .border_t_1()
        .border_color(move || design::line())
        .child(dynamic(move || -> AnyElement {
            // 提示只占左端一小段，出现时不挤动右侧编码信息。
            let Some(message) = notice.get() else {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            };
            return view()
                .max_w(px(420.0))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(crate::ui::icon::icon_view(
                    "check",
                    crate::ui::icon::IconSize::Compact,
                    design::ok,
                ))
                .child(
                    text(message)
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(crate::ui::SECONDARY_LINE))
                        .text_color(move || design::text2())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .into_element();
        }))
        .child(view().flex_1())
        .child(
            text(move || {
                let count = tab.get().map(|tab| tab.status.len()).unwrap_or(0);
                return i18n::with_arguments(
                    language.get().as_str(),
                    "changes.changesCount",
                    &[("count", count.to_string())],
                );
            })
            .text_size(px(crate::ui::SECONDARY))
            .line_height(px(crate::ui::SECONDARY_LINE))
            .text_color(move || design::text3()),
        )
        .child(
            text("UTF-8")
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        )
        .child(
            text("LF")
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        );
}

/// 分段控件里的一项：选中态用内容底色与主文本色。
fn mode_tab(
    language: Signal<SharedString>,
    key: &'static str,
    active: Arc<dyn Fn() -> bool>,
    on_press: impl Fn() + 'static,
    count: Option<Arc<dyn Fn() -> u32>>,
) -> impl IntoElement {
    let active_background = active.clone();
    let active_text = active.clone();
    return view()
        .h(px(22.0))
        .px(px(10.0))
        .flex_row()
        .items_center()
        .gap(px(5.0))
        .rounded(px(5.0))
        .bg(move || {
            if active_background() {
                design::content()
            } else {
                AdaptiveColor::TRANSPARENT
            }
        })
        .on_press(on_press)
        .child(
            text(move || i18n::t(language.get().as_str(), key))
                .text_size(px(crate::ui::BODY))
                .text_color(move || {
                    if active_text() {
                        design::text()
                    } else {
                        design::text2()
                    }
                }),
        )
        .when(count.is_some(), |this| {
            let count = count.clone();
            this.child(dynamic(move || -> AnyElement {
                match count.as_ref() {
                    Some(count) => text(count().to_string())
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(move || design::text3())
                        .into_element(),
                    None => view().w(px(0.0)).into_element(),
                }
            }))
        });
}

/// 可用的工具栏按钮（图标 + 文案）。
fn tool_button(
    language: Signal<SharedString>,
    icon: &'static str,
    key: &'static str,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    return crate::ui::button::button_icon(
        move || i18n::t(language.get().as_str(), key),
        icon,
        crate::ui::button::ButtonVariant::Ghost,
        crate::ui::button::ButtonSize::Toolbar,
        false,
        on_press,
    );
}

#[cfg(test)]
mod branch_picker_tests {
    use super::*;

    fn branch(name: &str, current: bool) -> git::Branch {
        git::Branch {
            name: name.into(),
            current,
            ..git::Branch::default()
        }
    }

    fn remote(remote: &str, name: &str) -> git::RemoteBranch {
        git::RemoteBranch {
            remote: remote.into(),
            name: name.into(),
        }
    }

    #[test]
    fn groups_local_first_and_keeps_full_remote_names() {
        let groups = branch_picker_groups(
            &[
                branch("feature/z", false),
                branch("main", false),
                branch("develop", false),
            ],
            &[
                remote("upstream", "main"),
                remote("origin", "feature/ui"),
                remote("origin", "main"),
            ],
            "develop",
        );

        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].kind, BranchPickerGroupKind::Local);
        assert_eq!(groups[0].choices[0].name, "develop");
        assert!(groups[0].choices[0].current);
        assert_eq!(groups[0].choices[1].name, "feature/z");
        assert_eq!(
            groups[1].kind,
            BranchPickerGroupKind::Remote("origin".into())
        );
        assert_eq!(groups[1].choices[0].label, "origin/feature/ui");
        assert_eq!(groups[1].choices[1].label, "origin/main");
        assert_eq!(
            groups[2].kind,
            BranchPickerGroupKind::Remote("upstream".into())
        );
    }

    #[test]
    fn search_matches_case_insensitive_full_name_and_leaf_and_drops_empty_groups() {
        let groups = branch_picker_groups(
            &[branch("feature/search-panel", false), branch("main", true)],
            &[
                remote("origin", "feature/search-panel"),
                remote("upstream", "main"),
            ],
            "main",
        );

        let full = filter_branch_picker_groups(&groups, "ORIGIN/FEATURE/SEARCH-PANEL");
        assert_eq!(full.len(), 1);
        assert_eq!(full[0].kind, BranchPickerGroupKind::Remote("origin".into()));
        assert_eq!(full[0].choices[0].label, "origin/feature/search-panel");

        let leaf = filter_branch_picker_groups(&groups, "PANEL");
        assert_eq!(leaf.len(), 2);
        assert!(leaf.iter().all(|group| group.choices.len() == 1));
        assert!(filter_branch_picker_groups(&groups, "not-found").is_empty());
        assert_eq!(filter_branch_picker_groups(&groups, "").len(), groups.len());
    }

    #[test]
    fn actions_keep_the_opened_tab_and_exact_branch_references() {
        let groups = branch_picker_groups(
            &[branch("main", true)],
            &[remote("origin", "feature/ui")],
            "main",
        );
        let local = &groups[0].choices[0];
        let remote = &groups[1].choices[0];

        assert_eq!(
            branch_picker_action(73, local, BranchPickerActionKind::Primary),
            BranchPickerAction::Checkout {
                tab_id: 73,
                branch: "main".into(),
            }
        );
        assert_eq!(
            branch_picker_action(73, remote, BranchPickerActionKind::Primary),
            BranchPickerAction::TrackRemote {
                tab_id: 73,
                local_name: "feature/ui".into(),
                remote_reference: "origin/feature/ui".into(),
            }
        );
        assert_eq!(
            branch_picker_action(73, remote, BranchPickerActionKind::History),
            BranchPickerAction::ShowHistory {
                tab_id: 73,
                reference: "refs/remotes/origin/feature/ui".into(),
            }
        );
    }
}
