# 快捷键设置 — 布局、状态与按钮

- **画板**：25 Settings · Shortcuts（设置窗口第 2 项 Shortcuts；有搜索行、一张录制卡、Global 4 / Repository 6 / Agent 3 三组十三行；无冲突行、无搜索结果、无“已清空”行）
- **来源**：`designs/graft-git/project/SettingsShortcuts.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

本分区是设置窗口的第 2 项（`Application` 组的第二个）。外壳与 23 General 同一套：`.settings .window`
`width:min(920px,100%)`、`height:min(700px,calc(100vh - 48px))`，画布 1440×900、`.desktop` 留白 24，因此窗口
**920×700**。内容区比别的分区多两层：标题下面先是一条工具栏（搜索框 + `Restore Defaults`）与一行提示，
再是一张高亮的录制卡，之后才进入三段快捷键列表。

## 1. 骨架

```
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ ● ● ●                                      Settings                   gitui 0.9.2            │
├─────────────────────────┬────────────────────────────────────────────────────────────────────┤
│                         │ Shortcuts                                                          │
│ ▣ gitui                 │ Global, repository, and agent actions.                             │
│   Version 0.9.2         │ [search shortcuts   ]                    [Restore Defaults]        │
│                         │ Click Record, then press a new combination.                        │
│ Application             │ Conflicts are highlighted.                                         │
│  [gear]   General       │                                                                    │
│* [keys]   Shortcuts     │ ● Recording · Toggle Command Palette                               │
│  [branch] Agent         │   [⌘] [P] |                                                        │
│ Git                     │ Press a new combination · [esc] cancel ...                         │
│  [branch] Git           │                                                                    │
│  [lock]   Credentials   │ Global · 4 shortcuts                                               │
│ About                   │Toggle Command Palette [⌘] [P]      [Record]                        │
│  [i]      About gitui   │Open Repository…       [⌘] [O]      [Record]                        │
│                         │Switch Repository      [⌘] [1…9]    [Record]                        │
│                         │Open Settings          [⌘] [,]      [Record]                        │
│                         │                                                                    │
│                         │ Repository · 6 shortcuts                                           │
│                         │ Agent · 3 shortcuts                                                │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ ● Changes are saved automatically                                                   [ Done ] │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 标题栏 | 顶部通栏 | 高 38；内距 0×13 | 窗口灯 12×3、居中 `Settings` 12.5/600、右端 `gitui 0.9.2` 11 |
| 左侧分区树 | 标题栏下左 | 宽 208；内距 12×8；行高 28 | 六项三组，`Shortcuts` 是当前项 |
| 内容区 | 标题栏下右 | 宽 710；内距 24×28 → 内容列 654、可视高 568 | 标题、工具栏、提示、录制卡、三段列表 |
| 页脚 | 底部通栏 | 高 44；内距 0×16 | 6 状态点、自动保存说明、右端 `Done` |

正文 616 = 700 − 2 − 38 − 44，内容列 654 = 710 − 28×2，可视高 568 = 616 − 24×2。三段十三行加录制卡与工具栏
累计约 1070，右栏滚动。

## 2. 外壳与左侧分区树

左树六项三组与其它五块板逐字相同：`Application` 下 General（齿轮）、Shortcuts（键帽）、Agent（分支）；`Git` 下
Git（分支）、Credentials（锁）；`About` 下 About gitui（信息）。本板当前项是 `Shortcuts`，`.on` 取末次声明
`background:var(--accent)` + 白字白图标 + 500，图标是键帽图形。行 28 高、内距 0×8、圆角 6、字号 13、图标盒 16 宽；
组头 `.sg` 24 高、10.5/600 `text3`、sentence case。本板没有二级页签。

## 3. 内容头、工具栏与录制卡

