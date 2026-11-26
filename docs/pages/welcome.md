# 欢迎页 — 布局、状态与按钮

- **画板**：01 Welcome（有最近仓库 · Clone）、01b Welcome · Open（空列表 · Open）、01c Welcome · Init（有最近仓库 · Init）
- **来源**：`designs/graft-git/project/Welcome*.dc.html` 与共享样式表。本文只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

画板将欢迎页表现为**独立的小窗口**：画板把它画成 1040×690（画板本身 1440×900，四周各留 24 的桌面），
比主窗口那套 1440×900 的板小一圈。当前实现把完整欢迎页放在主窗口内的模态层中，尺寸为 900×600；没有已打开仓库时自动显示且不能关闭，有仓库时可关闭。

## 1. 骨架

左右两栏：左栏固定 312，右栏占满剩余并自己滚动。下面是 01（Clone）的示意；行距与留白
按比例压缩过，只有标注的尺寸是画板原值。

```
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│  ● ● ●                  gitui                    ⚙   ?                                           │
├───────────────────────────────┬──────────────────────────────────────────────────────────────────┤
│                               │                                                                  │
│Repositories                   │▛▀▀▜  gitui                                                       │
│6 recent / 3 open      Open…   │▙▄▄▟  A native Git client.                                        │
│┌──────────────────────┐       │                                                                  │
││ ⌕ search repos   ⌘F  │       │[ Open ][ Clone ][ Init ]                                         │
│└──────────────────────┘       │                                                                  │
│Today                          │Clone a repository                        HTTPS | SSH             │
│▣ gitui        main   2m ×     │HTTPS, HTTP or SSH URL — creates the folder for you.              │
│  ~/Developer/gitui            │                                                                  │
│▣ atlas-web    feat  Yest ×    │Repository URL                                                    │
│▣ docs-site    main   Mon ×    │┌──────────────────────────────────────────────────┐              │
│Previous 7 days                ││   https://github.com/mia/gitui.git               │              │
│▣ dotfiles     main  Nov 6 ×   │└──────────────────────────────────────────────────┘              │
│▣ spectra      main  Nov 2 ×   │Destination                                                       │
│▣ billing-api  rel  Oct 28 ×   │┌──────────────────────────────────────┐ ┌───────────────┐        │
│                               ││ ~/Developer/gitui             ⌘V     │ │ Choose…       │        │
│                               │└──────────────────────────────────────┘ └───────────────┘        │
│                               │Branch                          Depth                             │
│gitui 0.9.2 (412)              │┌──────────────────┐  ┌──────────────────────────────┐            │
│Check for Updates   ⚙ Settings ││                  │  │ Full │ Shallow·1 │ Shallow·50 │           │
│                               │└──────────────────┘  └──────────────────────────────┘            │
│                               │☑ Recurse submodules      ☑ Open after clone                      │
│                               │                                                                  │
│                               │┌──────────────────────────────────────────────────────────────┐  │
│                               ││ ● GitHub · mia@github.com     [ Credentials ]                │  │
│                               │└──────────────────────────────────────────────────────────────┘  │
│                               │                                                                  │
│                               │[ Clone Repository ]   ~12 MB · public · http:// …                │
│                               │                                                                  │
│                               │ℹ gitui restores every repository from your last session.         │
│                               │  You can change this in Settings › General.                      │
└───────────────────────────────┴──────────────────────────────────────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 标题栏 | 顶部通栏 | 高 38；左右内距 13 | 三个窗口灯 12；标题居中 12.5/600；右侧两个 26×24 图标（设置、帮助） |
| 左栏 | 左侧固定 | 宽 312；底 `sidebar`；右边框 1 | 标题区、搜索、仓库分组、底栏 |
| 英雄区 | 右栏顶部 | 图标 64（圆角 15、底 `accent-2`）；图文间距 16 | 应用名 31/700；副标题 12.5 `text2`（板上上距 3） |
| 分段控件 | 英雄区下 | 上距 24；内距 2、项间距 2、圆角 7、底 `panel-2` | Open / Clone / Init，`role=tab` |
| 表单区 | 分段下 | 上距 14；最大宽 600（字段列 560） | 标题 + 副标题 + 表单（§3） |
| 提示条 | 表单下 | 上距 26；内距 10×12；圆角 8；最大宽 600 | 图标 + 一句话，前半句加粗 |
| 左栏底栏 | 左栏底部 | 内距 10×14；上边框 1 | 版本号、Check for Updates、Settings |
| 水印 | 右下角 | 380×220；距右 28、下 16；不透明度 .07 | 装饰性分支线条，不接收点击 |

右栏内距 44（上）/ 54（左右）/ 30（下）。左右两栏之间是一条 `line` 竖线。

## 2. 左栏

标题区（内距 16/16/10）：`Repositories` 17/700，下面一行 10.5 `text3` 是「6 recent · 3 open」——
最近数与**当前已打开**数两段信息；右端是 `Open…` 小按钮（23 高）。

搜索框：高 27，左右外距 12，内距 0 8，右端 `⌘F` 键帽。

仓库分组：只出现 **Today** 与 **Previous 7 days** 两种，按打开时间落组。分组标题 23 高、
9.5/600、**全大写**、字距 .045em（主窗口侧栏是 sentence case，这里是刻意的区分）。

一行仓库 47 高、圆角 7、内距 0 8，五个部分从左到右固定：图标 → 名称 + 路径 → 分支 chip →
相对时间 → 删除按钮。

```
┌────────────────────────────────────────────────┐
│ ┌──┐  gitui                main    2m   ×      │
│ │▣ │  ~/Developer/gitui                        │
│ └──┘                                           │
└────────────────────────────────────────────────┘
```

- **图标** 27×27、圆角 7、描边 `line`；当前打开的仓库改用 `content` 底 + `accent` 图标。
- **名称** 12/600，超长省略；**路径** mono 9.5 `text3`，显示成 `~/…`。
- **分支 chip** 高 16、圆角 4、mono 10，只写分支名（板上：`main`、`feature/hero`、`release`）。
  显示仓库**当前**分支：打开欢迎页时重新读一次 HEAD，不沿用上次打开时的快照；读不到分支
  （游离 HEAD 或仓库缺失）时整块不渲染。
- **时间** 9 `text3`，相对与简称混排：`2m`、`Yesterday`、`Mon`、`Nov 6`、`Oct 28`。
- **删除** 19×19 的 `×`，无障碍名「Remove <仓库名> from recent repositories」。
- 行状态：当前仓库 `wrepo on` 用 `sel` 底；hover 用 `hover` 底。整行可点，点开该仓库。

底栏：版本号（如 `gitui 0.9.2 (412)`）、弹性空隙、`Check for Updates` 文字链接、带 11pt 齿轮的
`Settings` 链接。

**空列表（01b）**：副标题改成「No repositories yet」；列表位置换成虚线卡片（`border:1px dashed`、
内距 24×16、圆角 9）——40×40 图标块 + 「No repositories yet」+「Open a local repository on the right,
or clone and create one from the tabs.」；分组标题不出现。

### 搜索行为（当前实现）

空查询只显示成功打开过的最近仓库；没有历史时保留空态与 Scan 入口。输入查询后，按路径文本搜索最近记录和 Git 设置中保存的**全部扫描结果**，不受命令面板 `scan_selected` 勾选影响。规范化仓库根相同的结果合并，历史行优先；扫描独占结果放在 `Scanned repositories` 分组，仍可打开、显示于 Finder 或复制路径，但菜单不显示 `Remove from List`。扫描不会自动写入最近历史；只有打开成功后才加入历史。

## 3. 右栏的三种表单

分段控件用 `on` 标出当前项，右栏其余内容随之整体替换。表单区标题与副标题配对如下：

| 分段 | 标题 | 副标题 |
|---|---|---|
| Open | Open a repository | Point gitui at a folder that already has a .git directory. |
| Clone | Clone a repository | HTTPS, HTTP or SSH URL — gitui creates the folder for you. |
| Init | Create a repository | Run `git init` in a new or existing folder. |

### 3.1 Open（01b）

```
Open a repository
Point gitui at a folder that already has a .git directory.
Repository path
┌────────────────────────────────────────────┐ ┌───────────┐
│ ~/Developer/gitui             ⌘V           │ │ Choose…   │
└────────────────────────────────────────────┘ └───────────┘
[ Open Repository ]   Also accepts a path to a worktree or a submodule.
Nothing listed yet — clone or create one from the tabs.        ⌘O
```

- 字段与 `Choose…` 同行：`grid 1fr auto`，底对齐，间距 8。
- 字段右端有 `⌘V` 键帽。
- 在路径字段里按回车等同于点 `Open Repository`：有值时直接打开，空值时先选目录。
- 主按钮下方一行 hint（9pt `text3`）。
- 最底是 meta-line（10pt `text3`）：左边提示可去分段克隆或新建，右边是 `⌘O`。**只有 Open 有这一行。**

### 3.2 Clone（01）

结构见 §1 的右栏，从上到下 7 段：

1. **协议切换**（HTTPS / SSH）贴在副标题行右端，只改 URL 示例的形态。
2. **Repository URL**：字段高 30，左侧 14pt 地球图标。
3. **Destination**：字段 + `Choose…` 同行，字段内有 `⌘V` 键帽。
4. **Branch + Depth** 同行，`grid 1fr 150px`；Branch 预填 `main`，Depth 三选一：Full history / Shallow · 1 / Shallow · 50。
5. **两个复选**竖排（行距 4）：Recurse submodules、Open after clone，板上都勾选。
6. **账号行**：`panel` 底、`line` 描边、圆角 7；左边状态点 +「GitHub · mia@github.com」，
   右边 `Credentials` 小按钮（画板里链到 27 Settings · Credentials）。
7. **动作行**：主按钮 `Clone Repository`（高 30）+ hint。hint 是动态摘要的占位：仓库大小、
   可见性、以及「http:// works for local mirrors」这类环境提示。

### 3.3 Init（01c）

```
Create a repository
Run `git init` in a new or existing folder.
Folder
┌────────────────────────────────────────────┐ ┌───────────┐
│ ~/Developer/new-project       ⌘V           │ │ Choose…   │
└────────────────────────────────────────────┘ └───────────┘
Repository name            Initial branch
┌────────────────────────────┐  ┌────────────────────────┐
│ new-project                │  │ main                   │
└────────────────────────────┘  └────────────────────────┘
☑ Create a README.md
☐ Add a .gitignore
☐ Add an MIT LICENSE
[ Create Repository ]   Creates the folder if it does not exist.
```

- Folder 与 `Choose…` 同行；下一行 `grid 1fr 160px` 放 Repository name（预填 `new-project`）与 Initial branch（预填 `main`）。
- 三个复选**竖排**（行距 4）：README、.gitignore（通用最小集合）、MIT LICENSE。板上只勾第一项。
- 主按钮 `Create Repository` + hint「Creates the folder if it does not exist.」。

## 4. 状态与切换

页面由**两条独立的轴**组成，三块板只覆盖了其中三个组合：

| 轴 | 取值 | 画板 |
|---|---|---|
| 左栏 | 有最近仓库 / 空 | 01、01c / 01b |
| 右栏 | Open / Clone / Init | 01b / 01 / 01c |

```
分段：  Open  ⇄  Clone  ⇄  Init      只换右栏表单，不弹窗、不二次确认
送出：  Open Repository / Clone Repository / Create Repository → 主窗口 · 新标签页
        Clone 只有勾了 Open after clone 才切走；从左栏打开仓库也走这条
