//! 设置页（设计板 23–27）：左侧导航 + 右侧分区设置。
//!
//! 窗口壳与 General 已按画板重做；Shortcuts / Git / Agent / Credentials / About
//! 仍沿用旧的 section 组合，后续阶段逐个替换。

use super::commands::SettingsCommands;
use super::state::{self, Section, SettingsState};
use crate::app::session::Tab;
use crate::brand::{self, ThemeMode};
use crate::design;
use crate::features::agent::commands::AgentCommands;
use crate::i18n;
use crate::ui;
use nsh_core::prelude::*;
use std::sync::Arc;

/// 左侧导航宽度（设计板 `.settings-side` 208）。
const SIDE_W: f32 = 208.0;
/// 页脚高度（画板 `.settings-foot` 44）。
const FOOTER_H: f32 = 44.0;

pub(crate) fn settings(
    state: &SettingsState,
    cmd: &SettingsCommands,
    tab: Option<Tab>,
    agent: &AgentCommands,
) -> impl IntoElement + use<> {
    let language = cmd.language;
    let section = state.settings_section;
    let last_session: Vec<String> = cmd
        .session
        .recent
        .get()
        .into_iter()
        .map(|entry| entry.path)
        .collect();
    let git_info = cmd.git_info.clone();
    return view()
        .w_full()
        .h_full()
        .flex_col()
        .bg(move || design::content())
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .flex_row()
                .child(dynamic(move || {
                    let _ = section.get();
                    side(language, section).into_element()
                }))
                .child(main(
                    state,
                    cmd,
                    tab,
                    agent,
                    git_info,
                    last_session,
                )),
        )
        .child(
            view()
                .w_full()
                .h(px(FOOTER_H))
                .px(px(16.0))
                .flex_row()
                .items_center()
                .gap(px(10.0))
                .border_t_1()
                .border_color(move || design::line())
                .bg(move || design::bar())
                .child(
                    text(move || i18n::t(language.get().as_str(), "settings.savedAutomatically"))
                        .text_size(px(ui::SECONDARY))
                        .line_height(px(ui::SECONDARY_LINE))
                        .text_color(move || design::text3()),
                )
                .child(view().flex_1())
                .child(ui::button::button(
                    move || i18n::t(language.get().as_str(), "common.done"),
                    ui::button::ButtonVariant::Secondary,
                    ui::button::ButtonSize::Regular,
                    false,
                    {
                        let cmd = cmd.clone();
                        move || cmd.session.close_settings()
                    },
                )),
        );
}

