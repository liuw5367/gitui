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

use crate::design;
use crate::i18n;
use crate::ui;
use crate::ui::badge::Tone;
use crate::ui::button::{button, ButtonSize, ButtonVariant};
use crate::{Actions, RepoForm};
use nsh_core::prelude::*;
use store::RecentRepo;

/// Init 表单的附带文件（设计板 03）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct InitFiles {
    pub gitignore: bool,
    pub readme: bool,
}

/// Clone 表单的选项（设计板 03）：分支、深度、子模块、克隆后打开。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CloneOptions {
    pub branch: String,
    /// 0 = 完整克隆。
    pub depth: u32,
    /// 用户自定义深度输入（空则用 `depth`）。
    pub depth_custom: String,
    pub recurse_submodules: bool,
    pub open_after_clone: bool,
}

/// 打开时间距今不足这些秒时归入“今天”。
const TODAY_SECONDS: i64 = 24 * 60 * 60;
/// 归入“最近 7 天”的上限。
const WEEK_SECONDS: i64 = 7 * 24 * 60 * 60;

/// 欢迎页左栏宽度（设计板 01 的 `.wside`）。
const SIDE_WIDTH: f32 = 312.0;
/// 右栏内容的最大宽度（设计板 01 的 `.wsection`）。
const MAIN_MAX_WIDTH: f32 = 600.0;
/// 右栏内边距：上、左右、下。
const MAIN_PAD: (f32, f32, f32) = (40.0, 48.0, 28.0);
/// 英雄区应用标记的边长。
const APP_MARK: f32 = 64.0;

#[allow(clippy::too_many_arguments)]
pub(crate) fn welcome(
    language: Signal<SharedString>,
    recent: Signal<Vec<RecentRepo>>,
    query: Signal<String>,
    clone_url: Signal<String>,
    clone_destination: Signal<String>,
    clone_options: Signal<CloneOptions>,
    init_files: Signal<InitFiles>,
    init_path: Signal<String>,
    clone_error: Signal<Option<String>>,
    cloning: Signal<bool>,
    clone_progress: Signal<String>,
    form: Signal<RepoForm>,
    actions: Actions,
) -> impl IntoElement {
    return view()
        .w_full()
        .h_full()
        .flex_row()
        .bg(move || design::content())
        .child(sidebar(language, recent, query, actions.clone()))
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
                            .child(entry_section(
                                language,
                                form,
                                clone_url,
                                clone_destination,
                                clone_options,
                                init_files,
                                init_path,
                                clone_error,
                                cloning,
                                actions.clone(),
                            ))
                            .child(tip(language)),
                    ),
                )
                .child(cloning_indicator(
                    language,
                    cloning,
                    clone_progress,
                    actions.cancel_clone.clone(),
                )),
        );
}

/// 克隆进行中的提示：浮在右下角，不遮挡表单。
fn cloning_indicator(
    language: Signal<SharedString>,
    cloning: Signal<bool>,
    progress: Signal<String>,
    cancel: std::sync::Arc<dyn Fn()>,
) -> impl IntoElement {
    return view()
        .absolute()
        .bottom(px(ui::GAP_XL))
        .right(px(ui::GAP_XL))
        .h(move || if cloning.get() { px(36.0) } else { px(0.0) })
        .flex_row()
        .items_center()
        .gap(px(ui::GAP))
        .px(px(ui::GAP_LG))
        .rounded(px(8.0))
        .border_1()
        .border_color(move || design::line())
        .bg(move || design::panel())
        .overflow_hidden()
        .child(ui::icon::icon_view(
            "refresh",
            ui::icon::IconSize::Inline,
            design::accent,
        ))
        .child(
            text(move || {
                if !cloning.get() {
                    return String::new();
                }
                let stage = progress.get();
                if stage.trim().is_empty() {
                    i18n::t(language.get().as_str(), "welcome.cloneWorking")
                } else {
                    stage
                }
            })
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
            {
                let cancel = cancel.clone();
                move || cancel()
            },
        ));
}

