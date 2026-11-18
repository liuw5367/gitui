# 文件 diff · 聚焦 — 布局、状态与按钮

- **画板**：04b File diff (focused)（Split 视图 · 有检查器 · 有授权横幅；无 Unified、无空 diff、无错误态）
- **来源**：`designs/graft-git/project/FileDiffFocus.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

板内 `.window` 没有内联尺寸，取样式表的 `height:calc(100vh - 48px)`：预览 1440×900、桌面留白 24，窗口 **1392×852**。
标题栏 / 工具条 / 横幅 / 侧栏 / 检查器 / 状态栏与 04 逐像素相同，差别只在中列：历史表换成「返回条 + diff 区」。

## 1. 骨架

```
┌───────────────┬─────────────────────────────────────────────┬───────────────────────────────┐
│ ● ● ●  [gitui ×] [docs-site ×] [atlas-web ×] [+]     ● Fetched 2m ago   [gear]              │
├─────────────────────────────────────────────────────────────────────────────────────────────┤
│ [=] | Fetch Pull Push | Branch Stash   [search commits ⌘F] | History Changes 12 | Agent [=] │
├─────────────────────────────────────────────────────────────────────────────────────────────┤
│ ℹ Authentication required for origin. Sign in to fetch or push. [Sign in…][Use SSH]         │
├─────────────────────────────────────────────────────────────────────────────────────────────┤
│ Branches 5    │ < Commits  3f2a1c9  Fix diff hunk ...       │ Fix diff hunk alignment       │
│ ● main * +2-0 │                   7 files  +162 -62         │ MC Mia Chen  2 hours ago      │
│ ● feature/dif │ src/git/diff-engine.ts      +18 -7          │ 3f2a1c9 [copy]                │
│ Remotes 2     │ [Split] Unified | [fold][wrap][ws][ignore]  │ Committed on main · 7 files   │
│   origin 8    │ 2 of 7   [<] [>]                            │  Changes 7  Files  Info       │
│ Tags 3        │ a/ src/git/diff-engine.ts                   │ 7 files changed +162 -62      │
│ ▣ v1.4.0      │ b/ src/git/diff-engine.ts                   │ [tree] [flat]                 │
│ Stashes 2     │ @@ -124,12 +124,13 @@  [Copy Hunk]          │ src/git                       │
│ Submodules 2  │ 124 export function ... | 124 export ...    │  diff-engine.ts M +18 -7      │
│ Worktrees 3   │ 128 ) {                | (empty)            │  hunk.ts A +44 -0             │
│               │ (empty)                | 128  maxWidth=96   │ (empty folder merged)         │
│               │ 129 const rows = lcs() | 130 const rows ... │ Open in Editor  [copy] [more] │
│               │ [v 14 unchanged lines]                      │                               │
├─────────────────────────────────────────────────────────────────────────────────────────────┤
│ main   ● 2 ahead · 0 behind        12 changes   UTF-8   LF   TypeScript                     │
└───────────────┴─────────────────────────────────────────────┴───────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 返回条 | 中列顶部 | 高 34（内联样式，不是类）；内距 0×11；底 `panel` | `< Commits` chip、SHA、提交标题、右端 `7 files · +162 −62` |
| diff 工具条 | 返回条下 | 高 38；内距 0×11；间距 8 | 路径 + 增删、Split/Unified、四个视图开关、`2 of 7`、前后改动 |
| diff 主体 | 工具条下 | 剩 640 高；等宽 11、行高 19 | 两栏文件头、hunk 行、上下文/增删行、折叠行 |
| 侧栏 | 主体左 | 宽 232；内距 2/8/12 | 与 04 同一套六组引用 |
| 检查器 | 主体右 | 宽 360；头内距 12/14/10 | 与 04 同一套提交头与页签，7 个文件 |
| 状态栏 | 底部通栏 | 高 24；内距 0×12；11 | 与 04 一字不差 |

主体 712 = 852 − 2 − 40 − 44 − 30 − 24；中列 798。返回条 34 + diff 工具条 38 之后，diff 主体正好 640 = 712 − 34 − 38。图中 `*` 是对勾，`[gear]` 是齿轮，`[=]` 是侧栏/检查器开关，`[ws]` 是显示空白的 `¶` 按钮。

## 2. 返回条

一条 34 高的「我在哪」摘要：`panel` 底、下边框 1、内距 0×11、间距 8。最左是 `< Commits` chip（22 高，左箭头 10），后面跟 SHA 徽章（mono 10.5、`panel-2` 底、1px `line-soft`、内距 2×5、圆角 4）、提交标题（10.5 `text3`，不省略）、弹性空隙、右端 `7 files · +162 −62`（同 10.5）。整条只有最左的 chip 可点，其余是静态摘要。

## 3. diff 工具条

左侧 crumb：路径 mono 11 `text` + 增删（`+18` 绿、`−7` 红，mono 9.5）；这里没有提交链，因为只有一个文件。
右半：`Split | Unified` 分段（tiny：容器内距 2、项高 21、内距 0×7、10.5，`Split` 白底为当前）→ 1×18 竖线 →
四个图标（折叠未改动上下文、自动换行、显示空白 `¶`、忽略空白差异）→ 竖线 → `2 of 7`（10.5、等宽数字）→
前后各一个 24×24 箭头。两个箭头的 aria 是 `Previous change` / `Next change`——**按「改动」计**，不像 06/07 按文件计。

`Show whitespace` 只把显示副本中的空格和 tab 换成可见符号；`Ignore whitespace` 重新调用 Git `--ignore-all-space`。两者互不影响，复制与导出始终保留原始 patch。DiffView 依据每个文件的路径独立选择内置语法，未知扩展名回退纯文本。

## 4. diff 主体

文件头 `.dhead` 左右两格各占一半（1fr 1fr），9.5 `text3`、内距 4×9，左格写 `a/` + 路径、右格写 `b/` + 路径，
两格之间 1px `line`。表格 `table-layout:fixed`，每行 4 格：左行号 42 / 左代码 / 右行号 42 / 右代码；行号右对齐、
内距左 4 右 7、9.5、右侧 1px `line-soft`；代码内距 0×10、`pre` 不换行（超出即裁）。行高由 11/19 定死。

```
┌───────┬────────────────────────────┬───────┬────────────────────────────┐
│   124 │ export function alignHunks(│   124 │ export function alignHunks(│
│   128 │ ) {                        │       │                            │
│       │                            │   128 │   maxWidth = 96,           │
│   129 │ const rows = lcs(a, b)     │   130 │ const rows = lcs(a, b)     │
└───────┴────────────────────────────┴───────┴────────────────────────────┘
```

- **hunk 行**横跨四格：高 21，`hunk-bg` 底 + 上下 1px `line-soft`，10 `hunk-fg`，内距 0×8；右端 `Copy Hunk` 是 mini 按钮（高 17、内距 0×5、无衬线 9.5、1px `line` 描边、圆角 4）。文案照抄 `@@ -124,12 +124,13 @@ export function alignHunks(`。
- **上下文行**两侧行号同号；**删除行**左格 `del-bg`、左行号 `del-ln`、右格与右行号是 `.empty`（`panel-2` + 0.55 透明）；**新增行**对称。
- **词级高亮**：删除侧 `columns = split ? 2 : 1`、新增侧 `maxWidth = 96,` 各是一块圆角 2 的 `del-word` / `add-word` 底色，板上一共只有这两处。
- **折叠行**横跨四格：高 20、`fold-bg`、上下 1px `line-soft`、居中 10 无衬线，文案 `14 unchanged lines` / `24 unchanged lines`，两侧各一个 11 的箭头（板上两个都朝下）。
- 语法色分六类：关键字、字符串、注释（斜体）、数字、类型、函数。

## 5. 检查器

与 04 同一套：标题、作者行、SHA 徽章、`Committed on main · 7 files`、`Changes 7 / Files / Info` 三个页签、
统计行 `7 files changed +162 −62` 与树/平铺图标对、4 个文件组 7 个文件、`Empty folder merged · src/git/legacy` 注记、
页脚 `Open in Editor` + 复制 + `⋯`。**唯一差别**：`diff-engine.ts` 这一行在 04 里是指向本页的链接，在本页改回普通按钮
（已经在这一页，不再跳转），并带 `aria-current=true`。

## 6. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| diff 布局 | Split / Unified | 只画 Split |
| 折叠 | 全展开 / 部分折叠 | 只画「两段各折叠 14、24 行，其余展开」 |
| 改动位置 | 第 n / m 处 | 只画 `2 of 7` |
| 检查器页签 | Changes / Files / Info | 只画 Changes |
| 授权横幅 | 显示 / 不显示 | 只画显示 |

```
进入：04 检查器里点文件行（板上只有第一条是链接）    切换：Split ⇄ Unified 只换渲染
离开：返回条 `< Commits` 回 04；检查器 `Files` 去 18；页签 × 关闭；齿轮去 23
```

## 7. 画板没覆盖的组合（待补设计）

1. **Unified**：本页自己没画单文件的「两列行号 + 一列代码」排法，只在 06/07 画了多提交版。
2. **极端 diff**：只有新增 / 只有删除 / 纯重命名 / 二进制 / 仅权限变更时 diff 区长什么样，没有板。
3. **折叠与边界**：折叠行两个箭头都朝下，「展开」与「收起」不区分；`1 of 7` / `7 of 7` 时前后按钮是否禁用没画。
4. **自动换行**：开启后行高从 19 变多少、横向滚动条消失没有，没有板。
5. **错误态**：文件被删除、diff 读取失败、文件过大时的提示没有画；检查器 `Files` / `Info` 的内容也不在本板。

## 8. 按钮与链接清单

共 12 组；分段、页签、整行可点的行不计入，见本节末。

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 返回条 | `< Commits`（`aria-label=Back to commit list`） | chip 22 高 + 左箭头 10 | 回 04 | 常驻 |
| 2 | diff 工具条 | Split / Unified | 分段两项（`role=group`） | 切渲染 | 当前项白底；板上 `Split` |
| 3 | diff 工具条 | 折叠图标（`aria-label=Collapse all hunks`） | 图标 24×24 | 折叠全部 hunk | 常驻，无按下态 |
| 4 | diff 工具条 | 换行图标（`aria-label=Toggle word wrap`） | 图标 24×24 | 开关自动换行 | 无 `on` 态 |
| 5 | diff 工具条 | `¶`（`aria-label=Show whitespace`） | 图标按钮 | 显示空格和 tab | 状态强调色 |
| 6 | diff 工具条 | Filter（`aria-label=Ignore whitespace changes`） | 图标按钮 | 让 Git 忽略空白差异 | 状态强调色 |
| 7 | diff 工具条 | `<`（`aria-label=Previous change`） | 图标按钮 | 上一处改动 | 无 hunk 时禁用 |
| 8 | diff 工具条 | `>`（`aria-label=Next change`） | 图标按钮 | 下一处改动 | 无 hunk 时禁用 |
| 9 | 每个 hunk 右端 | Copy Hunk | mini 17 高 | 复制该 hunk | 每个 hunk 一个 |
| 10 | 检查器页脚 | Open in Editor | 小按钮 22 高 + 外链 12 | 在编辑器打开 | 常驻 |
| 11 | 检查器页脚 | 复制（`aria-label=Copy commit SHA`） | 图标 24×24 | 复制 SHA | 常驻 |
| 12 | 检查器页脚 | `⋯`（`aria-label=More commit actions`） | 图标 24×24 | 更多提交动作 | 板上没有展开菜单 |

- **窗口级按钮**（页签 `+`、齿轮、侧栏/检查器开关、Fetch/Pull/Push/Branch/Stash/Agent、横幅 `Sign in…` 与 `Use SSH`）与 04 完全一致，逐个见 [workspace.md](workspace.md) §8。**页签**：检查器 `Changes 7 / Files / Info`（`role=tablist`，`Files` 链去 18）；**整行可点**：7 行文件（当前行 `aria-current=true`）。

## 9. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 04 Commits | 返回条 `< Commits` 回它；本页由它检查器的文件行进入 |
| 18 Files at commit | 检查器 `Files` 页签 |
| 06 / 07 | 共用同一套 diff 行结构，但那边是 Unified + 多提交 |
| 23 Settings / 27 Credentials / 01 Welcome | 齿轮 / 横幅两个按钮 / 页签 `+` |
