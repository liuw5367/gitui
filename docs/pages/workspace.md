# 提交历史 — 布局、状态与按钮

- **画板**：04 Commits（有授权横幅 · 有检查器 · 选中第一条提交；无空仓库、无搜索态、无多选）
- **来源**：`designs/graft-git/project/Workspace.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

板内 `.window` 没有内联尺寸，取样式表的 `height:calc(100vh - 48px)`：预览 1440×900、桌面四周留白 24，窗口实际 **1392×852**（圆角 12、1px 描边）。五段竖排：标题栏 / 工具条 / 授权横幅 / 三栏主体 / 状态栏；框内只放板上的英文与符号。

## 当前实现

当前工具栏依次提供 Fetch／Pull／Push、Stash、Reflog；Stash 打开 Stash Changes 表单，Stash 与 Reflog 复用同步按钮的颜色、尺寸和按压行为。标题栏右侧的当前分支按钮显示分支名与 upstream ahead／behind 数；点击后打开单选分支弹层，可搜索并选择本地或远端分支，也可从行尾跳转到 History。状态栏不重复显示分支，临时操作提示出现时位于左侧，右侧显示工作区改动数、UTF-8 与 LF。以下画板骨架保留设计来源。

## 1. 骨架

```
┌───────────────┬─────────────────────────────────────────────┬───────────────────────────────┐
│ ● ● ●  [gitui ×] [docs-site ×] [atlas-web ×] [+]     ● Fetched 2m ago   [gear]              │
├─────────────────────────────────────────────────────────────────────────────────────────────┤
│ [=] | Fetch Pull Push | Branch Stash   [search commits ⌘F] | History Changes 12 | Agent [=] │
├─────────────────────────────────────────────────────────────────────────────────────────────┤
│ ℹ Authentication required for origin. Sign in to fetch or push. [Sign in…][Use SSH]         │
├─────────────────────────────────────────────────────────────────────────────────────────────┤
│ Branches 5    │ History                                     │ Fix diff hunk alignment       │
│ ● main * +2-0 │ main 2  128 commits   Any time              │ MC Mia Chen  2 hours ago      │
│ ● feature/dif │ Compare      All authors  [filter]          │ 3f2a1c9 [copy]                │
│ Remotes 2     │ Description    Author       Date            │ Committed on main · 7 files   │
│   origin 8    │ | Fix diff hunk alignment  Mia Chen 2h      │  Changes 7  Files  Info       │
│ Tags 3        │ | Add split/unified   Jonas Weber  5h       │ 7 files changed +162 -62      │
│ ▣ v1.4.0      │ | Release 1.4.0       Mia Chen     1d       │ [tree] [flat]                 │
│ Stashes 2     │ | Merge feature/agent Mia Chen     3d       │ src/git                       │
│ Submodules 2  │ | Agent: propose      Priya Nair   4d       │  diff-engine.ts M +18 -7      │
│ Worktrees 3   │ | Add hunk staging    Sam Okafor   5d       │  hunk.ts A +44 -0             │
│               │ | Add keyboard nav    Jonas Weber  6d       │ (empty folder merged)         │
│               │                                             │ Open in Editor  [copy] [more] │
├─────────────────────────────────────────────────────────────────────────────────────────────┤
│ main   ● 2 ahead · 0 behind        12 changes   UTF-8   LF   TypeScript                     │
└───────────────┴─────────────────────────────────────────────┴───────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 标题栏 | 顶部通栏 | 高 40（内联 `height:40px` 覆盖样式表的 44）；内距 0×13 | 窗口灯 12×3、三个仓库页签、`+`、右端状态点与齿轮 |
| 工具条 | 标题栏下 | 高 44；内距 0×10；间距 6 | 侧栏开关、Fetch/Pull/Push、Branch/Stash、搜索、History·Changes、Agent、检查器开关 |
| 授权横幅 | 工具条下 | 高 30；内距 0×12 | 信息图标 14 + 一句警告 + 两个小按钮 |
| 侧栏 | 主体左 | 宽 232（含右边框 1）；内距 2/8/12 | 六组引用：Branches 5 / Remotes 2 / Tags 3 / Stashes 2 / Submodules 2 / Worktrees 3 |
| 历史区 | 主体中列 | 宽 798（内含过滤条 32、列头 23、行 28） | 四点过滤条、四列提交表（板上 23 行） |
| 检查器 | 主体右 | 宽 360（含左边框 1）；头内距 12/14/10 | 提交头、页签 33、可滚 body、页脚 |

