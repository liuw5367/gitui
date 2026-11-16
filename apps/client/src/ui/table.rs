//! 表格：`nsh_base::data_table` 提供虚拟化、行选择、列宽与键盘导航，这里给出
//! 应用统一的列定义、行数据与外观。
//!
//! 行的数据按列 `key` 逐列传入（`DataTableRows::texts`），不是行对象；单元格需要
//! 自绘时由 `render` 回调接管（图线、引用徽章、+/− 统计）。

use crate::design;
use crate::ui;
use nsh_base::data_table::{
    ColumnFixed, DataTableAlign, DataTableCell, DataTableColumn, DataTableContextMenu,
    DataTablePress, DataTableRows, DataTableSelection,
};
use nsh_core::prelude::*;

/// 一列。
pub(crate) struct Column {
    /// 数据键，与 `rows` 的列一一对应。
    pub(crate) key: &'static str,
    /// 表头文案；`None` 表示没有表头（例如图线列）。
    pub(crate) title: Option<&'static str>,
    /// 固定宽度；`None` 表示按内容自适应。
    pub(crate) width: Option<f32>,
    /// 是否固定在左侧（跨横向滚动保持可见）。
    pub(crate) fixed: bool,
}

/// 一张表。
pub(crate) struct TableSpec {
    pub(crate) columns: Vec<Column>,
    /// 每行的文本，顺序与 `columns` 一致。
    pub(crate) rows: Vec<Vec<String>>,
    /// 选中行（0 起）。
    pub(crate) selected: Option<usize>,
    /// 行高，默认 28。
    pub(crate) row_height: Option<f32>,
    /// 行被选中时的回调。
    pub(crate) on_select: Box<dyn Fn(usize)>,
    /// 自定义单元格：`(行, 列键, 文本)` 返回一个元素。
    ///
    /// 返回的元素**可以含交互子元素**（按钮、勾选框等）：`data_table` 的
    /// `press_with` 会先把按压交给该单元格视图，它若吞掉（返回 taken）就不再
    /// 执行行选中。所以行内「暂存 / 丢弃」这类次级按钮是安全的，不会连带选中行。
    /// 注意按钮要小（12–14pt）并留间距，避免与行按压误触。
    pub(crate) render: Option<Box<dyn Fn(u32, &str, String) -> AnyElement>>,
    /// 行被按压（含修饰键）：`(行, ⌘ 是否按下)`。给了它就由调用方决定选中还是
    /// 加入对比，而不是走默认的「点击即选中」。
    pub(crate) on_row_press: Option<Box<dyn Fn(u32, bool)>>,
    /// 行被右键：`(行, x, y)`。坐标是根视图坐标，可直接给 `popup_menu_at`。
    pub(crate) on_context_menu: Option<Box<dyn Fn(u32, f32, f32)>>,
}

/// 一张虚拟化表格。
pub(crate) fn table(spec: TableSpec) -> impl IntoElement {
    let columns = columns_of(&spec.columns);
    let data = rows_of(&spec.columns, &spec.rows);
    let render = spec.render;
    let row_press = spec.on_row_press.map(std::rc::Rc::new);
    let context_menu = spec.on_context_menu.map(std::rc::Rc::new);
    let on_select = std::rc::Rc::new(spec.on_select);
    return nsh_base::data_table()
        .columns(columns)
        .data(data)
        .row_height(Points(spec.row_height.unwrap_or(ui::ROW)))
        .header_height(Some(Points(ui::ROW)))
        .row_selectable(true)
        .cell_selectable(false)
        .column_selectable(false)
        .resizable_columns(false)
        .movable_columns(false)
        .stripe(false)
        .scrollbar(true)
        .font_size(Points(ui::CODE))
        .selection(spec.selected.map(|row| {
            let mut selection = DataTableSelection::default();
            selection.row = Some(row as u32);
            selection
        }))
        .render_cell(move |cell: &DataTableCell| -> AnyElement {
            if let Some(render) = &render {
                return render(cell.row, cell.key.as_ref(), cell.text.to_string());
            }
            return text(cell.text.clone())
                .text_size(px(ui::CODE))
                .line_height(px(ui::CODE_LINE))
                .text_color(move || design::text())
                .into_element();
        })
        .select_on_modifier_press(false)
        .on_row_press(move |press: &DataTablePress| {
            if let Some(handler) = &row_press {
                // 有 ⌘/Shift 时不走默认选中，交给调用方（历史表的对比集合）。
                handler(press.row, press.platform);
            }
        })
        .on_context_menu(move |event: &DataTableContextMenu| {
            if let (Some(handler), Some(row)) = (&context_menu, event.row) {
                handler(row, event.x.0, event.y.0);
            }
        })
        .on_selection_change(move |selection: &Option<DataTableSelection>| {
            if let Some(selection) = selection
                && let Some(row) = selection.row
            {
                on_select(row as usize);
            }
        });
}