fn side(language: Signal<SharedString>, section: Signal<Section>) -> impl IntoElement {
    let mut groups: Vec<AnyElement> = Vec::new();
    for (title_key, items) in state::groups() {
        let mut rows: Vec<AnyElement> = Vec::new();
        for item in items {
            rows.push(
                ui::row::nav_row(
                    move || i18n::t(language.get().as_str(), item.key()),
                    item.icon(),
                    section.get_untracked() == *item,
                    move || section.set(*item),
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
                                .text_size(px(ui::SECONDARY))
                                .line_height(px(ui::SECONDARY_LINE))
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
        .child(
            view()
                .w_full()
                .px(px(2.0))
                .pt(px(6.0))
                .pb(px(12.0))
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
                        .child(crate::widgets::icon("branch", 15.0, || {
                            design::token("primary-foreground")
                        })),
                )
                .child(
                    view()
                        .flex_col()
                        .child(
                            text(move || i18n::t(language.get().as_str(), "settings.title"))
                                .text_size(px(ui::BODY))
                                .line_height(px(15.0))
                                .font_weight(FontWeight(600))
                                .text_color(move || design::text()),
                        )
                        .child(
                            text(brand::version_line())
                                .text_size(px(ui::META))
                                .line_height(px(ui::SECONDARY_LINE))
                                .text_color(move || design::text3()),
                        ),
                ),
        )
        .child(groups);
}

/// 设置页渲染需要的回调集合：把 `SettingsCommands` 的方法包成视图用的 `Arc`。
struct SettingsActions {
    set_theme: Arc<dyn Fn(ThemeMode)>,
    set_language: Arc<dyn Fn(&'static str)>,
    set_update_mode: Arc<dyn Fn(String)>,
    set_launch_restore: Arc<dyn Fn(bool)>,
    set_date_style: Arc<dyn Fn(crate::i18n::DateStyle)>,
    set_date_pattern: Arc<dyn Fn(String)>,
    save_identity: Arc<dyn Fn(String, String)>,
    clear_identity: Arc<dyn Fn()>,
    set_use_global: Arc<dyn Fn(bool)>,
    check_updates: Arc<dyn Fn()>,
    open_browser: Arc<dyn Fn(String)>,
    record_shortcut: Arc<dyn Fn(&'static str)>,
    restore_shortcuts: Arc<dyn Fn()>,
}

impl SettingsActions {
    fn new(cmd: &SettingsCommands) -> SettingsActions {
        let theme = cmd.clone();
        let language = cmd.clone();
        let update = cmd.clone();
        let launch = cmd.clone();
        let date_style = cmd.clone();
        let date_pattern = cmd.clone();
        let save_identity = cmd.clone();
        let clear_identity = cmd.clone();
        let use_global = cmd.clone();
        let check_updates = cmd.clone();
        let browser = cmd.clone();
        let record = cmd.clone();
        let restore = cmd.clone();
        return SettingsActions {
            set_theme: Arc::new(move |mode: ThemeMode| theme.set_theme(mode)),
            set_language: Arc::new(move |language_id: &'static str| {
                language.set_language(language_id)
            }),
            set_update_mode: Arc::new(move |mode: String| update.set_update_mode(mode)),
            set_launch_restore: Arc::new(move |restore: bool| launch.set_launch_restore(restore)),
            set_date_style: Arc::new(move |style: crate::i18n::DateStyle| {
                date_style.set_date_style(style)
            }),
            set_date_pattern: Arc::new(move |pattern: String| {
                date_pattern.set_date_pattern(pattern)
            }),
            save_identity: Arc::new(move |name: String, email: String| {
                save_identity.save_identity(name, email)
            }),
            clear_identity: Arc::new(move || clear_identity.clear_identity()),
            set_use_global: Arc::new(move |enabled: bool| {
                use_global.set_use_global_identity(enabled)
            }),
            check_updates: Arc::new(move || check_updates.check_updates()),
            open_browser: Arc::new(move |url: String| {
                crate::shared::os::open_browser(browser.notice, browser.language, url)
            }),
            record_shortcut: Arc::new(move |id: &'static str| record.record_shortcut(id)),
            restore_shortcuts: Arc::new(move || restore.restore_shortcuts()),
        };
    }
}

#[allow(clippy::too_many_arguments)]
fn main(
    state: &SettingsState,
    cmd: &SettingsCommands,
    tab: Option<Tab>,
    agent: &AgentCommands,
    git_info: (String, String, String),
    last_session: Vec<String>,
) -> impl IntoElement + use<> {
    let language = cmd.language;
    let theme_mode = cmd.theme_mode;
    let settings_state = state.settings_state;
    let section = state.settings_section;
    let identity_name = state.identity_name;
    let identity_email = state.identity_email;
    let identity_use_global = state.identity_use_global;
    let date_pattern = state.date_pattern;
    let has_repo = tab.is_some();
    let branches_for_agent = branches(tab.as_ref());
    let agent_for_page = agent.clone();
    let shortcut_registry = state.shortcut_registry;
    let shortcut_query = state.shortcut_query;
    let recording_shortcut = state.recording_shortcut;
    let shortcut_error = state.shortcut_error;
    let notice = cmd.notice;
    let diff = cmd.diff.clone();
    let set_file_tree: Arc<dyn Fn(bool)> = {
        let diff = diff.clone();
        Arc::new(move |enabled: bool| diff.set_file_tree(enabled))
    };
    let set_tree_flag: Arc<dyn Fn(&'static str, bool)> = {
        let diff = diff.clone();
        Arc::new(move |which: &'static str, value: bool| diff.set_tree_flag(which, value))
    };
    let git_cmd = cmd.clone();
    let api = SettingsActions::new(cmd);
    return view().flex_1().min_w_0().h_full().child(
        scroll().w_full().h_full().child(
            view()
                .w_full()
                .pt(px(24.0))
                .pb(px(24.0))
                .px(px(28.0))
                .flex_col()
                .child(ui::setting_row::page_header(
                    move || i18n::t(language.get().as_str(), section.get().key()),
                    move || {
                        i18n::t(
                            language.get().as_str(),
                            section_subtitle_key(section.get()),
                        )
                    },
                ))
                .child(dynamic(move || -> AnyElement {
                    let _ = settings_state.get();
                    let _ = agent_for_page.state.settings.get();
                    let _ = identity_name.get();
                    let _ = identity_email.get();
                    let _ = identity_use_global.get();
                    let _ = date_pattern.get();
                    let _ = git_cmd.state.git_version.get();
                    let _ = git_cmd.state.git_exec_path.get();
                    let _ = git_cmd.state.git_path.get();
                    let _ = git_cmd.state.scan_folders.get();
                    let _ = git_cmd.state.scan_counts.get();
                    let _ = git_cmd.state.scan_found.get();
                    let _ = git_cmd.state.scan_selected.get();
                    let _ = git_cmd.state.scan_last_at.get();
                    match section.get() {
                        Section::General => super::general::general(
                            language,
                            theme_mode,
                            settings_state,
                            identity_name,
                            identity_email,
                            identity_use_global,
                            date_pattern,
                            has_repo,
                            api.set_theme.clone(),
                            api.set_language.clone(),
                            api.set_update_mode.clone(),
                            api.set_launch_restore.clone(),
                            set_file_tree.clone(),
                            set_tree_flag.clone(),
                            api.set_date_style.clone(),
                            api.set_date_pattern.clone(),
                            api.save_identity.clone(),
                            api.clear_identity.clone(),
                            api.set_use_global.clone(),
                            api.check_updates.clone(),
                            last_session.clone(),
                        )
                        .into_element(),
                        Section::About => super::about::about(
                            language,
                            identity_name.get_untracked(),
                            identity_email.get_untracked(),
                            api.open_browser.clone(),
                        )
                        .into_element(),
                        Section::Git => super::git::git_page(git_cmd.clone()).into_element(),
                        Section::Shortcuts => super::shortcuts::shortcuts_page(
                            language,
                            shortcut_registry,
                            shortcut_query,
                            recording_shortcut,
                            shortcut_error,
                            api.record_shortcut.clone(),
                            api.restore_shortcuts.clone(),
                        )
                        .into_element(),
                        Section::Credentials => {
                            super::about::credentials_page(language, git_info.clone(), notice)
                                .into_element()
                        }
                        Section::Agent => super::agent::agent_page(
                            language,
                            &agent_for_page.state,
                            &agent_for_page,
                            branches_for_agent.clone(),
                        )
                        .into_element(),
                    }
                })),
        ),
    );
}

fn section_subtitle_key(section: Section) -> &'static str {
    return match section {
        Section::General => "settings.subtitleGeneral",
        Section::Shortcuts => "settings.subtitleShortcuts",
        Section::Agent => "settings.subtitleAgent",
        Section::Git => "settings.subtitleGit",
        Section::Credentials => "settings.subtitleCredentials",
        Section::About => "settings.subtitleAbout",
    };
}

/// 设置页的受保护分支列表：取当前仓库的本地分支名。
fn branches(tab: Option<&Tab>) -> Vec<String> {
    return tab
        .map(|tab| {
            tab.refs
                .branches
                .iter()
                .map(|branch| branch.name.clone())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
}

/// 一组互斥选项里的一个：选中用 Secondary（描边 + 卡片底色），未选中用 Ghost。
pub(super) fn option_chip(
    label: impl Fn() -> String + 'static,
    active: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let variant = if active {
        ui::button::ButtonVariant::Secondary
    } else {
        ui::button::ButtonVariant::Ghost
    };
    return ui::button::button(
        label,
        variant,
        ui::button::ButtonSize::Small,
        false,
        on_press,
    );
}

pub(super) fn section_block(
    language: Signal<SharedString>,
    title_key: &'static str,
    hint_key: &'static str,
    body: AnyElement,
) -> impl IntoElement {
    return view()
        .w_full()
        .flex_col()
        .gap(px(ui::GAP_SM))
        .child(
            text(move || i18n::t(language.get().as_str(), title_key))
                .text_size(px(ui::BODY))
                .line_height(px(ui::BODY_LINE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || design::text()),
        )
        .child(
            text(move || i18n::t(language.get().as_str(), hint_key))
                .text_size(px(ui::SECONDARY))
                .line_height(px(ui::SECONDARY_LINE))
                .text_color(move || design::text3()),
        )
        .child(body);
}
