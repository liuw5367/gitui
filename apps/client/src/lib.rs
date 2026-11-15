//! 根组件：窗口属性、主题装载、语言状态与屏幕路由。
//!
//! 一个窗口、多个仓库标签页；屏幕为欢迎页、仓库管理页与工作区。当前只读：
//! 打开/克隆/初始化仓库、按条件筛选历史、多提交对比、查看提交内单文件补丁。
//! 写路径在后续阶段加入。

pub mod brand;
mod dates;
mod design;
mod filters;
mod i18n;
mod interactions;
mod model;
mod screens;
mod shell;
mod shortcuts;
// ui 层已建立；P1–P7 逐组迁入后仍有为后续阶段准备的组件，P8 收尾时
// 全部组件都有使用者，届时删除这条 allow 并保持零警告。
#[allow(dead_code)]
mod ui;
mod widgets;
mod workspace;

use nsh_base::theme_overrides;
use nsh_core::desktop::{TitleBarOptions, TitleBarStyle, WindowAttributes};

use nsh_core::element::desktop::{main_window, title_bar};
use nsh_core::desktop::MenuRole;
use nsh_core::element::desktop::MenuItem;
use nsh_core::element::set_tokens;
use nsh_core::prelude::*;
use std::sync::Arc;

use brand::ThemeMode;
use filters::When;
use git::LogFilter;
use model::Tab;

/// `theme.json` 与设计稿 CSS 的 light/dark 值一一对应。
const THEME_JSON: &str = include_str!("../theme.json");

/// 当前显示哪一屏。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Screen {
    Workspace,
    Settings,
}

/// 工作区主区的两种模式（工具栏的 History / Changes 分段）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    History,
    Changes,
    Refs,
    Agent,
}

/// 仓库管理页的三种入口（设计板 03 的分段控件）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RepoForm {
    Open,
    Clone,
    Init,
}

