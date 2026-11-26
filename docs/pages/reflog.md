# 引用日志与撤销 — 布局、状态与按钮

- **画板**：17 Reflog & undo（有 reset 条目选中 · 有恢复卡与明细 · 有 `42 entries`；无恢复完成态、无其它操作类型的卡片、无 Filter 弹层）
- **来源**：`designs/graft-git/project/Reflog.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

本板是**主窗口换主体**：标题栏、工具条、侧栏、状态栏沿用 04 的骨架（窗口 **1392×852**，标题栏内联 40、工具条 44、侧栏 232、状态栏 24、主体 742），中列 798 换成 reflog 表，右列 360 换成 Undo 检查器。

## 当前实现

工具栏的 Reflog 与 Fetch／Pull／Push 使用同一按钮样式，左侧的 Stash 直接打开 Stash Changes 表单。Refs／Reflog 页面仍可互切；Reflog 页不显示新建分支、标签、远程和贮藏操作。选择日志记录后，右栏与 History 共用提交检查器：文件支持 Tree／Flat 切换、展开或折叠全部目录，底部显示相同的提交 message、description 与信息；文件右键的 History 以该 Reflog 提交为基准。切回 Refs 总表后，右栏显示与应用 Changes 模式相同的变更清单和提交面板；点击文件行的非复选框区域会切到 Changes 并在中栏显示文件 Diff，复选框只暂存或取消暂存。列表在仓库载入或刷新时读取最近 100 条记录，过滤只作用于这批已加载记录；当前没有分页或滚到底加载更多。summary 单行显示，超出可用宽度时尾部省略。

在右栏 Reflog 文件树中点选文件后，中栏显示该 Reflog 提交对应的单文件 Diff，右栏检查器保留。标题显示返回 Reflog、提交摘要、当前文件、前后文件切换和提交文件数；工具栏与 History、Changes、Stash、Blame 共用同一 NSH DiffView 面板。差异直接按该记录的 SHA 读取，不依赖 History 提交列表，因此 Reflog 独有或不可达提交也可以查看。返回 Reflog 后保留原记录选中态。

## 1. 骨架

```
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│ ● ● ●  [gitui x] [docs-site x] [atlas-web x] [+]      ● Fetched 2m ago  [gear]                   │
├──────────────────────────────────────────────────────────────────────────────────────────────────┤
│ [=] Fetch Pull Push | Branch Stash     [search ⌘F] | History Changes 12 | Agent [=]              │
├──────────────────┬──────────────────────────────────────────────────────────┬────────────────────┤
│ Branches 5       │ Reflog  [main v]  42 entries                             │ Undo  History   ...│
│ ▣ main ✓  ↑2 ↓0  │                   Every ref movement  [Filter]           │                    │
│ ▣ feature/diff   │        Action                 Operation  When            │ Recover            │
│ ▣ feature/agent  │ 3f2a1c9  HEAD@{0} reset...   reset       12m             │ Undo reset HEAD~1  │
│ ▣ fix/wrapped    │ 8b41e07  HEAD@{1} commit...  commit       1h             │                    │
│ ▣ release/1.4    │ c72d9a4  HEAD@{2} checkout   checkout     1h             │ main moved from    │
│ Remotes 2        │ e19a3f2  HEAD@{3} rebase...  rebase       2h             │ 8b41e07 to 3f2a1c9.│
│   origin 8       │ a37c6b1  HEAD@{4} commit...  commit       2h             │ Restoring moves    │
│ Tags 3           │ 5d9f0e2  HEAD@{5} pull: FF   checkout     3h             │ main back; working │
│ ▣ v1.4.0 latest  │ b81c4a7  HEAD@{6} commit...  commit       4h             │ tree is preserved. │
│ ▣ v1.3.2 Oct 28  │ 2f6d8c3  HEAD@{7} reset...   reset        5h             │ [Restore 8b41e07]  │
│ Stashes 2        │ 9c0b1e4  HEAD@{8} checkout   checkout     6h             │ [Create branch]    │
│ Submodules 2     │ 77e2a90  HEAD@{9} clone...   checkout     1d             │                    │
│ Worktrees 3      │                                                          │ From 8b41e07       │
│                  │ 21 rows fit · 10 drawn                                   │ To 3f2a1c9         │
├──────────────────┴──────────────────────────────────────────────────────────┴────────────────────┤
│ main  ● 2 ahead · 0 behind            12 changes  UTF-8  LF  TypeScript                          │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 标题栏 | 顶部通栏 | 高 40（内联覆盖样式表 44）；内距 0×13 | 窗口灯、三个仓库页签、`+`、`Fetched 2m ago`、齿轮 |
| 工具条 | 标题栏下 | 高 44；内距 0×10；gap 6 | 侧栏开关、Fetch/Pull/Push、Branch/Stash、搜索 176×26、`History/Changes 12`、Agent、检查器开关 |
| 侧栏 | 主体左 | 宽 232（含右框 1）；内距 2/8/12 | 六组引用，行 25 |
| Reflog 区 | 主体中列 | 宽 798；过滤条 32 + 列头 23 + 行 32 | 分支 chip、`42 entries`、一句说明、Filter；四列 62/1fr/64/44 |
| Undo 检查器 | 主体右 | 宽 360（含左框 1）；页签 33 + 内距 12 的 body | `Undo`/`History` 页签、恢复卡、三行明细、说明、`Show in History` |
| 状态栏 | 底部通栏 | 高 24；内距 0×12；字号 11 | 分支、ahead/behind、`12 changes`、`UTF-8`、`LF`、`TypeScript` |

