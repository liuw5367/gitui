//! 本模块负责把 Git 的文本输出解析成 `domain` 的类型：日志、状态、上游跟踪。

use domain::{Commit, GitError, StatusEntry};

/// 把 Git 的输出压成一行可展示的摘要：去掉空行，保留最后一条有效信息。
pub(crate) fn summarise(output: &str) -> String {
    let mut lines: Vec<&str> = output
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();
    if lines.is_empty() {
        return String::new();
    }
    // fetch/push 的进度行很多，最后几行才是结果（`main -> main`、`up to date`）。
    if lines.len() > 3 {
        lines = lines.split_off(lines.len() - 3);
    }
    return lines.join("\n");
}

/// 解析 `%(upstream:track)`：`[ahead 2]`、`[behind 1]`、`[ahead 2, behind 1]`。
pub(crate) fn parse_track(track: &str) -> (u32, u32) {
    let mut ahead = 0;
    let mut behind = 0;
    for part in track.trim_matches(|c| c == '[' || c == ']').split(',') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix("ahead ") {
            ahead = rest.trim().parse().unwrap_or(0);
        } else if let Some(rest) = part.strip_prefix("behind ") {
            behind = rest.trim().parse().unwrap_or(0);
        }
    }
    return (ahead, behind);
}

pub(crate) fn parse_log(text: &str) -> Result<Vec<Commit>, GitError> {
    let mut commits = Vec::new();
    for record in text.split('\u{1e}') {
        let record = record.trim_start_matches('\n');
        if record.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = record.split('\u{1f}').collect();
        if fields.len() < 9 {
            return Err(GitError::Parse {
                context: "git log".to_string(),
                detail: format!("字段数量为 {}：{}", fields.len(), record),
            });
        }
        let refs = fields[7]
            .split(',')
            .map(|reference| reference.trim().to_string())
            .filter(|reference| !reference.is_empty())
            .collect::<Vec<_>>();
        let parents = fields[8..]
            .join("\u{1f}")
            .split_whitespace()
            .map(|parent| parent.to_string())
            .collect::<Vec<_>>();
        commits.push(Commit {
            sha: fields[0].to_string(),
            short: fields[1].to_string(),
            author: fields[2].to_string(),
            author_email: fields[3].to_string(),
            date: fields[4].to_string(),
            summary: fields[5].to_string(),
            message: fields[6].trim_end_matches('\n').to_string(),
            refs,
            branches: Vec::new(),
            parents,
        });
    }
    return Ok(commits);
}

pub(crate) fn parse_status(output: &[u8]) -> Result<Vec<StatusEntry>, GitError> {
    let mut entries = Vec::new();
    let text = String::from_utf8_lossy(output);
    for record in text.split('\0') {
        if record.is_empty() {
            continue;
        }
        let mut chars = record.chars();
        let index = chars.next().unwrap_or(' ');
        let worktree = chars.next().unwrap_or(' ');
        // 第 3 个字符是分隔空格。
        let path: String = chars.skip(1).collect();
        if path.is_empty() {
            continue;
        }
        entries.push(StatusEntry {
            path,
            index,
            worktree,
        });
    }
    return Ok(entries);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_strings_are_parsed() {
        assert_eq!(parse_track(""), (0, 0));
        assert_eq!(parse_track("[ahead 2]"), (2, 0));
        assert_eq!(parse_track("[behind 3]"), (0, 3));
        assert_eq!(parse_track("[ahead 2, behind 1]"), (2, 1));
    }
}
