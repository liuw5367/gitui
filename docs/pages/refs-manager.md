# 引用管理 — 布局、状态与按钮

- **画板**：09 Refs manager（`Branches` 段 · 选中当前分支 `main` · 检查器 `Overview`；无 `Tags`/`Remotes`/`Stashes` 表、无建分支表单、无未推送分支的检查器）
- **来源**：`designs/graft-git/project/RefsManager.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

板内 `.window` 没有内联尺寸，取样式表的 `height:calc(100vh - 48px)`：预览 1440×900、桌面四周留白 24，窗口 **1392×852**。本页是**主窗口内换主体**：标题栏、工具条、侧栏、状态栏与 04 相同，中列整个换成「Refs 条 + 引用表」，右列检查器换成分支面板。没有横幅，也没有 34 高的返回条，所以主体 **742** 全给中列与右列。

## 1. 骨架

```
┌────────────────┬──────────────────────────────────────────────────┬───────────────────────────────────┐
│ ● ● ●  [gitui ×] [docs-site ×] [atlas-web ×] [+]     ● Fetched 2m ago   [gear]                        │
├───────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ [=] | Fetch Pull Push | Branch Stash   [search ⌘F] | [History] Changes 12 | Agent [=]                 │
├───────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ Branches 5     │ Refs  [Branches] Tags Remotes Stashes            │ main                              │
│ ● main ✓ ↑2    │ Branch  Upstream  Sync  Last commit  Updated     │ ● Current branch                  │
│ ● feature/diff │ [current] main   origin/main  2↑ 3f2a1c9 1h      │ 2 ahead · 0 behind origin/main    │
│ Remotes 2      │ feature/diff-view  origin/f…  0  a37c6b1  3h     │ 3f2a1c9 · Fix diff hunk align…    │
│   origin 8     │ feature/agent      Not pushed 1  c72d9a4  4h     │ [Overview] Commits  Compare       │
│ Tags 3         │ fix/wrapped-lines  origin/f…  0  e19a3f2  5h     │ Upstream    origin/main           │
│ ▣ v1.4.0       │ release/1.4        origin/r…  0·2 77e2a90 1d     │ Tracking    ahead 2 · behind 0    │
│ Stashes 2      │ origin/main        remote  -  3f2a1c9  2m        │ Last commit 3f2a1c9 · 1h ago      │
│ Submodules 2   │                                                  │ Worktree    ~/Developer/gitui     │
│ Worktrees 3    │                                                  │ Actions                           │
│                │                                                  │ [Checkout] [Rename] [Delete]      │
│                │                                                  │ [Compare] [Merge] [Rebase]        │
│                │                                                  │ Open details                      │
├───────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ main   ● 2 ahead · 0 behind        12 changes   UTF-8   LF   TypeScript                               │
└────────────────┴──────────────────────────────────────────────────┴───────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 标题栏 | 顶部通栏 | 高 40（内联覆盖 44）；内距 0×13 | 与 04 相同 |
| 工具条 | 标题栏下 | 高 44；内距 0×10；间距 6 | 与 04 相同，`History` 仍为当前项 |
| Refs 条 | 中列顶部 | 高 32；内距 0×10；间距 7 | `Refs` 标题、四段引用类型、`+ New Branch`、过滤图标 |
| 表头 | 条下 | 高 22；内距 0×10；下边框 1 | `Branch / Upstream / Sync / Last commit / Updated` |
| 引用表 | 表头下 | 6 行 × 高 30；列 364 / 128 / 86 / 124 / 52 | 5 条本地分支 + 1 条远程分支，首行选中 |
| 侧栏 | 主体左 | 宽 232（含右边框 1）；内距 2/8/12 | 与 04 同一套六组引用，`main` 是 `sel` 行 |
| 检查器 | 主体右 | 宽 360（含左边框 1）；头内距 12/14/10 | 分支头、三页签、4 行键值、Actions、一段说明 |
| 状态栏 | 底部通栏 | 高 24；内距 0×12；11 | 与 04 一字不差 |

当前实现的 Branches 侧栏按 `/` 把名字组织为可折叠前缀树，同级文字升序。组头与前缀均可点按折叠；前缀右键菜单可单独展开/收起，也可一次展开或收起全部下级前缀。Branch 右键菜单完整包含 `Checkout`、`New Branch from '<name>'…`、`Compare with Current Branch`、`Rebase Current onto '<name>'`、`Merge '<name>' into Current`、`New Tag…`、`Rename…`、`Delete…`、`Copy Branch Name`、`Push '<name>' to origin`；当前分支不允许 Checkout、Rebase、Merge、Delete，Rebase 只在干净工作区且没有正在执行的 Git 操作时可用，Push 只在配置了 `origin` 时可用。六组标题的基础折叠交互共用此侧栏结构。

