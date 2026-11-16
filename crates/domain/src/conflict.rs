//! 冲突标记的解析与合并（设计板 14）。
//!
//! 只处理标准的 `<<<<<<<` / `|||||||` / `=======` / `>>>>>>>` 四类标记：Git 写出的
//! 冲突文件就是这种形态，`|||||||` 段只在配置了 `merge.conflictStyle=diff3` 时出现。
//! 解析结果是「片段序列」，合并时按选择逐段替换，未冲突的行原样保留。

/// 冲突文件的一个片段。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Segment {
    /// 冲突标记之外的原样内容（可能为空）。
    Common(Vec<String>),
    /// 一段冲突：可选 base、ours、theirs。
    Conflict {
        base: Option<Vec<String>>,
        ours: Vec<String>,
        theirs: Vec<String>,
    },
}

/// 合并时对每段冲突的选择。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Ours,
    Theirs,
    Both,
}

/// 解析冲突文件文本。
///
/// 标记不完整（例如用户已经手改坏了）时，把整份文本当作一段 `Common` 返回，
/// 由调用方决定是否提示；这样不会因为一个坏文件让整个视图崩掉。
pub fn parse(text: &str) -> Vec<Segment> {
    let mut segments: Vec<Segment> = Vec::new();
    let mut common: Vec<String> = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        if !line.starts_with("<<<<<<<") {
            common.push(line.to_string());
            continue;
        }
        // 进入冲突段。
        let mut ours: Vec<String> = Vec::new();
        let mut theirs: Vec<String> = Vec::new();
        let mut base: Option<Vec<String>> = None;
        let mut mode = "ours";
        let mut closed = false;
        for inner in lines.by_ref() {
            if inner.starts_with("|||||||") {
                base = Some(Vec::new());
                mode = "base";
                continue;
            }
            if inner.starts_with("=======") && mode != "theirs" {
                mode = "theirs";
                continue;
            }
            if inner.starts_with(">>>>>>>") {
                closed = true;
                break;
            }
            match mode {
                "ours" => ours.push(inner.to_string()),
                "base" => {
                    if let Some(base) = base.as_mut() {
                        base.push(inner.to_string());
                    }
                }
                _ => theirs.push(inner.to_string()),
            }
        }
        if !closed {
            // 标记不完整：把已读到的内容连同开头一起当普通文本。
            common.push(line.to_string());
            common.extend(ours);
            if let Some(base) = base {
                common.extend(base);
            }
            common.extend(theirs);
            break;
        }
        if !common.is_empty() {
            segments.push(Segment::Common(std::mem::take(&mut common)));
        }
        segments.push(Segment::Conflict { base, ours, theirs });
    }
    if !common.is_empty() {
        segments.push(Segment::Common(common));
    }
    return segments;
}

/// 文件里是否还有未解决的冲突标记。
pub fn has_markers(text: &str) -> bool {
    return text.lines().any(|line| {
        line.starts_with("<<<<<<<") || line.starts_with(">>>>>>>") || line.starts_with("|||||||")
    });
}

/// 按选择合并所有冲突段，返回新文本（保留末尾换行）。
pub fn merge(text: &str, side: Side) -> String {
    let segments = parse(text);
    let mut out: Vec<String> = Vec::new();
    for segment in segments {
        match segment {
            Segment::Common(lines) => out.extend(lines),
            Segment::Conflict { ours, theirs, .. } => match side {
                Side::Ours => out.extend(ours),
                Side::Theirs => out.extend(theirs),
                Side::Both => {
                    out.extend(ours);
                    out.extend(theirs);
                }
            },
        }
    }
    let mut merged = out.join("\n");
    if text.ends_with('\n') {
        merged.push('\n');
    }
    return merged;
}

/// 冲突段的数量。
pub fn conflict_count(text: &str) -> usize {
    return parse(text)
        .into_iter()
        .filter(|segment| matches!(segment, Segment::Conflict { .. }))
        .count();
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
one
<<<<<<< HEAD
mine
=======
yours
>>>>>>> feature
two
";

    #[test]
    fn parses_common_and_conflict_segments() {
        let segments = parse(SAMPLE);
        assert_eq!(segments.len(), 3, "前后各有普通段：{segments:?}");
        assert_eq!(conflict_count(SAMPLE), 1);
        match &segments[1] {
            Segment::Conflict { ours, theirs, base } => {
                assert_eq!(ours, &vec!["mine".to_string()]);
                assert_eq!(theirs, &vec!["yours".to_string()]);
                assert_eq!(base, &None);
            }
            other => panic!("第二段应是冲突：{other:?}"),
        }
    }

    #[test]
    fn merge_picks_each_side_and_keeps_common_lines() {
        assert_eq!(merge(SAMPLE, Side::Ours), "one\nmine\ntwo\n");
        assert_eq!(merge(SAMPLE, Side::Theirs), "one\nyours\ntwo\n");
        assert_eq!(merge(SAMPLE, Side::Both), "one\nmine\nyours\ntwo\n");
        assert!(!has_markers(&merge(SAMPLE, Side::Both)));
        assert!(has_markers(SAMPLE));
    }

    #[test]
    fn diff3_style_base_is_captured() {
        let text = "a\n<<<<<<< HEAD\nours\n||||||| base\nold\n=======\ntheirs\n>>>>>>> b\n";
        let segments = parse(text);
        match &segments[1] {
            Segment::Conflict { base, .. } => {
                assert_eq!(base.as_ref().map(|lines| lines.len()), Some(1));
            }
            other => panic!("应解析出 base：{other:?}"),
        }
        // 合并时 base 不参与结果。
        assert!(!merge(text, Side::Ours).contains("old"));
    }

    #[test]
    fn broken_markers_degrade_to_plain_text() {
        let text = "a\n<<<<<<< HEAD\nunterminated\n";
        assert_eq!(conflict_count(text), 0);
        let merged = merge(text, Side::Ours);
        assert!(merged.contains("unterminated"), "坏文件不应丢内容");
    }

    #[test]
    fn multiple_hunks_are_all_resolved() {
        let text = "\
<<<<<<< HEAD
a1
=======
b1
>>>>>>> x
mid
<<<<<<< HEAD
a2
=======
b2
>>>>>>> x
";
        assert_eq!(conflict_count(text), 2);
        assert_eq!(merge(text, Side::Theirs), "b1\nmid\nb2\n");
    }
}
