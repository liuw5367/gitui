# DESIGN-HANDOFF — canvas → NSH/AppKit

**Status**: v1, 2026-11-14  
**Stack**: NSH + Rust only; UI built with NSH's Rust API and AppKit-backed controls.  
**No** TSX, JS, HTML, or CSS in production. The canvas is a visual/state reference.  
**Source canvas**: `designs/graft-git/project/` (30 boards)  
**UI spec**: [UI-SPEC.md](UI-SPEC.md)

---

## 1. Principles

- **Native first**: 行为来自 NSH 的原生元素与 AppKit 后端（输入、菜单、告警、滚动、窗口）；外观由 `ui` 层按系统刻度实现；字体为系统字体；图标用 SVG sprite（见 §4）。原生菜单、sheet、焦点与 VoiceOver 语义优先。
- **Dense but legible**: Git client density is intentional; do not inflate rows to “web app” sizes.
- **Hierarchy through weight/colour**, not uppercase micro-labels or boxes everywhere.
- **One accent**: system blue; no second primary blue; destructive is a separate semantic colour.
- **State is design**: loading/empty/error/disabled/hook failure/operation are part of the screen, not afterthoughts.
- **Dark and light are equal**: no hard-coded light-only colours; test both.

---

## 2. Tokens

### 2.1 Colour

| Token | Light | Dark |
|---|---|---|
| Accent | `#007AFF` | `#0A84FF` |
| Primary button | accent | accent |
| Text | `#1D1D1F` | `#F5F5F7` |
| Secondary | `#5F636B` | `#A1A1A6` |
| Tertiary | `#86868B` | `#8E8E93` |
| Window/content | `#FFFFFF` | `#1E1E20` |
| Sidebar | `#F4F4F6` (material) | `#232325` |
| Panel | `#F7F7F8` | `#262628` |
| Line | `rgba(0,0,0,.10)` | `rgba(255,255,255,.10)` |
| Add | `#1F8A4C` / `#30D158` | same semantic |
| Delete | `#C7362F` / `#FF453A` | same semantic |
| Warning | `#B26A00` / `#FFD60A` | same semantic |
| Danger | `#C7362F` / `#FF453A` | same semantic |

- Accent is the **only** primary blue. `--primary-bg` = accent.
- Selection: source lists use accent fill + white text; tables use accent-soft + text for non-focused selection.
- Status is never colour-only: icon/text + colour.

### 2.2 Type

| Role | Size | Weight | Notes |
|---|---|---|---|
| Body / labels | 13 | regular | `ui::BODY`，行高 18 |
| Secondary | 11 | regular | `ui::SECONDARY`，帮助与说明，行高 15 |
| Metadata / table header | 11 | medium | `ui::META`，sentence case |
| Section header | 11 | medium | `ui::section_label` |
| Code / diff | 12 | regular | `ui::CODE`，SF Mono，行高 18 |
| Button | 11（Small）/ 13（Regular、Large） | medium | 高度 22 / 28 / 32 |
| Badge | 11 | medium | 高度 18，不用全大写（除已知缩写） |

- Font: system UI font; code SF Mono. No custom display font.
- 字号以系统刻度为准，不再采用设计稿的 8.5–10.5px（见 §2.4 与 §12）。界面中不存在低于 11pt 的用户可见文字。
- Line height: 正文 18/13，代码 18/12。

### 2.3 Spacing / shape

| Token | Value |
|---|---|
| Base unit | 4 |
| Gaps | 4 / 6 / 8 / 12 / 16 |
| Row height (tables/lists) | 28；紧凑行 24（菜单项、快捷键行） |
| Sidebar row | 28 |
| Button height | 22 small / 28 regular / 32 large |
| Field height | 28 |
| Segmented height | 28 |
| Menu item | 24 |
| Toolbar | 52 |
| Titlebar | 44（自绘仓库页签；系统窗口按钮区不重绘） |
| Status bar | 24 |
| Radius | 6 controls / 10 panels / 12 window |
| Hairline | 1 px (0.5 pt on Retina) |

- No gratuitous pills/cards; use separators and spacing.
- Sheets: top-only rounded corners, attached to window; not free-floating shadow cards.
- Menus: NSMenu; 6–7 px radius; 24 px items; vibrancy.

---

### 2.4 原生刻度

