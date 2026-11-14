//! 欢迎页（设计板 01 / 02 + 03 的入口）：左侧仓库栏 + 右侧英雄区与
//! Open / Clone / Init 三分段表单。
//!
//! 设计稿把「打开 / 克隆 / 新建」放在独立的仓库管理页（03）；按使用反馈合并进
//! 欢迎页：欢迎页本身就是入口页，打开仓库后进入多标签主窗口。
//!
//! 状态：有最近仓库（01）、没有仓库（02）、克隆中、克隆失败。

use crate::design::{self, metrics};
use crate::i18n;
use crate::widgets::{self, kbd};
use crate::{Actions, RepoForm};

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
use nsh_base::input;
use nsh_core::prelude::*;
use store::RecentRepo;

/// 打开时间距今不足这些秒时归入“今天”。
const TODAY_SECONDS: i64 = 24 * 60 * 60;
/// 归入“最近 7 天”的上限。
const WEEK_SECONDS: i64 = 7 * 24 * 60 * 60;

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
                            .pt(px(metrics::WELCOME_MAIN_PAD.0))
                            .pb(px(metrics::WELCOME_MAIN_PAD.2))
                            .px(px(metrics::WELCOME_MAIN_PAD.1))
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
                            .child(tip(language))
                            .child(view().h(px(24.0)).w_full()),
                    ),
                )
                .child(cloning_indicator(language, cloning)),
        );
}

