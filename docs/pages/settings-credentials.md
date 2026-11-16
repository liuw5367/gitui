# 凭据与工具 — 布局、状态与按钮

- **画板**：27 Settings · Credentials & tools（设置窗口第 5 项 Credentials；有 Connections / SSH keys / Tools / Network 四段十五行；无 OAuth 流程、无密钥生成、无 Manual 代理表单）
- **来源**：`designs/graft-git/project/SettingsCredentials.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

本分区是设置窗口的第 5 项（`Git` 组的第二项，树里写 `Credentials`，页面标题写 `Credentials & tools`）。外壳与
23 General 同一套：`.settings .window` `width:min(920px,100%)`、`height:min(700px,calc(100vh - 48px))`，画布
1440×900、`.desktop` 留白 24，因此窗口 **920×700**。内容区仍按四段纵向堆叠，每段切成标签列与控件列。

## 1. 骨架

```
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ ● ● ●                                      Settings                   gitui 0.9.2            │
├─────────────────────────┬────────────────────────────────────────────────────────────────────┤
│                         │ Credentials & tools                                                │
│ ▣ gitui                 │ Accounts, SSH keys, external editors, and network.                 │
│   Version 0.9.2         │                                                                    │
│                         │ Connections                                                        │
│ Application             │ Used for HTTPS clone, fetch, and push.                             │
│  [gear]   General       │GitHub            mia@github.com · OAuth  ● Connected [Sign out]    │
│  [keys]   Shortcuts     │GitLab            No account connected.             [Connect]       │
│  [branch] Agent         │Credential helper [ macOS Keychain          v ]                     │
│ Git                     │SSH for new remotes ☑                                               │
│  [branch] Git           │ SSH keys                                                           │
│* [lock]   Credentials   │ Keys available to git and the agent.                               │
│ About                   │id_ed25519        SHA256:9xK2…fQ1 · ~/.ssh/id_ed25519               │
│  [i]      About gitui   │                  [Copy] [Remove]                                   │
│                         │Add another key   [Add key…] [Generate…]                            │
│                         │ Tools                                                              │
│                         │ External apps used from diffs, files, and commit messages.         │
│                         │External editor   [ Visual Studio Code      v ]                     │
│                         │ ... Tools 4 more, Network 3 ...                                    │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ ● Changes are saved automatically                                                   [ Done ] │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 标题栏 | 顶部通栏 | 高 38；内距 0×13 | 窗口灯 12×3、居中 `Settings` 12.5/600、右端 `gitui 0.9.2` 11 |
| 左侧分区树 | 标题栏下左 | 宽 208；内距 12×8；行高 28 | 六项三组，`Credentials` 是当前项 |
| 内容区 | 标题栏下右 | 宽 710；内距 24×28 → 内容列 654、可视高 568 | 标题、副标题、四段十五行 |
| 页脚 | 底部通栏 | 高 44；内距 0×16 | 6 状态点、自动保存说明、右端 `Done` |

正文 616 = 700 − 2 − 38 − 44，内容列 654 = 710 − 28×2，可视高 568 = 616 − 24×2。四段十五行累加约 1200，右栏滚动
幅度是六块板里最大的；图中 `*` 是本分区，`...` 是被折叠的段，方括号内是图标或按钮的占位。

## 2. 外壳与左侧分区树

左树六项三组与其它五块板逐字相同：`Application` 下 General（齿轮）、Shortcuts（键帽）、Agent（分支）；`Git` 下
Git（分支）、Credentials（锁）；`About` 下 About gitui（信息）。本板当前项是 `Credentials`，`.on` 取末次声明
`background:var(--accent)` + 白字白图标 + 500，图标是锁。树项与标题不同名：树里为省宽度写 `Credentials`，标题写
`Credentials & tools`，副标题再把四块内容并列 `Accounts, SSH keys, external editors, and network.`

## 3. 内容头与 Connections

