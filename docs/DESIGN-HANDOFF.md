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
- [ ] 图标只用三档尺寸；图标按钮有可见提示与无障碍名称。
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
| 6 | 图标 | 内联 SVG sprite，多档尺寸 | 同一 sprite，三档 12/13/16 | nsh 0.5.0 无 SF Symbols 接口 | 否（需自绘字形） |
| 7 | 侧栏与工具栏材质 | 设计稿为纯色 | 纯色，未用 `macos_backdrop` | 保持与设计稿一致的底色；材质留到需要时再评估 | 是 |
| 8 | 欢迎页克隆表单 | HTTPS / SSH 分段 | 未放置 | 协议由 URL 决定，原控件没有后端含义，保留即为假控件 | 是（需要时接 URL 重写） |
| 9 | 欢迎页最近仓库行 | 47 | 44 | 双行列表行高（28 + 16） | 是 |
| 10 | 欢迎页分组标题 | 23 | 24 | 紧凑行高 | 是 |
