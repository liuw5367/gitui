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
    pub(crate) staged: bool,
    /// 原始 Git patch，保留真实 hunk 与行索引供暂存操作定位。
    pub(crate) location_patch: SharedString,
    pub(crate) on_piece: std::sync::Arc<dyn Fn(usize, Option<usize>, bool)>,
    /// 全文模式把所有改动并成一个可见 hunk，按该 hunk 操作时整文件暂存。
    pub(crate) on_file: std::sync::Arc<dyn Fn()>,
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
    let locations = staging
        .as_ref()
        .map(|staging| staging_locations(staging.location_patch.as_str()));
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
        let locations = locations.unwrap_or_default();
        let on_piece = staging.on_piece.clone();
        let on_file = staging.on_file.clone();
        let line_piece = on_piece.clone();
        let staged = staging.staged;
        let stage_handle = handle.clone();
        surface = surface
            .render_separator(move |separator: &DiffSeparator| -> AnyElement {
                let lines = separator.lines;
                let gap = separator.gap;
                let header = separator.header.clone().unwrap_or_default();
                let language = language;
                let mut row = view()
                    .w_full()
                    .h_full()
                    .px(px(ui::GAP_SM))
                    .flex_row()
                    .items_center()
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
                    let apply_file = on_file.clone();
                    let gap = separator.gap as usize;
                    row = row.child(crate::ui::button::button(
                        move || {
                            crate::i18n::t(
                                language.get().as_str(),
                                if staged {
                                    "changes.unstageHunk"
                                } else {
                                    "changes.stageHunk"
                                },
                            )
                        },
                        crate::ui::button::ButtonVariant::Ghost,
                        crate::ui::button::ButtonSize::Small,
                        false,
                        move || {
                            if show_full_file {
                                apply_file();
                            } else {
                                apply(gap, None, staged);
                            }
                        },
                    ));
                }
                row.into_element()
            })
            .render_add_annotation(move |range: &DiffLineRange| -> AnyElement {
                let Some((hunk, line)) = locations
                    .iter()
                    .find(|(side, line, _, _)| {
                        *side == range.side && *line == range.start && range.start == range.end
                    })
                    .map(|(_, _, hunk, line)| (*hunk, *line))
                else {
                    return view().into_element();
                };
                let on_piece = line_piece.clone();
                crate::ui::button::button(
                    move || {
                        crate::i18n::t(
                            language.get().as_str(),
                            if staged {
                                "changes.unstageLine"
                            } else {
                                "changes.stageLine"
                            },
                        )
                    },
                    crate::ui::button::ButtonVariant::Ghost,
                    crate::ui::button::ButtonSize::Small,
                    false,
                    move || on_piece(hunk, Some(line), staged),
                )
                .into_element()
            });
    }
    return surface;
}

/// 预览 Gallery 中可写变更的暂存外观。
pub(crate) fn staging_surface(
    patch: SharedString,
    staged: bool,
    language: Signal<SharedString>,
    on_piece: std::sync::Arc<dyn Fn(usize, Option<usize>, bool)>,
) -> impl IntoElement {
    let handle = DiffHandle::new();
    let location_patch = patch.clone();
    let on_file = on_piece.clone();
    return configured_surface(
        patch,
        Layout::Unified,
        false,
        true,
        &handle,
        language,
        std::sync::Arc::new(|_| {}),
        Some(StagingActions {
            staged,
            location_patch,
            on_piece,
            on_file: std::sync::Arc::new(move || on_file(0, None, staged)),
        }),
    );
}

fn staging_locations(patch: &str) -> Vec<(DiffSide, u32, usize, usize)> {
    let sections = domain::stage::Sections::parse(patch);
    let mut annotations = Vec::new();
    if let Some(file) = sections.files.first() {
        for (hunk, section) in file.hunks.iter().enumerate() {
            let (mut old, mut new) = (
                section.old_start.saturating_sub(1),
                section.new_start.saturating_sub(1),
            );
            // 行号是 DiffView 拼接各个 hunk 后的位置；应用索引保留原始正文中的上下文与无换行标记。
            for (line, raw) in section.lines.iter().enumerate() {
                let kind = domain::stage::classify(raw);
                if matches!(
                    kind,
                    Some(domain::stage::LineKind::Add | domain::stage::LineKind::Remove)
                ) {
                    let removed = kind == Some(domain::stage::LineKind::Remove);
                    let side = if removed {
                        DiffSide::Old
                    } else {
                        DiffSide::New
                    };
                    annotations.push((side, if removed { old } else { new }, hunk, line));
                }
                match kind {
                    Some(domain::stage::LineKind::Context) => {
                        old += 1;
                        new += 1;
                    }
                    Some(domain::stage::LineKind::Remove) => old += 1,
                    Some(domain::stage::LineKind::Add) => new += 1,
                    None => {}
                }
            }
        }
    }
    annotations
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
    static ACTIONS: std::sync::Mutex<Vec<(usize, Option<usize>, bool)>> =
        std::sync::Mutex::new(Vec::new());

    #[component]
    fn StagingFixture() -> impl IntoElement {
        staging_surface(
            SharedString::from(STAGING_PATCH),
            false,
            signal(SharedString::from("en")),
            std::sync::Arc::new(|hunk, line, reverse| {
                ACTIONS.lock().unwrap().push((hunk, line, reverse))
            }),
        )
    }

    #[test]
    fn staging_preserves_context_offsets_and_actions() {
        let locations = staging_locations(STAGING_PATCH);
        assert_eq!(locations.len(), 1);
        assert_eq!((locations[0].2, locations[0].3), (0, 1));
        assert_eq!(locations[0].1, 1);
        let patch = domain::stage::Sections::parse(STAGING_PATCH)
            .line_patch(0, locations[0].2, locations[0].3)
            .unwrap();
        assert!(patch.contains("+    let count = 42;"));
        ACTIONS.lock().unwrap().clear();
        let mut app = nsh_testing::Headless::start(nsh_engine::EngineConfig {
            viewport: Size::new(px(800.0), px(600.0)),
            ..nsh_engine::EngineConfig::new(nsh_core::element::Root::of::<StagingFixture>())
        })
        .unwrap();
        app.first_render().unwrap();
        app.wait_for("Stage Line").unwrap();
        app.press("Stage Line").unwrap();
        app.press("Stage Hunk").unwrap();
        assert_eq!(
            *ACTIONS.lock().unwrap(),
            vec![(0, Some(1), false), (0, None, false)]
        );
    }

    #[test]
    fn staging_locations_keep_raw_indices_across_hunks() {
        let raw = "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -10,2 +10,2 @@\n context\n-old\n+new\n@@ -30 +30,2 @@\n tail\n+added\n";
        let locations = staging_locations(raw);
        let indices = locations
            .iter()
            .map(|(side, line, hunk, raw_line)| (*side, *line, *hunk, *raw_line))
            .collect::<Vec<_>>();
        assert_eq!(
            indices,
            vec![
                (nsh_base::DiffSide::Old, 10, 0, 1),
                (nsh_base::DiffSide::New, 10, 0, 2),
                (nsh_base::DiffSide::New, 30, 1, 1)
            ]
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
