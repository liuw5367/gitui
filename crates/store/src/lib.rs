//! 本地持久化：最近仓库与会话。
//!
//! 采用逐行文本格式（每行一个路径），不需要额外依赖：
//!
//! - `recent.txt`：最近打开的仓库，最新的在第一行，每行 `<打开时间>\t<路径>`；
//! - `session.txt`：第一行是当前 tab 下标，其后每行是一个打开中的仓库路径。
//!
//! 所有函数都接受目录参数，便于测试用临时目录；应用层用 [`default_dir`] 取得
//! `~/Library/Application Support/gitui`。写入采用“先写临时文件再改名”，避免
//! 中途失败留下半个文件。路径里若含换行会被跳过，避免破坏行格式。

use std::path::{Path, PathBuf};

/// 最近仓库的数量上限。
const MAX_RECENT: usize = 12;

/// 一条最近仓库记录：路径与最后一次打开的时间（UTC 秒）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecentRepo {
    pub path: String,
    pub opened_at: i64,
}

/// 会话：当前选中的 tab 与打开的仓库路径。空 `paths` 表示显示欢迎页。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Session {
    /// 当前 tab 的下标；越界时按最后一个处理。
    pub active: usize,
    /// 打开的仓库路径，按 tab 顺序。
    pub paths: Vec<String>,
}

/// `~/Library/Application Support/<directory>`；`APP_HOME` 可用于测试或便携使用。
///
/// 目录名由调用方传入，避免把产品名写死在存储层。
pub fn default_dir(directory: &str) -> PathBuf {
    if let Some(home) = std::env::var_os("APP_HOME") {
        return PathBuf::from(home);
    }
    let base = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    return base.join("Library/Application Support").join(directory);
}

fn language_file(dir: &Path) -> PathBuf {
    return dir.join("language.txt");
}

/// 读取界面语言；未设置时返回 `None`（由应用跟随系统语言）。
pub fn load_language(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(language_file(dir)).ok()?;
    let language = text.trim();
    if language.is_empty() {
        return None;
    }
    return Some(language.to_string());
}

/// 保存界面语言。
pub fn save_language(dir: &Path, language: &str) {
    let _ = write_lines(&language_file(dir), &[language.to_string()]);
}

/// 一个变更清单：名称与其中的文件路径。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Changelist {
    pub name: String,
    pub paths: Vec<String>,
}

fn changelists_file(dir: &Path) -> PathBuf {
    return dir.join("changelists.txt");
}

/// 读取某个仓库的变更清单；文件缺失时返回空列表。
pub fn load_changelists(dir: &Path, repo: &str) -> Vec<Changelist> {
    let text = match std::fs::read_to_string(changelists_file(dir)) {
        Ok(text) => text,
        Err(_) => return Vec::new(),
    };
    let mut lists: Vec<Changelist> = Vec::new();
    for line in text.lines() {
        let mut fields = line.split('\t');
        let (Some(entry_repo), Some(name), Some(path)) =
            (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        if entry_repo != repo || name.is_empty() || path.is_empty() {
            continue;
        }
        match lists.iter_mut().find(|list| list.name == name) {
            Some(list) => list.paths.push(path.to_string()),
            None => lists.push(Changelist {
                name: name.to_string(),
                paths: vec![path.to_string()],
            }),
        }
    }
    return lists;
}

/// 保存某个仓库的变更清单，保留其他仓库的条目。
pub fn save_changelists(dir: &Path, repo: &str, lists: &[Changelist]) {
    let mut lines: Vec<String> = Vec::new();
    // 先写其他仓库的条目。
    if let Ok(text) = std::fs::read_to_string(changelists_file(dir)) {
        for line in text.lines() {
            if let Some((entry_repo, _)) = line.split_once('\t') {
                if entry_repo != repo && !line.trim().is_empty() {
                    lines.push(line.to_string());
                }
            }
        }
    }
    for list in lists {
        for path in &list.paths {
            if list.name.is_empty() || path.contains('\t') {
                continue;
            }
            lines.push(format!("{}\t{}\t{}", repo, list.name, path));
        }
    }
    let _ = write_lines(&changelists_file(dir), &lines);
}

/// 应用设置（除语言与外观外的偏好）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// 变更列表按目录分组（关闭时为平铺路径）。
    pub file_tree: bool,
    /// 启动时恢复上次打开的仓库；关闭则显示欢迎页。
    pub restore_session: bool,
    /// 更新检查方式：`auto` / `manual` / `never`。
    pub update_mode: String,
}

