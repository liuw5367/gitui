//! 补丁解析：把 `git show --patch` 的文本整理成可直接渲染的行。
//!
//! 只做展示所需的分类与行号计算，不构造补丁、不判断 hunk 归属；写路径的补丁生成
//! 在后续阶段单独实现。

/// 一行补丁的类别。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffLineKind {
    /// `+` 新增。
    Add,
    /// `-` 删除。
    Remove,
    /// 上下文行。
    Context,
    /// `@@ -a,b +c,d @@`。
    Hunk,
    /// 文件头、索引行、模式行等元信息。
    Meta,
}

/// 一行补丁。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    /// 旧文件行号；新增行与元信息行为 `None`。
    pub old: Option<u32>,
    /// 新文件行号；删除行与元信息行为 `None`。
    pub new: Option<u32>,
    /// 去掉前缀后的可见文本。
    pub text: String,
}

/// 一个文件在补丁中的起始位置与统计。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffFile {
    pub path: String,
    pub additions: u32,
    pub deletions: u32,
    /// 在返回的行列表中的起始下标。
    pub start: usize,
}

/// 解析后的补丁。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Patch {
    pub lines: Vec<DiffLine>,
    pub files: Vec<DiffFile>,
}

impl Patch {
    /// 依次读取按行分类的结果。
    pub fn parse(text: &str) -> Patch {
        let mut patch = Patch::default();
        let mut old_line: u32 = 0;
        let mut new_line: u32 = 0;
        let mut current_file: Option<DiffFile> = None;
        for raw in text.lines() {
            // 补丁里的行前缀才是分类依据，不能用 trim。
            let first = raw.chars().next().unwrap_or(' ');
            if let Some(rest) = raw.strip_prefix("diff --git ") {
                if let Some(file) = current_file.take() {
                    patch.files.push(file);
                }
                let path = rest
                    .rsplit_once(" b/")
                    .map(|(_, path)| path.to_string())
                    .unwrap_or_else(|| rest.to_string());
                current_file = Some(DiffFile {
                    path,
                    additions: 0,
                    deletions: 0,
                    start: patch.lines.len(),
                });
                patch.lines.push(DiffLine {
                    kind: DiffLineKind::Meta,
                    old: None,
                    new: None,
                    text: rest.to_string(),
                });
                continue;
            }
            // `--- a/x` / `+++ b/x` 是文件头，不是删除/新增行；必须在按首字符
            // 分类之前拦下，否则它们会被当成一行删除和一行新增。
            if raw.starts_with("--- ") || raw.starts_with("+++ ") {
                patch.lines.push(DiffLine {
                    kind: DiffLineKind::Meta,
                    old: None,
                    new: None,
                    text: raw.to_string(),
                });
                continue;
            }
            if raw.starts_with("@@") {
                let (old_start, new_start) = parse_hunk_header(raw);
                old_line = old_start;
                new_line = new_start;
                patch.lines.push(DiffLine {
                    kind: DiffLineKind::Hunk,
                    old: None,
                    new: None,
                    text: raw.to_string(),
                });
                continue;
            }
            match first {
                '+' => {
                    if let Some(file) = current_file.as_mut() {
                        file.additions += 1;
                    }
                    patch.lines.push(DiffLine {
                        kind: DiffLineKind::Add,
                        old: None,
                        new: Some(new_line),
                        text: raw.get(1..).unwrap_or("").to_string(),
                    });
                    new_line += 1;
                }
                '-' => {
                    if let Some(file) = current_file.as_mut() {
                        file.deletions += 1;
                    }
                    patch.lines.push(DiffLine {
                        kind: DiffLineKind::Remove,
                        old: Some(old_line),
                        new: None,
                        text: raw.get(1..).unwrap_or("").to_string(),
                    });
                    old_line += 1;
                }
                ' ' => {
                    patch.lines.push(DiffLine {
                        kind: DiffLineKind::Context,
                        old: Some(old_line),
                        new: Some(new_line),
                        text: raw.get(1..).unwrap_or("").to_string(),
                    });
                    old_line += 1;
                    new_line += 1;
                }
                _ => {
                    patch.lines.push(DiffLine {
                        kind: DiffLineKind::Meta,
                        old: None,
                        new: None,
                        text: raw.to_string(),
                    });
                }
            }
        }
        if let Some(file) = current_file.take() {
            patch.files.push(file);
        }
        return patch;
    }
}

/// 解析 `@@ -12,7 +12,9 @@ context`：返回旧、新的起始行号。
fn parse_hunk_header(line: &str) -> (u32, u32) {
    let mut old_start = 0;
    let mut new_start = 0;
    for part in line.trim_start_matches("@@").split(' ') {
        if let Some(range) = part.strip_prefix('-') {
            old_start = range
                .split(',')
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
        } else if let Some(range) = part.strip_prefix('+') {
            new_start = range
                .split(',')
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
        }
    }
    return (old_start, new_start);
}

/// 并排视图的一行：左右各一个单元格，可能为空（表示该侧没有对应的行）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitRow {
    pub left: Option<DiffLine>,
    pub right: Option<DiffLine>,
}

