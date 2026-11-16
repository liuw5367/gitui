//! 把一份补丁切成可单独应用的片段（设计板 10 的 hunk / 行级暂存）。
//!
//! `git apply --cached` 只认完整的 unified diff，所以「只暂存这个 hunk」必须重新
//! 拼一份只含该 hunk 的补丁，文件头要带、hunk 头要按实际行数重算。单行更极端：
//! 要合成一个只有一行的 hunk，且旧侧位置必须落在正确的那一行上。
//!
//! 这里只做字符串处理，不碰仓库；应用与校验在 `git` crate。

/// 一个文件的补丁片段。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileSection {
    /// `diff --git` 到 `+++` 之间的行（含），应用时需要原样带上。
    pub header: Vec<String>,
    pub path: String,
    pub hunks: Vec<Hunk>,
}

/// 一个 hunk。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hunk {
    /// 原始的 `@@ -a,b +c,d @@` 行。
    pub header: String,
    /// hunk 正文（带 ` +` / `-` / 空格 前缀）。
    pub lines: Vec<String>,
    /// 旧侧起始行号。
    pub old_start: u32,
    /// 新侧起始行号。
    pub new_start: u32,
}

/// 一小段改动落在哪一侧。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    Add,
    Remove,
    Context,
}

/// 解析出来的整份补丁。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sections {
    pub files: Vec<FileSection>,
}

impl Sections {
    /// 解析补丁文本。无法识别的行按上下文忽略，不影响已识别的片段。
    pub fn parse(text: &str) -> Sections {
        let mut files: Vec<FileSection> = Vec::new();
        let mut current: Option<FileSection> = None;
        let mut hunk: Option<Hunk> = None;
        for raw in text.lines() {
            if raw.starts_with("diff --git ") {
                if let Some(hunk) = hunk.take()
                    && let Some(file) = current.as_mut()
                {
                    file.hunks.push(hunk);
                }
                if let Some(file) = current.take() {
                    files.push(file);
                }
                let path = raw
                    .split(" b/")
                    .nth(1)
                    .map(|tail| tail.to_string())
                    .unwrap_or_default();
                current = Some(FileSection {
                    header: vec![raw.to_string()],
                    path,
                    hunks: Vec::new(),
                });
                continue;
            }
            let Some(file) = current.as_mut() else {
                continue;
            };
            if raw.starts_with("@@") {
                if let Some(hunk) = hunk.take() {
                    file.hunks.push(hunk);
                }
                let (old_start, new_start) = parse_hunk_header(raw);
                hunk = Some(Hunk {
                    header: raw.to_string(),
                    lines: Vec::new(),
                    old_start,
                    new_start,
                });
                continue;
            }
            match hunk.as_mut() {
                Some(hunk) => {
                    // `\ No newline at end of file` 之类的标记一并保留。
                    hunk.lines.push(raw.to_string());
                }
                None => file.header.push(raw.to_string()),
            }
        }
        if let Some(hunk) = hunk.take()
            && let Some(file) = current.as_mut()
        {
            file.hunks.push(hunk);
        }
        if let Some(file) = current.take() {
            files.push(file);
        }
        return Sections { files };
    }

    /// 只含某个 hunk 的补丁。
    pub fn hunk_patch(&self, file_index: usize, hunk_index: usize) -> Option<String> {
        let file = self.files.get(file_index)?;
        let hunk = file.hunks.get(hunk_index)?;
        let mut out: Vec<String> = file.header.clone();
        out.push(hunk.header.clone());
        out.extend(hunk.lines.iter().cloned());
        return Some(finish(out));
    }

    /// 只含某个 hunk 里某一行的补丁。
    ///
    /// 新增行合成 `@@ -<旧位置>,0 +<新行号>,1 @@`；删除行合成
    /// `@@ -<旧行号>,1 +<新位置>,0 @@`。上下文行没有可暂存的内容，返回 `None`。
    pub fn line_patch(
        &self,
        file_index: usize,
        hunk_index: usize,
        line_index: usize,
    ) -> Option<String> {
        let file = self.files.get(file_index)?;
        let hunk = file.hunks.get(hunk_index)?;
        let raw = hunk.lines.get(line_index)?;
        let kind = classify(raw)?;
        // 走到目标行之前，记下两侧各自「已经过去多少行」，得到插入/删除的位置。
        let mut old_cursor = hunk.old_start;
        let mut new_cursor = hunk.new_start;
        for previous in hunk.lines.iter().take(line_index) {
            match classify(previous)? {
                LineKind::Add => new_cursor += 1,
                LineKind::Remove => old_cursor += 1,
                LineKind::Context => {
                    old_cursor += 1;
                    new_cursor += 1;
                }
            }
        }
        let header = match kind {
            LineKind::Add => format!("@@ -{},0 +{},1 @@", old_cursor.saturating_sub(1), new_cursor),
            LineKind::Remove => format!("@@ -{},1 +{},0 @@", old_cursor, new_cursor.saturating_sub(1)),
            LineKind::Context => return None,
        };
        let mut out: Vec<String> = file.header.clone();
        out.push(header);
        out.push(raw.clone());
        return Some(finish(out));
    }
}