字号、行高、控件高度与区域高度以 macOS 系统刻度为准，设计 CSS 只决定结构与层级；颜色仍取自设计稿的变量（`theme.json`）。常量集中在 `apps/client/src/design/metrics.rs` 的 `native` 模块，`ui` 层的常量是它的别名。

| 项 | 值 |
|---|---|
| 正文 / 行高 | 13 / 18 |
| 次要文字 / 行高 | 11 / 15 |
| 元数据、表头 | 11，500 字重 |
| 代码 / 行高 | 12 / 18 |
| 控件高 | 22 / 28 / 32 |
| 行高 | 28；紧凑 24 |
| 工具栏 | 52 |
| 标题栏 | 44 |
| 状态栏 | 24 |
| 侧栏 | 232 |
| 圆角 | 6 / 10 / 12 |
| 图标 | 12 / 13 / 16 |

### 2.5 颜色

颜色只来自 `theme.json`（设计 CSS 的 `:root` 与 `.theme-dark`）。语义角色（正文、次要、分隔、选中、焦点、控件底与描边）在 `design/mod.rs` 里以函数暴露，`ui` 层只调用这些函数；页面不写颜色。系统材质（`macos_backdrop`）本版未使用，侧栏与工具栏用设计稿的底色，记入 §12 偏离清单。

## 3. Control mapping

| Canvas | Production |
|---|---|
| `.btn` / `.btn.primary` | `NSButton` (bezel styles); accent for primary; no gloss |
| `.btn.small` | `NSButton` small/regular 22 |
| `.chipbtn` | `NSButton` inline / borderless with background |
| `.seg` | `NSSegmentedControl` |
| `.field` | `NSSearchField` / `NSTextField` |
| `.checkrow` | `NSButton` checkbox / `NSSwitch` where appropriate |
| `.radio-pills` | `NSSegmentedControl` or radio group |
| `.side-row` | `NSOutlineView` row, source-list style |
| `.commit` / `.refs-row` / `.fhist-row` | `NSTableView` rows |
| `.diff` | `NSTextView` (read-only) in `NSScrollView` |
| `.commit-input` / description | `NSTextView` / `NSTextField` |
| `.hook-log` | `NSTextView` read-only, monospace |
| `.menu` / `.mi` | `NSMenu` / `NSMenuItem` |
| `.sync-sheet` / merge sheet | NSH sheet / `NSAlert` where appropriate |
| `.titlebar` tabs | native window tabs or NSH tab strip (product choice) |
| `.toolbar` | `NSToolbar` / NSH toolbar |
| `.statusbar` | NSH status view |
| traffic lights | system standard window buttons |

- Do not recreate AppKit controls with custom drawing unless the design requires a diff/graph/editor.
- 图标用设计稿的 SVG sprite；`ui::icon_view` 统一三档尺寸。

---

## 4. Icons

- 图标使用设计稿的 SVG sprite（`apps/client/src/design/icons.rs`），尺寸只有三档：按钮内 12、行内 13、工具栏与页头 16，线宽 1.5。
- NSH 0.5.0 没有 SF Symbols 的接口（只有 iOS 导航栏的 header symbol），因此不采用 SF Symbols。图标的语义命名对齐 SF Symbols，便于将来替换。
- 每个图标按钮必须有可见提示与无障碍名称；`ui::icon_button` 的 `label` 同时承担两者。
- 不新增第四档尺寸；需要更小的字形时改图形本身，不改字号。

---

## 5. Screen handoff

### S1 Repository + History

- Sidebar source list (vibrancy), 28 px rows; selected accent fill.
- Commit table 28–30 px; graph column fixed; message truncates with tooltip.
- Diff: 11–12 px SF Mono; line numbers tertiary; add/delete backgrounds subtle; no rainbow.
- Filter bar: search field + branch/date/author chips; “Clear”.
- Empty/loading/error states per UI-SPEC.

### S2 Changes + Commit + Hooks

- Staged/Unstaged sections with counts; checkboxes/rows; hunk action affordance.
- Composer: 300–340 px inspector; summary 13 semibold; description 12; options 12.
- Hook panel: red-tinted but restrained; status + steps + log; Retry primary-ish; bypass danger.
- Commit button 28–30 px; disabled state clear; no “or Commit & Push” when hooks failed.

### S3 Sync

