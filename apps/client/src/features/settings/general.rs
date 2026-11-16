//! 设置页的 General 分区（设计板 23）：外观、语言、日期、文件树、启动行为、
//! 身份与更新方式。

use super::view::{launch_choice, option_chip, section_block, toggle_row};
use crate::brand::ThemeMode;
use crate::design::{self, metrics};
use crate::i18n;
use nsh_core::prelude::*;
use std::sync::Arc;

/// 日期格式的四种预设。
const DATE_STYLES: [crate::i18n::DateStyle; 4] = [
    crate::i18n::DateStyle::Relative,
    crate::i18n::DateStyle::Absolute,
    crate::i18n::DateStyle::System,
    crate::i18n::DateStyle::Custom,
];

impl crate::i18n::DateStyle {
    /// 设置页上的文案键。
    fn key(self) -> &'static str {
        return match self {
            crate::i18n::DateStyle::Relative => "settings.dateRelative",
            crate::i18n::DateStyle::Absolute => "settings.dateAbsolute",
            crate::i18n::DateStyle::System => "settings.dateSystem",
            crate::i18n::DateStyle::Custom => "settings.dateCustom",
        };
    }
}

/// 更新检查的三种方式（设计板 23）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum UpdateMode {
    Auto,
    Manual,
    Never,
}

impl UpdateMode {
    fn id(self) -> &'static str {
        return match self {
            UpdateMode::Auto => "auto",
            UpdateMode::Manual => "manual",
            UpdateMode::Never => "never",
        };
    }

    fn key(self) -> &'static str {
        return match self {
            UpdateMode::Auto => "settings.updatesAuto",
            UpdateMode::Manual => "settings.updatesManual",
            UpdateMode::Never => "settings.updatesNever",
        };
    }
}

const UPDATE_MODES: [UpdateMode; 3] = [UpdateMode::Auto, UpdateMode::Manual, UpdateMode::Never];

fn theme_key(mode: ThemeMode) -> &'static str {
    return match mode {
        ThemeMode::System => "theme.system",
        ThemeMode::Light => "theme.light",
        ThemeMode::Dark => "theme.dark",
    };
}

