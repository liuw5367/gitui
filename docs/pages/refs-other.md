# 引用 · 标签、远程与贮藏 — 布局、状态与按钮

- **画板**：30 Refs · Tags, Remotes, Stashes（三张表同屏 · `Tags` 分段当前 · 6 行数据；无检查器、无选中行、无空表、无建/改表单）
- **来源**：`designs/graft-git/project/RefsOther.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

30 是 09 的另外三类引用，共用的是「Refs 条 32 + 表头 22 + 30 高一行的表」这套结构：`grid` 定列、1px `line-soft` 分隔、行高与内距逐项对齐；09 的 `refs-cols` / `refs-row` 是五列、行是 `button`，30 的 `dt-head` / `dt-row` 是三列、行是 `div`。换掉的是三件事：右列检查器整列消失，表从一张变三张（各自组头 + 表头 + 行），分段从四项变三项并把 `Branches` 提成条上的回链 chip。09 的五列 `Branch / Upstream / Sync / Last commit / Updated` 在这里按类型压成三列，`Upstream` 与 `Sync` 没有对应物，`Updated` 只剩表头。检查器里的六个动作按钮，缩成每行右端一个词。

板内 `.window` 没有内联尺寸，取样式表的 `height:calc(100vh - 48px)`：预览 1440×900、桌面留白 24，窗口 **1392×852**。本页是**主窗口内换主体**：标题栏 40 / 工具条 44 / 主体 **742** / 状态栏 24（742 = 852 − 2 − 40 − 44 − 24），没有横幅也没有返回条。因为没有右列，中列从 09 的 798 扩到 **1158** = 1390 − 232。

## 1. 骨架

```
┌────────────────┬─────────────────────────────────────────────────────────────────────────────────┐
│ ● ● ●  [gitui ×] [docs-site ×] [atlas-web ×] [+]     ● Fetched 2m ago   [gear]                   │
├──────────────────────────────────────────────────────────────────────────────────────────────────┤
│ [=] | Fetch Pull Push | Branch Stash   [search ⌘F] | History [Changes 12] | Agent [=]            │
├──────────────────────────────────────────────────────────────────────────────────────────────────┤
│ Branches 5     │ Refs  [Branches]  [Tags] Remotes Stashes                              [New Tag] │
│ ● main ✓ ↑2    │ Tags 3                                                                [New Tag] │
│ ● feature/diff │ Tag               Commit                                                   When │
│ Remotes 2      │ v1.4.1            3f2a1c9                                                  Push │
│   origin 8     │ v1.4.0            77e2a90                                                  Push │
│ Tags 3         │ latest            3f2a1c9                                                Delete │
│ ▣ v1.4.0       │ Remotes 1                                                          [Add Remote] │
│ Stashes 2      │ Remote            URL                                                     Fetch │
│ Submodules 2   │ origin            github.c…                                                Edit │
│ Worktrees 3    │ Stashes 2                                                       [Stash Changes] │
│                │ Stash             Branch                                                   When │
│                │ Wrapped-line…     main                                                    Apply │
│                │ Agent cleanup     main                                                      Pop │
├──────────────────────────────────────────────────────────────────────────────────────────────────┤
│ main   ● 2 ahead · 0 behind        12 changes   UTF-8   LF   TypeScript                          │
└────────────────┴─────────────────────────────────────────────────────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 标题栏 | 顶部通栏 | 高 40（内联覆盖 38）；内距 0×13 | 与 09 相同 |
| 工具条 | 标题栏下 | 高 44；内距 0×10；间距 6 | 与 09 相同，`History` 仍为当前项 |
| 侧栏 | 主体左 | 宽 232（含右边框 1）；内距 2/8/12 | 与 09 同一套六组引用 |
| Refs 条 | 中列顶 | 高 32；内距 0×10；间距 7 | `Refs`、`Branches` chip、三段、`New Tag` chip |
| 组头 ×3 | 表前 | 高 30（内联）；内距 0×11；间距 7 | `Tags 3` / `Remotes 1` / `Stashes 2` + 各自动作 chip |
| 表头 ×3 | 组头下 | 高 22；内距 0×10；列 980 / 92 / 54；列距 6 | `Tag·Commit·When` / `Remote·URL·Fetch` / `Stash·Branch·When` |
| 表 ×3 | 表头下 | 行高 30；共 3 + 1 + 2 行 | 见 §4 |
| 状态栏 | 底部通栏 | 高 24；内距 0×12；11 | 与 09 一字不差 |

滚动区 **710** = 742 − 32（Refs 条），三组内容共 **336** = 3×30 + 3×22 + 6×30，余 374 留白。第一列 980 = 1158 − 20（左右内距）− 12（两个 6 的列距）− 146（92 + 54）。图中第三列按板上的 `.r` 右对齐贴右画。

## 2. Refs 条：一项回链 + 三段