/// 把按行排列的补丁配成并排行。
///
/// 对齐规则（主流 git 客户端的最小形态）：连续的一段删除与紧跟的一段新增按位置
/// 一一配对，多出来的行留空；上下文行左右相同；`Hunk`/`Meta` 行只放左侧，右空。
///
/// 注意：设计稿没有并排 diff 的规格，这是按通用做法的推断实现。
pub fn split_rows(lines: &[DiffLine]) -> Vec<SplitRow> {
    let mut rows: Vec<SplitRow> = Vec::new();
    let mut index = 0usize;
    while index < lines.len() {
        let line = &lines[index];
        match line.kind {
            DiffLineKind::Context => {
                rows.push(SplitRow {
                    left: Some(line.clone()),
                    right: Some(line.clone()),
                });
                index += 1;
            }
            DiffLineKind::Remove | DiffLineKind::Add => {
                // 收集连续的删除，再收集紧跟的新增，两组按顺序配对。
                let mut removed: Vec<DiffLine> = Vec::new();
                while index < lines.len() && lines[index].kind == DiffLineKind::Remove {
                    removed.push(lines[index].clone());
                    index += 1;
                }
                let mut added: Vec<DiffLine> = Vec::new();
                while index < lines.len() && lines[index].kind == DiffLineKind::Add {
                    added.push(lines[index].clone());
                    index += 1;
                }
                let pairs = removed.len().max(added.len());
                for slot in 0..pairs {
                    rows.push(SplitRow {
                        left: removed.get(slot).cloned(),
                        right: added.get(slot).cloned(),
                    });
                }
            }
            DiffLineKind::Hunk | DiffLineKind::Meta => {
                rows.push(SplitRow {
                    left: Some(line.clone()),
                    right: None,
                });
                index += 1;
            }
        }
    }
    return rows;
}

#[cfg(test)]
mod tests {
    use super::*;

    // 原样字符串：补丁的分类依赖行首字符（含上下文行的前导空格），不能用
    // `\n\` 的续行写法，它会把下一行行首空白吞掉。
    const SAMPLE: &str = r#"diff --git a/a.txt b/a.txt
index 111..222 100644
--- a/a.txt
+++ b/a.txt
@@ -1,3 +1,4 @@
 one
-two
+TWO
+added
 three
"#;

    #[test]
    fn parses_kinds_line_numbers_and_counts() {
        let patch = Patch::parse(SAMPLE);
        let kinds: Vec<DiffLineKind> = patch.lines.iter().map(|line| line.kind).collect();
        assert_eq!(
            kinds,
            vec![
                DiffLineKind::Meta,
                DiffLineKind::Meta,
                DiffLineKind::Meta,
                DiffLineKind::Meta,
                DiffLineKind::Hunk,
                DiffLineKind::Context,
                DiffLineKind::Remove,
                DiffLineKind::Add,
                DiffLineKind::Add,
                DiffLineKind::Context,
            ]
        );
        let remove = patch
            .lines
            .iter()
            .find(|line| line.kind == DiffLineKind::Remove)
            .expect("remove line");
        assert_eq!(remove.old, Some(2));
        assert_eq!(remove.new, None);
        assert_eq!(remove.text, "two");

        let added: Vec<&DiffLine> = patch
            .lines
            .iter()
            .filter(|line| line.kind == DiffLineKind::Add)
            .collect();
        assert_eq!(added[0].new, Some(2));
        assert_eq!(added[1].new, Some(3));

        assert_eq!(patch.files.len(), 1);
        assert_eq!(patch.files[0].path, "a.txt");
        assert_eq!(patch.files[0].additions, 2);
        assert_eq!(patch.files[0].deletions, 1);
    }

    #[test]
    fn empty_patch_has_no_lines() {
        let patch = Patch::parse("");
        assert!(patch.lines.is_empty());
        assert!(patch.files.is_empty());
    }

    #[test]
    fn hunk_headers_set_the_starting_lines() {
        assert_eq!(parse_hunk_header("@@ -12,7 +12,9 @@ fn main()"), (12, 12));
        assert_eq!(parse_hunk_header("@@ -0,0 +1,3 @@"), (0, 1));
    }


    #[test]
    fn split_pairs_removed_and_added_lines() {
        let lines = vec![
            DiffLine {
                kind: DiffLineKind::Hunk,
                old: None,
                new: None,
                text: "@@ -1,3 +1,3 @@".to_string(),
            },
            DiffLine {
                kind: DiffLineKind::Context,
                old: Some(1),
                new: Some(1),
                text: "keep".to_string(),
            },
            DiffLine {
                kind: DiffLineKind::Remove,
                old: Some(2),
                new: None,
                text: "old".to_string(),
            },
            DiffLine {
                kind: DiffLineKind::Add,
                old: None,
                new: Some(2),
                text: "new".to_string(),
            },
        ];
        let rows = split_rows(&lines);
        assert_eq!(rows.len(), 3);
        // hunk 行只在左侧。
        assert!(rows[0].left.is_some() && rows[0].right.is_none());
        // 上下文行两侧相同。
        assert_eq!(rows[1].left, rows[1].right);
        // 删除与新增配对成同一行。
        assert_eq!(rows[2].left.as_ref().map(|line| line.text.as_str()), Some("old"));
        assert_eq!(rows[2].right.as_ref().map(|line| line.text.as_str()), Some("new"));
    }

    #[test]
    fn split_leaves_the_shorter_side_empty() {
        let lines = vec![
            DiffLine {
                kind: DiffLineKind::Remove,
                old: Some(1),
                new: None,
                text: "a".to_string(),
            },
            DiffLine {
                kind: DiffLineKind::Remove,
                old: Some(2),
                new: None,
                text: "b".to_string(),
            },
            DiffLine {
                kind: DiffLineKind::Add,
                old: None,
                new: Some(1),
                text: "only".to_string(),
            },
        ];
        let rows = split_rows(&lines);
        assert_eq!(rows.len(), 2, "两删一增应配成两行");
        assert_eq!(rows[0].right.as_ref().map(|line| line.text.as_str()), Some("only"));
        assert!(rows[1].right.is_none(), "多出来的删除行右侧为空");
    }
}
