//! 仓库管理页（设计板 03）：左侧仓库列表，右侧 Open / Clone / Init 三种入口。
//!
//! 列表项显示仓库名、当前分支与「已打开」标记；右侧是表单区，分段控件切换入口。
//! 与欢迎页复用同一份最近仓库数据与克隆表单状态。

use crate::design::{self, metrics};
use crate::i18n;
use crate::widgets;
use crate::{Actions, RepoForm};
use nsh_core::prelude::*;
use store::RecentRepo;

/// 左侧列表宽度（设计板 `.repo-side`）。
const SIDE_W: f32 = 300.0;

#[allow(clippy::too_many_arguments)]
pub(crate) fn repo_manager(
    language: Signal<SharedString>,
    recent: Signal<Vec<RecentRepo>>,
    query: Signal<String>,
    clone_url: Signal<String>,
    clone_destination: Signal<String>,
    init_path: Signal<String>,
    error: Signal<Option<String>>,
    busy: Signal<bool>,
    form: Signal<RepoForm>,
    actions: Actions,
) -> impl IntoElement {
    return view()
        .w_full()
        .h_full()
        .flex_col()
        .bg(move || design::content())
        .child(titlebar(language, recent, actions.clone()))
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .flex_row()
                .child(side(language, recent, query, actions.clone()))
                .child(form_area(
                    language,
                    clone_url,
                    clone_destination,
                    init_path,
                    error,
                    busy,
                    form,
                    actions,
                )),
        );
}

fn titlebar(
    language: Signal<SharedString>,
    recent: Signal<Vec<RecentRepo>>,
    actions: Actions,
) -> impl IntoElement {
    let go_welcome = actions.go_welcome.clone();
    return view()
        .w_full()
        .h(px(metrics::TITLEBAR_PLAIN_H))
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .px(px(metrics::WINDOW_PAD))
        .bg(move || design::titlebar())
        .border_b_1()
        .border_color(move || design::line())
        .window_drag(true)
        .child(
            view()
                .w(px(60.0))
                .h(px(14.0))
                .child(
                    text(move || i18n::t(language.get().as_str(), "repoManager.title"))
                        .text_size(px(12.5))
                        .font_weight(FontWeight(600))
                        .text_color(move || design::text()),
                ),
        )
        .child(view().flex_1())
        .child(
            text(move || {
                i18n::with_arguments(
                    language.get().as_str(),
                    "repoManager.count",
                    &[("count", recent.get().len().to_string())],
                )
            })
            .text_size(px(11.0))
            .text_color(move || design::text3()),
        )
        .child(widgets::icon_button(
            "chev_left",
            13.0,
            design::text2,
            move || go_welcome(),
        ));
}

