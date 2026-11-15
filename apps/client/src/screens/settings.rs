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

/// 左侧导航宽度（设计板 `.settings-side` 208）。
const SIDE_W: f32 = 208.0;

/// 侧栏分组（设计板 23 的 `.sg` 标题）。Agent 是我们新增的一页，归在应用设置里。
const GROUPS: [(&str, &[Section]); 3] = [
    (
        "settings.groupApplication",
        &[Section::General, Section::Shortcuts, Section::Agent],
    ),
    ("settings.groupGit", &[Section::Git, Section::Credentials]),
    ("settings.groupAbout", &[Section::About]),
];

/// 设置页的分区。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Section {
    General,
    Git,
    Shortcuts,
    About,
    Credentials,
    Agent,
}

impl Section {
    /// 侧栏图标（与设计稿 23 的 `.si` 一致；`keys` / `lock` 取自设计 sprite）。
    fn icon(self) -> &'static str {
        return match self {
            Section::General => "gear",
            Section::Shortcuts => "keys",
            Section::Git => "branch",
            Section::Credentials => "lock",
            Section::Agent => "sparkle",
            Section::About => "info",
        };
    }

    fn key(self) -> &'static str {
        return match self {
            Section::General => "settings.general",
            Section::Git => "settings.git",
            Section::Shortcuts => "settings.shortcuts",
            Section::About => "settings.about",
            Section::Credentials => "settings.credentials",
            Section::Agent => "settings.agent",
        };
    }
}