条 32 高、`content` 底、下框 1 `line-soft`、内距 0×10、间距 7：`Refs`（11.5/600 `text`）→ `Branches` chip（22 高、内距 0×7、圆角 5、1px `line`、`panel` 底、11 号，内嵌 10 的分支图标，是 `<a>` 链 09）→ 三段分段（内距 2、项间距 2、圆角 7、`panel-2` 底、1px `line-soft`；项 21 高、内距 0×7、10.5/500，当前项 `Tags` 白底 + 0 1px 1px 阴影）→ 弹性空隙 → `New Tag` chip（22 高、11 号，内嵌 10 的加号）。

与 09 的同一处比：09 的分段是四项且 `Branches` 在段内，条上右侧是 `+ New Branch` 加一枚 `Filter refs` 漏斗；30 把 `Branches` 抽出成 chip、分段只留 `Tags / Remotes / Stashes`，`New Tag` 顶替 `+ New Branch` 的位置，**漏斗没了**。分段仍是 `role=tablist`、`aria-label="Ref type"`，三项都是 `button`，没有 href。

## 3. 组头与表头

- **组头** 30 高（内联）、`panel` 底、10/600 `text`、内距 0×11、间距 7：组名 → `badge-soft` 计数（18 高、内距 0×6、圆角 4、`panel-2` 底、1px `line-soft`、9.5 `text3`）→ `spacer` → 组级 chip。第一组只有下框 1 `line`，第二、三组上下各 1 `line`。三枚 chip 都**不带图标**：`New Tag`、`Add Remote`、`Stash Changes`。
- **表头** 22 高、`content` 底、下框 1 `line-soft`、内距 0×10、列距 6、`grid` 三列 `minmax(0,1fr) 92px 54px`；10/500 `text3`（样式表初值 8/600/.04em/大写，被末次声明改掉），第三列 `.r` 右对齐。
- **行** 30 高、同一网格与内距、下框 1 `line-soft`，每组最后一行没有下框，靠下一组组头的上框接缝。第一列 mono 9.5 `text` 省略，第二列 9.5 `text3` 省略，第三列 9.5 `accent` 右对齐。行是 `div`：**没有 hover、没有选中态、不可点**——这是与 09 最实质的一处不同，09 每行是 `button.refs-row`，有 `sel` 底与左侧 2px `accent` 竖条。

## 4. 三张表的列语义

| 类型 | 组头 | 三列（板上原文） | 行（第一 / 第二 / 第三列） |
|---|---|---|---|
| Tags | `Tags` + `3` + `New Tag` | `Tag` `Commit` `When` | `v1.4.1` · `3f2a1c9` · `Push`；`v1.4.0` · `77e2a90` · `Push`；`latest` · `3f2a1c9` · `Delete` |
| Remotes | `Remotes` + `1` + `Add Remote` | `Remote` `URL` `Fetch` | `origin` · `github.com/gitui/gitui` · `Edit` |
| Stashes | `Stashes` + `2` + `Stash Changes` | `Stash` `Branch` `When` | `Wrapped-line fixes` · `main` · `Apply`；`Agent cleanup` · `main` · `Pop` |

对 09 五列的替换关系：`Branch` 变成各类型的名字列（`Tag` / `Remote` / `Stash`）；`Upstream` 与 `Sync` 完全消失；`Last commit` 只在 Tags 保留为 `Commit`，Remotes 与 Stashes 的第二列换成本类型自己的维度（`URL`、`Branch`）；`Updated` 只剩表头 `When`，行里没有一处时间。Remotes 的第三列表头写 `Fetch`，板上没有再解释它是 URL 的 fetch 规格还是动作。

第三列（54 宽、右对齐、`accent`）在 09 是 `Updated` 的相对时间，在这里被行内动作词占满：`Push`、`Delete`、`Edit`、`Apply`、`Pop`。**这些词是 `span.act`，不是按钮也不是链接**，板上没有 href，也没有二次确认。09 检查器里的两排六枚动作（`Checkout` `Rename` `Delete` / `Compare` `Merge` `Rebase`）、三个页签（`Overview` `Commits` `Compare`）在本板没有对应物：能做的动作只剩每行右端一个词，且与类型绑定——标签是推/删，远程是编辑，stash 是应用/弹出。

## 5. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 分段类型 | Tags / Remotes / Stashes | 只画 `Tags` 当前，但三张表同时可见 |
| 组可见性 | 有内容 / 空 | 三组都有内容 |
| 行选中 | 任意一行 / 无 | 无（行不是可点元素） |
| 过滤 | 关 / 开 | 无过滤控件 |
| 组级动作 | 建 / 改 / 删 | 只有建（`New Tag`、`Add Remote`、`Stash Changes`） |

```
进入：09 的 Open details → 本板；板上没有第二处指向本页的 href
切换：Tags ⇄ Remotes ⇄ Stashes 是分段按钮，但三张表都在，点了是滚动定位还是什么都不做未画
离开：Refs 条 Branches chip → 09；窗口级按钮 → 10 / 12 / 23 / 01
```

