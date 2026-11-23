//! 变更页的操作：装载/刷新变更、单文件 diff、按 hunk 暂存与清单管理。

use crate::app::session::{ChangeEntry, Tab};
use git::Repo;

/// 读取工作区与暂存区的变更，并附上逐文件行数统计。
pub(crate) fn load_changes(repo: &Repo) -> Vec<ChangeEntry> {
    let status = repo.status().unwrap_or_default();
    let staged_stats = repo.numstat_section(true).unwrap_or_default();
    let worktree_stats = repo.numstat_section(false).unwrap_or_default();
    let mut entries: Vec<ChangeEntry> = Vec::new();
    for entry in status {
        let staged = entry.is_staged();
        let untracked = entry.index == '?' && entry.worktree == '?';
        let stats = if staged {
            staged_stats.iter().find(|stat| stat.path == entry.path)
        } else {
            worktree_stats.iter().find(|stat| stat.path == entry.path)
        };
        entries.push(ChangeEntry {
            path: entry.path.clone(),
            index: entry.index,
            worktree: entry.worktree,
            staged,
            untracked,
            additions: stats.map(|stat| stat.additions).unwrap_or(0),
            deletions: stats.map(|stat| stat.deletions).unwrap_or(0),
            changelist: None,
        });
    }
    // 暂存的排在前面，组内按路径排序，与设计板的 Staged / Unstaged 两段一致。
    entries.sort_by(|left, right| {
        right
            .staged
            .cmp(&left.staged)
            .then_with(|| left.path.cmp(&right.path))
    });
    return entries;
}

