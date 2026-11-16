# 引用比较 — 布局、状态与按钮

- **画板**：07 Compare refs（`2-dot` · Unified · `main ↔ feature/diff-view`；无 `3-dot`、无 ref 选择器、无 0 差异）
- **来源**：`designs/graft-git/project/CompareRefs.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

板内 `.window` 没有内联尺寸，取样式表的 `height:calc(100vh - 48px)`：预览 1440×900、桌面留白 24，窗口 **1392×852**。
没有授权横幅也没有选择条，主体 **742** = 852 − 2 − 40 − 44 − 24；中列 798，侧栏 232、检查器 360、状态栏 24。

## 1. 骨架

```
┌───────────────┬─────────────────────────────────────────────┬───────────────────────────────┐
│ ● ● ●  [gitui ×] [docs-site ×] [atlas-web ×] [+]     ● Fetched 2m ago   [gear]              │
├─────────────────────────────────────────────────────────────────────────────────────────────┤
│ [=] | Fetch Pull Push | Branch Stash   [search commits ⌘F] | History Changes 12 | Agent [=] │
├─────────────────────────────────────────────────────────────────────────────────────────────┤
│ Branches 5    │ < Commits  Compare  [main] ⇄ [feature]      │ main ⇄ feature/diff-view      │
│ ● main * +2-0 │ 12 commits ahead · 0 behind                 │ 3  12 commits ahead           │
│ ● feature/dif │ [2-dot] 3-dot             [Swap]            │ 0 behind · 14 files   2-dot   │
│ Remotes 2     │ [12 commits] > src/ui/DiffView.tsx          │ 3f2a1c9...feature/diff        │
│   origin 8    │   +31 -18                                   │  Changes 14  Files  Info      │
│ Tags 3        │ Split [Unified] | [fold][wrap] | 7 of 14    │ 14 files changed +248 -81     │
│ ▣ v1.4.0      │ [<] [>]                                     │ src/git                       │
│ Stashes 2     │ a/ -> b/ src/ui/DiffView.tsx                │  diff-engine.ts M +18 -7      │
│ Submodules 2  │ @@ -38,7 +38,11 @@  [Copy Hunk]             │ src/ui                        │
│ Worktrees 3   │  38  38  export function DiffView(          │  DiffView.tsx M +31 -18       │
│               │  41     - const split = mode                │  CommitPanel.tsx M +42 -17    │
│               │      41  + const split = ... || width       │ tests                         │
│               │  44  48  }                                  │ Export Patch  [copy] [more]   │
├─────────────────────────────────────────────────────────────────────────────────────────────┤
│ main   ● Comparing 12 commits  8b41e07..3f2a1c9   14 files · +248 -81   UTF-8               │
└───────────────┴─────────────────────────────────────────────┴───────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 比较条 | 中列顶部 | 高 40（`.hist-bar` 内联改高，常规 32）；内距 0×10；间距 7 | `< Commits`、`Compare` 标题、两个 ref chip、`↔`、ahead/behind、模式分段、`Swap` |
| diff 工具条 | 比较条下 | 高 38；内距 0×11；间距 8 | `12 commits` 徽章 + 路径 + 增删、布局分段、三个开关、`7 of 14 files`、前后文件 |
| diff 主体 | 工具条下 | 剩 664 高；等宽 11、行高 19 | 单栏文件头、hunk、增删行、折叠行 |
| 检查器 | 主体右 | 宽 360；头内距 12/14/10 | 两端 ref 的摘要 + 14 个文件 |
| 侧栏 | 主体左 | 宽 232；内距 2/8/12 | 与 04 同一套六组引用（`main` 行 `sel`） |
| 状态栏 | 底部通栏 | 高 24；内距 0×12；11 | `● Comparing 12 commits`、`8b41e07..3f2a1c9`、`14 files · +248 −81` |

比较条 40 + 工具条 38 = 78，diff 主体 664 = 742 − 78。图中 `⇄` 是板上的 `↔`，`[gear]` 是齿轮，`[=]` 是侧栏/检查器开关，`[copy]`/`[more]` 是图标按钮。

## 2. 比较条

一行放完全部「比较什么」：`< Commits` chip（22 高、左箭头 10）→ `Compare` 标题（11.5/600）→ 左端点 chip
（分支图标 10 + `main` + chevron 9）→ 一个 10 的 `↔`（`text3`，内距 0×1）→ 右端点 chip（`feature/diff-view`）→
`12 commits ahead · 0 behind`（10.5 `text3`）→ 弹性空隙 → `2-dot | 3-dot` 分段（tiny：项高 21、内距 0×7、10.5，
`2-dot` 白底）→ `Swap` chip（22 高）。板上两个端点 chip 的形态与 04 的分支 chip 完全一致，但**都直接链回 04**——
点它们本该打开 ref 选择器，而板上没有那个浮层。

## 3. diff 区与检查器