```

- 左栏与右栏互不影响：列表为空时同样能切到 Clone / Init（画板只画了空 + Open 这一种）。
- 进入主窗口的动作有五个：左栏顶部 `Open…`、左栏任意仓库行、Open 的 `Open Repository`、
  Clone 的 `Clone Repository`、Init 的 `Create Repository`。三个 `Choose…` 只负责选目录，不自己提交。
  标题栏没有打开仓库的入口。
- 键盘：`⌘F` 聚焦左栏搜索，`⌘O` 打开仓库，`⌘V` 粘贴路径——板上都以键帽提示。

### 启动与切窗行为

- `Restore last session` 且保存会话非空时，按已存顺序恢复主窗口标签与活动仓库，不显示 Welcome；空会话仍显示主窗口和必需的 Welcome 模态层。
- `Show Welcome` 跳过会话恢复并打开主窗口内的模态层。若应用在没有打开仓库时退出，保存的会话为空，下次仍显示不可关闭的 Welcome。
- 主窗口标题栏的 `+`、Help > Welcome 与 Repository Manager 打开 Welcome 模态层。已有仓库时可用 Escape、遮罩或关闭按钮关闭；没有仓库时这些关闭方式均无效。成功打开仓库后自动关闭模态层并进入主窗口；同一规范化仓库只聚焦已有标签。
- 最近仓库打开失败时切到 Open 表单，保留路径并显示错误；不会加入打开标签。

## 5. 按钮与链接清单

下表 14 行：**13 个按钮/链接**，加 1 个分段控件（#7）；整行可点的仓库行不在表内。

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 标题栏右 | 齿轮图标（`aria=Settings`） | 图标 26×24 | 打开设置 | 常驻 |
| 2 | 标题栏右 | 问号图标（`aria=Help`） | 图标 26×24 | 打开帮助 | 常驻 |
| 3 | 左栏标题右 | Open… | 小按钮 23 高 | 打开本地仓库 | 常驻 |
| 4 | 左栏每行 | × | 图标按钮 19×19 | 从最近列表移除该仓库 | 每行一个 |
| 5 | 左栏底栏 | Check for Updates | 文字链接 | 检查更新 | 常驻 |
| 6 | 左栏底栏 | ⚙ Settings | 文字链接 + 11pt 齿轮 | 打开设置 | 常驻 |
| 7 | 右栏分段 | Open / Clone / Init | 分段项（`role=tab`） | 切换表单 | 当前项 `on` |
| 8 | Open 表单 | Choose… | 次按钮 | 选目录 | 仅 Open |
| 9 | Open 表单 | Open Repository | 主按钮 30 高 | 打开仓库并进入主窗口 | 仅 Open |
| 10 | Clone 表单 | Choose… | 次按钮 | 选目标目录 | 仅 Clone |
| 11 | Clone 表单 | Credentials | 小按钮 | 打开 27 设置 · 凭据 | 仅 Clone |
| 12 | Clone 表单 | Clone Repository | 主按钮 30 高 | 开始克隆 | 仅 Clone |
| 13 | Init 表单 | Choose… | 次按钮 | 选目录 | 仅 Init |
| 14 | Init 表单 | Create Repository | 主按钮 30 高 | 初始化仓库 | 仅 Init |

- 协议切换（HTTPS / SSH）是**选项对**，不算按钮：选中项高亮，切换不影响提交动作。
- 三个主按钮互斥出现在同一位置，所以一屏只有一个主操作。
- 所有图标按钮都带无障碍名。
- 画板不画 macOS 菜单栏：`⌘O` / `⌘F` / `⌘V` 只以键帽形式出现在页面里。

## 6. 输入与选项

| 表单 | 输入 | 形态 | 说明 |
|---|---|---|---|
| Open | Repository path | 单行 30 高 + `⌘V` | 可手填可 Choose… |
| Clone | Repository URL | 单行 30 高 + 地球图标 | 板上占位 `https://github.com/mia/gitui.git` |
| Clone | Destination | 单行 30 高 + `⌘V` | 占位 `~/Developer/gitui`；与 Choose… 同行 |
| Clone | Branch | 单行 30 高 | 预填 `main`；清空表示默认分支 |
| Clone | Depth | 三选一分段 | Full history / Shallow · 1 / Shallow · 50 |
| Clone | Recurse submodules | 复选 | 板上勾选 |
| Clone | Open after clone | 复选 | 板上勾选；决定成功后是否切到主窗口 |
| Init | Folder | 单行 30 高 + `⌘V` | 占位 `~/Developer/new-project` |
| Init | Repository name | 单行 30 高 | 预填 `new-project` |
| Init | Initial branch | 单行 30 高 | 预填 `main` |
| Init | README / .gitignore / LICENSE | 三个复选 | Create a README.md、Add a .gitignore、Add an MIT LICENSE |

