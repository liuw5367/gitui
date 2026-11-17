//! 工作区外壳：标题栏（仓库标签页）、工具栏与状态栏。
//!
//! 尺寸取自设计稿：`.titlebar.with-tabs` 44、`.toolbar.v2` 44、`.statusbar` 24。
//! P1 中 Fetch/Pull/Push、Branch/Stash、Agent 还没有对应的写路径或同步实现，
//! 这些按钮按禁用态渲染，不伪装成可用。

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
    let (left, right) = crate::window_insets();
    let open = session;
    return view()
        .w_full()
        .h(px(metrics::TITLEBAR_H))
        .flex_row()
        .items_center()
        .gap(px(4.0))
        .pl(px(left + 10.0))
        .pr(px(right + 10.0))
        .bg(move || design::titlebar())
        .border_b_1()
        .border_color(move || design::line())
        .window_drag(true)
        .child(
            view()
                .flex_1()
                .min_w_0()
                .h_full()
                .flex_row()
                .items_center()
                .gap(px(4.0))
                .child(crate::ui::tabs::repository_tabs(
                    tabs,
                    active,
                    open.clone(),
                    language,
                ))
                .child(
                    view()
                        .w(px(27.0))
                        .h(px(metrics::RTAB_H))
                        .items_center()
                        .justify_center()
                        .rounded(px(metrics::RADIUS_CONTROL))
                        .hover(|style| style.bg(move || design::hover()))
                        .label(SharedString::from(i18n::t(
                            language.get_untracked().as_str(),
                            "shell.addTab",
                        )))
                        .on_press({
                            let open = open.clone();
                            move || open.go_repo_manager()
                        })
                        .child(crate::ui::icon::icon_view(
                            "plus",
                            crate::ui::icon::IconSize::Button,
                            design::text2,
                        )),
                ),
        )
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
                )
                .child(crate::ui::button::icon_button(
                    "gear",
                    move || i18n::t(language.get().as_str(), "shell.settings"),
                    crate::ui::button::ButtonSize::Toolbar,
                    false,
                    {
                        let go_settings = settings.clone();
                        move || go_settings.go_settings()
                    },
                )),
        );
}

/// 工具栏：仓库操作、搜索、History/Changes 切换与 Agent 入口。
pub(crate) fn toolbar(
    language: Signal<SharedString>,
    history_query: Signal<String>,
    change_count: impl Fn() -> u32 + 'static,
    mode: Signal<crate::Mode>,
    search: &SearchCommands,
    sync: std::sync::Arc<dyn Fn(Sync) -> ()>,
    panel_open: Signal<bool>,
) -> impl IntoElement {
    // 工具栏的分段与两个入口都改主区模式，因此直接调搜索功能的用例。
    let do_agent = {
        let search = search.clone();
        move || search.set_mode_agent()
    };
    let do_refs = {
        let search = search.clone();
        move || search.set_mode_refs()
    };
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
        .child(crate::ui::button::icon_button(
            "sidebar",
            move || i18n::t(language.get().as_str(), "shell.sidebar"),
            crate::ui::button::ButtonSize::Toolbar,
            true,
            || {},
        ))
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
        .child(tool_button(language, "branch", "shell.branch", {
            let do_refs = do_refs.clone();
            move || do_refs()
        }))
        .child(disabled_tool(language, "archive", "shell.stash"))
        .child(view().flex_1())
        .child(
            view()
                .w(px(metrics::SEARCH_W))
                .child(crate::ui::text_input::search_field(
                    history_query,
                    move || i18n::t(language.get().as_str(), "history.filter"),
                    Some("⌘F"),
                    move |value| history_query.set(value.to_string()),
                )),
        )
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
                    Arc::new(move || mode.get() == crate::Mode::History),
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
        .child(tool_button(language, "sparkle", "shell.agent", move || {
            do_agent()
        }))
        // 设计稿：Agent 右侧的 tb-icon 是右侧面板的显隐开关。
        .child(crate::ui::button::icon_button_tuned(
            "sidebar",
            move || i18n::t(language.get().as_str(), "shell.sidebar"),
            crate::ui::button::ButtonSize::Toolbar,
            Some(crate::ui::icon::IconSize::Toolbar),
            Some(std::rc::Rc::new(move || {
                if panel_open.get() {
                    design::accent()
                } else {
                    design::text2()
                }
            })),
            false,
            move || panel_open.set(!panel_open.get_untracked()),
        ));
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
        .border_b_1()
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

pub(crate) fn sync_banner(tab: Memo<Option<Tab>>) -> impl IntoElement {
    let has_message = move || {
        return tab.get().and_then(|tab| tab.sync_message).is_some();
    };
    return view()
        .w_full()
        .h(move || match tab.get().and_then(|tab| tab.sync_message) {
            Some(_) => px(24.0),
            None => px(0.0),
        })
        .px(px(12.0))
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .bg(move || match tab.get() {
            Some(tab) if tab.sync_failed => design::del_bg(),
            _ => design::panel(),
        })
        .border_b_1()
        .border_color(move || design::line())
        .overflow_hidden()
        .child(dynamic(move || -> AnyElement {
            if !has_message() {
                return view().w_full().h(px(0.0)).into_element();
            }
            return view()
                .w_full()
                .h(px(24.0))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .child(
                    text(move || {
                        tab.get()
                            .and_then(|tab| tab.sync_message)
                            .unwrap_or_default()
                    })
                    .text_size(px(crate::ui::CODE))
                    .line_height(px(crate::ui::CODE_LINE))
                    .text_color(move || match tab.get() {
                        Some(tab) if tab.sync_failed => design::danger(),
                        _ => design::text2(),
                    }),
                )
                .child(view().flex_1())
                .child(
                    text(move || tab.get().map(|tab| tab.branch.clone()).unwrap_or_default())
                        .text_size(px(crate::ui::SECONDARY))
                        .text_color(move || design::text3()),
                )
                .into_element();
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
            return crate::ui::feedback::banner_sized(
                crate::ui::feedback::BannerTone::Accent,
                28.0,
                Some("info"),
                move || {
                    let operation = tab
                        .get()
                        .map(|tab| tab.operation.id().to_string())
                        .unwrap_or_default();
                    let conflicts = tab.get().map(|tab| tab.conflicts.len()).unwrap_or(0);
                    i18n::with_arguments(
                        language.get().as_str(),
                        "operation.banner",
                        &[
                            ("operation", operation),
                            ("conflicts", conflicts.to_string()),
                        ],
                    )
                },
                vec![
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

/// 尚未接入的动作：按钮按设计稿渲染，但处于禁用态（不响应按压）。
fn disabled_tool(
    language: Signal<SharedString>,
    icon: &'static str,
    key: &'static str,
) -> impl IntoElement {
    return crate::ui::button::button_icon(
        move || i18n::t(language.get().as_str(), key),
        icon,
        crate::ui::button::ButtonVariant::Ghost,
        crate::ui::button::ButtonSize::Toolbar,
        true,
        || {},
    );
}