- **工具条**：`12 commits` 徽章（18 高、圆角 4、内距 0×6、`panel-2` 底 + 1px `line-soft`）→ 10 的右箭头 → 路径 mono 11 → 增删 mono 9.5；右侧 `Split | Unified`（`Unified` 白底）、三个 24×24 开关、`7 of 14 files`（按文件计）、前后文件箭头。
- **表**：与 06 同一套三列（旧行号 42 + 新行号 42 + 代码），hunk 与折叠行 `colspan=3`，行高 19、hunk 21、折叠 20，词级高亮只出现一次（新增侧 `${density}`）。
- **检查器**：标题 `main ↔ feature/diff-view`（两端点名，13/600）；作者位是圆形 `3`（21），右边 `12 commits ahead`（11.5/600）+ `0 behind · 14 files`（10），再右边用分支徽章承载模式 `2-dot`；下一行 `3f2a1c9...feature/diff-view`（hash 徽章，注意是**三个点**）+ `·` + `14 files`。
- **页签与页脚**：`Changes 14 / Files / Info` 三个都是 `button`（不跳 18）；统计行 `14 files changed +248 −81`，没有树/平铺图标对；4 个文件组 14 个文件，没有 `Commit details` 段；页脚 `Export Patch` + `Copy combined diff` + `More actions`。

## 4. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 比较模式 | 2-dot / 3-dot | 只画 2-dot |
| 左端点 | 分支 / 标签 / 提交 / 工作区 | 只画分支 `main` |
| 右端点 | 同上 | 只画分支 `feature/diff-view` |
| 方向 | `main` 领先 / 落后 | 只画 ahead 12、behind 0 |
| diff 布局 | Split / Unified | 只画 Unified |

```
进入：06 选择条的 `Compare`（板上唯一的 href 入口）
切换：2-dot ⇄ 3-dot 只换比较语义        Swap 反转两端（ahead/behind 文案是否互换没画）
离开：比较条 `< Commits` 回 04
```

## 5. 画板没覆盖的组合（待补设计）

1. **3-dot**：以 merge-base 为第三端点时，比较条与检查器怎么写（多一个端点 chip、`↔` 变两段）没有画。
2. **ref 选择器**：点端点 chip 打开的浮层（分支 / 标签 / 提交 / stash / 工作区）没有板——板上两个 chip 直接链回 04。
3. **Swap 之后**：ahead/behind 是否互换、`12 commits ahead` 与 `0 behind` 的位置是否镜像，没有画。
4. **退化态**：两端无共同祖先（unrelated histories）、两端相同（`0 commits` / 空 diff）时的提示没有画。

## 6. 文案要点

同一块板上出现三种范围记号，不能混用：`8b41e07..3f2a1c9` 是**两点**，表示「区间」（摘要、状态栏、返回条摘要行都用它）；`3f2a1c9...feature/diff-view` 是**三点**，表示「以 ref 为端点的关系」（检查器 hash 徽章，左侧是提交、右侧是分支）；返回条里 `3f2a1c9 … c72d9a4` 用**省略号**，表示多选选区的两个端点。ahead/behind 一律写成 `12 commits ahead · 0 behind` 的「先 ahead 后 behind」，检查器里拆成两行时把 `0 behind · 14 files` 合并到第二行。

## 7. 按钮与链接清单

共 9 组；分段、页签、整行可点的行不计入，见本节末。

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 比较条 | `< Commits`（`aria-label=Back to commit list`） | chip 22 高 | 回 04 | 常驻 |
| 2 | 比较条 | `main` | chip 22 高 + 分支图标 10 + chevron 9 | 换左端点（板上回 04） | 常驻 |
| 3 | 比较条 | `feature/diff-view` | 同上 | 换右端点（板上回 04） | 常驻 |
| 4 | 比较条 | Swap | chip 22 高 | 反转两端 | 常驻 |
| 5 | 比较条右 | 2-dot / 3-dot | 分段两项（`role=group`） | 切比较语义 | 当前项白底：`2-dot` |
| 6 | diff 工具条 | Split / Unified | 分段两项 | 切渲染 | 当前项白底：`Unified` |
| 7 | diff 工具条 | 折叠 / 换行 / 空白（`aria-label=Collapse all hunks` / `Toggle word wrap` / `Show whitespace`） | 图标 24×24 | 视图开关 | 都无 `on` 态 |
| 8 | diff 工具条 | `<` / `>`（`aria-label=Previous file` / `Next file`） | 图标 24×24 | 上一个/下一个文件 | 板上常亮 |
| 9 | 检查器页脚 | Export Patch / 复制（`Copy combined diff`）/ `⋯`（`More actions`） | 小按钮 22 + 两个图标 24 | 导出补丁、复制合并 diff、更多 | 没有展开菜单 |

- **窗口级按钮**（页签 `+`、齿轮、侧栏/检查器开关、Fetch/Pull/Push/Branch/Stash/Agent）与 04 一致，见 [workspace.md](workspace.md) §8；本页**没有**授权横幅与选择条。
- **页签**：检查器 `Changes 14 / Files / Info` 三个都是按钮；**整行可点**：14 行文件（当前 `DiffView.tsx` 带 `aria-current=true`）。
- `↔` 是文本不是按钮，点击不做事。

## 8. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 06 Multi-commit diff | 它的选择条 `Compare` 指向本板；两板共用同一套 diff 行结构 |
| 04 Commits | 比较条 `< Commits` 回它；两个端点 chip 也误链回它（选择器未画） |
| 18 Files at commit | 检查器 `Files` 页签在本板是死按钮 |
| 23 Settings / 01 Welcome | 齿轮 / 页签 `+` |
