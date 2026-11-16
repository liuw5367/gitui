# 深色模式与右键菜单 — 布局、状态与按钮

- **画板**：21 Dark & context menu（有深色主题 · 有 15 项提交菜单压在 diff 上；无子菜单、无禁用项、无浅色对照）
- **来源**：`designs/graft-git/project/ContextMenu.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

本页是**主窗口**，只是把外层 `.desktop` 换成 `theme-dark`：骨架与 04 相同（1392×852、标题栏 40、工具条 44、侧栏 232、检查器 360、状态栏 24），主体仍是提交 diff。菜单绝对定位在 `.main` 内 `top:72` / `left:300`，宽 260、高 405。

## 1. 骨架

```
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ ● ● ●  [gitui x] [docs-site x] [atlas-web x] [+]     ● Fetched 2m ago  [gear]                │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ [=] Fetch Pull Push | Branch Stash   [search ⌘F] | History Changes 12 | Agent [=]            │
├────────────────┬──────────────────────────────────────────────────┬──────────────────────────┤
│ Branches 5     │ [< Commits] Fix diff hunk alignment              │ 7 files changed +162 -62 │
│ ▣ main ✓       │ 7 files                                          │ src/git                  │
│ ▣ feature/     │            ┌──────────────────────────────────┐  │   diff-engine.ts M +18 -7│
│ Remotes 2      │            │ Open Diff                        │  │   hunk.ts A +44 -0       │
│   origin 8     │            │ Edit Commit Message…             │  │ src/ui                   │
│ Tags 3         │            │ Checkout Revision                │  │   DiffView.tsx M +31 -18 │
│ ▣ v1.4.0       │            │ New Branch from Here…            │  │ Commit details           │
│ Stashes 2      │            │ New Tag…                         │  │ Parents 8b41e07          │
│ Worktrees 3    │            │──────────────────────────────────│  │ Author Mia Chen          │
│                │            │ Cherry-Pick Commit               │  │ [ Open in Editor ] [c][.]│
│                │            │ Revert Commit…                   │  │                          │
│                │            │ Squash Commits…                  │  │                          │
│                │            │ Rebase onto Here…                │  │                          │
│                │            │ Merge into Current…              │  │                          │
│                │            │ Reset Current Branch to Here  ►  │  │                          │
│                │            │──────────────────────────────────│  │                          │
│                │            │ Compare with Working Tree        │  │                          │
│                │            │ Copy SHA                         │  │                          │
│                │            │ Copy Commit Message              │  │                          │
│                │            │──────────────────────────────────│  │                          │
│                │            │ Open in Browser                  │  │                          │
│                │            └──────────────────────────────────┘  │                          │
├────────────────┴──────────────────────────────────────────────────┴──────────────────────────┤
│ main  ● 2 ahead · 0 behind        12 changes  UTF-8  LF  TypeScript                          │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 窗口 | 整页 | 1392×852；标题栏 40、工具条 44、状态栏 24 | 与 04 同构，全部取深色 token |
| 中列 | 主体 | 宽 798；标题条 34 + diffbar 38 + diffbody | 提交标题、`Split/Unified`、折叠/换行/空白、`2 of 7`、上/下一处、split diff |
| 菜单 | 中列内 `top:72` `left:300` | 宽 260；高 405；内距 5；圆角 7 | 4 组 15 项，3 条分隔 |
| 检查器 | 主体右 | 宽 360 | 与 04 相同，`Changes` 当前、`7 files changed +162 −62` |

菜单上沿对齐 diffbar 下沿（34 + 38 = 72），所以它贴着工具栏往下展开；高 405 = 2 边框 + 2×5 内距 + 15×24 项 + 3×(1 + 上下 5)；窗口内左沿 533 = 1 + 232 + 300。

## 2. 深色取色

`theme-dark` 只覆盖 token，不另写组件样式。本页用到的差值与浅色对照：

