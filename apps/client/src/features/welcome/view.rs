//! 欢迎页（设计板 01 / 02 + 03 的入口）：左侧仓库栏 + 右侧英雄区与
//! Open / Clone / Init 三分段表单。
//!
//! 设计稿把「打开 / 克隆 / 新建」放在独立的仓库管理页（03）；按使用反馈合并进
//! 欢迎页：欢迎页本身就是入口页，打开仓库后进入多标签主窗口。
//!
//! 状态：有最近仓库（01）、没有仓库（02）、克隆中、克隆失败。
//!
//! 界面全部由 `crate::ui` 组合：字号与控件高度取原生刻度，颜色取自 `design`。
//! 设计稿的协议切换（HTTPS / SSH）没有后端含义（协议由 URL 决定），这里不放置
//! 无法生效的控件；差异记在 `docs/DESIGN-HANDOFF.md` 的偏离清单。

use super::commands::WelcomeCommands;
use super::state::{CloneProtocol, WelcomeState};
use crate::design;
use crate::features::SessionHandle;
use crate::features::settings::state::Section;
use crate::i18n;
use crate::ui;
use crate::ui::button::{ButtonSize, ButtonVariant, button, button_hint, icon_button};
use nsh_core::prelude::*;
use store::RecentRepo;

/// 欢迎页左栏宽度（设计板 01 的 `.wside`）。
const SIDE_WIDTH: f32 = 312.0;
/// 右栏内容的最大宽度（设计板 01 的 `.wsection`；窗口收窄后同步收窄）。
const MAIN_MAX_WIDTH: f32 = 520.0;
/// 右栏内边距：上、左右、下。
const MAIN_PAD: (f32, f32, f32) = (44.0, 40.0, 30.0);
/// 英雄区应用标记的边长。
const APP_MARK: f32 = 64.0;

pub(crate) fn welcome(state: &WelcomeState, cmd: &WelcomeCommands) -> impl IntoElement + use<> {
    let language = cmd.language;
    let recent = cmd.session.recent;
    let query = state.query;
    let cloning = state.cloning;
    let clone_progress = state.clone_progress;
    return view()
        .w_full()
        .h_full()
        .flex_row()
        .bg(move || design::content())
        .child(sidebar(language, recent, query, state.now, cmd.clone()))
        .child(
            view()
                .flex_1()
                .min_w_0()
                .h_full()
                .overflow_hidden()
                .child(
                    scroll().w_full().h_full().child(
                        view()
                            .w_full()
                            .pt(px(MAIN_PAD.0))
                            .pb(px(MAIN_PAD.2))
                            .px(px(MAIN_PAD.1))
                            .flex_col()
                            .child(hero(language))
                            .child(entry_section(language, state.clone(), cmd.clone()))
                            .child(tip(language)),
                    ),
                )
                .child(cloning_indicator(
                    language,
                    cloning,
                    clone_progress,
                    cmd.clone(),
                )),
        );
}

/// 克隆进行中的提示：浮在右下角，不遮挡表单。
///
/// 未克隆时整块不渲染——留下 0 高的带边框卡片会画出一条横线（右下角那条多余的
/// 横线就是它）。
fn cloning_indicator(
    language: Signal<SharedString>,
    cloning: Signal<bool>,
    progress: Signal<String>,
    cmd: WelcomeCommands,
) -> impl IntoElement {
    let cancel = cmd.clone();
    return view()
        .absolute()
        .bottom(px(ui::GAP_XL))
        .right(px(ui::GAP_XL))
        .child(dynamic(move || -> AnyElement {
            if !cloning.get() {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            }
            let stage = progress.get();
            let label = if stage.trim().is_empty() {
                i18n::t(language.get().as_str(), "welcome.cloneWorking")
            } else {
                stage
            };
            let cancel = cancel.clone();
            return view()
                .h(px(36.0))
                .flex_row()
                .items_center()
                .gap(px(ui::GAP))
                .px(px(ui::GAP_LG))
                .rounded(px(8.0))
                .border_1()
                .border_color(move || design::line())
                .bg(move || design::panel())
                .child(ui::icon::icon_view(
                    "refresh",
                    ui::icon::IconSize::Inline,
                    design::accent,
                ))
                .child(
                    text(move || label.clone())
                        .text_size(px(ui::SECONDARY))
                        .line_height(px(ui::SECONDARY_LINE))
                        .text_color(move || design::text2())
                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                )
                .child(button(
                    move || i18n::t(language.get().as_str(), "common.cancel"),
                    ButtonVariant::Secondary,
                    ButtonSize::Small,
                    false,
                    move || cancel.cancel_clone(),
                ))
                .into_element();
        }));
}