画板底部状态栏高 24（内距 0×12、11）：分支、`● 2 ahead · 0 behind`、`12 changes`、`UTF-8`、`LF`、`TypeScript`。主体高 712 = 852 − 2 − 40 − 44 − 30 − 24；中列 798 = 1390 − 232 − 360。提交行四列 `64 / 580 / 96 / 58`：图列 64、作者列 96、日期列 58，描述列吃掉余下 580。图中 `*` 是对勾图标，`[gear]` 是齿轮，`[=]` 是侧栏/检查器开关，`[filter]`/`[tree]`/`[flat]`/`[copy]`/`[more]` 是图标按钮。

## 2. 标题栏与工具条

页签 `rtab` 高 27、圆角 6、内距左 9 右 4、间距 7、最大宽 178（超出省略），字号 12；活动页签 `content` 底 + `line-strong` 描边 + 状态圆点，字重 500。状态圆点是 6×6 的 `ok` 绿点，只画在活动页签上，未选中的页签不画点（画板是 5×5 的 `accent` 圆点，实现在 `DEVIATIONS.md` #36 登记）。`×` 的显隐：活动页签 0.35、其余 0.55，指针进入页签时 1.0（画板的活动页签是 0，见 #37）。右端是 6×6 `ok` 圆点 + `Fetched 2m ago`（11 `text3`）+ 齿轮 26×24。

仓库标签按稳定身份打开、切换、关闭和拖动；拖到目标标签中点后释放即可调整顺序，活动仓库会随重排保持不变。右键标签可关闭该标签、复制仓库路径、在 Finder 中显示仓库目录、关闭其右侧或左侧标签，或关闭全部标签；左右关闭以右键标签为锚点并保留它，原活动标签仍存在时继续保持活动，否则切到锚点。标签栏**按内容取宽**：`+` 紧跟最后一个标签页右侧（不是标题栏最右端），页签多到放不下时才收缩并横向滚动。标题栏从 NSH 动态读取原生窗口控制区宽度，再从其右侧开始排页签，确保红黄绿窗口按钮及其点击区域不被遮挡；标准 AppKit 标题栏附件适用于直接放在原生标题栏区域的自定义控件。活动页签的 `×` 平时以 0.35 不透明度淡显，指针进入该页签（含页签内的 `×` 本身）时提高至 1.0；其余页签的 `×` 平时为 0.55，悬停时同样提高至 1.0。`×` 保持 9pt 左内距，右内距收至 4pt。`+` 打开欢迎窗口选择仓库；标题栏这一行（页签与按钮之外）显式标记为拖动区，按住即拖动窗口。标题栏见 `DEVIATIONS.md` #38（`+` 的命中区）。重复打开同一规范化仓库会切回现有标签。关闭非活动标签保留活动仓库；关闭活动标签优先选择右邻，否则选择左邻。关闭标签页不提示，未提交改动保留在磁盘上。关闭最后一个标签会保存空会话、隐藏主窗口并显示欢迎页；关闭主窗口不提示，会保存当前标签顺序和活动仓库供下次启动恢复，是否恢复由 `Launch behavior` 决定；应用退出也会保存当前会话。

当前分支按钮是标题栏内可聚焦的原生语义按钮，保留 ahead／behind 计数并提供悬停、按下与可见焦点状态。单选弹层锚定在按钮下方，宽 320pt，列表高 288pt；打开时清空搜索并聚焦输入框，按 Esc 关闭后焦点回到按钮，切换活动仓库页签时弹层关闭。本地分支按名称排列、当前分支置顶并禁用切换；远端分支按 remote 分组，行内显示完整 remote／branch 名。搜索不区分大小写，可匹配完整名称或末段。工作区有改动时，本地分支显示「贮藏后切换并恢复」与「带过去」选项：前者记录新贮藏项，切换后按该项 OID 恢复改动；恢复冲突时保留贮藏项并在 Changes 中列出冲突。后者直接检出，若可能覆盖改动，Git 会拒绝切换。远端分支主区域打开已预填的跟踪分支表单；各行尾的 History 图标按完整 ref 导航，不切换当前分支。