/// 刷新变更列表（暂存、取消暂存、丢弃之后调用），保留选中项。
pub(crate) fn refresh_changes(tab: &mut Tab) {
    let repo = match tab.open_repo() {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    let previous = tab
        .selected_change
        .and_then(|index| tab.changes.get(index))
        .map(|entry| entry.path.clone());
    tab.changes = load_changes(&repo);
    tab.status = repo.status().unwrap_or_default();
    tab.operation = repo.operation();
    tab.conflicts = repo.conflicts().unwrap_or_default();
    tab.remote_urls = repo.remote_urls().unwrap_or_default();
    tab.selected_change = previous
        .and_then(|path| tab.changes.iter().position(|entry| entry.path == path))
        .or(if tab.changes.is_empty() {
            None
        } else {
            Some(0)
        });
    if let Some(index) = tab.selected_change {
        load_change_diff(tab, index);
    } else {
        tab.change_diff = domain::diff::Patch::default();
    }
    tab.error = None;
}

/// 选中一条变更并读取它的 diff。
pub(crate) fn load_change_diff(tab: &mut Tab, index: usize) {
    let Some(entry) = tab.changes.get(index).cloned() else {
        return;
    };
    let repo = match tab.open_repo() {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    tab.selected_change = Some(index);
    // 未跟踪文件没有 diff（git 不比较索引外的文件），留空由界面提示。
    tab.change_diff = if entry.untracked {
        domain::diff::Patch::default()
    } else {
        repo.file_diff(entry.path.as_str(), entry.staged)
            .unwrap_or_default()
    };
}

/// 把一个变更文件放进某个清单（`None` 放回默认清单）。
pub(crate) fn move_to_changelist(tab: &mut Tab, path: &str, list: Option<&str>) {
    let store_dir = store::default_dir(crate::brand::STORAGE_DIR);
    let repo = tab.path.clone();
    let mut lists = store::load_changelists(&store_dir, repo.as_str());
    // 先从所有清单里移除，再按需要加入目标清单。
    for existing in lists.iter_mut() {
        existing.paths.retain(|candidate| candidate != path);
    }
    if let Some(name) = list {
        match lists.iter_mut().find(|existing| existing.name == name) {
            Some(existing) => existing.paths.push(path.to_string()),
            None => lists.push(store::Changelist {
                name: name.to_string(),
                paths: vec![path.to_string()],
            }),
        }
    }
    lists.retain(|existing| !existing.paths.is_empty());
    store::save_changelists(&store_dir, repo.as_str(), &lists);
    tab.changelists = lists;
    // 变更行上的清单归属跟着更新。
    for entry in tab.changes.iter_mut() {
        if entry.path == path {
            entry.changelist = list.map(|name| name.to_string());
        }
    }
}

/// 新建清单（空清单也保留，便于先建后移）。
pub(crate) fn create_changelist(tab: &mut Tab, name: &str) {
    let store_dir = store::default_dir(crate::brand::STORAGE_DIR);
    let repo = tab.path.clone();
    let mut lists = store::load_changelists(&store_dir, repo.as_str());
    if lists.iter().any(|existing| existing.name == name) {
        return;
    }
    lists.push(store::Changelist {
        name: name.to_string(),
        paths: Vec::new(),
    });
    store::save_changelists(&store_dir, repo.as_str(), &lists);
    tab.changelists = lists;
}

/// 切换当前清单。
pub(crate) fn set_active_changelist(tab: &mut Tab, name: Option<String>) {
    tab.active_changelist = name;
}

/// 按 hunk 或单行暂存/取消暂存当前选中的变更。
///
/// `line_index` 为 `None` 时整段 hunk；`reverse` 为真表示从暂存区取回。
/// 补丁来源是 `tab.change_diff`（当前文件相对暂存区 diff 的解析结果，原文在 `raw`）。
pub(crate) fn apply_piece(
    tab: &mut Tab,
    hunk_index: usize,
    line_index: Option<usize>,
    reverse: bool,
) -> Result<(), String> {
    let sections = domain::stage::Sections::parse(tab.change_diff.raw.as_str());
    let patch = match line_index {
        Some(line) => sections.line_patch(0, hunk_index, line),
        None => sections.hunk_patch(0, hunk_index),
    }
    .ok_or_else(|| "这一段没有可暂存的内容".to_string())?;
    let repo = tab.open_repo().map_err(|error| error.detail())?;
    repo.apply_cached(patch.as_str(), reverse)
        .map_err(|error| error.detail())?;
    refresh_changes(tab);
    // 重新装载当前选中文件的 diff，让刚改的片段立刻反映出来。
    let selected = tab.selected_change;
    let filter = tab.filter.clone();
    let refreshed = crate::app::session::load_tab_with_executable(
        tab.path.as_str(),
        &filter,
        &tab.git_executable,
    );
    tab.commits = refreshed.commits;
    tab.changes = refreshed.changes;
    tab.status = refreshed.status;
    if let Some(index) = selected {
        if index < tab.changes.len() {
            load_change_diff(tab, index);
        } else {
            tab.selected_change = None;
            tab.change_diff = domain::diff::Patch::default();
        }
    }
    return Ok(());
}

/// 暂存或取消暂存一条变更。
pub(crate) fn toggle_stage(tab: &mut Tab, index: usize) {
    let Some(entry) = tab.changes.get(index).cloned() else {
        return;
    };
    let repo = match tab.open_repo() {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    let paths = vec![entry.path.clone()];
    let result = if entry.staged {
        repo.unstage(&paths)
    } else {
        repo.stage(&paths)
    };
    match result {
        Ok(()) => refresh_changes(tab),
        Err(error) => tab.error = Some(error.detail()),
    }
}

/// 丢弃一条变更的工作区改动（不可撤销，调用方负责确认）。
pub(crate) fn discard_change(tab: &mut Tab, index: usize) {
    let Some(entry) = tab.changes.get(index).cloned() else {
        return;
    };
    let repo = match tab.open_repo() {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    match repo.discard(&[entry.path.clone()]) {
        Ok(()) => refresh_changes(tab),
        Err(error) => tab.error = Some(error.detail()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::load_tab;
    use crate::test_support::TempRepo;
    use git::LogFilter;

    #[test]
    fn stage_and_unstage_a_line_after_context() {
        let repo = TempRepo::new("line-context");
        let git = Repo::open(&repo.path.display().to_string()).unwrap();
        std::fs::write(repo.path.join("main.rs"), "fn main() {\n}\n").unwrap();
        git.stage(&["main.rs".to_string()]).unwrap();
        git.commit("base", "").unwrap();
        std::fs::write(
            repo.path.join("main.rs"),
            "fn main() {\n    let count = 42;\n}\n",
        )
        .unwrap();
        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        let index = tab
            .changes
            .iter()
            .position(|entry| entry.path == "main.rs" && !entry.staged)
            .unwrap();
        load_change_diff(&mut tab, index);
        apply_piece(&mut tab, 0, Some(1), false).unwrap();
        let staged = git.file_diff("main.rs", true).unwrap();
        assert!(staged.raw.contains("+    let count = 42;"));
        let index = tab
            .changes
            .iter()
            .position(|entry| entry.path == "main.rs" && entry.staged)
            .unwrap();
        load_change_diff(&mut tab, index);
        apply_piece(&mut tab, 0, Some(1), true).unwrap();
        assert!(git.file_diff("main.rs", true).unwrap().raw.is_empty());
        assert!(
            git.file_diff("main.rs", false)
                .unwrap()
                .raw
                .contains("+    let count = 42;")
        );
    }

    #[test]
    fn changes_list_splits_staged_and_unstaged() {
        let repo = TempRepo::new("changes");
        let path = repo.path.display().to_string();
        std::fs::write(repo.path.join("a.txt"), "changed\n").expect("write");
        std::fs::write(repo.path.join("new.txt"), "new\n").expect("write");
        let mut tab = load_tab(&path, &LogFilter::default());
        assert_eq!(tab.changes.len(), 2);

        // 暂存 a.txt 后它应出现在 Staged 段。
        let index = tab
            .changes
            .iter()
            .position(|entry| entry.path == "a.txt")
            .expect("a.txt");
        toggle_stage(&mut tab, index);
        let a = tab
            .changes
            .iter()
            .find(|entry| entry.path == "a.txt")
            .expect("a.txt");
        assert!(a.staged);
        let new = tab
            .changes
            .iter()
            .find(|entry| entry.path == "new.txt")
            .expect("new.txt");
        assert!(!new.staged);
        assert!(new.untracked, "未跟踪文件应标记为 untracked");

        // 取消暂存后回到未暂存段。
        let index = tab
            .changes
            .iter()
            .position(|entry| entry.path == "a.txt")
            .expect("a.txt");
        toggle_stage(&mut tab, index);
        assert!(
            !tab.changes
                .iter()
                .find(|entry| entry.path == "a.txt")
                .expect("a.txt")
                .staged
        );
    }

    #[test]
    fn apply_piece_stages_one_hunk_of_the_selected_file() {
        let repo = TempRepo::new("piece");
        let lines: String = (1..=30).map(|index| format!("line{index}\n")).collect();
        std::fs::write(repo.path.join("f.txt"), lines).expect("write");
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo.path)
            .args(["add", "-A"])
            .status()
            .expect("git");
        assert!(status.success());
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo.path)
            .args(["commit", "-q", "-m", "base"])
            .status()
            .expect("git");
        assert!(status.success());

        // 两处远端改动，形成两个 hunk。
        let mut changed: Vec<String> = (1..=30).map(|index| format!("line{index}")).collect();
        changed[1] = "line2-changed".to_string();
        changed[27] = "line28-changed".to_string();
        std::fs::write(repo.path.join("f.txt"), format!("{}\n", changed.join("\n")))
            .expect("write");

        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        let index = tab
            .changes
            .iter()
            .position(|entry| entry.path == "f.txt")
            .expect("f.txt");
        load_change_diff(&mut tab, index);
        let sections = domain::stage::Sections::parse(tab.change_diff.raw.as_str());
        assert_eq!(sections.files[0].hunks.len(), 2, "应有两个 hunk");

        // 只暂存第二个 hunk。
        apply_piece(&mut tab, 1, None, false).expect("stage hunk");
        assert!(
            tab.changes
                .iter()
                .any(|entry| entry.path == "f.txt" && entry.staged),
            "文件应有一部分进了暂存区"
        );
        // 工作区仍保留第一个 hunk 的改动。
        let diff = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo.path)
            .args(["diff", "--", "f.txt"])
            .output()
            .expect("git");
        let diff = String::from_utf8_lossy(&diff.stdout).to_string();
        assert!(diff.contains("line2-changed"), "第一个 hunk 还在工作区");
    }

    #[test]
    fn discard_change_restores_the_file() {
        let repo = TempRepo::new("discard");
        let path = repo.path.display().to_string();
        std::fs::write(repo.path.join("a.txt"), "scribble\n").expect("write");
        let mut tab = load_tab(&path, &LogFilter::default());
        let index = tab
            .changes
            .iter()
            .position(|entry| entry.path == "a.txt")
            .expect("a.txt");
        discard_change(&mut tab, index);
        assert_eq!(
            std::fs::read_to_string(repo.path.join("a.txt")).expect("read"),
            "one\n"
        );
        assert!(tab.changes.is_empty());
    }

    #[test]
    fn changelists_group_changes_and_keep_storage() {
        // 用独立的 APP_HOME 隔离存储，避免污染真实的应用支持目录。
        let home = std::env::temp_dir().join(format!(
            "nsh-app-changelists-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&home).expect("home");
        // SAFETY：测试进程内串行使用该变量，只影响本测试的存储目录。
        unsafe { std::env::set_var("APP_HOME", &home) };

        let repo = TempRepo::new("changelists");
        std::fs::write(repo.path.join("a.txt"), "changed\n").expect("write");
        std::fs::write(repo.path.join("b.txt"), "new\n").expect("write");
        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        assert_eq!(tab.changes.len(), 2);

        create_changelist(&mut tab, "Commit composer");
        assert_eq!(tab.changelists.len(), 1);
        assert!(tab.changelists[0].paths.is_empty());

        move_to_changelist(&mut tab, "a.txt", Some("Commit composer"));
        let list = tab
            .changelists
            .iter()
            .find(|list| list.name == "Commit composer")
            .expect("list");
        assert_eq!(list.paths, vec!["a.txt".to_string()]);
        let entry = tab
            .changes
            .iter()
            .find(|entry| entry.path == "a.txt")
            .expect("a.txt");
        assert_eq!(entry.changelist.as_deref(), Some("Commit composer"));

        // 重新读取：清单从磁盘恢复。
        let reloaded = load_tab(&repo.path.display().to_string(), &LogFilter::default());
        assert!(
            reloaded
                .changelists
                .iter()
                .any(|list| list.name == "Commit composer"
                    && list.paths == vec!["a.txt".to_string()])
        );

        // 移回默认清单：清单条目被清空。
        move_to_changelist(&mut tab, "a.txt", None);
        assert!(tab.changelists.iter().all(|list| list.paths.is_empty()));

        set_active_changelist(&mut tab, Some("Commit composer".to_string()));
        assert_eq!(tab.active_changelist.as_deref(), Some("Commit composer"));

        unsafe { std::env::remove_var("APP_HOME") };
        let _ = std::fs::remove_dir_all(&home);
    }
}
