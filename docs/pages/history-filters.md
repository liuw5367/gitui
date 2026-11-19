# 历史过滤 — 布局、状态与按钮

- **画板**：05 History filters（弹层打开 · `Custom` 日期 · 6 个分支勾 3 个；无过滤生效后的列表、无 0 结果）
- **来源**：`designs/graft-git/project/HistoryFilter.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

## 当前实现

- 分支选择支持本地与远端分支多选；不选择分支表示全部引用，所选分支之间按 OR 合并。
- 作者支持多选，以 email 作为稳定 ID，显示 `姓名 <email>`，因此同名作者可区分；所选作者之间按 OR 合并。
- 日期保留不限、今天、最近 7 天、最近 30 天、最近一年预设，并提供自定义起止日期。自定义日期按 `YYYY-MM-DD` 输入；两端均包含，Git 按本地时区解析。无效日期与起止倒置会提示并阻止 Apply。
- Path 接受仓库相对文件或目录路径；Message 匹配完整提交消息（subject 和 body）的不区分大小写子串。分支、作者、日期、路径与消息维度之间按 AND 组合。
- 弹层内编辑只改草稿；Apply 才更新当前仓库的已应用条件。Esc 收起浮层但保留草稿；Reset 清空草稿和已应用条件。工具栏快速搜索单独工作，匹配消息、作者、短 SHA 与 ref。
- 提交列表按应用条件返回，列表计数显示筛选后的数量；0 结果沿用 History 空态。Graph 从未筛选的同分支范围读取祖先，并把隐藏节点折叠为可见祖先边。
- 引用侧栏定位到被应用条件隐藏的提交时，会清除该仓库已应用条件并滚动到目标，保留筛选草稿与快速搜索。

板内 `.window` 没有内联尺寸，取样式表的 `height:calc(100vh - 48px)`：预览 1440×900、桌面留白 24，窗口 **1392×852**。
与 04 相比少了授权横幅，所以主体是 **742** = 852 − 2 − 40 − 44 − 24；中列仍 798，侧栏 232、检查器 360。

## 1. 骨架

```
┌───────────────┬─────────────────────────────────────────────┬───────────────────────────────┐
│ ● ● ●  [gitui ×] [docs-site ×] [atlas-web ×] [+]     ● Fetched 2m ago   [gear]              │
├─────────────────────────────────────────────────────────────────────────────────────────────┤
│ [=] | Fetch Pull Push | Branch Stash   [search commits ⌘F] | History Changes 12 | Agent [=] │
├─────────────────────────────────────────────────────────────────────────────────────────────┤
│ Branches 5    │ History                                     │ Fix diff hunk alignment       │
│ ● main * +2-0 │ main 2  46 commits   Nov 1-14               │ MC Mia Chen  2 hours ago      │
│ ● feature/dif │ Compare     All authors  [filter on]        │ 3f2a1c9 [copy]                │
│ Remotes 2     │ Description    Author       Date            │ Committed on main · 7 files   │
│   origin 8    │ | Fix diff hunk       Mia Chen  2h          │  Changes 7  Files  Info       │
│ Tags 3        │   +-- filter pop: 432 wide, top 38 --+      │ 7 files changed +162 -62      │
│ ▣ v1.4.0      │   | Branches | Date range | Refine  |       │ [tree] [flat]                 │
│ Stashes 2     │   +----------------------------------+      │ src/git                       │
│ Submodules 2  │ | Align hunk headers  Mia Chen  6d          │  diff-engine.ts M +18 -7      │
│ Worktrees 3   │ | Fold long hunks     Mia Chen  1w          │  hunk.ts A +44 -0             │
│               │ | Split hunk parsing  Mia Chen  2w          │ (empty folder merged)         │
│               │ | Show hunk stats     Mia Chen  3w          │ Open in Editor  [copy] [more] │
│               │ | Initial hunk render Mia Chen  3mo         │                               │
├─────────────────────────────────────────────────────────────────────────────────────────────┤
│ main   ● 3 branches · Nov 1-14      12 changes   UTF-8   LF   TypeScript                    │
└───────────────┴─────────────────────────────────────────────┴───────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 过滤条 | 中列顶部 | 高 32；内距 0×10；间距 7 | `History`、分支 chip `main 2`、`46 commits`、时间 chip `Nov 1–14`、`Compare`、`All authors`、漏斗（`on`） |
| 列头 | 过滤条下 | 高 23；内距 0×8；10.5/500 | `Description / Author / Date`（四列网格 `64 / 1fr / 96 / 58`） |
| 提交表 | 列头下 | 行高 28；板上 23 行 | 46 条结果的可见部分，选中第一条 |
| 过滤弹层 | 中列上层 | 宽 432；距中列顶 38、左 12；头 38、脚 42 | 三段：Branches / Date range / Refine |
| 侧栏 | 主体左 | 宽 232；内距 2/8/12 | 与 04 同一套六组引用，`main` 行仍是 `sel` |
| 检查器 | 主体右 | 宽 360 | 与 04 同一套提交头、页签、7 个文件 |

