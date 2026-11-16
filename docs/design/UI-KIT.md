# UI-KIT — 组件层

`apps/client/src/ui/` 是应用自己的外观层：行为、键盘、焦点与无障碍来自 `nsh_base`，
外观只在这里定义。页面（`features/`、`app/`、`shell/`）组合这一层的组件，不自己
写颜色、字号、圆角。

刻度与色值见 [TOKENS.md](TOKENS.md)；这些组件的视觉规则见
[DESIGN-SYSTEM.md](DESIGN-SYSTEM.md)。

## 1. 为什么需要这一层

NSH 把界面拆成三层：`nsh-core` 给原语与布局，`nsh-base` 给**无外观**的行为组件
（`button` 的文档原话是 “a press target with no look of its own”），外观属于应用。
NSH 0.5.0 没有 `NSButton`、`NSSegmentedControl`、`NSTableView`、`NSSearchField`，
也没有 SF Symbols 接口。

所以一致性不可能靠「调用系统控件」得到，只能靠这一层：它同时是唯一允许写字号、颜色、
圆角、内边距的地方。

## 2. 页面必须遵守的规则

1. 只组合 `crate::ui` 的组件与 `view()` / `text()` / `scroll()` 这类布局原语，不
   `use nsh_base::`。
2. 不写 `text_size(px(<数字>))`：字号取 `ui::BODY` / `SECONDARY` / `META` / `CODE`。
3. 不写十六进制或 `rgba()` 颜色：颜色只取自 `crate::design`。
4. 不写 `font_family("…")`：等宽用 `ui::code(..)` 或 `ui::MONO`。
5. 行级按压用 `ui::row`，按钮用 `ui::button` / `ui::icon_button`，不给裸 `view()`
   挂 `.on_press`。
6. 图标按钮必须给名称：`ui::icon_button` 的 `label` 同时是可见提示与
   `accessibility_label`。
7. 需要新外观时先在 `ui` 里加变体或尺寸档，再在页面使用；不在页面覆盖组件的颜色与字号。

规则由两处机械校验，口径相同：

- [tools/check-ui.sh](../../tools/check-ui.sh)：A 段查 `src/ui` 自身（字号字面量、十六进制），
  B 段按 [tools/ui-migrated.txt](../../tools/ui-migrated.txt) 查已迁移页面（`nsh_base` 引用、
  裸 `on_press`、字号、色板、字体族）。
- `src/ui/mod.rs` 的 `guard_tests`：同样的检查跑在 `cargo test` 里，`MIGRATED` 常量与
  `ui-migrated.txt` 是同一份名单。

名单覆盖 `features/`、`app/`、`shared/`、`shell/` 下的全部页面文件（原 `screens/` 与
`workspace/` 已在重构中并入 `features/` 与 `app/`）。新增页面时两处名单一起加。

## 3. 组件清单

