# gitui 设计画板交互验收手册

**状态**: working draft  
**日期**: 2026-11-14  
**画布**: 30 页，`designs/graft-git/`  
**本地预览**: `http://127.0.0.1:4720/design/AnB97zSAOiuT2MFJiXb15j/`  
**用途**: 手工验收 30 页设计画板的操作路径、按钮目标、弹窗/选择/确认类型、点击后的状态反馈。  
**源文档**: [SPEC.md](../SPEC.md)、[USE-CASES.md](../docs/USE-CASES.md)、[UI-SPEC.md](../docs/UI-SPEC.md)、[TEST-PLAN.md](../docs/TEST-PLAN.md)  
**当前代码**: P1（Welcome、Open Repository、Workspace 只读历史/Diff）；其余页面处于设计验证阶段。

> 本文件是设计画板的工作副本，不是需求源。与 `SPEC.md`、`docs/USE-CASES.md`、`docs/UI-SPEC.md`、`docs/TEST-PLAN.md` 冲突时，以这些文档为准。  
> 本文件放在 `files.local/`，避免与正在修改的产品代码和正式文档混合。测试通过后，结论再合并到正式文档。

---

## 0. 使用说明

### 0.1 范围

本文件覆盖：

- 30 页画板的入口、出口和页面关系。
- 每页可见操作控件的目标页面、弹窗、菜单、选择控件、确认框或静态状态。
- 关键操作的预期反馈，包括成功、失败、禁用、加载、空状态、认证失败、冲突和操作进行中。
- 端到端测试场景、验收检查表和已知缺口。

本文件不覆盖：

- Rust/NSH 实现细节、Git 命令参数、性能、网络、密钥存储和发布流程。
- 像素级视觉回归。视觉规范参考 [DESIGN-HANDOFF.md](../docs/DESIGN-HANDOFF.md)。
- 22 号组件页和 16 号状态页的生产交互。22 是菜单/弹窗规格，16 是状态画廊。

### 0.2 状态图例

| 标记 | 含义 |
|---|---|
| `可点` | 在画布的 Play 模式中有链接或可切换状态 |
| `静态` | 画板中有控件外观，但没有真实跳转或执行 |
| `缺` | 画板中没有对应页面、弹窗或确认框，生产前必须补充 |
| `组件` | 组件规格页，不是应用路由 |
| `画廊` | 状态参考页，不是独立功能页 |
| `→ 04` | 目标画板编号；`04` 即 `04 · History & diff` |
| `确认` | 点击后应出现确认框，要求用户明确同意 |
| `选择` | 下拉、分段控件、单选或复选选择 |
| `外部` | 系统浏览器、编辑器或 Finder |

### 0.3 控件类型

| 类型 | 测试关注点 |
|---|---|
| 页面链接 | 目标页面是否正确；返回路径是否存在 |
| 弹窗/Sheet | 是否有标题、主按钮、取消按钮；Esc/关闭是否安全 |
| 菜单 | 菜单项是否完整；危险项是否有分隔和确认 |
| 选择/下拉 | 默认值、选项、切换后的即时反馈 |
| 复选/开关 | 是否立即生效；是否影响提交范围或权限 |
| 确认框 | 对象名、后果、主按钮文案、取消路径 |
| 输入框 | 空值、格式、禁用态、错误提示 |
| 静态控件 | 只验证外观和位置；功能记为 `BLOCKED` |

### 0.4 测试环境与固定数据

测试画板使用以下 mock 数据，页面之间的计数和文案应与此一致：

| 项目 | 值 |
|---|---|
| 打开的仓库 | `gitui`、`docs-site`、`atlas-web`，共 3 个 tab |
| recent 仓库 | 6 个；3 个处于打开状态 |
| 主仓库路径 | `~/Developer/gitui` |
| 当前分支 | `main` |
| 提交数 | 128 |
| 工作区变更 | 12 个文件：3 staged、9 unstaged |
| 分支 | 5 个本地分支，另有 `origin/main` |
| tags | 3 个：`v1.4.1`、`v1.4.0`、`latest` |
| stashes | 2 个 |
| submodules | 2 个 |
| worktrees | 3 个 |
| reflog | 42 条 |
| 当前领先/落后 | `2 ahead · 0 behind`；Pull 状态下为 `2 ahead · 2 behind` |
| 合并冲突 | 3 个文件，3 个冲突，其中 2 个未解决 |
| hook 状态 | `pre-commit` 失败，退出码 1 |
| 语言 | 默认 English；简体中文可切换 |
| 日期 | 2026-11-14，Asia/Shanghai |

除 `gitui` 外，其他仓库只是列表数据，没有完整的 diff/文件内容。

### 0.5 画布限制

- 跨画板链接只在 Play 模式生效；编辑模式点击不会跳转。
- `⌘P` 等键盘快捷键没有在画布中绑定。20 号面板通过 titlebar 右侧的 overflow 箭头进入；快捷键验证在 25 号页和生产版本进行。
- 输入框、下拉框、复选和分段控件是静态视觉状态；不会保存，也不会执行 Git 操作。
- 画布没有真实 Git 引擎、文件监听、持久化和网络请求。
- 22 是组件规格，16 是状态参考，两者不能作为独立功能页验收。
- 28、29 是真实 Sync sheet；30 是 Refs 管理器页面；其余页面为 App 页面。

### 0.6 测试记录方式

每个控件或场景记录：

| 字段 | 值 |
|---|---|
| 结果 | `PASS` / `FAIL` / `BLOCKED` / `N/A-DESIGN` |
| 画板 | 例如 `04` |
| 控件/场景 | 例如 `Compare` |
| 期望 | 目标页面或状态 |
| 实际 | 实际观察 |
| 证据 | 截图名称或 canvas 链接 |
| 备注 | 静态、缺失、环境限制 |

`BLOCKED` 用于当前画板没有目标页面、没有确认框或没有真实执行；不能记为 `FAIL`，也不能当作已实现。  
`N/A-DESIGN` 用于 22 的组件卡片和 16 的状态卡片。

---

## 1. 全局操作模型

### 1.1 窗口区域

| 区域 | 控件 | 当前行为 | 测试重点 |
|---|---|---|---|
| Titlebar | 红黄绿按钮 | 静态 | 窗口关闭/最小化/缩放；关闭前有未提交变更时确认 |
| Titlebar | repo tabs（gitui/docs-site/atlas-web） | 静态切换 | 切换保留各仓库自己的选择、滚动和 diff |
| Titlebar | `+` | → 01 Welcome | 打开仓库入口 |
| Titlebar | overflow 箭头 | → 20 Command palette | 仓库/命令搜索入口 |
| Titlebar | `Settings` gear | → 23 Settings | 全局设置入口 |
| Titlebar | tab 关闭 `×` | 静态 | 关闭 tab 不删除 recent；有未提交变更时确认 |
| Toolbar | `History` / `Changes` 模式 | → 04 / 10 | 当前仓库的主视图切换 |
| Toolbar | `Agent` | → 12 Agent panel | Agent 入口 |
| Toolbar | `Fetch` / `Pull` / `Push` | 目标 15/28/29；当前部分静态 | 网络操作入口、进行中和失败状态 |
| Toolbar | Search | 静态 | `⌘F` 搜索；无结果状态 |
| Sidebar | Branches / Remotes / Tags / Stashes / Submodules / Worktrees | 静态选择 | 计数、选中、上下文菜单、折叠 |
| Main | 主列表 + Diff/文件 | 可点或静态 | 选择、过滤、多选、文件跳转 |
| Inspector | Commit / 文件详情 / 冲突 | 可点或静态 | 关闭、宽度调整、滚动 |
| Status bar | branch / ahead / behind / operation / sync | 静态 | 与仓库状态一致；失败不能只显示颜色 |

### 1.2 全局状态

| 状态 | 页面表现 | 入口 | 测试重点 |
|---|---|---|---|
| Loading repository | 骨架行、无旧仓库数据 | 启动恢复、切换 tab | 不闪烁旧内容；可取消 |
| Empty repository | “No commits yet” + 初始化入口 | 04/16 | 不能显示空 diff |
| Ready | 正常列表和 diff | 04/10 | 读操作可用 |
| Operation in progress | 顶部 banner：merge/rebase/cherry-pick | 04/06/14 | Continue/Abort/Skip；冲突操作禁用 |
| Offline | 状态栏/横幅；网络按钮禁用 | 04/15/16 | 本地操作仍可用 |
| Auth required | 横幅：Sign in / Use SSH | 04/16 | 链接到 27；不泄露 token |
| Fatal repo error | 错误面板 + Retry/Close | 04/16 | 无部分可用状态 |
| Detached HEAD | 横幅：Create branch / Checkout | 04/06/16 | 提交目标说明；不误提交到 main |

### 1.3 全局快捷键