#[allow(clippy::too_many_arguments)]
pub(super) fn general(
    language: Signal<SharedString>,
    theme_mode: Signal<ThemeMode>,
    settings_state: Signal<store::Settings>,
    set_theme: Arc<dyn Fn(ThemeMode)>,
    toggle_language: Arc<dyn Fn()>,
    set_file_tree: Arc<dyn Fn(bool)>,
    set_update_mode: Arc<dyn Fn(String)>,
    set_tree_flag: Arc<dyn Fn(&'static str, bool)>,
    set_launch: Arc<dyn Fn(bool)>,
    last_session: Vec<String>,
    set_date_style: Arc<dyn Fn(crate::i18n::DateStyle)>,
    set_date_pattern: Arc<dyn Fn(String)>,
    identity_line: String,
    open_identity: Arc<dyn Fn()>,
    clear_identity: Arc<dyn Fn()>,
) -> impl IntoElement {
    let mut theme_rows: Vec<AnyElement> = Vec::new();
    for mode in [ThemeMode::System, ThemeMode::Light, ThemeMode::Dark] {
        let set_theme = set_theme.clone();
        theme_rows.push(
            option_chip(
                move || i18n::t(language.get().as_str(), theme_key(mode)),
                theme_mode.get_untracked() == mode,
                move || set_theme(mode),
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
                    .text_size(px(crate::ui::BODY))
                    .text_color(move || design::text()),
                )
                .child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "settings.switchLanguage"),
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    {
                        let toggle_language = toggle_language.clone();
                        move || toggle_language()
                    },
                ))
                .into_element(),
        ))
        // 日期格式：四种预设 + 自定义 pattern + 实时预览（设计板 23）。
        .child(section_block(
            language,
            "settings.dates",
            "settings.datesHint",
            view()
                .w_full()
                .flex_col()
                .gap(px(6.0))
                .child(
                    view().w_full().flex_row().gap(px(2.0)).child(
                        DATE_STYLES
                            .iter()
                            .map(|style| {
                                let style = *style;
                                let active =
                                    settings_state.get_untracked().date_format == style.id();
                                let set_date_style = set_date_style.clone();
                                let language = language;
                                return option_chip(
                                    move || i18n::t(language.get().as_str(), style.key()),
                                    active,
                                    move || set_date_style(style),
                                )
                                .into_element();
                            })
                            .collect::<Vec<AnyElement>>(),
                    ),
                )
                .child(
                    view()
                        .w_full()
                        .h(px(metrics::FIELD_H))
                        .px(px(8.0))
                        .items_center()
                        .rounded(px(metrics::RADIUS_CONTROL))
                        .border_1()
                        .border_color(move || design::field_border())
                        .bg(move || design::field())
                        .child(
                            nsh_base::input()
                                .value({
                                    let settings_state = settings_state;
                                    move || settings_state.get().date_pattern
                                })
                                .on_change({
                                    let set_date_pattern = set_date_pattern.clone();
                                    move |text: &str| set_date_pattern(text.to_string())
                                })
                                .placeholder(move || {
                                    i18n::t(language.get().as_str(), "settings.datePatternHint")
                                })
                                .text_size(px(crate::ui::CODE))
                                .flex_1(),
                        ),
                )
                .child(
                    text(move || {
                        // 预览用当前设置格式化「现在」，用户改 pattern 立刻能看到效果。
                        let now = store::now_unix();
                        i18n::with_arguments(
                            language.get().as_str(),
                            "settings.datePreview",
                            &[(
                                "value",
                                crate::dates::format(language.get().as_str(), now, now),
                            )],
                        )
                    })
                    .text_size(px(crate::ui::META))
                    .line_height(px(crate::ui::SECONDARY_LINE))
                    .text_color(move || design::text3()),
                )
                .into_element(),
        ))
        .child(section_block(
            language,
            "settings.fileTree",
            "settings.fileTreeHint",
            view()
                .w_full()
                .flex_col()
                .gap(px(2.0))
                .child(toggle_row(
                    language,
                    "settings.fileTreeToggle",
                    Arc::new(move || settings_state.get().file_tree),
                    move || set_file_tree(!settings_state.get_untracked().file_tree),
                ))
                .child(toggle_row(
                    language,
                    "settings.mergeEmptyDirs",
                    Arc::new(move || settings_state.get().merge_empty_dirs),
                    {
                        let set_tree_flag = set_tree_flag.clone();
                        move || {
                            set_tree_flag(
                                "merge_empty_dirs",
                                !settings_state.get_untracked().merge_empty_dirs,
                            )
                        }
                    },
                ))
                .child(toggle_row(
                    language,
                    "settings.compactSingleChild",
                    Arc::new(move || settings_state.get().compact_single_child),
                    {
                        let set_tree_flag = set_tree_flag.clone();
                        move || {
                            set_tree_flag(
                                "compact_single_child",
                                !settings_state.get_untracked().compact_single_child,
                            )
                        }
                    },
                ))
                .into_element(),
        ))
        .child(section_block(
            language,
            "settings.onLaunch",
            "settings.onLaunchHint",
            view()
                .w_full()
                .flex_col()
                .gap(px(4.0))
                .child(
                    view()
                        .w_full()
                        .flex_row()
                        .items_center()
                        .gap(px(2.0))
                        .child(launch_choice(
                            language,
                            "settings.restoreSession",
                            settings_state.get_untracked().restore_session,
                            {
                                let set_launch = set_launch.clone();
                                move || set_launch(true)
                            },
                        ))
                        .child(launch_choice(
                            language,
                            "settings.showWelcome",
                            !settings_state.get_untracked().restore_session,
                            {
                                let set_launch = set_launch.clone();
                                move || set_launch(false)
                            },
                        )),
                )
                // Last session：明确列出启动时会被恢复的仓库。
                .child(view().w_full().when(!last_session.is_empty(), |this| {
                    let rows: Vec<AnyElement> = last_session
                        .iter()
                        .map(|path| {
                            let label = path.clone();
                            return view()
                                .w_full()
                                .h(px(20.0))
                                .flex_row()
                                .items_center()
                                .gap(px(6.0))
                                .child(crate::ui::icon::icon_view(
                                    "folder",
                                    crate::ui::icon::IconSize::Compact,
                                    design::text3,
                                ))
                                .child(
                                    text(label)
                                        .flex_1()
                                        .text_size(px(crate::ui::CODE))
                                        .line_height(px(crate::ui::CODE_LINE))
                                        .text_color(move || design::text2())
                                        .text_overflow(nsh_core::EllipsizeMode::Middle),
                                )
                                .into_element();
                        })
                        .collect();
                    let count = rows.len().to_string();
                    this.child(
                        view()
                            .w_full()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                text(move || {
                                    i18n::with_arguments(
                                        language.get().as_str(),
                                        "settings.lastSession",
                                        &[("count", count.clone())],
                                    )
                                })
                                .text_size(px(crate::ui::META))
                                .line_height(px(crate::ui::SECONDARY_LINE))
                                .text_color(move || design::text3()),
                            )
                            .child(rows),
                    )
                }))
                .into_element(),
        ))
        .child(section_block(
            language,
            "settings.identity",
            "settings.identityHint",
            view()
                .w_full()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    text(identity_line.clone())
                        .flex_1()
                        .text_size(px(crate::ui::CODE))
                        .line_height(px(15.0))
                        .text_color(move || design::text2()),
                )
                .child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "settings.editIdentity"),
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    {
                        let open_form = open_identity.clone();
                        move || open_form()
                    },
                ))
                .child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "settings.useGlobalIdentity"),
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    {
                        let clear = clear_identity.clone();
                        move || clear()
                    },
                ))
                .into_element(),
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
                        .h(px(22.0))
                        .flex_row()
                        .items_center()
                        .gap(px(2.0))
                        .p(px(2.0))
                        .rounded(px(6.0))
                        .border_1()
                        .border_color(move || design::line_soft())
                        .bg(move || design::panel2())
                        .child(
                            UPDATE_MODES
                                .iter()
                                .map(|mode| {
                                    let mode = *mode;
                                    let active =
                                        settings_state.get_untracked().update_mode == mode.id();
                                    let set_update_mode = set_update_mode.clone();
                                    let language = language;
                                    let mode_id = mode.id().to_string();
                                    return option_chip(
                                        move || i18n::t(language.get().as_str(), mode.key()),
                                        active,
                                        move || set_update_mode(mode_id.clone()),
                                    )
                                    .into_element();
                                })
                                .collect::<Vec<AnyElement>>(),
                        ),
                )
                .child(
                    text(move || i18n::t(language.get().as_str(), "settings.updaterUnavailable"))
                        .text_size(px(crate::ui::META))
                        .text_color(move || design::text3()),
                )
                .into_element(),
        ));
}