- Sheet 800 px wide; tabs Fetch/Pull/Push; target row; options; auth; primary + Cancel.
- Progress/log right side; background state after close.
- Force options behind warning; no default force.

### S4 Merge

- Base strip + Ours / Result / Theirs; Result editable; line numbers; conflict markers highlighted.
- Continue disabled until 0 conflicts; Abort always available; Skip only rebase.
- Hook failure panel can appear without losing Result.

### S5 Settings

- Preferences window; sidebar: Application (General, Shortcuts), Git (Git, Credentials), About.
- Labels 13 / help 11; controls 22–28; Keychain notes; no secrets displayed.
- Identity local vs global; Credentials does not override user.name/email.

### S6 Refs

- Tabs; table 28–30; inspector 300–340; actions confirm destructive.
- Branch/tag/remote/stash details; no mixed tables without headers.

---

## 6. Dark mode

- Use semantic tokens; no hard-coded `#fff`/`#000` in views.
- Diff add/delete have sufficient contrast in dark; line numbers tertiary.
- Accent #0A84FF; selected source list white text.
- Menus/sheets use material; borders visible but subtle.
- Test: hooks red, conflict red, warnings yellow, disabled controls, focus rings.

---

## 7. Localization

- English default; Simplified Chinese complete.
- Keys for every string; no sentence concatenation.
- Buttons/rows tolerate +30% length; truncate with tooltip rather than clip.
- Dates: relative/absolute/system/custom; no hard-coded “today” strings.
- Plurals: “1 file / 2 files” via plural rules; Chinese no plural suffix.
- Keyboard shortcuts use localized key names but physical keys.

---

## 8. Accessibility

- Every control keyboard reachable; focus ring visible; tab order visual.
- Tables/outlines expose rows, selection, sort; diff exposes line numbers/content.
- Hook status announced; errors have accessible descriptions; not colour-only.
- Contrast ≥ 4.5:1 text; 3:1 large; check both themes.
- Reduced motion: spinners/progress can be static; no essential animation.
- VoiceOver labels for tabs, icon buttons, branch rows, status pills.

---

## 9. Motion

- 120–180 ms for hover/selection; no bounce.
- Progress: indeterminate spinner only when no percentage; otherwise bar + text.
- Sheets slide from top; no full-screen fades.
- No animation for diff rendering/hook logs that could hide state.

---

## 10. Board map

| Group | Boards |
|---|---|
| Start | 01 Welcome, 02 empty, 03 Repo manager |
| Review | 04 History, 05 filters, 06 multi-commit, 07 compare, 08 commit info, 09 refs |
| Worktree | 10 Changes/commit, 11 changelists, 12 Agent, 13 permissions |
| Operations | 14 merge, 15 sync, 16 states, 17 reflog |
| Files | 18 files at commit, 19 blame/history |
| Navigation | 20 palette, 21 dark/context, 22 menus/dialogs |
| Settings | 23 general, 24 Git, 25 shortcuts, 26 about, 27 credentials |
| Details | 28 fetch, 29 pull, 30 refs other |

- 22 is a component sheet; 16 is a state gallery; 28/29 are real sync sheets; 30 is a refs screen.
- No new boards planned; implement screens, not more pictures.

---

## 11. Handoff checklist

- [ ] Every screen has loading/empty/error/disabled states.
- [ ] Every action maps to a Rust service command and a use case.
- [ ] Buttons 22/24; rows 28–30; no 8 px labels.
- [ ] 图标只用三档尺寸（**未达成**：`widgets::icon` 仍用 9/10/11/12/13/14/15/20 八种，见 §12 第 6 条）；图标按钮有可见提示与无障碍名称。
- [x] 图标名必须存在于 sprite（`ui/icon.rs` 的 `every_icon_literal_exists_in_the_sprite` 扫描全部 `src/`，写错名字会让测试失败；`icon_view` 另有 runtime `debug_assert`）。
- [ ] Light/dark checked; contrast/focus checked.
- [ ] English/zh-Hans strings; +30% length checked.
- [ ] Destructive confirmations; no silent force/discard.
- [ ] Hook panel shows status, log, retry, bypass confirmation.
- [ ] No TSX/JS/HTML in production.

---

## 12. 偏离清单

设计稿与实现之间的每一处可见差异登记在这里，逐组补充；没有登记的差异视为缺陷。列含义：画板值与实现值、原因、能否回退。