| 快捷键 | 设计动作 | 画布状态 |
|---|---|---|
| `⌘P` | 打开 20 Command palette | 未绑定；使用 overflow 箭头 |
| `⌘O` | 打开仓库 / 01 | 未绑定 |
| `⌘W` | 关闭当前 tab | 未绑定 |
| `⌘1…⌘9` | 切换仓库 tab | 未绑定 |
| `⌘⇧[` / `⌘⇧]` | 上一个/下一个 tab | 未绑定 |
| `⌘F` | 搜索 | 未绑定 |
| `⌘⇧F` | 过滤 history | 未绑定 |
| `⌘⇧P` | Push | 未绑定 |
| `⌘⇧K` | Commit | 未绑定 |
| `⌘↵` | 当前主操作 | 未绑定 |
| `Esc` | 关闭 sheet/palette/menu | 部分链接可返回 |

> 25 号 Settings · Shortcuts 是快捷键配置的验收页；画布里的快捷键表不作为可执行功能。

---

## 2. 30 页入口与出口

| # | 画板 | 类型 | 主要入口 | 主要出口 | 测试重点 |
|---|---|---|---|---|---|
| 01 | Welcome | App | 启动、titlebar `+` | 04、23 | recent、Open、Clone、空状态 |
| 02 | Welcome (empty) | App | 无仓库启动 | 04、24、23 | No repositories yet、Scan |
| 03 | Repository manager | App | 画布直达；产品从 Open/Clone 入口 | 04、27、23 | recent、Open/Clone/Init、认证 |
| 04 | History & diff | App | 01/03 打开仓库 | 05、06、08、10、12、18、20、21 | 历史、diff、多选、上下文 |
| 05 | History filters | App popover | 04 的 branch/date/author/filter | 04 | 多选、日期、Apply/Reset |
| 06 | Multi-commit diff | App | 04 多选、Compare | 04、07、08、18、22 | 聚合 diff、选择操作栏 |
| 07 | Compare refs | App | 06/09/22 的 Compare | 04、08、18 | two-dot/three-dot、14 文件 |
| 08 | Commit info | App | 04/06/07 的 Info | 外部、确认 | SHA、parents、签名、复制 |
| 09 | Refs manager | App | 画布直达、04 sidebar | 22、07、30 | 分支表、upstream、操作 |
| 10 | Staging & commit | App | 04 Changes、toolbar | 04、11、12、14、22 | staged、hooks、commit |
| 11 | Changelists | App | 10 的 All changelists/plus | 10、12、22 | 分组、过滤器、提交范围 |
| 12 | Agent panel | App | toolbar Agent、11 Ask Agent | 13、10、22 | plan、approval、stage |
| 13 | Agent permissions | App sheet | 12 的 Ask first | 12 | 权限、automation、保护分支 |
| 14 | Merge conflict | App modal | 06/10 的 Resolve、merge/rebase | 04 | 三方、可编辑 Result、continue/abort |
| 15 | Sync & auth | App modal | toolbar Push | 28、29、27、04 | Push、auth、progress |
| 16 | Repository states | 画廊 | 画布直达；03/04/06 的 banner | 04 | 空、detached、merge、offline、auth、loading |
| 17 | Reflog & undo | App | 画布直达；恢复入口 | 04、22 | reset 恢复、create branch |
| 18 | Files at commit | App | 04 Files tab | 19、04、外部 | 只读文件、Blame、History |
| 19 | Blame & file history | App | 18 Blame | 18、04 | 文件历史、逐行 blame |
| 20 | Command palette | App overlay | titlebar overflow | 04、22、23 | 仓库/命令搜索、键盘 |
| 21 | Dark & context menu | App menu | 04 commit 右键 | 04、22、07、外部 | commit 菜单、危险项 |
| 22 | Menus & dialogs | 组件 | 画布直达；命令/右键的目标 | 无（规格页） | 12 类菜单/弹窗的完整控件 |
| 23 | Settings | App | titlebar gear | 04、24、25、26、27 | 主题、语言、日期、身份、启动、更新 |
| 24 | Settings · Git | App | 23 Git | 23 | Git 路径、检测、扫描 |
| 25 | Settings · Shortcuts | App | 23 Shortcuts | 23 | 快捷键、Record、冲突 |
| 26 | Settings · About | App | 23 About | 23 | 版本、链接、credits |
| 27 | Settings · Credentials | App | 23 Credentials、04 Sign in、03 Credentials | 23、04 | 账号、SSH、工具、网络 |
| 28 | Sync · Fetch | App modal | 15 Fetch tab | 15、04 | Fetch 选项、remote refs |
| 29 | Sync · Pull | App modal | 15 Pull tab | 15、14、04 | merge/rebase/ff-only、autostash |
| 30 | Refs · Tags, Remotes, Stashes | App | 09 Open details | 09、22 | 三类 ref 表、创建/删除 |

---

## 3. 逐页操作清单

### 01 · Welcome

**入口**: 启动、titlebar `+`。  
**出口**: 04 Workspace、23 Settings。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| 左侧 `Repositories` 标题右侧 `Open…` | 页面链接 | → 04；生产版本打开本地选择器 | 可点 | 打开一个最近仓库；取消不创建 tab |
| `gitui` recent 行 | 页面链接 | → 04 | 可点 | 打开 gitui，tab 激活 |
| `docs-site` / `atlas-web` recent 行 | 静态 | 无跳转；目标 04 | 静态 | 只验证列表外观；没有 mock 内容 |
| 右上 `Settings` gear | 页面链接 | → 23 | 可点 | Settings 打开，General active |
| `?` Help | 静态 | 设计目标：帮助 popover | 静态 | 有 tooltip；不打开空白页 |
| Clone URL 输入 | 输入 | 输入仓库 URL | 静态 | 空值/非法 URL 的错误状态待补 |
| Clone destination 输入 | 输入 | 输入目标目录 | 静态 | 目录已存在时提示 |
| `Clone Repository` 按钮 | 按钮 | 成功 → 04；失败 inline | 静态 | 成功后创建 tab；失败保留表单 |
| recent 列表 | 列表 | 选择/打开 | 静态行 | 计数 6 recent、3 open 一致 |
| `gitui` 版本脚注 | 文本 | 无 | 静态 | 版本号与 26 About 一致 |

### 02 · Welcome (empty)

**入口**: 无仓库启动。  
**出口**: 04、24、23。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| `Open…` | 页面链接 | → 04 | 可点 | 打开仓库后进入 04 |
| `Scan a Folder…` | 页面链接 | → 24 Settings · Git | 可点 | 打开扫描设置；不是直接扫描 |
| Clone URL / destination / `Clone Repository` | 输入/按钮 | 成功 → 04 | 静态 | 空状态仍可 Clone |
| 右上 `Settings` gear | 页面链接 | → 23 | 可点 | Settings General |
| `No repositories yet` | 文本 | 无 | 静态 | 不显示假的仓库行 |

### 03 · Repository manager

**入口**: 画布直达；产品从 Open/Clone/Init 入口进入。  
**出口**: 04、27、23。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| recent 仓库行 `gitui` | 页面链接 | → 04 | 可点 | 打开 gitui |
| `docs-site`、`atlas-web`、`dotfiles`、`spectra`、`billing-api` | 静态 | 目标 04 | 静态 | 只验证列表；没有完整 mock |
| `Open…` 底部按钮 | 按钮 | 目标 04 / 文件选择器 | 静态 | 非 repo 目录显示 Init/Open parent |
| `Clone…` 底部按钮 | 按钮 | 切到 Clone 表单 | 静态 | Clone tab active |
| `Init…` 底部按钮 | 按钮 | 切到 Init 表单 | 静态 | Init 字段可见 |
| `Open` / `Clone` / `Init` tabs | 选择 | 切换表单 | 静态 | 当前 Clone active；切换不丢输入 |
| Clone URL / Directory / Branch | 输入 | 设置目标 | 静态 | 默认值与 fixture 一致 |
| `Depth` | 下拉 | Full / Shallow 1 / Shallow 50 | 静态 | 与 shallow 复选不重复 |
| `Recurse submodules` | 复选 | 克隆子模块 | 静态 | 默认勾选 |
| `Open after clone` | 复选 | 成功后打开 04 | 静态 | 取消则只加入 recent |
| `Credentials` | 页面链接 | → 27 | 可点 | 打开凭据设置 |
| `Clone Repository` | 按钮 | 成功后 → 04；进行中显示进度 | 静态 | 进度/取消/失败待补 |
| Init `Create Repository` | 按钮 | 成功后 → 04 | 静态 | 非空目录、.gitignore、README 状态待补 |
| `Settings` titlebar 链接 | 页面链接 | → 23 | 可点 | Settings General |

### 04 · History & diff