impl Default for Settings {
    fn default() -> Self {
        return Settings {
            file_tree: true,
            restore_session: true,
            update_mode: "manual".to_string(),
        };
    }
}

fn settings_file(dir: &Path) -> PathBuf {
    return dir.join("settings.txt");
}

/// 读取设置；文件缺失时返回默认值。
pub fn load_settings(dir: &Path) -> Settings {
    let mut settings = Settings::default();
    let Ok(text) = std::fs::read_to_string(settings_file(dir)) else {
        return settings;
    };
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "file_tree" => settings.file_tree = value.trim() != "false",
            "restore_session" => settings.restore_session = value.trim() != "false",
            "update_mode" => settings.update_mode = value.trim().to_string(),
            _ => {}
        }
    }
    return settings;
}

/// 保存设置。
pub fn save_settings(dir: &Path, settings: &Settings) {
    let lines = vec![
        format!("file_tree={}", settings.file_tree),
        format!("restore_session={}", settings.restore_session),
        format!("update_mode={}", settings.update_mode),
    ];
    let _ = write_lines(&settings_file(dir), &lines);
}

/// Agent 的设置：每个动作的权限（`动作=权限` 行）。
pub fn load_agent_policies(dir: &Path) -> Vec<(String, String)> {
    let Ok(text) = std::fs::read_to_string(dir.join("agent-policies.txt")) else {
        return Vec::new();
    };
    return text
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(action, permission)| {
            (action.trim().to_string(), permission.trim().to_string())
        })
        .filter(|(action, permission)| !action.is_empty() && !permission.is_empty())
        .collect();
}

/// 保存 Agent 的动作权限。
pub fn save_agent_policies(dir: &Path, policies: &[(String, String)]) {
    let lines: Vec<String> = policies
        .iter()
        .map(|(action, permission)| format!("{}={}", action, permission))
        .collect();
    let _ = write_lines(&dir.join("agent-policies.txt"), &lines);
}

/// Agent 的审计日志：每行 `时间\t动作\t结果\t详情`。
pub fn load_agent_audit(dir: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(dir.join("agent-audit.txt")) else {
        return Vec::new();
    };
    return text.lines().map(|line| line.to_string()).collect();
}

/// 保存 Agent 的审计日志（最新的在前，调用方负责截断）。
pub fn save_agent_audit(dir: &Path, entries: &[String]) {
    let _ = write_lines(&dir.join("agent-audit.txt"), entries);
}

fn appearance_file(dir: &Path) -> PathBuf {
    return dir.join("appearance.txt");
}

/// 读取外观偏好；未设置时返回 `None`（跟随系统）。
pub fn load_appearance(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(appearance_file(dir)).ok()?;
    let value = text.trim();
    if value.is_empty() {
        return None;
    }
    return Some(value.to_string());
}

/// 保存外观偏好。
pub fn save_appearance(dir: &Path, appearance: &str) {
    let _ = write_lines(&appearance_file(dir), &[appearance.to_string()]);
}

fn recent_file(dir: &Path) -> PathBuf {
    return dir.join("recent.txt");
}

fn session_file(dir: &Path) -> PathBuf {
    return dir.join("session.txt");
}

/// 读取最近仓库；文件缺失或为空时返回空列表。
pub fn load_recent(dir: &Path) -> Vec<RecentRepo> {
    let text = match std::fs::read_to_string(recent_file(dir)) {
        Ok(text) => text,
        Err(_) => return Vec::new(),
    };
    let mut repos = Vec::new();
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        // 旧格式只有路径；新格式是 `<时间>\t<路径>`。
        let (opened_at, path) = match line.split_once('\t') {
            Some((timestamp, path)) => (timestamp.parse().unwrap_or(0), path),
            None => (0, line),
        };
        if path.is_empty() {
            continue;
        }
        repos.push(RecentRepo {
            path: path.to_string(),
            opened_at,
        });
    }
    repos.truncate(MAX_RECENT);
    return repos;
}

