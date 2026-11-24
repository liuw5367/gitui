//! 应用引导：建窗口、装主题与语言、构造各功能状态与句柄，再组装首屏。
//!
//! 本文件是原 `lib.rs` 的 `App()` 主体；页面版式在 `app/workspace.rs`，菜单、快捷键
//! 与窗口函数分别在 `menu.rs`、`hotkeys.rs`、`windows.rs`。

use crate::app::handles::AppHandles;
use crate::app::session::Tab;
use crate::brand::ThemeMode;
use crate::filters::FilterDraft;
use crate::{Mode, acceptance, brand, design, features, i18n, shortcuts, ui};
use git::LogFilter;
use nsh_base::theme_overrides;
use nsh_core::desktop::{CloseReason, TitleBarOptions, TitleBarStyle, WindowAttributes};
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
    title_bar.height = Some(Points(crate::design::metrics::TITLEBAR_H));
    let mut window = WindowAttributes::default();
    window.title = Some(brand::PRODUCT_NAME.to_string());
    window.title_bar = Some(title_bar);
    window.min_size = Some(Some(size(Points(1100.0), Points(700.0))));
    window.close_on_press = Some(false);
    main_window().set(window);
    // 属性会把隐藏标题栏（44pt）加进 frame：窗口打开时的居中因此偏上 43pt。属性与
    // 居中按顺序入队，同一批应用，拿到的是最终尺寸。
    main_window().center(None);

    let store_dir = store::default_dir(brand::STORAGE_DIR);
    let language = signal(SharedString::from(
        store::load_language(&store_dir).unwrap_or_else(|| i18n::DEFAULT_LANGUAGE.to_string()),
    ));
    let (askpass_ui, askpass_client, askpass_receiver) = crate::askpass::channel();
    askpass_ui.install(askpass_receiver);

    // 历史快速搜索与筛选浮层草稿分开；草稿在应用前不影响历史结果。
    let history_query = signal(String::new());
    let filter_draft = signal(FilterDraft::default());
    let filter_open = signal(false);
    // 验收开关：`GITUI_MODE` 指定启动主视图；reflog 用于核对 Reflog 列表与检查器。
    // blame 模式会预选首个可追溯文件；未指定时默认进入 Changes + Commit。
    let initial_reflog = acceptance::value("GITUI_MODE").as_deref() == Some("reflog");
    let mode = signal(match acceptance::value("GITUI_MODE").as_deref() {
        Some("history") => Mode::History,
        Some("changes") => Mode::Changes,
        Some("refs" | "reflog") => Mode::Refs,
        Some("agent") => Mode::Agent,
        Some("blame") => Mode::Blame,
        _ => Mode::Changes,
    });
    // 默认打开左侧引用栏；右栏默认显示 Changes 对应的 Commit 面板。
    // GITUI_MODE 可覆盖主视图，History 验收时用 GITUI_INSPECTOR=1 展开 Inspector。
    let sidebar_open = signal(true);
    let right_panel_open = signal(
        matches!(mode.get_untracked(), Mode::Changes | Mode::Blame)
            || initial_reflog
            || acceptance::flag("GITUI_INSPECTOR"),
    );
    let compare_mode = signal(false);
    let commit_summary = signal(String::new());
    let commit_description = signal(String::new());
    let changelist_open = signal(false);
    let rename_changelist = signal(None);
    let rename_changelist_value = signal(String::new());
    let collapsed_changelists = signal(std::collections::BTreeMap::new());
    let changelist_tree_expanded = signal(std::collections::BTreeMap::new());
    // 验收开关：`GITUI_PALETTE=1` 启动即打开命令面板（⌘K 的等价物），
    // 用于截图核对面板与列表项。
    let palette_open = signal(acceptance::flag("GITUI_PALETTE"));
    // 组件画廊是独立验收窗口：⌘⇧D 打开或关闭；`GITUI_GALLERY=1` 用于截图启动时打开。
    let gallery_open = signal(acceptance::flag("GITUI_GALLERY"));
    let settings_state = signal(store::load_settings(&store_dir));
    let git_executable: Arc<dyn Fn() -> git::GitExecutable + Send + Sync> = {
        let settings_state = settings_state;
        Arc::new(move || {
            let settings = settings_state.get_untracked();
            if settings.use_system_git {
                git::GitExecutable::system()
            } else {
                git::GitExecutable::from_path(settings.git_path)
            }
        })
    };
    let initial_git_executable = git_executable();
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
    // diff 的布局与显示选项（设计板 04）。
    let diff_options = signal(crate::shared::diff::DiffOptions::default());
    let diff_ignore_whitespace = signal(false);
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
    let pending_keystroke =
        signal(acceptance::value("GITUI_PENDING_KEY").map(|combo| combo.to_string()));
    // Refs / Reflog 的入口与中栏页签共用状态；右栏详情只在明确选择记录后读取。
    let refs_selected_tab = signal(if initial_reflog {
        features::refs::state::RefsTab::Reflog
    } else {
        features::refs::state::RefsTab::Refs
    });
    // reflog 的过滤关键字（动作或说明）。
    let reflog_query = signal(String::new());
    let reflog_selection = signal(None);
    let reflog_detail = signal(None);
    // 冲突视图的 Base 区域显隐。
    let conflict_base_open = signal(false);
    let conflict_files_open = signal(false);
    let conflict_files_mode = signal(features::merge::state::ConflictFilesMode::Tree);
    // 验收开关：`GITUI_SYNC=1` 启动即打开同步面板，用于截图核对弹层外观
    // （遮罩深浅、圆角、阴影、关闭行为）。与 GITUI_GALLERY 同思路。
    let sync_state = signal(if acceptance::flag("GITUI_SYNC") {
        Some(features::sync::state::SyncDialogState::new(
            crate::app::session::Sync::Fetch,
            None,
            None,
        ))
    } else {
        None
    });
    let agent_plan = signal(Option::<agent::Plan>::None);
    let stored_agent_preferences = store::load_agent_preferences(&store_dir);
    let agent_policies = signal({
        let mut policies = agent::Policies::default();
        for (action, permission) in &stored_agent_preferences.permissions {
            let matched = agent::Action::ALL
                .into_iter()
                .find(|candidate| candidate.id() == action);
            if let Some(action) = matched {
                policies.set(action, agent::Permission::from_id(permission.as_str()));
            }
        }
        policies
    });
    let agent_settings = signal(agent::Settings {
        automation: agent::Automation::from_id(stored_agent_preferences.automation.as_str()),
        review_millis: stored_agent_preferences.review_millis,
        protected: stored_agent_preferences.protected_branches,
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
    let session = store::load_session(&store_dir);
    let initial_settings = settings_state.get_untracked();
    let initial_filter = LogFilter::default();
    let mut initial_tabs: Vec<Tab> = if initial_settings.restore_session {
        session
            .paths
            .iter()
            .map(|path| {
                crate::app::session::load_tab_with_executable(
                    path,
                    &initial_filter,
                    &initial_git_executable,
                )
            })
            .collect()
    } else {
        Vec::new()
    };
    // 验收与命令行入口：`GITUI_REPO=/path/to/repo` 直接打开指定仓库，
    // 优先于会话恢复。截图验收需要它——没有打开的仓库，历史表与变更页
    // 根本不会渲染，截图只能看到欢迎页。
    if let Some(path) = acceptance::value("GITUI_REPO") {
        initial_tabs = vec![crate::app::session::load_tab_with_executable(
            &path,
            &initial_filter,
            &initial_git_executable,
        )];
    }
    if mode.get_untracked() == Mode::Blame
        && let Some(tab) = initial_tabs.first_mut()
        && tab.error.is_none()
    {
        if !tab.files.is_empty() {
            features::history::ops::select_file(tab, 0);
            tab.blame_return_mode = Some(Mode::History);
        } else if let Some(change) = tab.changes.first() {
            let path = change.path.clone();
            features::history::ops::open_blame(tab, path.as_str(), "HEAD", Mode::Changes);
        }
    }
    let preview_conflict = acceptance::flag("GITUI_CONFLICT");
    let preview_conflict_files = acceptance::flag("GITUI_CONFLICT_FILES");
    if (preview_conflict || preview_conflict_files)
        && let Some(tab) = initial_tabs
            .first_mut()
            .filter(|tab| tab.operation != git::Operation::None && !tab.conflicts.is_empty())
    {
        features::merge::ops::open_conflict(tab);
        if preview_conflict_files {
            conflict_files_open.set(true);
        }
    }
    // 验收开关：`GITUI_SETTINGS=1` 启动即打开设置窗口，用于截图核对。
    let settings_open = signal(acceptance::flag("GITUI_SETTINGS"));
    // 欢迎页是独立窗口：没有可恢复的标签页时打开它。
    // 验收开关：`GITUI_NO_WELCOME=1` 时不打开欢迎窗口。
    // 欢迎页与组件画廊都是独立窗口；截图脚本关闭欢迎页，避免误截较小的欢迎窗口。
    let suppress_welcome = acceptance::flag("GITUI_NO_WELCOME");
    let welcome_open = signal(initial_tabs.is_empty() && !suppress_welcome);
    let active = signal(session.active.min(initial_tabs.len().saturating_sub(1)));
    if initial_tabs.is_empty() {
        initial_tabs.clear();
    }
    let tabs = signal(initial_tabs);
    let quit_tabs = tabs;
    let quit_active = active;
    let quit_store_dir = store_dir.clone();
    nsh_core::element::desktop::on_app_quit(move || {
        crate::persist(&quit_store_dir, quit_active, quit_tabs);
        async {}
    });
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
        let tabs = tabs;
        let active = active;
        Arc::new(move || {
            return tabs
                .get_untracked()
                .get(active.get_untracked())
                .map(|tab| tab.filter.clone())
                .unwrap_or_default();
        })
    };

    // 会话动作（打开仓库、返回欢迎页、重新定位、关页签等）收进 SessionHandle。
    let session = features::SessionHandle {
        tabs,
        active,
        task_owner: Owner::current(),
        welcome_open,
        settings_open,
        recent,
        store_dir: store_dir.clone(),
        filter: current_filter.clone(),
        git_executable: git_executable.clone(),
        notice,
        language,
        auxiliary: false,
        persist_session: true,
        window: signal(None),
        welcome_window: signal(None),
        settings_window: signal(None),
    };
    let close_session = session.clone();
    main_window().on_close_press(move |reason| {
        if reason == CloseReason::System {
            nsh_core::element::desktop::quit();
        } else {
            close_session.close_workspace();
        }
    });
    let reopen_session = session.clone();
    main_window().on_focus_change(move |focused| {
        if !focused || suppress_welcome || !reopen_session.tabs.get_untracked().is_empty() {
            return;
        }
        main_window().hide();
        if !reopen_session.welcome_open.get_untracked() && !suppress_welcome {
            reopen_session.go_welcome();
        }
    });
    if !session.welcome_open.get_untracked() {
        main_window().show();
    }
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
        draft: filter_draft,
        filter_open,
        filter_position: signal((0.0, 0.0)),
        filter_menu: signal(None),
        filter_tab_id: signal(None),
        quick_filter_baseline: signal(None),
        file_tree_expanded: signal(None),
        compare_mode,
        compare_refs_open: signal(false),
        compare_refs_query: signal(String::new()),
        compare_refs_draft: signal(features::history::state::CompareRefsDraft::default()),
        fast_selection: signal(None),
        multi_commit_selection: signal(None),
        multi_commit_data: signal(None),
        commit_info_loading: signal(Vec::new()),
        commit_load_queue: std::sync::Arc::default(),
        table_ref: component_ref(),
        scroll_request: signal(None),
        filter: current_filter.clone(),
        author_column_width: signal(features::history::state::DEFAULT_AUTHOR_COLUMN_WIDTH),
        date_column_width: signal(features::history::state::DEFAULT_DATE_COLUMN_WIDTH),
    };
    // diff 的显示开关由 history 与 changes 共用，因此放在 shared。
    let diff_commands = crate::shared::diff::DiffCommands {
        state: crate::shared::diff::DiffState {
            split: diff_split,
            options: diff_options,
            ignore_whitespace: diff_ignore_whitespace,
        },
        settings: settings_state,
        store_dir: store_dir.clone(),
        session: session.clone(),
    };
    // 冲突解决（P3 批 A）：结果区与 base 显隐收进 MergeState，用例收进
    // MergeCommands；变更列表的冲突行要用它，所以先于 ChangesCommands 建。
    let merge_commands = features::merge::commands::MergeCommands {
        state: features::merge::state::MergeState {
            base_open: conflict_base_open,
            files_open: conflict_files_open,
            files_mode: conflict_files_mode,
        },
        session: session.clone(),
        language,
        notice,
    };
    let sync_commands = features::sync::commands::SyncCommands {
        state: features::sync::state::SyncState {
            dialog: sync_state,
            task: signal(None),
        },
        session: session.clone(),
        language,
        notice,
        askpass: askpass_client.clone(),
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
        sync: sync_commands.clone(),
    };
    // Agent（P3 批 B）：计划、权限、运行设置与审计收进 AgentState，用例收进
    // AgentCommands；设置页的 Agent 分区与提交面板的「请 Agent 起草」都用它。
    let agent_commands = features::agent::commands::AgentCommands {
        state: features::agent::state::AgentState {
            plan: agent_plan,
            running: signal(false),
            policies: agent_policies,
            settings: agent_settings,
            audit: agent_audit,
        },
        session: session.clone(),
        language,
        notice,
        store_dir: store_dir.clone(),
        commit_summary: commit_commands.state.summary,
        askpass: askpass_client.clone(),
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
    let changes_state = features::changes::state::ChangesState {
        changelist_open,
        rename_changelist,
        rename_changelist_value,
        collapsed_changelists,
        changelist_tree_expanded,
    };
    let changes_commands = features::changes::commands::ChangesCommands {
        state: changes_state,
        session: session.clone(),
        language,
        notice,
        mode,
        merge: merge_commands.clone(),
    };

    // 设置页（P3 批 B）：设置值、分区与快捷键编辑状态收进 SettingsState，用例收进
    // SettingsCommands。本机 Git 的版本、执行路径与凭据助手在启动时探测一次。
    let git_info = match git::info_with(&initial_git_executable) {
        Ok(info) => (
            info.version,
            info.exec_path,
            git::credential_helper_with(&initial_git_executable, std::path::Path::new("."))
                .unwrap_or_default(),
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
    let git_selected =
        features::settings::commands::load_scan_search_selection(&store_dir, &git_found);

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
        askpass: askpass_client.clone(),
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
        sidebar_open,
        right_panel_open,
    };

    // 引用管理（P3 批 A）：页签与 reflog 过滤收进 RefsState，用例收进 RefsCommands。
    // 「切到某个提交」的确认流程归恢复功能，这里只是持有它的用例。
    let initial_reflog_selection = if initial_reflog {
        let current_tabs = tabs.get_untracked();
        current_tabs.get(active.get_untracked()).and_then(|tab| {
            tab.reflog.first().map(|entry| {
                features::refs::state::ReflogEntryKey::new(tab.id, tab.path.clone(), 0, entry)
            })
        })
    } else {
        None
    };
    reflog_selection.set(initial_reflog_selection.clone());
    let refs_state = features::refs::state::RefsState {
        selected_tab: refs_selected_tab,
        reflog_query,
        reflog_selection,
        reflog_detail,
    };
    let refs_commands = features::refs::commands::RefsCommands {
        state: refs_state,
        session: session.clone(),
        language,
        notice,
        history: history_commands.clone(),
        recovery: recovery_commands.clone(),
        search: search_commands.clone(),
        sync: sync_commands.clone(),
        askpass: askpass_client.clone(),
        main_tabs: tabs,
        worktree_windows: signal(Vec::new()),
        open_worktree_window: None,
    };
    if let Some(selection) = initial_reflog_selection {
        refs_commands.select_reflog_entry(selection);
    }

    // 各功能的句柄收成一个结构：主窗口版式（`app::workspace`）只取这一个参数。
    let mut handles = AppHandles {
        askpass: askpass_ui.clone(),
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
    crate::app::windows::install_worktree_opener(&mut handles);

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
        .child(workspace_host(tabs, move || {
            crate::app::workspace::workspace(&handles_for_screen).into_element()
        }))
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
            tabs,
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

fn workspace_host(
    tabs: Signal<Vec<crate::app::session::Tab>>,
    content: impl Fn() -> AnyElement + 'static,
) -> impl IntoElement {
    // 只有首个仓库打开或最后一个仓库关闭才替换主区，Tab 内部更新保留组件身份。
    when(move || tabs.with(|tabs| !tabs.is_empty()), content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[component]
    fn WorkspaceHostFixture() -> impl IntoElement {
        let tabs = signal(vec![crate::app::session::Tab::default()]);
        let builds = std::rc::Rc::new(std::cell::Cell::new(0));
        view()
            .w_full()
            .h_full()
            .flex_col()
            .child(ui::button::button(
                || "Select commit".to_string(),
                ui::button::ButtonVariant::Secondary,
                ui::button::ButtonSize::Small,
                false,
                move || tabs.update(|tabs| tabs[0].selected = Some(1)),
            ))
            .child(workspace_host(tabs, move || {
                builds.set(builds.get() + 1);
                let mounted = builds.get();
                dynamic(move || {
                    scroll().w_full().h(px(100.0)).child(
                        view()
                            .h(px(1000.0))
                            .child(text(format!("Workspace mount {mounted}"))),
                    )
                })
                .into_element()
            }))
    }

    #[test]
    fn commit_selection_keeps_workspace_scroll_container() {
        let mut app = nsh_testing::Headless::start(nsh_engine::EngineConfig::new(
            nsh_core::element::Root::of::<WorkspaceHostFixture>(),
        ))
        .unwrap();
        app.first_render().unwrap();
        app.wait_for("Workspace mount 1").unwrap();
        let scroll_id = app
            .screen()
            .layout_nodes()
            .into_iter()
            .find(|node| node.scroll.is_some())
            .unwrap()
            .id;
        app.scroll("Workspace mount 1", px(250.0)).unwrap();
        app.press("Select commit").unwrap();
        app.wait(std::time::Duration::from_millis(50)).unwrap();
        assert!(
            app.screen()
                .layout_nodes()
                .iter()
                .any(|node| node.id == scroll_id),
            "选中提交不得重建 workspace 的滚动容器"
        );
    }
}