状态栏高 24（内距 0×12、11），第二段换成 `● 3 branches · Nov 1–14`。弹层是**绝对定位**在中列上的浮层（`top:38px; left:12px`），不推挤列表、不带遮罩，也不居中。图中 `☑`/`☐` 是板上 13×13 复选的两种状态，`[filter on]` 是点亮的漏斗，`[gear]`/`[=]`/`[tree]`/`[flat]` 是图标按钮。

## 2. 过滤条

与 04 同高同距，只有三处内容变化：`46 commits`（10.5 `text3`，未过滤时是 `128 commits`）、时间 chip 从 `Any time`
换成 `Nov 1–14`、最右漏斗按钮处于 `on` 态（`sel` 底 + `accent` 图标）。三个 chip 仍各自 22 高、内距 0×7、圆角 5、
1px `line` 描边、`panel` 底、11；分支 chip 内是分支图标 10 + 名字 + 计数 + chevron 9，时间 chip 内是时钟图标 10。

## 3. 过滤弹层

```
┌────────────────────────────────────────────────────┐
│ Filter History                              esc    │
│ BRANCHES                       Select all          │
│ ☑ ● main                         current           │
│ ☑ ● feature/diff-view                              │
│ ☐ ● feature/agent                                  │
│ ☑ ● fix/wrapped-lines                              │
│ ☐ ● release/1.4                                    │
│ ☐ ● origin/main                  remote            │
│ DATE RANGE                          Clear          │
│ [Any time][7 days][30 days][Custom]                │
│ [2026-11-01]   ->   [2026-11-14]                   │
│ S   M   T   W   T   F   S                          │
│ 1   2   3   4   5   6   7                          │
│ 8   9  10  11  12  13  14                          │
│ 15 16  17  18  19  20  21                          │
│ 22 23  24  25  26  27  28                          │
│ 29 30                                              │
│ REFINE  AUTHOR [Mia Chen]   PATH [src/git/]        │
│ MESSAGE [hunk]                                     │
│ 128 -> 46 commits           [Reset] [Apply]        │
└────────────────────────────────────────────────────┘
```

- **容器**：宽 432、圆角 10、`menu-bg` 底 + 1px `line-strong` 描边 + 投影；头 38、脚 42、body 内距 10/12/12 且最高 430 自滚。
- **头**：`Filter History` 12/600；右端 `esc`（10、`text3`、500）。
- **段**：每段上下内距 9、下边框 1（最后一段没有）；段标题 9/600、字距 .05em、渲染成大写，右端动作（`Select all` / `Clear`）9/500、不转大写、`accent-2`。
- **Branches**：6 行，行高 26、内距 0×4、间距 7、11；13×13 复选 + 7×7 色点 + 名字 + 右端标记（当前分支 `current` 8.5 `accent` 600；远端 `remote` 8.5 `text3` 500）。板上勾选 `main`、`feature/diff-view`、`fix/wrapped-lines`。
- **Date range**：四段 `Any time / 7 days / 30 days / Custom`（容器内距 2、项高 20、内距 0×7、9.5，`Custom` 白底）；两个日期字段高 25、mono 9.5（`2026-11-01` / `2026-11-14`），中间一个 10 的 `→`；下面 7 列日历（gap 2、列头 16 高 8.5、格子 22 高 9.5）——`1`–`14` 是 `accent` 底白字，`14` 另加 1px `accent` 描边表示今天，`15`–`30` 常态，末尾 5 个空位。
- **Refine**：两列网格（间距 7）`Author` = `Mia Chen`、`Path` = `src/git/`，下面通栏 `Message` = `hunk`；标签 11/600，字段 26 高、内距 0×7、10。
- **脚**：`128 → 46 commits`（9）+ 弹性空隙 + `Reset`（小按钮 22）+ `Apply`（主按钮 22）。这一行是唯一的「预览结果」提示。