/// 把列定义转成 `data_table` 的列。
///
/// `DataTableColumn` 是 `#[non_exhaustive]`，只能先 `default()` 再逐字段赋值。
fn columns_of(spec: &[Column]) -> Vec<DataTableColumn> {
    let mut columns = Vec::with_capacity(spec.len());
    for column in spec {
        let mut built = DataTableColumn::default();
        built.key = SharedString::from(column.key);
        built.title = column.title.map(SharedString::from);
        built.width = column.width.map(Points);
        built.fixed = if column.fixed {
            ColumnFixed::Left
        } else {
            ColumnFixed::None
        };
        built.resizable = Some(false);
        built.movable = Some(false);
        built.align = DataTableAlign::Start;
        columns.push(built);
    }
    return columns;
}

/// 把行数据按列键喂给 `data_table`；缺的格子补空串，多出的列忽略。
fn rows_of(columns: &[Column], rows: &[Vec<String>]) -> DataTableRows {
    let mut data = DataTableRows::new(rows.len());
    for (index, column) in columns.iter().enumerate() {
        let values: Vec<String> = rows
            .iter()
            .map(|row| row.get(index).cloned().unwrap_or_default())
            .collect();
        data = data.texts(column.key, values);
    }
    return data;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(key: &'static str, width: Option<f32>, fixed: bool) -> Column {
        return Column {
            key,
            title: Some(key),
            width,
            fixed,
        };
    }

    #[test]
    fn columns_keep_order_widths_and_fixed_side() {
        let columns = columns_of(&[
            column("graph", Some(64.0), true),
            column("summary", None, false),
        ]);
        assert_eq!(columns.len(), 2);
        assert_eq!(columns[0].key.as_ref(), "graph");
        assert_eq!(columns[0].width, Some(Points(64.0)));
        assert_eq!(columns[0].fixed, ColumnFixed::Left);
        assert_eq!(columns[1].key.as_ref(), "summary");
        assert_eq!(columns[1].width, None);
        assert_eq!(columns[1].fixed, ColumnFixed::None);
    }

    #[test]
    fn spec_defaults_have_no_row_press_or_context_menu() {
        // 不传这两个回调时，表格走默认的「点击即选中」，不额外拦截按压。
        let spec = TableSpec {
            columns: vec![column("a", None, false)],
            rows: vec![vec!["1".to_string()]],
            selected: None,
            row_height: None,
            on_select: Box::new(|_row| {}),
            render: None,
            on_row_press: None,
            on_context_menu: None,
        };
        assert!(spec.on_row_press.is_none());
        assert!(spec.on_context_menu.is_none());
    }

    #[test]
    fn rows_map_by_column_index_and_tolerate_short_rows() {
        let columns = [column("a", None, false), column("b", None, false)];
        let rows = vec![
            vec!["1".to_string(), "2".to_string()],
            vec!["3".to_string()],
        ];
        let data = rows_of(&columns, &rows);
        // 行数取自行数据；短行补空串、不 panic、不越界。
        assert_eq!(data.len(), 2);
    }
}