fn sidebar(
    language: Signal<SharedString>,
    recent: Signal<Vec<RecentRepo>>,
    query: Signal<String>,
    actions: Actions,
) -> impl IntoElement {
    let open_path = actions.open_path.clone();
    let choose_folder = actions.choose_folder.clone();
    let now = store::now_unix();
    return view()
        .w(px(SIDE_WIDTH))
        .h_full()
        .flex_shrink_0()
        .flex_col()
        .bg(move || design::sidebar())
        .border_r_1()
        .border_color(move || design::line())
        // 标题 + 打开按钮
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
                            text(move || {
                                i18n::t(language.get().as_str(), "welcome.repositories")
                            })
                            .text_size(px(ui::TITLE))
                            .line_height(px(20.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(move || design::text()),
                        )
                        .child(
                            text(move || {
                                i18n::with_arguments(
                                    language.get().as_str(),
                                    "welcome.recentCount",
                                    &[("count", recent.get().len().to_string())],
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
                        move || choose_folder()
                    },
                )),
        )
        // 搜索
        .child(
            view()
                .w_full()
                .px(px(ui::GAP_LG))
                .pb(px(ui::GAP_SM))
                .child(ui::text_input::search_field(
                    query,
                    move || i18n::t(language.get().as_str(), "welcome.search"),
                    Some("⌘F"),
                    move |_text| {},
                )),
        )
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .px(px(ui::GAP_SM))
                .child(
                    scroll().w_full().h_full().child(
                        view()
                            .w_full()
                            .py(px(ui::GAP_XS))
                            .flex_col()
                            .child(dynamic(move || -> AnyElement {
                                if recent.get().is_empty() {
                                    return ui::section::empty(
                                        move || {
                                            i18n::t(language.get().as_str(), "welcome.noRecent")
                                        },
                                        Some(Box::new(move || {
                                            i18n::t(
                                                language.get().as_str(),
                                                "welcome.noRecentHint",
                                            )
                                        })),
                                        None,
                                    )
                                    .into_element();
                                }
                                return view()
                                    .w_full()
                                    .flex_col()
                                    .child(repo_group(
                                        language,
                                        recent,
                                        query,
                                        now,
                                        "welcome.groupToday",
                                        0,
                                        TODAY_SECONDS,
                                        open_path.clone(),
                                    ))
                                    .child(repo_group(
                                        language,
                                        recent,
                                        query,
                                        now,
                                        "welcome.groupWeek",
                                        TODAY_SECONDS,
                                        WEEK_SECONDS,
                                        open_path.clone(),
                                    ))
                                    .into_element();
                            })),
                    ),
                ),
        )
        // 底部：版本与更新入口（更新检查尚未实现，只显示版本，不做假按钮）
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
                .child(
                    text(move || i18n::t(language.get().as_str(), "welcome.checkUpdates"))
                        .text_size(px(ui::META))
                        .line_height(px(ui::SECONDARY_LINE))
                        .text_color(move || design::text3()),
                ),
        );
}

