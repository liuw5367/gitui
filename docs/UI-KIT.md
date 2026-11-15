# UI-KIT — 应用的 ui 组件层

**Status**: v1, 2026-11-15
**范围**: `apps/client/src/ui/`（Rust + NSH 0.5.0，无 TSX/JS/HTML）
**刻度与偏离**: [DESIGN-HANDOFF.md](DESIGN-HANDOFF.md) 的「原生刻度」与「偏离清单」
**执行计划**: [../plans.local/2026-11-15-native-first-ui-alignment.md](../plans.local/2026-11-15-native-first-ui-alignment.md)

---

## 1. 为什么有这一层

NSH 把界面分成三层：`nsh-core` 提供原语与布局，`nsh-base` 提供**无外观**的行为组件（按压、键盘、焦点、无障碍状态），外观属于应用。`nsh_base::button` 的文档原话是 “a press target with no look of its own”，`switch`、`select`、`data_table` 同样是 “the look … is the app's”。NSH 0.5.0 没有 `NSButton`、`NSSegmentedControl`、`NSTableView`、`NSSearchField` 这些系统控件，也没有 SF Symbols 接口。

因此界面的一致性不能靠「使用系统控件」得到，只能靠应用自己的一层组件：行为来自 `nsh_base`，外观来自 `ui`，页面只组合 `ui`。这一层同时是唯一允许写颜色、字号、圆角、内边距的地方。

## 2. 使用规则

页面（`screens/`、`workspace/`、`shell/`）必须遵守：

1. 只组合 `crate::ui` 的组件与 `view()`/`text()`/`scroll()` 布局原语，不 `use nsh_base::`。
2. 不写 `text_size(px(<数字>))`：字号取 `ui::BODY`、`ui::SECONDARY`、`ui::META`、`ui::CODE`。
3. 不写十六进制或 `rgba()` 颜色：颜色只取自 `crate::design`。
4. 不写 `font_family(..)`：等宽用 `ui::code(..)` 或 `ui::MONO`。
5. 行级按压用 `ui::row`，按钮用 `ui::button`/`ui::icon_button`，不给裸 `view()` 挂 `.on_press`。
6. 图标按钮必须给无障碍名称：`ui::icon_button` 的 `label` 同时是可见提示与 `accessibility_label`。
7. 需要新外观时先在 `ui` 加变体或尺寸，再在页面使用；不在页面里覆盖组件的颜色与字号。

`ui` 层自身：字号取刻度常量、颜色取 `design`、不写十六进制。以上规则由 `tools/check-ui.sh` 与 `src/ui/mod.rs` 的 `guard_tests` 双向校验，`tools/ui-migrated.txt` 登记已迁移的页面文件，逐组追加。

## 3. 组件清单