工具条按钮 `tbtn` 高 26、内距 0×9、图标 13、间距 5、字号 12/500；Agent 是仅带 sparkle 图标的工具栏按钮，悬停提示和无障碍名称保留本地化的 Agent 标签。1×18 的 `tb-sep`（左右外距 3）把按钮切成三簇，搜索框 176×26 是唯一常驻输入。分段 `mode-pill`（`role=group`）容器内距 2、圆角 7、底 `panel-2`，两项高 22、内距 0×8、11.5，`History` 白底为当前项。

## 3. 侧栏

组头 `side-head` 高 27、内距 0×6、上距 8，10.5/600 `text3`，右端计数同号 500；板上**不转大写**——样式表末次覆盖 `text-transform:none`。行 `side-row` 高 25、内距 0×7、圆角 6、间距 7、字号 13，名字超长省略；当前分支 `main` 是 `sel` 行（`accent` 底 + 白字，`✓` 11 标「当前」，右端 `↑2` 10 `ok` / `↓0` 10 `warn`），其余分支只有 6×6 色点（`g1`–`g4`、最末 `line-strong`）。

Remotes 父行是地球 14 + 名 + 计数，每个 remote 下先挂一行 URL（内联 10.5、右端 `fetch`/`push`），再挂三条分支行（内联左内距 38），末行 `5 more branches…`（`text3`）。Tags 用 7×7 方块点（`warn` .65）+ 右端 `latest`/`Oct 28`/`Sep 30`；Stashes 用归档图标 + 消息；Submodules 右端 `clean`（`ok`）/`1 ahead`（`warn`）；Worktrees 用紫色文件夹 + 名 + 分支名。

实现中的六组标题都可点按折叠，并在组标题的右键菜单中展开或收起；折叠状态按仓库标签保留。默认展开 Branches 与 Remotes 外层；Branches 内部的前缀组和各 remote 默认折叠，Tags 组默认折叠。Branches 按分支名分层显示斜杠前缀（例如 `feature/ui` 收在 `feature` 下），同级先显示分组，再显示分支；分组内部、分支内部继续按文字升序排列。前缀右键菜单提供「展开 / 收起」和「展开所有下级分组 / 收起所有下级分组」。

Branch 行右键菜单依画板 22 的顺序包含：`Checkout`、`New Branch from '<name>'…`、`Compare with Current Branch`、`Rebase Current onto '<name>'`、`Merge '<name>' into Current`、`New Tag…`、`Rename…`、`Delete…`、`Copy Branch Name`、`Push '<name>' to origin`。当前分支的 Checkout、Rebase、Merge、Delete 禁用；Rebase 还要求当前 HEAD 位于分支上、工作区干净且没有其它 Git 操作；Push 在没有 `origin` 时禁用。会改写历史的 Rebase 会二次确认，冲突交给现有冲突面板处理。Branches 组头右键菜单另有 `New Branch…` 与组折叠操作。

## 4. 历史区

过滤条高 32、内距 0×10、间距 7：`History` 11.5/600 → 分支 chip `main 2` → `128 commits` 10.5 → 时间 chip `Any time` → 弹性空隙 → `Compare` chip → `All authors` chip → 漏斗图标按钮。chip 一律 22 高、内距 0×7、圆角 5、1px `line` 描边、`panel` 底、11，内部图标 10、chevron 9。列头高 23、内距 0×8、10.5/500 `text3`，与行同网格；`Description / Author / Date` 句首大写，`Date` 右对齐。

```
┌─────────┬─────────────────────────┬───────────┬───────┐
│ (graph) │ Description             │ Author    │ Date  │
│    |    │ Fix diff hunk alignment │ Mia Chen  │    2h │
│    |    │ [main] [origin/main]    │           │       │
└─────────┴─────────────────────────┴───────────┴───────┘
```

