//! Files popover for the conflict resolver.

use super::super::state::ConflictFilesMode;
use crate::design;
use crate::i18n;
use nsh_core::prelude::*;
use std::collections::BTreeMap;

#[derive(Default)]
struct Directory {
    directories: BTreeMap<String, Directory>,
    files: Vec<String>,
}

pub(super) fn picker(
    language: Signal<SharedString>,
    paths: Vec<String>,
    unresolved: Vec<String>,
    selected: Option<String>,
    mode: Signal<ConflictFilesMode>,
    on_select: impl Fn(String) + Clone + 'static,
) -> impl IntoElement {
    view()
        .w(px(260.0))
        .h(px(294.0))
        .max_h(px(328.0))
        .flex_col()
        .rounded(px(8.0))
        .border_1()
        .border_color(move || design::line())
        .bg(move || design::content())
        .shadow_lg()
        .overflow_hidden()
        .child(
            view()
                .w_full()
                .h(px(36.0))
                .flex_shrink_0()
                .px(px(8.0))
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .bg(move || design::panel())
                .border_b_1()
                .border_color(move || design::line_soft())
                .child(
                    text(move || i18n::t(language.get().as_str(), "conflicts.files"))
                        .flex_1()
                        .text_size(px(crate::ui::SECONDARY))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(move || design::text2()),
                )
                .child(dynamic(move || {
                    crate::ui::segmented::segmented(
                        vec![
                            crate::ui::segmented::Segment {
                                id: "tree",
                                label: Box::new({
                                    let language = language;
                                    move || i18n::t(language.get().as_str(), "conflicts.tree")
                                }),
                            },
                            crate::ui::segmented::Segment {
                                id: "list",
                                label: Box::new({
                                    let language = language;
                                    move || i18n::t(language.get().as_str(), "conflicts.list")
                                }),
                            },
                        ],
                        if mode.get() == ConflictFilesMode::Tree {
                            "tree"
                        } else {
                            "list"
                        },
                        move |id| {
                            mode.set(if id == "list" {
                                ConflictFilesMode::List
                            } else {
                                ConflictFilesMode::Tree
                            })
                        },
                    )
                    .into_element()
                })),
        )
        .child(dynamic(move || -> AnyElement {
            if paths.is_empty() {
                return view()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .items_center()
                    .justify_center()
                    .child(
                        text(move || i18n::t(language.get().as_str(), "conflicts.noFiles"))
                            .text_size(px(crate::ui::SECONDARY))
                            .text_color(move || design::text3()),
                    )
                    .into_element();
            }

            let flat = mode.get() == ConflictFilesMode::List;
            let (items, expanded) = file_items(&paths, flat);
            let selected_id = selected
                .clone()
                .map(|path| SharedString::from(format!("file:{path}")));
            let unresolved_paths = unresolved.clone();
            let choose = on_select.clone();
            let tree = crate::ui::tree::tree_with_renderer(
                items,
                expanded,
                || None,
                selected_id,
                move |id| {
                    if let Some(path) = id.strip_prefix("file:") {
                        choose(path.to_string());
                    }
                },
                |_| {},
                move |entry| render_item(entry, &unresolved_paths, flat),
            );
            view()
                .w_full()
                .flex_1()
                .min_h_0()
                .child(tree)
                .into_element()
        }))
        .into_element()
}

fn file_items(paths: &[String], flat: bool) -> (Vec<crate::ui::tree::TreeItem>, Vec<SharedString>) {
    if flat {
        return (
            paths
                .iter()
                .map(|path| crate::ui::tree::node(format!("file:{path}"), path.clone(), Vec::new()))
                .collect(),
            Vec::new(),
        );
    }

    let mut root = Directory::default();
    for path in paths {
        let mut segments = path.split('/').collect::<Vec<_>>();
        if segments.pop().is_none() {
            continue;
        }
        let mut directory = &mut root;
        for segment in segments {
            directory = directory
                .directories
                .entry(segment.to_string())
                .or_default();
        }
        directory.files.push(path.clone());
    }
    let mut expanded = Vec::new();
    let items = directory_items(&root, "", &mut expanded);
    return (items, expanded);
}