主体 742 = 852 − 2 − 40 − 44 − 24；中列 798 = 1390 − 232 − 360；行区 687 = 742 − 32 − 23，按 32 高可容 21 行，板上画 10 行。

## 2. 标题栏与工具条

与 04 逐项相同：页签 27 高、内距左 10 右 9、gap 7、最大宽 178、字号 12，当前页签 `content` 底 + `line-strong` 框 + 5×5 `accent` 圆点；右端 6×6 `ok` 圆点 + `Fetched 2m ago`（11 `text3`）+ 齿轮 26×24。

工具条 `tbtn` 26 高、内距 0×9、图标 13、gap 5、字号 12/500，`tb-sep` 1×18 把按钮切成三簇；搜索框 176×26 带 `⌘F` 键帽 10；`mode-pill` 内距 2、项 22 高、内距 0×8、11.5，`History` 白底为当前项，`Changes 12` 链到 10；`Agent` 用 `accent-soft` 底 + `accent-2` 字 + 600，链到 12。

本板与 04 的两处差别：`History` 仍是当前分段项，而主体已是 reflog；`Agent` 是链接而不是按钮。

## 3. 侧栏

六组：Branches 5 / Remotes 2 / Tags 3 / Stashes 2 / Submodules 2 / Worktrees 3。组头 27 高、内距 0×6、上距 8、10.5/600 `text3`、字距 0、**不转大写**。行 25 高、内距 0×7、圆角 6、gap 7、字号 13。

当前分支 `main` 是 `sel` 行（`accent` 底 + 白字，`↑2`/`↓0` 也转白）：6×6 `g1` 圆点、`✓` 11、`↑2` 10 `ok`、`↓0` 10 `warn`。其余分支只有色点（`g2` `g3` `g4`、最末 `line-strong`）。Remotes 下每个 remote 先挂一行 URL（10.5、右端 `fetch`/`push`），再挂分支行（内联左内距 38），末行 `5 more branches…` / `6 more branches…`。Tags 用 7×7 方块点（`warn` .65）+ `latest`/`Oct 28`/`Sep 30`；Stashes 用归档图标 + 消息；Submodules 右端 `clean`（`ok`）/ `1 ahead`（`warn`）；Worktrees 用紫色文件夹 + 分支名。

## 4. Reflog 表

过滤条 32 高、内距 0×10、gap 7：`Reflog` 11.5/600 → 分支 chip（分支图标 10 + `main` + chevron 9，chip 22 高、内距 0×7、圆角 5、1px `line`）→ `42 entries` 10.5 → 弹性空隙 → `Every ref movement, including resets`（10.5 `text3`）→ `Filter` chip（漏斗 10）。

列头 23 高、内距 0×8、10.5/500 `text3`、不转大写，`When` 右对齐；四列与行同一网格 62 / 1fr / 64 / 44，消息列 612 = 798 − 16 − 62 − 64 − 44。

行 32 高、内距 0×8、行间 1px `line-soft`：hash 9 号 mono `text3`；消息 11.5（`HEAD@{n}` 加粗，右侧留 6）；操作徽章 17 高、内距 0×5、圆角 4、8/600、靠列首对齐；日期 9 号右对齐、等宽数字。选中行 `sel` 底 + 左侧 2px `accent` 竖条，hover `hover` 底。

徽章四色：`reset` 用 warn 14% 混 content + `warn` 字；`commit` 用 `accent-soft` + `accent-2`；`rebase` 用 purple 12% + `purple`；其余（`checkout` `pull` `clone`）统一 `panel-2` + `text3`，所以板上 `pull` 与 `clone` 都显示成 checkout 色。

## 5. Undo 检查器

页签条 33 高、内距 0 14（右端 8）、gap 14、字号 12：`Undo` 当前（600 + 底部 2px `accent`）、`History` 链回 04、弹性空隙、`⋯` 24×24（`Undo options`）。body 内距 12、底 `panel`：

- **恢复卡**：1px `line`、圆角 8、`accent-soft` 底、内距 11、gap 5。kicker `Recover` 10.5/700 `accent-2`（源码写全大写，末次覆盖把 `text-transform` 取消）；标题 mono 10.5/600 `Undo reset to HEAD~1`；正文 9.5/1.5；动作行 gap 6、上距 4，两个按钮 24 高、字号 10，主按钮 `flex:1` 居中。
- **明细**：上距 11、三行 9.5、gap 3 —— `From 8b41e07 · Fix diff hunk alignment`、`To 3f2a1c9 · Release 1.4.0`、`Ref main · 1 commit orphaned`，值用 `text2`。
- **说明**：10.5/1.5、上距 10。
- **`Show in History`**：`btn` 24 高、`justify-content:center`、上距 12，整宽，链 04；它直接排在 body 末尾，没有单独的检查器页脚。