#[allow(clippy::too_many_arguments)]
fn repo_group(
    language: Signal<SharedString>,
    recent: Signal<Vec<RecentRepo>>,
    query: Signal<String>,
    now: i64,
    label_key: &'static str,
    from_seconds: i64,
    to_seconds: i64,
    open_path: std::sync::Arc<dyn Fn(String)>,
) -> impl IntoElement {
    return view()
        .w_full()
        .mt(px(ui::GAP_SM))
        .flex_col()
        .child(ui::section::section_label(move || {
            i18n::t(language.get().as_str(), label_key)
        }))
        .child(each(
            recent,
            |repo: &RecentRepo| repo.path.clone(),
            move |repo, _| {
                let age = (now - repo.get().opened_at).max(0);
                let visible = age >= from_seconds
                    && age < to_seconds
                    && matches_query(&repo.get().path, &query.get());
                let open_path = open_path.clone();
                return view()
                    .w_full()
                    .h(move || {
                        if visible {
                            px(44.0)
                        } else {
                            px(0.0)
                        }
                    })
                    .overflow_hidden()
                    .child(ui::row::row(
                        move || last_segment(&repo.get().path),
                        Some((
                            Box::new(move || home_path(&repo.get().path)),
                            true,
                        )),
                        Some(
                            view()
                                .w(px(26.0))
                                .h(px(26.0))
                                .flex_shrink_0()
                                .items_center()
                                .justify_center()
                                .rounded(px(6.0))
                                .border_1()
                                .border_color(move || design::line())
                                .bg(move || design::panel())
                                .child(ui::icon::icon_view(
                                    "folder",
                                    ui::icon::IconSize::Inline,
                                    design::text2,
                                ))
                                .into_element(),
                        ),
                        vec![
                            ui::meta(move || {
                                crate::dates::format(
                                    language.get().as_str(),
                                    repo.get().opened_at,
                                    now,
                                )
                            })
                            .into_element(),
                        ],
                        false,
                        false,
                        move || open_path(repo.get().path.clone()),
                    ));
            },
        ));
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
                .rounded(px(14.0))
                .bg(move || design::token("primary-bg"))
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
#[allow(clippy::too_many_arguments)]
fn entry_section(
    language: Signal<SharedString>,
    form: Signal<RepoForm>,
    clone_url: Signal<String>,
    clone_destination: Signal<String>,
    clone_options: Signal<CloneOptions>,
    init_files: Signal<InitFiles>,
    init_path: Signal<String>,
    error: Signal<Option<String>>,
    cloning: Signal<bool>,
    actions: Actions,
) -> impl IntoElement {
    let set_form = actions.set_repo_form.clone();
    let selected = match form.get_untracked() {
        RepoForm::Open => "open",
        RepoForm::Clone => "clone",
        RepoForm::Init => "init",
    };
    return view()
        .w_full()
        .max_w(px(MAIN_MAX_WIDTH))
        .mt(px(28.0))
        .pt(px(20.0))
        .border_t_1()
        .border_color(move || design::line())
        .flex_col()
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .gap(px(ui::GAP_LG))
                .child(ui::segmented::segmented(
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
                    {
                        let set_form = set_form.clone();
                        move |id| {
                            set_form(match id {
                                "clone" => RepoForm::Clone,
                                "init" => RepoForm::Init,
                                _ => RepoForm::Open,
                            })
                        }
                    },
                ))
                .child(view().flex_1())
                .child(ui::meta(move || {
                    match form.get() {
                        RepoForm::Open => "git open".to_string(),
                        RepoForm::Clone => "git clone".to_string(),
                        RepoForm::Init => "git init".to_string(),
                    }
                })),
        )
        .child(
            view()
                .w_full()
                .mt(px(ui::GAP_LG))
                .flex_col()
                .gap(px(ui::GAP_SM))
                .child(
                    text(move || {
                        i18n::t(
                            language.get().as_str(),
                            match form.get() {
                                RepoForm::Open => "repoManager.openTitle",
                                RepoForm::Clone => "welcome.cloneTitle",
                                RepoForm::Init => "repoManager.initTitle",
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
                                RepoForm::Open => "repoManager.openHint",
                                RepoForm::Clone => "welcome.cloneSub",
                                RepoForm::Init => "repoManager.initHint",
                            },
                        )
                    })
                    .text_size(px(ui::SECONDARY))
                    .line_height(px(ui::SECONDARY_LINE))
                    .text_color(move || design::text3()),
                )
                .child(dynamic(move || -> AnyElement {
                    match form.get() {
                        RepoForm::Open => open_form(language, actions.clone()).into_element(),
                        RepoForm::Clone => clone_form(
                            language,
                            clone_url,
                            clone_destination,
                            clone_options,
                            error,
                            cloning,
                            actions.clone(),
                        )
                        .into_element(),
                        RepoForm::Init => {
                            init_form(language, init_path, init_files, error, actions.clone())
                                .into_element()
                        }
                    }
                })),
        );
}