/// 克隆进行中的提示（替换此前的半透明遮罩：遮罩既挡不住点击，也看不出在做什么）。
fn cloning_indicator(
    language: Signal<SharedString>,
    cloning: Signal<bool>,
) -> impl IntoElement {
    return view()
        .absolute()
        .bottom(px(16.0))
        .right(px(16.0))
        .h(move || if cloning.get() { px(26.0) } else { px(0.0) })
        .px(px(10.0))
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .rounded(px(6.0))
        .border_1()
        .border_color(move || design::line())
        .bg(move || design::panel())
        .overflow_hidden()
        .child(widgets::icon("refresh", 12.0, design::accent))
        .child(
            text(move || {
                if cloning.get() {
                    i18n::t(language.get().as_str(), "welcome.cloneWorking")
                } else {
                    String::new()
                }
            })
            .text_size(px(10.5))
            .text_color(move || design::text2()),
        );
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
        .w(px(metrics::WELCOME_SIDE_W))
        .h_full()
        .flex_shrink_0()
        .flex_col()
        .bg(move || design::sidebar())
        .border_r_1()
        .border_color(move || design::line())
        .child(
            view()
                .w_full()
                .pt(px(metrics::WELCOME_SIDE_TOP_PAD.0))
                .pb(px(metrics::WELCOME_SIDE_TOP_PAD.2))
                .px(px(metrics::WELCOME_SIDE_TOP_PAD.1))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    view()
                        .flex_1()
                        .min_w_0()
                        .flex_col()
                        .child(
                            text(move || {
                                i18n::t(language.get().as_str(), "welcome.repositories")
                            })
                            .text_size(px(metrics::FONT_WELCOME_SIDE_TITLE))
                            .font_weight(FontWeight(700))
                            .line_height(px(21.0))
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
                            .text_size(px(10.5))
                            .line_height(px(14.0))
                            .text_color(move || design::text3()),
                        ),
                )
                .child(widgets::small_button(
                    move || i18n::t(language.get().as_str(), "welcome.openFolder"),
                    {
                        let choose_folder = choose_folder.clone();
                        move || choose_folder()
                    },
                )),
        )
        .child(
            view()
                .mx(px(12.0))
                .mt(px(2.0))
                .mb(px(8.0))
                .h(px(metrics::FIELD_H))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .px(px(8.0))
                .rounded(px(metrics::RADIUS_CONTROL))
                .border_1()
                .border_color(move || design::field_border())
                .bg(move || design::field())
                .child(widgets::icon("search", 12.0, design::text3))
                .child(
                    input()
                        .value(query)
                        .on_change(move |value: &str| query.set(value.to_string()))
                        .placeholder(move || i18n::t(language.get().as_str(), "welcome.search"))
                        .text_size(px(12.0))
                        .flex_1(),
                )
                .child(kbd("⌘F")),
        )
        .child(
            view()
                .flex_1()
                .min_h_0()
                .px(px(metrics::SIDE_SCROLL_PAD.1))
                .child(
                    scroll().w_full().h_full().child(
                        view()
                            .w_full()
                            .flex_col()
                            .child(dynamic(move || -> AnyElement {
                                if recent.get().is_empty() {
                                    return view()
                                        .w_full()
                                        .mt(px(16.0))
                                        .px(px(8.0))
                                        .flex_col()
                                        .gap(px(4.0))
                                        .child(
                                            text(move || {
                                                i18n::t(
                                                    language.get().as_str(),
                                                    "welcome.noRecent",
                                                )
                                            })
                                            .text_size(px(11.5))
                                            .text_color(move || design::text2()),
                                        )
                                        .child(
                                            text(move || {
                                                i18n::t(
                                                    language.get().as_str(),
                                                    "welcome.noRecentHint",
                                                )
                                            })
                                            .text_size(px(10.0))
                                            .line_height(px(15.0))
                                            .text_color(move || design::text3()),
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
        .child(
            view()
                .w_full()
                .py(px(10.0))
                .px(px(14.0))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .border_t_1()
                .border_color(move || design::line())
                .child(
                    text(crate::brand::version_line())
                        .text_size(px(9.5))
                        .text_color(move || design::text3()),
                )
                .child(view().flex_1())
                .child(
                    text(move || i18n::t(language.get().as_str(), "welcome.checkUpdates"))
                        .text_size(px(9.5))
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
        .mt(px(metrics::WELCOME_GROUP_GAP))
        .px(px(6.0))
        .flex_col()
        .child(
            view()
                .w_full()
                .h(px(metrics::WELCOME_GROUP_LABEL_H))
                .px(px(8.0))
                .items_center()
                .child(
                    text(move || i18n::t(language.get().as_str(), label_key))
                        .text_size(px(9.5))
                        .font_weight(FontWeight(600))
                        .text_color(move || design::text3()),
                ),
        )
        .child(each(
            recent,
            |repo: &RecentRepo| repo.path.clone(),
            move |repo, _| {
                let age = (now - repo.get().opened_at).max(0);
                let visible = age >= from_seconds
                    && age < to_seconds
                    && matches_query(&repo.get().path, &query.get());
                let open_path = open_path.clone();
                view()
                    .w_full()
                    .h(move || {
                        if visible {
                            px(metrics::WELCOME_REPO_H)
                        } else {
                            px(0.0)
                        }
                    })
                    .flex_row()
                    .items_center()
                    .gap(px(9.0))
                    .px(px(8.0))
                    .rounded(px(7.0))
                    .overflow_hidden()
                    .hover(|style| style.bg(move || design::hover()))
                    .on_press(move || open_path(repo.get().path.clone()))
                    .child(
                        view()
                            .w(px(metrics::WELCOME_REPO_ICON))
                            .h(px(metrics::WELCOME_REPO_ICON))
                            .items_center()
                            .justify_center()
                            .rounded(px(7.0))
                            .border_1()
                            .border_color(move || design::line())
                            .bg(move || design::panel())
                            .child(widgets::icon("folder", 13.0, design::text2)),
                    )
                    .child(
                        view()
                            .flex_1()
                            .min_w_0()
                            .flex_col()
                            .gap(px(1.0))
                            .child(
                                text(move || last_segment(&repo.get().path))
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight(600))
                                    .line_height(px(15.0))
                                    .text_color(move || design::text())
                                    .text_overflow(nsh_core::EllipsizeMode::Tail),
                            )
                            .child(
                                text(move || home_path(&repo.get().path))
                                    .text_size(px(9.5))
                                    .line_height(px(12.0))
                                    .font_family("SF Mono")
                                    .text_color(move || design::text3())
                                    .text_overflow(nsh_core::EllipsizeMode::Tail),
                            ),
                    )
                    .child(
                        text(move || {
                            crate::dates::format(
                                language.get().as_str(),
                                repo.get().opened_at,
                                now,
                            )
                        })
                        .text_size(px(9.0))
                        .text_color(move || design::text3()),
                    )
            },
        ));
}

fn hero(language: Signal<SharedString>) -> impl IntoElement {
    return view()
        .w_full()
        .flex_row()
        .items_center()
        .gap(px(16.0))
        .child(
            view()
                .w(px(metrics::WELCOME_APPMARK))
                .h(px(metrics::WELCOME_APPMARK))
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .rounded(px(metrics::RADIUS_APPMARK))
                .bg(move || design::accent2())
                .child(widgets::icon("branch", 30.0, || {
                    design::token("primary-foreground")
                })),
        )
        .child(
            view()
                .min_w_0()
                .flex_col()
                .child(
                    text(move || i18n::t(language.get().as_str(), "app.name"))
                        .text_size(px(metrics::FONT_WELCOME_TITLE))
                        .font_weight(FontWeight(700))
                        .line_height(px(34.0))
                        .text_color(move || design::text()),
                )
                .child(
                    text(move || i18n::t(language.get().as_str(), "app.tagline"))
                        .mt(px(4.0))
                        .text_size(px(12.5))
                        .line_height(px(17.0))
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
    return view()
        .w_full()
        .max_w(px(metrics::WELCOME_MAX_W))
        .mt(px(metrics::WELCOME_SECTION_GAP.0))
        .pt(px(metrics::WELCOME_SECTION_GAP.1))
        .border_t_1()
        .border_color(move || design::line())
        .flex_col()
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .gap(px(10.0))
                .child(segment(language, form, set_form))
                .child(view().flex_1())
                .child(
                    text(move || match form.get() {
                        RepoForm::Open => "git open".to_string(),
                        RepoForm::Clone => "git clone".to_string(),
                        RepoForm::Init => "git init".to_string(),
                    })
                    .text_size(px(9.5))
                    .font_family("SF Mono")
                    .text_color(move || design::text3()),
                ),
        )
        .child(
            view()
                .w_full()
                .mt(px(12.0))
                .flex_col()
                .gap(px(9.0))
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
                    .text_size(px(13.0))
                    .font_weight(FontWeight(600))
                    .line_height(px(18.0))
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
                    .text_size(px(10.5))
                    .line_height(px(15.0))
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

fn segment(
    language: Signal<SharedString>,
    form: Signal<RepoForm>,
    set_form: std::sync::Arc<dyn Fn(RepoForm)>,
) -> impl IntoElement {
    let mut items: Vec<AnyElement> = Vec::new();
    for (kind, key) in [
        (RepoForm::Open, "repoManager.tabOpen"),
        (RepoForm::Clone, "repoManager.tabClone"),
        (RepoForm::Init, "repoManager.tabInit"),
    ] {
        let set_form = set_form.clone();
        items.push(
            view()
                .h(px(22.0))
                .px(px(10.0))
                .items_center()
                .justify_center()
                .rounded(px(5.0))
                .bg(move || {
                    if form.get() == kind {
                        design::content()
                    } else {
                        AdaptiveColor::TRANSPARENT
                    }
                })
                .on_press(move || set_form(kind))
                .child(
                    text(move || i18n::t(language.get().as_str(), key))
                        .text_size(px(11.5))
                        .line_height(px(15.0))
                        .text_color(move || {
                            if form.get() == kind {
                                design::text()
                            } else {
                                design::text2()
                            }
                        }),
                )
                .into_element(),
        );
    }
    return view()
        .h(px(metrics::SEG_H))
        .flex_row()
        .items_center()
        .gap(px(2.0))
        .p(px(2.0))
        .rounded(px(7.0))
        .border_1()
        .border_color(move || design::line_soft())
        .bg(move || design::panel2())
        .child(items);
}

fn field_label(language: Signal<SharedString>, key: &'static str) -> impl IntoElement {
    return text(move || i18n::t(language.get().as_str(), key))
        .text_size(px(11.0))
        .font_weight(FontWeight(600))
        .line_height(px(15.0))
        .text_color(move || design::text2());
}

fn text_field(value: Signal<String>, placeholder: &'static str) -> impl IntoElement {
    return view()
        .w_full()
        .h(px(metrics::FIELD_H))
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .px(px(8.0))
        .rounded(px(metrics::RADIUS_CONTROL))
        .border_1()
        .border_color(move || design::field_border())
        .bg(move || design::field())
        .child(
            input()
                .value(value)
                .on_change(move |text: &str| value.set(text.to_string()))
                .placeholder(placeholder)
                .text_size(px(12.0))
                .flex_1(),
        );
}

fn open_form(language: Signal<SharedString>, actions: Actions) -> impl IntoElement {
    let choose_folder = actions.choose_folder.clone();
    let choose_folder_two = actions.choose_folder.clone();
    return view()
        .w_full()
        .flex_col()
        .gap(px(10.0))
        .child(
            text(move || i18n::t(language.get().as_str(), "repoManager.openHint"))
                .text_size(px(10.5))
                .line_height(px(15.0))
                .text_color(move || design::text2()),
        )
        .child(
            view()
                .flex_row()
                .gap(px(8.0))
                .child(widgets::primary_button(
                    move || i18n::t(language.get().as_str(), "welcome.openFolder"),
                    {
                        let choose_folder = choose_folder.clone();
                        move || choose_folder()
                    },
                ))
                .child(widgets::button(
                    move || i18n::t(language.get().as_str(), "repoManager.title"),
                    {
                        let choose_folder_two = choose_folder_two.clone();
                        move || choose_folder_two()
                    },
                )),
        );
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
    return view()
        .w_full()
        .flex_col()
        .gap(px(9.0))
        .child(
            view()
                .w_full()
                .flex_row()
                .items_end()
                .gap(px(9.0))
                .child(
                    view()
                        .flex_1()
                        .min_w_0()
                        .flex_col()
                        .gap(px(5.0))
                        .child(field_label(language, "welcome.url"))
                        .child(text_field(clone_url, "https://github.com/owner/repo.git")),
                )
                .child(protocol_toggle(language)),
        )
        .child(
            view()
                .w_full()
                .flex_row()
                .items_end()
                .gap(px(9.0))
                .child(
                    view()
                        .flex_1()
                        .min_w_0()
                        .flex_col()
                        .gap(px(5.0))
                        .child(field_label(language, "welcome.destination"))
                        .child(text_field(clone_destination, "~/Developer/repo")),
                )
                .child(widgets::button(
                    move || i18n::t(language.get().as_str(), "welcome.choose"),
                    {
                        let choose_folder = choose_folder.clone();
                        move || choose_folder()
                    },
                )),
        )
        // 选项行：分支、深度、子模块、克隆后打开（设计板 03）。
        .child(
            view()
                .w_full()
                .flex_row()
                .items_end()
                .gap(px(9.0))
                .child(
                    view()
                        .flex_1()
                        .min_w_0()
                        .flex_col()
                        .gap(px(5.0))
                        .child(field_label(language, "welcome.branch"))
                        .child(
                            view()
                                .w_full()
                                .h(px(metrics::FIELD_H))
                                .px(px(7.0))
                                .items_center()
                                .rounded(px(metrics::RADIUS_CONTROL))
                                .border_1()
                                .border_color(move || design::field_border())
                                .bg(move || design::field())
                                .child(
                                    nsh_base::input()
                                        .value({
                                        let options = options;
                                        move || options.get().branch
                                    })
                                        .on_change(move |text: &str| {
                                            let mut next = options.get_untracked();
                                            next.branch = text.to_string();
                                            options.set(next);
                                        })
                                        .placeholder(move || {
                                            i18n::t(
                                                language.get().as_str(),
                                                "welcome.branchHint",
                                            )
                                        })
                                        .text_size(px(11.0))
                                        .flex_1(),
                                ),
                        ),
                )
                .child(
                    view()
                        .w(px(170.0))
                        .flex_col()
                        .gap(px(5.0))
                        .child(field_label(language, "welcome.depth"))
                        .child(depth_toggle(language, options)),
                ),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(14.0))
                .child(check_line(
                    language,
                    "welcome.recurse",
                    options.get_untracked().recurse_submodules,
                    move || {
                        let mut next = options.get_untracked();
                        next.recurse_submodules = !next.recurse_submodules;
                        options.set(next);
                    },
                ))
                .child(check_line(
                    language,
                    "welcome.openAfterClone",
                    options.get_untracked().open_after_clone,
                    move || {
                        let mut next = options.get_untracked();
                        next.open_after_clone = !next.open_after_clone;
                        options.set(next);
                    },
                )),
        )
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(widgets::primary_button(
                    move || {
                        if cloning.get() {
                            i18n::t(language.get().as_str(), "welcome.cloneWorking")
                        } else {
                            i18n::t(language.get().as_str(), "welcome.clone")
                        }
                    },
                    {
                        let clone_repository = clone_repository.clone();
                        move || clone_repository()
                    },
                ))
                .child(
                    text(move || i18n::t(language.get().as_str(), "welcome.cloneHint"))
                        .text_size(px(10.5))
                        .text_color(move || design::text3()),
                ),
        )
        .child(error_line(error));
}

/// 深度三选一：完整 / 浅 1 / 浅 50。
fn depth_toggle(language: Signal<SharedString>, options: Signal<CloneOptions>) -> impl IntoElement {
    return view()
        .w_full()
        .h(px(metrics::FIELD_H))
        .flex_row()
        .items_center()
        .gap(px(2.0))
        .p(px(2.0))
        .rounded(px(metrics::RADIUS_CONTROL))
        .border_1()
        .border_color(move || design::field_border())
        .bg(move || design::panel2())
        .child(
            [0u32, 1, 50]
                .iter()
                .map(|depth| {
                    let depth = *depth;
                    let active = options.get_untracked().depth == depth;
                    let label = match depth {
                        0 => "welcome.depthFull",
                        1 => "welcome.depthOne",
                        _ => "welcome.depthFifty",
                    };
                    let language = language;
                    return view()
                        .flex_1()
                        .h(px(18.0))
                        .items_center()
                        .justify_center()
                        .rounded(px(4.0))
                        .bg(move || {
                            if active {
                                design::content()
                            } else {
                                AdaptiveColor::TRANSPARENT
                            }
                        })
                        .on_press(move || {
                            let mut next = options.get_untracked();
                            next.depth = depth;
                            next.depth_custom.clear();
                            options.set(next);
                        })
                        .child(
                            text(move || i18n::t(language.get().as_str(), label))
                                .text_size(px(9.5))
                                .line_height(px(12.0))
                                .text_color(move || {
                                    if active {
                                        design::text()
                                    } else {
                                        design::text3()
                                    }
                                }),
                        )
                        .into_element();
                })
                .collect::<Vec<AnyElement>>(),
        );
}

/// 一行复选（12px 方框 + 文案）。
fn check_line(
    language: Signal<SharedString>,
    key: &'static str,
    checked: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    return view()
        .h(px(18.0))
        .flex_row()
        .items_center()
        .gap(px(6.0))
        .on_press(on_press)
        .child(
            view()
                .w(px(12.0))
                .h(px(12.0))
                .items_center()
                .justify_center()
                .rounded(px(3.0))
                .border_1()
                .border_color(move || design::line_strong())
                .bg(move || {
                    if checked {
                        design::accent()
                    } else {
                        AdaptiveColor::TRANSPARENT
                    }
                })
                .child(
                    text(move || {
                        if checked {
                            "✓".to_string()
                        } else {
                            String::new()
                        }
                    })
                    .text_size(px(8.0))
                    .text_color(move || design::content()),
                ),
        )
        .child(
            text(move || i18n::t(language.get().as_str(), key))
                .text_size(px(10.5))
                .line_height(px(14.0))
                .text_color(move || design::text2()),
        );
}

fn init_form(
    language: Signal<SharedString>,
    init_path: Signal<String>,
    files: Signal<InitFiles>,
    error: Signal<Option<String>>,
    actions: Actions,
) -> impl IntoElement {
    let init_repository = actions.init_repository.clone();
    return view()
        .w_full()
        .flex_col()
        .gap(px(9.0))
        .child(field_label(language, "repoManager.initPath"))
        .child(text_field(init_path, "~/Developer/new-repo"))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap(px(14.0))
                .child(check_line(
                    language,
                    "welcome.gitignore",
                    files.get_untracked().gitignore,
                    move || {
                        let mut next = files.get_untracked();
                        next.gitignore = !next.gitignore;
                        files.set(next);
                    },
                ))
                .child(check_line(
                    language,
                    "welcome.readme",
                    files.get_untracked().readme,
                    move || {
                        let mut next = files.get_untracked();
                        next.readme = !next.readme;
                        files.set(next);
                    },
                )),
        )
        .child(widgets::primary_button(
            move || i18n::t(language.get().as_str(), "repoManager.initAction"),
            {
                let init_repository = init_repository.clone();
                move || init_repository()
            },
        ))
        .child(error_line(error));
}

fn error_line(error: Signal<Option<String>>) -> impl IntoElement {
    return view()
        .w_full()
        .h(move || {
            if error.get().is_some() {
                px(18.0)
            } else {
                px(0.0)
            }
        })
        .overflow_hidden()
        .items_center()
        .child(
            text(move || error.get().unwrap_or_default())
                .text_size(px(10.5))
                .line_height(px(15.0))
                .text_color(move || design::danger()),
        );
}

fn protocol_toggle(language: Signal<SharedString>) -> impl IntoElement {
    return view()
        .h(px(20.0))
        .flex_shrink_0()
        .flex_row()
        .items_center()
        .gap(px(2.0))
        .p(px(2.0))
        .rounded(px(5.0))
        .bg(move || design::panel2())
        .child(
            view()
                .h(px(16.0))
                .px(px(6.0))
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .bg(move || design::content())
                .child(
                    text("HTTPS")
                        .text_size(px(9.5))
                        .line_height(px(12.0))
                        .text_color(move || design::text()),
                ),
        )
        .child(
            view().h(px(16.0)).px(px(6.0)).items_center().justify_center().child(
                text("SSH")
                    .text_size(px(9.5))
                    .line_height(px(12.0))
                    .text_color(move || {
                        let _ = language;
                        design::text3()
                    }),
            ),
        );
}

fn tip(language: Signal<SharedString>) -> impl IntoElement {
    return view()
        .w_full()
        .max_w(px(metrics::WELCOME_MAX_W))
        .mt(px(metrics::WELCOME_TIP_TOP))
        .py(px(10.0))
        .px(px(12.0))
        .flex_row()
        .items_start()
        .gap(px(8.0))
        .rounded(px(metrics::RADIUS_PANEL))
        .border_1()
        .border_color(move || design::line())
        .bg(move || design::panel())
        .child(
            view()
                .mt(px(1.0))
                .flex_shrink_0()
                .child(widgets::icon("info", 12.0, design::accent)),
        )
        .child(
            text(move || {
                let language = language.get();
                format!(
                    "{}  {}",
                    i18n::t(language.as_str(), "welcome.tipTitle"),
                    i18n::t(language.as_str(), "welcome.tipBody")
                )
            })
            .text_size(px(10.5))
            .line_height(px(15.0))
            .text_color(move || design::text2()),
        );
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