## 7. 文案要点

- 左栏副标题是两段信息：「6 recent · 3 open」——最近数与当前已打开数。
- 右栏三句副标题都交代**后果**，不是重复标题：Clone 的说会替你建目录，Init 的说会在新目录或
  已有目录里跑 `git init`。
- 提示条固定三段式：加粗前半句（gitui restores every repository from your last session.），
  后半句给去处（You can change this in Settings › General.）。三块板一字不差。
- 空态与 Open 的 meta-line 都给了下一步（去右侧打开，或从分段里克隆/新建），不是干巴巴的「暂无数据」。

## 8. 画板没覆盖的组合（待补设计）

三块板只覆盖三个组合，下面这些状态**没有画**，实现时容易各写各的：

1. **空列表 + Clone**、**空列表 + Init**：空态文案写着「clone or create one from the tabs」，
   说明这两种组合是预期存在的，但没有画。
2. **克隆进行中**：主按钮是否变「Cloning…」并禁用、进度放哪、能否取消，板上没有。
3. **失败态**：URL 非法、目标目录非空、认证失败、无网络——错误显示在字段下方还是动作行下方，
   没有定义；Clone 与 Init 是否共用一处错误位置也没有定义。
4. **搜索**：有输入但无匹配时的空态没有画；分组标题是否保留也没有定义。
5. **删除最近仓库**：`×` 是否二次确认、移除后分组如何重排，没有画。
6. **窄窗口**：共享样式表在 ≤860 时把左栏 `display:none`、右栏内距收成 30/24，但没有画出窄版的
   实际样子（分段是否换行、表单是否变单列都不清楚）。
7. **帮助入口**：标题栏问号指向哪里没有画。
8. **首次启动**：提示条假设「上次会话有仓库」，但「上次也没有仓库」的首启状态没有单独画。

## 9. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 23 Settings | 左栏底栏 `Settings` 与标题栏齿轮都指向它 |
| 27 Settings · Credentials | Clone 的 `Credentials` 指向它（账号行显示当前登录） |
| 04 Commits 及以后 | 三种提交动作成功后进入的主窗口 |
| 16 Repository states | 主窗口的空仓库、缺失仓库等状态，与欢迎页无关 |