fn open_form(language: Signal<SharedString>, actions: Actions) -> impl IntoElement {
    let choose_folder = actions.choose_folder.clone();
    return view()
        .w_full()
        .pt(px(ui::GAP_SM))
        .flex_row()
        .items_center()
        .gap(px(ui::GAP))
        .child(button(
            move || i18n::t(language.get().as_str(), "welcome.openFolder"),
            ButtonVariant::Primary,
            ButtonSize::Regular,
            false,
            {
                let choose_folder = choose_folder.clone();
                move || choose_folder()
            },
        ));
}

#[allow(clippy::too_many_arguments)]
fn clone_form(
    language: Signal<SharedString>,
    clone_url: Signal<String>,
    clone_destination: Signal<String>,
    options: Signal<CloneOptions>,
    error: Signal<Option<String>>,
    cloning: Signal<bool>,
    actions: Actions,
) -> impl IntoElement {
    let choose_folder = actions.choose_folder.clone();
    let clone_repository = actions.clone_repository.clone();
    let set_url = clone_url;
    let set_destination = clone_destination;
    return view()
        .w_full()
        .pt(px(ui::GAP_SM))
        .flex_col()
        .gap(px(ui::GAP_LG))
        .child(ui::field::field(
            move || i18n::t(language.get().as_str(), "welcome.url"),
            ui::text_input::text_input(
                set_url,
                move || "https://github.com/owner/repo.git".to_string(),
                false,
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
                .child(
                    view()
                        .flex_1()
                        .min_w_0()
                        .child(ui::field::field(
                            move || i18n::t(language.get().as_str(), "welcome.destination"),
                            ui::text_input::text_input(
                                set_destination,
                                move || "~/Developer/repo".to_string(),
                                false,
                                move |text| set_destination.set(text.to_string()),
                            ),
                            None,
                            None,
                        )),
                )
                .child(button(
                    move || i18n::t(language.get().as_str(), "welcome.choose"),
                    ButtonVariant::Secondary,
                    ButtonSize::Regular,
                    false,
                    {
                        let choose_folder = choose_folder.clone();
                        move || choose_folder()
                    },
                )),
        )
        .child(
            view()
                .w_full()
                .flex_row()
                .items_end()
                .gap(px(ui::GAP))
                .child(
                    view()
                        .flex_1()
                        .min_w_0()
                        .child(ui::field::field(
                            move || i18n::t(language.get().as_str(), "welcome.branch"),
                            ui::text_input::text_input_bound(
                                {
                                    let options = options;
                                    move || options.get().branch
                                },
                                move || {
                                    i18n::t(language.get().as_str(), "welcome.branchHint")
                                },
                                false,
                                move |text| {
                                    let mut next = options.get_untracked();
                                    next.branch = text;
                                    options.set(next);
                                },
                            ),
                            None,
                            None,
                        )),
                )
                .child(
                    view()
                        .w(px(220.0))
                        .flex_col()
                        .gap(px(ui::GAP_XS))
                        .child(ui::field::label(move || {
                            i18n::t(language.get().as_str(), "welcome.depth")
                        }))
                        .child(ui::segmented::segmented(
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
                            match options.get_untracked().depth {
                                1 => "one",
                                50 => "fifty",
                                _ => "full",
                            },
                            move |id| {
                                let mut next = options.get_untracked();
                                next.depth = match id {
                                    "one" => 1,
                                    "fifty" => 50,
                                    _ => 0,
                                };
                                next.depth_custom.clear();
                                options.set(next);
                            },
                        )),
                ),
        )
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .gap(px(ui::GAP_XL))
                .child(ui::check::check_row(
                    move || i18n::t(language.get().as_str(), "welcome.recurse"),
                    options.get_untracked().recurse_submodules,
                    false,
                    move |next| {
                        let mut options_next = options.get_untracked();
                        options_next.recurse_submodules = next;
                        options.set(options_next);
                    },
                ))
                .child(ui::check::check_row(
                    move || i18n::t(language.get().as_str(), "welcome.openAfterClone"),
                    options.get_untracked().open_after_clone,
                    false,
                    move |next| {
                        let mut options_next = options.get_untracked();
                        options_next.open_after_clone = next;
                        options.set(options_next);
                    },
                )),
        )
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .gap(px(ui::GAP))
                .child(button(
                    move || {
                        if cloning.get() {
                            i18n::t(language.get().as_str(), "welcome.cloneWorking")
                        } else {
                            i18n::t(language.get().as_str(), "welcome.clone")
                        }
                    },
                    ButtonVariant::Primary,
                    ButtonSize::Regular,
                    cloning.get_untracked(),
                    {
                        let clone_repository = clone_repository.clone();
                        move || clone_repository()
                    },
                ))
                .child(ui::secondary(move || {
                    i18n::t(language.get().as_str(), "welcome.cloneHint")
                })),
        )
        .child(dynamic(move || -> AnyElement {
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
        }));
}

