# Git 设置 — 布局、状态与按钮

- **画板**：24 Settings · Git（设置窗口第 4 项 Git；有 Executable / Repository discovery 两段，含两张扫描卡；无扫描中、无空结果、无移除确认）
- **来源**：`designs/graft-git/project/SettingsGit.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

本分区是设置窗口的第 4 项（`Git` 组的第一项，树里与分区同名）。外壳与 23 General 同一套：`.settings .window`
`width:min(920px,100%)`、`height:min(700px,calc(100vh - 48px))`，画布 1440×900、`.desktop` 留白 24，因此窗口
**920×700**。内容区仍是“标题 + 副标题 + 逐段”，但本板第二段不用 `.set-row`，换成两张 `.settings-scan` 卡片。

## 1. 骨架

```
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ ● ● ●                                      Settings                   gitui 0.9.2            │
├─────────────────────────┬────────────────────────────────────────────────────────────────────┤
│                         │ Git                                                                │
│ ▣ gitui                 │ Executable, repository discovery, and scanning.                    │
│   Version 0.9.2         │                                                                    │
│                         │ Executable                                                         │
│ Application             │ gitui runs the selected Git binary directly, without a shell       │
│  [gear]   General       │  wrapper.                                                          │
│  [keys]   Shortcuts     │Git executable    [ /usr/bin/git        ] [Choose…] [Detect]        │
│  [branch] Agent         │                  Detected · Git 2.43.0 (Apple Git-154)             │
│ Git                     │Use system Git    ☐                                                 │
│* [branch] Git           │ Repository discovery                                               │
│  [lock]   Credentials   │ Scan folders for .git directories. Submodules and worktrees are    │
│ About                   │  included.                                                         │
│  [i]      About gitui   │ Scan folders                                    2                  │
│                         │   [dir] ~/Developer           8 repositories   -                   │
│                         │   [dir] ~/Work                4 repositories   -                   │
│                         │   [+ Add Folder…] [Scan Now]         Last scan 2m ago              │
│                         │ Found 12 repositories                3 selected                    │
│                         │   ☑ gitui       ~/Developer/gitui                                  │
│                         │   ☑ atlas-web   ~/Work/atlas-web                                   │
│                         │   ☐ spectra     ~/Labs/spectra                                     │
│                         │   [Select All]              [Add 3 Repositories]                   │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ ● Changes are saved automatically                                                   [ Done ] │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 标题栏 | 顶部通栏 | 高 38；内距 0×13 | 窗口灯 12×3、居中 `Settings` 12.5/600、右端 `gitui 0.9.2` 11 |
| 左侧分区树 | 标题栏下左 | 宽 208；内距 12×8；行高 28 | 六项三组，`Git` 是当前项 |
| 内容区 | 标题栏下右 | 宽 710；内距 24×28 → 内容列 654、可视高 568 | `Git` 标题、副标题、两段（§4–§5） |
| 页脚 | 底部通栏 | 高 44；内距 0×16 | 6 状态点、自动保存说明、右端 `Done` |

正文 616 = 700 − 2 − 38 − 44，内容列 654 = 710 − 28×2，可视高 568 = 616 − 24×2。两行表单、两张扫描卡与
两处按钮条按画板值累加约 640，略超 568，右栏会轻微滚动。

## 2. 外壳与左侧分区树

左树六项三组与其它五块板逐字相同：`Application` 下 General（齿轮）、Shortcuts（键帽）、Agent（分支）；`Git` 下
Git（分支）、Credentials（锁）；`About` 下 About gitui（信息）。本板当前项是 `Git`，`.on` 取末次声明
`background:var(--accent)` + 白字白图标 + 500；组头 `.sg` 24 高、10.5/600 `text3`、sentence case。

## 3. 内容头与行网格