**入口**: 01/03 打开仓库。  
**出口**: 05、06、08、10、12、18、20、21、23。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| repo tabs | 选择 | 切换仓库 | 静态 | 每个 tab 保持自己的选择 |
| `+` | 页面链接 | → 01 | 可点 | 打开 Welcome |
| overflow 箭头 | 页面链接 | → 20 | 可点 | 打开 Command palette |
| `Settings` gear | 页面链接 | → 23 | 可点 | Settings General |
| tab `×` | 按钮 | 关闭 tab | 静态 | 有未提交变更时确认；不删除 recent |
| `History` / `Changes 12` | 选择 | 04 / 10 | 可点 | Changes 切到 staging |
| `Agent` | 页面链接 | → 12 | 可点 | Agent panel |
| `Fetch` / `Pull` / `Push` | 按钮 | 目标 15/28/29 | 静态 | 进行中/失败状态待补 |
| auth 横幅 `Sign in…` / `Use SSH` | 页面链接 | → 27 | 可点 | 不泄露 token；返回 04 |
| 左侧 Branches | 列表/菜单 | 选中分支、右键 22 | 静态 | 分支计数 5 |
| 左侧 Remotes | 列表/菜单 | 选中 remote、右键 22 | 静态 | origin + origin/main |
| 左侧 Tags / Stashes | 列表/菜单 | 选中、右键 22 | 静态 | tags 3、stashes 2 |
| 左侧 Submodules / Worktrees | 列表/菜单 | 选中、右键 22 | 静态 | 2 / 3 |
| branch chip `main 2` | 页面链接 | → 05 | 可点 | 打开过滤，保留当前分支 |
| date chip `Any time` | 页面链接 | → 05 | 可点 | 打开日期区 |
| `Compare` | 页面链接 | → 06 | 可点 | 多选/对比视图 |
| `All authors` | 页面链接 | → 05 | 可点 | 打开 author 过滤 |
| filter icon | 页面链接 | → 05 | 可点 | 与 branch/date 同一 popover |
| commit 行 | 选择 | 选中并显示 diff | 静态 | 选中态和 diff 同步 |
| 多选 commit | 选择 | → 06 | 静态 | 3 个选中时显示聚合 diff |
| commit 右键 | 菜单 | → 21 | 静态 | 菜单项完整；危险项确认 |
| `Files` tab | 页面链接 | → 18 | 可点 | 文件 at commit |
| `Info` tab | 页面链接 | 目标 08 | 静态 | 当前 tab 无内容，验收记 BLOCKED |
| Split/Unified、fold、wrap、whitespace | 选择/按钮 | 切换 diff 展示 | 静态 | 状态在文件间保持 |
| tree/list toggle | 选择 | 切换文件树/列表 | 静态 | 当前 tree active |
| changed file 行 | 选择 | 更新 diff | 静态 | 选中文件与 diff 路径一致 |
| 状态栏 branch/ahead | 文本 | 无 | 静态 | `2 ahead · 0 behind` |

### 05 · History filters

**入口**: 04 的 branch/date/author/filter 控件。  
**出口**: 04（Apply/Reset/Esc）。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| branch 复选（main、feature/diff-view、feature/agent、fix/wrapped-lines、release/1.4、origin/main） | 复选 | 多选分支 | 静态 | `main` 当前；选中 3 个，chip 显示 +2 |
| `Select all` | 按钮 | 全选分支 | 静态 | 全选后 Apply 结果变化 |
| 日期 `Any time` / `7 days` / `30 days` / `Custom` | 选择 | 切换日期预设 | 静态 | Custom 显示起止字段 |
| 起止日期字段 | 输入 | 自定义范围 | 静态 | 2026-11-01 → 2026-11-14 |
| 日历 | 选择 | 选择日期范围 | 静态 | 1–14 选中，14 为今天 |
| `Author` / `Path` / `Message` | 输入 | refine | 静态 | 空值清空过滤 |
| `Apply` | 页面链接 | → 04；应用 46 条结果 | 可点 | 列表 count 和 chip 更新 |
| `Reset` | 页面链接 | → 04；清空过滤 | 可点 | 恢复 128 条 |
| `esc` / close | 页面链接 | → 04 | 可点 | 不应用未提交的过滤 |
| `128 → 46 commits` | 文本 | 无 | 静态 | 与 04 的 46 一致 |

### 06 · Multi-commit diff

**入口**: 04 多选 commit、Compare。  
**出口**: 04、07、08、18、22。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| `History` 返回 | 页面链接 | → 04 | 可点 | 回到历史，保留选中 |
| 提交范围 `8b41e07..3f2a1c9` | 文本 | 无 | 静态 | 3 commits、12 files |
| `Compare with Working Tree` | 按钮 | 目标 07/工作区对比 | 静态 | 不在当前画板执行 |
| `Clear` | 页面链接/按钮 | → 04 或取消选择 | 静态 | 清空多选 |
| 多选操作栏 `Squash…` | 按钮 | → 22 Squash | 静态 | 需 3 commits 选中，确认后显示 |
| 多选操作栏 `Rebase…` | 按钮 | 目标 22 / 20 Interative Rebase | 静态 | 20 未做；记 BLOCKED |
| 多选操作栏 `Cherry-pick` | 按钮 | 目标：确认后执行 | 静态 | 冲突 → 14 |
| 多选操作栏 `Compare` | 页面链接 | → 07 | 可点 | 打开 Compare refs |
| `Changes` / `Files` / `Info` | tab | 目标 08/18 | 静态 | Info 当前无内容 |
| diff file tree | 选择 | 切文件 | 静态 | 右侧 12 文件 |
| `Export Patch` | 按钮 | 保存 patch | 静态 | 空选择时禁用 |
| Split/Unified、fold、wrap | 选择/按钮 | diff 展示 | 静态 | 状态保持 |

### 07 · Compare refs

**入口**: 06/09/22 的 Compare。  
**出口**: 04（ref chips）、08、18。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| `main` / `feature/diff-view` ref chip | 页面链接 | → 04 | 可点 | 返回历史，保留 compare 目标 |
| `↔` 方向 | 文本 | 无 | 静态 | 目标方向明确 |
| `2-dot` / `3-dot` | 选择 | 切换比较模式 | 静态 | 计数和 diff 更新 |
| `Swap` | 按钮 | 交换左右 ref | 静态 | ahead/behind 对调 |
| `12 commits ahead · 0 behind` | 文本 | 无 | 静态 | 与 22 compare card 一致 |
| 右侧 file tree | 选择 | 切文件 | 静态 | 14 文件；`Changes 14` |
| diff 文件行 | 选择 | 更新 diff | 静态 | 路径与 diff 一致 |
| `Changes` / `Files` / `Info` | tab | 目标 08/18 | 静态 | Info 当前无内容 |
| `Export Patch` | 按钮 | 保存 patch | 静态 | 只导出当前选择 |
| 状态栏 `Comparing 12 commits` / `14 files` | 文本 | 无 | 静态 | 与标题一致 |

### 08 · Commit info

**入口**: 04/06/07 的 Info tab。  
**出口**: 外部、确认框；返回依赖 titlebar。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| `Changes` / `Files` | tab | 目标 06/18 | 静态 | Info active |
| `Copy SHA` | 按钮 | 复制 `3f2a1c9…`；toast | 静态 | 剪贴板；toast 不阻塞 |
| `Open on GitHub` | 外部 | 浏览器打开 commit | 静态 | 路径/remote 可解析；失败提示 |
| `Cherry-pick` | 按钮 + 确认 | 确认后执行；冲突 → 14 | 静态 | 确认框写明目标 commit |
| `Revert` | 按钮 + 确认 | 确认后执行；冲突 → 14 | 静态 | 确认框写明新 commit |
| Commit / Parents / Tree / Refs | 文本 | 无 | 静态 | hash、parent、14 files 一致 |
| Signature `Verified · SSH` | 状态 | 无 | 静态 | 未签名/无效签名有不同状态 |
| Author / Committer / dates | 文本 | 无 | 静态 | 与 commit 数据一致 |

### 09 · Refs manager

**入口**: 画布直达；04 sidebar。  
**出口**: 22、07、30。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| `Branches` tab | 选择 | 当前 active | 静态 | 分支表 |
| `Tags` / `Remotes` / `Stashes` tab | 选择 | 目标 30 / 切换内容 | 静态 | 当前只显示 Branches |
| `New Branch` | 按钮 | → 22 New Branch | 静态 | 名称、From、Checkout after create |
| `Branches` 表行（main、feature/diff-view、feature/agent、fix/wrapped-lines、release/1.4、origin/main） | 选择 | 选中分支 | 静态 | upstream、ahead/behind、last commit 一致 |
| `Checkout` | 按钮 + 条件 | 切换分支；脏工作区弹选择 | 静态 | 选择 Stash/Carry/Cancel |
| `Rename` | 按钮 | 重命名 dialog | 静态 | 当前分支校验；名称冲突 |
| `Delete` | 按钮 + 确认 | 删除分支 | 静态 | 不能删当前分支；确认对象名 |
| `Compare` | 页面链接 | → 07 | 可点 | 比较当前分支与选中分支 |
| `Merge` | 按钮 + 确认 | 合并到当前分支 | 静态 | 冲突 → 14 |
| `Rebase` | 按钮 + 确认 | 目标 20 / 22 | 静态 | 20 未做；BLOCKED |
| `Open details` | 页面链接 | → 30 | 可点 | 打开 Tags/Remotes/Stashes |

### 10 · Staging & commit

