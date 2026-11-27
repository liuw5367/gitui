//! diff 视图：把 `nsh_base::DiffView` 收成应用统一的入口。
//!
//! `DiffView` 负责差异正文、布局、软换行、行号、指示条与 hunk 导航；共享差异面板
//! 选择传入变更片段或完整文件 patch，本层固定外观并连接应用操作。

use crate::ui;
use nsh_base::{
    DiffExpand, DiffHunk, DiffIndicators, DiffLayout, DiffLine, DiffLineRange, DiffSeparator,
    DiffSide, DiffViewApi, DiffViewEvent, diff_view,
};
use nsh_core::prelude::*;

/// NSH 0.6.0 uses DiffView's line height for file headers as well as code rows.
const MULTI_FILE_ROW_HEIGHT: f32 = 22.0;

/// 当前 DiffView 的导航入口。历史差异只读，不显示变更暂存按钮。
#[derive(Clone)]
pub(crate) struct DiffHandle {
    reference: ComponentRef<nsh_base::DiffView>,
}

impl DiffHandle {
    pub(crate) fn new() -> Self {
        return Self {
            reference: component_ref(),
        };
    }

    pub(crate) fn next_hunk(&self) -> Option<u32> {
        return self
            .reference
            .api::<DiffViewApi>()
            .and_then(|api| api.next_hunk());
    }

    pub(crate) fn previous_hunk(&self) -> Option<u32> {
        return self
            .reference
            .api::<DiffViewApi>()
            .and_then(|api| api.previous_hunk());
    }

    pub(crate) fn set_scroll_offset(&self, offset: Points) {
        if let Some(api) = self.reference.api::<DiffViewApi>() {
            let mut point = Point::default();
            point.y = offset;
            api.set_scroll_offset(point);
        }
    }

    pub(crate) fn expand(&self, gap: u32) {
        if let Some(api) = self.reference.api::<DiffViewApi>() {
            api.expand(gap, Some(DiffExpand::All), Some(0));
        }
    }
}

/// 两种视图。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Layout {
    /// 左右分栏。
    Split,
    /// 统一纵向。
    Unified,
}

/// 一块 diff。`patch` 是 unified diff 文本；`wrap` 为真时软换行。
///
/// `show_full_file` 表示传入的 patch 已包含完整文件上下文；语言由 patch 路径逐文件识别。
pub(crate) fn diff_surface(
    patch: SharedString,
    layout: Layout,
    wrap: bool,
    show_full_file: bool,
    handle: &DiffHandle,
    language: Signal<SharedString>,
    on_hunk_copy: std::sync::Arc<dyn Fn(&DiffHunk)>,
) -> impl IntoElement + use<> {
    return configured_surface(
        patch,
        layout,
        wrap,
        show_full_file,
        handle,
        language,
        on_hunk_copy,
        None,
    );
}

/// 用 DiffView 显示单个完整源码文件，并将选中行与纵向滚动交给调用方同步。
/// 点击回调使用 DiffView 的零起始索引，选中行使用 Git blame 的一起始行号。
pub(crate) fn source_file(
    source: SharedString,
    path: SharedString,
    selected_line: Memo<Option<u32>>,
    handle: &DiffHandle,
    on_line_press: impl Fn(u32) + 'static,
    on_scroll_input: impl Fn() + 'static,
    on_scroll: impl Fn(Points) + 'static,
) -> impl IntoElement {
    let reference = handle.reference.clone();
    let laid_out = signal(false);
    effect(move || {
        let selected = selected_line.get();
        if laid_out.get()
            && let Some(api) = reference.api::<DiffViewApi>()
        {
            api.set_selected_lines(selected.map(|line| {
                let index = line.saturating_sub(1);
                let mut range = DiffLineRange::default();
                range.side = DiffSide::New;
                range.start = index;
                range.end = index;
                range
            }));
        }
    });
    return diff_view()
        .w_full()
        .h_full()
        .component_ref(&handle.reference)
        .new_value(Some(source))
        .language(Some(path))
        .layout(DiffLayout::Unified)
        .expand_all(true)
        .line_numbers(true)
        .indicators(DiffIndicators::None)
        .background(false)
        .font_family(ui::MONO)
        .font_size(Points(ui::CODE))
        .line_height(Points(ui::CODE_LINE))
        .soft_wrap(false)
        .on_line_number_press(move |line: &DiffLine| on_line_press(line.line))
        .track_scroll_input(true)
        .on_scroll_input(on_scroll_input)
        .on_scroll(move |event: &DiffViewEvent| {
            if let DiffViewEvent::Scroll(offset, content, viewport) = event {
                let limit = (content.height - viewport.height).max(Points::ZERO);
                on_scroll(offset.y.clamp(Points::ZERO, limit));
            }
        })
        .on_layout(move |_: &Bounds<Points>| {
            if !laid_out.get_untracked() {
                laid_out.set(true);
            }
        });
}