## 6. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 选中条目 | 第 1 行 / 其它 / 无 | 只画第 1 行 `on` |
| 操作类型 | reset / commit / checkout / rebase / pull / clone | 6 种标签都出现，徽章只有 4 色（pull、clone 复用 checkout 色） |
| 恢复卡 | reset 型 / 其它型 | 只画 reset 型 |
| 检查器页签 | Undo / History | 只画 Undo |
| 过滤 | 分支 chip / Filter | 只有 `main` 与空 Filter |

```
进入：无入链——没有任何板 href 到本板（04 的 `⋯` 菜单与 Info 页签都没画 Reflog 入口）
切换：点行换选中，右侧恢复卡跟着换；◄ ► 之类的条目导航没有
离开：History 页签 / Show in History → 04；分支 chip 也回 04
```

## 7. 画板没覆盖的组合（待补设计）

1. **非 reset 条目**：选中 `commit` / `checkout` / `rebase` / `clone` 行时右侧画什么，板上只有 reset 一种卡片；`Restore 8b41e07` 的按钮文案也绑定了 sha。
2. **恢复之后**：点 `Restore 8b41e07` 或 `Create branch` 的结果、失败、冲突、二次确认都没有画。
3. **`⋯` 菜单**：`Undo options` 只有图标，展开内容没有画，21 与 22 里也没有对应菜单。
4. **Filter**：reflog 的过滤条件（操作类型、作者、时间）没有弹层；05 是提交过滤，不能复用。
5. **入口**：本板没有入链，`42 entries` 之外也没有「更多 / 加载」的表现。
6. **列头语义**：`Action` 盖在消息列上、`Operation` 盖在徽章列上，与单元格内容（`HEAD@{n}` + 消息 / 操作徽章）对不上。
7. **空 reflog / 单条目**：新仓库只有 `clone` 一条时的样子没画。

## 8. 按钮与链接清单

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 标题栏页签右 | `+`（`aria-label=Open repository`） | 图标 27×27 | 回 01 | 常驻 |
| 2 | 标题栏右 | 齿轮（`aria-label=Settings`） | 图标 26×24 | 去 23 | 常驻 |
| 3 | 工具条 | Fetch / Pull / Push / Branch / Stash | `tbtn` 26 高 | 同步与建引用 | 板上都不禁用 |
| 4 | 工具条 | `Agent` | `tbtn` 26 高 + `accent-soft` | 去 12 | 本板是链接 |
| 5 | 工具条两端 | 侧栏 / 检查器开关（`aria-label=Toggle sidebar` / `Toggle inspector`） | 图标 26×24 | 收起两栏 | 无按下态 |
| 6 | 过滤条 | `main` + chevron | chip 22 高 | 回 04 | 当前分支筛选入口 |
| 7 | 过滤条右 | `Filter` + 漏斗 | chip 22 高 | 过滤 reflog | 无展开态 |
| 8 | 检查器页签 | `Undo` | 页签 12 | 本页 | 当前项 600 + 2px 下划线 |
| 9 | 检查器页签 | `History` | 页签 12 | 回 04 | 链 `Workspace.dc.html` |
| 10 | 检查器页签右 | `⋯`（`aria-label=Undo options`） | 图标 24×24 | 更多撤销选项 | 板上没有展开菜单 |
| 11 | 恢复卡 | `Restore 8b41e07` | `btn primary` 24 高 / 10 | 把 main 移回 8b41e07 | 常驻 |
| 12 | 恢复卡 | `Create branch` | `btn` 24 高 / 10 | 在该提交建分支 | 常驻 |
| 13 | body 底部 | `Show in History` | `btn` 24 高、整宽、居中 | 回 04 定位该提交 | 链 `Workspace.dc.html` |

- **整行可点**：10 条 reflog 行（选中行 `aria-current` 未标，只有 `on` 类）与侧栏 30 行引用，不计入按钮。
- 所有图标按钮都带 `aria-label`；板上没有 macOS 菜单栏。

## 9. 文案要点

- kicker 只写一个词 `Recover`，标题是命令原文 `Undo reset to HEAD~1`，用 mono 表示这是 reflog 条目而不是句子。
- 正文给因果：`main moved from 8b41e07 to 3f2a1c9. Restoring moves main back; the working tree is preserved.`
- 说明段解释安全性：`A reset is safe to undo: no commit is deleted, and the working tree is untouched. Use Create branch to keep both lines.`
- 过滤条右边那句 `Every ref movement, including resets` 是本页唯一的范围说明。

## 10. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 04 Commits | 三处 `href`：分支 chip、`History` 页签、`Show in History` |
| 10 Staging & commit | `Changes 12` 分段项链到它 |
| 12 Agent panel | 工具条 `Agent` |
| 23 Settings / 01 Welcome | 齿轮 / 页签 `+` |
| 21 Dark & context menu / 22 Menus & dialogs | 两板的菜单都没有 Reflog 与「撤销上一次 reset」条目，本页因此没有入口 |