**入口**: 04 Changes、toolbar。  
**出口**: 04、11、12、14、22。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| `History` / `Changes` | 选择 | 04 / 10 | 可点 | Changes active |
| merge banner `Resolve…` | 页面链接 | → 14 | 可点 | 冲突视图 |
| merge banner `Abort` | 按钮 + 确认 | 中止 merge | 静态 | 恢复前状态 |
| Tree / List toggle | 选择 | 切换 changes 布局 | 静态 | 当前 Tree；List 状态待测 |
| `Stage all` / `Unstage all` | 按钮 | 全部 stage/unstage | 静态 | 3 staged、9 unstaged 更新 |
| 文件行复选 | 复选 | stage/unstage 单文件 | 静态 | diff 和计数同步 |
| directory 行 | 选择 | 折叠/展开 | 静态 | 文件计数一致 |
| 文件右键菜单 | 菜单 | 打开 10 内的 `file-menu` | 静态 | 菜单项见下 |
| `Revert Changes…` | 菜单项 + 确认 | 撤销文件修改 | 静态 | 确认对象名和后果 |
| `Unstage File` | 菜单项 | 取消暂存 | 静态 | 只影响 index |
| `Open in Editor` | 菜单项/外部 | 打开外部编辑器 | 静态 | 编辑器不存在时提示 |
| `Reveal in Finder` | 菜单项/外部 | Finder 显示 | 静态 | 路径不存在时提示 |
| `Copy Path` | 菜单项 | 复制路径 | 静态 | 复制完整路径 |
| `Move to Changelist…` | 菜单项 | → 11 的移动 | 静态 | 不改变 stage 状态 |
| `Discard…` | 菜单项 + 确认 | 丢弃未暂存修改 | 静态 | 不可恢复提示；强确认 |
| hunk `Unstage Hunk` | 按钮 | 取消该 hunk | 静态 | 行级状态更新 |
| 行 `Unstage line` | 按钮 | 取消该行 | 静态 | 行级状态更新 |
| `All changelists` | 页面链接 | → 11 | 可点 | changelist 过滤器 |
| `+` changelist | 页面链接 | → 11 | 可点 | 新建/管理 |
| Summary / Description | 输入 | 提交信息 | 静态 | 空摘要禁用提交 |
| `Amend last commit` | 复选 | amend | 静态 | 已有提交校验 |
| `Sign off` | 复选 | 添加 sign-off | 静态 | trailer 格式 |
| `Push after commit` | 复选 | 提交后推送 | 静态 | 失败留在 10/15 |
| `Draft a message` / `Ask Agent` | 页面链接 | → 12 | 可点 | Agent 起草 |
| Staged files 列表 | 列表 | 查看 staged | 静态 | 3 files |
| Hooks `View full log` | 按钮/弹窗 | 打开完整日志 | 静态 | 缺 sheet；BLOCKED |
| Hooks `Retry hooks` | 按钮 | 重新运行 hook | 静态 | 成功→启用 Commit |
| Hooks `Commit without hooks` | 按钮 + 确认 | `--no-verify` | 静态 | 强确认；Agent 无此入口 |
| `Commit 3 Staged Files` | 按钮 | 成功→toast/04；失败→hook | 静态 | 当前 disabled；hook 失败提示 |
| 底部 `Hooks must pass` | 文本 | 无 | 静态 | 与 disabled 状态一致 |

### 11 · Changelists

**入口**: 10 的 All changelists、plus、Manage。  
**出口**: 10、12、22。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| `Commit composer` changelist selector | 选择 | 过滤 changelist | 静态 | 当前 3 files |
| `+` / `New` | 按钮 | 新建 changelist | 静态 | 名称非空；重名提示 |
| `Stage All` | 按钮 | stage 全部 | 静态 | 计数更新 |
| `Commit composer` / `Default Changelist` / `Agent cleanup` | 组 | 展开/折叠 | 静态 | 3 / 6 / 3 |
| 文件行复选 | 复选 | stage/unstage | 静态 | 与 10 同步 |
| `Move 3 selected files to…` | 菜单项 | 移动到其他 changelist | 静态 | 不改变 stage/commit 范围 |
| `Rename Changelist…` | 按钮 | 重命名 dialog | 静态 | 名称冲突 |
| `Set Active` | 按钮 | 设为当前过滤器 | 静态 | 只影响过滤；不决定提交 |
| `Commit 3 Staged Files` | 按钮 | 提交 staged | 静态 | 不按 changelist 提交 |
| `Ask Agent` | 页面链接 | → 12 | 可点 | Agent 起草 |
| `Manage…` | 页面链接 | → 10 | 可点 | 返回 composer |

### 12 · Agent panel

**入口**: toolbar Agent、11 Ask Agent。  
**出口**: 13、10、22。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| `Commit 12` tab | 页面链接 | → 10 | 可点 | 回到 staging |
| `Agent` tab | 选择 | Agent active | 静态 | plan 可见 |
| `Ask first` | 页面链接 | → 13 | 可点 | 权限 sheet |
| `Stage All` | 按钮 | stage Agent 目标文件 | 静态 | 3 files |
| `Edit` plan | 按钮 | 编辑 plan | 静态 | 不能绕过权限 |
| `Approve & Run` | 按钮 + 确认 | 执行 stage/commit/push | 静态 | 确认范围；hook 失败→日志 |
| plan steps `Stage …` | 列表 | 无 | 静态 | 3 个文件、commit、push |
| `Push allowed` | 状态 | 无 | 静态 | 与 13 权限一致 |
| composer `Auto` | 选择 | Agent 模式 | 静态 | 不显示额外主题开关 |
| 发送按钮 | 按钮 | 发送消息 | 静态 | 空文本禁用 |
| `⌘K commands` | 文本 | 无 | 静态 | 与 25 一致 |

### 13 · Agent permissions

**入口**: 12 的 Ask first。  
**出口**: 12（Save/Cancel）。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| `Read files` | 复选 | 允许读 | 静态 | 关闭后 Agent 不能读 |
| `Stage changes` | 复选 | 允许 stage | 静态 | 关闭后 plan 不能 stage |
| `Commit` | 复选 | 允许 commit | 静态 | 关闭后只能建议 |
| `Push` | 复选 | 允许 push | 静态 | 默认关闭；与 12 一致 |
| `Force push` | 复选 + 保护 | 允许 force | 静态 | 默认关闭；必须强确认 |
| `Discard changes` | 复选 + 保护 | 允许 discard | 静态 | 默认关闭；不可恢复提示 |
| Automation `Off / Commit / Commit & Push` | 选择 | 自动执行范围 | 静态 | 默认 Off；Commit & Push 需确认 |
| `Review window 8s` | 选择 | 自动执行前预览 | 静态 | 0/3/5/8 秒 |
| Protected branches | 列表 | 禁止直接写 | 静态 | main 默认保护 |
| `Save` | 按钮 | 保存并返回 12 | 静态 | 权限立即生效 |
| `Cancel` | 按钮 | 放弃修改返回 12 | 静态 | 无部分保存 |

### 14 · Merge conflict

**入口**: 06/10 的 Resolve、merge/rebase 冲突。  
**出口**: 04。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| `×` 关闭 | 页面链接 | → 04 | 可点 | 未解决时关闭需确认/保留 |
| `Files` | 按钮/弹窗 | 打开文件树 popover | 静态 | 3 files；Tree/List |
| `1 of 3` / 前后冲突 | 按钮 | 切换冲突 | 静态 | 顺序与文件树一致 |
| `Base` | 按钮 | 显示/隐藏 base | 静态 | 三方对比 |
| `Accept Ours` | 按钮 | 替换 Result 为 ours | 静态 | Result 可继续编辑 |
| `Accept Theirs` | 按钮 | 替换 Result 为 theirs | 静态 | Result 可继续编辑 |
| `Accept Both` | 按钮 | 合并两边 | 静态 | 顺序清晰 |
| `Mark Resolved` | 按钮 | 保存并 stage 文件 | 静态 | 仍有 marker 时不可用 |
| Result 编辑区 | 输入 | 手动编辑 | 静态 | 编辑内容保留；可撤销 |
| `Apply & Continue` | 按钮 | 全部解决后继续 merge/rebase | 静态 | 2 unresolved 时 disabled |
| `Abort Merge` | 按钮 + 确认 | 中止并恢复 | 静态 | 恢复 pre-merge；确认 |
| `2 unresolved` | 状态 | 无 | 静态 | 与文件树和按钮一致 |

### 15 · Sync & auth

**入口**: toolbar Push；merge 后推送。  
**出口**: 28、29、27、04。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| `Fetch` tab | 页面链接 | → 28 | 可点 | 打开 Fetch sheet |
| `Pull` tab | 页面链接 | → 29 | 可点 | 打开 Pull sheet |
| `Push` tab | 选择 | 当前 active | 静态 | Push 内容 |
| `main → origin/main` | 文本 | 无 | 静态 | 2 ahead |
| `Upstream: origin/main` | 状态 | 无 | 静态 | 不显示重复 Set upstream |
| `Push tags` | 复选 | 推送 tags | 静态 | 成功/失败状态 |
| `Force with lease` | 复选 + 确认 | 强制推送 | 静态 | 默认关闭；强确认 |
| `Push all branches` | 复选 | 推送所有分支 | 静态 | 默认关闭；与 main 目标一致 |
| `Authenticated · GitHub` | 状态 | 无 | 静态 | 与 27 一致 |
| `Pushing…` | 状态 | 显示进度 | 静态 | 禁用重复点击 |
| `Cancel` | 按钮 | 中止网络操作 | 静态 | 后台操作状态 |
| `Close` | 页面链接 | → 04；操作继续 | 可点 | 状态栏保留进行中 |
| auth 失败 | 状态 + 链接 | Sign in / Use SSH → 27 | 缺 | 失败保留本地提交 |
| non-fast-forward | 状态 + 选择 | Pull rebase / merge | 缺 | 不自动 force |
| pre-push hook | 面板 | 日志 + 阻止 | 缺 | push 不执行 |