/// 行首前缀 → 类别。
pub fn classify(raw: &str) -> Option<LineKind> {
    return match raw.chars().next()? {
        '+' => Some(LineKind::Add),
        '-' => Some(LineKind::Remove),
        ' ' => Some(LineKind::Context),
        _ => None,
    };
}

/// 补丁必须以换行结尾，`git apply` 才认。
fn finish(mut lines: Vec<String>) -> String {
    let mut text = lines.join("\n");
    lines.clear();
    text.push('\n');
    return text;
}

/// 从 `@@ -12,3 +12,4 @@ 说明` 里取两侧的起始行号。
fn parse_hunk_header(raw: &str) -> (u32, u32) {
    let mut old_start = 1;
    let mut new_start = 1;
    for (index, part) in raw.split_whitespace().enumerate() {
        if index == 0 || !part.starts_with(['-', '+']) {
            continue;
        }
        let number = part[1..]
            .split(',')
            .next()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(1);
        if part.starts_with('-') {
            old_start = number;
        } else {
            new_start = number;
        }
    }
    return (old_start, new_start);
}

#[cfg(test)]
mod tests {
    use super::*;

    const PATCH: &str = "\
diff --git a/src/lib.rs b/src/lib.rs
index 1111111..2222222 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,3 +1,3 @@
 line1
-old2
+new2
 line3
@@ -10,3 +10,4 @@ fn later() {
 ctx10
+added11
 ctx12
";

    #[test]
    fn parses_files_and_hunks() {
        let sections = Sections::parse(PATCH);
        assert_eq!(sections.files.len(), 1);
        let file = &sections.files[0];
        assert_eq!(file.path, "src/lib.rs");
        assert_eq!(file.hunks.len(), 2);
        assert_eq!(file.hunks[0].old_start, 1);
        assert_eq!(file.hunks[1].new_start, 10);
        // 文件头应包含 diff/index/---/+++ 四行。
        assert_eq!(file.header.len(), 4);
    }

    #[test]
    fn hunk_patch_keeps_the_file_header() {
        let sections = Sections::parse(PATCH);
        let patch = sections.hunk_patch(0, 1).expect("第二个 hunk");
        assert!(patch.starts_with("diff --git a/src/lib.rs b/src/lib.rs"));
        assert!(patch.contains("+added11"));
        assert!(!patch.contains("-old2"), "不应带上第一个 hunk 的内容");
        assert!(patch.ends_with('\n'));
    }

    #[test]
    fn single_added_line_gets_a_zero_length_old_side() {
        let sections = Sections::parse(PATCH);
        // 第二个 hunk 的第 2 行是新增行（下标 1）。
        let patch = sections.line_patch(0, 1, 1).expect("新增行");
        assert!(patch.contains("@@ -10,0 +11,1 @@"), "实际：{patch}");
        assert!(patch.contains("+added11"));
        assert!(!patch.contains("ctx10"));
    }

    #[test]
    fn single_removed_line_gets_a_zero_length_new_side() {
        let sections = Sections::parse(PATCH);
        // 第一个 hunk 的第 2 行是删除行（下标 1）。
        let patch = sections.line_patch(0, 0, 1).expect("删除行");
        assert!(patch.contains("@@ -2,1 +1,0 @@"), "实际：{patch}");
        assert!(patch.contains("-old2"));
    }

    #[test]
    fn context_lines_and_bad_indexes_have_no_patch() {
        let sections = Sections::parse(PATCH);
        assert_eq!(sections.line_patch(0, 0, 0), None, "上下文行不可暂存");
        assert_eq!(sections.line_patch(9, 0, 0), None);
        assert_eq!(sections.line_patch(0, 9, 0), None);
        assert_eq!(sections.hunk_patch(0, 9), None);
    }
}
