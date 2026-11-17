//! 树：可展开、可选中、键盘可达的层级列表。用于设置侧栏与文件树。
//!
//! 行为来自 `nsh_base::tree`（展开/折叠、方向键、Home/End、typeahead、选中语义、
//! 无障碍 role 全由它提供）。本层只做映射：把「带缩进的扁平节点表」转成
//! `TreeItem` 树，并把行高、字号、选中色统一。
//!
//! 用法：
//! ```ignore
//! ui::tree::tree(
//!     vec![
//!         ui::tree::node("general", "General", vec![]),
//!         ui::tree::node("git", "Git", vec![ui::tree::node("git.tools", "Tools", vec![])]),
//!     ],
//!     Some(selected_id),
//!     move |id| select(id),
//! )
//! ```

use crate::design;
use crate::ui;
use nsh_base::{SelectValue, TreeItem};
use nsh_core::prelude::*;

/// 一个节点。`children` 为空时是不可展开的叶子。
pub(crate) fn node(
    id: &'static str,
    label: impl Into<SharedString>,
    children: Vec<TreeItem>,
) -> TreeItem {
    // `TreeItem` 是 non_exhaustive：只能 default 后逐字段赋值。
    let mut item = TreeItem::default();
    item.id = SharedString::from(id);
    item.label = label.into();
    item.children = children;
    return item;
}

/// 一棵树。`selected` 是当前选中节点的 id；`on_select` 收到被选中的 id，
/// 由调用方对照自己的 id 表决定做什么（ui 层不认识业务 id）。
pub(crate) fn tree(
    items: Vec<TreeItem>,
    selected: Option<SharedString>,
    on_select: impl Fn(&str) + 'static,
) -> impl IntoElement {
    let on_select = std::rc::Rc::new(on_select);
    return nsh_base::tree()
        .items(items)
        .w_full()
        .h_full()
        .selected(SelectValue::from(selected))
        .item_style(move |style| {
            style
                .h(px(ui::ROW))
                .px(px(ui::GAP))
                .rounded(px(6.0))
                .text_color(move || design::text())
        })
        .on_selected_change(move |id: &SharedString| on_select(id.as_ref()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_ids_are_kept_verbatim() {
        // id 是选中与展开的键，必须原样保存：调用方靠它识别节点。
        let leaf = node("git.tools", "Tools", vec![]);
        assert_eq!(leaf.id.as_ref(), "git.tools");
        assert_eq!(leaf.label.as_ref(), "Tools");
        assert!(leaf.children.is_empty());
    }

    #[test]
    fn a_node_with_children_is_a_folder() {
        let folder = node("git", "Git", vec![node("git.tools", "Tools", vec![])]);
        assert_eq!(folder.children.len(), 1);
    }
}
