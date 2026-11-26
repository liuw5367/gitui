# 逐行追溯与文件历史 — 布局、状态与按钮

- **画板**：19 Blame & file history（有第 22 行选中 · 有 6 条文件历史 · 有 `16 lines · last change 2h ago`；未展示 Diff 内容、过滤弹层与重命名追踪）
- **来源**：`designs/graft-git/project/BlameHistory.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

本板是**主窗口换中列与右列**：标题栏 40、工具条 44、侧栏 232、状态栏 24 与 04 相同；中列 798 拆成「顶栏 34 + blame 代码区 708」，右列 360 换成文件历史列表。

## 当前实现

文件 History 右侧列表在行组件内距之外增加 8pt 左右留白，条目之间留 4pt 间隔；每条的第二行依次显示短 SHA、作者和时间，点击历史项仍切换该文件的 revision。

## 1. 骨架

```
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ ● ● ●  [gitui x] [docs-site x] [atlas-web x] [+]     ● Fetched 2m ago  [gear]                │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ [=] Fetch Pull Push | Branch Stash   [search ⌘F] | History Changes 12 | Agent [=]            │
├──────────────┬──────────────────────────────────────────────┬────────────────────────────────┤
│ Branches 5   │ [< Commits] src/git/diff-engine.ts           │ Changes 7  History      [*]    │
│ ▣ main ✓ ↑2  │ 3f2a1c9  [Blame|Diff] [fold] [wrap]          │ diff-engine.ts  6 commits      │
│ ▣ feature/   │ 16 lines · last change 2h ago                │ Fix diff hunk alignment        │
│ ▣ fix/wrappe │ e19a3f2 Mia Chen   18 export func            │   3f2a1c9 · Mia Chen     2h    │
│ Remotes 2    │ e19a3f2 Mia Chen   19   const hunk           │ Add split/unified toggle       │
│   origin 8   │ e19a3f2 Mia Chen   20   let start            │   a37c6b1 · Jonas Weber  2h    │
│ Tags 3       │ a37c6b1 Jonas Weber 21   for (let            │ Cache parsed diffs             │
│ ▣ v1.4.0     │ 3f2a1c9 Mia Chen   22   if (isChan           │   b81c4a7 · Priya Nair   4h    │
│ Stashes 2    │ 3f2a1c9 Mia Chen   23     hunks.pus          │ Extract hunk builder           │
│ Submodules   │ 3f2a1c9 Mia Chen   24   }                    │   e19a3f2 · Mia Chen     1d    │
│ Worktrees 3  │ b81c4a7 Priya Nair  25 }                     │ Handle empty diff input        │
│              │ b81c4a7 Priya Nair  26   return hu           │   9c0b1e4 · Sam Okafor   2d    │
│              │ e19a3f2 Mia Chen   27 }                      │ Initial diff engine            │
│              │ 9c0b1e4 Sam Okafor  29 function m            │   77e2a90 · Mia Chen     3d    │
│              │ 77e2a90 Mia Chen   31   return {             │ [ Open in Editor ]  [c]        │
├──────────────┴──────────────────────────────────────────────┴────────────────────────────────┤
│ main  ● 2 ahead · 0 behind      12 changes  UTF-8  LF  TypeScript                            │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 顶栏 | 中列顶 | 高 34（内联）；内距 0×11；gap 8 | `Commits` 返回 chip、路径、sha 徽章、`Blame/Diff`、折叠、换行、一句范围说明 |
| blame 区 | 顶栏下 | 高 708；mono 10 / 行高 18；上下内距 5 | 16 行，每行 = gutter 126 + 行号 32 + 源码 |
| 检查器页签 | 右列顶 | 高 33；内距 0×14（右 8）；gap 14 | `Changes 7`（链 18）/ `History`（当前）/ 漏斗 24×24 |
| 路径条 | 页签下 | 内距 9×10；下框 1 | 文件图标 12 + 路径 mono 10 + `6 commits` |
| 文件历史 | 路径条下 | 行 40 × 6；内距 0×10 | 标题 11 + 次行 9（sha · 作者）+ 右端日期 9 |
| 检查器脚 | 右列底 | 内距 8×12；gap 7 | `Open in Editor`、复制路径 |
| 状态栏 | 底部通栏 | 高 24；内距 0×12；字号 11 | 与 04 相同（`● 2 ahead · 0 behind`、`12 changes`、`UTF-8`、`LF`、`TypeScript`） |

blame 区 708 = 742 − 34；源码列 640 = 798 − 126 − 32，去掉左右内距 20 后可用 620。

## 2. 顶栏

底 `panel`、下框 1、高 34，比 18 的文件条矮 4。左端 `Commits` 返回 chip（22 高、11 号、chevron 10、`aria-label="Back to commit list"`）链 04；路径 `src/git/diff-engine.ts` 用的是 `fp` 类，但样式表只有 `.filepathbar .fp`（mono 11）与 `.blamebar .fp`（mono 10.5）两条后代规则，本板顶栏不在其中，因此它按正文 13 号渲染；后面是 sha 徽章 `3f2a1c9`（mono 10.5、`panel-2` 底、内距 2×5、圆角 4）。

右端：`Blame` / `Diff` 分段（`.seg.tiny`：容器内距 2、项 21 高、10.5，`Blame` 白底为当前项；`Diff` 是 `<button>`，没有链接目标）、`tb-sep` 1×18、折叠 24×24（`Collapse all hunks`）、换行 24×24（`Toggle word wrap`）、`tb-sep`、`16 lines · last change 2h ago`（10.5 `text3` 等宽数字）。

折叠按钮是从 diff 工具栏搬过来的，blame 没有 hunk，按下后无对象可变。

## 3. blame 代码区

`blamecode` 底 `content`、上下内距 5、mono 10 / 行高 18。每行 `blame-line` 是 flex，最小高 18，左侧 2px 透明边；选中行换成 `sel` 底 + 2px `accent` 左边。

三段固定宽度：**gutter 126**（内距 0×8、gap 5、右边框 1、8.5 号：sha 用 mono `text2`、作者用 `text3`，超出省略）、**行号 32**（右对齐、右内距 8、9 号 `text3`、右边框 1）、**源码**（内距 0×10、`white-space:pre`、`text2`）。着色只两种：关键字 `purple`、字符串 `ok`，其余 `text2` + 一个更淡的 `text3` 变体。

板上 16 行来自 6 个提交：`e19a3f2`（18–20、27–28）、`a37c6b1`（21）、`3f2a1c9`（22–24）、`b81c4a7`（25–26）、`9c0b1e4`（29–30）、`77e2a90`（31–33）。第 22 行选中。

## 4. 右侧文件历史

页签 33 高、gap 14：`Changes 7` 链 18、`History` 当前（600 + 2px `accent`）、弹性空隙、漏斗 24×24（`Filter file history`，图标 12）。

路径条：文件图标 12 `text3` + 路径 mono 10 `text2`（可省略）+ 右端 `6 commits` 10.5。

六行 `fhist-row` 用内联网格 `1fr auto`、高 40、内距 0×10，每行两行文字：标题 11（`text`，单行省略）+ 次行 9（`fhist-hash` 9 号 mono + `·` + 作者名）；右端日期 9 号（`2h` `2h` `4h` `1d` `2d` `3d`）。选中行 `sel` 底 + 2px `accent` 左边。

样式表里的 `fhist-row` 是 4 列 32 高的表格行（`58 / 1fr / 70 / 42`），板上改用两行 40 高的卡片式行；两套规格并存，`fhist-cols` 列头在本板没有出现。

## 5. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 中列视图 | Blame / Diff | 只画 Blame |
| 行选中 | 一个 / 无 | 只画第 22 行 |
| 检查器页签 | Changes / History | 只画 History |
| 折行 | 开 / 关 | 只画关（按钮无按下态） |
| 历史选中 | 一条 / 无 | 只画第 1 条 `3f2a1c9` |

```
进入：18 的 Blame 分段项 → 本板（CommitFiles.dc.html 的唯一入链）
切换：Blame ⇄ Diff 换中列；点历史行按该 revision 重算 blame，点源码行聚焦右侧 commit
离开：Commits chip → 04；Changes 7 → 18
```

## 6. 画板没覆盖的组合（待补设计）

实现采用以下明确行为，画板本身没有覆盖这些状态：

- **Diff 视图**：`Diff` 切换在中栏用共享 `DiffView` 显示当前文件版本的提交差异；保留并排/合并、折叠、换行与复制 patch / hunk。
- **历史与 blame 的联动**：点文件历史行会按该提交和当时的文件路径重新读取整份 blame。点 blame 行会选中右侧对应提交，并保留当前文件版本和滚动位置。 纵向滚动可从源码或归属列发起，另一列同步当前阅读位置。
- **revision 边界**：从 History 检查器进入时用选中提交 SHA；从工作区 Diff 进入时用 `HEAD`。历史列表按该 revision 截断并使用 `--follow` 跟踪重命名。选择删除文件的提交时，blame 内容读该提交的第一父版本。

尚未覆盖的设计组合：

1. **范围之外的行**：只画 18–33 共 16 行，向上/向下滚动、文件首尾的边界行没有。
2. **blame 选项**：忽略空白（`-w`）和按提交合并相邻行都没画；重命名跟踪由 Git `--follow` 提供。
3. **文件历史过滤**：画板漏斗没有展开态与筛选规则，当前不显示这个无行为控件。
4. **长名字**：gutter 固定 126，作者名与 sha 各占多少、如何省略没写；`77e2a90` 这类 7 位 sha 正好占满。
5. **空历史 / 单行文件**：`0 commits` 或只有一行时的样子没画。
6. **路径字号**：本板 `fp` 落到 13 号正文，18 的同一路径是 mono 11，两页不一致。

## 7. 按钮与链接清单

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 标题栏 | `+` / 齿轮 / 侧栏与检查器开关 | 图标 27×27、26×24 | 回 01 / 去 23 / 收起两栏 | 常驻，无按下态 |
| 2 | 工具条 | Fetch / Pull / Push / Branch / Stash | `tbtn` 26 高 | 同步与建引用 | 板上都不禁用 |
| 3 | 工具条 | `History` / `Changes 12` | 分段项 22 高 | 切仓库视图 | `History` 当前 |
| 4 | 工具条 | `Agent` | `tbtn` 26 高 + `accent-soft` | 去 12 | 本板是链接 |
| 5 | 顶栏左 | `Commits`（`aria-label=Back to commit list`） | chip 22 高 | 回 04 | 常驻 |
| 6 | 顶栏右 | `Blame` / `Diff` | 分段项 | 逐行追溯 / 当前文件 revision 差异 | `Blame` 当前 |
| 7 | Diff 顶栏右 | 折叠 | 图标按钮 | 折叠/展开未改动上下文 | 仅 Diff 视图显示 |
| 8 | Diff 顶栏右 | 换行 | 图标按钮 | 切换软换行 | 仅 Diff 视图显示 |
| 9 | 检查器页签 | `Changes 7` | 页签 12 | 回 18 | 带计数，链 18 |
| 10 | 检查器页签 | `History` | 页签 12 | 本列表 | 当前项 600 + 2px 下划线 |
| 11 | 检查器页签右 | 漏斗（`aria-label=Filter file history`） | 图标 24×24 | 过滤文件历史 | 无展开态 |
| 12 | 检查器脚 | `Open in Editor` | `btn small` 22 高 + 外链 12 | 外部编辑器打开 | 常驻 |
| 13 | 检查器脚 | 复制（`aria-label=Copy file path`） | 图标 24×24 | 复制文件路径 | 常驻 |

- **整行可点**：blame 行与文件历史行都使用公共 `ui::row` 按钮组件；选中 blame 行聚焦右侧对应提交，选中历史行重算当前 revision。
- 顶部搜索框（176×26、`Search commits`）在本板仍占位，与 blame 内容没有关系。

## 8. 文案要点

- 顶栏那句 `16 lines · last change 2h ago` 同时给规模与新鲜度，`tnum` 等宽数字。
- 路径条右侧 `6 commits` 与列表 6 行一致，是板上少数自洽的计数。
- 检查器页签只两个词 `Changes` / `History`，没有 `Blame`——blame 是视图，不是页签。
- blame 的 gutter 不写日期，只写 sha 与作者；日期留给右侧列表，避免每行重复。

## 9. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 18 Files at commit | 唯一入链（`CommitFiles.dc.html` 的 `Blame` 分段项）；本板 `Changes 7` 回链 |
| 04 Commits | `Commits` chip 与 `History` 模式项链到它 |
| 10 Staging & commit / 12 Agent panel | `Changes 12` 分段项 / `Agent` |
| 23 Settings / 01 Welcome | 齿轮 / 页签 `+` |
| 20 Command palette / 21 Dark & context menu | 两板的 `Split/Unified`、折叠、换行、`2 of 7` 是本板顶栏那组图标的完整版 |