#[derive(Clone)]
pub(crate) struct StagingActions {
    /// 展开 diff 中每个 hunk 是否包含暂存改动。
    pub(crate) hunk_staged: std::rc::Rc<dyn Fn(usize) -> bool>,
    /// 原始 Git patch，保留真实 hunk 边界供全文视图重建分组。
    pub(crate) location_patch: SharedString,
    pub(crate) on_piece: std::sync::Arc<dyn Fn(usize, bool)>,
    pub(crate) on_discard: std::sync::Arc<dyn Fn(usize)>,
}

pub(crate) fn configured_surface(
    patch: SharedString,
    layout: Layout,
    wrap: bool,
    show_full_file: bool,
    handle: &DiffHandle,
    language: Signal<SharedString>,
    on_hunk_copy: std::sync::Arc<dyn Fn(&DiffHunk)>,
    staging: Option<StagingActions>,
) -> impl IntoElement + use<> {
    let patch = if show_full_file {
        staging.as_ref().map_or(patch.clone(), |staging| {
            SharedString::from(
                full_context_with_hunk_headers(patch.as_str(), staging.location_patch.as_str())
                    .unwrap_or_else(|| staging.location_patch.to_string()),
            )
        })
    } else {
        patch
    };
    let layout_name = match layout {
        Layout::Split => "split",
        Layout::Unified => "unified",
    };
    let patch_text = patch.as_str();
    let file_count = patch_text
        .lines()
        .filter(|line| line.starts_with("diff --git "))
        .count();
    let line_height = if file_count > 1 {
        MULTI_FILE_ROW_HEIGHT
    } else {
        ui::CODE_LINE
    };
    log::debug!(
        target: "gitui::diff",
        "event=diff_surface.mount bytes={} files={} hunks={} layout={} wrap={} full_file={}",
        patch_text.len(),
        file_count,
        patch_text.lines().filter(|line| line.starts_with("@@")).count(),
        layout_name,
        wrap,
        show_full_file
    );
    let layout = match layout {
        Layout::Split => DiffLayout::Split,
        Layout::Unified => DiffLayout::Unified,
    };
    let mut classes = nsh_base::DiffViewClasses::default();
    classes.line_added = Some(SharedString::from(
        "bg-green-600/15 data-dark:bg-green-400/15",
    ));
    classes.line_deleted = Some(SharedString::from("bg-red-600/15 data-dark:bg-red-400/15"));
    let on_hunk_copy = std::rc::Rc::new(on_hunk_copy);
    let mut surface = diff_view()
        .w_full()
        .h_full()
        .component_ref(&handle.reference)
        .patch(Some(patch))
        .classes(classes)
        .layout(layout)
        .soft_wrap(wrap)
        // Each supplied patch contains either Git's compact context or the full file.
        // Keep all context lines that are present; the source patch controls availability.
        .expand_all(true)
        .line_numbers(true)
        .background(true)
        .font_family(ui::MONO)
        .font_size(Points(ui::CODE))
        .line_height(Points(line_height))
        .indicators(DiffIndicators::None)
        .render_hunk_actions(move |hunk: &DiffHunk| -> AnyElement {
            let handler = on_hunk_copy.clone();
            let hunk = hunk.clone();
            return crate::ui::button::icon_button_tip(
                "copy",
                {
                    let language = language;
                    move || crate::i18n::t(language.get().as_str(), "diff.copyHunk")
                },
                crate::ui::button::ButtonSize::Small,
                false,
                {
                    let language = language;
                    move || crate::i18n::t(language.get().as_str(), "diff.copyHunk")
                },
                move || handler(&hunk),
            )
            .into_element();
        });
    if let Some(staging) = staging {
        let on_piece = staging.on_piece.clone();
        let on_discard = staging.on_discard.clone();
        let hunk_staged = staging.hunk_staged.clone();
        let stage_handle = handle.clone();
        surface = surface.render_separator(move |separator: &DiffSeparator| -> AnyElement {
            let lines = separator.lines;
            let gap = separator.gap;
            let header = separator.header.clone().unwrap_or_default();
            let language = language;
            let mut row = view()
                .w_full()
                .h_full()
                .pl(px(ui::GAP_SM))
                .pr(px(ui::GAP_XL))
                .flex_row()
                .items_center()
                .gap(px(ui::GAP_SM))
                .justify_between()
                .bg(move || crate::design::panel2());
            if lines > 0 {
                let handle = stage_handle.clone();
                row = row.child(crate::ui::button::button(
                    move || format!("⋯ {lines}"),
                    crate::ui::button::ButtonVariant::Ghost,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || handle.expand(gap),
                ));
            } else {
                row = row.child(
                    text(header)
                        .flex_1()
                        .text_size(px(ui::CODE))
                        .text_color(move || crate::design::text3()),
                );
                let apply = on_piece.clone();
                let gap = separator.gap as usize;
                let checked = {
                    let hunk_staged = hunk_staged.clone();
                    std::rc::Rc::new(move || hunk_staged(gap))
                };
                let label_checked = checked.clone();
                let label_language = language;
                // Changes 用带文案的复选框表达暂存状态；文字与方框共用切换动作。
                row = row.child(crate::ui::check::check_inline(
                    move || {
                        crate::i18n::t(
                            label_language.get().as_str(),
                            if label_checked() {
                                "changes.unstageHunk"
                            } else {
                                "changes.stageHunk"
                            },
                        )
                    },
                    move || checked(),
                    22.0,
                    12.0,
                    false,
                    move |next| apply(gap, !next),
                ));
                let on_discard = on_discard.clone();
                let discard_label =
                    move || crate::i18n::t(language.get().as_str(), "changes.discardHunk");
                let discard_tip =
                    move || crate::i18n::t(language.get().as_str(), "changes.discardHunk");
                row = row.child(crate::ui::button::icon_button_tuned_tip(
                    "x",
                    discard_label,
                    crate::ui::button::ButtonSize::Small,
                    Some(crate::ui::icon::IconSize::Compact),
                    Some(std::rc::Rc::new(crate::design::danger)),
                    None,
                    false,
                    discard_tip,
                    move || on_discard(gap),
                ));
            }
            row.into_element()
        });
    }
    return surface;
}

