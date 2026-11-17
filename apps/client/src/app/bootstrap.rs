//! 应用引导：建窗口、装主题与语言、构造各功能状态与句柄，再组装首屏。
//!
//! 本文件是原 `lib.rs` 的 `App()` 主体；页面版式在 `app/workspace.rs`，菜单、快捷键
//! 与窗口函数分别在 `menu.rs`、`hotkeys.rs`、`windows.rs`。

use crate::app::handles::AppHandles;
use crate::app::session::Tab;
use crate::brand::ThemeMode;
use crate::filters::When;
use crate::{Mode, acceptance, brand, design, features, filters, i18n, shortcuts, ui};
use git::LogFilter;
use nsh_base::theme_overrides;
use nsh_core::desktop::{TitleBarOptions, TitleBarStyle, WindowAttributes};
use nsh_core::element::desktop::main_window;
use nsh_core::element::set_tokens;
use nsh_core::prelude::*;
use std::sync::Arc;

/// `theme.json` 与设计稿 CSS 的 light/dark 值一一对应。
const THEME_JSON: &str = include_str!("../../theme.json");

/// 应用根组件的实体：构造状态与句柄，再把首屏交给 `app::workspace` 组装。
pub(crate) fn app() -> impl IntoElement {
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
        Some(features::search::FormState::new(
            features::search::FormKind::NewBranch,
        ))
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
    let restore_confirm = signal(acceptance::flag("GITUI_RESTORE_CONFIRM"));
    let blink_on = signal(true);
    let blink_running = signal(false);
    let pending_keystroke = signal(
        acceptance::value("GITUI_PENDING_KEY").map(|combo| combo.to_string()),
    );
    // reflog 的过滤关键字（动作或说明）。
    let reflog_query = signal(String::new());
    // 冲突视图：可编辑的结果区与 base 显隐。
    let conflict_result = signal(String::new());
    let conflict_base_open = signal(false);
    // 验收开关：`GITUI_SYNC=1` 启动即打开同步面板，用于截图核对弹层外观
    // （遮罩深浅、圆角、阴影、关闭行为）。与 GITUI_GALLERY 同思路。
    let sync_state = signal(if acceptance::flag("GITUI_SYNC") {
        Some(features::sync::state::SyncDialogState::new(
            crate::app::session::Sync::Fetch,
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
    let settings_section = {
        let section = signal(features::settings::state::Section::General);
        if let Some(name) = acceptance::value("GITUI_SETTINGS_SECTION") {
            section.set(match name.as_str() {
                "shortcuts" => features::settings::state::Section::Shortcuts,
                "agent" => features::settings::state::Section::Agent,
                "git" => features::settings::state::Section::Git,
                "credentials" => features::settings::state::Section::Credentials,
                "about" => features::settings::state::Section::About,
                _ => features::settings::state::Section::General,
            });
        }
        section
    };
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
        .map(|path| crate::app::session::load_tab(path, &initial_filter))
        // 打不开的仓库仍保留标签页：横幅给出「重新定位 / 在此初始化」，而不是
        // 让它在启动时悄悄消失（手册 TC-01）。
        .filter(|tab| tab.error.is_none() || tab.problem != crate::app::session::RepoProblem::None)
        .collect();
    if !initial_settings.restore_session {
        initial_tabs.clear();
    }
    // 验收与命令行入口：`GITUI_REPO=/path/to/repo` 直接打开指定仓库，
    // 优先于会话恢复。截图验收需要它——没有打开的仓库，历史表与变更页
    // 根本不会渲染，截图只能看到欢迎页。
    if let Some(path) = acceptance::value("GITUI_REPO") {
        initial_tabs = vec![crate::app::session::load_tab(&path, &initial_filter)];
    }
    // 验收开关：`GITUI_SETTINGS=1` 启动即打开设置窗口，用于截图核对。
    let settings_open = signal(acceptance::flag("GITUI_SETTINGS"));
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
    // 验收开关：欢迎页的表单与状态；只影响 debug 构建。
    let welcome_state = acceptance::value("GITUI_WELCOME_STATE");
    let welcome_form = acceptance::value("GITUI_WELCOME_FORM");
    if welcome_state.as_deref() == Some("recent")
        && let Some(entries) = acceptance::value("GITUI_WELCOME_RECENT")
    {
        let mut list = store::load_recent(&store_dir);
        for entry in entries.split(':').filter(|entry| !entry.trim().is_empty()) {
            let (path, branch) = match entry.split_once('|') {
                Some((path, branch)) => (path.trim(), Some(branch.trim())),
                None => (entry.trim(), None),
            };
            if path.is_empty() {
                continue;
            }
            store::remember_recent(&store_dir, &list, path);
            if let Some(branch) = branch.filter(|branch| !branch.is_empty()) {
                store::set_recent_branch(&store_dir, path, branch);
            }
            list = store::load_recent(&store_dir);
        }
    }
    let recent = signal(store::load_recent(&store_dir));

    // 欢迎页与仓库管理页的输入框。
    let welcome_query = signal(String::new());
    let clone_url = signal(String::new());
    let clone_destination = signal(String::new());
    // Clone 的额外选项（设计板 03）合成一个结构，避免把四个信号串到窗口里。
    let clone_options = signal(features::welcome::state::CloneOptions::default());
    // Init 的附带文件（设计板 03）。
    let init_files = signal(features::welcome::state::InitFiles::default());
    let welcome_open_path = signal(String::new());
    let open_error = signal(Option::<String>::None);
    let clone_error = signal(Option::<String>::None);
    let init_error = signal(Option::<String>::None);
    if welcome_state.as_deref() == Some("clone-error") {
        clone_error.set(Some(i18n::t(
            language.get_untracked().as_str(),
            "welcome.error.commandFailed",
        )));
    }
    let cloning = signal(false);
    // 克隆进度（git 的 stderr 最后一行）与取消标志。
    let clone_progress = signal(String::new());
    let clone_cancel: std::sync::Arc<std::sync::atomic::AtomicBool> =
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let init_path = signal(String::new());
    let init_name = signal(String::new());
    let init_branch = signal("main".to_string());
    let repo_form = signal(if welcome_state.as_deref() == Some("clone-error") {
        features::welcome::state::RepoForm::Clone
    } else {
        match welcome_form.as_deref() {
            Some("clone") => features::welcome::state::RepoForm::Clone,
            Some("init") => features::welcome::state::RepoForm::Init,
            _ => features::welcome::state::RepoForm::Open,
        }
    });
    let welcome_now = signal(store::now_unix());
    let welcome_search_ref = element_ref();

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
        welcome_open,
        settings_open,
        recent,
        store_dir: store_dir.clone(),
        filter: current_filter.clone(),
        notice,
        language,
    };
    // 首屏直接落在欢迎页时，列表里的分支 tag 要先按仓库当前 HEAD 刷新一次。
    if session.welcome_open.get_untracked() {
        session.refresh_recent_branches();
    }

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
    let git_folders = store::load_git_scan_folders(&store_dir);
    let git_counts: Vec<usize> = git_folders
        .iter()
        .map(|folder| store::scan_git_repositories(std::slice::from_ref(folder)).len())
        .collect();
    let git_found = store::scan_git_repositories(&git_folders);
    let git_last = if git_found.is_empty() {
        None
    } else {
        Some(store::now_unix())
    };
    let git_selected = git_found.clone();

    let settings_commands = features::settings::commands::SettingsCommands {
        state: features::settings::state::SettingsState {
            settings_state,
            settings_section,
            shortcut_registry,
            shortcut_query,
            recording_shortcut,
            shortcut_error,
            restore_confirm,
            pending_keystroke,
            blink_on,
            blink_running,
            identity_name: signal(String::new()),
            identity_email: signal(String::new()),
            identity_use_global: signal(true),
            date_pattern: signal(settings_state.get_untracked().date_pattern),
            git_version: signal(git_info.0.clone()),
            git_exec_path: signal(git_info.1.clone()),
            credential_helper: signal(git_info.2.clone()),
            git_path: signal(settings_state.get_untracked().git_path),
            scan_folders: signal(git_folders),
            scan_counts: signal(git_counts),
            scan_found: signal(git_found),
            scan_selected: signal(git_selected),
            scan_last_at: signal(git_last),
            agent_base_url: signal(settings_state.get_untracked().agent_base_url),
            agent_temperature: signal(settings_state.get_untracked().agent_temperature.to_string()),
            agent_max_tokens: signal(settings_state.get_untracked().agent_max_tokens.to_string()),
            agent_app_key: signal(String::new()),
            ssh_keys: signal(features::settings::ssh::read_keys()),
        },
        session: session.clone(),
        language,
        notice,
        store_dir: store_dir.clone(),
        theme_mode,
        diff: diff_commands.clone(),
    };
    settings_commands.refresh_identity();
    // 验收开关：`GITUI_RECORD=<动作 id>` 启动即进入该动作的录制态，截图核对录制卡。
    if let Some(id) = acceptance::value("GITUI_RECORD") {
        let id = crate::shortcuts::ACTIONS
            .iter()
            .find(|action| action.id == id.as_str())
            .map(|action| action.id);
        if let Some(id) = id {
            settings_commands.record_shortcut(id);
            // `record_shortcut` 会清掉上一条回显；验收时把种子组合补回去，
            // 好让截图能核对「按下的键」那一行。
            if let Some(combo) = acceptance::value("GITUI_PENDING_KEY") {
                settings_commands.state.pending_keystroke.set(Some(combo));
            }
        }
    }

    // 欢迎页（P3 批 B）：表单输入收进 WelcomeState，用例收进 WelcomeCommands。
    let welcome_commands = features::welcome::commands::WelcomeCommands {
        state: features::welcome::state::WelcomeState {
            query: welcome_query,
            clone_url,
            clone_destination,
            clone_options,
            init_files,
            init_path,
            init_name,
            init_branch,
            open_path: welcome_open_path,
            open_error,
            clone_error,
            init_error,
            cloning,
            clone_progress,
            form: repo_form,
            search_ref: welcome_search_ref,
            now: welcome_now,
        },
        session: session.clone(),
        settings: settings_commands.clone(),
        language,
        cancel: clone_cancel,
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

    // 各功能的句柄收成一个结构：主窗口版式（`app::workspace`）只取这一个参数。
    let handles = AppHandles {
        session: session.clone(),
        history: history_commands.clone(),
        changes: changes_commands.clone(),
        commit: commit_commands.clone(),
        refs: refs_commands.clone(),
        sync: sync_commands.clone(),
        merge: merge_commands.clone(),
        agent: agent_commands.clone(),
        settings: settings_commands.clone(),
        search: search_commands.clone(),
        welcome: welcome_commands.clone(),
        recovery: recovery_commands.clone(),
        diff: diff_commands.clone(),
        language,
    };

    // 欢迎页的相对时间与分组每分钟刷新一次，窗口开着时不会停在挂载时刻。
    {
        let welcome_now = welcome_now;
        spawn(async move {
            loop {
                timer(std::time::Duration::from_secs(60)).await;
                welcome_now.set(store::now_unix());
            }
        });
    }

    // 原生菜单（设计板 22）：文案随语言重建。菜单项与命令面板共用同一批动作，
    // 因此不会指向空实现；系统角色项（关闭/撤销/最小化等）由系统给文案。
    {
        let run: Arc<dyn Fn(&'static str)> = {
            let search = handles.search.clone();
            Arc::new(move |id: &'static str| search.run_command(id))
        };
        let choose_folder: Arc<dyn Fn()> = {
            let welcome = handles.welcome.clone();
            Arc::new(move || welcome.choose_folder())
        };
        effect({
            let language = language;
            let shortcuts = shortcut_registry;
            let recording = settings_commands.state.recording_shortcut;
            move || {
                let lang = language.get();
                // 菜单里的加速键也取注册表：改键后菜单跟着变。
                let bindings = shortcuts.get();
                let combo = move |id: &str| bindings.combo(id).to_string();
                // 录制期间临时摘掉菜单加速键：原生菜单会先于窗口按键表消费组合键。
                let shortcuts_enabled = recording.get().is_none();
                let items = crate::app::menu::build_menu(
                    lang.as_str(),
                    run.clone(),
                    choose_folder.clone(),
                    &combo,
                    shortcuts_enabled,
                );
                nsh_core::element::desktop::set_application_menu(items);
            }
        });
    }

    // 快捷键与命令面板导航；Esc 的浮层退出顺序也按同一份状态处理。
    crate::app::hotkeys::install(&handles, gallery_open);

    // 欢迎与设置是两个独立窗口，由 `app::windows` 以声明式元素挂在这里：
    // 它们的开与关只跟 `welcome_open` / `settings_open` 走。
    let current_tab = computed(move || tabs.get().get(active.get()).cloned());
    let language_value = language;
    let handles_for_screen = handles.clone();
    let search_for_overlay = handles.search.clone();

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
                .child(crate::app::workspace::workspace(&handles_for_screen)),
        )
        // 两个独立窗口（欢迎、设置）。它们的内容渲染在各自的窗口里。
        .child(crate::app::windows::windows(&handles))
        .child(features::commit::rewrite_dialog::rewrite_dialog(
            &handles.commit.state,
            &handles.commit,
            language_value,
        ))
        .child(features::merge::view::conflict_view(
            &handles.merge.state,
            &handles.merge,
            language_value,
            current_tab,
        ))
        .child(features::commit::hook_sheet::hook_sheet(
            &handles.commit.state,
            &handles.commit,
            language_value,
            current_tab,
        ))
        .child(features::sync::view::sync_dialog(
            &handles.sync.state,
            &handles.sync,
            language_value,
            current_tab,
        ))
        // 表单浮层只在一扇窗口里画：设置窗口开着时归它（身份编辑从设置页进入）。
        .child(dynamic(move || -> AnyElement {
            if settings_open.get() {
                return view().w(px(0.0)).h(px(0.0)).into_element();
            }
            features::search::form_overlay(&search_for_overlay, language_value).into_element()
        }))
        .child(dynamic({
            let search = handles.search.clone();
            move || -> AnyElement {
                if !search.state.palette_open.get() {
                    return view().w(px(0.0)).h(px(0.0)).into_element();
                }
                let open_paths: Vec<String> =
                    tabs.get().iter().map(|tab| tab.path.clone()).collect();
                let recent_paths: Vec<String> = recent
                    .get()
                    .iter()
                    .map(|entry| entry.path.clone())
                    .collect();
                return features::search::view::palette(
                    &search.state,
                    &search,
                    open_paths,
                    recent_paths,
                )
                .into_element();
            }
        }))
        .child(ui::gallery::gallery(gallery_open));
}
