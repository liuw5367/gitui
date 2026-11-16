# 仓库状态 — 布局、状态与按钮

- **画板**：16 Repository states（有 6 张状态卡 · 有 `Back to Workspace` · 有加载骨架屏；无状态栏、无检查器、无恢复后的样子）
- **来源**：`designs/graft-git/project/RepositoryStates.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

本板是**独立的状态画廊页**，没有 `.window` 外壳：外层 `gallery-page`（`min-height:100vh`、内距 30×34、纵向 gap 14、desk1→desk2 渐变底），预览 **1440×1000**。因此没有 1392×852 的窗口、没有标题栏/工具条/状态栏；页内只有一行标题和一个三列栅格，每格一张「说明 + 238 高的缩微窗口」。缩微窗口不是等比缩放：整体 238 高，对应标题栏 25、工具条 22、侧栏 64、主体 189，只画到主体，六张卡都没有状态栏与检查器。

## 1. 骨架

```
┌────────────────────────────────────────────────────────────────────────────────┐
│ Repository states                              [ Back to Workspace ]           │
│ In-context banners, empty states and failures.                                 │
│ The same banners appear in 03 and 06.                                          │
├──────────────────────────┬──────────────────────────┬──────────────────────────┤
│ EMPTY REPOSITORY         │ DETACHED HEAD            │ MERGE IN PROGRESS        │
│ window: gitui - empty    │ window: gitui - 3f2a1c9  │ window: gitui - merging  │
│ side: 6 bars, 1st on     │ side: 6 bars, 2nd on     │ side: 6 bars, 2nd on     │
│ main: centered empty     │ main: warn banner        │ main: soft banner        │
│ state + 1 button         │ + 4 rows, btn right      │ + 4 rows (2 x 'C')       │
│ btn: Create initial      │ btn: Create branch       │ btn: Resolve             │
├────────────────────────────────────────────────────────────────────────────────┤
│ OFFLINE                  │ AUTH REQUIRED            │ LOADING                  │
│ window: gitui            │ window: gitui - remote   │ window: gitui - loading  │
│ side: 6 bars, 2nd on     │ side: 2nd on             │ side: none on            │
│ main: danger banner      │ main: danger banner      │ main: 8px label          │
│ + 4 rows                 │ + 2 buttons right        │ + 5 skeleton bars        │
│ btn: Retry               │ btns: Sign in, SSH       │ no button                │
└──────────────────────────┴──────────────────────────┴──────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 页面 | 整页 | 预览 1440×1000；内距 30×34；gap 14 | 标题行 + 状态栅格 |
| 标题行 | 页顶 | 高约 44（标题 20 + 2 + 副标 10.5）；右端按钮 22 高 | `Repository states`、一句副标、`Back to Workspace` |
| 状态栅格 | 标题下 | 3 列 `minmax(0,1fr)`；列距 22；`flex:1` | 6 张卡，2 行 |
| 状态卡 | 栅格内 | 宽 442.7；卡内 gap 7 | 说明行 + 238 高缩微窗口 |

内容宽 1372 = 1440 − 68，列宽 442.7 = (1372 − 44) / 3。

## 2. 说明行与缩微窗口外壳

说明行 `state-cap`：9/600、字距 .05em、**全大写**（源码是句首大写，靠 CSS 转大写），左端 7×7 圆角 2 色块 `#0A66E8` 透明度 .7。

缩微窗口 `state-window`：高 238、1px `#C9CDD6`、圆角 9、底 `content`、阴影 0 8 22（.08）、`overflow:hidden`、纵向 flex。里面只有三段：标题栏 25、工具条 22、主体 189。

标题栏：内距 0×8、gap 5，左端三个 6×6 圆点（`#FF5F57`/`#FEBC2E`/`#28C840`，间距 3），标题 8 号 `text3` 居中。工具条：内距 0×8、gap 5，三个 11 高圆角 3 的 `panel-2` 胶囊，宽 42 / 24 / 30。主体：侧栏宽 64（含右框 1）、内距 6、gap 5，六根 6 高圆角 2 的条（`line-soft`，当前项 `accent-soft`）；主列余下 376.7、内距 8、gap 5。