/// 当前 Unix 时间（秒）。用于记录打开时间，不依赖额外 crate。
pub fn now_unix() -> i64 {
    return std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
}

/// 保存最近仓库；把 `path` 放到最前（时间记为现在），去重并截断。
pub fn remember_recent(dir: &Path, repos: &[RecentRepo], path: &str) {
    let mut next = Vec::with_capacity(repos.len() + 1);
    next.push(RecentRepo {
        path: path.to_string(),
        opened_at: now_unix(),
    });
    for existing in repos {
        if existing.path != path && !existing.path.is_empty() {
            next.push(existing.clone());
        }
    }
    next.truncate(MAX_RECENT);
    let lines: Vec<String> = next
        .iter()
        .map(|repo| format!("{}\t{}", repo.opened_at, repo.path))
        .collect();
    let _ = write_lines(&recent_file(dir), &lines);
}

/// 从最近仓库里移除一条，返回剩余列表。
pub fn forget_recent(dir: &Path, repos: &[RecentRepo], path: &str) -> Vec<RecentRepo> {
    let next: Vec<RecentRepo> = repos
        .iter()
        .filter(|repo| repo.path != path)
        .cloned()
        .collect();
    let lines: Vec<String> = next
        .iter()
        .map(|repo| format!("{}\t{}", repo.opened_at, repo.path))
        .collect();
    let _ = write_lines(&recent_file(dir), &lines);
    return next;
}

/// 读取会话；文件缺失或格式不符时返回空会话。
pub fn load_session(dir: &Path) -> Session {
    let text = match std::fs::read_to_string(session_file(dir)) {
        Ok(text) => text,
        Err(_) => return Session::default(),
    };
    let mut lines = text.lines();
    let active: usize = lines.next().and_then(|line| line.trim().parse().ok()).unwrap_or(0);
    let paths: Vec<String> = lines
        .map(|line| line.trim_end().to_string())
        .filter(|line| !line.is_empty())
        .collect();
    if paths.is_empty() {
        return Session::default();
    }
    return Session {
        active: active.min(paths.len() - 1),
        paths,
    };
}

/// 保存会话。
pub fn save_session(dir: &Path, session: &Session) {
    let mut lines = Vec::with_capacity(session.paths.len() + 1);
    lines.push(session.active.to_string());
    lines.extend(session.paths.iter().cloned());
    let _ = write_lines(&session_file(dir), &lines);
}