/// 快捷键分区需要的状态与动作（注册表、搜索词、录制状态、改键错误）。
///
/// 打成一个结构体而不是继续加参数：设置页的参数已经很多，而这些值只服务于
/// 25 号「Shortcuts」一页。
pub(crate) struct ShortcutUi {
    pub registry: Signal<crate::shortcuts::Registry>,
    pub query: Signal<String>,
    pub recording: Signal<Option<&'static str>>,
    pub error: Signal<Option<String>>,
    pub record: std::sync::Arc<dyn Fn(&'static str)>,
    pub restore: std::sync::Arc<dyn Fn()>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn settings(
    language: Signal<SharedString>,
    theme_mode: Signal<ThemeMode>,
    settings_state: Signal<store::Settings>,
    section: Signal<Section>,
    identity: (String, String),
    git_info: (String, String, String),
    policies: Signal<agent::Policies>,
    set_permission: std::sync::Arc<dyn Fn(agent::Action, agent::Permission)>,
    agent_settings: Signal<agent::Settings>,
    branches_for_agent: Vec<String>,
    last_session: Vec<String>,
    use_system_git: bool,
    scan_folders: Vec<String>,
    found_repos: usize,
    add_found: std::sync::Arc<dyn Fn()>,
    set_automation: std::sync::Arc<dyn Fn(agent::Automation)>,
    set_review: std::sync::Arc<dyn Fn(u32)>,
    toggle_protected: std::sync::Arc<dyn Fn(String)>,
    shortcuts: ShortcutUi,
    notice: Signal<Option<String>>,
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
                    git_info,
                    policies,
                    set_permission,
                    agent_settings,
                    branches_for_agent,
                    last_session,
                    use_system_git,
                    scan_folders,
                    found_repos,
                    add_found,
                    set_automation,
                    set_review,
                    toggle_protected,
                    shortcuts,
                    notice,
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
                        .text_size(px(crate::ui::SECONDARY))
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
                .text_size(px(crate::ui::TITLE))
                .font_weight(FontWeight(600))
                .text_color(move || design::text()),
        )
        .child(view().flex_1())
        .child(crate::ui::button::icon_button_tuned(
            "chev-left",
            move || i18n::t(language.get().as_str(), "common.close"),
            crate::ui::button::ButtonSize::Small,
            Some(crate::ui::icon::IconSize::Inline),
            None,
            false,
            move || go_welcome(),
        ));
}

fn side(
    language: Signal<SharedString>,
    section: Signal<Section>,
    set_section: std::sync::Arc<dyn Fn(Section)>,
) -> impl IntoElement {
    let mut groups: Vec<AnyElement> = Vec::new();
    for (title_key, items) in GROUPS {
        let mut rows: Vec<AnyElement> = Vec::new();
        for item in items {
            let set_section = set_section.clone();
            // 图标在选中行用强调色，与设计稿的 `.settings-nav.on .si` 一致。
            rows.push(
                crate::ui::row::row(
                    move || i18n::t(language.get().as_str(), item.key()),
                    None,
                    Some(
                        view()
                            .w(px(16.0))
                            .items_center()
                            .justify_center()
                            .child(widgets::icon(item.icon(), 14.0, move || {
                                if section.get() == *item {
                                    design::accent()
                                } else {
                                    design::text3()
                                }
                            }))
                            .into_element(),
                    ),
                    vec![],
                    section.get() == *item,
                    false,
                    move || set_section(*item),
                )
                .into_element(),
            );
        }
        groups.push(
            view()
                .w_full()
                .flex_col()
                .gap(px(1.0))
                .child(
                    view()
                        .w_full()
                        .h(px(24.0))
                        .mt(px(8.0))
                        .px(px(8.0))
                        .items_center()
                        .child(
                            text(move || i18n::t(language.get().as_str(), title_key))
                                .text_size(px(crate::ui::SECONDARY))
                                .line_height(px(crate::ui::SECONDARY_LINE))
                                .font_weight(FontWeight(600))
                                .text_color(move || design::text3()),
                        ),
                )
                .child(rows)
                .into_element(),
        );
    }
    return view()
        .w(px(SIDE_W))
        .h_full()
        .flex_shrink_0()
        .flex_col()
        .p(px(8.0))
        .bg(move || design::sidebar())
        .border_r_1()
        .border_color(move || design::line())
        // 顶部产品标记 + 名称 + 版本（设计稿 23 的 `.appmark` 一行）。
        .child(
            view()
                .w_full()
                .px(px(2.0))
                .pt(px(4.0))
                .pb(px(10.0))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    view()
                        .w(px(28.0))
                        .h(px(28.0))
                        .items_center()
                        .justify_center()
                        .rounded(px(7.0))
                        .bg(move || design::accent2())
                        .child(widgets::icon("branch", 15.0, || {
                            design::token("primary-foreground")
                        })),
                )
                .child(
                    view()
                        .flex_col()
                        .child(
                            text(move || i18n::t(language.get().as_str(), "settings.title"))
                                .text_size(px(crate::ui::BODY))
                                .line_height(px(15.0))
                                .font_weight(FontWeight(600))
                                .text_color(move || design::text()),
                        )
                        .child(
                            text(brand::version_line())
                                .text_size(px(crate::ui::META))
                                .line_height(px(crate::ui::SECONDARY_LINE))
                                .text_color(move || design::text3()),
                        ),
                ),
        )
        .child(groups);
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
fn main(
    language: Signal<SharedString>,
    theme_mode: Signal<ThemeMode>,
    settings_state: Signal<store::Settings>,
    section: Signal<Section>,
    identity: (String, String),
    git_info: (String, String, String),
    policies: Signal<agent::Policies>,
    set_permission: std::sync::Arc<dyn Fn(agent::Action, agent::Permission)>,
    agent_settings: Signal<agent::Settings>,
    branches_for_agent: Vec<String>,
    last_session: Vec<String>,
    use_system_git: bool,
    scan_folders: Vec<String>,
    found_repos: usize,
    add_found: std::sync::Arc<dyn Fn()>,
    set_automation: std::sync::Arc<dyn Fn(agent::Automation)>,
    set_review: std::sync::Arc<dyn Fn(u32)>,
    toggle_protected: std::sync::Arc<dyn Fn(String)>,
    shortcuts: ShortcutUi,
    notice: Signal<Option<String>>,
    actions: Actions,
) -> impl IntoElement {
    let (name, email) = identity;
    // 仓库级身份为空时说明会回落到全局配置，而不是显示一对空尖括号。
    let identity_text = if name.is_empty() && email.is_empty() {
        i18n::t(language.get_untracked().as_str(), "settings.identityInherited")
    } else {
        format!("{} <{}>", name, email)
    };
    let set_theme = actions.set_theme.clone();
    let set_language = actions.toggle_language.clone();
    let set_file_tree = actions.set_file_tree.clone();
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
                        .text_size(px(crate::ui::TITLE_LARGE))
                        .font_weight(FontWeight(700))
                        .text_color(move || design::text()),
                )
                .child(dynamic(move || -> AnyElement {
                    // 读一次设置与 Agent 设置：任一项变化就整页重建，
                    // 静态样式的复选/选择控件因此能立刻反映新值。
                    let _ = settings_state.get();
                    let _ = agent_settings.get();
                    match section.get() {
                        Section::General => general(
                            language,
                            theme_mode,
                            settings_state,
                            set_theme.clone(),
                            set_language.clone(),
                            set_file_tree.clone(),
                            actions.set_update_mode.clone(),
                            actions.set_tree_flag.clone(),
                            actions.set_launch_restore.clone(),
                            last_session.clone(),
                            actions.set_date_style.clone(),
                            actions.set_date_pattern.clone(),
                            identity_text.clone(),
                            actions.open_identity.clone(),
                            actions.clear_identity.clone(),
                        )
                        .into_element(),
                        Section::About => {
                            about(language, name.clone(), email.clone(), actions.clone())
                                .into_element()
                        }
                        Section::Git => git_page(
                            language,
                            git_info.clone(),
                            actions.clone(),
                            use_system_git,
                            scan_folders.clone(),
                            found_repos,
                            add_found.clone(),
                        )
                        .into_element(),
                        Section::Shortcuts => shortcuts_page(
                            language,
                            shortcuts.registry,
                            shortcuts.query,
                            shortcuts.recording,
                            shortcuts.error,
                            shortcuts.record.clone(),
                            shortcuts.restore.clone(),
                        )
                        .into_element(),
                        Section::Credentials => {
                            credentials_page(language, git_info.clone(), notice).into_element()
                        }
                        Section::Agent => agent_page(
                            language,
                            policies,
                            set_permission.clone(),
                            agent_settings,
                            branches_for_agent.clone(),
                            set_automation.clone(),
                            set_review.clone(),
                            toggle_protected.clone(),
                        )
                        .into_element(),
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
    set_update_mode: std::sync::Arc<dyn Fn(String)>,
    set_tree_flag: std::sync::Arc<dyn Fn(&'static str, bool)>,
    set_launch: std::sync::Arc<dyn Fn(bool)>,
    last_session: Vec<String>,
    set_date_style: std::sync::Arc<dyn Fn(crate::i18n::DateStyle)>,
    set_date_pattern: std::sync::Arc<dyn Fn(String)>,
    identity_line: String,
    open_identity: std::sync::Arc<dyn Fn()>,
    clear_identity: std::sync::Arc<dyn Fn()>,
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
                .child(widgets::small_button(
                    move || i18n::t(language.get().as_str(), "settings.switchLanguage"),
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
                    view()
                        .w_full()
                        .flex_row()
                        .gap(px(2.0))
                        .child(
                            DATE_STYLES
                                .iter()
                                .map(|style| {
                                    let style = *style;
                                    let active = settings_state.get_untracked().date_format
                                        == style.id();
                                    let set_date_style = set_date_style.clone();
                                    let language = language;
                                    return option_chip(
                                        move || {
                                            i18n::t(language.get().as_str(), style.key())
                                        },
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
                                crate::dates::format(
                                    language.get().as_str(),
                                    now,
                                    now,
                                ),
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
                    std::sync::Arc::new(move || settings_state.get().file_tree),
                    move || set_file_tree(!settings_state.get_untracked().file_tree),
                ))
                .child(toggle_row(
                    language,
                    "settings.mergeEmptyDirs",
                    std::sync::Arc::new(move || settings_state.get().merge_empty_dirs),
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
                    std::sync::Arc::new(move || settings_state.get().compact_single_child),
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
                                .child(widgets::icon("folder", 10.0, design::text3))
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
                .child(widgets::small_button(
                    move || i18n::t(language.get().as_str(), "settings.editIdentity"),
                    {
                        let open_form = open_identity.clone();
                        move || open_form()
                    },
                ))
                .child(widgets::small_button(
                    move || i18n::t(language.get().as_str(), "settings.useGlobalIdentity"),
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
                                    let active = settings_state.get_untracked().update_mode
                                        == mode.id();
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

/// 启动方式的两个选项（设计板 23 的 Launch behavior）。
/// 一组互斥选项里的一个：选中用 Secondary（描边 + 卡片底色），未选中用 Ghost。
///
/// 设置页有六处这样的选择器（主题、日期样式、更新方式、启动方式、自动化、
/// 审查窗口），此前各自手写高度、圆角与 on_press；统一走这里。
fn option_chip(
    label: impl Fn() -> String + 'static,
    active: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let variant = if active {
        crate::ui::button::ButtonVariant::Secondary
    } else {
        crate::ui::button::ButtonVariant::Ghost
    };
    return crate::ui::button::button(
        label,
        variant,
        crate::ui::button::ButtonSize::Small,
        false,
        on_press,
    );
}

fn launch_choice(
    language: Signal<SharedString>,
    key: &'static str,
    active: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    return option_chip(
        move || i18n::t(language.get().as_str(), key),
        active,
        on_press,
    );
}

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

fn section_block(
    language: Signal<SharedString>,
    title_key: &'static str,
    hint_key: &'static str,
    body: AnyElement,
) -> impl IntoElement {
    return view()
        .w_full()
        .flex_col()
        .gap(px(crate::ui::GAP_SM))
        .child(
            text(move || i18n::t(language.get().as_str(), title_key))
                .text_size(px(crate::ui::BODY))
                .line_height(px(crate::ui::BODY_LINE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || design::text()),
        )
        .child(
            text(move || i18n::t(language.get().as_str(), hint_key))
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
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
    // 复选行：按压、键盘与无障碍由 nsh 的 Checkbox 提供；`checked` 在构建时
    // 取值，设置页在状态变化后整页重建，所以这里可以直接取。
    let value = checked();
    let toggle = move |_next: bool| on_press();
    return crate::ui::check::check_row(
        move || i18n::t(language.get().as_str(), label_key),
        value,
        false,
        toggle,
    )
    .into_element();
}

fn about(
    language: Signal<SharedString>,
    name: String,
    email: String,
    actions: Actions,
) -> impl IntoElement {
    return view()
        .w_full()
        .flex_col()
        .gap(px(8.0))
        .child(
            text(brand::version_line())
                .text_size(px(crate::ui::CODE))
                

                .text_color(move || design::text2()),
        )
        .child(
            text(move || i18n::t(language.get().as_str(), "settings.aboutHint"))
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        )
        .child(
            text(format!("{} <{}>", name, email))
                .text_size(px(crate::ui::CODE))
                

                .text_color(move || design::text2()),
        )
        // 四个外链；打不开时走状态栏提示，不弹错误框。
        .child(
            view()
                .w_full()
                .flex_row()
                .items_center()
                .flex_wrap()
                .gap(px(6.0))
                .child(link_button(
                    language,
                    "settings.website",
                    "https://example.com/git-client",
                    actions.open_browser.clone(),
                ))
                .child(link_button(
                    language,
                    "settings.source",
                    "https://example.com/git-client/source",
                    actions.open_browser.clone(),
                ))
                .child(link_button(
                    language,
                    "settings.reportIssue",
                    "https://example.com/git-client/issues",
                    actions.open_browser.clone(),
                ))
                .child(link_button(
                    language,
                    "settings.licenses",
                    "https://example.com/git-client/licenses",
                    actions.open_browser.clone(),
                )),
        );
}

/// 一个外链按钮。
fn link_button(
    language: Signal<SharedString>,
    key: &'static str,
    url: &'static str,
    open_browser: std::sync::Arc<dyn Fn(String)>,
) -> impl IntoElement {
    return crate::ui::button::button(
        move || i18n::t(language.get().as_str(), key),
        crate::ui::button::ButtonVariant::Secondary,
        crate::ui::button::ButtonSize::Small,
        false,
        move || open_browser(url.to_string()),
    );
}

/// Git 分区（24）：本机 Git 版本与执行路径、凭据助手（只读）、仓库发现（未实现）。
fn git_page(
    language: Signal<SharedString>,
    info: (String, String, String),
    actions: Actions,
    use_path: bool,
    scan_folders: Vec<String>,
    found: usize,
    add_found: std::sync::Arc<dyn Fn()>,
) -> AnyElement {
    let (version, exec_path, helper) = info;
    let detect = actions.detect_git.clone();
    let choose = actions.choose_git.clone();
    let toggle_path = actions.toggle_git_path.clone();
    let helper_text = if helper.trim().is_empty() {
        i18n::t(language.get_untracked().as_str(), "settings.gitHelperNone")
    } else {
        helper
    };
    return view()
        .w_full()
        .flex_col()
        .gap(px(18.0))
        .child(section_block(
            language,
            "settings.gitExecutable",
            "settings.gitExecutableHint",
            view()
                .flex_col()
                .gap(px(4.0))
                .child(
                    text(version)
                        .text_size(px(crate::ui::CODE))
                        

                        .line_height(px(15.0))
                        .text_color(move || design::text()),
                )
                .child(
                    text(exec_path)
                        .text_size(px(crate::ui::CODE))
                        

                        .line_height(px(14.0))
                        .text_color(move || design::text3())
                        .text_overflow(nsh_core::EllipsizeMode::Middle),
                )
                .child(
                    view()
                        .w_full()
                        .flex_row()
                        .items_center()
                        .gap(px(6.0))
                        .child(widgets::small_button(
                            {
                                let language = language;
                                move || i18n::t(language.get().as_str(), "settings.choose")
                            },
                            {
                                let choose = choose.clone();
                                move || choose()
                            },
                        ))
                        .child(widgets::small_button(
                            {
                                let language = language;
                                move || i18n::t(language.get().as_str(), "settings.detect")
                            },
                            {
                                let detect = detect.clone();
                                move || detect()
                            },
                        ))
                        .child(toggle_row(
                            language,
                            "settings.usePath",
                            std::sync::Arc::new(move || use_path),
                            {
                                let toggle_path = toggle_path.clone();
                                move || toggle_path()
                            },
                        )),
                )
                .into_element(),
        ))
        .child(section_block(
            language,
            "settings.discovery2",
            "settings.discovery2Hint",
            view()
                .w_full()
                .flex_col()
                .gap(px(6.0))
                .child(
                    view()
                        .w_full()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            scan_folders
                                .iter()
                                .map(|folder| {
                                    let label = folder.clone();
                                    return view()
                                        .w_full()
                                        .h(px(22.0))
                                        .px(px(7.0))
                                        .items_center()
                                        .rounded(px(4.0))
                                        .bg(move || design::panel2())
                                        .child(
                                            text(label)
                                                .text_size(px(crate::ui::CODE))
                                                .line_height(px(crate::ui::CODE_LINE))
                                                .text_color(move || design::text2())
                                                .text_overflow(nsh_core::EllipsizeMode::Middle),
                                        )
                                        .into_element();
                                })
                                .collect::<Vec<AnyElement>>(),
                        ),
                )
                .child(
                    view()
                        .w_full()
                        .flex_row()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            text(move || {
                                i18n::with_arguments(
                                    language.get().as_str(),
                                    "settings.scanFound",
                                    &[("count", found.to_string())],
                                )
                            })
                            .text_size(px(crate::ui::SECONDARY))
                            .line_height(px(crate::ui::SECONDARY_LINE))
                            .text_color(move || design::text3()),
                        )
                        .child(view().flex_1())
                        .child(widgets::small_button(
                            {
                                let language = language;
                                move || i18n::t(language.get().as_str(), "settings.scanNow")
                            },
                            {
                                let add = add_found.clone();
                                move || add()
                            },
                        )),
                )
                .into_element(),
        ))
        .child(section_block(
            language,
            "settings.credentialHelper",
            "settings.credentialHelperHint",
            view()
                .flex_col()
                .gap(px(4.0))
                .child(
                    text(helper_text)
                        .text_size(px(crate::ui::CODE))
                        

                        .line_height(px(15.0))
                        .text_color(move || design::text2()),
                )
                .child(
                    text(move || i18n::t(language.get().as_str(), "settings.gitHelperNote"))
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(crate::ui::SECONDARY_LINE))
                        .text_color(move || design::text3()),
                )
                .into_element(),
        ))
        .child(section_block(
            language,
            "settings.discovery",
            "settings.discoveryHint",
            disabled_row(language, "settings.scanNow"),
        ))
        .into_element();
}

/// 快捷键分区（25）：列出本构建真实的按键绑定。
fn shortcuts_page(
    language: Signal<SharedString>,
    registry: Signal<crate::shortcuts::Registry>,
    query: Signal<String>,
    recording: Signal<Option<&'static str>>,
    error: Signal<Option<String>>,
    record: std::sync::Arc<dyn Fn(&'static str)>,
    restore: std::sync::Arc<dyn Fn()>,
) -> AnyElement {
    let needle = query.get_untracked().to_lowercase();
    let bindings = registry.get_untracked();
    let mut rows: Vec<AnyElement> = Vec::new();
    for action in crate::shortcuts::ACTIONS.iter() {
        let label = crate::shortcuts::label(action, language.get_untracked().as_str());
        if !needle.is_empty() && !label.to_lowercase().contains(&needle) {
            continue;
        }
        let id = action.id;
        let is_default = bindings.combo(id) == action.default;
        let record = record.clone();
        rows.push(
            view()
                .w_full()
                .h(px(26.0))
                .flex_row()
                .items_center()
                .gap(px(10.0))
                .child(
                    text(label)
                        .flex_1()
                        .text_size(px(crate::ui::BODY))
                        .line_height(px(15.0))
                        .text_color(move || design::text()),
                )
                .child(
                    text(move || {
                        if is_default {
                            String::new()
                        } else {
                            i18n::t(language.get().as_str(), "settings.customized")
                        }
                    })
                    .text_size(px(crate::ui::META))
                    .line_height(px(crate::ui::SECONDARY_LINE))
                    .text_color(move || design::text3()),
                )
                .child(
                    text(move || {
                        let value = registry.get().combo(id).to_string();
                        crate::shortcuts::pretty(value.as_str())
                    })
                    .text_size(px(crate::ui::CODE))
                    

                    .line_height(px(14.0))
                    .text_color(move || design::text2()),
                )
                .child(widgets::small_button(
                    move || i18n::t(language.get().as_str(), "shortcut.record"),
                    {
                        let record = record.clone();
                        move || record(id)
                    },
                ))
                .into_element(),
        );
    }
    if rows.is_empty() {
        rows.push(
            text(move || i18n::t(language.get().as_str(), "shortcut.searchEmpty"))
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3())
                .into_element(),
        );
    }
    return view()
        .w_full()
        .flex_col()
        .gap(px(10.0))
        .child(
            text(move || i18n::t(language.get().as_str(), "settings.shortcutsHint"))
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        )
        .child(
            view()
                .w_full()
                .h(px(metrics::FIELD_H))
                .flex_row()
                .items_center()
                .px(px(8.0))
                .rounded(px(metrics::RADIUS_CONTROL))
                .border_1()
                .border_color(move || design::field_border())
                .bg(move || design::field())
                .child(crate::ui::icon::icon_view("search", crate::ui::icon::IconSize::Button, design::text3))
                .child(
                    nsh_base::input()
                        .value(query)
                        .on_change(move |text: &str| query.set(text.to_string()))
                        .placeholder(move || {
                            i18n::t(language.get().as_str(), "shortcut.search")
                        })
                        .text_size(px(crate::ui::BODY))
                        .flex_1(),
                ),
        )
        // 录制提示与改键错误：两者都只在有内容时占高度。
        .child(
            view()
                .w_full()
                .flex_col()
                .gap(px(2.0))
                .child(
                    text(move || match recording.get() {
                        Some(id) => {
                            let name = crate::shortcuts::ACTIONS
                                .iter()
                                .find(|action| action.id == id)
                                .map(|action| {
                                    crate::shortcuts::label(action, language.get_untracked().as_str())
                                })
                                .unwrap_or_default();
                            i18n::with_arguments(
                                language.get().as_str(),
                                "shortcut.recording",
                                &[("action", name)],
                            )
                        }
                        None => String::new(),
                    })
                    .text_size(px(crate::ui::SECONDARY))
                    .line_height(px(crate::ui::SECONDARY_LINE))
                    .text_color(move || design::accent()),
                )
                .child(
                    text(move || error.get().unwrap_or_default())
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(crate::ui::SECONDARY_LINE))
                        .text_color(move || design::danger()),
                ),
        )
        .child(view().w_full().flex_col().gap(px(2.0)).child(rows))
        .child(
            view()
                .w_full()
                .flex_row()
                .gap(px(8.0))
                .child(view().flex_1())
                .child(widgets::button(
                    move || i18n::t(language.get().as_str(), "shortcut.restoreDefaults"),
                    {
                        let restore = restore.clone();
                        move || restore()
                    },
                )),
        )
        .child(
            text(move || i18n::t(language.get().as_str(), "shortcut.catalogHint"))
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        )
        .into_element();
}

/// 凭据分区（27）：本机 Git 的凭据助手与 ~/.ssh 状态（只读）。
fn credentials_page(
    language: Signal<SharedString>,
    info: (String, String, String),
    notice: Signal<Option<String>>,
) -> AnyElement {
    let (_, _, helper) = info;
    let helper_text = if helper.trim().is_empty() {
        i18n::t(language.get_untracked().as_str(), "settings.gitHelperNone")
    } else {
        helper
    };
    let keys = std::env::var_os("HOME")
        .map(|home| std::path::PathBuf::from(home).join(".ssh"))
        .map(|dir| std::fs::read_dir(dir).map(|entries| entries.count()).unwrap_or(0))
        .unwrap_or(0);
    return view()
        .w_full()
        .flex_col()
        .gap(px(18.0))
        .child(section_block(
            language,
            "settings.connections",
            "settings.connectionsHint",
            view()
                .flex_col()
                .gap(px(4.0))
                .child(
                    text(helper_text)
                        .text_size(px(crate::ui::CODE))
                        

                        .line_height(px(15.0))
                        .text_color(move || design::text()),
                )
                .child(pending_row(language, "settings.signIn", notice))
                .into_element(),
        ))
        .child(section_block(
            language,
            "settings.sshKeys",
            "settings.sshKeysHint",
            view()
                .flex_col()
                .gap(px(4.0))
                .child(
                    text(move || {
                        i18n::with_arguments(
                            language.get().as_str(),
                            "settings.sshKeyCount",
                            &[("count", keys.to_string())],
                        )
                    })
                    .text_size(px(crate::ui::SECONDARY))
                    .line_height(px(15.0))
                    .text_color(move || design::text2()),
                )
                .child(
                    text(move || i18n::t(language.get().as_str(), "settings.sshAgentsNote"))
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(crate::ui::SECONDARY_LINE))
                        .text_color(move || design::text3()),
                )
                .into_element(),
        ))
        .into_element();
}

/// 可点但尚未接入的按钮：点一下在状态栏说明原因，不写假数据。
fn pending_row(
    language: Signal<SharedString>,
    label_key: &'static str,
    notice: Signal<Option<String>>,
) -> AnyElement {
    return view()
        .flex_row()
        .items_center()
        .gap(px(8.0))
        .child(crate::widgets::small_button(
            move || i18n::t(language.get().as_str(), label_key),
            move || {
                let lang = language.get_untracked();
                crate::interactions::notify(
                    notice,
                    i18n::t(lang.as_str(), "settings.credentialsPending"),
                );
            },
        ))
        .child(
            text(move || i18n::t(language.get().as_str(), "settings.notConnected"))
                .text_size(px(crate::ui::META))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        )
        .into_element();
}

/// 禁用态按钮 + 「尚未实现」说明。
fn disabled_row(language: Signal<SharedString>, label_key: &'static str) -> AnyElement {
    return view()
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
                    text(move || i18n::t(language.get().as_str(), label_key))
                        .text_size(px(crate::ui::SECONDARY))
                        .line_height(px(14.0))
                        .text_color(move || design::text()),
                ),
        )
        .child(
            text(move || i18n::t(language.get().as_str(), "settings.notImplemented"))
                .text_size(px(crate::ui::META))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        )
        .into_element();
}

/// Agent 权限分区（设计板 13）：逐动作的 Ask / Allow / Deny，并说明边界。
fn agent_page(
    language: Signal<SharedString>,
    policies: Signal<agent::Policies>,
    set_permission: std::sync::Arc<dyn Fn(agent::Action, agent::Permission)>,
    settings: Signal<agent::Settings>,
    branches: Vec<String>,
    set_automation: std::sync::Arc<dyn Fn(agent::Automation)>,
    set_review: std::sync::Arc<dyn Fn(u32)>,
    toggle_protected: std::sync::Arc<dyn Fn(String)>,
) -> AnyElement {
    let mut rows: Vec<AnyElement> = Vec::new();
    for action in [
        agent::Action::StageAll,
        agent::Action::Commit,
        agent::Action::UnstageAll,
        agent::Action::Fetch,
        agent::Action::Pull,
        agent::Action::Push,
        agent::Action::Discard,
        agent::Action::ReadFiles,
        agent::Action::ForcePush,
    ] {
        let set_permission = set_permission.clone();
        rows.push(
            view()
                .w_full()
                .h(px(28.0))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    text(move || {
                        i18n::t(language.get().as_str(), agent_action_key(action))
                    })
                    .flex_1()
                    .text_size(px(crate::ui::BODY))
                    .line_height(px(15.0))
                    .text_color(move || design::text()),
                )
                .child(
                    text(move || {
                        if action.is_destructive() {
                            i18n::t(language.get().as_str(), "agentPage.destructive")
                        } else {
                            String::new()
                        }
                    })
                    .text_size(px(crate::ui::META))
                    .text_color(move || design::danger()),
                )
                .child(crate::ui::button::button(
                    move || {
                        i18n::t(
                            language.get().as_str(),
                            match policies.get().permission(action) {
                                agent::Permission::Ask => "agent.ask",
                                agent::Permission::Allow => "agent.allow",
                                agent::Permission::Deny => "agent.deny",
                            },
                        )
                    },
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    {
                        let set_permission = set_permission.clone();
                        move || {
                            let next = match policies.get_untracked().permission(action) {
                                agent::Permission::Ask => agent::Permission::Allow,
                                agent::Permission::Allow => agent::Permission::Deny,
                                agent::Permission::Deny => agent::Permission::Ask,
                            };
                            set_permission(action, next);
                        }
                    },
                ))
                .into_element(),
        );
    }
        // 自动化程度：Off / Commit / Commit & Push。
    let automation_rows: Vec<AnyElement> = [
        agent::Automation::Off,
        agent::Automation::Commit,
        agent::Automation::CommitAndPush,
    ]
    .iter()
    .map(|mode| {
        let mode = *mode;
        let active = settings.get_untracked().automation == mode;
        let set_automation = set_automation.clone();
        let language = language;
        return option_chip(
            move || i18n::t(language.get().as_str(), automation_key(mode)),
            active,
            move || set_automation(mode),
        )
        .into_element();
    })
    .collect();

    // 预览时长：0 / 3 / 5 / 8 秒。
    let review_rows: Vec<AnyElement> = agent::Settings::REVIEW_CHOICES
        .iter()
        .map(|millis| {
            let millis = *millis;
            let active = settings.get_untracked().review_millis == millis;
            let set_review = set_review.clone();
            let language = language;
            let label = if millis == 0 {
                i18n::t(language.get_untracked().as_str(), "agentPage.reviewNone")
            } else {
                format!("{}s", millis / 1000)
            };
            return option_chip(
                move || label.clone(),
                active,
                move || set_review(millis),
            )
            .into_element();
        })
        .collect();

    // 受保护分支：列出本地分支，点一下切换保护状态。
    let protected_rows: Vec<AnyElement> = branches
        .iter()
        .map(|branch| {
            let name = branch.clone();
            let checked = settings.get_untracked().is_protected(name.as_str());
            let toggle = toggle_protected.clone();
            let language = language;
            let label = name.clone();
            let _ = language;
            return crate::ui::check::check_row(
                move || label.clone(),
                checked,
                false,
                {
                    let name = name.clone();
                    move |_next: bool| toggle(name.clone())
                },
            )
            .into_element();
        })
        .collect();

    let mut block = view()
        .w_full()
        .flex_col()
        .gap(px(10.0))
        .child(
            text(move || i18n::t(language.get().as_str(), "agentPage.hint"))
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        )
        .child(rows)
        .child(section_block(
            language,
            "agentPage.automation",
            "agentPage.automationHint",
            view().w_full().flex_row().gap(px(2.0)).child(automation_rows).into_element(),
        ))
        .child(section_block(
            language,
            "agentPage.review",
            "agentPage.reviewHint",
            view().w_full().flex_row().gap(px(2.0)).child(review_rows).into_element(),
        ));
    if !protected_rows.is_empty() {
        block = block.child(section_block(
            language,
            "agentPage.protected",
            "agentPage.protectedHint",
            view().w_full().flex_col().gap(px(2.0)).child(protected_rows).into_element(),
        ));
    }
    return block
        .child(
            text(move || i18n::t(language.get().as_str(), "agentPage.boundary"))
                .text_size(px(crate::ui::SECONDARY))
                .line_height(px(crate::ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        )
        .into_element();
}

/// 自动化程度的文案键。
fn automation_key(mode: agent::Automation) -> &'static str {
    return match mode {
        agent::Automation::Off => "agentPage.autoOff",
        agent::Automation::Commit => "agentPage.autoCommit",
        agent::Automation::CommitAndPush => "agentPage.autoCommitPush",
    };
}

fn agent_action_key(action: agent::Action) -> &'static str {
    return match action {
        agent::Action::StageAll => "agent.action.stageAll",
        agent::Action::Commit => "agent.action.commit",
        agent::Action::UnstageAll => "agent.action.unstageAll",
        agent::Action::Fetch => "agent.action.fetch",
        agent::Action::Pull => "agent.action.pull",
        agent::Action::Push => "agent.action.push",
        agent::Action::Discard => "agent.action.discard",
        agent::Action::ReadFiles => "agent.action.readFiles",
        agent::Action::ForcePush => "agent.action.forcePush",
    };
}