fn init_form(
    language: Signal<SharedString>,
    init_path: Signal<String>,
    files: Signal<InitFiles>,
    error: Signal<Option<String>>,
    actions: Actions,
) -> impl IntoElement {
    let init_repository = actions.init_repository.clone();
    let path = init_path;
    return view()
        .w_full()
        .pt(px(ui::GAP_SM))
        .flex_col()
        .gap(px(ui::GAP_LG))
        .child(ui::field::field(
            move || i18n::t(language.get().as_str(), "repoManager.initPath"),
            ui::text_input::text_input(
                path,
                move || "~/Developer/new-repo".to_string(),
                false,
                move |text| path.set(text.to_string()),
            ),
            None,
            None,
        ))
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .gap(px(ui::GAP_XL))
                .child(ui::check::check_row(
                    move || i18n::t(language.get().as_str(), "welcome.gitignore"),
                    files.get_untracked().gitignore,
                    false,
                    move |next| {
                        let mut updated = files.get_untracked();
                        updated.gitignore = next;
                        files.set(updated);
                    },
                ))
                .child(ui::check::check_row(
                    move || i18n::t(language.get().as_str(), "welcome.readme"),
                    files.get_untracked().readme,
                    false,
                    move |next| {
                        let mut updated = files.get_untracked();
                        updated.readme = next;
                        files.set(updated);
                    },
                )),
        )
        .child(button(
            move || i18n::t(language.get().as_str(), "repoManager.initAction"),
            ButtonVariant::Primary,
            ButtonSize::Regular,
            false,
            {
                let init_repository = init_repository.clone();
                move || init_repository()
            },
        ))
        .child(dynamic(move || -> AnyElement {
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
        }));
}

fn tip(language: Signal<SharedString>) -> impl IntoElement {
    return view()
        .w_full()
        .max_w(px(MAIN_MAX_WIDTH))
        .mt(px(24.0))
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
                    text(move || {
                        let language = language.get();
                        format!(
                            "{}  {}",
                            i18n::t(language.as_str(), "welcome.tipTitle"),
                            i18n::t(language.as_str(), "welcome.tipBody")
                        )
                    })
                    .flex_1()
                    .min_w_0()
                    .text_size(px(ui::SECONDARY))
                    .line_height(px(ui::SECONDARY_LINE))
                    .text_color(move || design::text2()),
                )
                .child(ui::badge::badge(move || "K".to_string(), Tone::Neutral)),
        ));
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