fn side(
    language: Signal<SharedString>,
    recent: Signal<Vec<RecentRepo>>,
    query: Signal<String>,
    actions: Actions,
) -> impl IntoElement {
    let open_path = actions.open_path.clone();
    let choose_folder = actions.choose_folder.clone();
    let clone_form = actions.set_repo_form.clone();
    let init_form = actions.set_repo_form.clone();
    return view()
        .w(px(SIDE_W))
        .h_full()
        .flex_shrink_0()
        .flex_col()
        .bg(move || design::sidebar())
        .border_r_1()
        .border_color(move || design::line())
        .child(
            view()
                .w_full()
                .pt(px(14.0))
                .pb(px(8.0))
                .px(px(14.0))
                .flex_col()
                .gap(px(2.0))
                .child(
                    text(move || i18n::t(language.get().as_str(), "welcome.repositories"))
                        .text_size(px(15.0))
                        .font_weight(FontWeight(700))
                        .text_color(move || design::text()),
                )
                .child(
                    text(move || {
                        i18n::with_arguments(
                            language.get().as_str(),
                            "repoManager.subtitle",
                            &[("count", recent.get().len().to_string())],
                        )
                    })
                    .text_size(px(10.5))
                    .text_color(move || design::text3()),
                )
                .child(
                    view()
                        .w_full()
                        .mt(px(8.0))
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
                            nsh_base::input()
                                .value(query)
                                .on_change(move |value: &str| query.set(value.to_string()))
                                .placeholder(move || {
                                    i18n::t(language.get().as_str(), "welcome.search")
                                })
                                .text_size(px(12.0))
                                .flex_1(),
                        ),
                ),
        )
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .px(px(6.0))
                .child(scroll().w_full().h_full().child(
                    view().w_full().flex_col().child(each(
                        recent,
                        |repo: &RecentRepo| repo.path.clone(),
                        move |repo, _| {
                            let open_path = open_path.clone();
                            let path = repo.get().path;
                            let visible = query.get().trim().is_empty()
                                || path.to_lowercase().contains(&query.get().to_lowercase());
                            let name = short_name(&path);
                            view()
                                .w_full()
                                .h(move || {
                                    if visible {
                                        px(36.0)
                                    } else {
                                        px(0.0)
                                    }
                                })
                                .px(px(8.0))
                                .flex_row()
                                .items_center()
                                .gap(px(8.0))
                                .rounded(px(metrics::RADIUS_CONTROL))
                                .hover(|style| style.bg(move || design::hover()))
                                .on_press(move || open_path(path.clone()))
                                .child(widgets::icon("folder", 12.0, design::text3))
                                .child(
                                    text(name)
                                        .flex_1()
                                        .text_size(px(12.5))
                                        .text_color(move || design::text())
                                        .text_overflow(nsh_core::EllipsizeMode::Tail),
                                )
                                .child(
                                    text(move || i18n::t(language.get().as_str(), "shell.openRepo"))
                                        .text_size(px(10.0))
                                        .text_color(move || design::text3()),
                                )
                        },
                    )),
                )),
        )
        .child(
            view()
                .w_full()
                .py(px(10.0))
                .px(px(12.0))
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .border_t_1()
                .border_color(move || design::line())
                .child(widgets::small_button(
                    move || i18n::t(language.get().as_str(), "repoManager.openEllipsis"),
                    {
                        let choose_folder = choose_folder.clone();
                        move || choose_folder()
                    },
                ))
                .child(widgets::small_button(
                    move || i18n::t(language.get().as_str(), "repoManager.cloneEllipsis"),
                    {
                        let clone_form = clone_form.clone();
                        move || clone_form(RepoForm::Clone)
                    },
                ))
                .child(widgets::small_button(
                    move || i18n::t(language.get().as_str(), "repoManager.initEllipsis"),
                    {
                        let init_form = init_form.clone();
                        move || init_form(RepoForm::Init)
                    },
                )),
        );
}

#[allow(clippy::too_many_arguments)]
fn form_area(
    language: Signal<SharedString>,
    clone_url: Signal<String>,
    clone_destination: Signal<String>,
    init_path: Signal<String>,
    error: Signal<Option<String>>,
    busy: Signal<bool>,
    form: Signal<RepoForm>,
    actions: Actions,
) -> impl IntoElement {
    let set_form = actions.set_repo_form.clone();
    let choose_folder = actions.choose_folder.clone();
    let clone_repository = actions.clone_repository.clone();
    let init_repository = actions.init_repository.clone();
    let open_folder = actions.choose_folder.clone();
    return view()
        .flex_1()
        .min_w_0()
        .h_full()
        .flex_col()
        .bg(move || design::content())
        .child(
            view()
                .w_full()
                .h(px(metrics::DIFFBAR_H))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .px(px(metrics::WINDOW_PAD))
                .border_b_1()
                .border_color(move || design::line())
                .child(segment(language, form, set_form.clone()))
                .child(view().flex_1())
                .child(
                    text(move || match form.get() {
                        RepoForm::Open => "git open".to_string(),
                        RepoForm::Clone => "git clone".to_string(),
                        RepoForm::Init => "git init".to_string(),
                    })
                    .text_size(px(10.5))
                    .font_family("SF Mono")
                    .text_color(move || design::text3()),
                ),
        )
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .pt(px(28.0))
                .px(px(40.0))
                .child(scroll().w_full().h_full().child(
                    view()
                        .w_full()
                        .max_w(px(560.0))
                        .flex_col()
                        .gap(px(6.0))
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
                            .text_size(px(18.0))
                            .font_weight(FontWeight(700))
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
                            .text_size(px(11.0))
                            .text_color(move || design::text3()),
                        )
                        .child(
                            view()
                                .w_full()
                                .mt(px(16.0))
                                .flex_col()
                                .gap(px(9.0))
                                .child(dynamic(move || -> AnyElement {
                                    match form.get() {
                                        RepoForm::Clone => clone_form(
                                            language,
                                            clone_url,
                                            clone_destination,
                                            error,
                                            busy,
                                            clone_repository.clone(),
                                            choose_folder.clone(),
                                        )
                                        .into_element(),
                                        RepoForm::Init => init_form(
                                            language,
                                            init_path,
                                            error,
                                            init_repository.clone(),
                                        )
                                        .into_element(),
                                        RepoForm::Open => open_form(
                                            language,
                                            error,
                                            open_folder.clone(),
                                        )
                                        .into_element(),
                                    }
                                })),
                        ),
                )),
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
        .h(px(26.0))
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