标题 17/700、字距 −.015em，副标题 11.5 `text3`、上距 4。`Connections` 段带内联 `border-top:0`、`margin-top:14px`、
`padding-top:0`；其余三段上距 20、上内距 16、上边框 1px `--line`；段标题 12.5/600、下距 2，段说明 10.5 `text3`、
下距 12。行间距 14、内距 `11px 0`、下边框 `--line-soft`，标签 12.5/500，说明 10.5 `text3`、上距 2。
`Connections`（`Used for HTTPS clone, fetch, and push.`）四行：`GitHub` → 说明 `mia@github.com · OAuth`，右列 6 的
`--ok` 状态点 + `.hint` `Connected` + `Sign out`；`GitLab` → `No account connected.`，右列只有 `Connect`；
`Credential helper` → `How HTTPS credentials are stored.`，右列原生 `select`（26 高、内距 0×8、字号 10.5）三项
`macOS Keychain`（选中）/ `None` / `Store in memory`；`SSH for new remotes` → `Prefer SSH URLs when adding a remote.`，
右列一枚勾选的复选。两枚按钮都是 `btn small` 22 高。板上两家远端一连接一未连接，同一段里因此有两种行形态：
带状态点与 `Sign out` 的已连接行、只有 `Connect` 的空行。

## 4. SSH keys

段说明 `Keys available to git and the agent.`，三行：`id_ed25519` → mono 9 的指纹加路径，右列 `Copy` + `Remove`；
`id_rsa` → 同样 `Copy` + `Remove`；`Add another key` → 右列 `Add key…` + `Generate…`。两行指纹写的是
`SHA256:9xK2…fQ1 · ~/.ssh/id_ed25519` 与 `SHA256:4mPz…c7R · ~/.ssh/id_rsa`。
两个指纹都做了中段省略，说明字号被内联压到 9（比其它行的 10.5 更小）且同样是 mono；两行右列各两枚按钮、间距 6，
是本板按钮最密的地方。

## 5. Tools 与 Network

**Tools**（`External apps used from diffs, files, and commit messages.`）五行，前四行都是 `select` + 一句说明：
`External editor` → `Visual Studio Code` / `Cursor` / `Zed` / `Terminal`（`Used by Open in Editor.`）；`Diff tool` →
`gitui built-in` / `FileMerge` / `Kaleidoscope`（`Used for side-by-side file comparison.`）；`Merge tool` →
`gitui merge editor` / `FileMerge`（`Used when resolving conflicts.`）；`Terminal` → `Terminal` / `iTerm2` / `Warp`
（`Used by Open in Terminal.`）。第五行 `Use external editor for commit messages` 是未勾选的复选。

**Network** 没有段说明（`.settings-sec-sub` 缺失），标题下只隔 2 就接第一行：`Proxy` → `System` / `None` / `Manual…`
（`Use the system proxy or a manual HTTP proxy.`）；`Verify SSL certificates` 勾选；`Use system Git config for other settings`
勾选。两句说明是 `Recommended. Disable only for trusted local hosts.`，以及
`Identity comes from General; other Git settings load from ~/.gitconfig.`。这里的 `select` 内联字号是 10.5，
而 General 与 Agent 的同位下拉写 11。

## 6. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 左侧分区 | 六项 | 只画 Credentials 选中 |
| 两个账号 | 各自已连接 / 未连接 | GitHub `● Connected`，GitLab 只有 `Connect` |
| helper 与密钥 | Keychain / None / Store in memory；0 / 1 / 2 把钥匙 | 只画 Keychain、2 把钥匙 |
| Proxy | System / None / Manual | 只画 System，`Manual…` 无表单 |

```
分区：General ⇄ Shortcuts ⇄ Agent ⇄ Git ⇄ Credentials ⇄ About gitui（只替换右栏）
账户：Connect ⇄ Sign out（只换该行右列，不动其它行）
离开：Done → Workspace.dc.html
```

## 7. 画板没覆盖的组合（待补设计）

1. **OAuth 流程与账号细节**：`Connect` 与 `Sign out` 之后的浏览器跳转、等待态、失败提示、scope 选择没有画；一个远端挂
   多个账号、换账号、token 过期时的行状态也没有画。
