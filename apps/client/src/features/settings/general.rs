//! General 分区（设计板 23）：外观、语言、日期、提交身份、文件树、启动行为与更新。

use crate::brand::{self, ThemeMode};
use crate::design;
use crate::i18n;
use crate::ui;
use crate::ui::select::SelectOption;
use nsh_core::prelude::*;
use std::sync::Arc;

const DATE_STYLES: [crate::i18n::DateStyle; 4] = [
    crate::i18n::DateStyle::Relative,
    crate::i18n::DateStyle::Absolute,
    crate::i18n::DateStyle::System,
    crate::i18n::DateStyle::Custom,
];

impl crate::i18n::DateStyle {
    fn key(self) -> &'static str {
        return match self {
            crate::i18n::DateStyle::Relative => "settings.dateRelative",
            crate::i18n::DateStyle::Absolute => "settings.dateAbsolute",
            crate::i18n::DateStyle::System => "settings.dateSystem",
            crate::i18n::DateStyle::Custom => "settings.dateCustom",
        };
    }
}

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
        ThemeMode::System => "settings.themeAuto",
        ThemeMode::Light => "settings.themeLight",
        ThemeMode::Dark => "settings.themeDark",
    };
}

#[allow(clippy::too_many_arguments)]
pub(super) fn general(
    language: Signal<SharedString>,
    theme_mode: Signal<ThemeMode>,
    settings_state: Signal<store::Settings>,
    identity_name: Signal<String>,
    identity_email: Signal<String>,
    identity_use_global: Signal<bool>,
    date_pattern: Signal<String>,
    has_repo: bool,
    set_theme: Arc<dyn Fn(ThemeMode)>,
    set_language: Arc<dyn Fn(&'static str)>,
    set_update_mode: Arc<dyn Fn(String)>,
    set_launch_restore: Arc<dyn Fn(bool)>,
    set_file_tree: Arc<dyn Fn(bool)>,
    set_tree_flag: Arc<dyn Fn(&'static str, bool)>,
    set_date_style: Arc<dyn Fn(crate::i18n::DateStyle)>,
    set_date_pattern: Arc<dyn Fn(String)>,
    save_identity: Arc<dyn Fn(String, String)>,
    clear_identity: Arc<dyn Fn()>,
    set_use_global: Arc<dyn Fn(bool)>,
    check_updates: Arc<dyn Fn()>,
) -> impl IntoElement {
    let settings = settings_state.get_untracked();
    let selected_theme = theme_mode.get_untracked().id();
    let theme_segments = [ThemeMode::Light, ThemeMode::Dark, ThemeMode::System]
        .into_iter()
        .map(|mode| ui::segmented::Segment {
            id: mode.id(),
            label: Box::new({
                let language = language;
                move || i18n::t(language.get().as_str(), theme_key(mode))
            }),
        })
        .collect::<Vec<ui::segmented::Segment>>();
    let language_selected = language.get_untracked().to_string();
    let language_options = vec![
        SelectOption::new("en", {
            let language = language;
            move || i18n::t(language.get().as_str(), "settings.languageEnglish")
        }),
        SelectOption::new("zh-Hans", {
            let language = language;
            move || i18n::t(language.get().as_str(), "settings.languageChinese")
        }),
    ];
    let date_options = DATE_STYLES
        .iter()
        .map(|style| {
            let style = *style;
            SelectOption::new(style.id(), {
                let language = language;
                move || i18n::t(language.get().as_str(), style.key())
            })
        })
        .collect::<Vec<SelectOption>>();
    let launch_options = vec![
        SelectOption::new("restore", {
            let language = language;
            move || i18n::t(language.get().as_str(), "settings.restoreSession")
        }),
        SelectOption::new("welcome", {
            let language = language;
            move || i18n::t(language.get().as_str(), "settings.showWelcome")
        }),
    ];
    let update_options = UPDATE_MODES
        .iter()
        .map(|mode| {
            let mode = *mode;
            SelectOption::new(mode.id(), {
                let language = language;
                move || i18n::t(language.get().as_str(), mode.key())
            })
        })
        .collect::<Vec<SelectOption>>();
    let launch_selected = if settings.restore_session {
        "restore"
    } else {
        "welcome"
    };
    let language_now = language;
    let preview = move || {
        let _ = date_pattern.get();
        i18n::with_arguments(
            language_now.get().as_str(),
            "settings.datePreview",
            &[(
                "value",
                crate::dates::format(
                    language_now.get().as_str(),
                    store::now_unix(),
                    store::now_unix(),
                ),
            )],
        )
    };
    let pattern_input = ui::text_input::text_input_bound_commit(
        move || date_pattern.get(),
        move || i18n::t(language.get().as_str(), "settings.datePatternHint"),
        false,
        false,
        {
            let date_pattern = date_pattern;
            move |text: String| {
                crate::dates::set_pattern(text.as_str());
                date_pattern.set(text);
            }
        },
        {
            let set_date_pattern = set_date_pattern.clone();
            let date_pattern = date_pattern;
            move || set_date_pattern(date_pattern.get_untracked())
        },
    );
    let name_input = ui::text_input::text_input_bound_commit(
        move || identity_name.get(),
        move || i18n::t(language.get().as_str(), "settings.namePlaceholder"),
        false,
        !has_repo || identity_use_global.get_untracked(),
        {
            let identity_name = identity_name;
            move |text: String| identity_name.set(text)
        },
        {
            let save = save_identity.clone();
            let identity_name = identity_name;
            let identity_email = identity_email;
            move || {
                save(
                    identity_name.get_untracked(),
                    identity_email.get_untracked(),
                )
            }
        },
    );
    let email_input = ui::text_input::text_input_bound_commit(
        move || identity_email.get(),
        move || i18n::t(language.get().as_str(), "settings.emailPlaceholder"),
        false,
        !has_repo || identity_use_global.get_untracked(),
        {
            let identity_email = identity_email;
            move |text: String| identity_email.set(text)
        },
        {
            let save = save_identity.clone();
            let identity_name = identity_name;
            let identity_email = identity_email;
            move || {
                save(
                    identity_name.get_untracked(),
                    identity_email.get_untracked(),
                )
            }
        },
    );
    let identity_toggle = ui::check::check_control(
        move || i18n::t(language.get().as_str(), "settings.useGlobalConfig"),
        identity_use_global.get_untracked(),
        !has_repo,
        {
            let set_use_global = set_use_global.clone();
            let clear = clear_identity.clone();
            move |next: bool| {
                if next {
                    clear();
                } else {
                    set_use_global(false);
                }
            }
        },
    );
    return view()
        .w_full()
        .flex_col()
        .child(ui::setting_row::settings_section(
            true,
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.updates")
            },
            Some(Box::new({
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.updatesHint")
            })),
            view()
                .w_full()
                .flex_col()
                .child(setting_row(
                    language,
                    "settings.checkForUpdates",
                    None,
                    ui::select::select(
                        update_options,
                        settings.update_mode.clone(),
                        Some(210.0),
                        {
                            let set_update_mode = set_update_mode.clone();
                            move |id| set_update_mode(id.to_string())
                        },
                    )
                    .into_element(),
                    true,
                ))
                .child(setting_row(
                    language,
                    "settings.status",
                    None,
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            text(move || {
                                format!(
                                    "{} · {}",
                                    brand::version_line(),
                                    i18n::t(language.get().as_str(), "settings.notChecked")
                                )
                            })
                            .text_size(px(ui::SECONDARY))
                            .text_color(move || design::text3()),
                        )
                        .child(ui::button::button(
                            move || i18n::t(language.get().as_str(), "settings.checkNow"),
                            ui::button::ButtonVariant::Secondary,
                            ui::button::ButtonSize::Small,
                            false,
                            {
                                let check = check_updates.clone();
                                move || check()
                            },
                        ))
                        .into_element(),
                    false,
                )),
        ))
        .child(ui::setting_row::settings_section(
            false,
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.appearance")
            },
            Some(Box::new({
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.appearanceHint")
            })),
            setting_row(
                language,
                "settings.theme",
                Some("settings.themeHint"),
                ui::segmented::segmented(theme_segments, selected_theme, {
                    let set_theme = set_theme.clone();
                    move |id| set_theme(ThemeMode::from_id(id))
                })
                .into_element(),
                false,
            ),
        ))
        .child(ui::setting_row::settings_section(
            false,
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.language")
            },
            Some(Box::new({
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.languageHint")
            })),
            setting_row(
                language,
                "settings.language",
                None,
                ui::select::select(language_options, language_selected, Some(210.0), {
                    let set_language = set_language.clone();
                    move |id| set_language(id)
                })
                .into_element(),
                false,
            ),
        ))
        .child(ui::setting_row::settings_section(
            false,
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.onLaunch")
            },
            Some(Box::new({
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.onLaunchHint")
            })),
            view()
                .w_full()
                .flex_col()
                .child(setting_row(
                    language,
                    "settings.launchBehavior",
                    None,
                    ui::select::select(launch_options, launch_selected, Some(210.0), {
                        let set_launch = set_launch_restore.clone();
                        move |id| set_launch(id == "restore")
                    })
                    .into_element(),
                    false,
                )),
        ))
        .child(ui::setting_row::settings_section(
            false,
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.dates")
            },
            Some(Box::new({
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.datesHint")
            })),
            view()
                .w_full()
                .flex_col()
                .child(setting_row(
                    language,
                    "settings.dateFormat",
                    Some("settings.dateFormatHint"),
                    ui::select::select(date_options, settings.date_format.clone(), Some(210.0), {
                        let set_style = set_date_style.clone();
                        move |id| set_style(crate::i18n::DateStyle::from_id(id))
                    })
                    .into_element(),
                    true,
                ))
                .child(setting_row(
                    language,
                    "settings.datePattern",
                    Some("settings.datePatternHint"),
                    view()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(view().w(px(210.0)).flex_shrink_0().child(pattern_input))
                        .child(
                            text(preview)
                                .text_size(px(ui::SECONDARY))
                                .text_color(move || design::text3()),
                        )
                        .into_element(),
                    false,
                )),
        ))
        .child(ui::setting_row::settings_section(
            false,
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.identity")
            },
            Some(Box::new({
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.identityHint")
            })),
            view()
                .w_full()
                .flex_col()
                .child(setting_row(
                    language,
                    "settings.name",
                    None,
                    view()
                        .w(px(210.0))
                        .flex_shrink_0()
                        .child(name_input)
                        .into_element(),
                    true,
                ))
                .child(setting_row(
                    language,
                    "settings.email",
                    None,
                    view()
                        .w(px(210.0))
                        .flex_shrink_0()
                        .child(email_input)
                        .into_element(),
                    true,
                ))
                .child(setting_row(
                    language,
                    "settings.useGlobalConfig",
                    Some("settings.useGlobalConfigHint"),
                    identity_toggle.into_element(),
                    false,
                )),
        ))
        .child(ui::setting_row::settings_section(
            false,
            {
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.fileTree")
            },
            Some(Box::new({
                let language = language;
                move || i18n::t(language.get().as_str(), "settings.fileTreeHint")
            })),
            view()
                .w_full()
                .flex_col()
                .child(setting_row(
                    language,
                    "settings.groupByFolder",
                    Some("settings.groupByFolderHint"),
                    ui::check::check_control(
                        move || i18n::t(language.get().as_str(), "settings.groupByFolder"),
                        settings.file_tree,
                        false,
                        {
                            let set_file_tree = set_file_tree.clone();
                            let value = settings.file_tree;
                            move |_next: bool| set_file_tree(!value)
                        },
                    )
                    .into_element(),
                    true,
                ))
                .child(setting_row(
                    language,
                    "settings.mergeEmptyDirs",
                    Some("settings.mergeEmptyDirsHint"),
                    ui::check::check_control(
                        move || i18n::t(language.get().as_str(), "settings.mergeEmptyDirs"),
                        settings.merge_empty_dirs,
                        false,
                        {
                            let set_tree_flag = set_tree_flag.clone();
                            let value = settings.merge_empty_dirs;
                            move |_next: bool| set_tree_flag("merge_empty_dirs", !value)
                        },
                    )
                    .into_element(),
                    true,
                ))
                .child(setting_row(
                    language,
                    "settings.compactSingleChild",
                    Some("settings.compactSingleChildHint"),
                    ui::check::check_control(
                        move || i18n::t(language.get().as_str(), "settings.compactSingleChild"),
                        settings.compact_single_child,
                        false,
                        {
                            let set_tree_flag = set_tree_flag.clone();
                            let value = settings.compact_single_child;
                            move |_next: bool| set_tree_flag("compact_single_child", !value)
                        },
                    )
                    .into_element(),
                    false,
                )),
        ));
}

fn setting_row(
    language: Signal<SharedString>,
    title_key: &'static str,
    hint_key: Option<&'static str>,
    control: AnyElement,
    divider: bool,
) -> AnyElement {
    let hint = hint_key.map(|key| {
        Box::new(move || i18n::t(language.get().as_str(), key)) as Box<dyn Fn() -> String>
    });
    return ui::setting_row::setting_row(
        move || i18n::t(language.get().as_str(), title_key),
        hint,
        control,
        divider,
    )
    .into_element();
}