fn sidebar(
    language: Signal<SharedString>,
    recent: Signal<Vec<RecentRepo>>,
    query: Signal<String>,
    now: Signal<i64>,
    cmd: WelcomeCommands,
) -> impl IntoElement {
    let session = cmd.session.clone();
    let choose_folder = cmd.clone();
    let updates = cmd.clone();
    let forget = cmd.clone();
    let scan = cmd.clone();
    let search_ref = cmd.state.search_ref;
    let count_open = session.tabs;
    return view()
        .w(px(SIDE_WIDTH))
        .h_full()
        .flex_shrink_0()
        .flex_col()
        .bg(move || design::sidebar())
        .border_r_1()
        .border_color(move || design::line())
        // 标题 + 最近/已打开计数 + 打开按钮
        .child(
            view()
                .w_full()
                .pt(px(ui::GAP_LG))
                .pb(px(ui::GAP_SM))
                .px(px(ui::GAP_LG))
                .flex_row()
                .items_center()
                .gap(px(ui::GAP))
                .child(
                    view()
                        .flex_1()
                        .min_w_0()
                        .flex_col()
                        .child(
                            text(move || i18n::t(language.get().as_str(), "welcome.repositories"))
                                .text_size(px(ui::TITLE))
                                .line_height(px(20.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(move || design::text()),
                        )
                        .child(
                            text(move || {
                                i18n::with_arguments(
                                    language.get().as_str(),
                                    "welcome.recentSummary",
                                    &[
                                        ("recent", recent.get().len().to_string()),
                                        ("open", count_open.get().len().to_string()),
                                    ],
                                )
                            })
                            .text_size(px(ui::META))
                            .line_height(px(ui::SECONDARY_LINE))
                            .text_color(move || design::text3()),
                        ),
                )
                .child(button(
                    move || i18n::t(language.get().as_str(), "welcome.openFolder"),
                    ButtonVariant::Ghost,
                    ButtonSize::Small,
                    false,
                    {
                        let choose_folder = choose_folder.clone();
                        move || choose_folder.choose_folder()
                    },
                )),
        )
        // 搜索
        .child(view().w_full().px(px(ui::GAP_LG)).pb(px(ui::GAP_SM)).child(
            ui::text_input::search_field_ref(
                query,
                move || i18n::t(language.get().as_str(), "welcome.search"),
                Some("⌘F"),
                move |text| query.set(text.to_string()),
                &search_ref,
            ),
        ))
        // 最近仓库：分组、搜索过滤与空态都在同一个 dynamic 里，输入即更新。
        .child(
            view().w_full().flex_1().min_h_0().px(px(ui::GAP_SM)).child(
                scroll().w_full().h_full().child(
                    view().w_full().py(px(ui::GAP_XS)).flex_col().child(dynamic(
                        move || -> AnyElement {
                            let all = recent.get();
                            let now_value = now.get();
                            let query_value = query.get();
                            if all.is_empty() {
                                let scan = scan.clone();
                                return view()
                                    .w_full()
                                    .px(px(ui::GAP_SM))
                                    .pt(px(24.0))
                                    .child(ui::section::empty_card(
                                        "folder-plus",
                                        move || {
                                            i18n::t(language.get().as_str(), "welcome.noRecent")
                                        },
                                        Some(Box::new(move || {
                                            i18n::t(language.get().as_str(), "welcome.noRecentHint")
                                        })),
                                        Some(
                                            button(
                                                move || {
                                                    i18n::t(
                                                        language.get().as_str(),
                                                        "welcome.scan",
                                                    )
                                                },
                                                ButtonVariant::Secondary,
                                                ButtonSize::Small,
                                                false,
                                                move || scan.scan_repositories(),
                                            )
                                            .into_element(),
                                        ),
                                    ))
                                    .into_element();
                            }
                            let visible: Vec<RecentRepo> = all
                                .into_iter()
                                .filter(|repo| matches_query(&repo.path, &query_value))
                                .collect();
                            if visible.is_empty() {
                                return ui::section::empty(
                                    move || {
                                        i18n::t(language.get().as_str(), "welcome.noMatch")
                                    },
                                    Some(Box::new(move || {
                                        i18n::t(language.get().as_str(), "welcome.noMatchHint")
                                    })),
                                    None,
                                )
                                .into_element();
                            }
                            // 已打开的仓库同时读取标签页，标签变化后选中态跟着更新。
                            let open_paths: Vec<String> = count_open
                                .get()
                                .iter()
                                .map(|tab| tab.path.clone())
                                .collect();
                            // 画板只有 Today / Previous 7 days；超过 7 天的记录
                            // 单独放进 Earlier，否则会从 12 条上限里消失。
                            let mut list = view().w_full().flex_col();
                            for (label_key, bucket) in [
                                ("welcome.groupToday", RecentBucket::Today),
                                ("welcome.groupWeek", RecentBucket::Week),
                                ("welcome.groupEarlier", RecentBucket::Earlier),
                            ] {
                                let items: Vec<RecentRepo> = visible
                                    .iter()
                                    .filter(|repo| bucket_of(repo.opened_at, now_value) == bucket)
                                    .cloned()
                                    .collect();
                                if items.is_empty() {
                                    continue;
                                }
                                list = list.child(ui::section::section_label_upper(move || {
                                    i18n::t(language.get().as_str(), label_key)
                                }));
                                for repo in items {
                                    let selected = open_paths.iter().any(|path| path == &repo.path);
                                    list = list.child(repo_row(
                                        language,
                                        repo,
                                        now_value,
                                        selected,
                                        session.clone(),
                                        forget.clone(),
                                    ));
                                }
                            }
                            return list.into_element();
                        },
                    )),
                ),
            ),
        )
        // 底部：版本与检查更新
        .child(
            view()
                .w_full()
                .h(px(36.0))
                .flex_row()
                .items_center()
                .gap(px(ui::GAP))
                .px(px(ui::GAP_LG))
                .border_t_1()
                .border_color(move || design::line())
                .child(
                    text(crate::brand::version_line())
                        .text_size(px(ui::META))
                        .line_height(px(ui::SECONDARY_LINE))
                        .text_color(move || design::text3()),
                )
                .child(view().flex_1())
                .child(button(
                    move || i18n::t(language.get().as_str(), "welcome.checkUpdates"),
                    ButtonVariant::Ghost,
                    ButtonSize::Small,
                    false,
                    {
                        let updates = updates.clone();
                        move || updates.check_updates()
                    },
                )),
        );
}

fn repo_row(
    language: Signal<SharedString>,
    repo: RecentRepo,
    now: i64,
    selected: bool,
    session: SessionHandle,
    cmd: WelcomeCommands,
) -> impl IntoElement {
    let path = repo.path.clone();
    let name = last_segment(&path);
    let subtitle = home_path(&path);
    let branch = repo.branch.clone();
    let opened_at = repo.opened_at;
    let path_for_press = path.clone();
    let path_for_remove = path.clone();
    let name_for_remove = name.clone();
    let language_for_time = language;
    let language_for_remove = language;
    let open = session;
    let remove = cmd;
    let leading = view()
        .w(px(27.0))
        .h(px(27.0))
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .rounded(px(7.0))
        .border_1()
        .border_color(move || design::line())
        .bg(move || {
            if selected {
                design::content()
            } else {
                design::panel()
            }
        })
        .child(ui::icon::icon_view(
            "folder",
            ui::icon::IconSize::Inline,
            move || {
                if selected {
                    design::accent()
                } else {
                    design::text2()
                }
            },
        ))
        .into_element();
    let mut aside: Vec<AnyElement> = Vec::new();
    if !branch.is_empty() {
        aside.push(ui::badge::branch_chip(move || branch.clone()).into_element());
    }
    let mut trailing: Vec<AnyElement> = Vec::new();
    trailing.push(
        ui::meta(move || {
            crate::dates::format(language_for_time.get().as_str(), opened_at, now)
        })
        .into_element(),
    );
    trailing.push(
        icon_button(
            "x",
            move || {
                i18n::with_arguments(
                    language_for_remove.get().as_str(),
                    "welcome.removeRepo",
                    &[("name", name_for_remove.clone())],
                )
            },
            ButtonSize::Small,
            false,
            move || remove.forget_recent(path_for_remove.clone()),
        )
        .into_element(),
    );
    // 画板的行高 47、删除按钮 19、分支 chip 10pt；这里用原生 47 / 22 / 11。
    // 第一行：名称 + 分支 chip + 时间 + ×；第二行：路径。
    return ui::row::row_stacked(
        move || name.clone(),
        Some((Box::new(move || subtitle.clone()), true)),
        Some(leading),
        aside,
        trailing,
        selected,
        false,
        47.0,
        move || open.open_path(path_for_press.clone()),
    );
}

fn hero(language: Signal<SharedString>) -> impl IntoElement {
    return view()
        .w_full()
        .flex_row()
        .items_center()
        .gap(px(ui::GAP_XL))
        .child(
            view()
                .w(px(APP_MARK))
                .h(px(APP_MARK))
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .rounded(px(15.0))
                .bg(move || design::accent2())
                .child(ui::icon::brand_mark("branch", 30.0, || {
                    design::token("primary-foreground")
                })),
        )
        .child(
            view()
                .min_w_0()
                .flex_col()
                .child(
                    text(move || i18n::t(language.get().as_str(), "app.name"))
                        .text_size(px(ui::TITLE_LARGE))
                        .line_height(px(32.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(move || design::text()),
                )
                .child(
                    text(move || i18n::t(language.get().as_str(), "app.tagline"))
                        .mt(px(ui::GAP_XS))
                        .text_size(px(ui::BODY))
                        .line_height(px(ui::BODY_LINE))
                        .text_color(move || design::text2()),
                ),
        );
}

/// Open / Clone / Init 三分段与对应表单（合并设计板 03 的入口）。
fn entry_section(
    language: Signal<SharedString>,
    state: WelcomeState,
    cmd: WelcomeCommands,
) -> impl IntoElement {
    let form = state.form;
    let set_form = cmd.clone();
    return view()
        .w_full()
        .max_w(px(MAIN_MAX_WIDTH))
        .mt(px(24.0))
        .flex_col()
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .gap(px(ui::GAP_LG))
                .child(dynamic(move || {
                    let selected = match form.get() {
                        super::state::RepoForm::Open => "open",
                        super::state::RepoForm::Clone => "clone",
                        super::state::RepoForm::Init => "init",
                    };
                    let set_form = set_form.clone();
                    return ui::segmented::segmented(
                        vec![
                            ui::segmented::Segment {
                                id: "open",
                                label: Box::new(move || {
                                    i18n::t(language.get().as_str(), "repoManager.tabOpen")
                                }),
                            },
                            ui::segmented::Segment {
                                id: "clone",
                                label: Box::new(move || {
                                    i18n::t(language.get().as_str(), "repoManager.tabClone")
                                }),
                            },
                            ui::segmented::Segment {
                                id: "init",
                                label: Box::new(move || {
                                    i18n::t(language.get().as_str(), "repoManager.tabInit")
                                }),
                            },
                        ],
                        selected,
                        move |id| {
                            set_form.set_repo_form(match id {
                                "clone" => super::state::RepoForm::Clone,
                                "init" => super::state::RepoForm::Init,
                                _ => super::state::RepoForm::Open,
                            })
                        },
                    )
                    .into_element();
                })),
        )
        .child(
            view()
                .w_full()
                .mt(px(14.0))
                .flex_col()
                .gap(px(ui::GAP_SM))
                .child(
                    view()
                        .w_full()
                        .flex_row()
                        .items_start()
                        .gap(px(ui::GAP))
                        .child(
                            view()
                                .flex_1()
                                .min_w_0()
                                .flex_col()
                                .child(
                                    text(move || {
                                        i18n::t(
                                            language.get().as_str(),
                                            match form.get() {
                                                super::state::RepoForm::Open => {
                                                    "repoManager.openTitle"
                                                }
                                                super::state::RepoForm::Clone => {
                                                    "welcome.cloneTitle"
                                                }
                                                super::state::RepoForm::Init => {
                                                    "repoManager.initTitle"
                                                }
                                            },
                                        )
                                    })
                                    .text_size(px(ui::BODY))
                                    .line_height(px(ui::BODY_LINE))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(move || design::text()),
                                )
                                .child(
                                    text(move || {
                                        i18n::t(
                                            language.get().as_str(),
                                            match form.get() {
                                                super::state::RepoForm::Open => {
                                                    "repoManager.openHint"
                                                }
                                                super::state::RepoForm::Clone => {
                                                    "welcome.cloneSub"
                                                }
                                                super::state::RepoForm::Init => {
                                                    "repoManager.initHint"
                                                }
                                            },
                                        )
                                    })
                                    .text_size(px(ui::SECONDARY))
                                    .line_height(px(ui::SECONDARY_LINE))
                                    .text_color(move || design::text3()),
                                ),
                        )
                        .child(dynamic(move || -> AnyElement {
                            if form.get() != super::state::RepoForm::Clone {
                                return view().w(px(0.0)).into_element();
                            }
                            let options = state.clone_options;
                            let selected = match options.get().protocol {
                                CloneProtocol::Https => "https",
                                CloneProtocol::Ssh => "ssh",
                            };
                            return ui::segmented::segmented(
                                vec![
                                    ui::segmented::Segment {
                                        id: "https",
                                        label: Box::new(move || {
                                            i18n::t(
                                                language.get().as_str(),
                                                "welcome.protocolHttps",
                                            )
                                        }),
                                    },
                                    ui::segmented::Segment {
                                        id: "ssh",
                                        label: Box::new(move || {
                                            i18n::t(
                                                language.get().as_str(),
                                                "welcome.protocolSsh",
                                            )
                                        }),
                                    },
                                ],
                                selected,
                                move |id| {
                                    let mut next = options.get_untracked();
                                    next.protocol = if id == "ssh" {
                                        CloneProtocol::Ssh
                                    } else {
                                        CloneProtocol::Https
                                    };
                                    options.set(next);
                                },
                            )
                            .into_element();
                        })),
                )
                .child(dynamic(move || -> AnyElement {
                    match form.get() {
                        super::state::RepoForm::Open => {
                            open_form(language, state.clone(), cmd.clone()).into_element()
                        }
                        super::state::RepoForm::Clone => {
                            clone_form(language, state.clone(), cmd.clone()).into_element()
                        }
                        super::state::RepoForm::Init => {
                            init_form(language, state.clone(), cmd.clone()).into_element()
                        }
                    }
                })),
        );
}

fn open_form(
    language: Signal<SharedString>,
    state: WelcomeState,
    cmd: WelcomeCommands,
) -> impl IntoElement {
    let path = state.open_path;
    let error = state.open_error;
    let recent = cmd.session.recent;
    let open = cmd;
    return view()
        .w_full()
        .pt(px(ui::GAP_SM))
        .flex_col()
        .gap(px(ui::GAP_LG))
        // 字段与主操作同行：有值校验后打开，无值先选目录再校验打开（两者都是
        // 同一个按钮，不再另起一行主按钮）。
        .child(
            view()
                .w_full()
                .flex_row()
                .items_end()
                .gap(px(ui::GAP))
                .child(view().flex_1().min_w_0().child(ui::field::field(
                    move || i18n::t(language.get().as_str(), "welcome.repositoryPath"),
                    ui::text_input::path_input(
                        path,
                        move || "~/Developer/gitui".to_string(),
                        Some("⌘V"),
                        move |text| path.set(text.to_string()),
                    ),
                    None,
                    None,
                )))
                .child(button_hint(
                    move || i18n::t(language.get().as_str(), "welcome.openAction"),
                    || "⌘O".to_string(),
                    ButtonVariant::Primary,
                    ButtonSize::Regular,
                    false,
                    move || open.open_or_choose(),
                )),
        )
        // 列表为空时给下一步；有最近仓库时这一行不占位置。
        .child(
            view().w_full().flex_row().items_center().child(ui::meta(move || {
                if recent.get().is_empty() {
                    i18n::t(language.get().as_str(), "welcome.openMeta")
                } else {
                    String::new()
                }
            })),
        )
        .child(form_error(error));
}

fn form_error(error: Signal<Option<String>>) -> impl IntoElement {
    return dynamic(move || -> AnyElement {
        match error.get() {
            Some(message) => {
                return view()
                    .w_full()
                    .child(
                        text(move || message.clone())
                            .text_size(px(ui::SECONDARY))
                            .line_height(px(ui::SECONDARY_LINE))
                            .text_color(move || design::danger()),
                    )
                    .into_element();
            }
            None => return view().w_full().h(px(0.0)).into_element(),
        }
    });
}

fn clone_form(
    language: Signal<SharedString>,
    state: WelcomeState,
    cmd: WelcomeCommands,
) -> impl IntoElement {
    let choose_destination = cmd.clone();
    let clone_repository = cmd.clone();
    let credentials = cmd.clone();
    let set_url = state.clone_url;
    let set_destination = state.clone_destination;
    let options = state.clone_options;
    let error = state.clone_error;
    let cloning = state.cloning;
    let url_for_hint = set_url;
    let url_for_account = set_url;
    return view()
        .w_full()
        .pt(px(ui::GAP_SM))
        .flex_col()
        .gap(px(ui::GAP_LG))
        .child(ui::field::field(
            move || i18n::t(language.get().as_str(), "welcome.url"),
            ui::text_input::url_input(
                set_url,
                move || match options.get().protocol {
                    CloneProtocol::Https => "https://github.com/owner/repo.git".to_string(),
                    CloneProtocol::Ssh => "git@github.com:owner/repo.git".to_string(),
                },
                move |text| set_url.set(text.to_string()),
            ),
            None,
            None,
        ))
        .child(
            view()
                .w_full()
                .flex_row()
                .items_end()
                .gap(px(ui::GAP))
                .child(view().flex_1().min_w_0().child(ui::field::field(
                    move || i18n::t(language.get().as_str(), "welcome.destination"),
                    ui::text_input::path_input(
                        set_destination,
                        move || "~/Developer/repo".to_string(),
                        Some("⌘V"),
                        move |text| set_destination.set(text.to_string()),
                    ),
                    None,
                    None,
                )))
                .child(button(
                    move || i18n::t(language.get().as_str(), "welcome.choose"),
                    ButtonVariant::Secondary,
                    ButtonSize::Regular,
                    false,
                    {
                        let choose_destination = choose_destination.clone();
                        move || choose_destination.choose_clone_destination()
                    },
                )),
        )
        .child(
            view()
                .w_full()
                .flex_row()
                .items_end()
                .gap(px(ui::GAP))
                .child(view().flex_1().min_w_0().child(ui::field::field(
                    move || i18n::t(language.get().as_str(), "welcome.branch"),
                    ui::text_input::text_input_bound(
                        {
                            let options = options;
                            move || options.get().branch
                        },
                        move || i18n::t(language.get().as_str(), "welcome.branchHint"),
                        false,
                        move |text| {
                            let mut next = options.get_untracked();
                            next.branch = text;
                            options.set(next);
                        },
                    ),
                    None,
                    None,
                )))
                .child(
                    view()
                        // 画板是 150。三个分段按内容取宽（约 210），写死宽度会把
                        // 最右侧的 Shallow 50 裁掉；branch 只占剩余宽度。
                        .flex_shrink_0()
                        .flex_col()
                        .gap(px(ui::GAP_XS))
                        .child(ui::field::label(move || {
                            i18n::t(language.get().as_str(), "welcome.depth")
                        }))
                        .child(dynamic(move || {
                            let selected = match options.get().depth {
                                1 => "one",
                                50 => "fifty",
                                _ => "full",
                            };
                            return ui::segmented::segmented(
                                vec![
                                    ui::segmented::Segment {
                                        id: "full",
                                        label: Box::new(move || {
                                            i18n::t(language.get().as_str(), "welcome.depthFull")
                                        }),
                                    },
                                    ui::segmented::Segment {
                                        id: "one",
                                        label: Box::new(move || {
                                            i18n::t(language.get().as_str(), "welcome.depthOne")
                                        }),
                                    },
                                    ui::segmented::Segment {
                                        id: "fifty",
                                        label: Box::new(move || {
                                            i18n::t(language.get().as_str(), "welcome.depthFifty")
                                        }),
                                    },
                                ],
                                selected,
                                move |id| {
                                    let mut next = options.get_untracked();
                                    next.depth = match id {
                                        "one" => 1,
                                        "fifty" => 50,
                                        _ => 0,
                                    };
                                    options.set(next);
                                },
                            )
                            .into_element();
                        })),
                ),
        )
        .child(dynamic(move || {
            let current = options.get();
            return view()
                .w_full()
                .flex_col()
                .gap(px(4.0))
                .child(ui::check::check_row_button(
                    move || i18n::t(language.get().as_str(), "welcome.recurse"),
                    current.recurse_submodules,
                    move |next| {
                        let mut options_next = options.get_untracked();
                        options_next.recurse_submodules = next;
                        options.set(options_next);
                    },
                ))
                .child(ui::check::check_row_button(
                    move || i18n::t(language.get().as_str(), "welcome.openAfterClone"),
                    current.open_after_clone,
                    move |next| {
                        let mut options_next = options.get_untracked();
                        options_next.open_after_clone = next;
                        options.set(options_next);
                    },
                ))
                .into_element();
        }))
        // 账号行：凭据流程尚未接入，显示未登录状态；按钮只做设置页导航。
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .gap(px(ui::GAP))
                .px(px(ui::GAP))
                .py(px(6.0))
                .rounded(px(7.0))
                .border_1()
                .border_color(move || design::line())
                .bg(move || design::panel())
                .child(
                    view()
                        .w(px(8.0))
                        .h(px(8.0))
                        .rounded(px(4.0))
                        .bg(move || design::text3()),
                )
                .child(
                    text(move || {
                        let language = language.get();
                        let status = i18n::t(language.as_str(), "welcome.notSignedIn");
                        return match remote_host(url_for_account.get().as_str()) {
                            Some(host) => format!("{host} · {status}"),
                            None => status,
                        };
                    })
                    .text_size(px(ui::SECONDARY))
                    .line_height(px(ui::SECONDARY_LINE))
                    .text_color(move || design::text2()),
                )
                .child(view().flex_1())
                .child(button(
                    move || i18n::t(language.get().as_str(), "welcome.credentials"),
                    ButtonVariant::Secondary,
                    ButtonSize::Small,
                    false,
                    {
                        let credentials = credentials.clone();
                        move || credentials.open_settings(Section::Credentials)
                    },
                )),
        )
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .gap(px(ui::GAP))
                .child(dynamic(move || {
                    let busy = cloning.get();
                    let clone_repository = clone_repository.clone();
                    let label_key = if busy {
                        "welcome.cloneWorking"
                    } else {
                        "welcome.clone"
                    };
                    return button(
                        move || i18n::t(language.get().as_str(), label_key),
                        ButtonVariant::Primary,
                        ButtonSize::Regular,
                        busy,
                        move || clone_repository.clone_repository(),
                    )
                    .into_element();
                }))
                .child(ui::secondary(move || {
                    let language = language.get();
                    let hint = i18n::t(language.as_str(), "welcome.cloneHint");
                    if url_for_hint.get().trim_start().starts_with("http://") {
                        format!("{hint}  {}", i18n::t(language.as_str(), "welcome.cloneHttpHint"))
                    } else {
                        hint
                    }
                })),
        )
        .child(form_error(error));
}

fn init_form(
    language: Signal<SharedString>,
    state: WelcomeState,
    cmd: WelcomeCommands,
) -> impl IntoElement {
    let create = cmd.clone();
    let choose = cmd.clone();
    let path = state.init_path;
    let name = state.init_name;
    let branch = state.init_branch;
    let files = state.init_files;
    let error = state.init_error;
    return view()
        .w_full()
        .pt(px(ui::GAP_SM))
        .flex_col()
        .gap(px(ui::GAP_LG))
        .child(
            view()
                .w_full()
                .flex_row()
                .items_end()
                .gap(px(ui::GAP))
                .child(view().flex_1().min_w_0().child(ui::field::field(
                    move || i18n::t(language.get().as_str(), "repoManager.initPath"),
                    ui::text_input::path_input(
                        path,
                        move || "~/Developer/new-project".to_string(),
                        Some("⌘V"),
                        move |text| path.set(text.to_string()),
                    ),
                    None,
                    None,
                )))
                .child(button(
                    move || i18n::t(language.get().as_str(), "welcome.choose"),
                    ButtonVariant::Secondary,
                    ButtonSize::Regular,
                    false,
                    {
                        let choose = choose.clone();
                        move || choose.choose_init_folder()
                    },
                )),
        )
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .gap(px(ui::GAP))
                .child(view().flex_1().min_w_0().child(ui::field::field(
                    move || i18n::t(language.get().as_str(), "welcome.repoName"),
                    ui::text_input::text_input(
                        name,
                        move || "new-project".to_string(),
                        false,
                        move |text| name.set(text.to_string()),
                    ),
                    None,
                    None,
                )))
                .child(view().w(px(160.0)).child(ui::field::field(
                    move || i18n::t(language.get().as_str(), "welcome.initialBranch"),
                    ui::text_input::text_input(
                        branch,
                        move || "main".to_string(),
                        false,
                        move |text| branch.set(text.to_string()),
                    ),
                    None,
                    None,
                ))),
        )
        .child(dynamic(move || {
            let current = files.get();
            return view()
                .w_full()
                .flex_col()
                .gap(px(4.0))
                .child(ui::check::check_row_button(
                    move || i18n::t(language.get().as_str(), "welcome.readme"),
                    current.readme,
                    move |next| {
                        let mut updated = files.get_untracked();
                        updated.readme = next;
                        files.set(updated);
                    },
                ))
                .child(ui::check::check_row_button(
                    move || i18n::t(language.get().as_str(), "welcome.gitignore"),
                    current.gitignore,
                    move |next| {
                        let mut updated = files.get_untracked();
                        updated.gitignore = next;
                        files.set(updated);
                    },
                ))
                .child(ui::check::check_row_button(
                    move || i18n::t(language.get().as_str(), "welcome.license"),
                    current.license,
                    move |next| {
                        let mut updated = files.get_untracked();
                        updated.license = next;
                        files.set(updated);
                    },
                ))
                .into_element();
        }))
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .gap(px(ui::GAP))
                .child(button(
                    move || i18n::t(language.get().as_str(), "repoManager.initAction"),
                    ButtonVariant::Primary,
                    ButtonSize::Regular,
                    false,
                    {
                        let create = create.clone();
                        move || create.init_repository()
                    },
                ))
                .child(ui::secondary(move || {
                    i18n::t(language.get().as_str(), "welcome.initHint")
                })),
        )
        .child(form_error(error));
}