行高 28、内距 0×8 且底部再让 2；图列 64 里的 SVG 按 viewBox 64×38 画、用 50×26 渲染（竖线 1.5、节点半径 3.4，选中行 3.6），所以图线比列窄。描述 12.5 超长省略、选中行 500；引用徽章高 16、圆角 4、9.5/600——`branch` 蓝底蓝字、`tag` 黄底黄字 + 1px `tag-border` 内描边、`remote` 透明底 + 1px `line-strong` 描边（500），徽章区最宽 190。作者 11 `text2`，日期 10.5 `text3` 右对齐（`2h 5h 1d 2d 3d 1w 2w 3w 1mo 2mo 3mo`）。选中 `sel` 底 + 左侧 2px `accent` 竖条，hover `hover` 底，行间 1px `line-soft`。

## 5. 检查器

头部：标题 13/600 行高 1.32；下距 9 是作者行（头像 21、`MC` 9/700；姓名 11.5/600 + `2 hours ago` 10）；Sha 徽章 mono 10.5（`panel-2` 底、1px `line-soft`、内距 2×5、圆角 4、后跟 10 的复制图标）；再下一行 `Committed on` + 分支徽章 + `·` + `7 files`（10.5 `text3`）。页签条高 33、内距 0×14、间距 14、字号 12，当前项 600 + 底部 2px `accent`；计数标签 9.5（`panel-2` 底、圆角 4、内距 0×4、行高 14），当前项换 `accent-soft` 底 + `accent-2` 字。

body 内距 10/10/12：统计行（`7 files changed` 10.5/600 + `+162` + `−62`，下边框、下距 8）右端是树/平铺图标对（各 22×18）；文件组头高 23、上距 3、10.5/600 `text3`（chevron 10 + 文件夹 12 + 路径）。文件行高 27、内距 0×7、圆角 5、间距 6：文件图标 13、名字 mono 10.5、状态字母占 11 宽（9.5/700，`M` `warn`、`A` `ok`、`D` `danger`、`R` `purple`）、右端 `+18 −7` mono 9.5，当前行 `sel` 底。`Empty folder merged · src/git/legacy` 是 9.5 注记（内距 5×7、`panel-2` 底、圆角 5、8×8 紫方块、路径 mono 9）。

底部 `Commit details` 段上距 13、上边框、上内距 9：标签 9.5/600 大写字距 .04em，三行 10.5 键值（`Parents 8b41e07` / `Branches main · origin/main` / `Author Mia Chen <mia@gitui.dev>`），值右对齐可省略。页脚内距 8×12：`Open in Editor`（小按钮 22 + 外链 12），右侧复制与 `⋯` 各 24×24。

## 6. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 仓库视图 | History / Changes | 只画 History（`Changes 12` 未选中） |
| 提交选中 | 选中一行 / 都不选 | 只画第一行 `sel` |
| 检查器页签 | Changes / Files / Info | 只画 Changes |
| 授权横幅 | 显示 / 不显示 | 只画显示（`warn` 变体） |

```
进入：01 里打开仓库 → 本板（新页签）；切换：History ⇄ Changes 12 只换主体；离开：页签 × 关闭 / `+` 回 01 / 齿轮去 23 / Agent 去 12 / Changes 去 10
```

## 7. 画板没覆盖的组合（待补设计）

1. **空仓库 / 零提交**：历史表为空时画什么、检查器跟着空还是隐藏，板上没有任何一行（`handle empty repositories` 只出现在提交标题里）。
2. **搜索无结果**：有输入但零匹配时的列表与检查器没画，`⌘F` 之后是否替换列头也没定。
3. **多选提交**：`⌘`/`⇧` 点选后行怎么变、过滤条是否让位给选择条没画；06 直接以「3 commits selected」开场。
4. **过滤生效后的本页**：05 说 `128 → 46`，但本板只有 `128 commits` + `Any time` + `All authors` 的未过滤样子。
5. **侧栏折叠 / 同步中**：组头 chevron 收起后是否只留组头没画；Fetch/Pull/Push 按下后的进度、禁用与错误也没画，横幅只有「认证失败」一种。

