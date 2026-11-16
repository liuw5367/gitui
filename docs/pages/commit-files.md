# 提交内文件 — 布局、状态与按钮

- **画板**：18 Files at commit（有选中 `diff-engine.ts` · 有平铺分组与 `142 files` · 有 `4 empty folders merged` 注记；无树视图、无二进制文件、无空提交）
- **来源**：`designs/graft-git/project/CommitFiles.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

本板是**主窗口换中列与右列**：标题栏 40、工具条 44、侧栏 232、状态栏 24 与 04 相同；中列 798 拆成「文件条 38 + 只读代码区 704」，右列 360 换成 `Files` 页签的仓库文件清单。

## 1. 骨架

```
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ ● ● ●  [gitui x] [docs-site x] [atlas-web x] [+]     ● Fetched 2m ago  [gear]                │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ [=] Fetch Pull Push | Branch Stash   [search ⌘F] | History Changes 12 | Agent [=]            │
├────────────────┬──────────────────────────────────────────────────┬──────────────────────────┤
│ Branches 5     │ [< History] 3f2a1c9 / src/git/diff-engine.ts     │ Fix diff hunk alignment  │
│ ▣ main ✓ ↑2 ↓0 │                       [Source|Blame] [wrap]      │ on wrapped lines         │
│ ▣ feature/agen │                       [ Open in Editor ]         │ MC Mia Chen 2 hours ago  │
│ ▣ fix/wrappe   │ 120  export function alignHunks(                 │ 3f2a1c9 [copy]           │
│ ▣ release/1.4  │ 121    a: string[],                              │ main · 7 files           │
│ Remotes 2      │ 122    b: string[],                              │ Changes 7  Files  Info   │
│   origin 8     │ 123    width = 3,                                │ Repository at 3f2a1c9    │
│ Tags 3         │ 124    maxWidth = 96,                            │ 142 files    [tree][flat]│
│ ▣ v1.4.0       │ 125  ) {                                         │ 4 empty folders merged   │
│ Stashes 2      │ 126    const rows = lcs(a, b)                    │ src/git                  │
│ Submodules 2   │ 127    let cursor = 0                            │   diff-engine.ts         │
│ Worktrees 3    │ 136    const wrapped = isWrapped(row             │   hunk.ts                │
│                │ 137      row.before = span.slice(0,              │ src/ui                   │
│                │ 138    }                                         │   DiffView.tsx           │
│                │ 140    return rows  (137 selected)               │ [ Copy Path ]   [c] [.]  │
├────────────────┴──────────────────────────────────────────────────┴──────────────────────────┤
│ main  Viewing 3f2a1c9  ● File content at commit   UTF-8  LF  TypeScript                      │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 文件条 | 中列顶 | 高 38；内距 0×11；gap 8 | 返回 chip、路径面包屑 + `At commit`、`Source/Blame`、换行、`Open in Editor` |
| 代码区 | 文件条下 | 高 704；mono 11 / 行高 19；行号列 46 | 35 行只读源码（120–154），第 137 行整行高亮 |
| 检查器头 | 右列顶 | 内距 12/14/10 | 提交标题 13/600、作者行、sha 徽章、`Committed on` 行 |
| 页签 | 头下 | 高 33；内距 0×14；gap 14 | `Changes 7` / `Files`（当前）/ `Info` |
| 文件清单 | body | 内距 10/10/12；组头 23、行 27 | 统计行、注记、4 组 10 个文件、`File` 明细 |
| 检查器脚 | 右列底 | 内距 8×12；gap 7 | `Copy Path`、复制 SHA、`⋯` |
| 状态栏 | 底部通栏 | 高 24；内距 0×12；字号 11 | `main`、`Viewing 3f2a1c9`、`File content at commit`、路径、编码、换行、语言 |

代码区 704 = 742 − 38；代码列 752 = 798 − 46，按 19 的行高可容 37 行，板上写 35 行。

## 2. 文件条

左端 `History` 返回 chip（22 高、11 号、chevron 10）链 04。面包屑 `filepathbar` 10.5、gap 5：sha 徽章 `3f2a1c9`（mono 10.5、`panel-2` 底、1px `line-soft`、内距 2×5、圆角 4）→ `/` → 目录段 `src` `git`（`text3`）→ 文件名 `diff-engine.ts`（mono 11、`text`）→ `At commit` 徽章（18 高、内距 0×6、圆角 4、9.5）。

右端依次：`Source` / `Blame` 分段（`.seg.tiny`：容器内距 2、项 21 高、10.5，`Source` 白底为当前项；`Blame` 是链到 19 的 `<a>`，被内联改成 18 高 / 9.5，比 `Source` 矮 3）、换行 24×24（`Toggle word wrap`）、`Open in Editor`（22 高、11 号 + 外链图标 12）。

## 3. 代码区

`.codetable` 100% 宽、`table-layout:fixed`、mono 11 / 行高 19。行号列 46 宽（内距 0 9 0 4、右对齐、9.5 `text3`、右边框 1 `line-soft`、不可选中）；代码列内距 0 14、`white-space:pre`。行 hover 整行 `hover` 底；第 137 行整行 `sel` 底、行号转 `accent` 600。

着色只用五个 token：关键字 `code-keyword`、字符串 `code-string`、数字 `code-number`、类型 `code-type`、函数名 `code-fn`。代码区没有 gutter 里的增删标记，也没有折叠条——它是提交里的文件内容，不是 diff。

## 4. 检查器

**头**：标题 13/600 行高 1.32；作者行上距 9（头像 21、`MC` 9/700；姓名 11.5/600 与 `2 hours ago` 10 两行；右端 sha 徽章 + 复制图标 10）；`Committed on` 行上距 9、10.5 `text3`：`main` 分支徽章（16 高、圆角 4、9.5/600）+ `·` + `7 files`。

**页签** 33 高、gap 14、字号 12：当前项 600 + 底部 2px `accent`；`Changes` 带计数 9.5（`panel-2` 底、内距 0×4、行高 14）。

**body**：统计行 10.5（`Repository at 3f2a1c9` 600 + `142 files`），下框 `line-soft`、下距 8，右端树/平铺图标对（22×18，`Flat` 当前）；注记 `4 empty folders merged into their parents`（9.5、内距 5×7、`panel-2`、圆角 5、8×8 紫方块、上距 5）；组头 23 高、上距 3（chevron 10 + 文件夹 12 + 路径 10.5/600 `text3`）；文件行 27 高、内距 0×7、圆角 5、gap 6（文件图标 13 `text3`、名字 mono 10.5、选中行 `sel` 底且图标转 `accent`）。

文件行**没有状态字母和 `+18/−7`**，04 的同一列表有；`File` 段上距 13、上框、上内距 9：标签 9.5/600 大写字距 .04em，两行键值 10.5（`Lines 214`、`Encoding UTF-8 · LF`）。

**脚**：`Copy Path`（22 高、复制图标 12）+ 弹性空隙 + 复制 SHA 24×24 + `⋯` 24×24。

## 5. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 视图模式 | 树 / 平铺 | 只画平铺（`Flat list view` 当前） |
| 文件选中 | 一个 / 无 | 只画 `diff-engine.ts` |
| 代码视图 | Source / Blame | 只画 Source |
| 折行 | 开 / 关 | 只画关（按钮无按下态） |
| 检查器页签 | Changes / Files / Info | 只画 Files |
| 代码行选中 | 有 / 无 | 只画第 137 行 |

```
进入：04 检查器的 Files 页签 → 本板（Workspace.dc.html 的 1 处 href）
切换：Source ⇄ Blame 换中列；Tree ⇄ Flat 换右列分组；点文件行换中列内容
离开：History chip / Changes 7 → 04；Blame → 19
```

## 6. 画板没覆盖的组合（待补设计）

1. **树视图**：`Tree view` 按钮画了但没内容，分组展开/收起、目录层级缩进都不清楚。
2. **文件内容为空或二进制**：空文件、二进制、超长行折行、`wrap` 按下态都没有。
3. **改动统计缺失**：commit 的文件列表里没有 `M/A/D` 与增删行数，与 04 的列表口径不同，没说明是否故意。
4. **两套文件数**：检查器头写 `7 files`、统计行写 `142 files`（整仓），关系没写。
5. **`At commit` 与 `Repository at 3f2a1c9`**：同一事实两处措辞，且面包屑里的 sha 与提交标题的 sha 没有联动说明。
6. **代码区的动作**：除 `Open in Editor` 与 `Copy Path` 外，没有选中行复制、跳行、搜索。
7. **返回文案**：本板 chip 写 `History`，19 的同一动作写 `Commits`。

## 7. 按钮与链接清单

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 标题栏 | `+` / 齿轮 / 侧栏与检查器开关 | 图标 27×27、26×24 | 回 01 / 去 23 / 收起两栏 | 常驻，无按下态 |
| 2 | 工具条 | Fetch / Pull / Push / Branch / Stash | `tbtn` 26 高 | 同步与建引用 | 板上都不禁用 |
| 3 | 工具条 | `History` / `Changes 12` | 分段项 22 高 | 切仓库视图 | `History` 当前 |
| 4 | 工具条 | `Agent` | `tbtn` 26 高 + `accent-soft` | 去 12 | 本板是 `<button>` |
| 5 | 文件条左 | `History` | chip 22 高 | 回 04 | 常驻 |
| 6 | 文件条右 | `Source` / `Blame` | 分段项 21 / 18 高 | 源码 / 逐行追溯 | `Blame` 链 19 |
| 7 | 文件条右 | 换行（`aria-label=Toggle word wrap`） | 图标 24×24 | 代码折行 | 无按下态 |
| 8 | 文件条右 | `Open in Editor` | `btn small` 22 高 + 外链 12 | 外部编辑器打开 | 常驻 |
| 9 | 检查器页签 | `Changes 7` | 页签 12 | 回 04 | 带计数，链 04 |
| 10 | 检查器页签 | `Files` / `Info` | 页签 12 | 本板 / 提交信息 | `Files` 当前 |
| 11 | 统计行右 | 树 / 平铺（`aria-label=Tree view` / `Flat list view`） | 图标对 22×18 | 换分组 | 当前项白底 |
| 12 | 检查器脚 | `Copy Path` | `btn small` 22 高 | 复制选中文件路径 | 常驻 |
| 13 | 检查器脚 | 复制（`aria-label=Copy commit SHA`） | 图标 24×24 | 复制 sha | 常驻 |
| 14 | 检查器脚 | `⋯`（`aria-label=More commit actions`） | 图标 24×24 | 更多提交动作 | 板上无展开菜单 |

- **整行可点**：10 个文件行（选中行 `aria-current="true"`）与 4 个组头；35 行代码只 hover 高亮，不可点。
- 三个仓库页签是整块可点的按钮，`aria-label` 形如 `gitui repository`，不计入上表。

## 8. 文案要点

- 面包屑把 sha 当第一段路径，读法是「哪个提交里的哪个文件」，`At commit` 再确认一次。
- 注记用 `4 empty folders merged into their parents` 解释为什么分组数比目录数少；04 的同一现象写成 `Empty folder merged · src/git/legacy`，两种句式并存。
- 状态栏把「看的是什么」写全：`Viewing 3f2a1c9` + `File content at commit`，避免与工作区文件混淆。
- 文件段的键值只留 `Lines` 与 `Encoding`，没有 `Language`，语言只在状态栏出现。

## 9. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 04 Commits | `History` chip、`Changes 7` 页签共 2 处 `href`；本板是它 `Files` 页签的去处 |
| 19 Blame & file history | `Blame` 分段项（唯一入链到 19 的板） |
| 10 Staging & commit | `Changes 12` 分段项链到它 |
| 12 Agent panel / 23 Settings / 01 Welcome | `Agent` / 齿轮 / 页签 `+` |
| 04b File diff、05 History filters、14 Merge conflict、15 Sync & auth、20 Command palette、28/29 Sync | 这 7 块板的文件入口都 `href` 到本板；加上 04 与 19，全仓共 9 块板、各 1 处入链 |