fn directory_items(
    directory: &Directory,
    parent: &str,
    expanded: &mut Vec<SharedString>,
) -> Vec<crate::ui::tree::TreeItem> {
    let mut items = Vec::new();
    for (name, child) in &directory.directories {
        let path = if parent.is_empty() {
            name.clone()
        } else {
            format!("{parent}/{name}")
        };
        let id = format!("directory:{path}");
        expanded.push(SharedString::from(id.clone()));
        items.push(crate::ui::tree::node(
            id,
            name.clone(),
            directory_items(child, path.as_str(), expanded),
        ));
    }
    for path in &directory.files {
        let name = path.rsplit('/').next().unwrap_or(path.as_str());
        items.push(crate::ui::tree::node(
            format!("file:{path}"),
            name.to_string(),
            Vec::new(),
        ));
    }
    return items;
}

fn render_item(
    entry: &crate::ui::tree::TreeEntry,
    unresolved: &[String],
    flat: bool,
) -> AnyElement {
    let depth = entry.depth as f32;
    let is_folder = entry.folder;
    let path = entry.id.strip_prefix("file:").map(str::to_string);
    let is_unresolved = path
        .as_ref()
        .is_some_and(|path| unresolved.iter().any(|candidate| candidate == path));
    let label = if flat {
        path.clone().unwrap_or_else(|| entry.label.to_string())
    } else {
        entry.label.to_string()
    };
    let mut row = view()
        .w_full()
        .h(px(crate::ui::ROW))
        .flex_row()
        .items_center()
        .gap(px(crate::ui::GAP_XS))
        .pl(px(6.0 + depth * 12.0))
        .pr(px(8.0));
    if is_folder {
        row = row
            .child(crate::ui::icon::icon_view(
                "folder",
                crate::ui::icon::IconSize::Compact,
                design::text3,
            ))
            .child(
                text(label)
                    .flex_1()
                    .text_size(px(crate::ui::SECONDARY))
                    .text_color(move || design::text2()),
            );
    } else {
        row = row
            .child(crate::ui::icon::icon_view(
                "file",
                crate::ui::icon::IconSize::Compact,
                design::text3,
            ))
            .child(
                text(label)
                    .flex_1()
                    .min_w_0()
                    .text_size(px(crate::ui::CODE))
                    .text_color(move || design::text()),
            )
            .child(
                text(if is_unresolved { "U" } else { "R" })
                    .text_size(px(crate::ui::META))
                    .font_weight(FontWeight::BOLD)
                    .text_color(move || {
                        if is_unresolved {
                            design::danger()
                        } else {
                            design::ok()
                        }
                    }),
            );
    }
    return row.into_element();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_mode_groups_directories_and_preserves_file_paths_as_ids() {
        let paths = vec!["src/git/merge.rs".to_string(), "README.md".to_string()];
        let (items, expanded) = file_items(&paths, false);

        let source = items
            .iter()
            .find(|item| item.id.as_ref() == "directory:src");
        assert!(source.is_some());
        assert!(expanded.iter().any(|id| id.as_ref() == "directory:src/git"));
        let merge = source
            .and_then(|item| item.children.first())
            .and_then(|item| item.children.first())
            .expect("nested file entry");
        assert_eq!(merge.id.as_ref(), "file:src/git/merge.rs");
        assert_eq!(merge.label.as_ref(), "merge.rs");
    }

    #[test]
    fn list_mode_keeps_the_full_path_visible() {
        let paths = vec!["src/git/merge.rs".to_string()];
        let (items, expanded) = file_items(&paths, true);

        assert!(expanded.is_empty());
        assert_eq!(items[0].id.as_ref(), "file:src/git/merge.rs");
        assert_eq!(items[0].label.as_ref(), "src/git/merge.rs");
    }
}