| 组件 | 文件 | 行为来源 | 变体 / 尺寸 | 状态 |
|---|---|---|---|---|
| `body` / `secondary` / `meta` / `code` | `ui/mod.rs` | `text` | 13 / 11 / 11 / 12 | — |
| `separator` / `v_separator` | `ui/mod.rs` | `view` | 横 1×∞ / 竖 1×16 | — |
| `button` | `ui/button.rs` | `nsh_base::button` | `Primary` / `Secondary` / `Ghost` / `Danger` × `Small` 22 / `Toolbar` 26 / `Regular` 28 / `Large` 32 | hover、pressed、focus、disabled |
| `button_icon` | `ui/button.rs` | 同上 | 同 `button` | 同上 |
| `icon_button` / `icon_button_tuned` | `ui/button.rs` | 同上 | 同上 | 同上；`label` 必填 |
| `icon_view` | `ui/icon.rs` | `svg` | `Compact` 11 / `Button` 12 / `Inline` 13 / `Toolbar` 16 | — |
| `text_input` / `text_area` / `text_input_bound` / `search_field` | `ui/text_input.rs` | `nsh_base::input` | 单行 28；多行高度自定 | focus、invalid |
| `field` / `label` / `help` / `error` | `ui/field.rs` | `view` + `text` | 标签 13 + 控件 + 说明/错误 11 | invalid |
| `check_row` / `check_inline` / `check_group` | `ui/check.rs` | `nsh_base::checkbox` | 行 28；指示器 14（inline 12–14） | checked、disabled |
| `switch_row` | `ui/check.rs` | `nsh_base::switch` | 轨道 38×22 | checked、disabled |
| `indicator` / `indicator_action` | `ui/check.rs` | `view` / `nsh_base::button` | 12×12 | checked |
| `segmented` | `ui/segmented.rs` | `nsh_base::tabs` + `tab` | 高 28，圆角 6 | 选中（底色 + MEDIUM）、hover、focus |
| `tabs` | `ui/tabs.rs` | `nsh_base::tabs` + `tab` | 行 28，选中底部 2pt | 选中（字重 + 下划线）、focus |
| `table` | `ui/table.rs` | `nsh_base::data_table` | 行 28、表头 28 | 行选中、键盘、虚拟化 |
| `tree` / `node` | `ui/tree.rs` | `nsh_base::tree` | 行 28 | 展开、选中、禁用 |
| `row` | `ui/row.rs` | `nsh_base::button` | 单行 28 / 双行 44 | selected、hover、focus、disabled |
| `badge` / `count` / `kbd` | `ui/badge.rs` | `view` + `text` | 高 18；`Tone` 五档 | — |
| `setting_row` / `read_only_row` | `ui/setting_row.rs` | `view` + `text` | 最小高 36 | 有/无说明、有/无分隔线 |
| `pane_header` / `section_label` / `empty` / `skeleton_row` | `ui/section.rs` | `view` + `text` | 28 / 24 | — |
| `banner` / `banner_sized` / `progress` / `card` | `ui/feedback.rs` | `nsh_base::progress` | `BannerTone` 六档：Info / Warning / Danger / Accent / Neutral / Highlight | 确定与不确定进度 |
| `dialog` / `scrim` / `modality` / `confirm_destructive` / `notice` | `ui/dialog.rs` | `view` + 遮罩 | 页头 44、页脚 48、正文内距 16，最大高 760 | — |
| `diff_surface` | `ui/diff_surface.rs` | `nsh_base::DiffView` | 分栏 / 统一；软换行、折叠 | hunk 动作回调 |
| `split` / `Pane` | `ui/split.rs` | `h_resizable` / `v_resizable` | 横向 / 纵向；`fixed` / `flex`，`.min()` / `.max()` | 拖拽、把手焦点 |
| `menu` / `menu_button` | `ui/menu.rs` | 系统 `NSMenu` | `action` / `checkable` / `danger` / `divider` / `section` | 启用、勾选 |
| `key_value` / `key_values` | `ui/key_value.rs` | `view` + `text` | 行 24，标签列 96 | — |
| `gallery` | `ui/gallery.rs` | 上述全部 | — | 调试入口 `⌘⇧D` |

## 4. 选用规则

同一用途出现第二种写法即为缺陷。

### 文字

| 用途 | 用 | 不要用 |
|---|---|---|
| 正文、标签、行标题 | `ui::body(..)` | `text(..).text_size(px(13))` |
| 说明、帮助、错误 | `ui::secondary(..)` / `ui::field::help` / `ui::field::error` | 手写 11pt 灰字 |
| 计数、时间、元数据 | `ui::meta(..)` / `ui::badge::count(..)` | 手写 9.5 / 10 / 10.5pt |
| 代码、diff、路径、SHA | `ui::code(..)` | `text(..).font_family("SF Mono")` |

### 动作

| 用途 | 用 |
|---|---|
| 主 / 次 / 幽灵 / 危险按钮 | `ui::button::button(label, variant, size, disabled, on_press)` |
| 纯图标按钮 | `ui::button::icon_button(name, label, size, disabled, on_press)` |
| 行级点击 | `ui::row::row(caption, subtitle, leading, trailing, selected, disabled, on_press)` |
| 面板内切视图 | `ui::tabs::tabs(items, selected, on_select)` |
| 表单里 2–4 选一 | `ui::segmented::segmented(items, selected, on_select)` |

