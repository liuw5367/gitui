//! diff 视图：把 `nsh_base::DiffView` 收成应用统一的入口。
//!
//! 行为（分栏/统一、折叠展开、软换行、行号、指示条、hunk 导航）由 `DiffView`
//! 提供；本层固定外观，并把只读历史 hunk 的复制动作接到应用剪贴板。

use crate::ui;
use nsh_base::{DiffHunk, DiffIndicators, DiffLayout, DiffViewApi, diff_view};
use nsh_core::prelude::*;

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
/// `expand_all` 控制未改动上下文是否全部展开；语言由 patch 的文件路径逐文件识别。
pub(crate) fn diff_surface(
    patch: SharedString,
    layout: Layout,
    wrap: bool,
    expand_all: bool,
    handle: &DiffHandle,
    language: Signal<SharedString>,
    on_hunk_copy: std::sync::Arc<dyn Fn(&DiffHunk)>,
) -> impl IntoElement + use<> {
    let layout_name = match layout {
        Layout::Split => "split",
        Layout::Unified => "unified",
    };
    let patch_text = patch.as_str();
    log::debug!(
        target: "gitui::diff",
        "event=diff_surface.mount bytes={} files={} hunks={} layout={} wrap={} expand_all={}",
        patch_text.len(),
        patch_text.lines().filter(|line| line.starts_with("diff --git ")).count(),
        patch_text.lines().filter(|line| line.starts_with("@@")).count(),
        layout_name,
        wrap,
        expand_all
    );
    let layout = match layout {
        Layout::Split => DiffLayout::Split,
        Layout::Unified => DiffLayout::Unified,
    };
    let on_hunk_copy = std::rc::Rc::new(on_hunk_copy);
    return diff_view()
        .w_full()
        .h_full()
        .component_ref(&handle.reference)
        .patch(Some(patch))
        .layout(layout)
        .soft_wrap(wrap)
        .expand_all(expand_all)
        .line_numbers(true)
        .background(false)
        .font_family(ui::MONO)
        .font_size(Points(ui::CODE))
        .line_height(Points(ui::CODE_LINE))
        .indicators(DiffIndicators::Classic)
        .render_hunk_actions(move |hunk: &DiffHunk| -> AnyElement {
            let handler = on_hunk_copy.clone();
            let hunk = hunk.clone();
            return crate::ui::button::button(
                {
                    let language = language;
                    move || crate::i18n::t(language.get().as_str(), "diff.copyHunk")
                },
                crate::ui::button::ButtonVariant::Ghost,
                crate::ui::button::ButtonSize::Small,
                false,
                move || handler(&hunk),
            )
            .into_element();
        });
}

/// 可写变更仍由原始 Git hunk / 行定位，DiffView 只负责显示与高亮。
pub(crate) fn staging_surface(
    patch: SharedString,
    staged: bool,
    language: Signal<SharedString>,
    on_piece: std::sync::Arc<dyn Fn(usize, Option<usize>, bool)>,
) -> impl IntoElement {
    let locations = staging_annotations(patch.as_str());
    let annotations = locations
        .iter()
        .map(|(annotation, _, _, _)| annotation.clone())
        .collect::<Vec<_>>();
    diff_view()
        .w_full()
        .h_full()
        .patch(Some(patch))
        .layout(DiffLayout::Unified)
        .expand_all(true)
        .line_numbers(true)
        .background(true)
        .font_family(ui::MONO)
        .font_size(Points(ui::CODE))
        .line_height(Points(ui::CODE_LINE))
        .indicators(DiffIndicators::Classic)
        .annotations(annotations)
        .render_annotation(move |annotation: &nsh_base::DiffAnnotation| -> AnyElement {
            let Some((_, hunk, line, first)) = locations
                .iter()
                .find(|(item, _, _, _)| item.id == annotation.id)
            else {
                return view().into_element();
            };
            let (hunk, line, first) = (*hunk, *line, *first);
            let apply = on_piece.clone();
            let mut actions = view()
                .w_full()
                .h(px(ui::ROW))
                .flex_row()
                .items_center()
                .justify_end()
                .gap(px(ui::GAP_SM));
            if first {
                let apply = apply.clone();
                actions = actions.child(ui::button::button(
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
                    ui::button::ButtonVariant::Secondary,
                    ui::button::ButtonSize::Small,
                    false,
                    move || apply(hunk, None, staged),
                ));
            }
            actions
                .child(ui::button::button(
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
                    ui::button::ButtonVariant::Secondary,
                    ui::button::ButtonSize::Small,
                    false,
                    move || apply(hunk, Some(line), staged),
                ))
                .into_element()
        })
}

fn staging_annotations(patch: &str) -> Vec<(nsh_base::DiffAnnotation, usize, usize, bool)> {
    let sections = domain::stage::Sections::parse(patch);
    let mut annotations = Vec::new();
    let (mut old, mut new) = (0, 0);
    if let Some(file) = sections.files.first() {
        for (hunk, section) in file.hunks.iter().enumerate() {
            let mut first = true;
            // 行号是 DiffView 拼接各个 hunk 后的位置；应用索引保留原始正文中的上下文与无换行标记。
            for (line, raw) in section.lines.iter().enumerate() {
                let kind = domain::stage::classify(raw);
                if matches!(
                    kind,
                    Some(domain::stage::LineKind::Add | domain::stage::LineKind::Remove)
                ) {
                    let removed = kind == Some(domain::stage::LineKind::Remove);
                    let mut annotation = nsh_base::DiffAnnotation::default();
                    annotation.id = format!("piece:{hunk}:{line}").into();
                    annotation.side = if removed {
                        nsh_base::DiffSide::Old
                    } else {
                        nsh_base::DiffSide::New
                    };
                    annotation.line = if removed { old } else { new };
                    annotations.push((annotation, hunk, line, first));
                    first = false;
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
        let locations = staging_annotations(STAGING_PATCH);
        assert_eq!(locations.len(), 1);
        assert_eq!(
            (locations[0].1, locations[0].2, locations[0].3),
            (0, 1, true)
        );
        assert_eq!(locations[0].0.line, 1);
        let patch = domain::stage::Sections::parse(STAGING_PATCH)
            .line_patch(0, locations[0].1, locations[0].2)
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
        let locations = staging_annotations(raw);
        let indices = locations
            .iter()
            .map(|(a, h, l, first)| (a.side, a.line, *h, *l, *first))
            .collect::<Vec<_>>();
        assert_eq!(
            indices,
            vec![
                (nsh_base::DiffSide::Old, 1, 0, 1, true),
                (nsh_base::DiffSide::New, 1, 0, 2, false),
                (nsh_base::DiffSide::New, 3, 1, 1, true)
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
