//! Model and persistence for the NSH app.

use gitui_git::{Branch, Commit, ConflictStages, Operation, Remote, Repo, Stash, StatusEntry, Tag};
use nsh_core::prelude::*;
use std::path::PathBuf;

#[derive(Clone, PartialEq)]
pub(crate) struct Loaded {
    pub(crate) path: String,
    pub(crate) branch: String,
    pub(crate) operation: Option<Operation>,
    pub(crate) conflicts: Vec<String>,
    pub(crate) branches: Vec<Branch>,
    pub(crate) tags: Vec<Tag>,
    pub(crate) stashes: Vec<Stash>,
    pub(crate) remotes: Vec<Remote>,
    pub(crate) conflict_path: String,
    pub(crate) conflict_stages: Option<ConflictStages>,
    pub(crate) ahead: usize,
    pub(crate) behind: usize,
    pub(crate) staged: usize,
    pub(crate) unstaged: usize,
    pub(crate) status_lines: Vec<SharedString>,
    pub(crate) status_entries: Vec<StatusEntry>,
    pub(crate) commits: Vec<Commit>,
    pub(crate) selected: SharedString,
    pub(crate) diff: SharedString,
    pub(crate) error: SharedString,
}

pub(crate) fn load(path: &str) -> Loaded {
    match Repo::open(path) {
        Ok(repo) => {
            let status = repo.status().unwrap_or_default();
            let commits = repo.log(15).unwrap_or_default();
            let diff = commits
                .first()
                .and_then(|commit| repo.diff(&commit.sha).ok())
                .unwrap_or_default();
            let branch = if status.branch.is_empty() {
                "HEAD".to_string()
            } else {
                status.branch
            };
            let selected = commits
                .first()
                .map(|commit| commit.sha.clone())
                .unwrap_or_default();
            Loaded {
                path: repo.root().display().to_string(),
                branch,
                operation: repo.operation(),
                conflicts: repo.conflicts().unwrap_or_default(),
                branches: repo.branches().unwrap_or_default(),
                tags: repo.tags().unwrap_or_default(),
                stashes: repo.stashes().unwrap_or_default(),
                remotes: repo.remotes().unwrap_or_default(),
                conflict_path: repo
                    .conflicts()
                    .unwrap_or_default()
                    .first()
                    .cloned()
                    .unwrap_or_default(),
                conflict_stages: repo
                    .conflicts()
                    .unwrap_or_default()
                    .first()
                    .and_then(|path| repo.conflict_stages(path).ok()),
                ahead: status.ahead,
                behind: status.behind,
                staged: status.staged,
                unstaged: status.unstaged,
                status_lines: status.lines.into_iter().map(SharedString::from).collect(),
                status_entries: status.entries,
                commits,
                selected: SharedString::from(selected),
                diff: SharedString::from(diff),
                error: SharedString::from(""),
            }
        }
        Err(error) => Loaded {
            path: path.to_string(),
            branch: "—".to_string(),
            operation: None,
            conflicts: Vec::new(),
            branches: Vec::new(),
            tags: Vec::new(),
            stashes: Vec::new(),
            remotes: Vec::new(),
            conflict_path: String::new(),
            conflict_stages: None,
            ahead: 0,
            behind: 0,
            selected: SharedString::from(""),
            staged: 0,
            unstaged: 0,
            status_lines: Vec::new(),
            status_entries: Vec::new(),
            commits: Vec::new(),
            diff: SharedString::from(format!("Could not open repository:\n{error}")),
            error: SharedString::from(error.to_string()),
        },
    }
}

pub(crate) fn short_name(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

pub(crate) fn recent_file() -> Option<PathBuf> {
    let home = std::env::var_os("GITUI_HOME").or_else(|| std::env::var_os("HOME"))?;
    Some(PathBuf::from(home).join(".gitui").join("recent"))
}

pub(crate) fn load_recent() -> Vec<String> {
    let Some(file) = recent_file() else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(file) else {
        return Vec::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(5)
        .map(str::to_string)
        .collect()
}

pub(crate) fn save_recent(paths: &[SharedString]) {
    let Some(file) = recent_file() else {
        return;
    };
    if let Some(parent) = file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let text = paths
        .iter()
        .map(|path| path.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::write(file, text);
}

pub(crate) fn session_file() -> Option<PathBuf> {
    let home = std::env::var_os("GITUI_HOME").or_else(|| std::env::var_os("HOME"))?;
    Some(PathBuf::from(home).join(".gitui").join("session"))
}

pub(crate) fn load_session() -> (usize, Vec<String>) {
    let Some(file) = session_file() else {
        return (0, Vec::new());
    };
    let Ok(text) = std::fs::read_to_string(file) else {
        return (0, Vec::new());
    };
    let mut lines = text.lines();
    let active = lines
        .next()
        .and_then(|line| line.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let paths = lines
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(10)
        .map(str::to_string)
        .collect();
    (active, paths)
}

pub(crate) fn save_session(active: usize, tabs: &[Loaded]) {
    let Some(file) = session_file() else {
        return;
    };
    if let Some(parent) = file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut lines = vec![active.to_string()];
    lines.extend(tabs.iter().map(|tab| tab.path.clone()));
    let _ = std::fs::write(file, lines.join("\n"));
}