| 组件 | 文件 | 基于 | 变体 / 尺寸 | 状态 | 说明 |
|---|---|---|---|---|---|
| `button` | `ui/button.rs` | `nsh_base::button` | `Primary` / `Secondary` / `Ghost` / `Danger`；`Small` 22 / `Regular` 28 / `Large` 32 | hover、pressed、focus-visible、disabled | 文字按钮；`label` 是闭包，语言切换后取新文案 |
| `icon_button` | `ui/button.rs` | `nsh_base::button` | 同上 | 同上 | 图标按钮；`label` 同时作为 `accessibility_label` 与提示文案 |
| `icon_view` | `ui/icon.rs` | `svg` | `Button` 12 / `Inline` 13 / `Toolbar` 16 | — | 设计稿 sprite；`inline_icon`、`toolbar_icon` 是默认色的简写；`brand_mark` 是唯一允许自定义点数的品牌图标 |
| `text_input` | `ui/text_input.rs` | `nsh_base::input` | 单行 28 | focus-visible、invalid | 值受控于 `Signal<String>`；`text_input_bound` 用于值在结构体信号里的字段，`search_field` 是带放大镜与键帽的搜索框 |
| `text_area` | `ui/text_input.rs` | `nsh_base::input(multiline)` | 自定义高度 | focus-visible、invalid | 提交说明、附注、冲突结果 |
| `field` | `ui/field.rs` | `view` + `text` | — | invalid | 标签 13 + 控件 + 帮助/错误 11；错误与帮助互斥 |
| `check_row` | `ui/check.rs` | `nsh_base::checkbox` | 行高 28 | checked、disabled | 整行是按压目标，14pt 指示器 |
| `switch_row` | `ui/check.rs` | `nsh_base::switch` | 38×22 轨道 | checked、disabled | 左侧文案 + 可选说明，右侧开关 |
| `segmented` | `ui/segmented.rs` | `nsh_base::tabs` + `tab` | 高 28 | 选中、hover、focus-visible | 设计稿 `.seg` 的对等物；选中段用卡片底色 |
| `table` | `ui/table.rs` | `nsh_base::data_table` | 行高 28、表头 28 | 行选中、键盘导航、虚拟化 | 列定义 + 列式行数据 + 自定义单元格回调；`DataTableColumn`/`DataTableSelection` 是 non_exhaustive，先用 `default()` 再逐字段赋值；列与行的映射是纯函数，有单测 |
| `tabs` | `ui/tabs.rs` | `nsh_base::tabs` + `tab` | 行高 28 | 选中、focus-visible | 面板内互斥视图（检查器三页签、同步三页签）；选中项底部 2pt 强调线 |
| `badge` / `count` / `kbd` | `ui/badge.rs` | `view` + `text` | `Tone`：Neutral / Accent / Success / Warning / Danger | — | 徽章 18 高；键帽 18 高 |
| `menu` | `ui/menu.rs` | 系统 `NSMenu` | `action` / `checkable` / `danger` / `divider` | 启用、勾选 | 只提供菜单项构造，不重画外观（系统菜单决定外观）；危险项的区分靠「选择后弹确认」而非颜色 |
| `key_value` | `ui/key_value.rs` | `view` + `text` | 行高 24，标签列 96 | — | 左标签 11pt + 右值 13pt；等宽值（SHA、路径）由 `mono` 标记走 12pt `ui::MONO` |
| `pane_header` / `section_label` / `empty` / `skeleton_row` | `ui/section.rs` | `view` + `text` | 行高 28 / 24 | — | 面板标题条、分组标题、空态、骨架行 |
| `banner` / `progress` / `card` | `ui/feedback.rs` | `nsh_base::progress` | `BannerTone`：Info / Warning / Danger | 确定与不确定进度 | 状态条收敛为一条横幅，颜色之外必有文案 |
| `dialog` | `ui/dialog.rs` | `view` + 遮罩层 | 宽度由调用方给定，最大高 760 | — | 页头 44 + 正文 16 内边距 + 页脚 48；点遮罩关闭 |
| `row` | `ui/row.rs` | `nsh_base::button` | 单行 28 / 双行 44 | 选中、hover、focus-visible、disabled | 列表行；前后槽放图标与徽章，副标题可指定等宽 |
| `gallery` | `ui/gallery.rs` | 上述全部 | — | — | 组件画廊：全部变体与状态并排显示 |

刻度常量集中在 `design/metrics.rs` 的 `native` 模块，`ui` 里的常量是它的别名，页面不直接引用数字。

## 4. 状态与无障碍

| 状态 | 表现 | 来源 |
|---|---|---|
| hover | 背景换 `design::hover()`（选中行不改） | `view().hover` |
| pressed | 透明度 0.8–0.9 或 `line_soft` 底色 | `view().active` |
| focus-visible | 2pt `focus` 色轮廓（`outline`，不改布局） | `view().focus_visible` |
| selected | `design::selected()` 底色 + `sel-strong` 文字 | 组件参数 |
| disabled | 透明度 0.45，且组件本身不再接收按压 | `nsh_base` 的 `disabled` |
| invalid | 描边换危险色 + 一行 11pt 错误文案 | 组件参数 + `ui::field::error` |

无障碍由 `nsh_base` 提供 role、选中与禁用状态；图标按钮由 `ui::icon_button` 传 `accessibility_label`。表格、树、页签、命令面板的 role 在各自组件落地时由对应 `nsh_base` 组件提供。

## 4.5 签名与选用规则

调用方按这张表选组件；同一用途出现第二种写法即为缺陷。

### 文字

| 用途 | 用 | 不要用 |
|---|---|---|
| 正文、标签、行标题 | `body(\|\| String)` | `text(..).text_size(px(13))` |
| 说明、帮助、错误 | `secondary(..)` / `field::help` / `field::error` | 手写 11pt 灰字 |
| 计数、时间、元数据 | `meta(..)` / `badge::count(..)` | 手写 9.5/10/10.5pt |
| 代码、diff、路径 | `code(..)` | `text(..).font_family("SF Mono")` |