## 3. 卡内主体

```
┌──────────────────────────────────────────────────────────┐
│ ● ● ●            gitui - empty-repo                      │
├──────────────────────────────────────────────────────────┤
│ [======] [==] [===]                                      │
├────────────┬─────────────────────────────────────────────┤
│ ▣▣▣▣       │             [ icon 24 ]                     │
│ ▣▣▣▣       │           No commits yet                    │
│ ▣▣▣▣       │   This repository has no commits. Create the│
│ ▣▣▣▣       │   first one to start tracking history.      │
│ ▣▣▣▣       │       [ Create initial commit ]             │
└────────────┴─────────────────────────────────────────────┘
```

- **横幅**：最小高 24（含内距 4×7）、圆角 4、gap 5、8 号字、行高 1.3；三档底色——默认 `accent-soft`，`.warn` 用 warn 12% 混 content，`.danger` 用 danger 10% 混 content；状态名加粗，右端动作靠 `flex:1` 的空隙推到边上。
- **横幅按钮** `mini-btn`：15 高、内距 0×5、圆角 3、7 号字、`content` 底 + 1px `line`。
- **空态**：图标 24×24 圆角 6（`accent-soft` 底、`accent` 图标，下距 2）、标题 10/600、正文 7.5/1.35 最大宽 170、按钮 17 高 7.5 号（比横幅按钮高一档）。
- **列表行**：12 高、gap 5；hash 位 7 号 mono `text3`；占位条 5 高、宽 52% / 35% / 70%。样式表另有 4×4 的 `dot`，六张卡都没用。
- **骨架**：5 根 7 高圆角 3 的条，宽 70 / 45 / 88 / 60 / 70%，1.6s 循环的斜向扫光。

## 4. 六张卡逐张

| # | 说明行 | 窗口标题 | 侧栏高亮 | 主体画了什么 | 动作 |
|---|---|---|---|---|---|
| 1 | `Empty repository · no commits` | `gitui — empty-repo` | 第 1 根 | 居中空态：`No commits yet` + `This repository has no commits. Create the first one to start tracking history.` | `Create initial commit` |
| 2 | `Detached HEAD` | `gitui — 3f2a1c9` | 第 2 根 | `warn` 横幅 `Detached HEAD at 3f2a1c9` + 4 行（`3f2a1c9` `8b41e07` `c72d9a4` `e19a3f2`） | `Create branch` |
| 3 | `Merge in progress` | `gitui — merging` | 第 2 根 | 默认 `accent-soft` 横幅 `Merging origin/main · 2 conflicts` + 4 行（前两行 hash 位是 `C`，后两行空） | `Resolve` |
| 4 | `Offline` | `gitui` | 第 2 根 | `danger` 横幅 `Offline · last fetched 2h ago` + 4 行 hash | `Retry` |
| 5 | `Authentication required` | `gitui — remote` | 第 2 根 | `danger` 横幅 `Authentication required for origin` + 4 行（hash 位 `HTTPS`、`SSH`、空、空） | `Sign in` + `SSH` |
| 6 | `Loading repository` | `gitui — loading` | 无 | 8 号 `Loading repository…` + 5 根骨架条 | 无 |

每张缺什么：

1. **空仓库**：侧栏仍是 6 根占位条，`Branches` 组头与 0 计数的样子没画；有未跟踪文件时 `Changes` 页签的空态也没有。
2. **Detached HEAD**：侧栏第 2 根高亮却没有分支名；离开 detached 后横幅是否消失、`Create branch` 是否就是 22 的 New Branch 对话框，都没定。
3. **合并中**：写 `2 conflicts`，14 写 `3 files · 3 conflicts`；横幅用默认 `accent-soft` 而不是 warn/danger，和 04 的 30 高 warn 横幅不同一套。
4. **离线**：没有「离线时 Fetch/Pull/Push 是否禁用、操作是否排队」；`last fetched 2h ago` 与 04 标题栏 `Fetched 2m ago` 的口径没说明。
5. **需认证**：同一状态在 04 写 `Sign in to fetch or push.` 配 `Sign in…` / `Use SSH`（30 高 warn 横幅、20 高按钮），这里写 `Sign in` / `SSH`（24 高 danger 横幅、15 高按钮），文案与色调都不一致。
6. **加载中**：没有时长、进度、取消、部分内容；5 根骨架条只是装饰，与真实行数无关。

