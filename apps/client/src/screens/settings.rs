//! 设置页（设计板 23–27）：左侧导航 + 右侧分区设置。
//!
//! 已接入的项：外观（跟随系统/浅色/深色）、语言（英文/中文）、提交身份（读
//! `git config`，只读展示）、文件树分组、启动行为、版本信息。其余分区（Git、快捷键、
//! 凭据）按设计稿呈现标题与说明，内容明确标注「尚未实现」，不做假控件。

use crate::brand::{self, ThemeMode};
use crate::design::{self, metrics};
use crate::i18n;
use crate::widgets;
use crate::Actions;
use nsh_core::prelude::*;

/// 左侧导航宽度（设计板 `.settings-side`）。
const SIDE_W: f32 = 210.0;

/// 设置页的分区。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Section {
    General,
    Git,
    Shortcuts,
    About,
    Credentials,
}

impl Section {
    const ALL: [Section; 5] = [
        Section::General,
        Section::Git,
        Section::Shortcuts,
        Section::About,
        Section::Credentials,
    ];

    fn key(self) -> &'static str {
        return match self {
            Section::General => "settings.general",
            Section::Git => "settings.git",
            Section::Shortcuts => "settings.shortcuts",
            Section::About => "settings.about",
            Section::Credentials => "settings.credentials",
        };
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn settings(
    language: Signal<SharedString>,
    theme_mode: Signal<ThemeMode>,
    settings_state: Signal<store::Settings>,
    section: Signal<Section>,
    identity: (String, String),
    actions: Actions,
) -> impl IntoElement {
    let set_section = std::sync::Arc::new(move |next: Section| section.set(next));
    let done = actions.go_welcome.clone();
    return view()
        .w_full()
        .h_full()
        .flex_col()
        .bg(move || design::content())
        .child(titlebar(language, actions.clone()))
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .flex_row()
                .child(side(language, section, set_section))
                .child(main(
                    language,
                    theme_mode,
                    settings_state,
                    section,
                    identity,
                    actions,
                )),
        )
        .child(
            view()
                .w_full()
                .h(px(38.0))
                .px(px(16.0))
                .flex_row()
                .items_center()
                .gap(px(10.0))
                .border_t_1()
                .border_color(move || design::line())
                .bg(move || design::bar())
                .child(
                    text(move || i18n::t(language.get().as_str(), "settings.savedAutomatically"))
                        .text_size(px(10.5))
                        .text_color(move || design::text3()),
                )
                .child(view().flex_1())
                .child(widgets::button(
                    move || i18n::t(language.get().as_str(), "common.done"),
                    {
                        let go_welcome = done.clone();
                        move || go_welcome()
                    },
                )),
        );
}

fn titlebar(language: Signal<SharedString>, actions: Actions) -> impl IntoElement {
    let go_welcome = actions.go_welcome.clone();
    return view()
        .w_full()
        .h(px(metrics::TITLEBAR_PLAIN_H))
        .px(px(metrics::WINDOW_PAD))
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .bg(move || design::titlebar())
        .border_b_1()
        .border_color(move || design::line())
        .window_drag(true)
        .child(
            text(move || i18n::t(language.get().as_str(), "settings.title"))
                .text_size(px(12.5))
                .font_weight(FontWeight(600))
                .text_color(move || design::text()),
        )
        .child(view().flex_1())
        .child(widgets::icon_button(
            "chev_left",
            13.0,
            design::text2,
            move || go_welcome(),
        ));
}

