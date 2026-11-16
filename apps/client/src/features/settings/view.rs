//! 设置页（设计板 23–27）：左侧导航 + 右侧分区设置。
//!
//! 已接入的项：外观（跟随系统/浅色/深色）、语言（英文/中文）、提交身份（读
//! `git config`，只读展示）、文件树分组、启动行为、日期格式、更新方式、Git 分区、
//! 快捷键与 Agent。其余项按设计稿呈现标题与说明，内容明确标注「尚未实现」。
//!
//! 入口收本功能的状态与用例；身份编辑走引用管理，Agent 的权限与运行设置走
//! `features::agent`（单向：settings → agent，agent 不反向依赖设置页）。

use super::commands::SettingsCommands;
use super::state::{self, Section, SettingsState};
use crate::brand::{self, ThemeMode};
use crate::design::{self, metrics};
use crate::features::agent::commands::AgentCommands;
use crate::features::refs::commands::RefsCommands;
use crate::i18n;
use crate::model::Tab;
use nsh_core::prelude::*;
use std::sync::Arc;

/// 左侧导航宽度（设计板 `.settings-side` 208）。
const SIDE_W: f32 = 208.0;

pub(crate) fn settings(
    state: &SettingsState,
    cmd: &SettingsCommands,
    tab: Option<Tab>,
    refs: &RefsCommands,
    agent: &AgentCommands,
) -> impl IntoElement + use<> {
    let language = cmd.language;
    let section = state.settings_section;
    let identity = identity(tab.as_ref());
    let last_session: Vec<String> = cmd
        .session
        .recent
        .get()
        .into_iter()
        .map(|entry| entry.path)
        .collect();
    let scan_folders: Vec<String> = store::load_git_scan_folders(&cmd.store_dir)
        .iter()
        .map(|folder| folder.display().to_string())
        .collect();
    let found_repos =
        store::scan_git_repositories(&store::load_git_scan_folders(&cmd.store_dir)).len();
    let git_info = cmd.git_info.clone();
    let session = cmd.session.clone();
    return view()
        .w_full()
        .h_full()
        .flex_col()
        .bg(move || design::content())
        .child(titlebar(language, session.clone()))
        .child(
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .flex_row()
                .child(side(language, section))
                .child(main(
                    state,
                    cmd,
                    tab,
                    refs,
                    agent,
                    identity,
                    git_info,
                    last_session,
                    scan_folders,
                    found_repos,
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
                .child(crate::ui::button::button(
                    move || i18n::t(language.get().as_str(), "common.done"),
                    crate::ui::button::ButtonVariant::Secondary,
                    crate::ui::button::ButtonSize::Regular,
                    false,
                    {
                        let session = session.clone();
                        move || session.go_welcome()
                    },
                )),
        );
}

fn titlebar(
    language: Signal<SharedString>,
    session: crate::features::SessionHandle,
) -> impl IntoElement {
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
            move || session.go_welcome(),
        ));
}