## 5. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 仓库状态 | 空 / detached / 合并中 / 离线 / 需认证 / 加载中 | 6 种各一张 |
| 横幅 | 有 / 无 | 空态与加载态没有 |
| 横幅色调 | `accent-soft` / `warn` / `danger` | 合并中 / detached / 离线与认证 |
| 动作数 | 1 个 / 2 个 / 0 个 | 4 张 1 个、认证 2 个、加载 0 个 |

```
切换：同一窗口内横幅出现或消失，主列内容下移 24（横幅高）+ 5（主列 gap）
入链：没有任何板 href 到本板，只能从文件列表单独打开
出链：Back to Workspace → 04
```

## 6. 画板没覆盖的组合（待补设计）

1. **横幅与三栏的关系**：六张卡都没有状态栏与检查器，04 里「有横幅时主体 742 → 712」的规则在卡里没有体现。
2. **多横幅**：离线 + 需认证、合并中 + detached 这类叠加没有画，也没有说是否只留最高优先级一条。
3. **动作后的样子**：`Retry` 失败、`Sign in` 取消、`Create branch` 确认之后横幅变成什么，都没画。
4. **合并中的冲突计数**：`2 conflicts` 与浮层里两个 `U` 行是否同源没写。
5. **副标过期**：页副标写 `The same banners appear in 03 and 06`，当前画板没有 03，06 也没有仓库横幅。
6. **窄窗**：三列在 1080 / 860 断点下如何换列没画；`@media` 整块剔除后看不到。
7. **加载与空仓库的其它栏**：`Fetch` 与搜索禁用、侧栏 skeleton 都缺。

## 7. 按钮与链接清单

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 标题行右 | `Back to Workspace` | `btn small` 22 高 / 11 号 | 回 04 | 常驻（唯一带 `href` 的控件） |
| 2 | 空仓库卡 | `Create initial commit` | mini-btn 17 高 / 7.5 | 建首个提交 | 仅第 1 张 |
| 3 | Detached 卡 | `Create branch` | mini-btn 15 高 / 7 | 在 HEAD 建分支 | 仅第 2 张 |
| 4 | 合并中卡 | `Resolve` | mini-btn 15 高 / 7 | 进 14 | 仅第 3 张 |
| 5 | 离线卡 | `Retry` | mini-btn 15 高 / 7 | 重试拉取 | 仅第 4 张 |
| 6 | 认证卡 | `Sign in` | mini-btn 15 高 / 7 | 去 27 凭据 | 与 #7 并排，左 |
| 7 | 认证卡 | `SSH` | mini-btn 15 高 / 7 | 切 SSH 传输 | 与 #6 并排，右 |

六个缩微窗口都标了 `aria-hidden="true"`，卡内按钮是画面的一部分，不进入可访问树，也没有无障碍名。整张卡不可点。

## 8. 文案要点

- 说明行统一「状态名 · 条件」：`Empty repository · no commits`；`Detached HEAD` 与 `Offline` 不带条件。
- 空态正文给下一步（`Create the first one to start tracking history.`），不是「暂无数据」。
- 横幅模板：加粗状态名 + 一句条件 + 右端动作；认证卡是唯一两个动作的。
- 窗口标题用 em dash 连接状态（`gitui — merging`），只有离线卡是光秃秃的 `gitui`。

## 9. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 04 Commits | 唯一出链（`Workspace.dc.html`）；认证横幅在 04 是 30 高 warn 版，本板是缩微 danger 版 |
| 14 Merge conflict | 合并中卡的 `Resolve` 指向同一流程，14 的入链来自 10 的冲突横幅 |
| 15 Sync & auth | 离线、认证失败的完整面板在 15，本板只给横幅 |
| 27 Settings · Credentials | 认证卡的 `Sign in` 应对应它，板上没有 `href` |
| 10 Staging & commit | 空仓库卡没有画 `Changes` 页签，与 10 的提交入口对不上 |