fn side(
    language: Signal<SharedString>,
    section: Signal<Section>,
    set_section: std::sync::Arc<dyn Fn(Section)>,
) -> impl IntoElement {
    let mut rows: Vec<AnyElement> = Vec::new();
    for item in Section::ALL {
        let set_section = set_section.clone();
        rows.push(
            view()
                .w_full()
                .h(px(26.0))
                .px(px(8.0))
                .flex_row()
                .items_center()
                .rounded(px(metrics::RADIUS_CONTROL))
                .bg(move || {
                    if section.get() == item {
                        design::selected()
                    } else {
                        AdaptiveColor::TRANSPARENT
                    }
                })
                .hover(|style| style.bg(move || design::hover()))
                .on_press(move || set_section(item))
                .child(
                    text(move || i18n::t(language.get().as_str(), item.key()))
                        .text_size(px(12.0))
                        .text_color(move || {
                            if section.get() == item {
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
                .pt(px(12.0))
                .pb(px(8.0))
                .px(px(12.0))
                .flex_col()
                .gap(px(2.0))
                .child(
                    text(move || i18n::t(language.get().as_str(), "settings.title"))
                        .text_size(px(15.0))
                        .font_weight(FontWeight(700))
                        .text_color(move || design::text()),
                )
                .child(
                    text(brand::version_line())
                        .text_size(px(9.5))
                        .text_color(move || design::text3()),
                ),
        )
        .child(view().w_full().px(px(6.0)).child(rows));
}

#[allow(clippy::too_many_arguments)]
fn main(
    language: Signal<SharedString>,
    theme_mode: Signal<ThemeMode>,
    settings_state: Signal<store::Settings>,
    section: Signal<Section>,
    identity: (String, String),
    actions: Actions,
) -> impl IntoElement {
    let (name, email) = identity;
    let set_theme = actions.set_theme.clone();
    let set_language = actions.toggle_language.clone();
    let set_file_tree = actions.set_file_tree.clone();
    let set_restore = actions.set_restore_session.clone();
    return view()
        .flex_1()
        .min_w_0()
        .h_full()
        .child(scroll().w_full().h_full().child(
            view()
                .w_full()
                .max_w(px(640.0))
                .pt(px(24.0))
                .pb(px(24.0))
                .px(px(28.0))
                .flex_col()
                .gap(px(18.0))
                .child(
                    text(move || i18n::t(language.get().as_str(), section.get().key()))
                        .text_size(px(17.0))
                        .font_weight(FontWeight(700))
                        .text_color(move || design::text()),
                )
                .child(dynamic(move || -> AnyElement {
                    match section.get() {
                        Section::General => general(
                            language,
                            theme_mode,
                            settings_state,
                            set_theme.clone(),
                            set_language.clone(),
                            set_file_tree.clone(),
                            set_restore.clone(),
                        )
                        .into_element(),
                        Section::About => about(language, name.clone(), email.clone()).into_element(),
                        other => placeholder(language, other).into_element(),
                    }
                })),
        ));
}

#[allow(clippy::too_many_arguments)]
fn general(
    language: Signal<SharedString>,
    theme_mode: Signal<ThemeMode>,
    settings_state: Signal<store::Settings>,
    set_theme: std::sync::Arc<dyn Fn(ThemeMode)>,
    toggle_language: std::sync::Arc<dyn Fn()>,
    set_file_tree: std::sync::Arc<dyn Fn(bool)>,
    set_restore_session: std::sync::Arc<dyn Fn(bool)>,
) -> impl IntoElement {
    let mut theme_rows: Vec<AnyElement> = Vec::new();
    for mode in [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark] {
        let set_theme = set_theme.clone();
        theme_rows.push(
            view()
                .h(px(24.0))
                .px(px(10.0))
                .items_center()
                .rounded(px(5.0))
                .bg(move || {
                    if theme_mode.get() == mode {
                        design::content()
                    } else {
                        AdaptiveColor::TRANSPARENT
                    }
                })
                .on_press(move || set_theme(mode))
                .child(
                    text(move || i18n::t(language.get().as_str(), theme_key(mode)))
                        .text_size(px(11.5))
                        .text_color(move || design::text()),
                )
                .into_element(),
        );
    }
    return view()
        .w_full()
        .flex_col()
        .gap(px(18.0))
        .child(section_block(
            language,
            "settings.appearance",
            "settings.appearanceHint",
            view()
                .h(px(30.0))
                .flex_row()
                .items_center()
                .gap(px(2.0))
                .p(px(2.0))
                .rounded(px(7.0))
                .border_1()
                .border_color(move || design::line_soft())
                .bg(move || design::panel2())
                .child(theme_rows)
                .into_element(),
        ))
        .child(section_block(
            language,
            "settings.language",
            "settings.languageHint",
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    text(move || {
                        if language.get().as_str() == i18n::ZH_HANS_LANG {
                            "中文".to_string()
                        } else {
                            "English".to_string()
                        }
                    })
                    .text_size(px(11.5))
                    .text_color(move || design::text()),
                )
                .child(widgets::small_button(
                    move || i18n::t(language.get().as_str(), "settings.switchLanguage"),
                    {
                        let toggle_language = toggle_language.clone();
                        move || toggle_language()
                    },
                ))
                .into_element(),
        ))
        .child(section_block(
            language,
            "settings.fileTree",
            "settings.fileTreeHint",
            toggle_row(
                language,
                "settings.fileTreeToggle",
                std::sync::Arc::new(move || settings_state.get().file_tree),
                move || set_file_tree(!settings_state.get_untracked().file_tree),
            ),
        ))
        .child(section_block(
            language,
            "settings.onLaunch",
            "settings.onLaunchHint",
            toggle_row(
                language,
                "settings.restoreSession",
                std::sync::Arc::new(move || settings_state.get().restore_session),
                move || set_restore_session(!settings_state.get_untracked().restore_session),
            ),
        ))
        .child(section_block(
            language,
            "settings.updates",
            "settings.updatesHint",
            view()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    view()
                        .h(px(metrics::BUTTON_SMALL_H))
                        .px(px(8.0))
                        .items_center()
                        .rounded(px(5.0))
                        .border_1()
                        .border_color(move || design::btn_border())
                        .bg(move || design::btn())
                        .opacity(0.45)
                        .child(
                            text(move || {
                                i18n::t(language.get().as_str(), "settings.checkNow")
                            })
                            .text_size(px(11.0))
                            .text_color(move || design::text()),
                        ),
                )
                .child(
                    text(move || i18n::t(language.get().as_str(), "settings.notImplemented"))
                        .text_size(px(9.5))
                        .text_color(move || design::text3()),
                )
                .into_element(),
        ));
}

fn theme_key(mode: ThemeMode) -> &'static str {
    return match mode {
        ThemeMode::System => "theme.system",
        ThemeMode::Light => "theme.light",
        ThemeMode::Dark => "theme.dark",
    };
}

fn section_block(
    language: Signal<SharedString>,
    title_key: &'static str,
    hint_key: &'static str,
    body: AnyElement,
) -> impl IntoElement {
    return view()
        .w_full()
        .flex_col()
        .gap(px(6.0))
        .child(
            text(move || i18n::t(language.get().as_str(), title_key))
                .text_size(px(12.0))
                .font_weight(FontWeight(600))
                .text_color(move || design::text()),
        )
        .child(
            text(move || i18n::t(language.get().as_str(), hint_key))
                .text_size(px(10.5))
                .line_height(px(15.0))
                .text_color(move || design::text3()),
        )
        .child(body);
}

fn toggle_row(
    language: Signal<SharedString>,
    label_key: &'static str,
    checked: std::sync::Arc<dyn Fn() -> bool>,
    on_press: impl Fn() + 'static,
) -> AnyElement {
    let checked_for_bg = checked.clone();
    let checked_for_text = checked.clone();
    return view()
        .w_full()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(
            view()
                .w(px(14.0))
                .h(px(14.0))
                .items_center()
                .justify_center()
                .rounded(px(3.0))
                .border_1()
                .border_color(move || design::line_strong())
                .bg(move || {
                    if checked_for_bg() {
                        design::accent()
                    } else {
                        AdaptiveColor::TRANSPARENT
                    }
                })
                .on_press(on_press)
                .child(
                    text(move || {
                        if checked_for_text() {
                            "✓".to_string()
                        } else {
                            String::new()
                        }
                    })
                    .text_size(px(9.0))
                    .text_color(move || design::content()),
                ),
        )
        .child(
            text(move || i18n::t(language.get().as_str(), label_key))
                .text_size(px(11.5))
                .text_color(move || design::text()),
        )
        .into_element();
}

fn about(
    language: Signal<SharedString>,
    name: String,
    email: String,
) -> impl IntoElement {
    return view()
        .w_full()
        .flex_col()
        .gap(px(8.0))
        .child(
            text(brand::version_line())
                .text_size(px(11.5))
                .font_family("SF Mono")
                .text_color(move || design::text2()),
        )
        .child(
            text(move || i18n::t(language.get().as_str(), "settings.aboutHint"))
                .text_size(px(10.5))
                .line_height(px(15.0))
                .text_color(move || design::text3()),
        )
        .child(
            text(format!("{} <{}>", name, email))
                .text_size(px(11.0))
                .font_family("SF Mono")
                .text_color(move || design::text2()),
        );
}

fn placeholder(language: Signal<SharedString>, section: Section) -> impl IntoElement {
    return view()
        .w_full()
        .flex_col()
        .gap(px(6.0))
        .child(
            text(move || i18n::t(language.get().as_str(), section.key()))
                .text_size(px(11.5))
                .font_weight(FontWeight(600))
                .text_color(move || design::text2()),
        )
        .child(
            text(move || i18n::t(language.get().as_str(), "settings.notImplemented"))
                .text_size(px(10.5))
                .text_color(move || design::text3()),
        );
}