Remotes 侧栏按远端名称升序显示配置节点，即使远端还没有跟踪分支也保留节点。展开后列出配置的全部 Fetch URL、Push URL 和远端跟踪分支；`origin/HEAD` 等符号别名不作为分支展示。分支名按 `/` 分成多级折叠树，每层按文字升序。组头右键有 `Add Remote…` 与组展开/收起；远端节点菜单含 `Fetch…`（打开并预选此远端的同步面板）、`Edit Remote URL…`、`Copy Remote Name`、`Copy URL`（默认复制 Fetch URL）与 `Remove Remote…`；URL 行菜单有 `Copy URL`。远端分支叶菜单依序提供 `Checkout as New Local Branch`（预填本地名称和远端起点，创建跟踪分支并检出）、`Compare with Current`、`Merge into Current`、`Rebase Current onto`、`Pull into Current…`（确认框显示源与当前分支，要求干净工作区）、`Delete Remote Branch…`（确认后从服务器删除并剪枝刷新本地跟踪引用）、`Copy Name`（复制含远端前缀的全名）。远端分支变基和合并仅在仓库 HEAD 位于分支、工作区干净且没有其它 Git 操作时启用；确认期间菜单目标仍绑定发起操作的仓库标签。

Tags 侧栏按标签名升序显示。组头右键提供 `New Tag…`，表单针对发起操作的仓库并从当前 HEAD 创建。标签行右键提供 `Push Tag…` 子菜单并列出配置远端，选择后推送该标签；没有配置远端时该菜单项禁用。其余动作是 `Copy Name` 和 `Delete Tag…`；删除确认后仅移除本地标签，不影响远端副本。删除确认与推送都绑定发起菜单的仓库标签，若标签或远端已变化则提示重新打开菜单。

Stashes 侧栏按摘要升序显示。组头的 `Stash Changes…` 打开全量贮藏表单；`Stash Selected Files…` 打开改动文件多选表单，支持说明和包含未跟踪文件选项，只贮藏所选路径。无工作区改动时两个入口禁用。贮藏行提供 `Apply Stash`、`Pop Stash`、确认后的 `Drop Stash…`、`Show Diff` 和 `Copy Stash Name`；显示差异时可返回普通改动视图。菜单目标以贮藏提交 OID 绑定，所以索引变化后会重新定位目标；若目标已不存在则提示重新选择。

中列 798 = 1390 − 232 − 360。Refs 条 32 + 表头 22 之后，表格区 **688** = 742 − 54；6 行只占 180，其余留白。第一列 364 = 798 − 20（左右内距）− 24（4 个 6 的列距）− 390（128 + 86 + 124 + 52）。

## 2. Refs 条与表头

Refs 条 32 高、`content` 底、下边框 1 `line-soft`：`Refs`（11.5/600 `text`）→ 分段容器（内距 2、圆角 7、`panel-2` 底、1px `line-soft`、项间距 2；四项高 21、内距 0×7、10.5/500，当前项白底 + 0 1px 1px 阴影）→ 弹性空隙 → `+ New Branch` chip（22 高、内距 0×7、圆角 5、1px `line`、`panel` 底、11，内嵌 10 的加号）→ `Filter refs` 图标（24×24，13 的漏斗）。分段是 `role=tablist`、`aria-label="Ref type"`，四项都是 `button`，板上只有 `Branches` 带选中样式。

表头 22 高、10/500 `text3`，与行同网格、同内距，`Updated` 右对齐；不转大写（样式表 `text-transform:none`）。表头行自带 1px `line-soft` 下边框，行与行之间也是 1px `line-soft`。

## 3. 引用表

```
┌──────────────────────┬────────────────────┬───────────┬───────────────────┬──────────┐
│ Branch               │ Upstream           │ Sync      │ Last commit       │ Updated  │
│ [current] main       │ origin/main        │ 2 ahead   │ 3f2a1c9 Fix hunk… │ 1h       │
│ feature/diff-view    │ origin/feature…    │ 0 · 0     │ a37c6b1 Add…      │ 3h       │
│ feature/agent        │ Not pushed         │ 1 ahead   │ c72d9a4 Merge…    │ 4h       │
│ fix/wrapped-lines    │ origin/fix…        │ 0 · 0     │ e19a3f2 Extra…    │ 5h       │
│ release/1.4          │ origin/release…    │ 0 · 2     │ 77e2a90 Release   │ 1d       │
│ origin/main          │ remote             │ -         │ 3f2a1c9 Fix hunk… │ 2m       │
└──────────────────────┴────────────────────┴───────────┴───────────────────┴──────────┘
```