| # | 位置 | 画板值 | 实现值 | 原因 | 可回退 |
|---|---|---|---|---|---|
| 1 | 全库字号 | 8.5–10.5pt | 11–13pt（§2.4） | 设计稿字号低于系统刻度，11pt 是可读下限 | 是（改 `native` 常量） |
| 2 | 行高 | 侧栏 25、提交行 28、菜单 24 | 28、28、24 | 系统列表行高 | 是 |
| 3 | 控件高 | 按钮 24/22、字段 26、分段 26 | 28/22、28、28 | 系统控件高度 | 是 |
| 4 | 工具栏 | 44（`.toolbar.v2`） | 52 | 系统工具栏高度 | 是 |
| 5 | 圆角 | 控件 6、面板 8、窗口 10 | 6、10、12 | 系统外观 | 是 |
| 6 | 图标 | 内联 SVG sprite，多档尺寸 | 同一 sprite；`ui::icon_view` 三档 12/13/16，但 `widgets::icon` 仍接受任意点数，实际用了 9/10/11/12/13/14/15/20 八种 | nsh 0.5.0 无 SF Symbols 接口；`widgets.rs` 尚未并入 `ui`，尺寸未统一 | 部分（合并 `widgets.rs` 后可收敛到三档） |
| 7 | 侧栏与工具栏材质 | 设计稿为纯色 | 纯色，未用 `macos_backdrop` | 保持与设计稿一致的底色；材质留到需要时再评估 | 是 |
| 8 | 欢迎页克隆表单 | HTTPS / SSH 分段 | 未放置 | 协议由 URL 决定，原控件没有后端含义，保留即为假控件 | 是（需要时接 URL 重写） |
| 9 | 欢迎页最近仓库行 | 47 | 44 | 双行列表行高（28 + 16） | 是 |
| 10 | 欢迎页分组标题 | 23 | 24 | 紧凑行高 | 是 |
| 11 | 变更列表的目录标题 | 独立分组标题，不在列表内 | 目录标题是表格的一行（方案 A） | `data_table` 没有「非数据行」概念，只有 `row_selectable` 整表开关，无法让某行不可选 | 是（拍平列表则失去「按目录分组」设置，属功能退化，故不采用） |
| 12 | 变更列表键盘导航 | 目录标题不参与导航 | 上下键会经过目录标题行 | 同 11：无法逐行禁用选中 | 同 11 |
| 13 | 变更清单 | 弹层（dialog） | 锚定式 popover（300 宽、贴按钮、无遮罩） | 它由「变更清单」按钮触发并贴在其下方，不是模态；套 `ui::dialog` 会变居中遮罩，语义相反 | 是（若改为模态交互） |
| 14 | 历史表与变更表的列宽 | 按内容分配多列 | 单列 1200pt，整行在 `render` 里自绘 | 图线、引用徽章、对比底色、状态方框都要自绘，`data_table` 的多列模型装不下 | 是（改为多列则需重画这些元素） |
| 15 | 命令面板遮罩 | 全屏遮罩 | 遮罩与面板同层，面板顶部对齐（`mt -120`） | 面板要贴在窗口上方而非居中；`ui::dialog::scrim` 会居中内容 | 是（若接受居中） |
| 16 | 侧栏宽 | 240 | 232 | 系统侧栏宽度（`native` 刻度） | 是 |
| 17 | 状态栏高 | 22 | 24 | 系统状态栏高度 | 是 |
| 18 | 工具按钮的图标与文案间距 | 画板 `.tbtn` 内 `gap` 未固定 | 5pt | 与 `ui` 其它带图标按钮统一 | 是 |
| 19 | 分块暂存按钮与行内开关的尺寸 | 画板为可点方框，尺寸随行 | 12×12（`ui::check::indicator_action`） | 表格单元格里可用空间有限，且要与 `indicator` 一致 | 是 |