## 4. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 弹层 | 开 / 合 | 只画开 |
| 日期模式 | Any time / 7 days / 30 days / Custom | 只画 Custom（两个字段有值） |
| 分支勾选 | 任意子集 | 只画 6 选 3 |
| Refine | 三字段任意组合 | 三个都填了 |
| 列表 | 46 条 / 128 条 | 只画 46 条 |

```
进入：04 过滤条的三个 chip 与漏斗图标       应用：Apply → 回 04（过滤后的样子没画）
取消：esc / Reset 都回 04                    Compare chip → 06
```

## 5. 画板没覆盖的组合（待补设计）

1. **过滤生效后的 04**：弹层写着 `128 → 46`，但 04 只画未过滤的 `128 commits` + `Any time` + `All authors`；chip 上的「已过滤」标记没有板。
2. **0 结果**：全部取消勾选、或日期范围为空时的空态与提示没有画，`Apply` 是否禁用也没定。
3. **未 Apply 的改动**：`esc` 与 `Apply` 是同一个目标（都回 04），板上看不出「关闭是否丢弃草稿」。
4. **日历**：上/下月切换、跨月拖选、范围中段的连续高亮都没画，`today` 描边只出现在 `14`。
5. **窄窗**：共享样式表在 ≤860 时把侧栏与检查器 `display:none`，但 432 宽的弹层如何摆放没有画。

## 6. 按钮与链接清单

共 8 组；分段、页签、日历格、整行可点的行不计入，见本节末。

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 过滤条 | `main 2` / `Nov 1–14` | chip 22 高 | 重开本弹层的 Branches / Date range 段 | 未过滤时是 `Any time` |
| 2 | 过滤条 | Compare / All authors | chip 22 高 | 打开 06 / 重开 Refine 段 | 常驻 |
| 3 | 过滤条右 | 漏斗（`aria-label=Filter commits`） | 图标 24×24 | 打开本弹层 | 本页是 `on`（`sel` 底 + `accent`） |
| 4 | 弹层头 | `esc` | 文字 10 | 关闭弹层回 04 | 常驻 |
| 5 | 弹层 Branches 段 | Select all | 文字链接 9 | 全选分支 | 板上有 3 个未勾 |
| 6 | 弹层 Date range 段 | Clear | 文字链接 9 | 清空日期范围 | 常驻 |
| 7 | 弹层脚 | Reset / Apply | 小按钮 22 / 主按钮 22 | 恢复默认 / 应用回 04 | 常驻 |
| 8 | 标题栏 + 检查器页脚 | `+`、齿轮、Open in Editor、复制、`⋯` | 同 04 | 打开 01 / 23、编辑器、复制 SHA、更多 | 常驻 |

- **分段**：`Any time / 7 days / 30 days / Custom`（容器内距 2、项高 20）；窗口级工具条按钮（侧栏/检查器开关、Fetch/Pull/Push/Branch/Stash、Agent、`History / Changes 12`）与 04 一致，见 [workspace.md](workspace.md) §8。
- **整行可点**：6 行分支（`label` + 13×13 复选，点整行即勾选）、23 行提交、7 行文件、侧栏 30 行引用。**日历格**：7 列共 35 个格子（高 22、列宽均分），`1`–`14` 选中、`14` today、`15`–`30` 可点、末尾 5 个空位；不是按钮。
- **页签**：检查器 `Changes 7 / Files / Info`（`Files` 链去 18）。

## 7. 输入与选项

弹层里共有四类输入：6 个 13×13 分支复选（板上勾 `main`、`feature/diff-view`、`fix/wrapped-lines`）、四段日期模式（`Custom` 当前，另三项只给预设范围）、两个 mono 9.5 的起止日期字段（`2026-11-01` / `2026-11-14`，与日历联动）、三个 26 高单行字段 `Author` = `Mia Chen`、`Path` = `src/git/`、`Message` = `hunk`（子串匹配）。

## 8. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 04 Commits | 弹层的 `esc`、`Reset`、`Apply` 都回它；本页由它的三个 chip 与漏斗进入 |
| 06 Multi-commit diff | 过滤条的 `Compare` |
| 18 Files at commit / 23 Settings / 01 Welcome | 检查器 `Files` 页签 / 齿轮 / 页签 `+` |