/// 组件间共享的动作集合。统一用 `Arc<dyn Fn>` 传递，避免把十几个信号逐个
/// 穿到子视图里。
#[derive(Clone)]
pub(crate) struct Actions {
    pub(crate) open_path: Arc<dyn Fn(String)>,
    pub(crate) choose_folder: Arc<dyn Fn()>,
    pub(crate) close_tab: Arc<dyn Fn(usize)>,
    pub(crate) activate_tab: Arc<dyn Fn(usize)>,
    pub(crate) select_commit: Arc<dyn Fn(usize)>,
    pub(crate) toggle_compare: Arc<dyn Fn(usize)>,
    pub(crate) set_compare_mode: Arc<dyn Fn(bool)>,
    pub(crate) select_file: Arc<dyn Fn(usize)>,
    pub(crate) set_mode: Arc<dyn Fn(Mode)>,
    pub(crate) select_change: Arc<dyn Fn(usize)>,
    pub(crate) toggle_stage: Arc<dyn Fn(usize)>,
    pub(crate) discard_change: Arc<dyn Fn(usize)>,
    pub(crate) stage_all: Arc<dyn Fn()>,
    pub(crate) unstage_all: Arc<dyn Fn()>,
    pub(crate) commit: Arc<dyn Fn()>,
    /// 跳过钩子提交（强确认）。
    pub(crate) commit_without_hooks: Arc<dyn Fn()>,
    /// 提交面板的三个开关：`amend` / `signoff` / `pushAfter`。
    pub(crate) set_commit_option: Arc<dyn Fn(&'static str, bool)>,
    /// 打开或关闭钩子完整日志弹层。
    pub(crate) toggle_hook_log: Arc<dyn Fn(bool)>,
    /// 切换 diff 的并排/合并视图。
    pub(crate) set_diff_split: Arc<dyn Fn(bool)>,
    /// 按 hunk / 单行暂存或取消暂存。
    pub(crate) apply_piece: Arc<dyn Fn(usize, Option<usize>, bool)>,
    /// 切换 diff 显示开关（`fold` / `wrap` / `whitespace`）。
    pub(crate) toggle_diff_option: Arc<dyn Fn(&'static str)>,
    /// 把当前 diff 导出成 patch 文件。
    pub(crate) export_patch: Arc<dyn Fn()>,
    /// 游离头时回到 main/master。
    pub(crate) checkout_default_branch: Arc<dyn Fn()>,
    /// 在当前标签页目录里初始化仓库。
    pub(crate) init_here: Arc<dyn Fn()>,
    /// 重新定位仓库（缺失仓库的 Locate…）。
    pub(crate) locate_repo: Arc<dyn Fn()>,
    /// 打开当前路径的上一级（不是仓库时的 Open parent）。
    pub(crate) open_parent: Arc<dyn Fn()>,
    /// 空仓库里创建第一个提交。
    pub(crate) create_initial_commit: Arc<dyn Fn()>,
    /// 清掉同步失败状态。
    pub(crate) clear_sync_fault: Arc<dyn Fn()>,
    /// 打开「改写提交说明」弹窗。
    pub(crate) open_reword: Arc<dyn Fn(String, String, String)>,
    /// 打开「合并提交」弹窗（from, to, 提交列表）。
    pub(crate) open_squash: Arc<dyn Fn(String, String, Vec<(String, String)>)>,
    /// 改写弹窗里编辑说明。
    pub(crate) set_rewrite_message: Arc<dyn Fn(String)>,
    /// 提交改写。
    pub(crate) submit_rewrite: Arc<dyn Fn()>,
    /// 关闭改写弹窗。
    pub(crate) close_rewrite: Arc<dyn Fn()>,
    /// 打开冲突视图。
    pub(crate) open_conflict: Arc<dyn Fn()>,
    /// 切换冲突文件。
    pub(crate) select_conflict: Arc<dyn Fn(usize)>,
    /// 前/后一个冲突文件。
    pub(crate) step_conflict: Arc<dyn Fn(bool)>,
    /// 用某一侧填充结果区。
    pub(crate) apply_side: Arc<dyn Fn(domain::conflict::Side)>,
    /// 标记当前冲突文件已解决。
    pub(crate) mark_resolved: Arc<dyn Fn()>,
    /// 继续被冲突打断的操作。
    pub(crate) continue_op: Arc<dyn Fn()>,
    /// 关闭冲突视图。
    pub(crate) close_conflict: Arc<dyn Fn()>,
    pub(crate) resolve_conflict: Arc<dyn Fn(String, bool)>,
    pub(crate) abort_operation: Arc<dyn Fn()>,
    pub(crate) run_command: Arc<dyn Fn(&'static str)>,
    pub(crate) set_theme: Arc<dyn Fn(ThemeMode)>,
    pub(crate) set_file_tree: Arc<dyn Fn(bool)>,
    pub(crate) go_settings: Arc<dyn Fn()>,
    pub(crate) undo_reflog: Arc<dyn Fn(usize)>,
    pub(crate) set_mode_refs: Arc<dyn Fn()>,
    pub(crate) set_mode_agent: Arc<dyn Fn()>,
    pub(crate) propose_plan: Arc<dyn Fn()>,
    pub(crate) run_agent_step: Arc<dyn Fn(usize)>,
    /// 删掉计划里的一步（计划编辑）。
    pub(crate) remove_agent_step: Arc<dyn Fn(usize)>,
    /// 往计划末尾补一步（参数是动作 id）。
    pub(crate) add_agent_step: Arc<dyn Fn(&'static str)>,
    /// 批准确认后按顺序执行计划里可执行的步骤。
    pub(crate) run_plan: Arc<dyn Fn()>,
    pub(crate) set_agent_permission: Arc<dyn Fn(agent::Action, agent::Permission)>,
    /// 设置 Agent 的自动化程度。
    pub(crate) set_automation: Arc<dyn Fn(agent::Automation)>,
    /// 设置自动执行前的预览时长。
    pub(crate) set_review_millis: Arc<dyn Fn(u32)>,
    /// 切换某个分支的受保护状态。
    pub(crate) toggle_protected_branch: Arc<dyn Fn(String)>,
    pub(crate) set_compare_refs: Arc<dyn Fn(Option<String>, Option<String>)>,
    pub(crate) checkout_branch: Arc<dyn Fn(String)>,
    pub(crate) delete_branch: Arc<dyn Fn(String)>,
    pub(crate) open_form: Arc<dyn Fn(screens::FormKind)>,
    pub(crate) open_sync: Arc<dyn Fn(model::Sync)>,
    /// 同步面板里的选项开关。
    pub(crate) set_sync_option: Arc<dyn Fn(&'static str, bool)>,
    pub(crate) run_sync: Arc<dyn Fn(model::Sync)>,
    pub(crate) close_sync: Arc<dyn Fn()>,
    pub(crate) drop_tag: Arc<dyn Fn(String)>,
    pub(crate) drop_remote: Arc<dyn Fn(String)>,
    pub(crate) drop_stash: Arc<dyn Fn(String)>,
    pub(crate) set_active_changelist: Arc<dyn Fn(Option<String>)>,
    pub(crate) move_to_changelist: Arc<dyn Fn(String, Option<String>)>,
    pub(crate) create_changelist: Arc<dyn Fn(String)>,
    pub(crate) checkout_new_branch: Arc<dyn Fn(String, String)>,
    pub(crate) merge_reference: Arc<dyn Fn(String)>,
    /// 切到某个提交（detached，先确认）。
    pub(crate) checkout_commit: Arc<dyn Fn(String)>,
    /// 拣选提交（先确认）。
    pub(crate) cherry_pick: Arc<dyn Fn(String)>,
    /// 反向提交（先确认）。
    pub(crate) revert_commit: Arc<dyn Fn(String)>,
    /// reflog 的「在历史中显示」：切到 History 并选中该提交。
    pub(crate) show_in_history: Arc<dyn Fn(String)>,
    /// 推送标签。
    pub(crate) push_tag: Arc<dyn Fn(String)>,
    /// 应用贮藏（保留条目）。
    pub(crate) apply_stash: Arc<dyn Fn(String)>,
    /// 弹出贮藏（取出并删除条目）。
    pub(crate) pop_stash: Arc<dyn Fn(String)>,
    /// 打开表单并把第二个字段预填（从某个提交新建分支/标签）。
    pub(crate) open_form_from: Arc<dyn Fn(screens::FormKind, String)>,
    pub(crate) reset_to: Arc<dyn Fn(String, git::ResetMode)>,
    pub(crate) apply_filter: Arc<dyn Fn()>,
    pub(crate) clear_filter: Arc<dyn Fn()>,
    pub(crate) clone_repository: Arc<dyn Fn()>,
    /// 取消正在进行的克隆。
    pub(crate) cancel_clone: Arc<dyn Fn()>,
    pub(crate) init_repository: Arc<dyn Fn()>,
    pub(crate) toggle_language: Arc<dyn Fn()>,
    pub(crate) refresh: Arc<dyn Fn()>,
    pub(crate) go_welcome: Arc<dyn Fn()>,
    pub(crate) go_repo_manager: Arc<dyn Fn()>,
    pub(crate) set_repo_form: Arc<dyn Fn(RepoForm)>,
    /// 复制文本到剪贴板（成功后状态栏提示）。
    pub(crate) copy_text: Arc<dyn Fn(String)>,
    /// 用系统默认程序打开路径。
    pub(crate) open_editor: Arc<dyn Fn(String)>,
    /// 在 Finder 中显示路径。
    pub(crate) reveal_path: Arc<dyn Fn(String)>,
    /// 用浏览器打开链接。
    pub(crate) open_browser: Arc<dyn Fn(String)>,
    /// 打开身份编辑表单（仓库级 user.name / user.email）。
    pub(crate) open_identity: Arc<dyn Fn()>,
    /// 清掉仓库级身份，回到全局配置。
    pub(crate) clear_identity: Arc<dyn Fn()>,
    /// 扫描已配置目录里的仓库并加入最近列表。
    pub(crate) add_found_repos: Arc<dyn Fn()>,
    /// 选择 Git 可执行文件。
    pub(crate) choose_git: Arc<dyn Fn()>,
    /// 自动探测 Git。
    pub(crate) detect_git: Arc<dyn Fn()>,
    /// 切换「使用 PATH 里的 git」。
    pub(crate) toggle_git_path: Arc<dyn Fn()>,
    /// 设置文件树的两项开关（`merge_empty_dirs` / `compact_single_child`）。
    pub(crate) set_tree_flag: Arc<dyn Fn(&'static str, bool)>,
    /// 设置启动方式：true = 恢复上次会话，false = 显示欢迎页。
    pub(crate) set_launch_restore: Arc<dyn Fn(bool)>,
    /// 切换日期显示方式。
    pub(crate) set_date_style: Arc<dyn Fn(crate::i18n::DateStyle)>,
    /// 设置自定义日期 pattern。
    pub(crate) set_date_pattern: Arc<dyn Fn(String)>,
    /// 设置更新检查方式（`auto` / `manual` / `never`）。
    pub(crate) set_update_mode: Arc<dyn Fn(String)>,
}

/// 应用根组件。
#[component]
pub fn App() -> impl IntoElement {
    let tokens = theme_overrides(THEME_JSON).expect("theme.json 可以解析");
    set_tokens(tokens);

    let mut title_bar = TitleBarOptions::default();
    title_bar.style = TitleBarStyle::Hidden;
    title_bar.height = Some(Points(44.0));
    let mut window = WindowAttributes::default();
    window.title = Some(brand::PRODUCT_NAME.to_string());
    window.title_bar = Some(title_bar);
    window.min_size = Some(Some(size(Points(1100.0), Points(700.0))));
    main_window().set(window);

    let store_dir = store::default_dir(brand::STORAGE_DIR);
    let language = signal(SharedString::from(
        store::load_language(&store_dir)
            .unwrap_or_else(|| i18n::DEFAULT_LANGUAGE.to_string()),
    ));

    // 历史筛选：关键字在工具栏搜索框，其余条件在 History 的筛选浮层。
    let history_query = signal(String::new());
    let filter_reference = signal(String::new());
    let filter_author = signal(String::new());
    let filter_when = signal(SharedString::from(When::All.id()));
    let filter_open = signal(false);
    let inspector_open = signal(false);
    let compare_mode = signal(false);
    // 验收开关：`GITUI_MODE=history|changes|refs|agent` 指定启动时的主视图，
    // 用于逐个巡检各页渲染（历史表曾整行不可见却连续几轮被截图判过）。
    let mode = signal(match std::env::var("GITUI_MODE").as_deref() {
        Ok("changes") => Mode::Changes,
        Ok("refs") => Mode::Refs,
        Ok("agent") => Mode::Agent,
        _ => Mode::History,
    });
    let commit_summary = signal(String::new());
    let commit_description = signal(String::new());
    let changelist_open = signal(false);
    // 验收开关：`GITUI_PALETTE=1` 启动即打开命令面板（⌘K 的等价物），
    // 用于截图核对面板与列表项。
    let palette_open = signal(
        std::env::var("GITUI_PALETTE")
            .map(|value| value == "1")
            .unwrap_or(false),
    );
    // 组件画廊（P0 的验收入口）：⌘⇧D 打开，Esc 关闭。P6 并入命令面板。
    // `GITUI_GALLERY=1` 用于截图验收：不依赖键盘自动化也能直接看到组件层。
    let gallery_open = signal(
        std::env::var("GITUI_GALLERY")
            .map(|value| value == "1")
            .unwrap_or(false),
    );
    let settings_state = signal(store::load_settings(&store_dir));
    {
        // 日期格式是全局显示偏好，装进 dates 模块供各视图直接读取。
        let loaded = settings_state.get_untracked();
        crate::dates::load(loaded.date_format.as_str(), loaded.date_pattern.as_str());
    }
    // 验收开关：`GITUI_FORM=1` 启动即打开「新建分支」表单，用于核对表单弹层。
    let form_state = signal(if std::env::var("GITUI_FORM").map(|v| v == "1").unwrap_or(false) {
        Some(screens::FormState::new(screens::FormKind::NewBranch))
    } else {
        None
    });
    let reset_request = signal(Option::<(String, git::ResetMode)>::None);
    // 状态栏右端的短提示：复制成功、外部打开失败等。
    let notice = signal(Option::<String>::None);
    // diff 的并排/合并切换（设计板 04）。
    let diff_split = signal(false);
    // diff 的三个显示开关（设计板 04）。
    let diff_options = signal(crate::workspace::diff::DiffOptions::default());
    // 历史改写弹窗（改写说明 / 合并提交）。
    let rewrite_state = signal(Option::<screens::RewriteState>::None);
    // 命令面板的高亮行。
    let palette_selected = signal(0_usize);
    // 快捷键：注册表、录制中的动作、设置页的搜索词与改键错误。
    let shortcut_registry = signal(shortcuts::Registry::with_overrides(&store::load_shortcuts(
        &store_dir,
    )));
    let recording_shortcut = signal(Option::<&'static str>::None);
    let shortcut_query = signal(String::new());
    let shortcut_error = signal(Option::<String>::None);
    // reflog 的过滤关键字（动作或说明）。
    let reflog_query = signal(String::new());
    // 冲突视图：可编辑的结果区与 base 显隐。
    let conflict_result = signal(String::new());
    let conflict_base_open = signal(false);
    // 验收开关：`GITUI_SYNC=1` 启动即打开同步面板，用于截图核对弹层外观
    // （遮罩深浅、圆角、阴影、关闭行为）。与 GITUI_GALLERY 同思路。
    let sync_state = signal(if std::env::var("GITUI_SYNC").map(|v| v == "1").unwrap_or(false) {
        Some(screens::SyncDialogState::new(crate::model::Sync::Fetch))
    } else {
        None
    });
    let agent_plan = signal(Option::<agent::Plan>::None);
    let agent_policies = signal({
        let mut policies = agent::Policies::default();
        for (action, permission) in store::load_agent_policies(&store_dir) {
            let matched = agent::Action::ALL
                .into_iter()
                .find(|candidate| candidate.id() == action);
            if let Some(action) = matched {
                policies.set(action, agent::Permission::from_id(permission.as_str()));
            }
        }
        policies
    });
    let agent_settings = signal({
        let mut settings = agent::Settings::default();
        for (key, value) in store::load_agent_policies(&store_dir) {
            settings.read_pair(key.as_str(), value.as_str());
        }
        settings
    });
    let agent_audit = signal(
        store::load_agent_audit(&store_dir)
            .into_iter()
            .filter_map(|line| {
                let mut fields = line.split('\t');
                let timestamp = fields.next()?.parse().ok()?;
                let action = fields.next()?.to_string();
                let ok = fields.next()? == "ok";
                let detail = fields.next().unwrap_or("").to_string();
                return Some(agent::AuditEntry {
                    timestamp,
                    action,
                    ok,
                    detail,
                });
            })
            .collect::<Vec<_>>(),
    );
    let settings_section = signal(screens::SettingsSection::General);
    let palette_query = signal(String::new());
    let theme_mode = signal(ThemeMode::from_id(
        store::load_appearance(&store_dir)
            .unwrap_or_else(|| ThemeMode::System.id().to_string())
            .as_str(),
    ));
    nsh_core::element::set_color_scheme(theme_mode.get_untracked().appearance());
    let now = store::now_unix();

    let session = store::load_session(&store_dir);
    let initial_settings = settings_state.get_untracked();
    let initial_filter = LogFilter::default();
    let mut initial_tabs: Vec<Tab> = session
        .paths
        .iter()
        .map(|path| model::load_tab(path, &initial_filter))
        // 打不开的仓库仍保留标签页：横幅给出「重新定位 / 在此初始化」，而不是
        // 让它在启动时悄悄消失（手册 TC-01）。
        .filter(|tab| tab.error.is_none() || tab.problem != model::RepoProblem::None)
        .collect();
    if !initial_settings.restore_session {
        initial_tabs.clear();
    }
    // 验收与命令行入口：`GITUI_REPO=/path/to/repo` 直接打开指定仓库，
    // 优先于会话恢复。截图验收需要它——没有打开的仓库，历史表与变更页
    // 根本不会渲染，截图只能看到欢迎页。
    if let Ok(path) = std::env::var("GITUI_REPO") {
        let path = path.trim().to_string();
        if !path.is_empty() {
            initial_tabs = vec![model::load_tab(&path, &initial_filter)];
        }
    }
    // 验收开关：`GITUI_SETTINGS=1` 启动即进入设置页，用于截图核对。
    let screen = signal(if std::env::var("GITUI_SETTINGS")
        .map(|value| value == "1")
        .unwrap_or(false)
    {
        Screen::Settings
    } else {
        Screen::Workspace
    });
    // 欢迎页是独立窗口：没有可恢复的标签页时打开它。
    // 验收开关：`GITUI_NO_WELCOME=1` 时不打开欢迎窗口。
    // 欢迎窗口是独立窗口且默认铺在主窗口之上，截图验收组件画廊时它会挡住画廊，
    // 之前把「截到欢迎页的空白」误当成「画廊没渲染」，就是因为它。
    let suppress_welcome = std::env::var("GITUI_NO_WELCOME")
        .map(|value| value == "1")
        .unwrap_or(false);
    let welcome_open = signal(initial_tabs.is_empty() && !suppress_welcome);
    let active = signal(session.active.min(initial_tabs.len().saturating_sub(1)));
    if initial_tabs.is_empty() {
        initial_tabs.clear();
    }
    let tabs = signal(initial_tabs);
    let recent = signal(store::load_recent(&store_dir));

    // 欢迎页与仓库管理页的输入框。
    let welcome_query = signal(String::new());
    let clone_url = signal(String::new());
    let clone_destination = signal(String::new());
    // Clone 的额外选项（设计板 03）合成一个结构，避免把四个信号串到窗口里。
    let clone_options = signal(screens::CloneOptions::default());
    // Init 的附带文件（设计板 03）。
    let init_files = signal(screens::InitFiles::default());
    let clone_error = signal(Option::<String>::None);
    let cloning = signal(false);
    // 克隆进度（git 的 stderr 最后一行）与取消标志。
    let clone_progress = signal(String::new());
    let clone_cancel: std::sync::Arc<std::sync::atomic::AtomicBool> =
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let init_path = signal(String::new());
    let repo_form = signal(RepoForm::Open);

    let current_filter: Arc<dyn Fn() -> LogFilter> = {
        let history_query = history_query;
        let filter_reference = filter_reference;
        let filter_author = filter_author;
        let filter_when = filter_when;
        Arc::new(move || {
            return filters::build(
                history_query.get_untracked().as_str(),
                filter_reference.get_untracked().as_str(),
                filter_author.get_untracked().as_str(),
                When::from_key(filter_when.get_untracked().as_str()),
                now,
            );
        })
    };

    let open_path: Arc<dyn Fn(String)> = {
        let welcome_open = welcome_open;
        let tabs = tabs;
        let active = active;
        let recent = recent;
        let screen = screen;
        let store_dir = store_dir.clone();
        let current_filter = current_filter.clone();
        Arc::new(move |path: String| {
            // 先插入一个「加载中」的占位标签页，再在后台读仓库：
            // 大仓库的 `git log`/`status` 不再卡住界面，用户也能看到骨架。
            let filter = current_filter();
            let mut list = tabs.get_untracked();
            let existing = list
                .iter()
                .position(|candidate| candidate.path == path.as_str());
            match existing {
                Some(index) => {
                    list[index].loading = true;
                    active.set(index);
                }
                None => {
                    list.push(model::placeholder_tab(path.as_str(), &filter));
                    active.set(list.len() - 1);
                }
            }
            tabs.set(list);
            store::remember_recent(&store_dir, &recent.get_untracked(), path.as_str());
            recent.set(store::load_recent(&store_dir));
            screen.set(Screen::Workspace);
            // 从欢迎页进入主窗口：关掉欢迎窗口并聚焦主窗口。
            welcome_open.set(false);
            main_window().focus();
            persist(&store_dir, active, tabs);

            let path_for_load = path.clone();
            spawn(async move {
                let loaded = model::load_tab(path_for_load.as_str(), &filter);
                let mut list = tabs.get_untracked();
                // 加载期间用户可能关掉了这个标签页，找不到就丢弃结果。
                let Some(index) = list
                    .iter()
                    .position(|candidate| candidate.path == loaded.path)
                else {
                    return;
                };
                list[index] = loaded;
                tabs.set(list);
            });
        })
    };

    let choose_folder: Arc<dyn Fn()> = {
        let open_path = open_path.clone();
        Arc::new(move || {
            let answer = pick_paths(nsh_core::file_dialog::OpenDialog::folders());
            let open_path = open_path.clone();
            spawn(async move {
                if let Ok(Some(paths)) = answer.await {
                    if let Some(path) = paths.first() {
                        open_path(path.display().to_string());
                    }
                }
            });
        })
    };

    let close_tab: Arc<dyn Fn(usize)> = {
        let welcome_open = welcome_open;
        let tabs = tabs;
        let active = active;
        let screen = screen;
        let store_dir = store_dir.clone();
        let language = language;
        // 关闭动作本身；有未提交改动时先确认再走这里。
        let perform: Arc<dyn Fn(usize)> = {
            let welcome_open = welcome_open;
            let tabs = tabs;
            let active = active;
            let store_dir = store_dir.clone();
            Arc::new(move |index: usize| {
                let mut list = tabs.get_untracked();
                if index >= list.len() {
                    return;
                }
                list.remove(index);
                let next_active = if list.is_empty() {
                    0
                } else {
                    active.get_untracked().min(list.len() - 1)
                };
                tabs.set(list.clone());
                active.set(next_active);
                if list.is_empty() {
                    welcome_open.set(true);
                }
                persist(&store_dir, active, tabs);
            })
        };
        let perform = perform;
        Arc::new(move |index: usize| {
            let list = tabs.get_untracked();
            let Some(tab) = list.get(index) else {
                return;
            };
            let changes = tab.status.len();
            let name = tab.label();
            if changes == 0 {
                perform(index);
                return;
            }
            let language_now = language.get_untracked();
            let pending = interactions::confirm(
                i18n::with_arguments(
                    language_now.as_str(),
                    "confirm.closeTitle",
                    &[("name", name)],
                ),
                i18n::with_arguments(
                    language_now.as_str(),
                    "confirm.closeMessage",
                    &[("count", changes.to_string())],
                ),
                "close",
                i18n::t(language_now.as_str(), "confirm.closeAction"),
                false,
            );
            let perform = perform.clone();
            spawn(async move {
                if pending.await {
                    perform(index);
                }
            });
        })
    };

    let activate_tab: Arc<dyn Fn(usize)> = {
        let active = active;
        let tabs = tabs;
        let store_dir = store_dir.clone();
        Arc::new(move |index: usize| {
            if index < tabs.get_untracked().len() {
                active.set(index);
                persist(&store_dir, active, tabs);
            }
        })
    };

    let select_commit: Arc<dyn Fn(usize)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |index: usize| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::select_commit(tab, index);
                tabs.set(list);
            }
        })
    };

    let toggle_compare: Arc<dyn Fn(usize)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |index: usize| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::toggle_compare(tab, index);
                tabs.set(list);
            }
        })
    };

    let set_compare_mode: Arc<dyn Fn(bool)> = {
        let tabs = tabs;
        let active = active;
        let compare_mode = compare_mode;
        Arc::new(move |enabled: bool| {
            compare_mode.set(enabled);
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                tab.comparing = enabled;
                if !enabled {
                    tab.compare.clear();
                }
                tabs.set(list);
            }
        })
    };

    let select_file: Arc<dyn Fn(usize)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |index: usize| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::select_file(tab, index);
                tabs.set(list);
            }
        })
    };

    let reload: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        let current_filter = current_filter.clone();
        Arc::new(move || {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get(current).cloned() {
                list[current] = model::reload_with_filter(&tab, &current_filter());
                tabs.set(list);
            }
        })
    };

    let apply_filter: Arc<dyn Fn()> = {
        let reload = reload.clone();
        let filter_open = filter_open;
        Arc::new(move || {
            filter_open.set(false);
            reload();
        })
    };

    let clear_filter: Arc<dyn Fn()> = {
        let history_query = history_query;
        let filter_reference = filter_reference;
        let filter_author = filter_author;
        let filter_when = filter_when;
        let reload = reload.clone();
        Arc::new(move || {
            history_query.set(String::new());
            filter_reference.set(String::new());
            filter_author.set(String::new());
            filter_when.set(SharedString::from(When::All.id()));
            reload();
        })
    };

    let refresh: Arc<dyn Fn()> = {
        let reload = reload.clone();
        Arc::new(move || reload())
    };

    let clone_repository: Arc<dyn Fn()> = {
        let clone_url = clone_url;
        let clone_destination = clone_destination;
        let clone_options = clone_options;
        let store_dir = store_dir.clone();
        let clone_error = clone_error;
        let cloning = cloning;
        let clone_progress = clone_progress;
        let clone_cancel = clone_cancel.clone();
        let language = language;
        let open_path = open_path.clone();
        Arc::new(move || {
            let url = clone_url.get_untracked();
            let destination = clone_destination.get_untracked();
            let chosen = clone_options.get_untracked();
            let options = git::CloneOptions {
                branch: Some(chosen.branch.clone()).filter(|b| !b.trim().is_empty()),
                depth: chosen.depth,
                recurse_submodules: chosen.recurse_submodules,
            };
            if url.trim().is_empty() {
                clone_error.set(Some(i18n::t(language.get_untracked().as_str(), "welcome.missingUrl")));
                return;
            }
            if destination.trim().is_empty() {
                clone_error.set(Some(i18n::t(language.get_untracked().as_str(), "welcome.missingDestination")));
                return;
            }
            // 「克隆后打开」关闭时只记入最近仓库，不切走当前工作区。
            let open_after = chosen.open_after_clone;
            cloning.set(true);
            clone_progress.set(String::new());
            clone_cancel.store(false, std::sync::atomic::Ordering::Relaxed);
            clone_error.set(None);
            let open_path = open_path.clone();
            let recent = recent;
            let store_dir = store_dir.clone();
            let clone_error = clone_error;
            let cloning = cloning;
            let clone_progress = clone_progress;
            let cancel = clone_cancel.clone();
            spawn(async move {
                // 启动后按 100ms 轮询：界面一直可响应，取消也能立刻生效。
                let mut run = match git::start_clone(
                    url.trim(),
                    destination.trim(),
                    &options,
                ) {
                    Ok(run) => run,
                    Err(error) => {
                        cloning.set(false);
                        clone_error.set(Some(error.detail()));
                        return;
                    }
                };
                let result = loop {
                    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                        let _ = run.cancel();
                        let lang = language.get_untracked();
                        break Err(i18n::t(lang.as_str(), "welcome.cloneCancelled"));
                    }
                    if let Some(outcome) = run.try_finish() {
                        break outcome.map_err(|error| error.detail());
                    }
                    let progress = run.progress();
                    if !progress.is_empty() {
                        clone_progress.set(progress);
                    }
                    timer(std::time::Duration::from_millis(100)).await;
                };
                cloning.set(false);
                clone_progress.set(String::new());
                match result {
                    Ok(()) => {
                        if open_after {
                            open_path(destination.trim().to_string());
                        } else {
                            let path = destination.trim().to_string();
                            let list = recent.get_untracked();
                            // 复用既有的「记住最近仓库」入口，顺序与去重规则一致。
                            store::remember_recent(&store_dir, &list, path.as_str());
                            recent.set(store::load_recent(&store_dir));
                        }
                    }
                    Err(detail) => clone_error.set(Some(detail)),
                }
            });
        })
    };

    let cancel_clone: Arc<dyn Fn()> = {
        let cancel = clone_cancel.clone();
        Arc::new(move || cancel.store(true, std::sync::atomic::Ordering::Relaxed))
    };

    let init_repository: Arc<dyn Fn()> = {
        let init_path = init_path;
        let clone_error = clone_error;
        let init_files = init_files;
        let open_path = open_path.clone();
        Arc::new(move || {
            let path = init_path.get_untracked();
            if path.trim().is_empty() {
                clone_error.set(Some(i18n::t(language.get_untracked().as_str(), "welcome.missingDirectory")));
                return;
            }
            let files = init_files.get_untracked();
            let options = git::InitOptions {
                gitignore: files.gitignore,
                readme: files.readme,
            };
            match git::init_with(path.trim(), &options) {
                Ok(()) => open_path(path.trim().to_string()),
                Err(error) => clone_error.set(Some(error.detail())),
            }
        })
    };

    let toggle_language: Arc<dyn Fn()> = {
        let language = language;
        let store_dir = store_dir.clone();
        Arc::new(move || {
            let next = i18n::toggle(language.get_untracked().as_str());
            language.set(SharedString::from(next));
            store::save_language(&store_dir, next);
        })
    };

    let go_welcome: Arc<dyn Fn()> = {
        let welcome_open = welcome_open;
        let screen = screen;
        Arc::new(move || {
            screen.set(Screen::Workspace);
            welcome_open.set(true);
        })
    };

    let go_repo_manager: Arc<dyn Fn()> = {
        let go_welcome = go_welcome.clone();
        Arc::new(move || go_welcome())
    };

    let go_settings: Arc<dyn Fn()> = {
        let screen = screen;
        Arc::new(move || screen.set(Screen::Settings))
    };

    let set_repo_form: Arc<dyn Fn(RepoForm)> = {
        let repo_form = repo_form;
        Arc::new(move |form: RepoForm| repo_form.set(form))
    };

    let set_mode: Arc<dyn Fn(Mode)> = {
        let mode = mode;
        Arc::new(move |next: Mode| mode.set(next))
    };

    let select_change: Arc<dyn Fn(usize)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |index: usize| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::load_change_diff(tab, index);
                tabs.set(list);
            }
        })
    };

    let toggle_stage: Arc<dyn Fn(usize)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |index: usize| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::toggle_stage(tab, index);
                tabs.set(list);
            }
        })
    };

    // 丢弃不可撤销：先弹系统确认框，确认后才动工作区。
    let discard_change: Arc<dyn Fn(usize)> = {
        let tabs = tabs;
        let active = active;
        let language = language;
        Arc::new(move |index: usize| {
            let list = tabs.get_untracked();
            let current = active.get_untracked();
            let Some(tab) = list.get(current).cloned() else {
                return;
            };
            let Some(entry) = tab.changes.get(index).cloned() else {
                return;
            };
            let language_now = language.get_untracked();
            let pending = crate::ui::dialog::confirm_destructive(
                i18n::t(language_now.as_str(), "changes.discardTitle"),
                i18n::with_arguments(
                    language_now.as_str(),
                    "changes.discardMessage",
                    &[("path", entry.path.clone())],
                ),
                i18n::t(language_now.as_str(), "changes.discardAction"),
                i18n::t(language_now.as_str(), "common.cancel"),
            );
            let tabs = tabs;
            let active = active;
            spawn(async move {
                if let Ok(result) = pending.await {
                    if result.action.as_deref() != Some("discard") {
                        return;
                    }
                    let current = active.get_untracked();
                    let mut list = tabs.get_untracked();
                    if let Some(tab) = list.get_mut(current) {
                        model::discard_change(tab, index);
                        tabs.set(list);
                    }
                }
            });
        })
    };

    let stage_all: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move || {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::stage_all_or_none(tab, true);
                tabs.set(list);
            }
        })
    };

    let unstage_all: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move || {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::stage_all_or_none(tab, false);
                tabs.set(list);
            }
        })
    };

    // 提交：成功后按「Push after commit」决定是否立刻推送。
    let commit_inner: Arc<dyn Fn(bool)> = {
        let tabs = tabs;
        let active = active;
        let commit_summary = commit_summary;
        let commit_description = commit_description;
        let notice = notice;
        let language = language;
        Arc::new(move |no_verify: bool| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                tab.summary = commit_summary.get_untracked();
                tab.description = commit_description.get_untracked();
                model::commit_staged_with(tab, no_verify);
                if tab.hook_ok {
                    commit_summary.set(String::new());
                    commit_description.set(String::new());
                }
                let push_after = tab.push_after_commit && tab.hook_ok;
                if push_after {
                    tab.push_after_commit = false;
                }
                tabs.set(list.clone());
                if push_after {
                    let mut list = list;
                    if let Some(tab) = list.get_mut(current) {
                        model::sync(tab, model::Sync::Push);
                        tabs.set(list);
                    }
                    interactions::notify(
                        notice,
                        i18n::t(language.get_untracked().as_str(), "commit.pushed"),
                    );
                }
            }
        })
    };

    let commit: Arc<dyn Fn()> = {
        let commit_inner = commit_inner.clone();
        Arc::new(move || commit_inner(false))
    };

    // 跳过钩子：先强确认再提交（`--no-verify`）。
    let commit_without_hooks: Arc<dyn Fn()> = {
        let commit_inner = commit_inner.clone();
        let language = language;
        Arc::new(move || {
            let language_now = language.get_untracked();
            let pending = interactions::confirm(
                i18n::t(language_now.as_str(), "confirm.noVerifyTitle"),
                i18n::t(language_now.as_str(), "confirm.noVerifyMessage"),
                "no_verify",
                i18n::t(language_now.as_str(), "confirm.noVerifyAction"),
                true,
            );
            let commit_inner = commit_inner.clone();
            spawn(async move {
                if pending.await {
                    commit_inner(true);
                }
            });
        })
    };

    // 面板上的三个开关（设计板 10）。
    let set_commit_option: Arc<dyn Fn(&'static str, bool)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |kind: &'static str, value: bool| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                match kind {
                    "amend" => tab.amend = value,
                    "signoff" => tab.signoff = value,
                    "pushAfter" => tab.push_after_commit = value,
                    _ => return,
                }
                tabs.set(list);
            }
        })
    };

    // 钩子完整日志弹层。
    let toggle_hook_log: Arc<dyn Fn(bool)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |open: bool| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                tab.hook_open = open;
                tabs.set(list);
            }
        })
    };

    let resolve_conflict: Arc<dyn Fn(String, bool)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |path: String, ours: bool| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::resolve_conflict(tab, path.as_str(), ours);
                tabs.set(list);
            }
        })
    };

    // 同步动作：先在对话框里确认，再执行并留在对话框展示结果。
    // 打开同步面板；已经打开时只切 tab，保留当前选项。
    let open_sync: Arc<dyn Fn(model::Sync)> = {
        let sync_state = sync_state;
        Arc::new(move |kind: model::Sync| {
            let mut state = sync_state
                .get_untracked()
                .unwrap_or_else(|| screens::SyncDialogState::new(kind));
            state.kind = kind;
            state.result = None;
            sync_state.set(Some(state));
        })
    };

    // 选项开关：键名固定，策略三项互斥。
    let set_sync_option: Arc<dyn Fn(&'static str, bool)> = {
        let sync_state = sync_state;
        Arc::new(move |key: &'static str, value: bool| {
            let Some(mut state) = sync_state.get_untracked() else {
                return;
            };
            match key {
                "prune" => state.options.fetch.prune = value,
                "fetchTags" => state.options.fetch.tags = value,
                "allRemotes" => state.options.fetch.all_remotes = value,
                "autostash" => state.options.pull.autostash = value,
                "rebaseLocal" => state.options.pull.rebase_local = value,
                "pushTags" => state.options.push.tags = value,
                "forceLease" => state.options.push.force_with_lease = value,
                "allBranches" => state.options.push.all_branches = value,
                "strategyMerge" => state.options.pull.strategy = git::PullStrategy::Merge,
                "strategyRebase" => state.options.pull.strategy = git::PullStrategy::Rebase,
                "strategyFfOnly" => state.options.pull.strategy = git::PullStrategy::FfOnly,
                _ => return,
            }
            sync_state.set(Some(state));
        })
    };

    let run_sync: Arc<dyn Fn(model::Sync)> = {
        let tabs = tabs;
        let active = active;
        let sync_state = sync_state;
        Arc::new(move |kind: model::Sync| {
            let Some(state) = sync_state.get_untracked() else {
                return;
            };
            let options = state.options.clone();
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::sync_with(tab, kind, &options);
                let result = match (&tab.sync_message, tab.sync_failed) {
                    (Some(message), false) => Ok(message.clone()),
                    (Some(message), true) => Err(message.clone()),
                    (None, _) => Ok(String::new()),
                };
                let rejected = kind == model::Sync::Push
                    && tab.sync_failed
                    && tab.sync_fault == Some(domain::NetworkFault::Rejected);
                tabs.set(list);
                sync_state.set(Some(screens::SyncDialogState {
                    kind,
                    options: options.clone(),
                    result: Some(result),
                    running: false,
                }));
                if !rejected {
                    return;
                }
                // 远端已前进：不自动强推，让用户在「拉取变基 / 拉取合并 / 取消」里选。
                let lang = language.get_untracked();
                let title = i18n::t(lang.as_str(), "confirm.nonFastForwardTitle");
                let message = i18n::t(lang.as_str(), "confirm.nonFastForwardMessage");
                let rebase_label = i18n::t(lang.as_str(), "confirm.nonFastForwardRebase");
                let merge_label = i18n::t(lang.as_str(), "confirm.nonFastForwardMerge");
                let cancel_label = i18n::t(lang.as_str(), "common.cancel");
                let pending = nsh_base::alert_dialog()
                    .title(title)
                    .description(message)
                    .action("rebase", rebase_label)
                    .action("merge", merge_label)
                    .cancel("cancel", cancel_label)
                    .show();
                let language = language;
                let notice = notice;
                spawn(async move {
                    let Ok(result) = pending.await else {
                        return;
                    };
                    let Some(action) = result.action else {
                        return;
                    };
                    if action.as_str() == "cancel" {
                        return;
                    }
                    let strategy = if action.as_str() == "rebase" {
                        git::PullStrategy::Rebase
                    } else {
                        git::PullStrategy::Merge
                    };
                    let pull_options = model::SyncOptions {
                        pull: git::PullOptions {
                            strategy,
                            ..options.pull.clone()
                        },
                        ..options.clone()
                    };
                    let mut list = tabs.get_untracked();
                    if let Some(tab) = list.get_mut(active.get_untracked()) {
                        model::sync_with(tab, model::Sync::Pull, &pull_options);
                        let pulled = !tab.sync_failed;
                        // 拉取成功后再推一次；失败就只报拉取结果，不再连环弹框。
                        if pulled {
                            model::sync_with(tab, model::Sync::Push, &options);
                        }
                        let outcome = match (&tab.sync_message, tab.sync_failed) {
                            (Some(message), false) => Some((message.clone(), false)),
                            (Some(message), true) => Some((message.clone(), true)),
                            (None, _) => None,
                        };
                        tabs.set(list);
                        if let Some((message, failed)) = outcome {
                            if failed {
                                interactions::notify(notice, message);
                            } else {
                                let lang = language.get_untracked();
                                interactions::notify(
                                    notice,
                                    i18n::t(lang.as_str(), "toast.pushed"),
                                );
                            }
                        }
                    }
                });
            }
        })
    };

    let close_sync: Arc<dyn Fn()> = {
        let sync_state = sync_state;
        Arc::new(move || sync_state.set(None))
    };

    let resolve_conflict: Arc<dyn Fn(String, bool)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |path: String, ours: bool| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::resolve_conflict(tab, path.as_str(), ours);
                tabs.set(list);
            }
        })
    };

    let close_sync: Arc<dyn Fn()> = {
        let sync_state = sync_state;
        Arc::new(move || sync_state.set(None))
    };

    let sync: Arc<dyn Fn(model::Sync)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |kind: model::Sync| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::sync(tab, kind);
                tabs.set(list);
            }
        })
    };

    let abort_operation: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        let language = language;
        Arc::new(move || {
            let language_now = language.get_untracked();
            let operation = tabs
                .get_untracked()
                .get(active.get_untracked())
                .map(|tab| tab.operation.id().to_string())
                .unwrap_or_default();
            let pending = interactions::confirm(
                i18n::with_arguments(
                    language_now.as_str(),
                    "confirm.abortTitle",
                    &[("operation", operation)],
                ),
                i18n::t(language_now.as_str(), "confirm.abortMessage"),
                "abort",
                i18n::t(language_now.as_str(), "confirm.abortAction"),
                true,
            );
            spawn(async move {
                if !pending.await {
                    return;
                }
                let current = active.get_untracked();
                let mut list = tabs.get_untracked();
                if let Some(tab) = list.get_mut(current) {
                    model::abort_operation(tab);
                    tabs.set(list);
                }
            });
        })
    };

    // 命令面板与快捷键共用同一个分发器：命令 id 就是快捷键注册表里的动作 id。
    let run_command: Arc<dyn Fn(&'static str)> = {
        let open_repo = choose_folder.clone();
        let close_tab = close_tab.clone();
        let refresh = refresh.clone();
        let set_mode_cmd = set_mode.clone();
        let commit_cmd = commit.clone();
        let inspector_open = inspector_open;
        let compare_mode = compare_mode;
        let open_sync = open_sync.clone();
        let go_welcome = go_welcome.clone();
        let go_repo_manager = go_repo_manager.clone();
        let go_settings = go_settings.clone();
        let toggle_language = toggle_language.clone();
        let palette_open = palette_open;
        let palette_selected = palette_selected;
        let theme_mode = theme_mode;
        let store_dir = store_dir.clone();
        let filter_open = filter_open;
        let mode_signal = mode;
        let tabs_cmd = tabs;
        let active_cmd = active;
        let screen_cmd = screen;
        Arc::new(move |id: &'static str| {
            match id {
                "open" => open_repo(),
                "openPalette" => {
                    // 打开时把高亮拉回第一行。
                    palette_selected.set(0);
                    palette_open.set(true);
                }
                "closeTab" => close_tab(active_cmd.get_untracked()),
                "refresh" => refresh(),
                "history" => set_mode_cmd(Mode::History),
                "changes" => set_mode_cmd(Mode::Changes),
                "inspector" => inspector_open.set(!inspector_open.get_untracked()),
                "compare" => compare_mode.set(!compare_mode.get_untracked()),
                "fetch" => open_sync(model::Sync::Fetch),
                "pull" => open_sync(model::Sync::Pull),
                "push" => open_sync(model::Sync::Push),
                // ⌘F 搜索与 ⇧⌘F 过滤共用历史视图的过滤浮层（手册 1.3）。
                "filter" | "filterHistory" => {
                    set_mode_cmd(Mode::History);
                    filter_open.set(true);
                }
                // ⌘↵：只在 Changes 里提交，避免在历史视图误提交。
                "primary" => {
                    if mode_signal.get_untracked() == Mode::Changes {
                        commit_cmd();
                    }
                }
                "prevTab" => cycle_tab(tabs_cmd, active_cmd, screen_cmd, false),
                "nextTab" => cycle_tab(tabs_cmd, active_cmd, screen_cmd, true),
                "tab1" => activate_slot(tabs_cmd, active_cmd, screen_cmd, 0),
                "tab2" => activate_slot(tabs_cmd, active_cmd, screen_cmd, 1),
                "tab3" => activate_slot(tabs_cmd, active_cmd, screen_cmd, 2),
                "tab4" => activate_slot(tabs_cmd, active_cmd, screen_cmd, 3),
                "tab5" => activate_slot(tabs_cmd, active_cmd, screen_cmd, 4),
                "tab6" => activate_slot(tabs_cmd, active_cmd, screen_cmd, 5),
                "tab7" => activate_slot(tabs_cmd, active_cmd, screen_cmd, 6),
                "tab8" => activate_slot(tabs_cmd, active_cmd, screen_cmd, 7),
                "tab9" => activate_slot(tabs_cmd, active_cmd, screen_cmd, 8),
                "welcome" => go_welcome(),
                "repoManager" => go_repo_manager(),
                "settings" => go_settings(),
                "toggleLanguage" => toggle_language(),
                "toggleTheme" => {
                    let next = theme_mode.get_untracked().next();
                    theme_mode.set(next);
                    nsh_core::element::set_color_scheme(next.appearance());
                    store::save_appearance(&store_dir, next.id());
                }
                _ => {}
            }
        })
    };

    // 快捷键改键：写入注册表并持久化；冲突/不可绑定时把原因交给设置页显示。
    let rebind_shortcut: Arc<dyn Fn(&'static str, String)> = {
        let registry = shortcut_registry;
        let recording = recording_shortcut;
        let error = shortcut_error;
        let store_dir = store_dir.clone();
        let language = language;
        Arc::new(move |id: &'static str, combo: String| {
            let mut next = registry.get_untracked();
            let outcome = next.rebind(id, combo.as_str());
            let lang = language.get_untracked();
            match outcome {
                Ok(()) => {
                    registry.set(next.clone());
                    store::save_shortcuts(&store_dir, &next.overrides());
                    error.set(None);
                    recording.set(None);
                }
                // 失败时保持录制状态，用户可以直接改按另一个组合。
                Err(shortcuts::BindError::NotBindable) => {
                    error.set(Some(i18n::t(lang.as_str(), "shortcut.notBindable")));
                }
                Err(shortcuts::BindError::Conflict(other)) => {
                    let action = shortcuts::ACTIONS
                        .iter()
                        .find(|candidate| candidate.id == other);
                    let name = action
                        .map(|action| shortcuts::label(action, lang.as_str()))
                        .unwrap_or_else(|| other.to_string());
                    error.set(Some(i18n::with_arguments(
                        lang.as_str(),
                        "shortcut.conflict",
                        &[("action", name)],
                    )));
                }
            }
        })
    };

    let record_shortcut: Arc<dyn Fn(&'static str)> = {
        let recording = recording_shortcut;
        let error = shortcut_error;
        Arc::new(move |id: &'static str| {
            error.set(None);
            recording.set(Some(id));
        })
    };

    // 恢复默认：先确认（会覆盖用户的自定义），再重置并清空持久化。
    let restore_shortcuts: Arc<dyn Fn()> = {
        let registry = shortcut_registry;
        let recording = recording_shortcut;
        let error = shortcut_error;
        let store_dir = store_dir.clone();
        let notice = notice;
        let language = language;
        Arc::new(move || {
            let lang = language.get_untracked();
            let title = i18n::t(lang.as_str(), "confirm.restoreShortcutsTitle");
            let message = i18n::t(lang.as_str(), "confirm.restoreShortcutsMessage");
            let action_label = i18n::t(lang.as_str(), "confirm.restoreShortcutsAction");
            let done = i18n::t(lang.as_str(), "shortcut.restored");
            let registry = registry;
            let recording = recording;
            let error = error;
            let store_dir = store_dir.clone();
            let notice = notice;
            spawn(async move {
                let ok = interactions::confirm(
                    title,
                    message,
                    "restoreShortcuts",
                    action_label,
                    false,
                )
                .await;
                if !ok {
                    return;
                }
                let mut next = registry.get_untracked();
                next.restore_defaults();
                registry.set(next.clone());
                store::save_shortcuts(&store_dir, &next.overrides());
                recording.set(None);
                error.set(None);
                interactions::notify(notice, done);
            });
        })
    };

    let set_theme: Arc<dyn Fn(ThemeMode)> = {        let theme_mode = theme_mode;
        let store_dir = store_dir.clone();
        Arc::new(move |mode: ThemeMode| {
            theme_mode.set(mode);
            nsh_core::element::set_color_scheme(mode.appearance());
            store::save_appearance(&store_dir, mode.id());
        })
    };

    let set_file_tree: Arc<dyn Fn(bool)> = {
        let settings_state = settings_state;
        let store_dir = store_dir.clone();
        Arc::new(move |enabled: bool| {
            let mut next = settings_state.get_untracked();
            next.file_tree = enabled;
            settings_state.set(next.clone());
            store::save_settings(&store_dir, &next);
        })
    };


    let undo_reflog: Arc<dyn Fn(usize)> = {
        let tabs = tabs;
        let active = active;
        let language = language;
        Arc::new(move |index: usize| {
            let list = tabs.get_untracked();
            let current = active.get_untracked();
            let Some(tab) = list.get(current).cloned() else {
                return;
            };
            let Some(entry) = tab.reflog.get(index).cloned() else {
                return;
            };
            let language_now = language.get_untracked();
            let pending = crate::ui::dialog::confirm_destructive(
                i18n::t(language_now.as_str(), "reflog.undoTitle"),
                i18n::with_arguments(
                    language_now.as_str(),
                    "reflog.undoMessage",
                    &[("short", entry.short.clone())],
                ),
                i18n::t(language_now.as_str(), "reflog.undoAction"),
                i18n::t(language_now.as_str(), "common.cancel"),
            );
            let tabs = tabs;
            spawn(async move {
                if let Ok(result) = pending.await {
                    if result.action.as_deref() != Some("undo") {
                        return;
                    }
                    let current = active.get_untracked();
                    let mut list = tabs.get_untracked();
                    if let Some(tab) = list.get_mut(current) {
                        model::undo_to_reflog(tab, index);
                        tabs.set(list);
                    }
                }
            });
        })
    };

    let persist_audit: Arc<dyn Fn()> = {
        let store_dir = store_dir.clone();
        let agent_audit = agent_audit;
        Arc::new(move || {
            let lines: Vec<String> = agent_audit
                .get_untracked()
                .iter()
                .map(|entry| {
                    format!(
                        "{}\t{}\t{}\t{}",
                        entry.timestamp,
                        entry.action,
                        if entry.ok { "ok" } else { "fail" },
                        entry.detail
                    )
                })
                .collect();
            store::save_agent_audit(&store_dir, &lines);
        })
    };

    let propose_plan: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        let agent_plan = agent_plan;
        let commit_summary = commit_summary;
        Arc::new(move || {
            let current = active.get_untracked();
            let list = tabs.get_untracked();
            let Some(tab) = list.get(current) else {
                return;
            };
            let staged = tab.changes.iter().filter(|entry| entry.staged).count();
            let unstaged = tab.changes.len() - staged;
            let message = commit_summary.get_untracked();
            let plan = agent::propose(
                "本地计划：整理当前仓库",
                staged,
                unstaged,
                tab.ahead,
                tab.behind,
                message.as_str(),
            );
            agent_plan.set(Some(plan));
        })
    };

    let run_agent_step: Arc<dyn Fn(usize)> = {
        let tabs = tabs;
        let active = active;
        let agent_plan = agent_plan;
        let agent_audit = agent_audit;
        let agent_policies = agent_policies;
        let agent_settings = agent_settings;
        let commit_summary = commit_summary;
        let persist_audit = persist_audit.clone();
        Arc::new(move |index: usize| {
            let Some(plan) = agent_plan.get_untracked() else {
                return;
            };
            let Some(step) = plan.steps.get(index).cloned() else {
                return;
            };
            let now = store::now_unix();
            // 受保护分支：写动作一律不执行，记审计并说明原因。
            let branch_now = {
                let list = tabs.get_untracked();
                list.get(active.get_untracked())
                    .map(|tab| tab.branch.clone())
                    .unwrap_or_default()
            };
            if let Some(reason) =
                agent_settings
                    .get_untracked()
                    .blocks(step.action, branch_now.as_str())
            {
                let mut audit = agent_audit.get_untracked();
                audit.insert(
                    0,
                    agent::AuditEntry::new(now, step.action, false, reason.as_str()),
                );
                audit.truncate(agent::Audit::LIMIT);
                agent_audit.set(audit);
                persist_audit();
                return;
            }
            // 权限：Deny 直接记审计并跳过；Ask 在界面上就是用户按了这一行。
            if agent_policies.get_untracked().is_denied(step.action) {
                let mut audit = agent_audit.get_untracked();
                audit.insert(
                    0,
                    agent::AuditEntry::new(now, step.action, false, "denied by policy"),
                );
                audit.truncate(agent::Audit::LIMIT);
                agent_audit.set(audit);
                persist_audit();
                return;
            }
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            let Some(tab) = list.get_mut(current) else {
                return;
            };
            let detail = match step.action {
                agent::Action::StageAll => {
                    model::stage_all_or_none(tab, true);
                    "staged all".to_string()
                }
                agent::Action::UnstageAll => {
                    model::stage_all_or_none(tab, false);
                    "unstaged all".to_string()
                }
                // 只读动作与强推不是 plan 的步骤：前者不产生副作用，后者必须由人
                // 在同步面板里勾选确认，不交给 Agent 自动执行。
                agent::Action::ReadFiles | agent::Action::ForcePush => {
                    "skipped".to_string()
                }
                agent::Action::Commit => {
                    tab.summary = step
                        .argument
                        .clone()
                        .unwrap_or_else(|| commit_summary.get_untracked());
                    model::commit_staged(tab);
                    tab.hook_log.clone().unwrap_or_default()
                }
                agent::Action::Fetch => {
                    model::sync(tab, model::Sync::Fetch);
                    tab.sync_message.clone().unwrap_or_default()
                }
                agent::Action::Pull => {
                    model::sync(tab, model::Sync::Pull);
                    tab.sync_message.clone().unwrap_or_default()
                }
                agent::Action::Push => {
                    model::sync(tab, model::Sync::Push);
                    tab.sync_message.clone().unwrap_or_default()
                }
                agent::Action::Discard => "skipped: destructive".to_string(),
            };
            let ok = match step.action {
                agent::Action::Commit => tab.hook_ok,
                agent::Action::Fetch | agent::Action::Pull | agent::Action::Push => {
                    !tab.sync_failed
                }
                agent::Action::Discard => false,
                _ => true,
            };
            let mut audit = agent_audit.get_untracked();
            audit.insert(0, agent::AuditEntry::new(now, step.action, ok, detail));
            audit.truncate(agent::Audit::LIMIT);
            agent_audit.set(audit);
            tabs.set(list);
            persist_audit();
        })
    };

    let remove_agent_step: Arc<dyn Fn(usize)> = {
        let agent_plan = agent_plan;
        Arc::new(move |index: usize| {
            let Some(mut plan) = agent_plan.get_untracked() else {
                return;
            };
            if plan.remove_step(index) {
                agent_plan.set(Some(plan));
            }
        })
    };

    let add_agent_step: Arc<dyn Fn(&'static str)> = {
        let agent_plan = agent_plan;
        let language = language;
        Arc::new(move |id: &'static str| {
            let Some(action) = agent::Action::ALL
                .into_iter()
                .find(|candidate| candidate.id() == id)
            else {
                return;
            };
            let lang = language.get_untracked();
            let label = i18n::t(lang.as_str(), workspace::agent::action_key(action));
            let mut plan = agent_plan.get_untracked().unwrap_or_default();
            if plan.summary.trim().is_empty() {
                plan.summary = i18n::t(lang.as_str(), "agent.manualSummary");
            }
            plan.push_step(agent::Step::new(action, label));
            agent_plan.set(Some(plan));
        })
    };

    // Approve & Run：先按策略算出可执行范围，列出来让用户确认，再按顺序执行。
    let run_plan: Arc<dyn Fn()> = {
        let agent_plan = agent_plan;
        let agent_policies = agent_policies;
        let agent_settings = agent_settings;
        let tabs = tabs;
        let active = active;
        let language = language;
        let notice = notice;
        let run_step = run_agent_step.clone();
        Arc::new(move || {
            let Some(plan) = agent_plan.get_untracked() else {
                return;
            };
            let branch = tabs
                .get_untracked()
                .get(active.get_untracked())
                .map(|tab| tab.branch.clone())
                .unwrap_or_default();
            let runnable = plan.runnable(
                &agent_policies.get_untracked(),
                &agent_settings.get_untracked(),
                branch.as_str(),
            );
            let lang = language.get_untracked();
            if runnable.is_empty() {
                interactions::notify(notice, i18n::t(lang.as_str(), "agent.nothingRunnable"));
                return;
            }
            let listed: Vec<String> = runnable
                .iter()
                .filter_map(|index| plan.steps.get(*index))
                .map(|step| format!("• {}", step.label))
                .collect();
            let message = format!(
                "{}\n\n{}",
                i18n::t(lang.as_str(), "agent.approveMessage"),
                listed.join("\n")
            );
            let action_label = i18n::t(lang.as_str(), "agent.approveRun");
            let title = i18n::t(lang.as_str(), "agent.approveTitle");
            let run_step = run_step.clone();
            spawn(async move {
                let ok = interactions::confirm(
                    title,
                    message,
                    "runPlan",
                    action_label,
                    false,
                )
                .await;
                if !ok {
                    return;
                }
                for index in runnable {
                    run_step(index);
                }
            });
        })
    };

    let set_agent_permission: Arc<dyn Fn(agent::Action, agent::Permission)> = {
        let agent_policies = agent_policies;
        let store_dir = store_dir.clone();
        Arc::new(move |action: agent::Action, permission: agent::Permission| {
            let mut policies = agent_policies.get_untracked();
            policies.set(action, permission);
            agent_policies.set(policies.clone());
            let entries: Vec<(String, String)> = agent::Action::ALL
                .into_iter()
                .map(|candidate| {
                (
                    candidate.id().to_string(),
                    policies.permission(candidate).id().to_string(),
                )
            })
            .collect();
            store::save_agent_policies(&store_dir, &entries);
        })
    };

    let open_form: Arc<dyn Fn(screens::FormKind)> = {
        let form_state = form_state;
        Arc::new(move |kind: screens::FormKind| {
            form_state.set(Some(screens::FormState::new(kind)));
        })
    };

    // 表单提交：按种类执行对应的 Git 写操作，失败时把原因留在对话框里。
    let submit_form: Arc<dyn Fn(screens::FormState)> = {
        let tabs = tabs;
        let active = active;
        let form_state = form_state;
        Arc::new(move |form: screens::FormState| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            let Some(tab) = list.get_mut(current) else {
                return;
            };
            let first = form.first.trim().to_string();
            let second = form.second.trim().to_string();
            if first.is_empty() {
                let mut next = form.clone();
                next.error = Some("名称不能为空".to_string());
                form_state.set(Some(next));
                return;
            }
            let result = match form.kind {
                screens::FormKind::NewBranch => model::create_branch(
                    tab,
                    first.as_str(),
                    second.as_str(),
                    form.flag,
                ),
                screens::FormKind::NewTag => model::create_tag(
                    tab,
                    first.as_str(),
                    second.as_str(),
                    form.third.as_str(),
                    form.flag,
                ),
                screens::FormKind::AddRemote => model::add_remote(tab, first.as_str(), second.as_str()),
                screens::FormKind::NewStash => {
                    model::stash_changes(tab, first.as_str(), form.flag)
                }
                // 重命名与改 URL：`second` 是被改对象的原名，`first` 是新值。
                screens::FormKind::RenameBranch => {
                    model::rename_branch(tab, second.as_str(), first.as_str())
                }
                screens::FormKind::EditRemote => {
                    model::edit_remote(tab, second.as_str(), first.as_str())
                }
                // 身份：`first` 是姓名，`second` 是邮箱。
                screens::FormKind::Identity => {
                    model::set_identity(tab, first.as_str(), second.as_str())
                }
            };
            tabs.set(list);
            match result {
                Ok(()) => form_state.set(None),
                Err(detail) => {
                    let mut next = form;
                    next.error = Some(detail);
                    form_state.set(Some(next));
                }
            }
        })
    };

    let cancel_form: Arc<dyn Fn()> = {
        let form_state = form_state;
        Arc::new(move || form_state.set(None))
    };

    let set_active_changelist: Arc<dyn Fn(Option<String>)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |name: Option<String>| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::set_active_changelist(tab, name);
                tabs.set(list);
            }
        })
    };

    let move_to_changelist: Arc<dyn Fn(String, Option<String>)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |path: String, list_name: Option<String>| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::move_to_changelist(tab, path.as_str(), list_name.as_deref());
                tabs.set(list);
            }
        })
    };

    let create_changelist: Arc<dyn Fn(String)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |name: String| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::create_changelist(tab, name.as_str());
                tabs.set(list);
            }
        })
    };

    let checkout_new_branch: Arc<dyn Fn(String, String)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |name: String, from: String| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                if let Err(detail) = model::checkout_new_branch(tab, name.as_str(), from.as_str())
                {
                    tab.error = Some(detail);
                }
                tabs.set(list);
            }
        })
    };

    let merge_reference: Arc<dyn Fn(String)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |reference: String| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                if let Err(detail) = model::merge_reference(tab, reference.as_str()) {
                    tab.error = Some(detail);
                }
                tabs.set(list);
            }
        })
    };

    // 重置不可撤销（--hard 会丢改动）：先确认。
    let reset_to: Arc<dyn Fn(String, git::ResetMode)> = {
        let tabs = tabs;
        let active = active;
        let language = language;
        Arc::new(move |sha: String, mode: git::ResetMode| {
            let language_now = language.get_untracked();
            let pending = crate::ui::dialog::confirm_destructive(
                i18n::t(language_now.as_str(), "branch.resetTitle"),
                i18n::with_arguments(
                    language_now.as_str(),
                    "branch.resetMessage",
                    &[("short", sha.chars().take(7).collect::<String>())],
                ),
                i18n::t(language_now.as_str(), "branch.resetAction"),
                i18n::t(language_now.as_str(), "common.cancel"),
            );
            let tabs = tabs;
            let active = active;
            spawn(async move {
                if let Ok(result) = pending.await {
                    if result.action.as_deref() != Some("reset") {
                        return;
                    }
                    let current = active.get_untracked();
                    let mut list = tabs.get_untracked();
                    if let Some(tab) = list.get_mut(current) {
                        if let Err(detail) = model::reset_to(tab, sha.as_str(), mode) {
                            tab.error = Some(detail);
                        }
                        tabs.set(list);
                    }
                }
            });
        })
    };

    let drop_tag: Arc<dyn Fn(String)> = {
        let tabs = tabs;
        let active = active;
        let language = language;
        Arc::new(move |name: String| {
            let language_now = language.get_untracked();
            let pending = interactions::confirm(
                i18n::t(language_now.as_str(), "confirm.dropTagTitle"),
                i18n::with_arguments(
                    language_now.as_str(),
                    "confirm.dropTagMessage",
                    &[("name", name.clone())],
                ),
                "drop",
                i18n::t(language_now.as_str(), "confirm.deleteAction"),
                true,
            );
            spawn(async move {
                if !pending.await {
                    return;
                }
                let current = active.get_untracked();
                let mut list = tabs.get_untracked();
                if let Some(tab) = list.get_mut(current) {
                    let _ = model::drop_tag(tab, name.as_str());
                    tabs.set(list);
                }
            });
        })
    };

    let drop_remote: Arc<dyn Fn(String)> = {
        let tabs = tabs;
        let active = active;
        let language = language;
        Arc::new(move |name: String| {
            let language_now = language.get_untracked();
            let pending = interactions::confirm(
                i18n::t(language_now.as_str(), "confirm.dropRemoteTitle"),
                i18n::with_arguments(
                    language_now.as_str(),
                    "confirm.dropRemoteMessage",
                    &[("name", name.clone())],
                ),
                "drop",
                i18n::t(language_now.as_str(), "confirm.removeAction"),
                true,
            );
            spawn(async move {
                if !pending.await {
                    return;
                }
                let current = active.get_untracked();
                let mut list = tabs.get_untracked();
                if let Some(tab) = list.get_mut(current) {
                    let _ = model::drop_remote(tab, name.as_str());
                    tabs.set(list);
                }
            });
        })
    };

    let drop_stash: Arc<dyn Fn(String)> = {
        let tabs = tabs;
        let active = active;
        let language = language;
        Arc::new(move |reference: String| {
            let language_now = language.get_untracked();
            let pending = interactions::confirm(
                i18n::t(language_now.as_str(), "confirm.dropStashTitle"),
                i18n::with_arguments(
                    language_now.as_str(),
                    "confirm.dropStashMessage",
                    &[("name", reference.clone())],
                ),
                "drop",
                i18n::t(language_now.as_str(), "confirm.dropAction"),
                true,
            );
            spawn(async move {
                if !pending.await {
                    return;
                }
                let current = active.get_untracked();
                let mut list = tabs.get_untracked();
                if let Some(tab) = list.get_mut(current) {
                    let _ = model::drop_stash(tab, reference.as_str());
                    tabs.set(list);
                }
            });
        })
    };

    let checkout_branch: Arc<dyn Fn(String)> = {
        let tabs = tabs;
        let active = active;
        let language = language;
        let notice = notice;
        Arc::new(move |branch: String| {
            let current = active.get_untracked();
            let dirty = tabs
                .get_untracked()
                .get(current)
                .is_some_and(model::is_dirty);
            if !dirty {
                let mut list = tabs.get_untracked();
                if let Some(tab) = list.get_mut(current) {
                    model::checkout_branch(tab, branch.as_str());
                    tabs.set(list);
                }
                return;
            }
            // 有改动时先问：贮藏后切、把改动带过去、还是取消（手册第 4 节）。
            let lang = language.get_untracked();
            let title = i18n::t(lang.as_str(), "confirm.dirtyCheckoutTitle");
            let message = i18n::with_arguments(
                lang.as_str(),
                "confirm.dirtyCheckoutMessage",
                &[("branch", branch.clone())],
            );
            let stash_label = i18n::t(lang.as_str(), "confirm.dirtyCheckoutStash");
            let carry_label = i18n::t(lang.as_str(), "confirm.dirtyCheckoutCarry");
            let cancel_label = i18n::t(lang.as_str(), "common.cancel");
            let pending = nsh_base::alert_dialog()
                .title(title)
                .description(message)
                .action("stash", stash_label)
                .action("carry", carry_label)
                .cancel("cancel", cancel_label)
                .show();
            let language = language;
            let notice = notice;
            spawn(async move {
                let Ok(result) = pending.await else {
                    return;
                };
                let Some(action) = result.action else {
                    return;
                };
                if action.as_str() == "cancel" {
                    return;
                }
                let stash = action.as_str() == "stash";
                let mut list = tabs.get_untracked();
                if let Some(tab) = list.get_mut(current) {
                    model::checkout_branch_with(tab, branch.as_str(), stash);
                    let failed = tab.error.clone();
                    tabs.set(list);
                    if let Some(detail) = failed {
                        interactions::notify(notice, detail);
                    } else if stash {
                        let lang = language.get_untracked();
                        interactions::notify(
                            notice,
                            i18n::t(lang.as_str(), "toast.stashedBeforeCheckout"),
                        );
                    }
                }
            });
        })
    };

    // 删除分支不可撤销：先确认。
    let delete_branch: Arc<dyn Fn(String)> = {
        let tabs = tabs;
        let active = active;
        let language = language;
        Arc::new(move |branch: String| {
            let language_now = language.get_untracked();
            let pending = crate::ui::dialog::confirm_destructive(
                i18n::t(language_now.as_str(), "branch.deleteTitle"),
                i18n::with_arguments(
                    language_now.as_str(),
                    "branch.deleteMessage",
                    &[("name", branch.clone())],
                ),
                i18n::t(language_now.as_str(), "branch.deleteAction"),
                i18n::t(language_now.as_str(), "common.cancel"),
            );
            let tabs = tabs;
            let active = active;
            spawn(async move {
                if let Ok(result) = pending.await {
                    if result.action.as_deref() != Some("delete") {
                        return;
                    }
                    let current = active.get_untracked();
                    let mut list = tabs.get_untracked();
                    if let Some(tab) = list.get_mut(current) {
                        model::delete_branch(tab, branch.as_str());
                        tabs.set(list);
                    }
                }
            });
        })
    };

    let set_compare_refs: Arc<dyn Fn(Option<String>, Option<String>)> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move |base: Option<String>, target: Option<String>| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::set_compare_refs(tab, base, target);
                tabs.set(list);
            }
        })
    };

    let set_mode_agent: Arc<dyn Fn()> = {
        let mode = mode;
        Arc::new(move || mode.set(Mode::Agent))
    };

    let set_mode_refs: Arc<dyn Fn()> = {
        let mode = mode;
        Arc::new(move || mode.set(Mode::Refs))
    };

    let copy_text: Arc<dyn Fn(String)> = {
        let notice = notice;
        let language = language;
        Arc::new(move |text: String| {
            let language_now = language.get_untracked();
            interactions::copy(
                notice,
                text,
                i18n::t(language_now.as_str(), "toast.copied"),
                i18n::t(language_now.as_str(), "toast.copyFailed"),
            );
        })
    };

    let open_editor: Arc<dyn Fn(String)> = {
        let notice = notice;
        let language = language;
        Arc::new(move |path: String| {
            let language_now = language.get_untracked();
            interactions::open_in_editor(
                notice,
                path,
                i18n::t(language_now.as_str(), "toast.openFailed"),
            );
        })
    };

    let open_browser: Arc<dyn Fn(String)> = {
        let notice = notice;
        let language = language;
        Arc::new(move |url: String| {
            let language_now = language.get_untracked();
            interactions::open_browser(
                notice,
                url,
                i18n::t(language_now.as_str(), "toast.browserFailed"),
            );
        })
    };

    let reveal_path: Arc<dyn Fn(String)> = {
        let notice = notice;
        let language = language;
        Arc::new(move |path: String| {
            let language_now = language.get_untracked();
            interactions::reveal(
                notice,
                path,
                i18n::t(language_now.as_str(), "toast.revealFailed"),
            );
        })
    };

    // 历史操作：都先确认，再落到 model → git。
    let history_action: Arc<dyn Fn(&'static str, String)> = {
        let tabs = tabs;
        let active = active;
        let language = language;
        let notice = notice;
        Arc::new(move |kind: &'static str, sha: String| {
            let language_now = language.get_untracked();
            let lang = language_now.as_str();
            let (title_key, message_key, action_key) = match kind {
                "checkout" => (
                    "confirm.checkoutTitle",
                    "confirm.checkoutMessage",
                    "confirm.checkoutAction",
                ),
                "cherry" => (
                    "confirm.cherryPickTitle",
                    "confirm.cherryPickMessage",
                    "confirm.cherryPickAction",
                ),
                _ => (
                    "confirm.revertTitle",
                    "confirm.revertMessage",
                    "confirm.revertAction",
                ),
            };
            let short = sha.chars().take(7).collect::<String>();
            let pending = interactions::confirm(
                i18n::with_arguments(lang, title_key, &[("short", short.clone())]),
                i18n::t(lang, message_key),
                "history",
                i18n::t(lang, action_key),
                kind == "checkout",
            );
            spawn(async move {
                if !pending.await {
                    return;
                }
                let current = active.get_untracked();
                let mut list = tabs.get_untracked();
                if let Some(tab) = list.get_mut(current) {
                    let result = match kind {
                        "checkout" => model::checkout_commit(tab, sha.as_str()),
                        "cherry" => model::cherry_pick(tab, sha.as_str()),
                        _ => model::revert_commit(tab, sha.as_str()),
                    };
                    match result {
                        Ok(_) => {
                            let lang = language.get_untracked();
                            interactions::notify(
                                notice,
                                i18n::t(lang.as_str(), "toast.historyDone"),
                            );
                        }
                        Err(detail) => tab.error = Some(detail),
                    }
                    tabs.set(list);
                }
            });
        })
    };

    let checkout_commit: Arc<dyn Fn(String)> = {
        let history_action = history_action.clone();
        Arc::new(move |sha: String| history_action("checkout", sha))
    };

    let cherry_pick: Arc<dyn Fn(String)> = {
        let history_action = history_action.clone();
        Arc::new(move |sha: String| history_action("cherry", sha))
    };

    let revert_commit: Arc<dyn Fn(String)> = {
        let history_action = history_action.clone();
        Arc::new(move |sha: String| history_action("revert", sha))
    };

    // 从某个提交新建分支/标签：表单第二个字段预填该提交。
    let open_form_from: Arc<dyn Fn(screens::FormKind, String)> = {
        let form_state = form_state;
        Arc::new(move |kind: screens::FormKind, from: String| {
            let mut state = screens::FormState::new(kind);
            state.second = from;
            form_state.set(Some(state));
        })
    };

    // 冲突视图（设计板 14）。
    let open_conflict: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        let conflict_result = conflict_result;
        Arc::new(move || {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::open_conflict(tab);
                conflict_result.set(tab.conflict_text.clone());
                tabs.set(list);
            }
        })
    };

    let select_conflict: Arc<dyn Fn(usize)> = {
        let tabs = tabs;
        let active = active;
        let conflict_result = conflict_result;
        Arc::new(move |index: usize| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::select_conflict(tab, index);
                conflict_result.set(tab.conflict_text.clone());
                tabs.set(list);
            }
        })
    };

    let step_conflict: Arc<dyn Fn(bool)> = {
        let tabs = tabs;
        let active = active;
        let conflict_result = conflict_result;
        Arc::new(move |forward: bool| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::step_conflict(tab, forward);
                conflict_result.set(tab.conflict_text.clone());
                tabs.set(list);
            }
        })
    };

    let apply_side: Arc<dyn Fn(domain::conflict::Side)> = {
        let tabs = tabs;
        let active = active;
        let conflict_result = conflict_result;
        Arc::new(move |side: domain::conflict::Side| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                let merged = model::conflict_take_side(tab, side);
                conflict_result.set(merged);
                tabs.set(list);
            }
        })
    };

    let mark_resolved: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        let conflict_result = conflict_result;
        let notice = notice;
        let language = language;
        Arc::new(move || {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                let resolved = conflict_result.get_untracked();
                match model::mark_conflict_resolved(tab, resolved.as_str()) {
                    Ok(()) => {
                        conflict_result.set(tab.conflict_text.clone());
                        let lang = language.get_untracked();
                        interactions::notify(
                            notice,
                            i18n::t(lang.as_str(), "conflicts.marked"),
                        );
                    }
                    Err(detail) => tab.error = Some(detail),
                }
                tabs.set(list);
            }
        })
    };

    let continue_op: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        let notice = notice;
        let language = language;
        Arc::new(move || {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                match model::continue_operation(tab) {
                    Ok(_) => {
                        let lang = language.get_untracked();
                        interactions::notify(
                            notice,
                            i18n::t(lang.as_str(), "toast.historyDone"),
                        );
                    }
                    Err(detail) => tab.error = Some(detail),
                }
                tabs.set(list);
            }
        })
    };

    let close_conflict: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move || {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                tab.conflict_open = false;
                tabs.set(list);
            }
        })
    };

    // 引用行上的动作：推送标签、应用/弹出贮藏。
    let push_tag: Arc<dyn Fn(String)> = {
        let tabs = tabs;
        let active = active;
        let notice = notice;
        let language = language;
        Arc::new(move |name: String| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                match model::push_tag(tab, name.as_str()) {
                    Ok(_) => {
                        let lang = language.get_untracked();
                        interactions::notify(notice, i18n::t(lang.as_str(), "toast.pushed"));
                    }
                    Err(detail) => tab.error = Some(detail),
                }
                tabs.set(list);
            }
        })
    };

    let stash_action: Arc<dyn Fn(&'static str, String)> = {
        let tabs = tabs;
        let active = active;
        let notice = notice;
        let language = language;
        Arc::new(move |kind: &'static str, reference: String| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                let result = if kind == "apply" {
                    model::apply_stash(tab, reference.as_str())
                } else {
                    model::pop_stash(tab, reference.as_str())
                };
                match result {
                    Ok(()) => {
                        let lang = language.get_untracked();
                        interactions::notify(notice, i18n::t(lang.as_str(), "toast.stashDone"));
                    }
                    Err(detail) => tab.error = Some(detail),
                }
                tabs.set(list);
            }
        })
    };

    let apply_stash: Arc<dyn Fn(String)> = {
        let stash_action = stash_action.clone();
        Arc::new(move |reference: String| stash_action("apply", reference))
    };

    let pop_stash: Arc<dyn Fn(String)> = {
        let stash_action = stash_action.clone();
        Arc::new(move |reference: String| stash_action("pop", reference))
    };

    let show_in_history: Arc<dyn Fn(String)> = {
        let tabs = tabs;
        let active = active;
        let mode = mode;
        let notice = notice;
        let language = language;
        Arc::new(move |sha: String| {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                if model::select_by_sha(tab, sha.as_str()) {
                    mode.set(Mode::History);
                    tabs.set(list);
                } else {
                    let lang = language.get_untracked();
                    interactions::notify(
                        notice,
                        i18n::t(lang.as_str(), "reflog.notFound"),
                    );
                }
            }
        })
    };

    // 键盘与命令面板共用同一个派发入口（Actions 会把它移走，因此先留一份）。
    let run_command_for_key = run_command.clone();

    let set_update_mode: Arc<dyn Fn(String)> = {
        let settings_state = settings_state;
        let store_dir = store_dir.clone();
        Arc::new(move |mode: String| {
            let mut next = settings_state.get_untracked();
            next.update_mode = mode;
            settings_state.set(next.clone());
            store::save_settings(&store_dir, &next);
        })
    };

    let open_identity: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        let form_state = form_state;
        Arc::new(move || {
            let list = tabs.get_untracked();
            let Some(tab) = list.get(active.get_untracked()) else {
                return;
            };
            let (name, email) = model::identity(tab);
            let mut state = screens::FormState::new(screens::FormKind::Identity);
            state.first = name;
            state.second = email;
            form_state.set(Some(state));
        })
    };

    let clear_identity: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        let notice = notice;
        let language = language;
        Arc::new(move || {
            let list = tabs.get_untracked();
            let Some(tab) = list.get(active.get_untracked()) else {
                return;
            };
            match model::clear_identity(tab) {
                Ok(()) => {
                    let lang = language.get_untracked();
                    interactions::notify(notice, i18n::t(lang.as_str(), "toast.identityCleared"));
                }
                Err(detail) => interactions::notify(notice, detail),
            }
        })
    };

    // 历史改写（设计板 22）：打开 → 编辑说明 → 执行。
    let open_reword: Arc<dyn Fn(String, String, String)> = {
        let tabs = tabs;
        let active = active;
        let rewrite_state = rewrite_state;
        Arc::new(move |sha: String, short: String, summary: String| {
            let blocker = {
                let list = tabs.get_untracked();
                list.get(active.get_untracked())
                    .and_then(model::rewrite_blocker)
            };
            rewrite_state.set(Some(screens::RewriteState::new(
                screens::RewriteKind::Reword { sha, short, summary: summary.clone() },
                summary,
                blocker,
            )));
        })
    };

    let open_squash: Arc<dyn Fn(String, String, Vec<(String, String)>)> = {
        let tabs = tabs;
        let active = active;
        let rewrite_state = rewrite_state;
        Arc::new(move |from: String, to: String, commits: Vec<(String, String)>| {
            let blocker = {
                let list = tabs.get_untracked();
                list.get(active.get_untracked())
                    .and_then(model::rewrite_blocker)
            };
            // 结果说明默认取最旧那条提交的说明，用户可改。
            let message = commits
                .last()
                .map(|(_, summary)| summary.clone())
                .unwrap_or_default();
            rewrite_state.set(Some(screens::RewriteState::new(
                screens::RewriteKind::Squash { from, to, commits },
                message,
                blocker,
            )));
        })
    };

    let set_rewrite_message: Arc<dyn Fn(String)> = {
        let rewrite_state = rewrite_state;
        Arc::new(move |message: String| {
            let mut state = match rewrite_state.get_untracked() {
                Some(state) => state,
                None => return,
            };
            state.message = message;
            state.error = None;
            rewrite_state.set(Some(state));
        })
    };

    let submit_rewrite: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        let rewrite_state = rewrite_state;
        let notice = notice;
        let language = language;
        Arc::new(move || {
            let Some(state) = rewrite_state.get_untracked() else {
                return;
            };
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            let Some(tab) = list.get_mut(current) else {
                return;
            };
            let result = match &state.kind {
                screens::RewriteKind::Reword { sha, .. } => {
                    model::reword_commit(tab, sha.as_str(), state.message.as_str())
                }
                screens::RewriteKind::Squash { from, to, .. } => model::squash_commits(
                    tab,
                    from.as_str(),
                    to.as_str(),
                    state.message.as_str(),
                ),
            };
            match result {
                Ok(()) => {
                    tabs.set(list);
                    rewrite_state.set(None);
                    let lang = language.get_untracked();
                    interactions::notify(notice, i18n::t(lang.as_str(), "rewrite.done"));
                }
                Err(detail) => {
                    tabs.set(list);
                    let mut next = state;
                    next.error = Some(detail);
                    rewrite_state.set(Some(next));
                }
            }
        })
    };

    let close_rewrite: Arc<dyn Fn()> = {
        let rewrite_state = rewrite_state;
        Arc::new(move || rewrite_state.set(None))
    };

    // 状态横幅上的三个动作（设计板 16）。
    let checkout_default_branch: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        let notice = notice;
        let language = language;
        Arc::new(move || {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                match model::checkout_default_branch(tab) {
                    Ok(()) => {
                        let lang = language.get_untracked();
                        interactions::notify(notice, i18n::t(lang.as_str(), "toast.historyDone"));
                    }
                    Err(detail) => interactions::notify(notice, detail),
                }
                tabs.set(list);
            }
        })
    };

    let init_here: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        let notice = notice;
        let language = language;
        Arc::new(move || {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                match model::init_here(tab) {
                    Ok(()) => {
                        let lang = language.get_untracked();
                        interactions::notify(notice, i18n::t(lang.as_str(), "toast.initDone"));
                    }
                    Err(detail) => interactions::notify(notice, detail),
                }
                tabs.set(list);
            }
        })
    };

    let open_parent: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        let open_path = open_path.clone();
        Arc::new(move || {
            let current = active.get_untracked();
            let list = tabs.get_untracked();
            let Some(tab) = list.get(current) else {
                return;
            };
            let parent = model::parent_path(tab.path.as_str());
            if !parent.is_empty() {
                open_path(parent);
            }
        })
    };

    // 缺失仓库的 Locate…：选一个新目录打开，然后关掉这个打不开的标签页。
    let locate_repo: Arc<dyn Fn()> = {
        let open_path = open_path.clone();
        let close_tab = close_tab.clone();
        let active = active;
        Arc::new(move || {
            let index = active.get_untracked();
            let open_path = open_path.clone();
            let close_tab = close_tab.clone();
            let answer = pick_paths(nsh_core::file_dialog::OpenDialog::folders());
            spawn(async move {
                let Ok(Some(paths)) = answer.await else {
                    return;
                };
                let Some(path) = paths.first() else {
                    return;
                };
                open_path(path.display().to_string());
                // 先打开再关旧的：关掉会让下标前移。
                close_tab(index);
            });
        })
    };

    let create_initial_commit: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        let notice = notice;
        let language = language;
        Arc::new(move || {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                let lang = language.get_untracked();
                let message = i18n::t(lang.as_str(), "state.initialCommitMessage");
                match model::create_initial_commit(tab, message.as_str()) {
                    Ok(()) => interactions::notify(notice, i18n::t(lang.as_str(), "toast.historyDone")),
                    Err(detail) => interactions::notify(notice, detail),
                }
                tabs.set(list);
            }
        })
    };

    let clear_sync_fault: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        Arc::new(move || {
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current) {
                model::clear_sync_fault(tab);
                tabs.set(list);
            }
        })
    };

    let toggle_diff_option: Arc<dyn Fn(&'static str)> = {
        let diff_options = diff_options;
        Arc::new(move |which: &'static str| {
            let mut next = diff_options.get_untracked();
            match which {
                "fold" => next.fold = !next.fold,
                "wrap" => next.wrap = !next.wrap,
                "whitespace" => next.whitespace = !next.whitespace,
                _ => return,
            }
            diff_options.set(next);
        })
    };

    // 导出 patch：写到用户选择的路径。没有选择就不写，不落默认文件名。
    let export_patch: Arc<dyn Fn()> = {
        let tabs = tabs;
        let active = active;
        let notice = notice;
        let language = language;
        Arc::new(move || {
            let list = tabs.get_untracked();
            let Some(tab) = list.get(active.get_untracked()) else {
                return;
            };
            let patch = tab.patch.clone();
            if patch.trim().is_empty() {
                let lang = language.get_untracked();
                interactions::notify(notice, i18n::t(lang.as_str(), "diff.nothingToExport"));
                return;
            }
            let lang = language.get_untracked();
            let suggested = i18n::t(lang.as_str(), "diff.exportName");
            let pending =
                pick_save_path(nsh_core::file_dialog::SaveDialog::new().file_name(suggested));
            spawn(async move {
                let Ok(Some(path)) = pending.await else {
                    return;
                };
                match std::fs::write(&path, patch.as_bytes()) {
                    Ok(()) => {
                        interactions::notify(notice, i18n::t(lang.as_str(), "diff.exported"));
                    }
                    Err(error) => {
                        interactions::notify(notice, error.to_string());
                    }
                }
            });
        })
    };

    let apply_piece: Arc<dyn Fn(usize, Option<usize>, bool)> = {
        let tabs = tabs;
        let active = active;
        let notice = notice;
        let language = language;
        Arc::new(
            move |hunk: usize, line: Option<usize>, reverse: bool| {
                let current = active.get_untracked();
                let mut list = tabs.get_untracked();
                if let Some(tab) = list.get_mut(current) {
                    match model::apply_piece(tab, hunk, line, reverse) {
                        Ok(()) => {
                            let lang = language.get_untracked();
                            interactions::notify(
                                notice,
                                i18n::t(lang.as_str(), "changes.pieceApplied"),
                            );
                        }
                        Err(detail) => tab.error = Some(detail),
                    }
                    tabs.set(list);
                }
            },
        )
    };

    // Agent 的三项运行设置：自动化、预览时长、受保护分支。改动即持久化。
    let persist_agent = {
        let store_dir = store_dir.clone();
        let agent_policies = agent_policies;
        Arc::new(move |settings: &agent::Settings| {
            let pairs = settings.write_pairs();
            let mut lines: Vec<(String, String)> = store::load_agent_policies(&store_dir);
            for (key, value) in pairs {
                lines.retain(|(existing, _)| existing != &key);
                lines.push((key, value));
            }
            let _ = agent_policies;
            store::save_agent_policies(&store_dir, &lines);
        })
    };

    let set_automation: Arc<dyn Fn(agent::Automation)> = {
        let agent_settings = agent_settings;
        let persist = persist_agent.clone();
        Arc::new(move |mode: agent::Automation| {
            let mut next = agent_settings.get_untracked();
            next.automation = mode;
            agent_settings.set(next.clone());
            persist(&next);
        })
    };

    let set_review_millis: Arc<dyn Fn(u32)> = {
        let agent_settings = agent_settings;
        let persist = persist_agent.clone();
        Arc::new(move |millis: u32| {
            let mut next = agent_settings.get_untracked();
            next.review_millis = millis;
            agent_settings.set(next.clone());
            persist(&next);
        })
    };

    let toggle_protected_branch: Arc<dyn Fn(String)> = {
        let agent_settings = agent_settings;
        let persist = persist_agent.clone();
        Arc::new(move |branch: String| {
            let mut next = agent_settings.get_untracked();
            if next.is_protected(branch.as_str()) {
                next.protected.retain(|name| name != &branch);
            } else {
                next.protected.push(branch);
            }
            agent_settings.set(next.clone());
            persist(&next);
        })
    };

    // 日期显示：改设置立刻生效并持久化（设置页有实时预览）。
    let set_date_style: Arc<dyn Fn(crate::i18n::DateStyle)> = {
        let settings_state = settings_state;
        let store_dir = store_dir.clone();
        Arc::new(move |style: crate::i18n::DateStyle| {
            crate::dates::set_style(style);
            let mut next = settings_state.get_untracked();
            next.date_format = style.id().to_string();
            settings_state.set(next.clone());
            store::save_settings(&store_dir, &next);
        })
    };

    let set_date_pattern: Arc<dyn Fn(String)> = {
        let settings_state = settings_state;
        let store_dir = store_dir.clone();
        Arc::new(move |pattern: String| {
            crate::dates::set_pattern(pattern.as_str());
            let mut next = settings_state.get_untracked();
            next.date_pattern = pattern;
            // 输入 pattern 时自动切到 Custom，否则用户改了看不到效果。
            next.date_format = crate::i18n::DateStyle::Custom.id().to_string();
            crate::dates::set_style(crate::i18n::DateStyle::Custom);
            settings_state.set(next.clone());
            store::save_settings(&store_dir, &next);
        })
    };

    // Git 可执行文件：探测用 `git --version`，选择用系统打开对话框。
    let detect_git: Arc<dyn Fn()> = {
        let notice = notice;
        let language = language;
        Arc::new(move || {
            let lang = language.get_untracked();
            match git::info() {
                Ok(info) => interactions::notify(
                    notice,
                    format!("{} · {}", info.version, info.exec_path),
                ),
                Err(error) => {
                    interactions::notify(notice, i18n::t(lang.as_str(), "settings.detectFailed"))
                }
            }
        })
    };

    let choose_git: Arc<dyn Fn()> = {
        let notice = notice;
        let language = language;
        Arc::new(move || {
            // 只让选可执行文件；选完把路径记进设置并立刻用它探测一次。
            let pending = pick_paths(nsh_core::file_dialog::OpenDialog::files());
            let language = language;
            spawn(async move {
                let Ok(Some(paths)) = pending.await else {
                    return;
                };
                let Some(path) = paths.into_iter().next() else {
                    return;
                };
                let lang = language.get_untracked();
                match std::process::Command::new(&path).arg("--version").output() {
                    Ok(output) if output.status.success() => {
                        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        interactions::notify(notice, format!("{version} · {}", path.display()));
                    }
                    _ => {
                        interactions::notify(
                            notice,
                            i18n::t(lang.as_str(), "settings.detectFailed"),
                        );
                    }
                }
            });
        })
    };

    let toggle_git_path: Arc<dyn Fn()> = {
        let settings_state = settings_state;
        let store_dir = store_dir.clone();
        Arc::new(move || {
            let mut next = settings_state.get_untracked();
            let enabled = !next.use_system_git;
            next.use_system_git = enabled;
            settings_state.set(next.clone());
            store::save_settings(&store_dir, &next);
        })
    };

    // 扫描目录里的仓库（设计板 24 的 Add N Repositories）。
    let add_found_repos: Arc<dyn Fn()> = {
        let store_dir = store_dir.clone();
        let recent = recent;
        let notice = notice;
        let language = language;
        Arc::new(move || {
            let lang = language.get_untracked();
            let folders = store::load_git_scan_folders(&store_dir);
            let found = store::scan_git_repositories(&folders);
            if found.is_empty() {
                interactions::notify(notice, i18n::t(lang.as_str(), "settings.scanEmpty"));
                return;
            }
            let mut list = recent.get_untracked();
            for path in &found {
                store::remember_recent(&store_dir, &list, path.as_str());
                list = store::load_recent(&store_dir);
            }
            recent.set(list);
            interactions::notify(
                notice,
                i18n::with_arguments(
                    lang.as_str(),
                    "settings.scanAdded",
                    &[("count", found.len().to_string())],
                ),
            );
        })
    };

    let set_diff_split: Arc<dyn Fn(bool)> = {
        let diff_split = diff_split;
        Arc::new(move |split: bool| diff_split.set(split))
    };

    // 文件树两项与启动方式：改了就持久化。
    let set_tree_flag: Arc<dyn Fn(&'static str, bool)> = {
        let settings_state = settings_state;
        let store_dir = store_dir.clone();
        Arc::new(move |which: &'static str, value: bool| {
            let mut next = settings_state.get_untracked();
            match which {
                "merge_empty_dirs" => next.merge_empty_dirs = value,
                "compact_single_child" => next.compact_single_child = value,
                _ => return,
            }
            settings_state.set(next.clone());
            store::save_settings(&store_dir, &next);
        })
    };

    let set_launch_restore: Arc<dyn Fn(bool)> = {
        let settings_state = settings_state;
        let store_dir = store_dir.clone();
        Arc::new(move |restore: bool| {
            let mut next = settings_state.get_untracked();
            next.restore_session = restore;
            settings_state.set(next.clone());
            store::save_settings(&store_dir, &next);
        })
    };

    let actions = Actions {
        open_path: open_path.clone(),
        choose_folder: choose_folder.clone(),
        close_tab,
        activate_tab,
        select_commit,
        toggle_compare,
        set_compare_mode,
        select_file,
        set_mode,
        select_change,
        toggle_stage,
        discard_change,
        stage_all,
        unstage_all,
        commit,
        commit_without_hooks,
        set_commit_option,
        toggle_hook_log,
        checkout_default_branch,
        init_here,
        locate_repo,
        open_parent,
        set_diff_split,
        apply_piece,
        toggle_diff_option,
        export_patch,
        create_initial_commit,
        clear_sync_fault,
        set_automation,
        set_review_millis,
        toggle_protected_branch,
        open_reword,
        open_squash,
        set_rewrite_message,
        submit_rewrite,
        close_rewrite,
        open_conflict,
        select_conflict,
        step_conflict,
        apply_side,
        mark_resolved,
        continue_op,
        close_conflict,
        resolve_conflict,
        abort_operation,
        run_command,
        set_theme,
        set_file_tree,
        go_settings,
        undo_reflog,
        set_mode_refs,
        set_mode_agent,
        propose_plan,
        run_agent_step,
        remove_agent_step,
        add_agent_step,
        run_plan,
        set_agent_permission,
        set_compare_refs,
        checkout_branch,
        delete_branch,
        open_form,
        open_sync,
        set_sync_option,
        run_sync,
        close_sync,
        drop_tag,
        drop_remote,
        drop_stash,
        set_active_changelist,
        move_to_changelist,
        create_changelist,
        checkout_new_branch,
        merge_reference,
        checkout_commit,
        cherry_pick,
        revert_commit,
        open_form_from,
        show_in_history,
        push_tag,
        apply_stash,
        pop_stash,
        reset_to,
        apply_filter,
        clear_filter,
        clone_repository,
        cancel_clone,
        init_repository,
        toggle_language,
        refresh,
        go_welcome,
        go_repo_manager,
        set_repo_form,
        copy_text,
        open_editor,
        reveal_path,
        open_browser,
        set_update_mode,
        add_found_repos,
        choose_git,
        detect_git,
        toggle_git_path,
        set_tree_flag,
        set_launch_restore,
        set_date_style,
        set_date_pattern,
        open_identity,
        clear_identity,
    };

    // 原生菜单（设计板 22）：文案随语言重建。菜单项与命令面板共用同一批动作，
    // 因此不会指向空实现；系统角色项（关闭/撤销/最小化等）由系统给文案。
    {
        let run = actions.run_command.clone();
        let choose_folder = actions.choose_folder.clone();
        effect({
            let language = language;
            let shortcuts = shortcut_registry;
            move || {
                let lang = language.get();
                // 菜单里的加速键也取注册表：改键后菜单跟着变。
                let bindings = shortcuts.get();
                let combo = move |id: &str| bindings.combo(id).to_string();
                let items = build_menu(lang.as_str(), run.clone(), choose_folder.clone(), &combo);
                nsh_core::element::desktop::set_application_menu(items);
            }
        });
    }

    // 重置请求由历史行的右键菜单写入，这里统一执行（--mixed 保留改动、--hard 丢弃）。
    {
        let reset_request = reset_request;
        let reset_to = actions.reset_to.clone();
        effect(move || {
            if let Some((sha, mode)) = reset_request.get() {
                reset_request.set(None);
                reset_to(sha, mode);
            }
        });
    }

    // 快捷键：把清单里的每个组合各绑一次，按下时查注册表决定跑哪个命令。
    // 注册表是可改的（设置页 25 的 Record / Restore Defaults），而 nsh 0.5.0 的
    // on_key 只能在挂载时绑定一次，所以绑定的是「组合」而不是「动作」。
    for combo in shortcuts::catalog() {
        let pressed = combo.clone();
        let registry = shortcut_registry;
        let recording = recording_shortcut;
        let run = actions.run_command.clone();
        let rebind = rebind_shortcut.clone();
        on_key(combo.as_str(), move |_| {
            // 录制期间按键只用于改键，不执行命令。
            if let Some(id) = recording.get_untracked() {
                rebind(id, pressed.clone());
                return;
            }
            if let Some(id) = registry.get_untracked().action_for(pressed.as_str()) {
                run(id);
            }
        });
    }
    // 组件画廊：调试入口，独立于命令注册表，避免 P0 就改动快捷键清单。
    on_key("mod+shift+d", move |_| {
        gallery_open.set(!gallery_open.get_untracked());
    });

    // 命令面板的键盘导航：↑↓ 移动高亮，↵ 执行，打开面板时高亮回到第一行。
    on_key("down", {
        let palette_open = palette_open;
        let palette_selected = palette_selected;
        let palette_query = palette_query;
        let language = language;
        move |_| {
            if !palette_open.get_untracked() {
                return;
            }
            let open_paths: Vec<String> = tabs
                .get_untracked()
                .iter()
                .map(|tab| tab.path.clone())
                .collect();
            let recent_paths: Vec<String> = recent
                .get_untracked()
                .iter()
                .map(|entry| entry.path.clone())
                .collect();
            let count = screens::palette_rows(
                palette_query.get_untracked().as_str(),
                language.get_untracked().as_str(),
                &open_paths,
                &recent_paths,
            )
            .len();
            if count == 0 {
                return;
            }
            let next = (palette_selected.get_untracked() + 1).min(count - 1);
            palette_selected.set(next);
        }
    });

    on_key("up", {
        let palette_open = palette_open;
        let palette_selected = palette_selected;
        move |_| {
            if !palette_open.get_untracked() {
                return;
            }
            let next = palette_selected.get_untracked().saturating_sub(1);
            palette_selected.set(next);
        }
    });

    on_key("enter", {
        let palette_open = palette_open;
        let palette_selected = palette_selected;
        let palette_query = palette_query;
        let language = language;
        let run = run_command_for_key.clone();
        move |_| {
            if !palette_open.get_untracked() {
                return;
            }
            let open_paths: Vec<String> = tabs
                .get_untracked()
                .iter()
                .map(|tab| tab.path.clone())
                .collect();
            let recent_paths: Vec<String> = recent
                .get_untracked()
                .iter()
                .map(|entry| entry.path.clone())
                .collect();
            let rows = screens::palette_rows(
                palette_query.get_untracked().as_str(),
                language.get_untracked().as_str(),
                &open_paths,
                &recent_paths,
            );
            let index = palette_selected
                .get_untracked()
                .min(rows.len().saturating_sub(1));
            let Some(row) = rows.get(index).cloned() else {
                return;
            };
            match row {
                screens::PaletteRow::Command(id) => run(id),
                screens::PaletteRow::Repo { path, .. } => open_path(path),
            }
            palette_open.set(false);
        }
    });

    // Esc 层层退出：先关最上面的浮层，再退到过滤器；都不开时什么也不做。
    on_key("escape", {
        let filter_open = filter_open;
        let palette_open = palette_open;
        let changelist_open = changelist_open;
        let sync_state = sync_state;
        let form_state = form_state;
        let rewrite_state = rewrite_state;
        let tabs = tabs;
        let active = active;
        let recording = recording_shortcut;
        let gallery_open = gallery_open;
        move |_| {
            if gallery_open.get_untracked() {
                gallery_open.set(false);
                return;
            }
            // 录制中按 Esc 只取消录制，不继续关下面的浮层。
            if recording.get_untracked().is_some() {
                recording.set(None);
                return;
            }
            if rewrite_state.get_untracked().is_some() {
                rewrite_state.set(None);
                return;
            }
            if form_state.get_untracked().is_some() {
                form_state.set(None);
                return;
            }
            if sync_state.get_untracked().is_some() {
                sync_state.set(None);
                return;
            }
            if palette_open.get_untracked() {
                palette_open.set(false);
                return;
            }
            let current = active.get_untracked();
            let mut list = tabs.get_untracked();
            if let Some(tab) = list.get_mut(current)
                && tab.hook_open
            {
                tab.hook_open = false;
                tabs.set(list);
                return;
            }
            if changelist_open.get_untracked() {
                changelist_open.set(false);
                return;
            }
            filter_open.set(false);
        }
    });

    // 身份按当前仓库读（仓库级 config），没有打开仓库时回落到全局配置。
    // 用计算值而不是一次性读取：编辑身份或切换标签后设置页会跟着变。
    // 设置页的受保护分支列表：取当前仓库的本地分支名。
    let branches_for_settings = {
        let tabs = tabs;
        let active = active;
        tabs.get_untracked()
            .get(active.get_untracked())
            .map(|tab| {
                tab.refs
                    .branches
                    .iter()
                    .map(|branch| branch.name.clone())
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default()
    };
    let identity_for_view = {
        let tabs = tabs;
        let active = active;
        computed(move || {
            let list = tabs.get();
            if let Some(tab) = list.get(active.get()) {
                return model::identity(tab);
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
        })
    };
    // 设置页 Git 分区展示本机 Git 的版本、执行路径与凭据助手。
    let git_info_for_view = match git::info() {
        Ok(info) => (
            info.version,
            info.exec_path,
            git::credential_helper(std::path::Path::new(".")).unwrap_or_default(),
        ),
        Err(error) => (error.detail(), String::new(), String::new()),
    };

    // 欢迎页放在独立窗口里：它是入口页，不与多标签主窗口混在一起。
    // `open_window` 同一 id 只建一次；在组件主体里建一次即可。
    open_welcome_window(
        welcome_open,
        language,
        recent,
        welcome_query,
        clone_url,
        clone_destination,
        clone_options,
        init_files,
        init_path,
        clone_error,
        cloning,
        clone_progress,
        repo_form,
        actions.clone(),
    );

    let current_tab = computed(move || tabs.get().get(active.get()).cloned());
    let language_value = language;
    let screen_value = screen;
    let actions_for_view = actions.clone();

    return view()
        .w_full()
        .h_full()
        .flex_col()
        .overflow_hidden()
        .bg(move || design::content())
        .text_color(move || design::text())
        .child(
            view()
                .w_full()
                .h_full()
                .flex_col()
                .child(dynamic(move || -> AnyElement {
                    match screen_value.get() {
                        Screen::Settings => screens::settings(
                            language_value,
                            theme_mode,
                            settings_state,
                            settings_section,
                            identity_for_view.get(),
                            git_info_for_view.clone(),
                            agent_policies,
                            actions_for_view.set_agent_permission.clone(),
                            agent_settings,
                            branches_for_settings.clone(),
                            recent
                                .get()
                                .into_iter()
                                .map(|entry| entry.path)
                                .collect::<Vec<String>>(),
                            settings_state.get().use_system_git,
                            store::load_git_scan_folders(&store_dir)
                                .iter()
                                .map(|folder| folder.display().to_string())
                                .collect::<Vec<String>>(),
                            store::scan_git_repositories(&store::load_git_scan_folders(&store_dir))
                                .len(),
                            actions_for_view.add_found_repos.clone(),
                            actions_for_view.set_automation.clone(),
                            actions_for_view.set_review_millis.clone(),
                            actions_for_view.toggle_protected_branch.clone(),
                            screens::ShortcutUi {
                                registry: shortcut_registry,
                                query: shortcut_query,
                                recording: recording_shortcut,
                                error: shortcut_error,
                                record: record_shortcut.clone(),
                                restore: restore_shortcuts.clone(),
                            },
                            notice,
                            actions_for_view.clone(),
                        )
                        .into_element(),
                        Screen::Workspace => screens::workspace(
                            language_value,
                            tabs,
                            active,
                            history_query,
                            filter_reference,
                            filter_author,
                            filter_when,
                            filter_open,
                            inspector_open,
                            compare_mode,
                            mode,
                            commit_summary,
                            commit_description,
                            changelist_open,
                            settings_state,
                            agent_plan,
                            agent_audit,
                            agent_policies,
                            reset_request,
                            notice,
                            reflog_query,
                            diff_options,
                            diff_split,
                            actions_for_view.clone(),
                        )
                        .into_element(),
                    }
                })),
        )
        .child(screens::rewrite_dialog(
            language,
            rewrite_state,
            actions.set_rewrite_message.clone(),
            actions.submit_rewrite.clone(),
            actions.close_rewrite.clone(),
        ))
        .child(screens::conflict_view(
            language,
            current_tab,
            conflict_result,
            conflict_base_open,
            actions.clone(),
        ))
        .child(screens::hook_sheet(
            language,
            current_tab,
            actions.clone(),
        ))
        .child(screens::sync_dialog(
            language,
            current_tab,
            sync_state,
            actions.open_sync.clone(),
            actions.set_sync_option.clone(),
            actions.run_sync.clone(),
            actions.close_sync.clone(),
        ))
        .child(screens::form_dialog(
            language,
            form_state,
            submit_form.clone(),
            cancel_form.clone(),
        ))
        .child(dynamic(move || -> AnyElement {
            if !palette_open.get() {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            }
            let open_paths: Vec<String> = tabs
                .get()
                .iter()
                .map(|tab| tab.path.clone())
                .collect();
            let recent_paths: Vec<String> = recent
                .get()
                .iter()
                .map(|entry| entry.path.clone())
                .collect();
            let run_command = actions.run_command.clone();
            let open_repo = actions.open_path.clone();
            return screens::palette(
                language,
                palette_query,
                open_paths,
                recent_paths,
                run_command,
                open_repo,
                palette_open,
                palette_selected,
            )
            .into_element();
        }))
        .child(ui::gallery::gallery(gallery_open));
}

/// 构建原生菜单。`language` 取当前语言，切换语言后重建即可。
fn build_menu(
    language: &str,
    run: Arc<dyn Fn(&'static str)>,
    choose_folder: Arc<dyn Fn()>,
    combo: &dyn Fn(&str) -> String,
) -> Vec<MenuItem> {
    let lang = language;
    let menu = move |key: &'static str| i18n::t(lang, key);
    let menu_run = {
        let run = run.clone();
        move |id: &'static str| {
            let run = run.clone();
            return move || run(id);
        }
    };
    return vec![
        MenuItem::submenu(
            menu("menu.app.file"),
            vec![
                MenuItem::new(menu("menu.openRepo"))
                    .shortcut(&combo("open"))
                    .on_select({
                        let choose_folder = choose_folder.clone();
                        move || choose_folder()
                    }),
                MenuItem::new(menu("menu.closeTab"))
                    .shortcut(&combo("closeTab"))
                    .on_select(menu_run("closeTab")),
                MenuItem::separator(),
                MenuItem::role(MenuRole::Close),
            ],
        ),
        MenuItem::submenu(
            menu("menu.app.edit"),
            vec![
                MenuItem::role(MenuRole::Undo),
                MenuItem::role(MenuRole::Redo),
                MenuItem::separator(),
                MenuItem::role(MenuRole::Cut),
                MenuItem::role(MenuRole::Copy),
                MenuItem::role(MenuRole::Paste),
                MenuItem::role(MenuRole::SelectAll),
            ],
        ),
        MenuItem::submenu(
            menu("menu.app.view"),
            vec![
                MenuItem::new(menu("menu.commandPalette"))
                    .shortcut(&combo("openPalette"))
                    .on_select(menu_run("openPalette")),
                MenuItem::separator(),
                MenuItem::new(menu("menu.showHistory")).on_select(menu_run("history")),
                MenuItem::new(menu("menu.showChanges")).on_select(menu_run("changes")),
                MenuItem::new(menu("menu.toggleInspector")).on_select(menu_run("inspector")),
                MenuItem::separator(),
                MenuItem::new(menu("menu.cycleAppearance"))
                    .shortcut(&combo("toggleTheme"))
                    .on_select(menu_run("toggleTheme")),
                MenuItem::new(menu("menu.switchLanguage")).on_select(menu_run("toggleLanguage")),
            ],
        ),
        MenuItem::submenu(
            menu("menu.app.repository"),
            vec![
                MenuItem::new(menu("menu.fetch")).on_select(menu_run("fetch")),
                MenuItem::new(menu("menu.pull")).on_select(menu_run("pull")),
                MenuItem::new(menu("menu.push")).on_select(menu_run("push")),
                MenuItem::separator(),
                MenuItem::new(menu("menu.refresh"))
                    .shortcut(&combo("refresh"))
                    .on_select(menu_run("refresh")),
                MenuItem::new(menu("menu.repoManager")).on_select(menu_run("repoManager")),
                MenuItem::separator(),
                MenuItem::new(menu("menu.settings"))
                    .shortcut(&combo("settings"))
                    .on_select(menu_run("settings")),
            ],
        ),
        MenuItem::submenu(
            menu("menu.app.window"),
            vec![
                MenuItem::role(MenuRole::Minimize),
                MenuItem::role(MenuRole::Zoom),
                MenuItem::separator(),
                MenuItem::role(MenuRole::ToggleFullScreen),
            ],
        ),
        MenuItem::submenu(
            menu("menu.app.help"),
            vec![
                MenuItem::new(menu("menu.welcome")).on_select(menu_run("welcome")),
                MenuItem::separator(),
                MenuItem::role(MenuRole::About),
            ],
        ),
    ];
}

#[cfg(test)]
mod menu_tests {
    use super::*;

    /// 菜单文案全部来自 locales：两种语言都必须能取到，且内容不同。
    #[test]
    fn menu_labels_exist_in_both_languages() {
        let keys = [
            "menu.app.file",
            "menu.app.edit",
            "menu.app.view",
            "menu.app.repository",
            "menu.app.window",
            "menu.app.help",
            "menu.openRepo",
            "menu.closeTab",
            "menu.commandPalette",
            "menu.showHistory",
            "menu.showChanges",
            "menu.toggleInspector",
            "menu.cycleAppearance",
            "menu.switchLanguage",
            "menu.fetch",
            "menu.pull",
            "menu.push",
            "menu.refresh",
            "menu.repoManager",
            "menu.settings",
            "menu.welcome",
        ];
        for key in keys {
            let english = i18n::t(i18n::DEFAULT_LANGUAGE, key);
            let chinese = i18n::t(i18n::ZH_HANS_LANG, key);
            assert_ne!(english, key, "{key} 在英文里缺文案");
            assert_ne!(chinese, key, "{key} 在中文里缺文案");
            assert_ne!(english, chinese, "{key} 两种语言不应相同");
        }
    }
}

/// 打开欢迎页窗口（独立窗口，主窗口不受影响）。
///
/// 同一 id 只建一次；`open` 绑定到信号，从欢迎页打开仓库后窗口自动关闭。
#[allow(clippy::too_many_arguments)]
fn open_welcome_window(
    open: Signal<bool>,
    language: Signal<SharedString>,
    recent: Signal<Vec<store::RecentRepo>>,
    query: Signal<String>,
    clone_url: Signal<String>,
    clone_destination: Signal<String>,
    clone_options: Signal<screens::CloneOptions>,
    init_files: Signal<screens::InitFiles>,
    init_path: Signal<String>,
    clone_error: Signal<Option<String>>,
    cloning: Signal<bool>,
    clone_progress: Signal<String>,
    form: Signal<RepoForm>,
    actions: Actions,
) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static OPENED: AtomicBool = AtomicBool::new(false);
    if OPENED.swap(true, Ordering::SeqCst) {
        return;
    }
    let welcome = nsh_core::element::desktop::window()
        .id("welcome")
        .title(brand::PRODUCT_NAME)
        .default_size(size(Points(980.0), Points(620.0)))
        .open(open)
        .on_open_change(move |event| {
            if !event.open {
                open.set(false);
            }
        })
        .content(move || {
            return screens::welcome(
                language,
                recent,
                query,
                clone_url,
                clone_destination,
                clone_options,
                init_files,
                init_path,
                clone_error,
                cloning,
                clone_progress,
                form,
                actions.clone(),
            );
        });
    let _ = nsh_core::element::desktop::open_window(welcome);
}

/// 把打开的标签页写回会话，供下次启动恢复。
pub(crate) fn persist(dir: &std::path::Path, active: Signal<usize>, tabs: Signal<Vec<Tab>>) {
    let list = tabs.get_untracked();
    let paths: Vec<String> = list.iter().map(|tab| tab.path.clone()).collect();
    let session = store::Session {
        active: active.get_untracked().min(paths.len().saturating_sub(1)),
        paths,
    };
    store::save_session(dir, &session);
}

/// 窗口标题栏留白（系统窗口按钮占用的左右宽度）。
pub(crate) fn window_insets() -> (f32, f32) {
    let edges = title_bar();
    return (edges.left.0, edges.right.0);
}

/// 切到第 `slot` 个标签页；越界不动（不把用户送到空标签）。
fn activate_slot(
    tabs: Signal<Vec<Tab>>,
    active: Signal<usize>,
    screen: Signal<Screen>,
    slot: usize,
) {
    if slot < tabs.get_untracked().len() {
        active.set(slot);
        screen.set(Screen::Workspace);
    }
}

/// 前/后切标签页，到头回绕（符合 macOS 习惯）；不足两个标签时不动。
fn cycle_tab(
    tabs: Signal<Vec<Tab>>,
    active: Signal<usize>,
    screen: Signal<Screen>,
    forward: bool,
) {
    let count = tabs.get_untracked().len();
    if count < 2 {
        return;
    }
    let current = active.get_untracked();
    let next = if forward {
        (current + 1) % count
    } else {
        (current + count - 1) % count
    };
    active.set(next);
    screen.set(Screen::Workspace);
}

/// 供子模块使用的设计常量。
pub use design::metrics;

#[native_module("app/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}