- **行**：30 高、`grid` 五列、内距 0×10、间距 6、无圆角，整行可点。hover 用 `hover` 底；选中行 `.on` 用 `sel` 底，并在最左画 2px `accent` 竖条（`::before`，压在单元格下面）。
- **Branch 列**（364）：分支名 10.5 `text`，超长省略，右侧留 6；`current` 徽章 14 高、内距 0×3、7.5/600、圆角 3、`accent-soft` 底 `accent-2` 字，与名字间距 5。
- **Upstream 列**（128）：mono 8.5 `text3`，写跟踪分支全名。`feature/agent` 行写 `Not pushed`，用内联 `warn` 色；远程分支行 `origin/main` 写 `remote`。
- **Sync 列**（86）：8.5、等宽数字、`text3`，板上出现四种写法：`2 ahead · 0 behind`、`0 · 0`、`1 ahead`、`0 · 2`，远程分支行是 `—`。
- **Last commit 列**（124）：mono 8.5，短 SHA + 空格 + 截断的消息（`3f2a1c9 Fix diff hunk…`），超出省略。
- **Updated 列**（52）：8.5 `text3` 右对齐，相对时间 `1h 3h 4h 5h 1d 2m`。
六行内容：`main`（选中）、`feature/diff-view`、`feature/agent`、`fix/wrapped-lines`、`release/1.4`、`origin/main`。前五条是本地分支，最后一条是远程跟踪引用，所以它的 Upstream 写 `remote`、Sync 写 `—`。

## 4. 检查器

- **头部**：标题 `main`（13/600、行高 1.32）；下距 9 的 meta 是 6 的 `ok` 圆点 + `Current branch`（11.5/600）+ 第二行 `2 ahead · 0 behind origin/main`（10 `text3`，行高 1.2）；再下 9 的 hashrow：`3f2a1c9` 徽章（mono 10.5、`panel-2` 底、1px `line-soft`、内距 2×5、圆角 4）· `Fix diff hunk alignment`（10.5 `text3`）。
- **页签** 33 高、内距 0×14、间距 14、字号 12：`Overview`（当前，600 + 底部 2px `accent`）/ `Commits` / `Compare`。三个都没有计数徽章。
- **body** 内距 10/10/12，先 4 行键值（10.5，左 `text2` 右 `text` 500）：`Upstream origin/main`、`Tracking ahead 2 · behind 0`、`Last commit 3f2a1c9 · 1h ago`、`Worktree ~/Developer/gitui`。
- **Actions**：`statline` 上距 12 的 `Actions` 之后两排按钮，排内间距 6、第二排再上距 6。按钮 24 高、内距 0×10、圆角 6、12/500。第一排 `Checkout` 是 `primary`（`accent` 底、白字，`flex:1` 撑开），`Rename` 与 `Delete` 不设 `flex`，宽度随文字自适应，所以第一排不是等分。第二排 `Compare`（`<a>` 链 07）、`Merge`、`Rebase` 三个都 `flex:1`，等分。
- **末尾说明** 10.5 `text3`、行高 1.5、上距 12：`Branch management includes upstream, tracking, checkout, rename, delete, compare, merge, and rebase. Tag/remote/stash tabs share this table.`，句末 `Open details` 是 `accent-2` 500 的链接，指向 30。

## 5. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 引用类型 | Branches / Tags / Remotes / Stashes | 只画 `Branches` |
| 表选中行 | 任意一行 / 无 | 只画第 1 行 `main` |
| 当前分支 | `main` / 其它 | 只画 `main`（`current` 徽章 + `Overview` 的 `Current branch`） |
| 检查器页签 | Overview / Commits / Compare | 只画 `Overview` |
| 过滤 | 关 / 开 | 只画关（一个漏斗图标） |

```
进入：板上没有指向本页的 href；能到达本页的唯一板内证据在 30 里（`RefsOther.dc.html` → `RefsManager.dc.html`）
切换：Branches ⇄ Tags ⇄ Remotes ⇄ Stashes 换表与检查器（后三者的表在 30）
离开：检查器 `Compare` 去 07；`Open details` 去 30；工具条 `Changes 12` 去 10；`Agent` 去 12
```