fn tip(language: Signal<SharedString>) -> impl IntoElement {
    return view()
        .w_full()
        .max_w(px(MAIN_MAX_WIDTH))
        .mt(px(26.0))
        .child(ui::feedback::card(
            view()
                .w_full()
                .flex_row()
                .items_start()
                .gap(px(ui::GAP))
                .p(px(ui::GAP_LG))
                .child(ui::icon::icon_view(
                    "info",
                    ui::icon::IconSize::Inline,
                    design::accent,
                ))
                .child(
                    view()
                        .flex_1()
                        .min_w_0()
                        .flex_row()
                        .items_start()
                        .gap(px(ui::GAP_XS))
                        .child(
                            text(move || {
                                i18n::t(language.get().as_str(), "welcome.tipLead")
                            })
                            .text_size(px(ui::SECONDARY))
                            .line_height(px(ui::SECONDARY_LINE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(move || design::text2()),
                        )
                        .child(
                            text(move || {
                                i18n::t(language.get().as_str(), "welcome.tipRest")
                            })
                            .text_size(px(ui::SECONDARY))
                            .line_height(px(ui::SECONDARY_LINE))
                            .text_color(move || design::text2()),
                        ),
                ),
        ));
}

/// 最近仓库在左栏的三种时间分组。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RecentBucket {
    Today,
    Week,
    Earlier,
}

/// 按 UTC 日切分，避免用 24 小时窗口把“昨天 23 点”算成今天。
fn bucket_of(opened_at: i64, now: i64) -> RecentBucket {
    let today = now.div_euclid(86_400);
    let opened = opened_at.div_euclid(86_400);
    return match (today - opened).max(0) {
        0 => RecentBucket::Today,
        days if days <= 7 => RecentBucket::Week,
        _ => RecentBucket::Earlier,
    };
}

/// 从克隆 URL 里取主机名；只用于账号行的短提示，不解析完整 URL。
fn remote_host(url: &str) -> Option<String> {
    let url = url.trim();
    if let Some(rest) = url.strip_prefix("http://").or_else(|| url.strip_prefix("https://")) {
        return rest
            .split('/')
            .next()
            .filter(|host| !host.is_empty())
            .map(|host| host.to_string());
    }
    // SSH 形式：git@host:owner/repo.git
    let rest = url.strip_prefix("git@")?;
    let host = rest.split(':').next().unwrap_or_default();
    if host.is_empty() {
        return None;
    }
    return Some(host.to_string());
}

fn matches_query(path: &str, query: &str) -> bool {
    if query.trim().is_empty() {
        return true;
    }
    return path.to_lowercase().contains(&query.to_lowercase());
}

fn last_segment(path: &str) -> String {
    return path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_string();
}

/// 设计稿把完整路径显示为 `~/Developer/gitui`。
fn home_path(path: &str) -> String {
    if let Some(home) = std::env::var_os("HOME") {
        let home = home.to_string_lossy().to_string();
        if let Some(rest) = path.strip_prefix(home.as_str()) {
            return format!("~{}", rest);
        }
    }
    return path.to_string();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_use_calendar_days() {
        let now = 10 * 86_400 + 3_600;
        assert_eq!(bucket_of(now, now), RecentBucket::Today);
        assert_eq!(bucket_of(now - 86_400, now), RecentBucket::Week);
        assert_eq!(bucket_of(now - 7 * 86_400, now), RecentBucket::Week);
        assert_eq!(bucket_of(now - 8 * 86_400, now), RecentBucket::Earlier);
    }

    #[test]
    fn remote_host_reads_https_and_ssh() {
        assert_eq!(remote_host("https://github.com/mia/gitui.git").as_deref(), Some("github.com"));
        assert_eq!(remote_host("git@github.com:mia/gitui.git").as_deref(), Some("github.com"));
        assert_eq!(remote_host(""), None);
    }

    #[test]
    fn query_matches_path_without_case() {
        assert!(matches_query("/Users/Mia/Repo", "mia"));
        assert!(matches_query("/Users/Mia/Repo", ""));
        assert!(!matches_query("/Users/Mia/Repo", "git"));
    }
}