fn side(language: Signal<SharedString>, section: Signal<Section>) -> impl IntoElement {
    let mut groups: Vec<AnyElement> = Vec::new();
    for (title_key, items) in state::groups() {
        let mut rows: Vec<AnyElement> = Vec::new();
        for item in items {
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
                            .child(crate::ui::icon::icon_view(
                                item.icon(),
                                crate::ui::icon::IconSize::Inline,
                                move || {
                                    if section.get() == *item {
                                        design::accent()
                                    } else {
                                        design::text3()
                                    }
                                },
                            ))
                            .into_element(),
                    ),
                    vec![],
                    section.get() == *item,
                    false,
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
                        .child(crate::widgets::icon("branch", 15.0, || {
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

/// 设置页渲染需要的回调集合：把 `SettingsCommands` 的方法包成原先视图用的
/// `Arc<dyn Fn>` 形状，避免每个分区各自克隆整份用例。
struct SettingsActions {
    set_theme: Arc<dyn Fn(ThemeMode)>,
    toggle_language: Arc<dyn Fn()>,
    set_update_mode: Arc<dyn Fn(String)>,
    set_launch_restore: Arc<dyn Fn(bool)>,
    set_date_style: Arc<dyn Fn(crate::i18n::DateStyle)>,
    set_date_pattern: Arc<dyn Fn(String)>,
    add_found_repos: Arc<dyn Fn()>,
    open_browser: Arc<dyn Fn(String)>,
    detect_git: Arc<dyn Fn()>,
    choose_git: Arc<dyn Fn()>,
    toggle_git_path: Arc<dyn Fn()>,
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
        let add_found = cmd.clone();
        let browser = cmd.clone();
        let detect = cmd.clone();
        let choose = cmd.clone();
        let toggle_path = cmd.clone();
        let record = cmd.clone();
        let restore = cmd.clone();
        return SettingsActions {
            set_theme: Arc::new(move |mode: ThemeMode| theme.set_theme(mode)),
            toggle_language: Arc::new(move || language.toggle_language()),
            set_update_mode: Arc::new(move |mode: String| update.set_update_mode(mode)),
            set_launch_restore: Arc::new(move |restore: bool| launch.set_launch_restore(restore)),
            set_date_style: Arc::new(move |style: crate::i18n::DateStyle| {
                date_style.set_date_style(style)
            }),
            set_date_pattern: Arc::new(move |pattern: String| {
                date_pattern.set_date_pattern(pattern)
            }),
            add_found_repos: Arc::new(move || add_found.add_found_repos()),
            open_browser: Arc::new(move |url: String| {
                crate::shared::os::open_browser(browser.notice, browser.language, url)
            }),
            detect_git: Arc::new(move || detect.detect_git()),
            choose_git: Arc::new(move || choose.choose_git()),
            toggle_git_path: Arc::new(move || toggle_path.toggle_git_path()),
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
    refs: &RefsCommands,
    agent: &AgentCommands,
    identity: (String, String),
    git_info: (String, String, String),
    last_session: Vec<String>,
    scan_folders: Vec<String>,
    found_repos: usize,
) -> impl IntoElement + use<> {
    let language = cmd.language;
    let theme_mode = cmd.theme_mode;
    let settings_state = state.settings_state;
    let section = state.settings_section;
    let branches_for_agent = branches(tab.as_ref());
    // `dynamic` 是 `'static` 闭包，Agent 的用例先克隆一份再进去。
    let agent_for_page = agent.clone();
    // `dynamic` 是 `'static` 闭包，信号先取出来，别把 `&state` 带进去。
    let shortcut_registry = state.shortcut_registry;
    let shortcut_query = state.shortcut_query;
    let recording_shortcut = state.recording_shortcut;
    let shortcut_error = state.shortcut_error;
    // 身份编辑已归引用管理：把它的用例包成回调，沿用原来的参数位。
    let open_identity: Arc<dyn Fn()> = {
        let refs = refs.clone();
        Arc::new(move || refs.open_identity())
    };
    let clear_identity: Arc<dyn Fn()> = {
        let refs = refs.clone();
        Arc::new(move || refs.clear_identity())
    };
    let (name, email) = identity;
    // 仓库级身份为空时说明会回落到全局配置，而不是显示一对空尖括号。
    let identity_text = if name.is_empty() && email.is_empty() {
        i18n::t(
            language.get_untracked().as_str(),
            "settings.identityInherited",
        )
    } else {
        format!("{} <{}>", name, email)
    };
    let use_system_git = settings_state.get_untracked().use_system_git;
    let notice = cmd.notice;
    // 文件树属于 diff 的显示偏好，命令在 shared::diff。
    let diff = cmd.diff.clone();
    let set_file_tree: Arc<dyn Fn(bool)> = {
        let diff = diff.clone();
        Arc::new(move |enabled: bool| diff.set_file_tree(enabled))
    };
    let set_tree_flag: Arc<dyn Fn(&'static str, bool)> = {
        let diff = diff.clone();
        Arc::new(move |which: &'static str, value: bool| diff.set_tree_flag(which, value))
    };
    let api = SettingsActions::new(cmd);
    return view().flex_1().min_w_0().h_full().child(
        scroll().w_full().h_full().child(
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
                    let _ = agent_for_page.state.settings.get();
                    match section.get() {
                        Section::General => super::general::general(
                            language,
                            theme_mode,
                            settings_state,
                            api.set_theme.clone(),
                            api.toggle_language.clone(),
                            set_file_tree.clone(),
                            api.set_update_mode.clone(),
                            set_tree_flag.clone(),
                            api.set_launch_restore.clone(),
                            last_session.clone(),
                            api.set_date_style.clone(),
                            api.set_date_pattern.clone(),
                            identity_text.clone(),
                            open_identity.clone(),
                            clear_identity.clone(),
                        )
                        .into_element(),
                        Section::About => super::about::about(
                            language,
                            name.clone(),
                            email.clone(),
                            api.open_browser.clone(),
                        )
                        .into_element(),
                        Section::Git => super::git::git_page(
                            language,
                            git_info.clone(),
                            api.choose_git.clone(),
                            api.detect_git.clone(),
                            api.toggle_git_path.clone(),
                            use_system_git,
                            scan_folders.clone(),
                            found_repos,
                            api.add_found_repos.clone(),
                        )
                        .into_element(),
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

/// 身份按当前仓库读（仓库级 config），没有打开仓库时回落到全局配置。
fn identity(tab: Option<&Tab>) -> (String, String) {
    if let Some(tab) = tab {
        return crate::model::identity(tab);
    }
    let read = |key: &str| {
        return std::process::Command::new("git")
            .args(["config", "--get", key])
            .output()
            .ok()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_default();
    };
    return (read("user.name"), read("user.email"));
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
///
/// 设置页有六处这样的选择器（主题、日期样式、更新方式、启动方式、自动化、
/// 审查窗口），此前各自手写高度、圆角与 on_press；统一走这里。
pub(super) fn option_chip(
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

/// 启动方式的两个选项（设计板 23 的 Launch behavior）。
pub(super) fn launch_choice(
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

pub(super) fn section_block(
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

pub(super) fn toggle_row(
    language: Signal<SharedString>,
    label_key: &'static str,
    checked: Arc<dyn Fn() -> bool>,
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