2. **密钥操作与空态**：`Copy` 复制的指纹还是公钥、`Remove` 是否二次确认、`Add key…` 与 `Generate…` 的对话框都没有画；
   SSH keys 段为空时的空态与引导也没有画。
3. **代理与工具缺失**：`Manual…` 选中后主机、端口、认证字段放哪没有画，SSL 校验关闭时的警告也没有画；选的编辑器
   没安装、`Diff tool` 指向不存在的命令时的错误提示也没有画。

## 8. 按钮与链接清单

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | Connections 段 GitHub 行 | Sign out | `btn small` 22 高 | 断开该账号 | 仅已连接时出现 |
| 2 | Connections 段 GitLab 行 | Connect | `btn small` 22 高 | 连接账号 | 未连接时出现，板上无进行中 |
| 3 | SSH keys 段每行 | Copy / Remove | `btn small` 22 高 ×2 | 复制标识 / 移除密钥 | 两把钥匙各一组，Remove 无确认 |
| 4 | SSH keys 段末行 | Add key… / Generate… | `btn small` 22 高 ×2 | 导入已有密钥 / 生成新的 ed25519 对 | 常驻 |
| 5 | 页脚右端 | Done | `btn` 24 高、内距 0×10、字号 12 | 关设置回主窗口 | 常驻 |
| 6 | 左侧树 | 六项 | 整行链接 28 高，图标 14 | 切分区，替换右栏 | 当前项 `--accent` 底 + 白字 |

- 板上没有图标按钮，因此本分区没有 `aria-label` 需求；八枚 `btn small`（`Copy`/`Remove` 各两枚）加页脚 `Done` 共九处，
  文案都直接说明动作。六个 `select` 与四枚复选是选项，不计入按钮；`Manual…` 的省略号表示会有后续界面，板上没有画。

## 9. 输入与选项

| 字段 | 形态 | 默认值（画板） | 说明 |
|---|---|---|---|
| GitHub / GitLab | 状态点 + 文本 + 按钮 | `mia@github.com · OAuth`（`Connected`）/ `No account connected.` | 右端分别是 `Sign out`、`Connect` |
| Credential helper | 原生 select 26 高 | `macOS Keychain` | 另两项 `None`、`Store in memory` |
| SSH for new remotes | 原生复选 | 勾 | 添加远端时优先 SSH URL |
| id_ed25519 / id_rsa | 只读 mono 9 | 指纹 + `~/.ssh/…` 路径 | 右端 `Copy`、`Remove` |
| External editor | 原生 select 26 高 | `Visual Studio Code` | 另三项 `Cursor`、`Zed`、`Terminal` |
| Diff / Merge / Terminal | 原生 select 26 高 ×3 | `gitui built-in` / `gitui merge editor` / `Terminal` | 备选分别含 `FileMerge`、`Kaleidoscope`、`iTerm2`、`Warp` |
| 提交信息用外部编辑器 | 原生复选 | 不勾 | 说明 `Open the full message in the editor.` |
| Proxy | 原生 select 26 高 | `System` | 另两项 `None`、`Manual…` |
| 其余复选 | 原生复选 ×2 | 都勾 | SSL 只对可信本地主机关闭；身份来自 General，其余来自 `~/.gitconfig` |

## 10. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 23 / 23b / 24 / 25 / 26 | 板内五个 `href`：`Settings.dc.html`、`SettingsShortcuts.dc.html`、`SettingsAgent.dc.html`、`SettingsGit.dc.html`、`SettingsAbout.dc.html` |
| 01 Welcome / 04 Commits | Clone 的 `Credentials`、主窗口授权横幅的 `Sign in…` 都指向本板；`Open in Editor` 用本板的 External editor |
| 23b Settings · Agent | Agent 的 `Local log` 的 `Open Log` 指向本板（板上还没有日志区） |
| 14 Merge conflict / 15 Sync & auth | `Merge tool` 在那块板生效；凭据与代理被同步流程复用 |