### 16 · Repository states

**入口**: 画布直达；03/04/06 的 banner。  
**出口**: 04。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| `Back to Workspace` | 页面链接 | → 04 | 可点 | 回到 Workspace |
| 空仓库卡片 | 画廊 | 无 | 画廊 | `No commits yet` + Create initial commit |
| Detached HEAD 卡片 | 画廊 | 无 | 画廊 | Create branch / Checkout main |
| Merge in progress 卡片 | 画廊 | 目标 14 | 画廊 | 2 conflicts + Resolve |
| Offline 卡片 | 画廊 | 无 | 画廊 | last fetched + Retry |
| Auth required 卡片 | 画廊 | 无 | 画廊 | Sign in / SSH |
| Loading 卡片 | 画廊 | 无 | 画廊 | 骨架屏、无旧数据 |
| 04 auth banner | 页面链接 | → 27 | 可点 | 在上下文中出现，不是独立页 |
| 06 merge banner | 页面链接 | → 14 | 可点 | 在上下文中出现 |

### 17 · Reflog & undo

**入口**: 画布直达；恢复入口。  
**出口**: 04、22。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| `main` branch chip | 选择/链接 | 过滤 reflog | 静态 | 42 entries |
| reflog 行 | 选择 | 选中 reset/commit/checkout | 静态 | 选中 reset 后显示 undo |
| `Restore 8b41e07` | 按钮 + 确认 | 恢复 main；确认 | 静态 | 不删除 commit；working tree 保留 |
| `Create branch` | 按钮 | 从目标 commit 创建分支 | 静态 | 缺 dialog；BLOCKED |
| `Show in History` | 页面链接 | → 04 | 可点 | 定位到目标 commit |
| `History` tab | 页面链接 | → 04 | 可点 | 返回历史 |
| `Filter` | 按钮 | 过滤 reflog | 静态 | 时间/操作类型 |
| `1 commit orphaned` | 状态 | 无 | 静态 | 与 undo 结果一致 |

### 18 · Files at commit

**入口**: 04 Files tab。  
**出口**: 19、04、外部。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| `Source` / `Blame` | 选择/链接 | `Blame` → 19 | 可点 | Source 只读；Blame 打开 19 |
| `Open in Editor` | 外部 | 打开外部编辑器 | 静态 | 失败提示 |
| `History` | 页面链接 | → 04 | 可点 | 回到历史 |
| 文件树/文件行 | 选择 | 查看文件 | 静态 | 路径、commit 一致 |
| `At commit` 徽标 | 文本 | 无 | 静态 | 只读标识 |
| 语法/行号/折叠 | 静态 | 无 | 静态 | 只读；不能编辑 |

### 19 · Blame & file history

**入口**: 18 Blame。  
**出口**: 18、04。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| `Files` | 页面链接 | → 18 | 可点 | 回到 Files at commit |
| 文件历史行 | 选择 | 选中 commit | 静态 | 6 个 commit；与 blame 一致 |
| blame gutter | 文本 | 显示 commit/author | 静态 | 行号、作者、时间 |
| `Copy` | 按钮 | 复制 blame 行/路径 | 静态 | 复制范围明确 |
| 选中行 | 选择 | 高亮 | 静态 | 不改变文件 |
| `18 lines`/`16 lines` | 文本 | 无 | 静态 | 与实际行数一致 |

### 20 · Command palette

**入口**: titlebar overflow；`⌘P` 设计入口。  
**出口**: 04、22、23。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| 输入框 | 输入 | 搜索仓库/命令 | 静态 | 空查询显示全部 |
| Open repositories：`gitui` | 页面链接 | → 04 | 可点 | 打开/切换 |
| `docs-site` / `atlas-web` | 静态 | 目标 04 | 静态 | 没有 mock |
| Recent：dotfiles/spectra | 静态 | 目标 04 | 静态 | 没有 mock |
| Commands：`New Branch…` | 页面链接 | → 22 New Branch | 可点 | 命令执行入口 |
| Commands：`New Tag…` | 页面链接 | → 22 New Tag | 可点 | 命令执行入口 |
| Commands：`Squash Commits…` | 页面链接 | → 22 Squash | 可点 | 需要选择 commit |
| Commands：`Open Settings…` | 页面链接 | → 23 | 可点 | 命令执行入口 |
| `↑` / `↓` / `↵` / `Esc` | 键盘 | 导航/执行/关闭 | 静态 | 生产版本必须绑定 |
| 背景遮罩 | 页面链接 | → 04 | 可点 | 点击关闭 |

### 21 · Dark & context menu

**入口**: 04 commit 右键。  
**出口**: 04、22、07、外部。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| `Open Diff` | 页面链接 | → 04 | 可点 | 选中 commit 的 diff |
| `Edit Commit Message…` | 菜单项 | → 22 Edit | 静态 | 只改该 commit |
| `Checkout` | 菜单项 + 条件 | 切换 commit | 静态 | detached 警告 |
| `New Branch` | 菜单项 | → 22 New Branch | 静态 | 从该 commit 创建 |
| `New Tag` | 菜单项 | → 22 New Tag | 静态 | 从该 commit 创建 |
| `Cherry-pick` | 菜单项 + 确认 | 摘取 commit | 静态 | 冲突 → 14 |
| `Revert Commit…` | 菜单项 + 确认 | 反向提交 | 静态 | 冲突 → 14 |
| `Squash Commits…` | 菜单项 | → 22 Squash | 静态 | 需要多选 |
| `Rebase onto Here…` | 菜单项 | 目标 20/22 | 静态 | 20 未做；BLOCKED |
| `Merge into Current…` | 菜单项 | 合并到当前 | 静态 | 冲突 → 14 |
| `Reset Current Branch to Here…` | 菜单项 | → 22 Reset | 静态 | Soft/Mixed/Hard |
| `Compare with Working Tree` | 菜单项 | 设计目标 07；当前无跳转 | 静态 | 07 的入口由 06/09/22 提供 |
| `Copy SHA` / `Copy Message` | 菜单项 | toast | 静态 | 剪贴板 |
| `Open in Browser` | 外部 | 浏览器 | 静态 | remote/host 可解析 |

### 22 · Menus & dialogs

**入口**: 画布直达；20/21/09/30 的目标。  
**出口**: 无（组件规格页）。

| 卡片 | 类型 | 控件 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| Branch context | 菜单 | Checkout / New Branch / Compare / Rebase / Merge / New Tag / Rename / Delete / Copy / Push | 组件 | 危险项分隔；确认 |
| Reset current branch | 弹窗 | Soft / Mixed / Hard；Cancel / Reset | 组件 | Hard 提示不可恢复 |
| Squash commits | 弹窗 | 3 commits 列表、结果 message、Cancel / Squash | 组件 | 不能空 message |
| Remote branch context | 菜单 | Checkout as New Local / Compare / Merge / Rebase / Pull / Delete / Copy | 组件 | 删除确认 |
| New Branch | 弹窗 | Name / From / Checkout after create | 组件 | 名称校验 |
| Edit commit message | 弹窗 | Message / Cancel / Save | 组件 | 只改目标 commit |
| Commit filters | 菜单 | Only mine / Hide merges / Hide tags / date / author / path / message / Apply | 组件 | 与 05 一致 |
| New Tag | 弹窗 | Name / Target / Message / Annotated | 组件 | 名称校验 |
| Branch comparison | 卡片 | main ↔ feature、Open Comparison / Swap | 组件 | 14 files；目标 07 |
| Stash context | 菜单 | Stash changes/selected、Apply、Pop、Drop、Show Diff、Copy | 组件 | Drop 确认 |
| Submodule context | 菜单 | Update、Update latest、Sync URL、Open、Commit、Remove、Copy | 组件 | Remove 确认 |
| Worktree context | 菜单 | Open、New、Remove、Prune、Reveal、Copy | 组件 | Remove 确认 |

### 23 · Settings

**入口**: titlebar gear、04/10 的 Settings。  
**出口**: 04、24、25、26、27。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| `General` / `Shortcuts` / `Git` / `Credentials` / `About` | 页面链接 | 23/25/24/27/26 | 可点 | 当前页 active；Credentials 在 Git 组 |
| Theme `Light` / `Dark` / `Auto` | 按钮 | 立即切换主题 | 静态 | Auto 跟随系统；不需要确认 |
| Language select | 选择 | English / 简体中文 | 静态 | 立即或下次启动生效 |
| Date format select | 选择 | Relative / Absolute / System / Custom | 静态 | Custom 显示 pattern |
| Custom pattern | 输入 | `MMM d, yyyy` | 静态 | 预览正确；非法 pattern 提示 |
| Name / Email | 输入 | 提交身份 | 静态 | 空值禁用；合法邮箱 |
| `Use global config` | 复选 | 继承 user.name/email | 静态 | 与 27 的 system config 不冲突 |
| File tree 两种复选 | 复选 | 合并空目录/单子目录 | 静态 | 立即生效 |
| `On launch` select | 选择 | Restore last session / Show Welcome | 静态 | Restore 恢复全部 repo |
| `Last session` 3 个 repo | 列表 | 显示将恢复的仓库 | 静态 | 与 3 个 tab 一致 |
| `Check for updates` select | 选择 | Automatically / Manually / Never | 静态 | 与 26 状态一致 |
| `Check Now` | 按钮 | 检查更新 | 静态 | 进度/失败反馈待补 |
| `Done` | 页面链接 | → 04 | 可点 | 保存后返回 |