标题 `Shortcuts` 17/700、字距 −.015em；副标题 `Global, repository, and agent actions.` 11.5 `text3`、上距 4。
工具栏内联 `margin-top:14px`、间距 8：左侧 240 宽 `set-field` 内是 13 的放大镜 + 无边框 input，占位
`Search shortcuts`；弹性空隙；右端 `Restore Defaults`（`btn small` 22 高）。工具栏下 6 是一行 `.hint`：
`Click Record, then press a new combination. Conflicts are highlighted.`，在板上折成两行；这段没有段头与上边框。

录制卡 `.shortcut-recorder`：上距 10、1px `--accent` 描边、圆角 8、`--accent-soft` 底、内距 `9px 11px`。头一行是
6 的 `--danger` 红点 + `Recording · ` + 600 的 `Toggle Command Palette`；下面 `.rec-keys`
（`margin:7px 0 5px`、间距 4）依次 `⌘` 键帽、`P` 键帽、一个 2×16 的 `--accent` 光标条（`rec-blink` 1.05s 步进闪烁）；
底行 `.rr-foot` 是 `Press a new combination · esc cancel · ⌫ clear`，其中 `esc` 与 `⌫` 是键帽，
其余是 10.5 的 `.hint` 文字。卡片只画了“正在录 `Toggle Command Palette`”这一种。

## 4. 三组快捷键

`Global`（段说明 `4 shortcuts`）四行：`Toggle Command Palette` `⌘P`（`Switch repository or run a command.`）、
`Open Repository…` `⌘O`（`Open a local repository.`）、`Switch Repository` `⌘1…9`（`Open the command palette for open repos.`）、
`Open Settings` `⌘,`（`Open this window.`）。

`Repository`（`6 shortcuts`）六行：`Fetch` `⌘⇧F`（`Fetch from origin.`）、`Pull` `⌘⇧P`（`Pull the current branch.`）、
`Push` `⌘⇧K`（`Push the current branch.`）、`Commit Staged` `⌘↵`（`Commit all staged changes.`）、
`New Branch…` `⌘⇧B`（`Create a branch from HEAD.`）、`New Tag…` `⌘⇧T`（`Tag HEAD.`）。

`Agent`（`3 shortcuts`）三行：`Focus Agent` `⌘J`（`Focus the agent composer.`）、`Approve & Run` `⌘⇧↵`
（`Approve the current agent plan.`）、`Agent Permissions` `⌘⇧,`（`Open repository agent permissions.`）。

## 5. 快捷键行的度量

每段是一个 `.settings-sec`（上距 20、上内距 16、上边框 1px `--line`），段标题 12.5/600、下距 2，段说明只写
条数（`4 shortcuts` 这类），10.5 `text3`、下距 12。行沿用 `.set-row`：两列 flex、间距 14、内距 `11px 0`、
下边框 `--line-soft`、末行无边框；标签 12.5/500，说明 10.5 `text3`、上距 2、行高 1.4、可折行。

右列 `.sr` 间距 6，内容是 `.shortcut-keys`（间距 3 的一组键帽）加一枚 `Record`（`btn small ghost`，22 高、
透明底、无描边、`text2` 字）。键帽 `.kbd`：字号 10、`--sans`、`text3`、1px `--line` 描边、圆角 4、内距 0×4、
行高 15、`--panel` 底；所以一个三键组合大约 3×（字符宽 + 8）+ 2×3，靠右排列。本板 13 行每行只有一个 `Record`。

## 6. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 左侧分区 | 六项 | 只画 Shortcuts 选中 |
| 录制 | 空闲 / 录制中 / 冲突 | 只画录制中（`Toggle Command Palette`） |
| 单行与搜索 | 有键位 / 已清空；搜索无输入 / 有结果 / 无结果 | 13 行都有键位，搜索只有占位 `Search shortcuts` |
| 分组折叠 | 展开 / 收起 | 三段都展开 |