### 动作

| 用途 | 用 | 签名要点 |
|---|---|---|
| 主/次/幽灵/危险按钮 | `button::button(label, variant, size, disabled, on_press)` | `label` 是闭包，语言切换后重取 |
| 纯图标按钮 | `button::icon_button(name, label, size, disabled, on_press)` | `label` 同时是无障碍名称，不可省 |
| 行级点击 | `row::row(caption, subtitle, leading, trailing, selected, disabled, on_press)` | 替代裸 `view().on_press`；单行 28 / 双行 44 |
| 页签切换 | `tabs::tabs(items, selected_id, on_select)` | 面板内互斥视图 |
| 分段选择 | `segmented::segmented(items, selected_id, on_select)` | 表单里的 2–4 选一 |

### 输入

| 用途 | 用 | 说明 |
|---|---|---|
| 值在 `Signal<String>` 里 | `text_input::text_input(value, placeholder, invalid, on_change)` | 最常见 |
| 值在结构体信号里 | `text_input::text_input_bound(get, placeholder, invalid, set)` | 提交说明的 `branch`、`depth_custom` 这类 |
| 多行 | `text_input::text_area(value, placeholder, height, invalid, on_change)` | 说明、附注、冲突结果 |
| 搜索 | `text_input::search_field(value, placeholder, shortcut, on_change)` | 自带放大镜与键帽 |
| 带标签的字段 | `field::field(caption, control, help, error)` | 错误与帮助互斥，别两个都传 |

### 结构

| 用途 | 用 |
|---|---|
| 面板标题条（标题 + 计数 + 右侧动作） | `section::pane_header(caption, counter, actions)` |
| 分组标题 | `section::section_label(caption)` |
| 空态 | `section::empty(caption, description, action)` |
| 加载骨架 | `section::skeleton_row(&[宽, 宽, ..])` |
| 横幅（信息/警告/危险） | `feedback::banner(tone, message, actions)` |
| 进度（确定/不确定） | `feedback::progress(Some(0.4))` / `progress(None)` |
| 卡片容器 | `feedback::card(child)` |
| 模态框 | `dialog::dialog(open, width, title, body, footer, on_dismiss)` |
| 表格 | `table::table(TableSpec { .. })` |

### 常被选错的三处

1. **`tabs` 还是 `segmented`**：一屏内切换主视图用 `tabs`（有下划线指示）；表单里选一个值用 `segmented`（有底色块）。
2. **`row` 还是 `button`**：整行可点用 `row`（含选中态与前后槽）；带边框的独立按钮用 `button`。
3. **`field::field` 的 help 与 error**：两者都传会被 error 覆盖，只传一个；需要「有错时也不隐藏帮助」是新的需求，得先改组件。

## 5. 增改流程

1. 先在 `ui/gallery.rs` 加该组件的全部变体与状态示例。
2. 再在 `ui` 内实现（字号取常量、颜色取 `design`）。
3. 运行 `cargo test --lib ui::` 与 `tools/check-ui.sh`。
4. 然后才在页面使用；页面若需要新的外观，回到第 1 步加变体，不在页面覆盖。

## 6. 尚未进入本层的组件

以下组件属于后续阶段，落地时同样只在此表登记：

| 组件 | 落地阶段 | 对应 nsh_base |
|---|---|---|
| ~~`table`~~ 基座已就绪（待接入） | P2 | `nsh_base::data_table` |
| ~~`tabs`~~ 已完成 | P2 | `nsh_base::tabs` + `tab` |
| `toolbar`（工作区工具栏，高 52） | P2 | `toolbar` + `toolbar_group` |
| `split`（检查器、冲突三栏） | P2 / P4 | `h_resizable` / `v_resizable` |
| `diff_surface`（diff 与 fold/wrap 开关） | P3 | `diff` |
| `command`（命令面板） | P6 | `command` |
| `menu`（上下文菜单与下拉） | P6 | `menu` / `context_menu` |
| `tree`（设置侧栏、文件树） | P7 | `tree` |
| `tooltip`（图标按钮提示） | P2 | `tooltip` |