| token | 浅色 | 深色 |
|---|---|---|
| `content` / `panel` / `panel-2` | `#FFFFFF` / `#F7F7F9` / `#F1F1F4` | `#1B1B1D` / `#1F1F21` / `#262628` |
| `line` / `line-soft` / `line-strong` | `#E2E2E6` / `#ECECEF` / `#D2D2D8` | `#343438` / `#2C2C30` / `#414147` |
| `text` / `text2` / `text3` | `#1D1D1F` / `#5D5D65` / `#6E6E76` | `#F2F2F4` / `#B3B3BB` / `#8E8E96` |
| `menu-bg` / `menu-line` | `#FCFCFD` / `#E5E5EA` | `#2C2C2E` / `#3D3D42` |
| `shadow-menu` | 0 12 34（.22） | 0 14 40（.55）+ 0 1 4（.4） |

`accent` 与 `primary-bg` 在深色下都取 `#0A84FF`（浅色是 `#007AFF`）。深色下 `--sel` 变成 `#15324F`，所以状态里被选中的提交行、`sel` 的文件行都偏蓝；菜单 hover 用的是 `primary-bg`（`#0A84FF`）反白，比面板亮得多。

## 3. 菜单规格

容器 `.menu`：`position:absolute`、z 30、宽 260（样式表末次覆盖 252）、内距 5、底 `menu-bg`、1px `menu-line`、圆角 7、`--shadow-menu`、26px 背景模糊。

菜单项 `.mi`：24 高、内距 0×8、圆角 5、gap 6、字号 12.5（末次覆盖）；hover 换 `primary-bg` 底 + 白字；快捷键 `.sc` 10 号 `text3`（hover 转 `rgba(255,255,255,.78)`）右对齐；子菜单箭头 `.chev` 10 号也右对齐。禁用态 `.disabled` 定义为 `text3` + .55 透明，本板没有用到。

分隔 `.msep`：1px `menu-line`，上下各 5、左右 7。

## 4. 菜单内容

四组，从上到下：

1. `Open Diff`（`↵`）· `Edit Commit Message…` · `Checkout Revision` · `New Branch from Here…`（`⌘B`）· `New Tag…`
2. `Cherry-Pick Commit` · `Revert Commit…` · `Squash Commits…`（`⌥⌘S`）· `Rebase onto Here…` · `Merge into Current…` · `Reset Current Branch to Here…`（`►`）
3. `Compare with Working Tree` · `Copy SHA`（`⌥⌘C`）· `Copy Commit Message` · `Open in Browser`（`⇧⌘O`）

`Open Diff` 是唯一带 `↵` 的项，也是唯一带 `href` 的项（回 04）；`Reset Current Branch to Here…` 是唯一有子菜单箭头的项。

## 5. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 主题 | 浅色 / 深色 | 只画深色 |
| 菜单 | 开 / 关 | 只画开 |
| 项状态 | 可用 / 禁用 | 只画可用 |
| 子菜单 | 无 / 有（仅箭头） | 只画箭头，面板没画 |
| 触发目标 | 提交行 / diff 行 / 空白 | 没有画 |

```
打开：右键（触发位置没有画）；菜单位置 top 72 = 标题条 34 + diffbar 38
切换：hover 反白；子菜单项应向右侧展开，没有画
关闭：点空白或 esc；两种都没有画
```

## 6. 画板没覆盖的组合（待补设计）

1. **子菜单**：`Reset Current Branch to Here…` 只画了 `►`，展开后的 `Soft / Mixed / Hard` 三选没有；22 的 Reset 对话框是另一个入口，两者关系没写。
2. **禁用态**：样式表有 `.mi.disabled`，板上 15 项全可点；「当前分支不能 checkout」「没有 remote 不能 push」这类规则没有示例。
3. **键盘与首项高亮**：打开时哪一项被聚焦、方向键怎么移动、`↵` 与 hover 如何分工，都没画。
4. **浅色版本**：全仓库只有这一块深色板，同一菜单在浅色下的 `sel` 表现无法对照。
5. **触发对象**：菜单位于 diff 区域上方，看不出是提交行、diff 行还是空白处的右键；`aria-label="Commit actions"` 只说明了语义。
6. **14 项的去向**：只有 `Open Diff` 是链接，其余都没有目标；22 里画好的 Edit message / Reset / Squash / New Branch / New Tag 五个对话框没有被菜单引用。
7. **边界与背板**：`top:72` / `left:300` 是硬编码，靠近窗口右下时是否翻转、是否限制在窗口内没有写；菜单没有遮罩，点外部关闭也没有画。