fn write_lines(file: &Path, lines: &[String]) -> std::io::Result<()> {
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut text = String::new();
    for line in lines {
        // 含换行的路径无法用逐行格式表达，直接跳过。
        if line.contains('\n') || line.contains('\r') {
            continue;
        }
        text.push_str(line);
        text.push('\n');
    }
    let temporary = file.with_extension("txt.tmp");
    std::fs::write(&temporary, text)?;
    std::fs::rename(&temporary, file)?;
    return Ok(());
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(name: &str) -> TempDir {
            let unique = format!(
                "store-test-{}-{}-{}",
                name,
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_nanos())
                    .unwrap_or(0)
            );
            let path = std::env::temp_dir().join(unique);
            std::fs::create_dir_all(&path).expect("temp dir");
            return TempDir { path };
        }

        fn path(&self) -> &Path {
            return &self.path;
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn recent_puts_the_latest_first_and_deduplicates() {
        let dir = TempDir::new("recent");
        remember_recent(dir.path(), &[], "/a");
        remember_recent(dir.path(), &load_recent(dir.path()), "/b");
        remember_recent(dir.path(), &load_recent(dir.path()), "/a");
        let paths: Vec<String> = load_recent(dir.path())
            .into_iter()
            .map(|repo| repo.path)
            .collect();
        assert_eq!(paths, vec!["/a".to_string(), "/b".to_string()]);
        assert!(load_recent(dir.path())[0].opened_at > 0);
    }

    #[test]
    fn recent_caps_the_list() {
        let dir = TempDir::new("cap");
        for index in 0..20 {
            let path = format!("/repo-{index}");
            let current = load_recent(dir.path());
            remember_recent(dir.path(), &current, &path);
        }
        let paths = load_recent(dir.path());
        assert_eq!(paths.len(), MAX_RECENT);
        assert_eq!(paths[0].path, "/repo-19");
    }

    #[test]
    fn agent_policies_and_audit_round_trip() {
        let dir = TempDir::new("agent");
        assert!(load_agent_policies(dir.path()).is_empty());
        save_agent_policies(
            dir.path(),
            &[("push".to_string(), "deny".to_string())],
        );
        assert_eq!(
            load_agent_policies(dir.path()),
            vec![("push".to_string(), "deny".to_string())]
        );

        save_agent_audit(dir.path(), &["1\tfetch\tok\t2 refs".to_string()]);
        assert_eq!(load_agent_audit(dir.path()).len(), 1);
    }

    #[test]
    fn settings_round_trip_with_defaults() {
        let dir = TempDir::new("settings");
        assert_eq!(load_settings(dir.path()), Settings::default());
        save_settings(
            dir.path(),
            &Settings {
                file_tree: false,
                restore_session: false,
                update_mode: "never".to_string(),
            },
        );
        let loaded = load_settings(dir.path());
        assert!(!loaded.file_tree);
        assert!(!loaded.restore_session);
        assert_eq!(loaded.update_mode, "never", "更新方式应持久化");
        assert_eq!(Settings::default().update_mode, "manual");
    }

    #[test]
    fn appearance_round_trips() {
        let dir = TempDir::new("appearance");
        assert_eq!(load_appearance(dir.path()), None);
        save_appearance(dir.path(), "dark");
        assert_eq!(load_appearance(dir.path()), Some("dark".to_string()));
    }

    #[test]
    fn language_round_trips() {
        let dir = TempDir::new("language");
        assert_eq!(load_language(dir.path()), None);
        save_language(dir.path(), "zh-Hans");
        assert_eq!(load_language(dir.path()), Some("zh-Hans".to_string()));
    }

    #[test]
    fn changelists_round_trip_per_repository() {
        let dir = TempDir::new("changelists");
        let lists = vec![
            Changelist {
                name: "Commit composer".to_string(),
                paths: vec!["src/a.ts".to_string(), "src/b.ts".to_string()],
            },
            Changelist {
                name: "Agent cleanup".to_string(),
                paths: vec!["src/agent.ts".to_string()],
            },
        ];
        save_changelists(dir.path(), "/repo-one", &lists);
        assert_eq!(load_changelists(dir.path(), "/repo-one"), lists);

        // 另一个仓库有自己的清单，两者互不影响。
        save_changelists(
            dir.path(),
            "/repo-two",
            &[Changelist {
                name: "Other".to_string(),
                paths: vec!["README.md".to_string()],
            }],
        );
        assert_eq!(load_changelists(dir.path(), "/repo-one"), lists);
        assert_eq!(load_changelists(dir.path(), "/repo-two").len(), 1);
    }

    #[test]
    fn missing_files_fall_back_to_empty_state() {
        let dir = TempDir::new("missing");
        assert!(load_recent(dir.path()).is_empty());
        assert_eq!(load_session(dir.path()), Session::default());
    }

    #[test]
    fn session_round_trips_and_clamps_active() {
        let dir = TempDir::new("session");
        save_session(
            dir.path(),
            &Session {
                active: 9,
                paths: vec!["/a".to_string(), "/b".to_string()],
            },
        );
        let loaded = load_session(dir.path());
        assert_eq!(loaded.active, 1);
        assert_eq!(loaded.paths, vec!["/a".to_string(), "/b".to_string()]);
    }

    #[test]
    fn forgetting_one_recent_keeps_the_others() {
        let dir = TempDir::new("forget");
        remember_recent(dir.path(), &[], "/a");
        remember_recent(dir.path(), &load_recent(dir.path()), "/b");
        let next = forget_recent(dir.path(), &load_recent(dir.path()), "/a");
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].path, "/b");
        assert_eq!(load_recent(dir.path())[0].path, "/b");
    }
}