## 8. 按钮与链接清单

共 15 组；分段、页签、整行可点的行不计入，见本节末。

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 标题栏页签右 | `+`（`aria-label=Open repository`） | 图标 27×27 | 打开 01 | 常驻 |
| 2 | 标题栏右 | 齿轮（`aria-label=Settings`） | 图标 26×24 | 打开 23 设置；macOS 应用菜单也提供 Settings（⌘,） | 常驻 |
| 3 | 工具条最左 | 侧栏图标（`aria-label=Toggle sidebar`） | 图标 26×24 | 收起/展开侧栏 | 板上无按下态 |
| 4 | 工具条 | Fetch / Pull / Push / Branch / Stash / Agent | 文字+图标 26 高 | 同步、建分支、存 stash、打开 12 | 板上都不禁用 |
| 5 | 工具条最右 | 镜像侧栏图标（`aria-label=Toggle inspector`） | 图标 26×24 | 收起/展开检查器 | 板上无按下态 |
| 6 | 横幅 | Sign in… / Use SSH | 小按钮 22 高 | 去 27 凭据 | 仅横幅出现时 |
| 7 | 过滤条 | `main 2` | chip 22 高 | 打开 05 | 分支过滤入口 |
| 8 | 过滤条 | Any time | chip 22 高 | 打开 05 | 时间过滤入口 |
| 9 | 过滤条 | Compare | chip 22 高 | 打开 06 | 常驻 |
| 10 | 过滤条 | All authors | chip 22 高 | 打开 05 | 作者过滤入口 |
| 11 | 过滤条右 | 漏斗（`aria-label=Filter commits`） | 图标 24×24 | 打开 05 | 05 里是 `on`（`sel` 底 + `accent` 图标） |
| 12 | 检查器统计行右 | 文件夹 / 三条横线（`aria-label=Tree view` / `Flat list view`） | 图标对 22×18 | 文件树 / 平铺 | 当前项白底 |
| 13 | 检查器页脚 | Open in Editor | 小按钮 22 高 + 外链图标 | 在编辑器打开 | 常驻 |
| 14 | 检查器页脚 | 复制（`aria-label=Copy commit SHA`） | 图标 24×24 | 复制 SHA | 常驻 |
| 15 | 检查器页脚 | `⋯`（`aria-label=More commit actions`） | 图标 24×24 | 更多提交动作 | 板上没有展开菜单 |

- **分段**：工具条 `History / Changes 12`（`role=group`，当前项白底）；**页签**：检查器 `Changes / Files / Info`（`role=tablist`，`Files` 链去 18，另两个是按钮）与三个仓库页签（整块可点，`aria-label` 形如 `gitui repository`）。**整行可点**：23 行提交（选中行 `aria-current=true`）、7 行文件、侧栏 30 行引用；图标按钮都带 `aria-label`，板上没有 macOS 菜单栏，`⌘F` 只以键帽出现在搜索框右端。

## 9. 输入与选项

工具条搜索框 176×26（占位 `Search commits`，右端 `⌘F` 键帽 10，聚焦时描边转 `accent` + 3px 光晕）。另有三处「二选一」：工具条 `History / Changes 12`（当前项白底）、检查器 `Tree view / Flat list view`（图标对 22×18）、`Changes / Files / Info`（页签 12，当前项 600 + 2px `accent` 下划线）。板上都没有禁用态与第三态。

## 10. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 05 History filters | 三个过滤 chip 与漏斗图标（板上 4 处 `href`） |
| 06 Multi-commit diff | 过滤条的 `Compare` |
| 04b File diff (focused) | 检查器第一条文件行 `src/git/diff-engine.ts` |
| 18 Files at commit | 检查器 `Files` 页签 |
| 10 Staging & commit / 12 Agent panel | `Changes 12` 分段项 / 工具条 `Agent` |
| 23 Settings / 27 Credentials / 01 Welcome | 标题栏齿轮 / 横幅两个按钮 / 页签 `+` |
