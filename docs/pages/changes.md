# 暂存与提交 — 布局、状态与按钮

- **画板**：10 Staging & commit（合并中横幅 · 钩子失败 · 选中 1 个已暂存文件 · 提交按钮禁用；无干净工作区、无钩子通过、无空暂存）
- **来源**：`designs/graft-git/project/Changes.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

板内 `.window` 没有内联尺寸，取样式表的 `height:calc(100vh - 48px)`：预览 1440×900、桌面留白 24，窗口 **1392×852**；`.desktop` 另带内联 `position:absolute;left:0;top:-1px`，只把画布上移 1px，不改尺寸。本页是**主窗口内换主体**（工具条 `Changes 12` 为当前项），五段竖排外多一条合并横幅：标题栏 40 / 工具条 44 / 横幅 30 / 三栏主体 / 状态栏 24。主体 **712** = 852 − 2 − 40 − 44 − 30 − 24，中列 798 = 1390 − 232 − 360。

运行时初始入口按当前工作流显示：工具条保持 Changes 选中，右栏显示变更清单树与底部提交区，中栏显示 History 提交列表。点击右栏文件后，中栏整栏切换为本页画板所示的 Diff；返回条 `< Commits` 回到提交列表。中栏不同时堆叠 Changes 文件列表和 Diff。

## 当前 Changes 入口实现

- Changes 右栏不显示独立的 `Commit` / `Agent` 标题行，也不显示 `Ask Agent to draft`；全局工具栏的 Agent 入口仍保留。Refs 使用的提交检查器保持原样。
- Changelists 是无外围圆角与边框的弹性区域，列表在内部滚动。标题行显示文件总数及 staged / unstaged 数，并提供全部折叠／展开切换；筛选框为 24 高、有上下留白，输入有内容时可在框内清空。每个清单名称行保持方角，整行点击折叠或展开，右键菜单提供设为默认、修改名称、删除、暂存全部和取消暂存全部；折叠状态按仓库标签保留。
- 保留底部拖动提示与清单管理入口，提示说明拖动文件可将其移动到其他清单，去掉前置省略号图标；Manage 保持贴右，按钮文案不再带省略号。移除改变 Changelists 高度的拖拽手柄。
- 清单管理弹窗在标题栏右侧提供关闭按钮，贴近底部的 Manage 入口显示；移除底部 “Filters only” 提示。默认清单也显示禁用的删除按钮，以保持操作列对齐；非默认清单可显式删除，删除后其变更回到默认清单。拖动清空的自定义清单会保留，且空清单可跨应用重启恢复。
- 清单文件树复用提交信息面板的共享层级树控件，文件夹可折叠并显示从仓库根目录起的完整相对路径，空间不足时从左侧省略；平铺模式显示完整文件路径并在悬停时提示。文件行仍使用 Changes 的暂存、选择、拖放和右键菜单操作，状态标记排在增删行数之后，已暂存复选框不显示边框。拖放目标以整个清单区域的背景色标记。
- Summary 和 Description 固定在提交区底部，没有单独标签，通过 placeholder 说明用途；Description 从一行随内容增高，最多三行。amend、sign-off、no verify 位于第一行，push 与 force push 位于第二行，Commit 在右侧垂直居中。勾选 no verify 后点击 Commit 会先要求确认，再使用 `--no-verify` 跳过钩子；提交成功后恢复关闭。Force push 始终显示，Push 未开启时禁用，启用后使用 `--force-with-lease`。无暂存改动时不显示额外提示。提交与钩子结果显示在输入区上方，状态块可关闭；勾选提交区的 push 后，提交反馈标题显示“推送中”和当前步骤/总步数，推送进行中为橙点，整个流程成功后变为绿点，失败显示红点。工具栏 Push 的结果仍只显示在同步弹窗。
- 开启 amend 时，自动载入 `HEAD` 的提交摘要和正文作为新的提交草稿。
- 该运行布局是按用户反馈批准的设计差异，见 [DEVIATIONS #59](../design/DEVIATIONS.md#59-changes-面板与提交区布局)。

## 1. 骨架

当前工作区文件 Diff 与 Stash Diff 使用共享的 NSH DiffView 面板，支持 Split / Unified、折叠上下文、软换行、显示空白、改动导航、复制 patch 和按文件路径语法高亮。Split / Unified、折叠上下文、软换行、显示空白与忽略空白选择作为应用级本地偏好保存，History、Changes、Stash、Blame 和 Reflog 入口共用，切换文件或重启应用后保持一致。差异行只使用红／绿背景，不显示左侧加减号。工具条显示新增与删除行数；Export 已移除。Changes 标题显示返回 Commits 和当前文件名。Stage Hunk 位于原有 hunk 分隔行，Stage Line 位于悬浮 gutter 操作槽，两者不增加 DiffView 的代码行高度。暂存索引仍依据原始 Git 补丁的上下文与变更行，不依赖 DiffView 重新编号。无文本 hunk 的二进制文件或模式变更显示无文本变化提示。

```
┌────────────────┬──────────────────────────────────────────────────┬───────────────────────────────────┐
│ ● ● ●  [gitui ×] [docs-site ×] [atlas-web ×] [+]     ● Fetched 2m ago   [gear]                        │
├───────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ [=] | Fetch Pull Push | Branch Stash   [search ⌘F] | History [Changes 12] | Agent [=]                 │
├───────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ [branch] Merging origin/main into main · 2 conflicts to resolve.   [Resolve…] [Abort]                 │
├───────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ Branches 5     │ < Commits [Staged] src/ui/CommitPanel.tsx        │ Commit 12                  [more] │
│ ● main ✓ ↑2    │                              12 files · +149 -90 │ Changelists 12 files [tree][flat] │
│ ● feature/diff │ [Staged] > src/ui/CommitPanel.tsx  +42 -17       │ v ● Wrapped-line fix 3            │
│ Remotes 2      │ [Split] Unified [fold][wrap][ws] 1 of 3          │ ☑ CommitPanel.tsx  M +42 -17      │
│   origin 8     │ a/ src/ui/CommitPanel.tsx                        │ ☑ AgentPanel.tsx   M +18  -6      │
│ Tags 3         │ b/ src/ui/CommitPanel.tsx                        │ ☑ commit.ts        M  +9  -2      │
│ ▣ v1.4.0       │ @@ -20,9 +20,11 @@   [Unstage Hunk]              │ v · Default Changelist 9          │
│ Stashes 2      │  20 const staged = … | 20 const staged = …       │   ☐ tokens.css     M +14  -8      │
│ Submodules 2   │  21 canCommit=true  | (empty)                    │ 4 more unstaged files…            │
│ Worktrees 3    │ (empty)             | 21 !!message.trim()        │ Drag files onto another…          │
│                │  22 return (        | 22 return (                │ Commit to main                    │
│                │ [Unstage line]  added-line action                │             3 staged · 9 unstaged │
│                │ [v] 9 unchanged lines                            │ Summary  Polish composer…         │
│                │                                                  │ Description  Disable…             │
│                │                                                  │ ☑ Sign off                        │
│                │                                                  │ ● Hooks  Pre-commit failed        │
│                │                                                  │ 1.2s              [View full log] │
│                │                                                  │ [Retry hooks] or                  │
│                │                                                  │ [Commit without hooks]            │
│                │                                                  │ Commit 3 Staged Files (disabled)  │
│                │                                                  │ Hooks must pass before…           │
├───────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ main   ● 2 ahead · 0 behind        12 changes   UTF-8   LF   TypeScript                               │
└────────────────┴──────────────────────────────────────────────────┴───────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 合并横幅 | 工具条下 | 高 30；内距 0×12；间距 7 | 14 的分支图标、一句合并状态、`Resolve…` / `Abort` |
| 返回条 | 中列顶部 | 高 34（内联）；内距 0×11；间距 8 | `< Commits`、`Staged` 徽章、文件名、`12 files · +149 −90` |
| diff 工具条 | 返回条下 | 高 38；内距 0×11；间距 8 | `Staged` 徽章 + 路径 + 增删、Split/Unified、三个开关、`1 of 3`、一个前进箭头 |
| diff 主体 | 工具条下 | 剩 640 = 712 − 34 − 38；等宽 11、行高 19 | 两栏文件头、1 个 hunk、13 行代码、1 行折叠 |
| 侧栏 | 主体左 | 宽 232；内距 2/8/12 | 与 04 同一套六组引用 |
| 提交面板 | 主体右 | 宽 360；页签 33；表单内距 12 | `Commit 12` / `Agent` 页签、变更集列表、提交表单、钩子块 |
| 状态栏 | 底部通栏 | 高 24；内距 0×12；11 | `main`、`● 3 staged · 9 unstaged`、`12 files · +149 −90`、编码、换行、语言 |

图中 `[more]` 是行级 `…`，`[tree]/[flat]` 是树/平铺图标对，`[v]` 是折叠箭头，末行 `Commit 3 Staged Files (disabled)` 是禁用态主按钮。

## 2. 横幅与中列

- **横幅** 30 高、`accent-soft` 底、下边框 1、10 `text2`：图标位 14×14（`accent` 分支图标）→ `<b>Merging origin/main into main</b>`（600、`text`）→ `· 2 conflicts to resolve.` → 弹性空隙 → `Resolve…`（`<a>` 链 14）与 `Abort` 两个小按钮都被压到 20 高、9.5、内距 0×7。样式表另有 `.op-banner.warn`（`warn` 11% 混底、图标转 `warn`），板上没用。
- **返回条与工具条**：返回条是 `< Commits` chip（22 高、左箭头 10、`aria-label="Back to commit list"`）→ `Staged` 徽章（18 高、圆角 4、内距 0×6、`panel-2` 底、1px `line-soft`、9.5）→ 文件名（10.5 `text3`）→ 空隙 → `12 files · +149 −90`；工具条左簇是同一枚徽章 + 10 的右箭头 + 路径（mono 11）+ `+42` / `−17`（mono 9.5），右簇为 `Split | Unified`（tiny：项高 21、内距 0×7、10.5）、1×18 竖线、三个 24×24 图标（折叠/换行/`¶`）、竖线、`1 of 3`（10.5 等宽数字）、只有 `Next change` 一个箭头。
- **主体**：由 NSH DiffView 绘制，行号与语法高亮随文件路径识别；新增／删除行仅使用对应背景色。Stage Hunk 复用 hunk 标题行，Stage Line 通过行号 gutter 的悬浮动作槽出现，不插入行下注释。

## 3. 提交面板的 Changelists 段

```
┌──────────────────────────────────────────────────────┐
│ v ● Wrapped-line fix                 3      …        │
│   ☑ CommitPanel.tsx       M      +42  -17   …        │
│   ☑ AgentPanel.tsx        M      +18   -6   …        │
│   ☑ commit.ts             M       +9   -2   …        │
│ v · Default Changelist              9      …         │
│   ☐ tokens.css            M      +14   -8   …        │
│   ☐ router.ts             M       +6   -3   …        │
│ 4 more unstaged files…                               │
│ v · Docs                            1      …         │
│   ☐ README.md             M       +3   -1   …        │
└──────────────────────────────────────────────────────┘
```

- **容器** 1px `line`、圆角 7、`content` 底、`overflow:visible`。头部 26 高、`panel-2` 底、下边框 1、内距 0×8、间距 6、10/600 `text2`：11 的分支图标 → `Changelists` → `12 files`（10/500，`margin-left:auto` 贴右）→ 树/平铺开关（容器内距 2、圆角 5、1px `line-soft`；两键 22×18，当前项白底）→ `Filter changes`（24×24，12 的漏斗）。
- **列表** 限高 252、自身滚动。组头 23 高、内距 0×7、上距 3、间距 6、10.5/600：10 的箭头 + 7×7 圆点（活动组 `accent`，其余 `text3`）+ 组名（10/600 `text`）+ 计数 10.5/600 + 空隙 + `…`（18×17、12）。三组：`Wrapped-line fix 3`（活动）、`Default Changelist 9`（组头上方另加 2px `accent` 边框）、`Docs 1`。
- **文件行** 28 高（内联覆盖 27）、内距 0×7、圆角 5、间距 6：复选框 13 → 文件名 mono 10.5 → 状态字母 11 宽（9.5/700，`M` `warn`、`A` `ok`、`D` `danger`、`R` `purple`）→ 右端 `+42 −17`（mono 9.5）→ `…`（18×18）。`CommitPanel.tsx` 一行是 `sel` 底；`tokens.css` 一行用 0.55 不透明度 + `outline:1px dashed accent`（负 2 偏移）画投放目标，全板唯一。省略行 `4 more unstaged files…` 22 高、左内距 20、9.5 `text3`。
- **分隔条与提示** 9 高：`panel-2` 底、上边框 1、正中四个 2×2 圆点（间距 3），`role=separator`、`aria-label="Resize changelist panel"`、`cursor:row-resize`；其下 24 高的 `Drag files onto another changelist to move them`（`⋯` 10 + 9.5）。
- **文件菜单** 宽 252（`.menu.flat` 覆盖 `.menu` 的 260 与 `.file-menu` 的 212）、内距 5、圆角 7、1px `menu-line`、菜单阴影，内联 `left:12px;top:150px`。首行 `mlabel` 是完整路径 `src/ui/CommitPanel.tsx`（9/600 大写），随后 7 项 `mi`（24 高、12.5、内距 0×8）：`Revert Changes…`、`Unstage File`、`Open in Editor`、`Reveal in Finder`、`Copy Path`、`Move to Changelist…`、`Discard…`，两条 `msep` 分组，末项 `danger` 色。

## 4. 表单、选项与钩子

- **页签** 33 高、内距 0×14（右 8）：`Commit 12`（当前，计数徽章 9.5、`panel-2` 底、圆角 4、`padding:0 3px`）→ `Agent`（`<a>` 链 12，11 的 sparkle）→ 空隙 → `Commit options`（`⋯` 24×24）。表单 `.form` 自带内距 12、间距 9、`min-height:100%`。
- `Commit to main` 行 11 `text2`：12 的分支图标 + `Commit to` + `main`（600 `text`）+ 空隙 + `3 staged · 9 unstaged`；下一行 `✓` 10 `ok` + `3 staged files across all changelists`（10.5 `text3`）。
- **Summary** 标签 11/600 + 右端 `Required`（10.5），字段高 29（内联，样式表默认 26）、内距 0×8、圆角 6、内文 12.5，板上值 `Polish commit composer states`；**Description** 标签 + `Markdown`，`textarea` `rows=3`、`min-height:76`、内距 6×8、11.5/1.45。meta-line 10 `text3` 两端：`Only staged files are committed` / `⌘↵ to commit`。`agent-nudge` 是 1px `line`、圆角 7、内距 7×8、间距 7、10.5 的一行：sparkle 13 `accent` + `Draft a message from staged files` + `Ask Agent`（`accent-2` 500，链 12）；下方三个复选竖排、行距 2（22 高、间距 7、11.5 `text2`）：`Amend last commit` / `Sign off`（勾选）/ `Push after commit`。
- **钩子块** 1px `danger` 26% 混边、圆角 7、`danger` 4% 混底、内距 8×9、行距 5。头 10.5：6 的 `danger` 圆点 + `Hooks`（600 `text`）+ `Pre-commit failed`（`danger` 500）+ 右端 `1.2s`（9）+ `View full log`（9 `accent`）。四个步骤胶囊 15 高（mono 8、内距 0×5、圆角 4）：`pre-commit` 失败态、另三个中性。日志框限高 78、内距 6×7、圆角 5、mono 8.5/14、`pre-wrap`，五行从 `$ .husky/pre-commit` 到 `husky - pre-commit hook exited with code 1`。动作行：`Retry hooks`（小按钮 22）+ `or` + `Commit without hooks`（ghost、`danger` 字、`title` 注明 `--no-verify`）。
- 末尾 `Changelists are filters; staging commits.` + `Manage…`（链 11）。**主按钮** `Commit 3 Staged Files` 高 30、`primary`、**禁用**（`opacity:.45`、无阴影、不吃指针），`form-actions` 用 `margin-top:auto` 压在面板底；其下居中的 `Hooks must pass before committing` 是禁用原因。

## 5. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 合并与钩子 | 进行中 + 失败 / 都没有 | 只画「进行中 + 失败」 |
| 暂存区 | 有已暂存（3）/ 空 | 只画 3 |
| 选中行 | 任一文件 / 无 | 只画 `CommitPanel.tsx`（`sel`） |
| diff 布局 | Split / Unified | 只画 Split |
| 提交按钮 | 可用 / 禁用 | 只画禁用 |

```
进入：启动时工具条 `Changes` 选中，中栏先显示 History 提交列表，右栏显示 Commit；工具条 `Changes 12`（04 / 07 / 08 都链到这里）
切换：点击右栏文件行的非复选框区域会选中文件并在中栏显示整栏 Diff；点击复选框只暂存或取消暂存并更新计数，只有中栏当前显示同一文件时才刷新 Diff；树/平铺只换列表排法；拖动文件到另一清单会移动文件；横幅不随选择变化
离开：返回条 `< Commits` 回提交列表；`Resolve…` 去 14；`Manage…` 去 11；`Agent` / `Ask Agent` 去 12
```

## 6. 画板没覆盖的组合（待补设计）

1. **正常态**：横幅与钩子块都不出现时，主体从 712 回到 742、`staged-box` 与表单怎么重新分配高度没有画；`12 changes` 归零后工具条分段的计数怎么写也没有画。
2. **钩子通过 / 重试中**：成功态的头部（绿点 + 什么文案）、`Retry hooks` 按下后的进行与再次失败、`View full log` 落在哪里都没有画；`Commit without hooks` 是否二次确认也没有画。
3. **空暂存与多选**：`0 staged` 时主按钮文案（数字取自 `3 Staged Files`）与禁用依据、`3 staged · 9 unstaged` 两侧同时为 0 的写法、`Default Changelist` 为空时是否折叠都没有画；多选后 `Move to Changelist…` 的目标选择、拖动经过其它组时的高亮也没有画，板上只有 `tokens.css` 一行的虚线投放态。

## 7. 按钮与链接清单

共 11 组；分段、页签、整行可点的行不计入，见本节末。

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 横幅 | `Resolve…` / `Abort` | 小按钮压到 20 高 | 去 14 解冲突 / 放弃合并 | 仅横幅出现时 |
| 2 | 返回条 | `< Commits`（`aria-label=Back to commit list`） | chip 22 高 | 回 04 | 常驻 |
| 3 | diff 工具条 | 折叠 / 换行 / `¶` / `>`（`Collapse all hunks` / `Toggle word wrap` / `Show whitespace` / `Next change`） | 图标 24×24 | 视图开关 / 下一处改动 | 都无 `on` 态；没有上一个 |
| 4 | hunk 右端 / 新增行内 | Unstage Hunk / Unstage line | mini 17 高 / 行内 8.5 | 反暂存该 hunk / 该行 | 每 hunk、每个新增行各一个 |
| 5 | 列表头 | 树 / 平铺（`Tree view` / `List view`）；漏斗（`Filter changes`） | 图标对 22×18；图标 24×24 | 换列表排法；过滤文件 | 当前项白底 |
| 6 | 组头 / 文件行 | `…`（`aria-label=<组名> options` / `<文件名> actions`） | 18×17 / 18×18 | 组操作 / 文件菜单 | 每组、每行各一个 |
| 7 | 文件菜单 | 7 项，`Revert Changes…` 至 `Discard…` | 菜单项 24 高 | 见 §3 | 末项 `danger` |
| 8 | 页签行右 | `⋯`（`aria-label=Commit options`） | 图标 24×24 | 提交选项 | 常驻 |
| 9 | agent-nudge | Ask Agent | 文字链接 | 去 12 起草消息 | 常驻 |
| 10 | 钩子块 | `View full log` / `Retry hooks` / Commit without hooks | 9 的文字链接 / 小按钮 22 / ghost | 看日志 / 重跑 / 跳过钩子 | 仅钩子失败时 |
| 11 | 表单 | Manage… / Commit 3 Staged Files | 文字链接 500 / 主按钮 30 高 | 去 11 / 提交 | 主按钮板上禁用 |

- **分段**：`Split / Unified`（`role=group`）、树/平铺（`role=group`、`aria-label="Working tree layout"`）；**页签**：`Commit 12` 是 `button`、`Agent` 是 `a`（`role=tab`，链 12）。
- **整行可点**：9 行文件、3 个组头、侧栏 30 行引用；复选框是输入，不是按钮。**窗口级按钮**与 04 一致，见 [workspace.md](workspace.md) §8；本板工具条 `Agent` 是 `button`（08/09 是链到 12 的 `<a>`）。

## 8. 文案要点

- 三处计数口径不同：`12 files · +149 −90`（整个工作区）、`3 staged · 9 unstaged`（暂存/未暂存）、`3 staged files across all changelists`（跨变更集合计），主按钮只数已暂存。清单负责分组和筛选，真正进入提交的是已暂存文件。
- `Unstage Hunk` / `Unstage line` / `Unstage File` 分别对应 hunk、行、文件三级，三个词不要互换。

## 9. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 04 Commits | 返回条 `< Commits`、工具条 `History`（板内两处 `Workspace.dc.html`） |
| 14 Merge conflict | 横幅 `Resolve…`（板内 `MergeConflict.dc.html`） |
| 11 Changelists | `Manage…`（板内 `Changelists.dc.html`） |
| 12 Agent | 检查器 `Agent` 页签与 `Ask Agent`（板内两处 `Agent.dc.html`） |
| 23 Settings / 01 Welcome | 齿轮 / 页签 `+`（板内各一处 href） |