/// 在全文补丁里保留默认 diff 的 hunk 边界，让每个变更组继续对应原始暂存操作。
fn full_context_with_hunk_headers(full_context: &str, raw_patch: &str) -> Option<String> {
    let full_sections = domain::stage::Sections::parse(full_context);
    let raw_sections = domain::stage::Sections::parse(raw_patch);
    let (Some(full_file), Some(raw_file)) =
        (full_sections.files.first(), raw_sections.files.first())
    else {
        return None;
    };
    if raw_file.hunks.is_empty() {
        return Some(full_context.to_string());
    }
    if raw_file.hunks.len() == 1 {
        return Some(full_context.to_string());
    }
    if full_file.hunks.len() != 1 {
        return None;
    }
    let full_hunk = &full_file.hunks[0];
    let mut boundaries = vec![(0, full_hunk.old_start, full_hunk.new_start)];
    for hunk in raw_file.hunks.iter().skip(1) {
        let mut old_line = full_hunk.old_start;
        let mut new_line = full_hunk.new_start;
        let mut index = None;
        for (line_index, line) in full_hunk.lines.iter().enumerate() {
            if old_line == hunk.old_start && new_line == hunk.new_start {
                index = Some(line_index);
                break;
            }
            match domain::stage::classify(line) {
                Some(domain::stage::LineKind::Add) => new_line += 1,
                Some(domain::stage::LineKind::Remove) => old_line += 1,
                Some(domain::stage::LineKind::Context) => {
                    old_line += 1;
                    new_line += 1;
                }
                None => {}
            }
        }
        let Some(index) = index else {
            return None;
        };
        boundaries.push((index, hunk.old_start, hunk.new_start));
    }
    if boundaries.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
        return None;
    }

    let mut lines = full_file.header.clone();
    for (index, (start, old_start, new_start)) in boundaries.iter().enumerate() {
        let end = boundaries
            .get(index + 1)
            .map(|boundary| boundary.0)
            .unwrap_or(full_hunk.lines.len());
        let body = &full_hunk.lines[*start..end];
        let (old_count, new_count) = body.iter().fold((0, 0), |(old, new), line| {
            match domain::stage::classify(line) {
                Some(domain::stage::LineKind::Add) => (old, new + 1),
                Some(domain::stage::LineKind::Remove) => (old + 1, new),
                Some(domain::stage::LineKind::Context) => (old + 1, new + 1),
                None => (old, new),
            }
        });
        lines.push(format!(
            "@@ -{old_start},{old_count} +{new_start},{new_count} @@"
        ));
        lines.extend(body.iter().cloned());
    }
    let mut patch = lines.join("\n");
    patch.push('\n');
    return Some(patch);
}