## 6. 画板没覆盖的组合（待补设计）

1. **行内动作点下去做什么**：`Push` / `Delete` / `Edit` / `Apply` / `Pop` 全是 `span`，没有确认、没有禁用条件（如当前标签已推送、stash 与工作区冲突）、没有结果反馈。
2. **动作不成组**：Tags 把 `Push` 与 `Delete` 混在同一列，Remotes 只有 `Edit`（没有 `Remove`、`Fetch now`），Stashes 只有 `Apply` 与 `Pop`（没有 `Drop`、没有看 diff）。同一个标签为什么该 `Push`、另一个该 `Delete`，板上没有依据列。
3. **表头与内容错位**：Tags / Stashes 第三列标题是 `When`，行里放的却是动作词，时间一栏实际不存在；Remotes 标题 `Fetch` 语义不明（是 URL 还是 fetch 规格），行里却是 `Edit`。
4. **空态**：三类各自的空表（无标签、无远程、无 stash）没画，组头计数为 0 时是否保留组、是否显示占位行也没画。
5. **分段与内容矛盾**：`Tags` 是当前项，`Remotes` 与 `Stashes` 两张表却同时可见；分段到底过滤、跳转还是滚动锚点，板上没有行为线索。
6. **建/改表单**：`New Tag`（两处）、`Add Remote`、`Stash Changes` 与 `Edit` 都没有表单、没有字段与校验。
7. **计数与名单不一致**：侧栏 `Remotes 2`（origin + upstream）而组头写 `1`；侧栏标签是 `v1.4.0 / v1.3.2 / v1.2.0`，主表是 `v1.4.1 / v1.4.0 / latest`；侧栏 stash 是 `WIP: composer states / agent: token guard`，主表是 `Wrapped-line fixes / Agent cleanup`。同一仓库两处名单对不上。
8. **过滤**：09 条上有 `Filter refs` 漏斗，本板没有；三类合并后是否需要过滤、按什么过滤没画。

## 7. 按钮与链接清单

5 个 chip；分段、组头与行内动作词不计入，见本节末。

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | Refs 条左 | `Branches` | chip 22 高 + 分支图标 10 | 回 09（板内唯一回链） | 常驻 |
| 2 | Refs 条右 | `New Tag` | chip 22 高 + 加号 10 | 建标签 | 常驻，与第 3 项重复 |
| 3 | Tags 组头右 | `New Tag` | chip 22 高，无图标 | 建标签 | 常驻 |
| 4 | Remotes 组头右 | `Add Remote` | chip 22 高，无图标 | 加远程 | 常驻 |
| 5 | Stashes 组头右 | `Stash Changes` | chip 22 高，无图标 | 贮藏当前改动 | 常驻 |

- **分段**：`Tags / Remotes / Stashes`（`role=tablist`、`aria-label="Ref type"`，项 21 高、10.5，当前项白底 600）。
- **行内动作**：`Push` `Push` `Delete` `Edit` `Apply` `Pop`，9.5 `accent` 右对齐的 `span`，**不是按钮**，不计入。
- **窗口级按钮**：页签 `+`、齿轮、`Fetch/Pull/Push/Branch/Stash`、`Changes 12`、`Agent`、侧栏开关，与 04 / 09 一致，见 [workspace.md](workspace.md) §8；本页没有返回条。

## 8. 文案要点

- 计数的位置换了：09 把每类计数放在分段的 `.count` 里（10、`text3`），30 放到组头的 `badge-soft` 胶囊（18 高、9.5），所以同屏出现三个胶囊、而分段上一个数字也没有。
- 同一列承担五种语义：`Push` `Delete` `Edit` `Apply` `Pop` 全在第三列右对齐，没有图标、没有后缀、没有颜色区分。
- 表头一律是单数名词（`Tag` `Commit` `Remote` `URL` `Stash` `Branch`），没有一处写成「对象 + 动作」；`Fetch` 是其中唯一一个像动作的表头，行里却是 `Edit`。
- `New Tag` 说了两遍（条上一次、Tags 组头一次），措辞与形态相同，只差组头那枚没有加号图标；其余两组只有组头一次。

## 9. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 09 Refs manager | `Branches` chip 链 `RefsManager.dc.html`；09 的 `Open details` 反过来链本板，且它的说明句自称 `Tag/remote/stash tabs share this table` |
| 04 Commits / 10 Changes / 12 Agent | 共用同一套窗口框架与侧栏；工具条 `Changes 12`、`Agent`（板内各一处 href） |
| 23 Settings / 01 Welcome | 齿轮与页签 `+`（板内各一处 href） |
| 15 / 28 / 29 Sync | 只共用工具条的 `Fetch Pull Push` 三个按钮；本板与同步流程没有链接 |
| 07 Compare refs | 09 的 `Compare` 去 07；本板三类引用都没有比较入口 |
