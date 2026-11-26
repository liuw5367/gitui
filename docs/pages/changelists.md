# 变更列表 — 布局、状态与按钮

- **画板**：11 Changelists（三个变更集 · 浮层展开 · 提交按钮可用；无空工作区、无新建表单、无多选）
- **来源**：`designs/graft-git/project/Changelists.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

板内 `.window` 没有内联尺寸，取样式表的 `height:calc(100vh - 48px)`：预览 1440×900、桌面留白 24，窗口 **1392×852**。本页是**主窗口内换主体**（工具条 `Changes 12` 为当前项），与 10 同一副骨架但没有合并横幅：标题栏 40 / 工具条 44 / 三栏主体 **742** / 状态栏 24，742 = 852 − 2 − 40 − 44 − 24；中列 diff 区 708 = 742 − 34，diff 主体 670 = 708 − 38。

当前运行界面的底部提示为 `Drag files to move them to another list`，明确说明拖动会移动文件；Manage 按钮保留在最右侧且显示为 `Manage`（中文 `管理`）。交互入口说明见[暂存与提交](changes.md)。

## 1. 骨架

```
┌────────────────┬──────────────────────────────────────────────────┬───────────────────────────────────┐
│ ● ● ●  [gitui ×] [docs-site ×] [atlas-web ×] [+]     ● Fetched 2m ago   [gear]                        │
├───────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ [=] | Fetch Pull Push | Branch Stash   [search ⌘F] | History [Changes 12] | Agent [=]                 │
├───────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ Branches 5     │ < Commits [Commit composer] CommitPanel.tsx      │ Commit 12                  [more] │
│ ● main ✓ ↑2    │                              12 files · +149 -90 │ Changelists 12 files  [+][more]   │
│ ● feature/diff │ [Commit composer] > CommitPanel.tsx  +42 -17     │ v ● Commit composer 3             │
│ Remotes 2      │ [Split] Unified [fold][wrap][ws] 1 of 3          │ ☑ CommitPanel.tsx  M +42 -17      │
│   origin 8     │ a/ src/ui/CommitPanel.tsx                        │ ☑ AgentPanel.tsx   M +18  -6      │
│ Tags 3         │ b/ src/ui/CommitPanel.tsx                        │ ☑ commit.ts        M  +9  -2      │
│ ▣ v1.4.0       │ @@ -20,9 +20,11 @@   [Unstage Hunk]              │ v · Default Changelist 6          │
│ Stashes 2      │  20 const staged = … | 20 const staged = …       │   ☐ tokens.css     M +14  -8      │
│ Submodules 2   │  21 canCommit=true  | (empty)                    │ 3 more files…                     │
│ Worktrees 3    │ (empty)             | 21 !!message.trim()        │ > · Agent cleanup 3               │
│                │  22 return (        | 22 return (                │ [changelist popover]              │
│                │ [v] 9 unchanged lines                            │ Commit to main                    │
│                │                                                  │             3 staged · 9 unstaged │
│                │                                                  │ Summary  Polish composer…         │
│                │                                                  │ Description  Disable…             │
│                │                                                  │ ☑ Sign off                        │
│                │                                                  │ Commit 3 Staged Files (enabled)   │
│                │                                                  │ or Commit & Push                  │
├───────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ main   ● 2 ahead · 0 behind        12 changes   UTF-8   LF   TypeScript                               │
└────────────────┴──────────────────────────────────────────────────┴───────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 返回条 | 中列顶部 | 高 34（内联）；内距 0×11；间距 8 | `< Commits`、`Commit composer` 徽章、文件名、`12 files · +149 −90` |
| diff 工具条 | 返回条下 | 高 38；内距 0×11；间距 8 | 同一枚徽章 + 路径 + 增删、Split/Unified、三个开关、`1 of 3`、一个前进箭头 |
| diff 主体 | 工具条下 | 剩 670 高；等宽 11、行高 19 | 两栏文件头、1 个 hunk、13 行代码、1 行折叠 |
| 侧栏 | 主体左 | 宽 232；内距 2/8/12 | 与 04 同一套六组引用 |
| 提交面板 | 主体右 | 宽 360；页签 33；表单内距 12 | `Commit 12` / `Agent` 页签、变更集列表、Changelists 浮层、提交表单 |
| 状态栏 | 底部通栏 | 高 24；内距 0×12；11 | `main`、`● 3 staged · 9 unstaged`、`12 files · +149 −90`、编码、换行、语言 |

中列 798 = 1390 − 232 − 360。图中 `[changelist popover]` 是 §3 的浮层，`[+]` 是 `New changelist`，`[more]` 是行级 `…`，`(enabled)` 表示主按钮可用；`Alt-M` 与 `-` 分别代板上的 `⌥M` 与 `—`，框内只放单宽字符。

## 2. 中列的 diff 区

与 10 逐像素相同，只有两处换字、少一个按钮。返回条与 crumb 的徽章都写 `Commit composer`（10 写 `Staged`），所以同一句话出现两次；徽章本身 18 高、圆角 4、内距 0×6、`panel-2` 底、1px `line-soft`、9.5 `text3`。工具条右簇仍是 `Split | Unified` → 1×18 竖线 → 三个 24×24 图标 → 竖线 → `1 of 3` → 一个 `Next change`。主体是同一条四格表：hunk 行 21 高、右端 mini `Unstage Hunk`、折叠行 20 高写 `9 unchanged lines`；**本板没有 `Unstage line`**——10 在新增行末尾多一个按行反暂存按钮，这里没有。

## 3. Changelists 段与浮层

- **头部** 26 高、`panel-2` 底、下边框 1、内距 0×8、间距 6、10/600：11 的分支图标 → `Changelists` → `12 files`（10/500，`margin-left:auto` 贴右）→ 两个图标按钮（各 12 的 `+` 与 `…`）：`New changelist`、`Changelist options`。**没有**树/平铺开关也**没有**漏斗——10 的头部有这两样，这里换成「新建 + 选项」。
- **列表** 限高 252、自身滚动。组头 23 高、内距 0×7、上距 3、间距 6、10.5/600：箭头 + 7×7 圆点（活动组 `accent`，其余 `text3`）+ 组名（10/600 `text`）+ 计数 10.5/600 + 空隙 + `…`。三组：`Commit composer 3` 展开（圆点 `accent`，3 个已暂存文件全勾选）；`Default Changelist 6` 展开（组头上方另加 2px `accent` 边框，圆点 `text3`；先 `tokens.css` 的 0.55 透明 + `outline:1px dashed accent` 投放态，再 `router.ts`、`commit.test.ts`，后接 `3 more files…`——22 高、左内距 20、9.5）；`Agent cleanup 3` **折叠**（箭头朝右，无文件行，计数照留）。
- **文件行** 28 高（内联覆盖 27）、内距 0×7、圆角 5、间距 6：复选框 13 → 文件名 mono 10.5 → 状态字母 11 宽（9.5/700，`M` `warn`、`A` `ok`）→ 右端 `+42 −17`（mono 9.5）→ `…`（18×18）。当前行 `CommitPanel.tsx` 是 `sel` 底。分隔条 9 高（四个 2×2 圆点、`role=separator`、`cursor:row-resize`）与 24 高的 `Drag files onto another changelist to move them` 都与 10 相同。

```
┌──────────────────────────────────────────┐
│ Changelists                        + New │
│ (o) Commit composer        3 files   …   │
│ ( ) Default Changelist     6 files   …   │
│ ( ) Agent cleanup          3 files   …   │
│──────────────────────────────────────────│
│ Move 3 selected files to…          Alt-M │
│ Rename Changelist…                       │
│──────────────────────────────────────────│
│ Filters only - staging commits           │
│ [Set Active]                             │
└──────────────────────────────────────────┘
```

- **浮层** 宽 **288**（内联覆盖 `.cl-pop` 的 320）、`top:34px`、`left:8px`，圆角 9、1px `line-strong`、内距 5、`menu-bg` 底、菜单阴影，`role="dialog"`、`aria-label="Changelists"`；靠 `staged-box` 的 `overflow:visible` 露在外面。
- 头 30 高、内距 0×7、11/600：`Changelists` → 空隙 → `New`（10、`accent-2`、500、内嵌 10 的加号）。
- 三个变更集行 28 高、内距 0×8、间距 7、11.5，整行是 `<a href="Changes.dc.html">`：12 的圆形 radio（选中项 `accent` 实心 + `inset 0 0 0 2px menu-bg` 留白圈）→ 名称（选中行 600）→ 右端 `3 files` / `6 files`（9.5）→ `…`。
- 两条命令：`Move 3 selected files to…`（右端 `⌥M`）与 `Rename Changelist…`，都是整行 28 高的 `button`。页脚上边框 1、上距 4、内距 6×7：`Filters only — staging commits`（9 `text3`）+ `Set Active`（小按钮 22、`primary`）。`Set Active` 与三个 radio 行说的是同一件事，画板把「选中」和「确认」拆成两步。

## 4. 提交面板的其余部分

与 10 相同的部分：`Commit to main`（`3 staged · 9 unstaged`）、`✓ 3 staged files across all changelists`、Summary（29 高、`Required`）、Description（`rows=3`、`min-height:76`、`Markdown`）、`Only staged files are committed / ⌘↵ to commit`、`agent-nudge` + `Ask Agent`、三个复选（`Sign off` 勾选）、末尾 `Changelists are filters; staging commits.` + `Manage…`。页签同为 33 高：`Commit 12`（当前）+ `Agent`（`<a>` 链 12）+ 空隙 + `Commit options`；本板计数徽章没有内联 `padding`，用样式表的 `0 4px`。

三处不同：**没有钩子块**（10 的 `Hooks` 失败卡在本板不存在，表单因此短一截）；**主按钮可用**（高 30、`primary`、无 `disabled`，按钮下是 `or Commit & Push` 的 `#` 链接，不是 10 的 `Hooks must pass before committing`）；**`Manage…` 回指 10**（本板是变更集的集中管理入口，说明句仍把它链回 10）。

## 5. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 变更集数量 | 1 / 多 | 只画 3 个 |
| 组展开 | 展开 / 折叠 | 画展开 2 个 + 折叠 1 个 |
| 浮层 | 关 / 开 | 只画开（常驻在板上） |
| 选中行 | 任一文件 / 无 | 只画 `CommitPanel.tsx`（`sel`） |
| diff 布局 | Split / Unified | 只画 Split |
| 提交按钮 | 可用 / 禁用 | 只画可用 |

```
进入：10 的 `Manage…`（板内 Changes 指向 Changelists.dc.html）
切换：点组头收起/展开；三个 radio 行只改「当前变更集」，要按 `Set Active` 才生效
离开：返回条 `< Commits` 回 04；浮层三行与 `Manage…` 回 10；`Agent` / `Ask Agent` 去 12
```

## 6. 画板没覆盖的组合（待补设计）

1. **新建变更集**：组头的 `+` 与浮层的 `New` 指向同一个还没画的命名对话框；新建后的空组长什么样、能否重名、是否立刻成为当前组，都没定。
2. **多选文件**：浮层写 `Move 3 selected files to…`，说明存在多选，但列表只画了单行 `sel` 与逐个复选框；多选的高亮、选中计数条、`⌥M` 之后选目标的那一步都没有画。
3. **浮层的开与关**：浮层直接画在板上常驻，打开它的 `Changelist options` 没有展开态；点外部关闭、`Move 3 selected files to…` 之后浮层是否关闭也没有画。
4. **空组与删除**：`Agent cleanup` 折叠后仍留计数 3；删掉一个变更集后文件回落到哪里（`Default Changelist`？）、只剩默认组时浮层还显不显示 radio 行，都没有画。

## 7. 按钮与链接清单

共 10 组；分段、页签、整行可点的行不计入，见本节末。

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 列表头右 | 加号（`aria-label=New changelist`） | 图标 24×24，12 图标 | 新建变更集 | 常驻 |
| 2 | 列表头最右 | `⋯`（`aria-label=Changelist options`） | 图标 24×24，12 图标 | 打开浮层 | 板上无展开态 |
| 3 | 每个组头 | `…`（`aria-label=<组名> options`） | 18×17 | 组的操作 | 每组一个，折叠组也有 |
| 4 | 每个文件行 | `…`（`aria-label=<文件名> actions`） | 18×18 | 文件菜单（同 10） | 每行一个 |
| 5 | 浮层头 | `New` | 文字按钮 10 | 新建变更集 | 常驻 |
| 6 | 浮层 | `Move 3 selected files to…`（`⌥M`）/ `Rename Changelist…` | 整行 28 高 | 移动选中文件 / 重命名 | 未选文件时文案如何变没画 |
| 7 | 浮层脚 | `Set Active` | 小按钮 22、`primary` | 确认当前变更集 | 常驻 |
| 8 | agent-nudge | Ask Agent | 文字链接 | 去 12 | 常驻 |
| 9 | 表单末 | Manage… | 文字链接 500 | 回 10 | 常驻 |
| 10 | 表单底 | Commit 3 Staged Files | 主按钮 30 高 | 提交 | 可用；下方另有 `Commit & Push` |

- **分段**：`Split / Unified`（`role=group`）；**页签**：`Commit 12`（`button`）、`Agent`（`a`，`role=tab`，链 12）。
- **整行可点**：6 行文件、3 个组头、浮层 3 行变更集（radio 行，当前行 600 + 实心）、侧栏 30 行引用；复选框是输入，不是按钮。**窗口级按钮**与 10 一致，见 [changes.md](changes.md) §7 与 [workspace.md](workspace.md) §8；本板工具条 `Agent` 是 `button`。

## 8. 文案要点

- 本板计数自洽：`12 files` = `3 + 6 + 3`；10 的板是 `3 + 9 + 1 = 13`，与 `12 files` 不一致，两板对照时以本板为准。
- 省略行写法随页变化：本板 `3 more files…`，10 写 `4 more unstaged files…`——后者点明「未暂存」，前者不点。
- 「变更集只是过滤，不改变提交集」出现两次：表单 `Changelists are filters; staging commits.`、浮层脚 `Filters only — staging commits`。`Set Active` 说明「当前变更集」要显式确认，点一行不生效。

## 9. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 10 Staging & commit | 浮层三行、`Manage…` 共 4 处 `Changes.dc.html`；本板由它的 `Manage…` 进入 |
| 04 Commits | 返回条 `< Commits`、工具条 `History`（板内两处 `Workspace.dc.html`） |
| 12 Agent | `Agent` 页签与 `Ask Agent`（板内两处 `Agent.dc.html`） |
| 14 Merge conflict | 本板没有横幅，冲突入口只在 10 的 `Resolve…` |
| 23 Settings / 01 Welcome | 齿轮 / 页签 `+`（板内各一处 href） |
| 30 Refs · Tags, Remotes, Stashes | 与变更集无关，两块板之间没有 href 互链；30 只由 09 的 `Open details` 进入 |