fn label(language: Signal<SharedString>, key: &'static str) -> impl IntoElement {
    return text(move || i18n::t(language.get().as_str(), key))
        .text_size(px(11.0))
        .font_weight(FontWeight(600))
        .text_color(move || design::text2());
}

fn field_row(value: Signal<String>, placeholder: &'static str) -> impl IntoElement {
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
            nsh_base::input()
                .value(value)
                .on_change(move |text: &str| value.set(text.to_string()))
                .placeholder(placeholder)
                .text_size(px(12.0))
                .flex_1(),
        );
}

fn open_form(
    language: Signal<SharedString>,
    error: Signal<Option<String>>,
    choose_folder: std::sync::Arc<dyn Fn()>,
) -> impl IntoElement {
    return view()
        .w_full()
        .flex_col()
        .gap(px(10.0))
        .child(
            text(move || i18n::t(language.get().as_str(), "repoManager.openHint"))
                .text_size(px(11.0))
                .line_height(px(16.0))
                .text_color(move || design::text2()),
        )
        .child(
            view()
                .flex_row()
                .gap(px(8.0))
                .child(widgets::primary_button(
                    move || i18n::t(language.get().as_str(), "welcome.choose"),
                    {
                        let choose_folder = choose_folder.clone();
                        move || choose_folder()
                    },
                ))
                .child(widgets::button(
                    move || i18n::t(language.get().as_str(), "welcome.openFolder"),
                    {
                        let choose_folder = choose_folder.clone();
                        move || choose_folder()
                    },
                )),
        )
        .child(error_line(error));
}

fn clone_form(
    language: Signal<SharedString>,
    clone_url: Signal<String>,
    clone_destination: Signal<String>,
    error: Signal<Option<String>>,
    busy: Signal<bool>,
    clone_repository: std::sync::Arc<dyn Fn()>,
    choose_folder: std::sync::Arc<dyn Fn()>,
) -> impl IntoElement {
    return view()
        .w_full()
        .flex_col()
        .gap(px(9.0))
        .child(label(language, "welcome.url"))
        .child(field_row(clone_url, "https://github.com/owner/repo.git"))
        .child(label(language, "welcome.destination"))
        .child(
            view()
                .flex_row()
                .gap(px(8.0))
                .child(view().flex_1().min_w_0().child(field_row(clone_destination, "~/Developer/repo")))
                .child(widgets::button(
                    move || i18n::t(language.get().as_str(), "welcome.choose"),
                    {
                        let choose_folder = choose_folder.clone();
                        move || choose_folder()
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
                        if busy.get() {
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

fn init_form(
    language: Signal<SharedString>,
    init_path: Signal<String>,
    error: Signal<Option<String>>,
    init_repository: std::sync::Arc<dyn Fn()>,
) -> impl IntoElement {
    return view()
        .w_full()
        .flex_col()
        .gap(px(9.0))
        .child(label(language, "repoManager.initPath"))
        .child(field_row(init_path, "~/Developer/new-repo"))
        .child(
            view().flex_row().items_center().gap(px(8.0)).child(widgets::primary_button(
                move || i18n::t(language.get().as_str(), "repoManager.initAction"),
                {
                    let init_repository = init_repository.clone();
                    move || init_repository()
                },
            )),
        )
        .child(error_line(error));
}

fn error_line(error: Signal<Option<String>>) -> impl IntoElement {
    return view()
        .w_full()
        .h(move || if error.get().is_some() { px(18.0) } else { px(0.0) })
        .items_center()
        .child(
            text(move || error.get().unwrap_or_default())
                .text_size(px(10.5))
                .text_color(move || design::danger()),
        );
}

fn short_name(path: &str) -> String {
    return path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_string();
}