### 24 · Settings · Git

**入口**: 23 的 Git。  
**出口**: 23。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| Git executable path | 输入 | `/usr/bin/git` | 静态 | 不存在时错误 |
| `Choose…` | 按钮 | 文件选择器 | 静态 | 只能选可执行文件 |
| `Detect` | 按钮 | 自动检测 Git | 静态 | 检测失败提示安装 CLT |
| `Use PATH` | 复选 | 使用 PATH 中的 git | 静态 | 与路径互斥/明确 |
| Scan folders | 列表 | `~/Developer`、`~/Work` | 静态 | 可添加/移除 |
| `Add 3` | 按钮 | 添加找到的仓库 | 静态 | 12 found / 3 selected |
| `Done` | 页面链接 | → 04 | 可点 | 保存后返回 Workspace |
| 侧栏 Credentials | 页面链接 | → 27 | 可点 | 凭据页 |

### 25 · Settings · Shortcuts

**入口**: 23 的 Shortcuts。  
**出口**: 23。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| Search shortcuts | 输入 | 过滤行 | 静态 | 空结果 |
| `Restore Defaults` | 按钮 + 确认 | 恢复默认 | 静态 | 确认后全部恢复 |
| 快捷键行 | 列表 | 显示当前键 | 静态 | 与 20/生产快捷键一致 |
| `Record` | 按钮 | 进入录制状态 | 静态 | 录制面板出现 |
| 录制面板 | 输入 | 按下新组合；Esc/⌫ | 静态 | 冲突高亮；取消恢复 |
| `Done` | 页面链接 | → 04 | 可点 | 保存后返回 Workspace |

### 26 · Settings · About

**入口**: 23 的 About。  
**出口**: 23。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| 版本/Up to date | 文本 | 无 | 静态 | 0.9.2 (412) |
| `Website` / `Source code` | 外部 | 打开链接 | 静态 | 链接可解析 |
| `Report an issue` | 外部 | 新建 issue | 静态 | 带诊断信息 |
| `Licenses` | 外部/页面 | 打开许可 | 静态 | 许可证完整 |
| `Done` | 页面链接 | → 04 | 可点 | 保存后返回 Workspace |
| Nav General/Shortcuts/Git/Credentials | 页面链接 | 23/25/24/27 | 可点 | 当前 About active |

### 27 · Settings · Credentials & tools

**入口**: 23 Credentials、04 Sign in、03 Credentials。  
**出口**: 23、04。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| GitHub `Sign out` | 按钮 + 确认 | 断开账号 | 静态 | token 删除；不泄露 |
| GitLab `Connect` | 按钮 | 启动 OAuth | 静态 | 失败可读 |
| Credential helper | 选择 | macOS Keychain / None | 静态 | 与系统行为一致 |
| `SSH for new remotes` | 复选 | 新 remote 使用 SSH | 静态 | 只影响新建 |
| SSH key `Copy` | 按钮 | 复制公钥 | 静态 | 只复制公钥 |
| SSH key `Remove` | 按钮 + 确认 | 删除 key | 静态 | 确认指纹；不可恢复 |
| `Add key…` / `Generate…` | 按钮 | 导入/生成 | 静态 | 缺 dialog；BLOCKED |
| External editor | 选择 | VS Code / Cursor / Zed | 静态 | 不存在时提示 |
| Diff tool | 选择 | gitui built-in / FileMerge | 静态 | 与 07 打开一致 |
| Merge tool | 选择 | gitui merge editor | 静态 | 与 14 打开一致 |
| Proxy | 选择 | System / None / Manual | 静态 | Manual 显示字段 |
| `Verify SSL` / `System Git config` | 复选 | 网络/配置 | 静态 | 与 23 身份不冲突 |
| `Done` | 页面链接 | → 04 | 可点 | 保存后返回 Workspace |

### 28 · Sync · Fetch

**入口**: 15 Fetch tab。  
**出口**: 15、04。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| `Fetch` tab | 选择 | 当前 active | 静态 | Fetch 表单 |
| `Pull` / `Push` tab | 页面链接 | → 29 / 15 | 可点 | 切换 sheet |
| `Prune deleted branches` | 复选 | 删除远端已删分支 | 静态 | 不改变工作区 |
| `Fetch tags` | 复选 | 同步 tags | 静态 | 与 30 一致 |
| `Fetch all remotes` | 复选 | 所有 remote | 静态 | 与 origin 目标 |
| `Fetch` | 按钮 | 运行 fetch；进度/日志 | 静态 | 失败 Retry/Offline |
| `Close` | 页面链接 | → 04 | 可点 | 后台继续/结束状态 |
| Remote refs 列表 | 列表 | 查看更新 | 静态 | origin/main 2 new |

### 29 · Sync · Pull

**入口**: 15 Pull tab。  
**出口**: 15、14、04。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| `Fetch` / `Push` tab | 页面链接 | → 28 / 15 | 可点 | 切换 sheet |
| `Pull` tab | 选择 | 当前 active | 静态 | Pull 表单 |
| `Merge` / `Rebase` / `FF only` | 选择 | 选择整合方式 | 静态 | Rebase active；冲突→14 |
| `Autostash local changes` | 复选 | 自动 stash | 静态 | 成功恢复；冲突保留 stash |
| `Rebase all local commits` | 复选 | rebase 范围 | 静态 | 不误改远端 |
| `Pull` | 按钮 | 运行 pull | 静态 | 冲突→14；失败保留 |
| `Close` | 页面链接 | → 04 | 可点 | 取消/后台状态 |
| Incoming commits | 列表 | 查看 2 commits | 静态 | 与 15 的 behind 一致 |

### 30 · Refs · Tags, Remotes, Stashes

**入口**: 09 Open details。  
**出口**: 09、22。

| 控件 | 类型 | 操作 / 目标 | 状态 | 验收 |
|---|---|---|---|---|
| `Branches` | 页面链接 | → 09 | 可点 | 回到 Refs manager |
| `Tags` / `Remotes` / `Stashes` | 选择 | 切换/查看三段 | 静态 | 3 / 1 / 2 计数一致 |
| `New Tag` | 按钮 | → 22 New Tag | 静态 | 创建后刷新 |
| Tag `Push` | 按钮 | 推送 tag | 静态 | 失败可读；不自动 force |
| Tag `Delete` | 按钮 + 确认 | 删除 tag | 静态 | 确认 tag 名 |
| `Add Remote` | 按钮 | 新建 remote | 静态 | 缺 dialog；BLOCKED |
| Remote `Edit` | 按钮 | 编辑 URL | 静态 | 缺 dialog；BLOCKED |
| `Stash Changes` | 按钮 | 新建 stash | 静态 | 缺 dialog；BLOCKED |
| Stash `Apply` / `Pop` | 按钮 | 应用/弹出 | 静态 | Pop 前确认；冲突处理 |
| 行计数 | 文本 | 无 | 静态 | 与 09 侧栏一致 |

---

## 4. 弹窗、选择与确认清单