## 6. 画板没覆盖的组合（待补设计）

1. **Tags / Remotes / Stashes 三段的表**：表头在本板写死 `Branch / Upstream / Sync / Last commit / Updated`，只对 `Branches` 成立；标签没有 Upstream 与 Sync，stash 没有 Upstream。这三类的行与列在 30，本板没有画切换后的样子，也没画检查器跟着换。
2. **选中非当前分支**：检查器只画当前分支（`Current branch`、`2 ahead · 0 behind origin/main`、主按钮 `Checkout`）。选中 `feature/agent`（`Not pushed`）或 `origin/main`（远程引用）时头部第二行怎么写、主按钮是否禁用或换成别的动作，都没有画。
3. **建分支与行级操作**：`+ New Branch` 的表单、`Rename` / `Delete` / `Merge` / `Rebase` 的确认与冲突结果都没有画；`Delete` 落在当前分支上应禁用，也没有画。
4. **过滤与空表**：`Filter refs` 只有图标，没有输入态；仓库只有一条分支、或过滤后零命中时的空态没有画。

## 7. 按钮与链接清单

共 9 组；分段、页签、整行可点的行不计入，见本节末。

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | Refs 条右 | `+ New Branch` | chip 22 高 + 加号 10 | 建分支 | 常驻 |
| 2 | Refs 条最右 | 漏斗（`aria-label=Filter refs`） | 图标 24×24 | 过滤引用 | 无 `on` 态 |
| 3 | 检查器 Actions | Checkout | 主按钮 24 高，`flex:1` | 切到该分支 | 当前分支上仍可点，板上没禁用 |
| 4 | 检查器 Actions | Rename | 按钮 24 高 | 改名 | 常驻 |
| 5 | 检查器 Actions | Delete | 按钮 24 高 | 删除分支 | 常驻，板上没确认弹窗 |
| 6 | 检查器 Actions | Compare | 按钮 24 高，`flex:1` | 去 07 比较 | 常驻 |
| 7 | 检查器 Actions | Merge | 按钮 24 高，`flex:1` | 合入当前分支 | 常驻 |
| 8 | 检查器 Actions | Rebase | 按钮 24 高，`flex:1` | 变基 | 常驻 |
| 9 | 检查器说明末 | Open details | 文字链接（`accent-2` 500） | 去 30 | 常驻 |

- **整行可点**：6 行引用（30 高，选中行 `sel` 底 + 左侧 2px `accent` 竖条，点一行即刷新检查器）。**分段**：Refs 条 `Branches / Tags / Remotes / Stashes`（`role=tablist`，当前项白底）；**页签**：检查器 `Overview / Commits / Compare`（`role=tablist`，当前项 600 + 2px `accent` 下划线，三个都是 `button`）。
- **窗口级按钮**（页签 `+`、齿轮、侧栏/检查器开关、Fetch/Pull/Push/Branch/Stash、`Changes 12`、`Agent`）与 04 一致，见 [workspace.md](workspace.md) §8；本页**没有**返回条，也没有横幅。

## 8. 文案要点

- Sync 列是四种写法混排：`2 ahead · 0 behind`（完整）、`1 ahead`（只写有值的一侧）、`0 · 2`（只有数字，位次仍是 ahead · behind）、`—`（远程引用不适用）。实现需要挑一种，不能四种都用。
- Upstream 列把「没有上游」写成 `Not pushed`（`warn`），不是 `—`；`—` 只留给远程引用行。
- `Current branch` 是检查器的状态说明，不是页签名；页签只有 `Overview / Commits / Compare`。
- 说明句点明后三个分段与本表同构（`share this table`），但表头只写了分支的列。

## 9. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 30 Refs · Tags, Remotes, Stashes | `Open details` 指向它（板内 `RefsOther.dc.html`）；它反过来有指向本页的 `RefsManager.dc.html`，且用 3 段（Tags/Remotes/Stashes）替代本页的 4 段 |
| 07 Compare refs | 检查器 `Compare`（板内 `CompareRefs.dc.html`） |
| 10 Staging & commit / 12 Agent | 工具条 `Changes 12` / `Agent`（板内各一处 href） |
| 23 Settings / 01 Welcome | 齿轮 / 页签 `+`（板内各一处 href） |
| 04 Commits | 共用同一套窗口框架与侧栏；本页没有返回条，中列全给引用表 |
