//! 欢迎页（设计板 01 / 02）：左侧仓库栏 + 右侧 hero 与克隆表单。
//!
//! 左侧列表按“今天 / 最近 7 天”分组，分组依据是上次打开时间而不是设计稿里的
//! 固定文案；时间显示与语言一致。

use crate::design::{self, metrics};
use crate::i18n;
use crate::widgets;
use crate::Actions;
use store::RecentRepo;
use nsh_base::input;
use nsh_core::prelude::*;

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
    clone_error: Signal<Option<String>>,
    cloning: Signal<bool>,
    actions: Actions,
) -> impl IntoElement {
    let open_path = actions.open_path.clone();
    let choose_folder = actions.choose_folder.clone();
    let clone_repository = actions.clone_repository.clone();
    return view()
        .w_full()
        .h_full()
        .flex_row()
        .bg(move || design::content())
        .child(sidebar(language, recent, query, actions))
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
                            .child(clone_section(
                                language,
                                clone_url,
                                clone_destination,
                                clone_error,
                                cloning,
                                choose_folder,
                                clone_repository,
                            ))
                            .child(tip(language))
                            .child(view().h(px(24.0)).w_full()),
                    ),
                )
                .child(
                    view()
                        .absolute()
                        .inset(px(0.0))
                        .z_index(10)
                        .items_center()
                        .justify_center()
                        .bg(move || {
                            if cloning.get() {
                                AdaptiveColor::from(rgb(0x000000)).opacity(0.18)
                            } else {
                                AdaptiveColor::TRANSPARENT
                            }
                        })
                        .on_press({
                            let open_path = open_path.clone();
                            move || {
                                // 遮罩按下不做任何事；保留占位以吞掉事件。
                                let _ = &open_path;
                            }
                        }),
                ),
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
                .items_start()
                .gap(px(8.0))
                .child(
                    view()
                        .flex_1()
                        .min_w_0()
                        .flex_col()
                        .child(
                            text(move || i18n::t(language.get().as_str(), "welcome.repositories"))
                                .text_size(px(metrics::FONT_WELCOME_SIDE_TITLE))
                                .font_weight(FontWeight(700))
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
                .child(widgets::kbd("⌘F")),
        )
        .child(
            view()
                .flex_1()
                .min_h_0()
                .px(px(metrics::SIDE_SCROLL_PAD.1))
                .child(scroll().w_full().h_full().child(
                    view()
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
                            open_path,
                        )),
                )),
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
                    .h(move || if visible { px(metrics::WELCOME_REPO_H) } else { px(0.0) })
                    .flex_row()
                    .items_center()
                    .gap(px(9.0))
                    .px(px(8.0))
                    .rounded(px(7.0))
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
                            .child(
                                text(move || last_segment(&repo.get().path))
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight(600))
                                    .text_color(move || design::text())
                                    .text_overflow(nsh_core::EllipsizeMode::Tail),
                            )
                            .child(
                                text(move || home_path(&repo.get().path))
                                    .text_size(px(9.5))
                                    .font_family("SF Mono")
                                    .text_color(move || design::text3())
                                    .text_overflow(nsh_core::EllipsizeMode::Tail),
                            ),
                    )
                    .child(
                        text(move || {
                            i18n::relative_time(
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
                .child(widgets::icon("branch", 30.0, || design::token("primary-foreground"))),
        )
        .child(
            view()
                .min_w_0()
                .flex_col()
                .child(
                    text(move || i18n::t(language.get().as_str(), "app.name"))
                        .text_size(px(metrics::FONT_WELCOME_TITLE))
                        .font_weight(FontWeight(700))
                        .text_color(move || design::text()),
                )
                .child(
                    text(move || i18n::t(language.get().as_str(), "app.tagline"))
                        .mt(px(7.0))
                        .text_size(px(12.5))
                        .text_color(move || design::text2()),
                ),
        );
}

fn clone_section(
    language: Signal<SharedString>,
    clone_url: Signal<String>,
    clone_destination: Signal<String>,
    clone_error: Signal<Option<String>>,
    cloning: Signal<bool>,
    choose_folder: std::sync::Arc<dyn Fn()>,
    clone_repository: std::sync::Arc<dyn Fn()>,
) -> impl IntoElement {
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
                .items_start()
                .gap(px(12.0))
                .child(
                    view()
                        .flex_1()
                        .min_w_0()
                        .flex_col()
                        .child(
                            text(move || {
                                i18n::t(language.get().as_str(), "welcome.cloneTitle")
                            })
                            .text_size(px(13.0))
                            .font_weight(FontWeight(600))
                            .text_color(move || design::text()),
                        )
                        .child(
                            text(move || i18n::t(language.get().as_str(), "welcome.cloneSub"))
                                .mt(px(3.0))
                                .text_size(px(10.5))
                                .text_color(move || design::text3()),
                        ),
                )
                .child(protocol_toggle(language)),
        )
        .child(
            view()
                .w_full()
                .mt(px(13.0))
                .flex_col()
                .gap(px(9.0))
                .child(
                    view()
                        .w_full()
                        .flex_col()
                        .gap(px(5.0))
                        .child(
                            text(move || i18n::t(language.get().as_str(), "welcome.url"))
                                .text_size(px(11.0))
                                .font_weight(FontWeight(600))
                                .text_color(move || design::text2()),
                        )
                        .child(
                            view()
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
                                        .value(clone_url)
                                        .on_change(move |value: &str| clone_url.set(value.to_string()))
                                        .placeholder("https://github.com/owner/repo.git")
                                        .text_size(px(12.0))
                                        .flex_1(),
                                ),
                        ),
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
                                .child(
                                    text(move || {
                                        i18n::t(language.get().as_str(), "welcome.destination")
                                    })
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight(600))
                                    .text_color(move || design::text2()),
                                )
                                .child(
                                    view()
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
                                                .value(clone_destination)
                                                .on_change(move |value: &str| {
                                                    clone_destination.set(value.to_string())
                                                })
                                                .placeholder("~/Developer/repo")
                                                .text_size(px(12.0))
                                                .flex_1(),
                                        )
                                        .child(widgets::kbd("⌘V")),
                                ),
                        )
                        .child(widgets::button(
                            move || i18n::t(language.get().as_str(), "welcome.choose"),
                            {
                                let choose_folder = choose_folder.clone();
                                move || choose_folder()
                            },
                        )),
                ),
        )
        .child(
            view()
                .w_full()
                .mt(px(10.0))
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
        .child(
            view()
                .w_full()
                .h(move || if clone_error.get().is_some() { px(18.0) } else { px(0.0) })
                .items_center()
                .child(
                    text(move || clone_error.get().unwrap_or_default())
                        .text_size(px(10.5))
                        .text_color(move || design::danger()),
                ),
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
                .rounded(px(4.0))
                .bg(move || design::content())
                .child(
                    text("HTTPS")
                        .text_size(px(9.5))
                        .text_color(move || design::text()),
                ),
        )
        .child(
            view().h(px(16.0)).px(px(6.0)).items_center().child(
                text("SSH")
                    .text_size(px(9.5))
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
