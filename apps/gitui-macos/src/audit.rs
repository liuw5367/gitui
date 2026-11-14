//! Append-only local audit log for destructive and agent actions.

use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join("Library/Application Support/gitui/agent-audit.log"),
    )
}

pub(crate) fn append(summary: &str, allowed: bool) {
    let Some(path) = path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        if create_dir_all(parent).is_err() {
            return;
        }
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let summary = summary.replace(['\n', '\t'], " ");
    let summary: String = summary.chars().take(500).collect();
    let line = format!(
        "{timestamp}\t{}\t{summary}\n",
        if allowed { "allow" } else { "deny" }
    );
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    if let Ok(mut file) = options.open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}