### 输入

| 用途 | 用 |
|---|---|
| 值在 `Signal<String>` 里 | `ui::text_input::text_input(value, placeholder, invalid, on_change)` |
| 值在结构体信号里 | `ui::text_input::text_input_bound(get, placeholder, invalid, set)` |
| 多行 | `ui::text_input::text_area(value, placeholder, height, invalid, on_change)` |
| 搜索 | `ui::text_input::search_field(value, placeholder, shortcut, on_change)` |
| 带标签的字段 | `ui::field::field(caption, control, help, error)` |

### 结构

| 用途 | 用 |
|---|---|
| 面板标题条（标题 + 计数 + 右侧动作） | `ui::section::pane_header(..)` |
| 分组标题 | `ui::section::section_label(..)` |
| 空态 | `ui::section::empty(..)` |
| 加载骨架 | `ui::section::skeleton_row(&[..])` |
| 横幅 / 进度 / 卡片 | `ui::feedback::banner(..)` / `progress(Some(x))` / `card(..)` |
| 模态框 | `ui::dialog::dialog(..)`；确认类用 `confirm_destructive` / `notice` |
| 表格 | `ui::table::table(TableSpec { .. })` |
| 可拖拽分栏 | `ui::split::horizontal(vec![Pane::fixed(..).min(..), Pane::flex(..)])` |

### 常选错的三处

1. **`tabs` 还是 `segmented`**：一屏内切换主视图用 `tabs`（底部强调线）；表单里选一个值
   用 `segmented`（选中段底色）。
2. **`row` 还是 `button`**：整行可点、有选中态用 `row`；带边框的独立按钮用 `button`。
3. **`field` 的 help 与 error**：两者互斥，都传会被 error 覆盖。需要「有错时仍显示帮助」
   得先改组件。

## 5. 状态与无障碍

| 状态 | 表现 |
|---|---|
| hover | 底色换 `design::hover()`；Ghost 按钮用透明度 0.9 |
| pressed | 实心按钮透明度 0.8；描边与 Ghost 按钮换 `line-soft` 底色 |
| selected | `design::selected()` 底色；页签与分段同时加字重 |
| focus-visible | 2pt `focus` 轮廓（`outline`，不改布局） |
| disabled | 透明度 0.45，且不再接收按压 |
| invalid | 描边换危险色 + 一行 11pt 错误文案 |

无障碍 role、选中与禁用状态来自 `nsh_base`；图标按钮由 `icon_button` 传
`accessibility_label`；表格、树、页签的 role 由各自的 `nsh_base` 组件提供。

**已知缺陷（待修）**：`ui/row.rs:59` 在选中时把文字色设成 `sel-strong`（浅色 `#D5E6FF`），
而底色是 `sel`（`#E7F0FF`），对比度约 1.1:1，选中行的标题实际不可读；深色模式同样
（`#1B3D60` on `#15324F`）。设置页侧栏（`features/settings/view.rs` 的 `row(.., selected, ..)`）
会走到这条路径。正确做法是底色 `sel` + 文字 `text`。`sel-strong` 全库只在这一处被引用，
改成 `text` 不会牵连别处。

## 6. 增改流程

1. 在 `ui/gallery.rs` 加上新组件的全部变体与状态示例。
2. 在 `ui` 内实现：字号取常量、颜色取 `design`、尺寸取 `metrics`。
3. 跑 `(cd apps/client && cargo test)` 与 `tools/check-ui.sh`。
4. 才在页面使用；页面需要新外观时回到第 1 步加变体，不在页面覆盖。
5. 新页面迁移到组件层时，`tools/ui-migrated.txt` 与 `ui/mod.rs` 的 `MIGRATED` 一起加。

## 7. 还不在这一层里

- `widgets::icon`：仍接受任意点数，实际有 3 处例外（9 行内折叠箭头、15 设置页分支、
  20 错误横幅），其余图标已走 `ui::icon_view` 的四档。合并 `widgets.rs` 后可收敛。
- 原先在名单外的 `palette` 与 `diff` 已随重构并入 `features/search/` 与 `shared/`，
  现在 21 个页面文件全部在名单里。