```
分区：General ⇄ Shortcuts ⇄ Agent ⇄ Git ⇄ Credentials ⇄ About gitui（只替换右栏）
录制：点 Record → 卡片出现并接管键盘；esc 取消，⌫ 清空，再按组合即写入
离开：Done → Workspace.dc.html
```

## 7. 画板没覆盖的组合（待补设计）

1. **冲突**：提示说 `Conflicts are highlighted.`，但冲突行、冲突后是否允许保存、与系统保留键（如 `⌘Q`）冲突怎么办，都没有画。
2. **搜索结果**：有输入时的过滤规则（按名称、按操作还是按段）、无结果空态、搜索是否自动展开折叠的段，都没有画。
3. **录制结束与重置**：按下非法组合（单字母、Esc 之外的保留键）时的反馈、录制卡是常驻还是收起、`⌫` 清空后行里
   显示什么、`Restore Defaults` 是否二次确认与重置范围，都没有画。
4. **计数与作用域**：`4 / 6 / 3 shortcuts` 是硬编码还是随行数变没有画，也没有“已改过”的标记；`Switch Repository ⌘1…9`
   按到第 10 个仓库怎么办、仓库级与全局的覆盖关系也没有画。

## 8. 按钮与链接清单

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 工具栏右端 | Restore Defaults | `btn small` 22 高 | 恢复默认键位 | 板上无确认、无禁用 |
| 2 | 每行右列 | Record | `btn small ghost` 22 高 | 进入录制 | 13 行各一个，没画按下态 |
| 3 | 页脚右端 | Done | `btn` 24 高、字号 12 | 关设置回主窗口 | 常驻 |
| 4 | 左侧树 | 六项 | 整行链接 28 高，图标 14 | 切分区，替换右栏 | 当前项 `--accent` 底 + 白字 |

- 键帽 `⌘` `P` `esc` `⌫` 都是 `.kbd` 文本，不是按钮；录制卡里那条 2×16 的竖线是光标装饰，不接收点击。
- 搜索框是唯一的文本输入，右端没有清除按钮，也没有键帽提示（`⌘F` 在主窗口搜索框上，本板没画）。

## 9. 输入与选项

| 字段 | 形态 | 默认值（画板） | 说明 |
|---|---|---|---|
| Search shortcuts | 240 宽字段 + 13 放大镜 | 空，占位 `Search shortcuts` | 内层 input 无描边，11 号 |
| Toggle Command Palette | 键帽对 + Record | `⌘` `P` | 录制卡正在录的就是它 |
| Open Repository… | 键帽对 + Record | `⌘` `O` | — |
| Switch Repository | 键帽 + 范围 + Record | `⌘` `1…9` | 一个键帽里带范围符号 |
| Open Settings | 键帽对 + Record | `⌘` `,` | — |
| Repository 段 6 行 | 键帽 2–3 枚 + Record | `⌘⇧F` `⌘⇧P` `⌘⇧K` `⌘` `↵` `⌘⇧B` `⌘⇧T` | Fetch / Pull / Push / Commit Staged / New Branch… / New Tag… |
| Agent 段 3 行 | 键帽 2–3 枚 + Record | `⌘` `J`、`⌘⇧↵`、`⌘⇧,` | Focus Agent / Approve & Run / Agent Permissions |

键位本身不可直接编辑：行里只有 `Record`，改键必须先进入录制态，这是本板唯一一种写入口。

## 10. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 23 / 23b / 24 / 26 / 27 | 板内五个 `href`：`Settings.dc.html`、`SettingsAgent.dc.html`、`SettingsGit.dc.html`、`SettingsCredentials.dc.html`、`SettingsAbout.dc.html` |
| 20 Command palette / 12 Agent panel / 13 Agent permissions | `Toggle Command Palette` 与 `Switch Repository` 的说明都写着命令面板；Agent 段三个键位指向后两块板 |
| 04 Commits（Workspace） | `Fetch / Pull / Push / Commit Staged / New Branch / New Tag` 在主窗口工具条与菜单里生效 |