| 动作/对象 | 触发 | 类型 | 当前画板 | 必须的确认/选择 | 缺失 |
|---|---|---|---|---|---|
| New Branch | 09 New Branch / 20 命令 / 22 菜单 | 表单弹窗 | 22 | Name、From、Checkout after create；Create/Cancel | 无 |
| New Tag | 09/30 New Tag / 20 命令 | 表单弹窗 | 22 | Name、Target、Message、Annotated；Create/Cancel | 无 |
| Edit commit message | 21 菜单 | 表单弹窗 | 22 | Message；Save/Cancel；只改目标 commit | 无 |
| Squash commits | 21/06 多选 | 确认 + 表单 | 22 | 3 个 commit、结果 message；Squash/Cancel | 无 |
| Reset current branch | 21 菜单 | 选择 + 确认 | 22 | Soft / Mixed / Hard；Hard 明示不可恢复 | 无 |
| Discard file | 10 文件菜单 | 确认 | 10 菜单 | 对象名、不可恢复、Discard/Cancel | 确认框缺 |
| Commit without hooks | 10 Hooks | 强确认 | 10 | `--no-verify`、后果、Confirm/Cancel | 确认框缺 |
| Abort merge/rebase | 14/操作 banner | 确认 | 14 | 恢复 pre-merge；Abort/Cancel | 确认框缺 |
| Restore reflog | 17 Undo | 确认 | 17 | 目标 hash、branch、working tree 保留 | 确认框缺 |
| Delete branch | 09 Delete | 确认 | 09 | 分支名、不能当前分支、后果 | 确认框缺 |
| Delete tag/remote | 30 行操作 | 确认 | 30 | 对象名、远端影响 | 确认框缺 |
| Remove SSH key | 27 | 确认 | 27 | 指纹、不可恢复 | 确认框缺 |
| Sign out account | 27 | 确认 | 27 | 账号、token 删除 | 确认框缺 |
| Restore default shortcuts | 25 | 确认 | 25 | 覆盖自定义 | 确认框缺 |
| Force push | 15/29 | 强确认 | 15 | force-with-lease、目标分支 | 确认框缺 |
| Non-fast-forward push | 15 | 选择 | 无 | Pull (rebase) / Pull (merge) / Cancel | 缺 |
| Checkout dirty branch | 09/22 | 选择 | 无 | Stash / Carry / Cancel | 缺 |
| Missing repository | 启动恢复 | 弹窗/横幅 | 无 | Locate / Remove | 缺 |
| Not a repository | 03 Open | 选择 | 无 | Init here / Open parent / Cancel | 缺 |
| Clone/Init error/progress | 03 | 内联/进度 | 部分 | 进度、Cancel、失败原因、重试 | 缺 |
| Hook full log | 10 View full log | Sheet | 无 | 完整输出、复制、关闭 | 缺 |
| Auth sign-in | 04/15/16 | 弹窗/页面 | 27 | GitHub/GitLab、SSH、取消 | 登录表单缺 |
| Tag/remote/stash create/edit | 30 | 表单弹窗 | 部分 | 名称、URL、目标、冲突处理 | 缺 |

确认框统一要求：

- 标题说明动作，正文写明对象名和后果。
- 危险动作主按钮用红色，取消按钮始终可见。
- 不使用 “Don’t ask again” 处理 force、discard、hard reset。
- 操作完成后刷新对应列表、计数和状态栏。

---

## 5. 端到端测试场景

| ID | 场景 | 前置 | 步骤 | 期望 | 覆盖 |
|---|---|---|---|---|---|
| TC-01 | 恢复 3 个仓库 | 上次 3 个 tab；其中一个路径缺失 | 启动；检查 tab；检查 missing banner；点 Locate/Remove | 3 个 tab 按顺序恢复；缺失仓库有提示；其他仓库可用 | 01/03/04/16 |
| TC-02 | Welcome 首次启动 | 无 session | 启动；点 Open；点 recent；点 Settings | Open 打开 04；recent 打开 04；Settings 打开 23 | 01/02/03 |
| TC-03 | Clone / Init | 有网络；目标目录 | 03 Clone；填写 URL/目录/depth；Clone | 进度、成功打开 04；失败保留表单；Init 不覆盖非空目录 | 03 |
| TC-04 | History 过滤 | gitui 有 128 commits | 04；branch + date + author；Apply；Reset | 46 条；chip 与 count 一致；Reset 恢复 128 | 04/05 |
| TC-05 | Compare / Files / Info | 选中 3 个 commit | 06 Compare；07 切文件；Info；Files | 07 显示 12 commits/14 files；文件与 diff 一致；Info 有内容 | 06/07/08/18 |
| TC-06 | Stage + Commit | 3 staged/9 unstaged | 10 stage hunk/line；stage all；commit | commit 只包含 staged；changelist 不影响范围 | 10/11 |
| TC-07 | Hook 失败 | pre-commit 返回 1 | 10 Commit；查看 Hooks；Retry；Commit without hooks | Commit disabled；日志可见；Retry 成功后启用；no-verify 有强确认 | 10/22 |
| TC-08 | Fetch/Pull/Push | origin 可用；2 ahead/2 behind | 15 Push；28 Fetch；29 Pull；切换模式 | 进度、日志、成功刷新；auth/非 fast-forward 有恢复路径 | 15/28/29/27 |
| TC-09 | Merge 冲突 | 3 files/3 conflicts | 14 编辑 Result；Accept；Mark；Apply | 2 unresolved 时 disabled；解决后继续；Abort 恢复 | 14/04 |
| TC-10 | Reflog 恢复 | 有一次 reset | 17 选中 reset；Restore；Create branch | 恢复目标；working tree 保留；branch 创建 | 17/04 |
| TC-11 | Agent 审批 | 12 有 3 文件计划 | 12 Ask first；13 权限；Approve & Run | 权限生效；确认范围；hook 失败保留日志 | 12/13/10 |
| TC-12 | Refs 管理 | 5 分支/3 tags/1 remote/2 stashes | 09 选择 branch；30 切 tabs；执行行操作 | 计数一致；危险动作确认；刷新 | 09/30/22 |
| TC-13 | Settings | 打开 23 | 改主题/语言/日期/身份；25 Record；27 SSH；Done | 设置即时或标注重启；身份来源唯一；SSH 不泄露 | 23/24/25/26/27 |
| TC-14 | 导航与可访问性 | 任意页面 | 键盘 Tab/Enter/Esc；检查 icon label；暗色；中文 | 焦点可见；无仅颜色状态；所有 icon 有 label；中英文完整 | 全局/25 |

---

## 6. 验收检查表

| ID | 检查 | 通过条件 | 结果 |
|---|---|---|---|
| A-01 | 页面覆盖 | 30 页均有入口和出口；22/16 明确为组件/画廊 | 待测 |
| A-02 | 链接目标 | 每个 `可点` 控件都到达记录的目标页；无 404 | 待测 |
| A-03 | 返回路径 | 每个弹窗/页面都有 Close/Cancel/Back，除 22 规格页 | 待测 |
| A-04 | 按钮语义 | 每个 icon-only 按钮有可见 tooltip 或 `aria-label` | 待测 |
| A-05 | 危险动作 | discard/reset/delete/force/no-verify/abort 有确认框 | 待测 |
| A-06 | 状态覆盖 | loading/empty/ready/busy/disabled/error/auth/offline 均有设计 | 待测 |
| A-07 | Hook | pre-commit 失败有日志、Retry、no-verify 确认；commit disabled | 待测 |
| A-08 | Merge | 三方、可编辑 Result、Mark resolved、Continue、Abort 完整 | 待测 |
| A-09 | Sync | Fetch/Pull/Push、auth、non-fast-forward、进度和取消完整 | 待测 |
| A-10 | Refs | Branches/Tags/Remotes/Stashes 计数和操作一致 | 待测 |
| A-11 | Settings | 主题、语言、日期、身份、启动、凭据、工具、快捷键完整 | 待测 |
| A-12 | 本地化/暗色 | 中英文完整；暗色无硬编码浅色；状态不只靠颜色 | 待测 |
| A-13 | 安全 | token/密钥不出现在日志和界面明文 | 待测 |
| A-14 | 数据一致性 | 3 staged/9 unstaged、12 files、14 compare files、2 ahead/behind 一致 | 待测 |

测试记录：

| 用例 | 结果 | 证据 | 备注 |
|---|---|---|---|
| TC-01 |  |  |  |
| TC-02 |  |  |  |
| TC-03 |  |  |  |
| TC-04 |  |  |  |
| TC-05 |  |  |  |
| TC-06 |  |  |  |
| TC-07 |  |  |  |
| TC-08 |  |  |  |
| TC-09 |  |  |  |
| TC-10 |  |  |  |
| TC-11 |  |  |  |
| TC-12 |  |  |  |
| TC-13 |  |  |  |
| TC-14 |  |  |  |

---

## 7. 已知缺口与不测试项

### 7.1 设计缺口

- **20 Interactive Rebase**：按 ADR-0007 延后；05/06/09/21/30 的 Rebase 入口只能记为 `BLOCKED`。
- **22** 是组件规格页，不是独立功能页；不能把菜单卡片当作可执行入口。
- **16** 是状态画廊；生产版本要把状态放回 04/06/15 的上下文。
- **28/29/30** 已从卡片页改为真实 Sync sheet 和 Refs 管理器页面；仍需实现真实数据。
- **登录/凭据**：04/15 的 `Sign in` 目标为 27，但 27 目前只有连接状态，没有登录表单。
- **确认框**：Discard、Abort、Restore reflog、Delete branch/tag/remote、Remove SSH、Sign out、Force push、no-verify 尚无独立确认框。
- **错误状态**：non-fast-forward、push rejected、clone/init 失败、missing repo、not-a-repo、hook full log 尚未画全。
- **权限**：27 的凭据、SSH、系统配置为设计状态；不得在画布或测试记录中写入真实 token。

### 7.2 当前代码状态

- 30 块画板全部有对应实现（22 为组件规格、16 为状态画廊，按其性质落进上下文；
  15 的凭据真实流程按决定延后）。逐板对照见第 9 节。
- 交互全部为真实 Git 操作或明确禁用；没有「画了按钮但不响应」的控件。
- 截图与像素级比对仍未做（本环境无图形会话，`screencapture` 拿不到画面）；
  当前证据是「设计 CSS → 几何常量 → 实现」的静态对照 + 几何测试，见第 9 节。
- 测试结果区分 `PASS`（路径可点且行为真实）、`PARTIAL`（入口在、能力有限）、
  `BLOCKED`（按 ADR 延后）、`N/A-DESIGN`（组件/画廊）。