标题 `Git` 17/700、字距 −.015em；副标题 `Executable, repository discovery, and scanning.` 11.5 `text3`、上距 4。
`Executable` 段带内联 `border-top:0;margin-top:14px;padding-top:0`；`Repository discovery` 上距 20、上内距 16、
上边框 1px `--line`；段标题 12.5/600、下距 2，段说明 10.5 `text3`、下距 12。第一段两行用 `.set-row`（间距 14、
内距 `11px 0`、下边框 `--line-soft`），标签 12.5/500、说明 10.5 `text3`，写死 240 的字段对应标签列 400。

## 4. Executable

第一行 `Git executable`（说明 `Used for every command in this app.`）的右列是**竖排两行**：内联
`flex-direction:column;align-items:flex-end;gap:4`，上面一行是 240 字段 `[ /usr/bin/git ]` + `Choose…` +
`Detect`（三个控件间距 6），下面一行是 `--ok` 绿字 `.hint` `Detected · Git 2.43.0 (Apple Git-154)`。
因为右列高过左列，这一行比其它行高（26 + 4 + 15）。字段 `set-field` 26 高、内距 0×8、圆角 6、内层 input 11；
`Choose…` 与 `Detect` 都是 `btn small` 22 高，`Detect` 另加 `.ghost`（透明底、无描边、`text2` 字）。
第二行 `Use system Git from PATH`（说明 `Resolve git at launch instead of pinning a path.`）右列只有一枚未勾选的复选。

板上同时成立：路径已锚定 `/usr/bin/git`、状态显示已检测到 2.43.0、而“从 PATH 解析”没有勾；这与 23 General 的
`Use global config` 同一组合：字段有值，开关未开。

## 5. Repository discovery

段说明 `Scan folders for .git directories. Submodules and worktrees are included.`，主体是两张卡片。
**卡一（扫描目录）** `.settings-scan`：上距 12、1px `--line` 描边、圆角 8、`overflow:hidden`。
头 `.scan-head` 高 28、内距 0×9、间距 6、`--panel-2` 底、下边框 `--line-soft`、10/600 `text2`，文案 `Scan folders`，
右端 `.cnt` `2`（500 `text3`）。两行 `.scan-row` 各 30 高、内距 0×9、间距 8、11px、下边框 `--line-soft`：文件夹
图标 14（`text3`）+ `~/Developer`（500）+ 右端 mono 9.5 `8 repositories` + `−`（`.cl-more` 18×17、圆角 4、12px
`text3`，无障碍名 `Remove ~/Developer`）；第二行 `~/Work`、`4 repositories`。按钮条内联 `padding:8px`、间距 6：
`+ Add Folder…`、`Scan Now`（`small primary`，蓝底白字）、右端 `.hint` `Last scan 2m ago`。

**卡二（找到的仓库）** 上距 10，同一套卡片样式。头 `Found 12 repositories`，右端 `.cnt` `3 selected`。
四行是 `<label class="scan-row">`，每行一枚 `accent-color` 的复选 + 名字（500）+ 右端 mono 9.5 绝对路径：
`gitui ~/Developer/gitui`、`atlas-web ~/Work/atlas-web`、`docs-site ~/Work/docs-site` 三行勾选，`spectra` 未勾；
卡片下方一行是 `Select All`（`btn small`）+ 弹性空隙 + `Add 3 Repositories`（`small primary`），上距 8。头里写
`Found 12 repositories` 却只列四项，`3 selected` 与按钮同步，页面上没有“还有 8 条”的提示，也没有滚动条。

## 6. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 左侧分区 | 六项 | 只画 Git 选中 |
| Git 来源与检测 | 固定路径或 PATH；已检测 / 未检测 / 失败 | 只画固定路径 + 已检测（`Git 2.43.0`），PATH 未勾 |
| 扫描 | 目录 0/N 个、结果 0/N 个、上次扫描时间 | 只画 2 个目录、`Found 12`（列 4 行、选中 3）、`Last scan 2m ago` |

```
分区：General ⇄ Shortcuts ⇄ Agent ⇄ Git ⇄ Credentials ⇄ About gitui（只替换右栏）
扫描：Scan Now 刷新卡二与 Last scan；Select All ⇄ 逐行勾选
离开：Done → Workspace.dc.html
```