## 7. 按钮与链接清单

15 个菜单项逐行列出；底层被菜单压住的 04 按钮不计入。

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 菜单第 1 组 | `Open Diff` + `↵` | `mi` 24 高 | 回 04 看该提交 | 唯一 `href`（`Workspace.dc.html`） |
| 2 | 菜单第 1 组 | `Edit Commit Message…` | `mi` 24 高 | 改提交信息（22 有对应对话框） | 无目标 |
| 3 | 菜单第 1 组 | `Checkout Revision` | `mi` 24 高 | 检出该提交 | 无目标 |
| 4 | 菜单第 1 组 | `New Branch from Here…` + `⌘B` | `mi` 24 高 | 建分支（22 有对话框） | 无目标 |
| 5 | 菜单第 1 组 | `New Tag…` | `mi` 24 高 | 建标签（22 有对话框） | 无目标 |
| 6 | 菜单第 2 组 | `Cherry-Pick Commit` | `mi` 24 高 | 摘下该提交 | 无目标 |
| 7 | 菜单第 2 组 | `Revert Commit…` | `mi` 24 高 | 反向提交 | 无目标 |
| 8 | 菜单第 2 组 | `Squash Commits…` + `⌥⌘S` | `mi` 24 高 | 压缩提交（22 有对话框） | 无目标 |
| 9 | 菜单第 2 组 | `Rebase onto Here…` | `mi` 24 高 | 以该提交为基点变基 | 无目标 |
| 10 | 菜单第 2 组 | `Merge into Current…` | `mi` 24 高 | 合并进当前分支 | 无目标 |
| 11 | 菜单第 2 组 | `Reset Current Branch to Here…` + `►` | `mi` 24 高 + 子菜单箭头 | 重置当前分支 | 子菜单没画 |
| 12 | 菜单第 3 组 | `Compare with Working Tree` | `mi` 24 高 | 与工作区比较 | 无目标 |
| 13 | 菜单第 3 组 | `Copy SHA` + `⌥⌘C` | `mi` 24 高 | 复制 sha | 无目标 |
| 14 | 菜单第 3 组 | `Copy Commit Message` | `mi` 24 高 | 复制提交信息 | 无目标 |
| 15 | 菜单第 4 组 | `Open in Browser` + `⇧⌘O` | `mi` 24 高 | 在浏览器打开该提交 | 无目标 |

- `role="menu"` / `role="menuitem"`，分隔是 `role="separator"`；15 项都没有 `aria-keyshortcuts`。
- 底层窗口的提交行、文件行、工具条按钮仍在 DOM 里，但被菜单与浮层语义遮住，不计入本页。

## 8. 文案要点

- 8 项带 `…`（`Edit Commit Message…` `New Branch from Here…` `New Tag…` `Revert Commit…` `Squash Commits…` `Rebase onto Here…` `Merge into Current…` `Reset Current Branch to Here…`）表示会先开对话框；其余 7 项直接执行。
- 菜单项一律用动词开头，`Checkout Revision` 用 `Revision` 而不是 `Commit`，与 `Open Diff` 的提交语境区分。
- 只有 4 项带快捷键（`⌘B` `⌥⌘S` `⌥⌘C` `⇧⌘O`）；子菜单项把 `…` 与箭头并用，同时表示「会开对话框」与「还有一层」。

## 9. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 04 Commits | `Open Diff` 与底层 `Commits` chip 共 2 处 `href`；本板就是它的提交视图 |
| 22 Menus & dialogs | 菜单里 5 项在 22 有对应对话框（Edit message / Reset / Squash / New Branch / New Tag），但两板之间没有 `href` |
| 10 Staging & commit | `Changes 12` 分段项链到它 |
| 23 Settings / 01 Welcome | 齿轮 / 页签 `+` |
| 14 Merge conflict / 15 Sync & auth | 同为主窗口浮层，但那两块有遮罩（.34 / .28），本板菜单没有 |