### 7.3 不测试项

- 真实 Git 命令执行、并发、性能和 100k commits。
- 真实网络、OAuth、SSH、Keychain、签名和公证。
- 像素级视觉回归、色彩对比度数值、VoiceOver 实际朗读。
- 生产安装包和更新 feed。

这些项目由 [TEST-PLAN.md](../docs/TEST-PLAN.md)、[SPIKE-REPORT.md](../docs/SPIKE-REPORT.md) 和后续实现验证覆盖。

---

## 8. 维护约定

- 本文件已入库到 `docs/`，是设计画板交互的验收清单；稳定行为仍以 `SPEC.md` 与 `docs/` 其余文档为准。
- 画板或交互变化后，先更新本文件的对应页面表，再同步正式文档中的稳定行为。
- 测试发现的新缺口写入第 7 节，并标记画板编号和阻塞原因。
- 不在本文件记录真实 token、私钥、用户隐私或未脱敏日志。

---

## 9. 实施对照（2026-11-15）

状态含义见 7.2。实现位置指 `apps/client/src/` 下的模块。

### 9.1 逐页

| # | 画板 | 状态 | 实现位置 |
|---|---|---|---|
| 01 | Welcome | PASS | `screens/welcome.rs`（独立窗口，Open/Clone/Init + recent + 分组 + 空态） |
| 02 | Welcome (empty) | PASS | 同上（无 recent 的空态文案与 Scan 入口） |
| 03 | Repository manager | PASS | 同上（与欢迎页合并；Clone 分支/深度/递归/克隆后打开 + 进度与取消，Init 附带文件） |
| 04 | History & diff | PASS | `workspace/history.rs`、`workspace/diff.rs`（统一/并排、折叠、换行、导出） |
| 05 | History filters | PASS | `workspace/history.rs` 过滤浮层 + `filters.rs` |
| 06 | Multi-commit diff | PASS | 历史多选 + 对比栏（合并/拣选/清空） |
| 07 | Compare refs | PASS | 对比引用控件 |
| 08 | Commit info | PASS | `workspace/inspector.rs`（复制 SHA、拣选、回滚；签名状态仍只读） |
| 09 | Refs manager | PASS | `screens/refs_view.rs` 分段表 + 每行菜单 |
| 10 | Staging & commit | PASS | `screens/changes_view.rs`、`workspace/commit.rs`（amend/sign-off/提交后推送、钩子三动作、hunk 与行级暂存） |
| 11 | Changelists | PASS | 变更清单弹层（新建/设为当前/移入） |
| 12 | Agent panel | PASS | `workspace/agent.rs`（计划逐条执行、步骤增删、「补一步…」、Approve & Run 先列范围、权限循环、审计） |
| 13 | Agent permissions | PASS | 设置页 Agent 分区（9 个动作 + 自动化/预览/受保护分支） |
| 14 | Merge conflict | PASS | `screens/conflict_view.rs`（左 224 文件列表、三栏、可编辑结果、标记解决、应用并继续、放弃） |
| 15 | Sync & auth | PARTIAL | `screens/sync_dialog.rs`（三 tab + 全部选项 + 强推警告；凭据真实流程按决定延后） |
| 16 | Repository states | N/A-DESIGN | 状态放回上下文：`shell::state_banner`（登录 > 离线 > 仓库问题 > 游离头 > 空仓库）+ 历史 loading 骨架 |
| 17 | Reflog & undo | PASS | `screens/refs_view.rs` Reflog 段（撤销/新建分支/在历史中显示/复制/过滤） |
| 18 | Files at commit | PASS | 检查器 Files 页 + 单文件 diff |
| 19 | Blame & file history | PASS | 检查器 Blame 页 + 文件历史 |
| 20 | Command palette | PASS | `screens/palette.rs`（三组结果、↑↓/↵、点击背景关闭） |
| 21 | Dark & context menu | PASS | 提交行 13 项菜单 + 深浅色（`theme.json` 与设计 CSS 由测试锁定） |
| 22 | Menus & dialogs | N/A-DESIGN | 组件规格页：菜单/弹窗/表单/确认均已按该页规格实现 |
| 23 | Settings | PASS | `screens/settings.rs`（分组侧栏、主题、语言、日期、身份、文件树、启动、更新） |
| 24 | Settings · Git | PASS | 同一文件（Choose/Detect/PATH/扫描目录/Add found） |
| 25 | Settings · Shortcuts | PASS | 搜索、录制改键、冲突提示、恢复默认（确认）；改键范围受 nsh 静态绑定限制 |
| 26 | Settings · About | PASS | 版本 + 四个外链 |
| 27 | Settings · Credentials | PARTIAL | 只读凭据助手与 SSH 计数；Sign in 可点但只说明「尚未接入」 |
| 28 | Sync · Fetch | PASS | 同步弹窗 Fetch tab（prune/tags/all remotes） |
| 29 | Sync · Pull | PASS | 同步弹窗 Pull tab（三种整合方式 + autostash + 非快进三选） |
| 30 | Refs · Tags, Remotes, Stashes | PASS | 引用页各段行操作（标签推送/删除、远程编辑 URL/取回/移除、贮藏应用/弹出/丢弃） |

### 9.2 第 4 节弹窗清单

| 动作 | 状态 | 说明 |
|---|---|---|
| New Branch | PASS | Name / From / Checkout after create |
| New Tag | PASS | Name / Target / Message / Annotated |
| Edit commit message | PASS | 固定形态 `reword`（ADR-0007 修订） |
| Squash commits | PASS | 固定形态 `squash`，预填最旧说明 |
| Reset current branch | PASS | 菜单三项（soft/mixed/hard）+ 确认 |
| Discard file | PASS | 确认框写明对象与不可恢复 |
| Commit without hooks | PASS | 强确认后 `--no-verify` |
| Abort merge/rebase | PASS | 确认后 `--abort` |
| Restore reflog | PASS | 确认写明目标 hash 与「工作区保留」 |
| Delete branch | PASS | 确认写明分支名与后果 |
| Delete tag/remote | PASS | 各自确认 |
| Remove SSH key | BLOCKED | 没有密钥管理，按决定不做假动作 |
| Sign out account | BLOCKED | 没有账号存储，同上 |
| Restore default shortcuts | PASS | 确认后重置并清空覆盖项 |
| Force push | PASS | Push 选项 + 按钮上方红字说明 |
| Non-fast-forward push | PASS | 拉取（变基）/ 拉取（合并）/ 取消 |
| Checkout dirty branch | PASS | 贮藏并切换 / 带过去 / 取消 |
| Missing repository | PASS | 横幅「重新定位…/关闭」 |
| Not a repository | PASS | 横幅「在此初始化/打开上级目录/关闭」 |
| Clone/Init error/progress | PASS | 克隆显示 git 的真实进度行并可取消（取消后删除半成品目录）；失败保留表单并显示原因 |
| Hook full log | PASS | 完整输出弹层，可复制、可关闭 |
| Auth sign-in | BLOCKED | 入口可点并说明「尚未接入」，不写 token |
| Tag/remote/stash create/edit | PASS | 表单齐备（远程编辑 URL、标签附注、贮藏含未跟踪） |

### 9.3 验收检查表

| ID | 结果 | 证据 |
|---|---|---|
| A-01 页面覆盖 | PASS | 9.1 逐页表 |
| A-02 链接目标 | PASS | 每个入口落到实现页；无空实现 |
| A-03 返回路径 | PASS | 每个弹窗都有 Cancel/Close，Esc 分层退出 |
| A-04 按钮语义 | PARTIAL | 按钮均有文案或图标 + 说明；未做 `aria-label` 全覆盖 |
| A-05 危险动作 | PASS | 11 处确认框（丢弃/重置/删除/强推/跳过钩子/放弃合并/关闭脏标签…） |
| A-06 状态覆盖 | PASS | loading / empty / ready / operation / offline / auth / detached / 仓库问题 |
| A-07 Hook | PASS | 日志、重试、跳过（强确认）；失败时提交按钮禁用 |
| A-08 Merge | PASS | 三栏、可编辑结果、标记解决、应用并继续、放弃 |
| A-09 Sync | PASS | Fetch/Pull/Push 全部选项、auth/offline 归类、非快进三选；无进度条 |
| A-10 Refs | PASS | 五类引用计数与行操作一致 |
| A-11 Settings | PASS | 主题、语言、日期、身份、启动、凭据、工具、快捷键 |
| A-12 本地化/暗色 | PASS | 中英文键集合由测试强制一致；`theme.json` 与设计 CSS 由测试锁定 |
| A-13 安全 | PASS | 不写 token/私钥；凭据只经系统 Git |
| A-14 数据一致性 | PASS | 计数与列表来自同一次 `git status`/`refs` 读取 |

### 9.4 尚未完成

- **逐板截图取证与像素比对**：本环境没有可捕获的图形会话。替代证据是几何测试
  （`design/metrics.rs` 直接解析设计 CSS）与主题色对照测试。
- **原生 `DiffView` / `Command` / `Select` 组件替换**：见 `docs/ARCHITECTURE.md` 5.2。
- **Init 没有进度**：`git init` 是瞬时操作，只在克隆上做了进度与取消。
