//! 工作区外壳：标题栏（仓库标签页）、工具栏与状态栏。
//!
//! 尺寸取自设计稿：`.titlebar.with-tabs` 与 `.toolbar.v2` 使用紧凑 40pt，高度不改变 tabs。
//! 工具栏提供 Fetch/Pull/Push、Stash、Reflog、History/Changes 与 Agent 入口。

use crate::app::session::{RepoProblem, Sync, Tab};
use crate::design::{self, metrics};
use crate::features::search::commands::SearchCommands;
use crate::i18n;
use nsh_core::prelude::*;
use std::sync::Arc;

/// 标题栏：左侧留出系统窗口按钮，其后是仓库标签页与状态。
pub(crate) fn titlebar(
    language: Signal<SharedString>,
    tabs: Signal<Vec<Tab>>,
    active: Signal<usize>,
    session: crate::features::SessionHandle,
    settings: crate::features::settings::commands::SettingsCommands,
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
                .child(
                    view()
                        .w(px(6.0))
                        .h(px(6.0))
                        .rounded(px(3.0))
                        .bg(move || design::ok()),
                )
                .child(
                    text(move || {
                        let list = tabs.get();
                        let index = active.get();
                        match list.get(index) {
                            Some(tab) => {
                                let mut text = tab.branch.clone();
                                if tab.ahead > 0 {
                                    text.push_str(&format!(" ↑{}", tab.ahead));
                                }
                                if tab.behind > 0 {
                                    text.push_str(&format!(" ↓{}", tab.behind));
                                }
                                return text;
                            }
                            None => String::new(),
                        }
                    })
                    .text_size(px(crate::ui::SECONDARY))
                    .text_color(move || design::text3()),
                )
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
/// 状态栏：分支、操作状态、变更数量与编码信息。
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
        .child(crate::ui::icon::icon_view(
            "branch",
            crate::ui::icon::IconSize::Compact,
            design::text3,
        ))
        .child(
            text(move || {
                return tab.get().map(|tab| tab.branch).unwrap_or_default();
            })
            .text_size(px(crate::ui::SECONDARY))
            .line_height(px(crate::ui::SECONDARY_LINE))
            .text_color(move || design::text3()),
        )
        .child(view().flex_1())
        .child(dynamic(move || -> AnyElement {
            // 提示只占右端一小段，出现时不挤动左侧状态。
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
        .child(
            text(move || {
                let count = tab.get().map(|tab| tab.status.len()).unwrap_or(0);
                return i18n::with_arguments(
                    language.get().as_str(),
                    "history.commits",
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
