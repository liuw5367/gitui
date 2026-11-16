# 多提交 diff — 布局、状态与按钮

- **画板**：06 Multi-commit diff（Unified · 选中 3 个提交 · 有检查器；无单提交、无空选择、无同步横幅）
- **来源**：`designs/graft-git/project/DiffFocus.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

板内 `.window` 没有内联尺寸，取样式表的 `height:calc(100vh - 48px)`：预览 1440×900、桌面留白 24，窗口 **1392×852**。
与 04 相比没有授权横幅，主体 **742** = 852 − 2 − 40 − 44 − 24；中列 798，侧栏 232、检查器 360、状态栏 24。

## 1. 骨架

```
┌───────────────┬─────────────────────────────────────────────┬───────────────────────────────┐
│ ● ● ●  [gitui ×] [docs-site ×] [atlas-web ×] [+]     ● Fetched 2m ago   [gear]              │
├─────────────────────────────────────────────────────────────────────────────────────────────┤
│ [=] | Fetch Pull Push | Branch Stash   [search commits ⌘F] | History Changes 12 | Agent [=] │
├─────────────────────────────────────────────────────────────────────────────────────────────┤
│ Branches 5    │ < Commits  3f2a1c9 … c72d9a4                │ 3 commits selected            │
│ ● main * +2-0 │         [Compare with Working Tree]         │ 3  Mia Chen, Jonas Weber +1   │
│ ● feature/dif │ [Clear]   3 commits selected                │    Jun 12 - Jun 14   main     │
│ Remotes 2     │ [Squash…][Rebase…][Cherry-pick]             │ 8b41e07..3f2a1c9 · 12 files   │
│   origin 8    │ [Compare] [Clear]                           │  Changes 12  Files  Info      │
│ Tags 3        │ [3 commits] > src/ui/DiffView.tsx +31 -18   │ 12 files changed +227 -74     │
│ ▣ v1.4.0      │ Split [Unified] | [fold][wrap] | 7 of 12    │ src/git                       │
│ Stashes 2     │ [<] [>]                                     │  diff-engine.ts M +18 -7      │
│ Submodules 2  │ @@ -38,7 +38,11 @@  [Copy Hunk]             │  hunk.ts A +44 -0             │
│ Worktrees 3   │  38  38  export function DiffView(          │ src/ui                        │
│               │  41     - const split = mode                │  DiffView.tsx M +31 -18       │
│               │      41  + const split = ... || width       │  theme.ts M +34 -19           │
│               │  44  48  }                                  │ Export Patch  [copy] [more]   │
├─────────────────────────────────────────────────────────────────────────────────────────────┤
│ main   ● Comparing 3 commits  8b41e07..3f2a1c9   12 files · +227 -74   UTF-8                │
└───────────────┴─────────────────────────────────────────────┴───────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 返回条 | 中列顶部 | 高 34（内联样式）；内距 0×11；间距 8 | `< Commits`、两个 SHA、`3 of 128 commits`、右端两个 chip |
| 选择条 | 返回条下 | 高 34；内距 0×10；间距 6；`accent-soft` 底 | `3 commits selected` + 范围摘要 + 五个动作 |
| diff 工具条 | 选择条下 | 高 38；内距 0×11；间距 8 | `3 commits` 徽章 + 路径 + 增删、布局分段、三个开关、`7 of 12 files`、前后文件 |
| diff 主体 | 工具条下 | 剩 636 高；等宽 11、行高 19 | 单栏文件头、hunk、增删行、折叠行 |
| 检查器 | 主体右 | 宽 360；头内距 12/14/10 | 三个提交的合并摘要 + 12 个文件 |
| 侧栏 | 主体左 | 宽 232；内距 2/8/12 | 与 04 同一套六组引用，`main` 行仍是 `sel` |

三条竖排的高度：34 + 34 + 38 = 106，diff 主体 636 = 742 − 106。图中 `[gear]` 是齿轮，`[=]` 是侧栏/检查器开关，`[copy]`/`[more]` 是图标按钮，`[<]`/`[>]` 是前后文件。

## 2. 返回条

`panel` 底、下边框 1、内距 0×11。最左 `< Commits` chip（22 高，左箭头 10）→ SHA 徽章 `3f2a1c9` → 省略号 `…` →
SHA 徽章 `c72d9a4` → `3 of 128 commits`（10.5 `text3`）→ 弹性空隙 → `Compare with Working Tree` chip → `Clear` chip。
两个 SHA 之间用 `…`（三个点）而不是范围写法 `..`，这与 05/06 检查器里的 `8b41e07..3f2a1c9` 是两种记号：这里表达
「选区的两个端点」，那里表达「区间」。两个 chip 都是 22 高、内距 0×7、1px `line` 描边。

## 3. 选择条

`accent-soft` 底 + 下边框 1，内距 0×10、间距 6、字号 10。左侧 `3 commits selected`（600 `text`）+ 摘要
`8b41e07..3f2a1c9 · 12 files`（10.5 `text3`）；右侧五个动作依次 `Squash…`、`Rebase…`、`Cherry-pick`、`Compare`、
`Clear`。前四个是白底按钮、最后一个 `Clear` 是 ghost（透明底与描边）。这一条里的按钮被作用域压到**高 22、字号 9.5、
内距 0×7**——比常规 `btn`（24/12）小一号，是板上唯一这样缩的地方。

## 4. diff 区