/// 预览 Gallery 中可写变更的暂存外观。
pub(crate) fn staging_surface(
    patch: SharedString,
    staged: bool,
    language: Signal<SharedString>,
    on_piece: std::sync::Arc<dyn Fn(usize, bool)>,
    on_discard: std::sync::Arc<dyn Fn(usize)>,
) -> impl IntoElement {
    let handle = DiffHandle::new();
    let location_patch = patch.clone();
    return configured_surface(
        patch,
        Layout::Unified,
        false,
        true,
        &handle,
        language,
        std::sync::Arc::new(|_| {}),
        Some(StagingActions {
            hunk_staged: std::rc::Rc::new(move |_| staged),
            location_patch,
            on_piece,
            on_discard,
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[component]
    fn DiffFixture() -> impl IntoElement {
        let language = signal(SharedString::from("en"));
        let handle = DiffHandle::new();
        diff_surface(
            SharedString::from(
                "diff --git a/main.rs b/main.rs\n--- a/main.rs\n+++ b/main.rs\n@@ -1 +1 @@\n-let count = 41;\n+let count = 42;\n",
            ),
            Layout::Split,
            false,
            true,
            &handle,
            language,
            std::sync::Arc::new(|_| {}),
        )
    }

    #[component]
    fn HorizontalDiffFixture() -> impl IntoElement {
        let long = "x".repeat(256);
        diff_view()
            .w_full()
            .h_full()
            .old_value(Some(SharedString::from(format!(
                "{}\n{long}",
                "a".repeat(32)
            ))))
            .new_value(Some(SharedString::from(format!(
                "{}\n{long}",
                "z".repeat(32)
            ))))
            .layout(DiffLayout::Split)
    }

    #[test]
    fn either_split_side_can_drive_horizontal_scrolling() {
        use nsh_core::{
            Command, MountItem,
            element::Root,
            platform::{Idiom, Os},
        };
        use std::sync::{Arc, Mutex};

        let old = "a".repeat(32);
        let new = "z".repeat(32);
        let mut config = nsh_engine::EngineConfig {
            viewport: Size::new(px(900.0), px(600.0)),
            ..nsh_engine::EngineConfig::new(Root::of::<HorizontalDiffFixture>())
        };
        config.environment.os = Os::MacOs;
        config.environment.idiom = Idiom::Desktop;
        let mut app = nsh_testing::Headless::start(config).unwrap();
        app.wait_for(&format!("text:{old}")).unwrap();
        let writes = Arc::new(Mutex::new(Vec::new()));
        let observed = writes.clone();
        app.observe(move |_, batch| {
            observed
                .lock()
                .unwrap()
                .extend(batch.items.iter().filter_map(|item| {
                    let MountItem::Command {
                        id,
                        command: Command::ScrollTo { x, .. },
                    } = item
                    else {
                        return None;
                    };
                    Some((*id, *x))
                }));
        });

        app.scroll(&format!("text:{old}"), px(80.0)).unwrap();
        let from_old = std::mem::take(&mut *writes.lock().unwrap());
        let (follower, x) = *from_old.last().expect("left side moves the right side");
        assert_eq!(x, px(80.0));
        assert!(from_old.iter().all(|(id, _)| *id == follower));

        app.wheel(&format!("text:{new}"), point(px(60.0), px(0.0)))
            .unwrap();
        let from_new = std::mem::take(&mut *writes.lock().unwrap());
        let (other_follower, x) = *from_new.last().expect("right wheel moves the left side");
        assert_eq!(x, px(140.0));
        assert_ne!(other_follower, follower);
        assert!(from_new.iter().all(|(id, _)| *id == other_follower));
    }

    const STAGING_PATCH: &str = "diff --git a/main.rs b/main.rs\n--- a/main.rs\n+++ b/main.rs\n@@ -1,2 +1,3 @@\n fn main() {\n+    let count = 42;\n }\n";
    static ACTIONS: std::sync::Mutex<Vec<(usize, bool)>> = std::sync::Mutex::new(Vec::new());
    static DISCARDED: std::sync::Mutex<Vec<usize>> = std::sync::Mutex::new(Vec::new());

    #[component]
    fn StagingFixture() -> impl IntoElement {
        staging_surface(
            SharedString::from(STAGING_PATCH),
            false,
            signal(SharedString::from("en")),
            std::sync::Arc::new(|hunk, reverse| ACTIONS.lock().unwrap().push((hunk, reverse))),
            std::sync::Arc::new(|hunk| DISCARDED.lock().unwrap().push(hunk)),
        )
    }

    #[test]
    fn staging_exposes_hunk_action() {
        ACTIONS.lock().unwrap().clear();
        DISCARDED.lock().unwrap().clear();
        let mut app = nsh_testing::Headless::start(nsh_engine::EngineConfig {
            viewport: Size::new(px(800.0), px(600.0)),
            ..nsh_engine::EngineConfig::new(nsh_core::element::Root::of::<StagingFixture>())
        })
        .unwrap();
        app.first_render().unwrap();
        app.press("Stage Hunk").unwrap();
        assert_eq!(*ACTIONS.lock().unwrap(), vec![(0, false)]);
        app.press("Discard Hunk").unwrap();
        assert_eq!(*DISCARDED.lock().unwrap(), vec![0]);
    }

    #[test]
    fn full_context_keeps_each_raw_hunk_as_a_staging_group() {
        let header = "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n";
        let mut full_lines = vec!["@@ -1,30 +1,30 @@".to_string()];
        for line in 1..=30 {
            match line {
                2 => {
                    full_lines.push("-line2".to_string());
                    full_lines.push("+line2-new".to_string());
                }
                14 => {
                    full_lines.push("-line14".to_string());
                    full_lines.push("+line14-new".to_string());
                }
                26 => {
                    full_lines.push("-line26".to_string());
                    full_lines.push("+line26-new".to_string());
                }
                _ => full_lines.push(format!(" line{line}")),
            }
        }
        let full = format!("{header}{}\n", full_lines.join("\n"));
        let mut raw_lines = vec!["@@ -1,5 +1,5 @@".to_string()];
        raw_lines.extend([
            " line1".to_string(),
            "-line2".to_string(),
            "+line2-new".to_string(),
            " line3".to_string(),
            " line4".to_string(),
            " line5".to_string(),
            "@@ -11,7 +11,7 @@".to_string(),
        ]);
        raw_lines.extend((11..=17).flat_map(|line| match line {
            14 => vec!["-line14".to_string(), "+line14-new".to_string()],
            _ => vec![format!(" line{line}")],
        }));
        raw_lines.push("@@ -23,7 +23,7 @@".to_string());
        raw_lines.extend((23..=29).flat_map(|line| match line {
            26 => vec!["-line26".to_string(), "+line26-new".to_string()],
            _ => vec![format!(" line{line}")],
        }));
        let raw = format!("{header}{}\n", raw_lines.join("\n"));

        let display = full_context_with_hunk_headers(&full, &raw).expect("hunk boundaries");
        let sections = domain::stage::Sections::parse(&display);
        assert_eq!(sections.files[0].hunks.len(), 3);
        assert!(
            sections.files[0].hunks[0]
                .lines
                .iter()
                .any(|line| line == "+line2-new")
        );
        assert!(
            sections.files[0].hunks[1]
                .lines
                .iter()
                .any(|line| line == "+line14-new")
        );
        assert!(
            sections.files[0].hunks[2]
                .lines
                .iter()
                .any(|line| line == "+line26-new")
        );
    }

    #[test]
    fn diff_has_visible_highlighted_content() {
        let mut app = nsh_testing::Headless::start(nsh_engine::EngineConfig {
            viewport: Size::new(px(800.0), px(600.0)),
            ..nsh_engine::EngineConfig::new(nsh_core::element::Root::of::<DiffFixture>())
        })
        .unwrap();
        app.first_render().unwrap();
        app.wait_for("let count = 42;").unwrap();
        app.wait_until(
            "syntax highlighting",
            std::time::Duration::from_secs(5),
            |screen| {
                screen.layout_nodes().iter().any(|node| {
                    node.text.as_ref().is_some_and(|text| {
                        let spans = &text.text.spans;
                        text.text.text == "let count = 42;"
                            && spans.first().is_some_and(|first| {
                                spans
                                    .iter()
                                    .any(|span| span.style.color != first.style.color)
                            })
                    })
                })
            },
        )
        .unwrap();
        let nodes = app.screen().layout_nodes();
        let mut node = nodes
            .iter()
            .find(|node| {
                node.text
                    .as_ref()
                    .is_some_and(|text| text.text.text == "let count = 42;")
            })
            .unwrap();
        let spans = &node.text.as_ref().unwrap().text.spans;
        assert!(
            spans
                .iter()
                .any(|span| span.style.color != spans[0].style.color),
            "代码没有语法高亮"
        );
        while let Some(parent) = nodes
            .iter()
            .find(|parent| parent.id == node.parent && parent.id != node.id)
        {
            if parent.clips {
                assert!(
                    parent.bounds.size.height.0 > 0.0,
                    "Diff 内容被零高度容器裁剪"
                );
            }
            node = parent;
        }
    }

    #[test]
    fn layout_values_are_distinct() {
        // 两个视图用于「Unified / Split」切换，不能等价。
        assert_ne!(Layout::Split, Layout::Unified);
    }
}
