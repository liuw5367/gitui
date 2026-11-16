//! 根组件：窗口属性、主题装载、语言状态与屏幕路由。
//!
//! 一个窗口、多个仓库标签页；屏幕为欢迎页、仓库管理页与工作区。当前只读：
//! 打开/克隆/初始化仓库、按条件筛选历史、多提交对比、查看提交内单文件补丁。
//! 写路径在后续阶段加入。

mod acceptance;
pub mod brand;
mod dates;
mod design;
mod features;
mod filters;
mod i18n;
mod interactions;
mod model;
mod screens;
mod shared;
mod shell;
mod shortcuts;
// ui 层已建立；P1–P7 逐组迁入后仍有为后续阶段准备的组件，P8 收尾时
// 全部组件都有使用者，届时删除这条 allow 并保持零警告。
#[allow(dead_code)]
mod ui;
mod widgets;

use nsh_base::theme_overrides;
use nsh_core::desktop::{TitleBarOptions, TitleBarStyle, WindowAttributes};

use nsh_core::desktop::MenuRole;
use nsh_core::element::desktop::MenuItem;
use nsh_core::element::desktop::{main_window, title_bar};
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
    /// 单文件的 blame + 文件历史（板 19）。从检查器的 Blame 页或文件行进入，
    /// 不占工具栏的分段——画板的入口是 `.blamebar` 里的 Files 按钮。
    Blame,
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
        store::load_language(&store_dir).unwrap_or_else(|| i18n::DEFAULT_LANGUAGE.to_string()),
    ));

    // 历史筛选：关键字在工具栏搜索框，其余条件在 History 的筛选浮层。
    let history_query = signal(String::new());
    let filter_reference = signal(String::new());
    let filter_author = signal(String::new());
    let filter_when = signal(SharedString::from(When::All.id()));
    let filter_open = signal(false);
    // 验收开关（debug 构建）：`GITUI_INSPECTOR=1` 启动即展开右侧检查器（板 08 / 18）。
    let inspector_open = signal(acceptance::flag("GITUI_INSPECTOR"));
    let compare_mode = signal(false);
    // 验收开关：`GITUI_MODE=history|changes|refs|agent` 指定启动时的主视图，
    // 用于逐个巡检各页渲染（历史表曾整行不可见却连续几轮被截图判过）。
    let mode = signal(match acceptance::value("GITUI_MODE").as_deref() {
        Some("changes") => Mode::Changes,
        Some("refs") => Mode::Refs,
        Some("agent") => Mode::Agent,
        Some("blame") => Mode::Blame,
        _ => Mode::History,
    });
    let commit_summary = signal(String::new());
    let commit_description = signal(String::new());
    let changelist_open = signal(false);
    // 验收开关：`GITUI_PALETTE=1` 启动即打开命令面板（⌘K 的等价物），
    // 用于截图核对面板与列表项。
    let palette_open = signal(acceptance::flag("GITUI_PALETTE"));
    // 组件画廊（P0 的验收入口）：⌘⇧D 打开，Esc 关闭。P6 并入命令面板。
    // `GITUI_GALLERY=1` 用于截图验收：不依赖键盘自动化也能直接看到组件层。
    let gallery_open = signal(acceptance::flag("GITUI_GALLERY"));
    let settings_state = signal(store::load_settings(&store_dir));
    {
        // 日期格式是全局显示偏好，装进 dates 模块供各视图直接读取。
        let loaded = settings_state.get_untracked();
        crate::dates::load(loaded.date_format.as_str(), loaded.date_pattern.as_str());
    }
    // 验收开关：`GITUI_FORM=1` 启动即打开「新建分支」表单，用于核对表单弹层。
    let form_state = signal(if acceptance::flag("GITUI_FORM") {
        Some(screens::FormState::new(screens::FormKind::NewBranch))
    } else {
        None
    });
    // 状态栏右端的短提示：复制成功、外部打开失败等。
    let notice = signal(Option::<String>::None);
    // diff 的并排/合并切换（设计板 04）。
    let diff_split = signal(false);
    // diff 的三个显示开关（设计板 04）。
    let diff_options = signal(crate::shared::diff::DiffOptions::default());
    // 历史改写弹窗（改写说明 / 合并提交）。
    let rewrite_state = signal(Option::<features::commit::state::RewriteState>::None);
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
    let sync_state = signal(if acceptance::flag("GITUI_SYNC") {
        Some(features::sync::state::SyncDialogState::new(
            crate::model::Sync::Fetch,
        ))
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
    let settings_section = signal(features::settings::state::Section::General);
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
    if let Some(path) = acceptance::value("GITUI_REPO") {
        initial_tabs = vec![model::load_tab(&path, &initial_filter)];
    }
    // 验收开关：`GITUI_SETTINGS=1` 启动即进入设置页，用于截图核对。
    let screen = signal(if acceptance::flag("GITUI_SETTINGS") {
        Screen::Settings
    } else {
        Screen::Workspace
    });
    // 欢迎页是独立窗口：没有可恢复的标签页时打开它。
    // 验收开关：`GITUI_NO_WELCOME=1` 时不打开欢迎窗口。
    // 欢迎窗口是独立窗口且默认铺在主窗口之上，截图验收组件画廊时它会挡住画廊，
    // 之前把「截到欢迎页的空白」误当成「画廊没渲染」，就是因为它。
    let suppress_welcome = acceptance::flag("GITUI_NO_WELCOME");
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
    let clone_options = signal(features::welcome::state::CloneOptions::default());
    // Init 的附带文件（设计板 03）。
    let init_files = signal(features::welcome::state::InitFiles::default());
    let clone_error = signal(Option::<String>::None);
    let cloning = signal(false);
    // 克隆进度（git 的 stderr 最后一行）与取消标志。
    let clone_progress = signal(String::new());
    let clone_cancel: std::sync::Arc<std::sync::atomic::AtomicBool> =
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let init_path = signal(String::new());
    let repo_form = signal(features::welcome::state::RepoForm::Open);

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

    // 会话动作（打开仓库、返回欢迎页、重新定位、关页签等）收进 SessionHandle。
    let session = features::SessionHandle {
        tabs,
        active,
        screen,
        welcome_open,
        recent,
        store_dir: store_dir.clone(),
        filter: current_filter.clone(),
        notice,
        language,
    };

    // 恢复操作（P3 批 B）：撤销 reflog、重置、拣选与回滚收进 RecoveryCommands。
    // 没有独立视图，入口在 refs 的 reflog 表与 history 的提交右键菜单。
    let recovery_commands = features::recovery::commands::RecoveryCommands {
        session: session.clone(),
        language,
        notice,
    };

    // 提交历史（P2 竖切的第一片）：信号收进 HistoryState，用例收进 HistoryCommands。
    let history_state = features::history::state::HistoryState {
        query: history_query,
        reference: filter_reference,
        author: filter_author,
        when: filter_when,
        filter_open,
        compare_mode,
        filter: current_filter.clone(),
    };
    // diff 的显示开关由 history 与 changes 共用，因此放在 shared。
    let diff_commands = crate::shared::diff::DiffCommands {
        state: crate::shared::diff::DiffState {
            split: diff_split,
            options: diff_options,
        },
        settings: settings_state,
        store_dir: store_dir.clone(),
        session: session.clone(),
    };
    // 冲突解决（P3 批 A）：结果区与 base 显隐收进 MergeState，用例收进
    // MergeCommands；变更列表的冲突行要用它，所以先于 ChangesCommands 建。
    let merge_commands = features::merge::commands::MergeCommands {
        state: features::merge::state::MergeState {
            result: conflict_result,
            base_open: conflict_base_open,
        },
        session: session.clone(),
        language,
        notice,
    };
    // 提交（P3 批 A）：说明、描述与改写弹窗收进 CommitState，用例收进
    // CommitCommands；历史表的改写入口与空仓库的初始提交都要用它。
    let commit_commands = features::commit::commands::CommitCommands {
        state: features::commit::state::CommitState {
            summary: commit_summary,
            description: commit_description,
            rewrite: rewrite_state,
        },
        session: session.clone(),
        language,
        notice,
    };
    // Agent（P3 批 B）：计划、权限、运行设置与审计收进 AgentState，用例收进
    // AgentCommands；设置页的 Agent 分区与提交面板的「请 Agent 起草」都用它。
    let agent_commands = features::agent::commands::AgentCommands {
        state: features::agent::state::AgentState {
            plan: agent_plan,
            policies: agent_policies,
            settings: agent_settings,
            audit: agent_audit,
        },
        session: session.clone(),
        language,
        notice,
        store_dir: store_dir.clone(),
        commit_summary: commit_commands.state.summary,
    };
    let history_commands = features::history::commands::HistoryCommands {
        state: history_state,
        session: session.clone(),
        diff: diff_commands.clone(),
        commit: commit_commands.clone(),
        recovery: recovery_commands.clone(),
        mode,
        notice,
        language,
    };

    // 变更/暂存（P2 竖切的第二片）：清单浮层开关收进 ChangesState，用例收进
    // ChangesCommands；变更列表、选中项与选中补丁仍挂在 `Tab` 上。
    let changes_state = features::changes::state::ChangesState { changelist_open };
    let changes_commands = features::changes::commands::ChangesCommands {
        state: changes_state,
        session: session.clone(),
        language,
        notice,
        merge: merge_commands.clone(),
    };

    // 欢迎页（P3 批 B）：表单输入收进 WelcomeState，用例收进 WelcomeCommands。
    let welcome_commands = features::welcome::commands::WelcomeCommands {
        state: features::welcome::state::WelcomeState {
            query: welcome_query,
            clone_url,
            clone_destination,
            clone_options,
            init_files,
            init_path,
            clone_error,
            cloning,
            clone_progress,
            form: repo_form,
        },
        session: session.clone(),
        language,
        cancel: clone_cancel,
    };

    // 同步（P3 批 A）：面板状态收进 SyncState，用例收进 SyncCommands。
    let sync_commands = features::sync::commands::SyncCommands {
        state: features::sync::state::SyncState { dialog: sync_state },
        session: session.clone(),
        language,
        notice,
    };
    // 设置页（P3 批 B）：设置值、分区与快捷键编辑状态收进 SettingsState，用例收进
    // SettingsCommands。本机 Git 的版本、执行路径与凭据助手在启动时探测一次。
    let git_info = match git::info() {
        Ok(info) => (
            info.version,
            info.exec_path,
            git::credential_helper(std::path::Path::new(".")).unwrap_or_default(),
        ),
        Err(error) => (error.detail(), String::new(), String::new()),
    };
    let settings_commands = features::settings::commands::SettingsCommands {
        state: features::settings::state::SettingsState {
            settings_state,
            settings_section,
            shortcut_registry,
            shortcut_query,
            recording_shortcut,
            shortcut_error,
        },
        session: session.clone(),
        language,
        notice,
        store_dir: store_dir.clone(),
        theme_mode,
        diff: diff_commands.clone(),
        git_info,
    };

    // 搜索（P3 最后一片）：命令面板、视图模式与表单浮层收进 SearchState，命令分发
    // 收进 SearchCommands。原生菜单、快捷键与面板本身都经它分发。
    let search_commands = features::search::commands::SearchCommands {
        state: features::search::state::SearchState {
            palette_open,
            palette_selected,
            palette_query,
            mode,
            form: form_state,
        },
        session: session.clone(),
        language,
        welcome: welcome_commands.clone(),
        history: history_commands.clone(),
        commit: commit_commands.clone(),
        settings: settings_commands.clone(),
        sync: sync_commands.clone(),
        inspector_open,
    };

    // 引用管理（P3 批 A）：页签与 reflog 过滤收进 RefsState，用例收进 RefsCommands。
    // 「切到某个提交」的确认流程归恢复功能，这里只是持有它的用例。
    let refs_state = features::refs::state::RefsState { reflog_query };
    let refs_commands = features::refs::commands::RefsCommands {
        state: refs_state,
        session: session.clone(),
        language,
        notice,
        history: history_commands.clone(),
        recovery: recovery_commands.clone(),
        search: search_commands.clone(),
        sync: sync_commands.clone(),
    };

    // 原生菜单（设计板 22）：文案随语言重建。菜单项与命令面板共用同一批动作，
    // 因此不会指向空实现；系统角色项（关闭/撤销/最小化等）由系统给文案。
    {
        let run: Arc<dyn Fn(&'static str)> = {
            let search = search_commands.clone();
            Arc::new(move |id: &'static str| search.run_command(id))
        };
        let choose_folder: Arc<dyn Fn()> = {
            let welcome = welcome_commands.clone();
            Arc::new(move || welcome.choose_folder())
        };
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

    // 快捷键：把清单里的每个组合各绑一次，按下时查注册表决定跑哪个命令。
    // 注册表是可改的（设置页 25 的 Record / Restore Defaults），而 nsh 0.5.0 的
    // on_key 只能在挂载时绑定一次，所以绑定的是「组合」而不是「动作」。
    for combo in shortcuts::catalog() {
        let pressed = combo.clone();
        let settings_cmd = settings_commands.clone();
        let registry = settings_cmd.state.shortcut_registry;
        let recording = settings_cmd.state.recording_shortcut;
        let run: Arc<dyn Fn(&'static str)> = {
            let search = search_commands.clone();
            Arc::new(move |id: &'static str| search.run_command(id))
        };
        on_key(combo.as_str(), move |_| {
            // 录制期间按键只用于改键，不执行命令。
            if let Some(id) = recording.get_untracked() {
                settings_cmd.rebind_shortcut(id, pressed.clone());
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
    // 结果表由 `SearchCommands` 自己算，和高亮用的是同一份。
    on_key("down", {
        let search = search_commands.clone();
        move |_| search.move_selection(true)
    });
    on_key("up", {
        let search = search_commands.clone();
        move |_| search.move_selection(false)
    });
    on_key("enter", {
        let search = search_commands.clone();
        move |_| search.activate_selected()
    });

    // Esc 层层退出：先关最上面的浮层，再退到过滤器；都不开时什么也不做。
    on_key("escape", {
        let sync_commands_for_esc = sync_commands.clone();
        let filter_open = history_commands.state.filter_open;
        let search_state = search_commands.state;
        let palette_open = search_state.palette_open;
        let changelist_open = changes_commands.state.changelist_open;
        let sync_state = sync_commands_for_esc.state.dialog;
        let form_state = search_state.form;
        let rewrite_state = commit_commands.state.rewrite;
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

    // 欢迎页放在独立窗口里：它是入口页，不与多标签主窗口混在一起。
    // `open_window` 同一 id 只建一次；在组件主体里建一次即可。
    open_welcome_window(welcome_commands.clone());

    let current_tab = computed(move || tabs.get().get(active.get()).cloned());
    let language_value = language;
    let screen_value = screen;
    let history_for_view = history_commands.clone();
    let changes_for_view = changes_commands.clone();
    let refs_for_view = refs_commands.clone();
    // 设置页与工作区各用一份 Agent 用例：`dynamic` 里两屏共用同一个变量。
    let agent_for_view = agent_commands.clone();
    // 同步面板在 `dynamic` 之外渲染，工作区在它里面：两份克隆，避免被提前移走。
    let sync_for_view = sync_commands.clone();
    let sync_for_body = sync_commands.clone();
    // 屏幕分支的 `dynamic` 会拿走一份 `session`，后面命令面板还要用。
    let session_for_screen = session.clone();
    // 工作区与三个浮层各取一份，避免被提前移走。
    let commit_for_workspace = commit_commands.clone();
    let merge_for_workspace = merge_commands.clone();
    let commit_for_dialogs = commit_commands.clone();
    let merge_for_dialogs = merge_commands.clone();
    // 搜索功能：屏幕分支、表单浮层与命令面板各取一份（状态是同一组信号）。
    let search_for_screen = search_commands.clone();
    let search_for_form = search_commands.clone();
    let search_for_palette = search_commands.clone();

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
                        Screen::Settings => features::settings::view::settings(
                            &settings_commands.state,
                            &settings_commands,
                            current_tab.get(),
                            &refs_for_view,
                            &agent_for_view,
                        )
                        .into_element(),
                        Screen::Workspace => screens::workspace(
                            language_value,
                            tabs,
                            active,
                            inspector_open,
                            changes_for_view.clone(),
                            settings_state,
                            agent_for_view.clone(),
                            notice,
                            refs_for_view.clone(),
                            sync_for_body.clone(),
                            history_for_view.clone(),
                            merge_for_workspace.clone(),
                            commit_for_workspace.clone(),
                            session_for_screen.clone(),
                            settings_commands.clone(),
                            search_for_screen.clone(),
                        )
                        .into_element(),
                    }
                })),
        )
        .child(features::commit::rewrite_dialog::rewrite_dialog(
            &commit_for_dialogs.state,
            &commit_for_dialogs,
            language,
        ))
        .child(features::merge::view::conflict_view(
            &merge_for_dialogs.state,
            &merge_for_dialogs,
            language,
            current_tab,
        ))
        .child(features::commit::hook_sheet::hook_sheet(
            &commit_for_dialogs.state,
            &commit_for_dialogs,
            language,
            current_tab,
        ))
        .child(features::sync::view::sync_dialog(
            &sync_for_view.state,
            &sync_for_view,
            language,
            current_tab,
        ))
        .child(screens::form_dialog(
            language,
            search_for_form.state.form,
            {
                let search = search_for_form.clone();
                Arc::new(move |form: screens::FormState| search.submit_form(form))
            },
            {
                let search = search_for_form.clone();
                Arc::new(move || search.cancel_form())
            },
        ))
        .child(dynamic(move || -> AnyElement {
            if !search_for_palette.state.palette_open.get() {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            }
            let open_paths: Vec<String> = tabs.get().iter().map(|tab| tab.path.clone()).collect();
            let recent_paths: Vec<String> = recent
                .get()
                .iter()
                .map(|entry| entry.path.clone())
                .collect();
            return features::search::view::palette(
                &search_for_palette.state,
                &search_for_palette,
                open_paths,
                recent_paths,
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
fn open_welcome_window(cmd: features::welcome::commands::WelcomeCommands) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static OPENED: AtomicBool = AtomicBool::new(false);
    if OPENED.swap(true, Ordering::SeqCst) {
        return;
    }
    let open = cmd.session.welcome_open;
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
            return features::welcome::view::welcome(&cmd.state, &cmd);
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

/// 供子模块使用的设计常量。
pub use design::metrics;

#[native_module("app/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}