- **工具条左侧** crumb 换成 `3 commits` 徽章（高 18、圆角 4、内距 0×6、`panel-2` 底 + 1px `line-soft`）+ 一个 10 的右箭头 + 路径 mono 11 + 增删 mono 9.5。
- **右侧**：`Split | Unified`（tiny 分段，`Unified` 白底——与 04b 相反）→ 三个 24×24 开关（折叠全部、自动换行、显示空白）→ `7 of 12 files`（10.5 等宽）→ `Previous file` / `Next file` 两个 24×24 箭头，**按文件**计。
- **文件头**只有一格（内联 `grid-template-columns:1fr`）：`a/` + 路径 + `→` + `b/` + 路径，9.5 `text3`、内距 4×9。
- **表**是三列：旧行号 42 + 新行号 42 + 代码。统一视图也保留**两列行号**，删除行的新行号格与新增行的旧行号格是 `.empty`；代码列前手写 `-` / `+` 号（`minus` / `plus` 色），不是靠底色区分。
- hunk 行与折叠行横跨三格（`colspan=3`），高 21 / 20，与 04b 同规格；hunk 右端也有 `Copy Hunk` mini。
- 词级高亮只出现一次（新增侧 `width > 720`）；底色、语法色、行高都与 04b 共用同一套。

## 5. 检查器

标题换成 `3 commits selected`（13/600）；作者位是一个圆形 `3`（21、`avatar` 样式），右边 `Mia Chen, Jonas Weber +1`
（11.5/600）+ `Jun 12 – Jun 14`（10），再右边放 `main` 分支徽章；下一行是 `8b41e07..3f2a1c9`（hash 徽章）`·` `12 files`。
页签 `Changes 12 / Files / Info`——三个都是 `button`，板上没有链接（`Files` 不跳 18）。统计行只有
`12 files changed +227 −74`，**没有** 树/平铺图标对；4 个文件组 12 个文件，**没有** `Commit details` 段；
页脚是 `Export Patch`（小按钮 22 + 外链 12）+ `Copy combined diff` + `More actions`（各 24×24）。

## 6. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 选中提交数 | 1 / n / 0 | 只画 n = 3 |
| 范围 | 连续区间 / 不连续 | 只画连续（`8b41e07..3f2a1c9`），不连续怎么写没画 |
| diff 布局 | Split / Unified | 只画 Unified |
| 文件定位 | 第 n / m 个 | 只画 `7 of 12 files` |
| 检查器页签 | Changes / Files / Info | 只画 Changes |

```
进入：04 过滤条 `Compare`（板上直达本板）；或在 04 里多选后进来（多选动作没画）
切换：Split ⇄ Unified 只换渲染        Compare 按钮 → 07        Clear → 回未选择状态
离开：返回条 `< Commits` 回 04
```

## 7. 画板没覆盖的组合（待补设计）

1. **怎么进入多选**：04 里 `⌘`/`⇧` 点选、`⇧↑↓` 扩选、以及选中后过滤条如何让位给选择条，都没有板；本板直接以「3 commits」开场。
2. **单提交与空选择**：只选 1 个时落在 04b 还是本板、`Clear` 之后回到哪（04？还是选中第一条？）没有画。
3. **三个动作的后续**：`Squash…`、`Rebase…`、`Cherry-pick` 只画了按钮，确认弹窗、冲突与结果页都没有板。
4. **对工作区比较**：`Compare with Working Tree` 打开后返回条写什么（工作区没有 SHA）没画。
5. **merge / 空 diff**：选中含 merge commit 时以哪侧为基准、12 个文件全为二进制时 diff 区长什么样，没有板。

## 8. 按钮与链接清单

共 11 组；分段、页签、整行可点的行不计入，见本节末。

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 返回条 | `< Commits`（`aria-label=Back to commit list`） | chip 22 高 | 回 04 | 常驻 |
| 2 | 返回条 | Compare with Working Tree | chip 22 高 | 与工作区比较 | 板上无目标页 |
| 3 | 返回条 | Clear | chip 22 高 | 清空选择 | 与选择条 `Clear` 重复 |
| 4 | 选择条 | Squash… / Rebase… / Cherry-pick | 按钮 22 高、9.5 | 批量改写历史 | 常驻，板上都不禁用 |
| 5 | 选择条 | Compare | 按钮 22 高、9.5 | 打开 07 | 常驻 |
| 6 | 选择条 | Clear | ghost 按钮 22 高 | 清空选择 | 常驻 |
| 7 | diff 工具条 | Split / Unified | 分段两项 | 切渲染 | 当前项白底：`Unified` |
| 8 | diff 工具条 | 折叠 / 换行 / 空白（`aria-label=Collapse all hunks` / `Toggle word wrap` / `Show whitespace`） | 图标 24×24 | 视图开关 | 板上都无 `on` 态 |
| 9 | diff 工具条 | `<` / `>`（`aria-label=Previous file` / `Next file`） | 图标 24×24 | 上一个/下一个文件 | 板上常亮 |
| 10 | 每个 hunk | Copy Hunk | mini 17 高 | 复制该 hunk | 每个 hunk 一个 |
| 11 | 检查器页脚 | Export Patch / 复制（`Copy combined diff`）/ `⋯`（`More actions`） | 小按钮 22 + 两个图标 24 | 导出补丁、复制合并 diff、更多 | 板上没有展开菜单 |

- **窗口级按钮**（页签 `+`、齿轮、侧栏/检查器开关、Fetch/Pull/Push/Branch/Stash/Agent）与 04 一致，见 [workspace.md](workspace.md) §8。
- **页签**：检查器 `Changes 12 / Files / Info` 三个都是按钮（`role=tablist`），板上没有指向 18；**整行可点**：12 行文件（当前 `DiffView.tsx`）。

## 9. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 04 Commits | 返回条回它；它过滤条的 `Compare` 进入本板 |
| 07 Compare refs | 选择条的 `Compare` 指向它（两个 ref 的比较） |
| 04b File diff (focused) | 单提交时看同一套行结构（那是 Split、单文件） |
| 23 Settings / 01 Welcome | 齿轮 / 页签 `+` |