## 7. 画板没覆盖的组合（待补设计）

1. **扫描中与空结果**：`Scan Now` 的按钮状态、进度、`Last scan` 是否转成 `Scanning…` 没有画；`Scan folders` 为 0 或
   `Found 0 repositories` 时的空态与按钮禁用也没有画。
2. **超过四行**：`Found 12` 只列 4 行，其余 8 条是滚动还是分页没有画；`Select All` 之后按钮文案怎么变也没画。
3. **移除与添加**：`−` 是否二次确认、移除最后一个目录后卡片是否还在；`+ Add Folder…` 打开的目录选择器没有画。
4. **检测失败**：路径写错、二进制不可执行、版本过低时 `Detected · …` 那行怎么变，`Detect` 是否报错，没有画。
5. **PATH 模式**：勾上 `Use system Git from PATH` 后路径字段与两个按钮是禁用还是隐藏，没有画。
6. **窄窗口**：240 字段 + 两个按钮 + 回显行在 920 以下如何换行，没有画。

## 8. 按钮与链接清单

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | Executable 段 | Choose… / Detect | `btn small` 22 高 / 另加 `.ghost` | 选 git 可执行文件 / 自动探测 git 与版本 | Detect 的结果写在下方绿字 |
| 2 | 卡一每行右端 | `−`（`aria-label=Remove ~/Developer` / `Remove ~/Work`） | `.cl-more` 18×17 | 从扫描列表移除该目录 | 每行一个，板上无确认 |
| 3 | 卡一按钮条 | + Add Folder… | `btn small` 22 高 | 添加扫描目录 | 常驻 |
| 4 | 卡一按钮条 | Scan Now | `btn small primary` 22 高 | 立即扫描并刷新卡二 | 板上无进行中/禁用 |
| 5 | 卡二下方 | Select All | `btn small` 22 高 | 全选找到的仓库 | 板上无“已全选”态 |
| 6 | 卡二下方右 | Add 3 Repositories | `btn small primary` 22 高 | 把勾选的仓库加入列表 | 数字随选中数变（板上是 3） |
| 7 | 页脚右端 | Done | `btn` 24 高、字号 12 | 关设置回主窗口 | 常驻 |
| 8 | 左侧树 | 六项 | 整行链接 28 高 | 切分区 | 当前项 `--accent` 底 + 白字 |

- 卡二的四行是整行可点的 `<label>`（点行即切换复选），不计入按钮；`[dir]` 图标与 `−` 之外的图标都不接收点击。

## 9. 输入与选项

| 字段 | 形态 | 默认值（画板） | 说明 |
|---|---|---|---|
| Git executable | 字段 240 宽 | `/usr/bin/git` | 所有命令都用这个二进制 |
| Use system Git from PATH | 原生复选 | 不勾 | 勾上则启动时解析 `git`，不固定路径 |
| Scan folders | 只读行 + `−` | `~/Developer`（8 repositories）、`~/Work`（4 repositories） | 目录计数是扫描结果 |
| Found repositories | 复选行 ×4 | 勾 `gitui`、`atlas-web`、`docs-site`；不勾 `spectra` | 右端是绝对路径 |

复选框用 `accent-color:var(--accent)`（样式表只对 `.scan-row input` 设置），尺寸未定义，按原生渲染。

## 10. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 23 / 23b / 25 / 26 / 27 | 板内五个 `href`：`Settings.dc.html`、`SettingsAgent.dc.html`、`SettingsShortcuts.dc.html`、`SettingsCredentials.dc.html`、`SettingsAbout.dc.html` |
| 27 Settings · Credentials & tools | 它的 `Use system Git config for other settings` 把“身份来自 General、其他来自 ~/.gitconfig”交回这两块板 |
| 04 Commits（Workspace） | `Done` 指回主窗口；本板选的二进制按说明用于 `every command in this app`，因此参与它的 Fetch/Pull/Push 与提交 |
