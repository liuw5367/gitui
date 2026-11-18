# 提交信息 — 布局、状态与按钮

- **画板**：08 Commit info（有检查器 `Info` 页 · 有单文件 diff · 有 `Verified · SSH`；无 `Changes`/`Files` 内容、无未签名提交、无合并提交多父）
- **来源**：`designs/graft-git/project/CommitInfo.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

板内 `.window` 没有内联尺寸，取样式表的 `height:calc(100vh - 48px)`：预览 1440×900、桌面四周留白 24，窗口 **1392×852**（1px 描边、圆角 12）。本页是**主窗口内换主体**：标题栏、工具条、侧栏、状态栏与 04 相同，中列换成「返回条 + 单文件 diff」，右列检查器把 `Info` 页签切到选中态。没有横幅，主体高 **742** = 852 − 2 − 40 − 44 − 24。

## 1. 骨架

```
┌────────────────┬──────────────────────────────────────────────────┬───────────────────────────────────┐
│ ● ● ●  [gitui ×] [docs-site ×] [atlas-web ×] [+]     ● Fetched 2m ago   [gear]                        │
├───────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ [=] | Fetch Pull Push | Branch Stash   [search ⌘F] | [History] Changes 12 | Agent [=]                 │
├───────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ Branches 5     │ < Commits  3f2a1c9  Fix diff hunk ...            │ Fix diff hunk alignment           │
│ ● main ✓ ↑2    │ src/git/diff-engine.ts     +18 -7                │ M Mia Chen  2h ago · Jun 14       │
│ ● feature/diff │ [Split] Unified   [fold][wrap][ws][ignore]       │ 3f2a1c9 · 8b41e07 · 14 files      │
│ Remotes 2      │ 2 of 7   [<] [>]                                 │ Changes  Files  [Info]            │
│   origin 8     │ a/ src/git/diff-engine.ts                        │ Commit    3f2a1c9e8b41e07         │
│ Tags 3         │ b/ src/git/diff-engine.ts                        │ Parents   8b41e07                 │
│ ▣ v1.4.0       │ @@ -124,12 +124,13 @@   [Copy Hunk]              │ Tree      14 files · +248 -81     │
│ Stashes 2      │ 124 export function... | 124 export function...  │ Refs      main, origin/main       │
│ Submodules 2   │ 128 ) {                | (empty)                 │ Signature Verified · SSH          │
│ Worktrees 3    │ (empty)                | 128   maxWidth = 96,    │ Author    Mia Chen <mia@…>        │
│                │ 129 const rows = lcs()  | 130 const rows = lcs() │ Committer Mia Chen                │
│                │ [v] 12 unchanged lines                           │ Authored  Jun 14, 11:42           │
│                │                                                  │ Actions                           │
│                │                                                  │ [Copy SHA] [Open on GitHub]       │
│                │                                                  │ [Cherry-pick] [Revert]            │
├───────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ main   ● 2 ahead · 0 behind        12 changes   UTF-8   LF   TypeScript                               │
└────────────────┴──────────────────────────────────────────────────┴───────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 标题栏 | 顶部通栏 | 高 40（内联 `height:40px` 覆盖样式表的 44）；内距 0×13 | 窗口灯 12、三个仓库页签、`+`、右端 `● Fetched 2m ago` 与齿轮 |
| 工具条 | 标题栏下 | 高 44；内距 0×10；间距 6 | 侧栏开关、Fetch/Pull/Push、Branch/Stash、搜索框 176×26、`History / Changes 12`、`Agent`、检查器开关 |
| 返回条 | 中列顶部 | 高 34（内联，不是类）；内距 0×11；间距 8 | `< Commits` chip、提交标题、右端 `7 files` |
| diff 工具条 | 返回条下 | 高 38；内距 0×11；间距 8 | SHA 徽章 + 路径 + 增删、`Split / Unified` 分段、四个视图开关、`2 of 7`、前后改动 |
| diff 主体 | 工具条下 | 剩 670 高 = 742 − 34 − 38；等宽 11、行高 19 | 两栏文件头、1 个 hunk、14 行代码、1 行折叠 |
| 侧栏 | 主体左 | 宽 232（含右边框 1）；内距 2/8/12 | 与 04 同一套六组引用 |
| 检查器 | 主体右 | 宽 360（含左边框 1）；头内距 12/14/10 | 提交头、三个页签、8 行键值、Actions、一段说明 |
| 状态栏 | 底部通栏 | 高 24；内距 0×12；11 | 分支、`● 2 ahead · 0 behind`、`12 changes`、`UTF-8`、`LF`、`TypeScript` |

中列 798 = 1390 − 232 − 360。图中 `[Split] Unified` 是分段，`[fold]/[wrap]/[ws]/[ignore]` 是四个图标按钮，`[Copy SHA]` 一类方括号是按钮占位，`M` 是头像里的首字母。

## 2. 返回条与 diff 工具条

**返回条**只有四个元素，34 高、`panel` 底、下边框 1：`< Commits` chip（高 22、内距 0×7、圆角 5、1px `line` 描边、`panel` 底、11，内嵌 10 的左箭头，`aria-label="Back to commit list"`）→ 提交标题（套 `hist-count`：10.5 `text3`，不省略）→ 弹性空隙 → `7 files`（同 10.5）。除 chip 外都不可点。

**diff 工具条** 38 高，左右两簇：

- 左簇 crumb：SHA 徽章（mono 10.5、`panel-2` 底、1px `line-soft`、内距 2×5、圆角 4）→ 10 的右箭头 → 路径（mono 11 `text`，可省略）→ `+18`（`add-fg`）/ `−7`（`del-fg`），后两者 mono 9.5。
- 右簇：`Split | Unified`（分段容器内距 2、圆角 7、`panel-2` 底、1px `line-soft`；项高 21、内距 0×7、10.5/500，`Split` 白底 + 0 1px 1px 阴影）→ 折叠、换行、显示空白 `¶`、忽略空白差异四个开关 → `2 of 7`（10.5、等宽数字）→ 前后两个改动导航箭头。忽略空白由 Git 重新生成差异；显示空白只转换展示副本。

## 3. diff 主体

```
┌──────────┬──────────────────────────────────┬──────────┬──────────────────────────────────┐
│      124 │ export function alignHunks(      │      124 │ export function alignHunks(      │
│      128 │ ) {                              │          │                                  │
│          │                                  │      128 │   maxWidth = 96,                 │
│      129 │ const rows = lcs(a, b)           │      130 │ const rows = lcs(a, b)           │
└──────────┴──────────────────────────────────┴──────────┴──────────────────────────────────┘
```

- **文件头** `.dhead`：`grid 1fr 1fr`，9.5 mono `text3`，内距 4×9，左格 `a/`、右格 `b/`，前缀 600 用 `text2`，两格之间 1px `line`。
- **表格** `table-layout:fixed`，每行 4 格：行号 42 + 代码 + 行号 42 + 代码。798 减两条 42 后两栏各 357。
- **行号** 9.5 `text3`、右对齐、内距左 4 右 7、右侧 1px `line-soft`、不可选中。
- **代码** 内距 0×10、`white-space:pre`、超出裁掉；字号取 `.diffbody` 的 11、行高 19，所以一行固定 19 高。样式表另有 `.split` 下的 10.5 一档，本板 `diffbody` 没有 `split` 类，画出来的是 11。
- **hunk 行**横跨四格：21 高、`hunk-bg` 底、上下各 1px `line-soft`、10 `hunk-fg`、内距 0×8；右端 `Copy Hunk` 是 mini 按钮（17 高、内距 0×5、无衬线 9.5、1px `line` 描边、圆角 4）。文案照抄 `@@ -124,12 +124,13 @@ export function alignHunks(`。
- **上下文行**两侧行号同号；**删除行**整行 `del-bg`、左行号列 `del-ln`，右侧三格是 `.empty`（`panel-2` + 0.55 透明）；**新增行**对称。板上的词级高亮只有一处：新增侧 `maxWidth = 96,`（`add-word` 底、圆角 2）。
- **折叠行**横跨四格：20 高、`fold-bg`、上下 1px `line-soft`、居中 10 无衬线，文案 `12 unchanged lines`，两侧各一个 11 的箭头（板上都朝下）。
- 语法色六类：关键字、字符串、注释、数字、类型、函数。

## 4. 检查器的 Info 页

- **头部**（内距 12/14/10、`panel` 底、下边框 1）：标题 13/600、行高 1.32、字距 −.005em；下距 9 是作者行（头像 21 圆形、`avatar-bg` 底 `avatar-fg` 字、9/700，板上字母 `M`；右侧姓名 11.5/600 与 `2h ago · Jun 14, 11:42` 10 `text3`；右端 `main` 是分支徽章，高 16、内距 0×5、圆角 4、9.5/600）；再下 9 是 hashrow（10.5 `text3`，间距 6）：`3f2a1c9` 徽章 · `8b41e07` · `14 files`。后两段是纯文本，没有徽章底。
- **页签** 33 高、内距 0×14、间距 14、字号 12，当前项 600 + 底部 2px `accent`。三个页签都没有计数徽章（04 的 `Changes 7` 带一个）。
- **body** 内距 10/10/12，先 8 行键值 `.kv`：10.5，左 `text2`、右 `text` 500 右对齐且可省略，行内距 2×4、左右间距 10。板上的键与值：`Commit` 是完整 22 位 SHA（mono）、`Parents` `8b41e07`、`Tree` `14 files · +248 −81`、`Refs` `main, origin/main`、`Signature` `Verified · SSH`（整值 `ok` 绿）、`Author` `Mia Chen <mia@gitui.dev>`、`Committer` `Mia Chen`、`Authored` `Jun 14, 11:42`。
- **Actions**：`statline`（10.5/600 `text2`、上距 12、下边框、下距 8）之后两排按钮，每排间距 6、第二排再上距 6。按钮 24 高、内距 0×10、圆角 6、12/500，四个都 `flex:1` 且居中，所以等宽四格。
- 末尾说明 10.5 `text3`、行高 1.5、上距 12：`Info shows commit metadata, parents, refs, signature, and actions. Changes/Files tabs keep the diff and file tree.`

## 5. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 检查器页签 | Changes / Files / Info | 只画 `Info` |
| 签名 | Verified / Unverified / 无签名 | 只画 `Verified · SSH` |
| 改动位置 | 第 n / m 处 | 只画 `2 of 7` |
| diff 布局 | Split / Unified | 只画 Split |
| 提交父子 | 单父 / 多父 / 根提交 | 只画单父 |

```
进入：04 检查器页签 `Info`（04 里也是 button，没有 href）
切换：Changes / Files / Info 只换右列；Split ⇄ Unified 只换渲染
离开：返回条 `< Commits` 回 04；工具条 `Changes 12` 去 10；`Agent` 去 12
```

## 6. 画板没覆盖的组合（待补设计）

1. **`Changes` / `Files` 两页签**：本板只画 `Info`，另两个在板上是 `button` 且没有 href；切过去以后中列的 diff 保留还是换掉、`Files` 是否等价于 18，都没有画。
2. **签名三态**：只有 `Verified · SSH` 一行绿字；`Unverified`、签名失败、无签名时这一行写什么、用什么颜色、是否整行隐藏都没有画。
3. **父提交数量**：`Parents` 只有一个短 SHA。合并提交两个父、根提交零个父时这一行怎么排（并列、逗号还是省略）没有画。
4. **动作结果**：`Open on GitHub` 在没有 remote 时是否禁用、`Cherry-pick` / `Revert` 的确认与冲突提示、`Copy SHA` 的反馈都没有画。

## 7. 按钮与链接清单

共 12 组；分段、页签、整行可点的行不计入，见本节末。

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 返回条 | `< Commits`（`aria-label=Back to commit list`） | chip 22 高 + 左箭头 10 | 回 04 | 常驻 |
| 2 | diff 工具条 | 折叠（`aria-label=Collapse all hunks`） | 图标 24×24 | 折叠全部 hunk | 无按下态 |
| 3 | diff 工具条 | 换行（`aria-label=Toggle word wrap`） | 图标 24×24 | 开关自动换行 | 无 `on` 态 |
| 4 | diff 工具条 | `¶`（`aria-label=Show whitespace`） | 图标按钮 | 显示空格和 tab | 状态强调色 |
| 5 | diff 工具条 | Filter（`aria-label=Ignore whitespace changes`） | 图标按钮 | 忽略空白差异 | 状态强调色 |
| 6 | diff 工具条 | `<`（`aria-label=Previous change`） | 图标 24×24 | 上一处改动 | 无 hunk 时禁用 |
| 7 | diff 工具条 | `>`（`aria-label=Next change`） | 图标 24×24 | 下一处改动 | 无 hunk 时禁用 |
| 8 | 每个 hunk 右端 | Copy Hunk | mini 17 高 | 复制该 hunk | 每 hunk 一个 |
| 9 | 检查器 Actions | Copy SHA | 按钮 24 高 | 复制完整 SHA | 常驻 |
| 10 | 检查器 Actions | Open on GitHub | 按钮 24 高 | 在浏览器打开该提交 | 板上无禁用态 |
| 11 | 检查器 Actions | Cherry-pick | 按钮 24 高 | 摘取到当前分支 | 常驻 |
| 12 | 检查器 Actions | Revert | 按钮 24 高 | 生成反向提交 | 常驻 |

- **分段**：diff 工具条 `Split / Unified`（`role=group`，当前项白底）；**页签**：检查器 `Changes / Files / Info` 三个都是 `button`，只有 `Info` 带 `aria-selected=true`。
- **窗口级按钮**（页签 `+`、齿轮、侧栏/检查器开关、Fetch/Pull/Push/Branch/Stash、`Changes 12`、`Agent`）与 04 一致，见 [workspace.md](workspace.md) §8。
- 板上没有 macOS 菜单栏；`2 of 7`、`+18`、`−7`、`14 files` 都是文本，不可点。

## 8. 文案要点

- 同一提交在板上有四个文件数：返回条 `7 files`、hashrow `14 files`、`Tree 14 files · +248 −81`、状态栏 `12 changes`（工作区）。前三者本应一致，画板自己没对齐，实现需要指定唯一来源。
- `2 of 7` 的 7 与返回条 `7 files` 是同一口径（提交内文件数）；`+18 −7` 是当前文件，`+248 −81` 是整提交。
- `Verified · SSH` 是「结论 · 签名类型」，不写签名者；`Author` 那一行才带邮箱。
- 说明句把页签分工写死：`Info` 只放元数据与动作，diff 与文件树留给 `Changes` / `Files`。

## 9. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 04 Commits | 窗口框架来自它；返回条 `< Commits` 回它（板内 `Workspace.dc.html`）；它的 `Info` 页签是本页的入口，但两边都没有 href |
| 10 Staging & commit / 12 Agent | 工具条 `Changes 12` / `Agent`（板内各一处 href） |
| 23 Settings / 01 Welcome | 齿轮 / 页签 `+`（板内各一处 href） |
| 18 Files at commit | 语义上对应 `Files` 页签，本板没有指向它的 href |
| 04b File diff (focused) | 共用同一套四格 diff 行结构，`2 of 7` 的写法也相同 |