| 20 | 主按钮 | 画板是 `height:30px` 的 `.btn.primary` | `ui::button(Primary, Regular)` 28 高、`--primary-bg` 底 | 原实现是 24 高 + `--accent-2`，与画板的 30 和 ui 的 28 都不符；统一到 `ui::button` 后向画板靠近 2pt，且只保留一个主按钮实现（`widgets::primary_button` 已删） | 是（若要严格 30，需给 `ButtonSize` 再加一档） |
| 21 | 图标尺寸档位 | 画板 11pt 用于侧栏与状态横幅 | `IconSize` 已补 `Compact`(11)，12/13/16 不变 | 11pt 有 3 处真实使用者（`branch`/`file`/侧栏动态图标），且侧栏行比按钮密，用 12 会偏大 | 否（已对齐设计稿） |
| 22 | 图标尺寸的实际分布 | 四档 11/12/13/16 | 仍有 9/10/15/20 共 7 处非档位尺寸 | 都是小指示图标（勾、折叠箭头、文件夹）或特例（20 是错误横幅图标）；映射到最近档有 1–2pt 变化，属可见改动 | 是（需确认是否接受这 1–2pt） |

### 板 14 接 `ui::split` 为什么不是「换个组件」

原记录写「换 `ui::split`」，像是把三个面板套进一个组件。核对后不是：

- 冲突页的布局是**两层**：外层 `flex_row` 只有两个 child（左 224 固定列表 +
  右侧整体），右侧内部的 `flex_row` 才是三个等宽面板（Ours / Result / Theirs）；
- `ui::split::horizontal` 要一次收齐**同一层**的所有面板（`Vec<Pane>`），所以接入
  必须把「左栏 + 右三栏」扁平化到同一个 `horizontal`，拆掉中间那层 `flex_row`
  与 `flex_1`；
- 三个面板各自还带标题栏、滚动区、底部动作，搬进 `Pane::flex(..)` 时它们的
  内部结构不变，但外层的 `.child(...)` 层级要重排。

因此这不是小改动，是约 150 行的结构重排，且无法分小步做（三面板共用容器，换就是
三个一起换）。要做得留一整轮的预算，并且按「先列元素清单→逐个搬→编译看 unused
警告→四项门禁」的流程走。

### `widgets.rs` 为什么不能批量并入 `ui`

逐函数核实后的结论：`widgets.rs` 与 `ui/` 的差异不是「重复实现」，
而是**两个不同的尺寸模型**——前者几乎都接受可变点数，后者只认固定档位：

| `widgets` 函数 | 实际传入 | `ui` 对应 | 能否无损替换 |
|---|---|---|---|
| `icon(name, size, color)` | 9/10/11/12/13/15/20 | `icon_view(.., IconSize, ..)`（12/13/16） | 只有 12、13 那 10 处可以；其余 15 处不行 |
| `check(label, checked, size, on_change)` | 12、14 | `check_row`（整行 28 高、指示器 14） | 否：两处都是 22 高行内的复选，换过去会改行高 |
| `icon_button(name, size, color, on_press)` | 12/13/14，且是 26×24 | `icon_button`（正方形，边长 = 档位） | 否：会改尺寸与比例 |
| `primary_button(label, on_press)` | 高 24、`--accent-2` | `button(Primary)`（22/28/32、`--primary-bg`） | 否：高度与底色都不同 |
| `small_button(label, on_press)` | 高 22、字号 11 | `button(Secondary, Small)` | **是**（已迁移，见 history 的 4 处） |
| `button` | 见调用点 | `button` | 需逐处核对 |
| `kbd` | — | 无 | `ui` 层没有键帽 |

所以「97 处调用」不能当一次机械替换完成。可行路径只有两条：

1. **给 `ui` 补档位**（11 的图标、24 的按钮、行内复选），把 `widgets` 的功能搬过来，
   再逐处换调用；
2. **接受并存**：`widgets` 保留为「自由尺寸」的低层，`ui` 是「档位化」的高层，
   在文档里写清分工。

本轮没有把这两条中的任何一条单方面定下来——它影响 90 处调用与全库的图标/控件尺寸。

### 记录但不算偏离

以下是为实现画板而新增、画板本身未规定的概念，登记在此避免被误认为偏离：

- **`ButtonSize::Toolbar`（26pt）**：画板用 `.tbtn` 类表达工具栏项，但没有给出与其它按钮的关系；实现里单独设一档，因为 22/28/32 都不匹配。
- **`ui::check::indicator_action`**：画板的暂存方框是「可点」，而 `ui` 原有的 `indicator` 不可点；在表格单元格里需要「点了只切换、不选中行」的语义，故单独提供。
- **`RowSpec`（`d|<目录>` / `f|<索引>`）**：`data_table` 每格只有一个字符串，目录标题与文件行共处一表时用它区分。
