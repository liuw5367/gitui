# 常规设置 — 布局、状态与按钮

- **画板**：23 Settings（设置窗口第 1 项 General；有 Appearance / Language & region / Dates & times / Commit identity / File tree / On launch / Updates 七段十三行；无深色、无窄窗、无保存失败态）
- **来源**：`designs/graft-git/project/Settings.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

本分区是设置窗口的**第 1 项**（`Application` 组的第一个），也是六块设置板共用的同一扇窗：`.settings .window`
`width:min(920px,100%)`、`height:min(700px,calc(100vh - 48px))`；预览画布 1440×900、`.desktop` 四周留白 24，
因此窗口 **920×700**，不是主窗口的 1392×852；右栏切成「标题 + 副标题 + 七段」，换分区只替换这部分。

实现用独立普通窗口（`WindowKind::Normal`）承担这扇窗，标题栏用**系统原生**的而不是画板自绘的 38 高栏，
标题 `Settings`，支持系统最小化、缩放与关闭；⌘W 关闭 Settings 自己，不会关闭主窗口仓库标签。Esc 不关窗，
因为快捷键页录制中 Esc 用于取消录制。页脚已移除，见 DEVIATIONS #34。见 [DEVIATIONS #23](../design/DEVIATIONS.md)。

欢迎页齿轮、主窗口齿轮、macOS 产品菜单中的 Settings 和快捷键 `⌘,` 都进入这扇窗。Settings 已打开时再次调用入口会聚焦已有窗口；不会重新读取并覆盖正在编辑的提交身份。关闭后窗口引用清空，下次打开才刷新当前仓库或全局身份。

## 1. 骨架

```
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ ● ● ●                                      Settings                   gitui 0.9.2            │
├─────────────────────────┬────────────────────────────────────────────────────────────────────┤
│                         │ General                                                            │
│ ▣ gitui                 │ Appearance, language, dates, and startup behavior.                 │
│   Version 0.9.2         │                                                                    │
│                         │ Appearance                                                         │
│ Application             │ Auto follows the macOS appearance.                                 │
│* [gear]   General       │Theme             Switch now or follow the system.                  │
│  [keys]   Shortcuts     │                  [ Light | Dark | Auto ]                           │
│  [branch] Agent         │ Dates & times                                                      │
│ Git                     │ Used in history, tags, reflog, and the agent log.                  │
│  [branch] Git           │Date format       [ Custom pattern     v ]                          │
│  [lock]   Credentials   │ ... 4 more sections ...                                            │
│ About                   │ Updates                                                            │
│  [i]      About gitui   │ Check for a new release at launch or on demand.                    │
│                         │Check for updates [ Automatically at launch v ]                     │
│                         │Status            gitui 0.9.2 (412) is up to date.  [Check Now]     │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 标题栏 | 顶部通栏 | 高 38；内距 0×13；间距 8 | 窗口灯 12×3、居中 `Settings` 12.5/600、右端 `gitui 0.9.2` 11 `text3` |
| 左侧分区树 | 标题栏下左 | 宽 208（含右边框 1）；内距 12×8；行高 28 | 应用标识块 + 三组六项 |
| 内容区 | 标题栏下右 | 宽 710；内距 24×28 → 内容列 654、可视高 568 | 标题、副标题、七段十三行表单 |
| ~~页脚~~ | — | — | 已移除（DEVIATIONS #34）：不再有「自动保存」说明与 `Done` |

920 = `min(920, 1392)`，700 = `min(700, 900 − 48)`；正文 616 = 700 − 2 描边 − 38 − 44，右栏 710 = 920 − 2 − 208，
内容列 654 = 710 − 28×2，可视高 568 = 616 − 24×2。本板 13 行按“行 57 / 段头 46 / 段距 37”累加约 1340，必然滚动。
图中 `*` 是本分区，`...` 是右栏滚动后被折叠的段，框内只放板上的英文与符号。

## 2. 外壳与左侧分区树

标题栏灯座 `gap:8`，标题绝对居中（`left:50%` + `translateX(-50%)`），右端只放版本号：设置窗口没有齿轮、
没有仓库页签，与主窗口 40 高的标题栏不是一套。左树头部内联 `padding:6px 10px 12px`，`appmark` 压到 28×28、
圆角 7、去投影，图标 15 白色，右侧 `gitui` 12/600 + `Version 0.9.2`（`.hint` 10.5 `text3`）。

组头 `.sg` 高 24、上距 8、内距 0×8、10.5/600 `text3`；样式表前面的 `9px` + `.05em` + `uppercase` 被末次声明
覆盖成 `10.5px / 0 / none`，所以板上是 sentence case。行 `.settings-nav` 高 28、内距 0×8、圆角 6、图标与文字
间距 7、字号 13，图标盒 16 宽、图形 14、常态 `text3`；当前项 `.on` 取末次声明 `background:var(--accent)` +
白字白图标、字重 500。六块板的树逐字相同，3 组 6 项：General（齿轮）、Shortcuts（键帽）、Agent（分支）、
Git（分支）、Credentials（锁）、About gitui（信息），分组依次是 `Application` / `Git` / `About`。

## 3. 内容头与行网格

右栏是一整块滚动容器，标题与副标题随内容滚走。`General` 17/700、字距 −.015em；副标题 11.5 `text3`、
上距 4。每段 `.settings-sec` 上距 20、上内距 16、上边框 1px `--line`，第一段被内联 `border-top:0`、
`margin-top:14px`、`padding-top:0` 压掉（与 `:first-of-type` 同值）；段标题 12.5/600、下距 2，段说明 10.5 `text3`。
行 `.set-row` 两列 flex：标签列 `flex:1`、控件列 `flex:none`、间距 14，行内距 `11px 0`，下边框 1px `--line-soft`，
段内末行无边框；标签 12.5/500，说明 10.5 `text3`、上距 2、行高 1.4 可折行。控件列固定贴右，因此
**标签列 = 654 − 控件宽 − 14**，本板写死 210 的字段对应标签列 430，`select` 取内容自然宽。

## 4. 七段的结构

| 段 | 段说明（板上原文） | 行（标签 → 控件与取值） |
|---|---|---|
| Appearance | Auto follows the macOS appearance. | Theme → `Light / Dark / Auto`，选中 Auto |
| Language & region | Default is English. A restart may be required. | Language → `English / 简体中文`，选中 English |
| Dates & times | Used in history, tags, reflog, and the agent log. | Date format → 四选一，选中 `Custom pattern`；Custom pattern → 字段 210 + `Preview · Nov 14, 2026` |
| Commit identity | Used for commits you create. Overrides the global Git config in this repository. | Name → `Mia Chen`；Email → `mia@gitui.dev`；Use global config → 未勾选 |
| File tree | How changed files and repository contents are grouped. | Merge empty directories → 勾；Compact single-child folders → 勾 |
| On launch | Choose what gitui opens when it starts. | Launch behavior → `Restore last session / Show Welcome`；Last session → 三个徽章 |
| Updates | Check for a new release at launch or on demand. | Check for updates → 三选一，选中 `Automatically at launch`；Status → `Check Now` |

`Dates & times` 的第二行内联 `padding-top:0`，紧贴上一行，两行之间只隔上行的 11 下内距与 1px 细线；Agent 与
Git 两块板用同一手法做“主行 + 回显行”。`Commit identity` 三行同时成立：说明讲覆盖全局配置，复选框却没勾，
Name/Email 仍是可编辑的实填值；`On launch` 右列三个 `.ref.branch` 徽章只读。

## 5. 控件与尺寸

`radio-pills` 容器内距 2、项间距 2、圆角 7、`--panel-2` 底 + 1px `--line-soft`；每项高 22、内距 0×10、10.5 `text2`，
选中项换 `--content` 白底 + 500 + 投影。下拉一律是原生 `select` 加内联样式：26 高、内距 0×8、圆角 6、
1px `--field-border`、字号 11；样式表另有 `.set-select`（26 高、右内距 24 留给 chevron），六块板都没用。
字段 `set-field` 26 高、内距 0×8、圆角 6，内层 input 11，本板写死宽 210。徽章 `.ref` 高 16、内距 0×5、圆角 4、
9.5/600、行高 16，分支变体用 `--branch-bg` 蓝底 + `--branch-fg` 蓝字。按钮 `btn small` 22 高、内距 0×8、字号 11、
圆角 5。复选全是原生控件，样式表只给 `.scan-row input` 设了
`accent-color`，画板没给尺寸。

## 6. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| Theme | Light / Dark / Auto | 只画 Auto，整窗仍是浅色 |
| Date format | Relative / Absolute / System / Custom pattern | 只画 Custom pattern，字段同时出现 |
| Use global config | 勾 / 不勾 | 只画不勾，Name/Email 已填 |
| File tree 两项 | 各自勾 / 不勾 | 都勾 |
| 两个下拉 | Launch behavior 二选一、Check for updates 三选一 | 各只画第一项 |

```
分区：General ⇄ Shortcuts ⇄ Agent ⇄ Git ⇄ Credentials ⇄ About gitui
      只替换右栏内容与树的 on 项，窗口 920×700 不重建
离开：Done → Workspace.dc.html
```

## 7. 画板没覆盖的组合（待补设计）

1. **深色**：`Theme` 有 Light/Dark 两项，六块板却全是浅色；深色下窗口底、边框、分段选中态都没画。
2. **Custom pattern 的依赖**：说明写 `Only used when Date format is Custom pattern.`，选其它值时字段是禁用、隐藏还是留值没画。
3. **输入校验与联动**：Name/Email 为空、邮箱非法、pattern 写错时错误放哪没有画；勾上 Use global config 之后两个字段是变灰还是回填 `~/.gitconfig` 也没画。
5. **窄窗口**：`min(920px,100%)` 会缩，但内容列、右侧控件、分段是否换行没有画。

## 8. 按钮与链接清单

本分区只有 2 个按钮；左侧导航是链接、分段与复选是选项，都不计入（六个树项都是 `<a>`，无障碍名即文案，板上也没有取消/应用按钮）。

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 左侧树 | General / Shortcuts / Agent / Git / Credentials / About gitui | 整行链接 28 高，图标 14 | 切分区，替换右栏 | 当前项 `--accent` 底 + 白字白图标，其余 hover `--hover` |
| 2 | Updates 段 Status 行右 | Check Now | `btn small` 22 高 | 立即检查新版本 | 板上只有常态，无进行中/禁用 |
| 3 | 页脚右端 | Done | `btn` 24 高、内距 0×10、字号 12 | 关设置回主窗口 | 常驻 |

## 9. 输入与选项

| 字段 | 形态 | 默认值（画板） | 说明 |
|---|---|---|---|
| Theme | 分段三项 | `Auto` | Light / Dark / Auto，Auto 跟系统 |
| Language | 原生 select 26 高 | `English` | 另一项 `简体中文`；段说明提示可能要重启 |
| Date format | 原生 select 26 高 | `Custom pattern` | Relative / Absolute / System / Custom pattern |
| Custom pattern | 字段 210 宽 | `MMM d, yyyy` | 右端固定预览 `Preview · Nov 14, 2026` |
| Name / Email | 字段 210 宽 ×2 | `Mia Chen` / `mia@gitui.dev` | 提交作者名与元数据 |
| Use global config | 原生复选 | 不勾 | 继承 `~/.gitconfig` 的 user.name / user.email |
| File tree 两项 | 原生复选 ×2 | 都勾 | 空目录并入最近父级；单子目录压成一条路径 |
| Launch behavior | 原生 select 26 高 | `Restore last session` | 另一项 `Show Welcome`；键帽写在 25 Shortcuts |
| Check for updates | 原生 select 26 高 | `Automatically at launch` | Manually / Never |
| Last session | 三个只读徽章 | `gitui` / `docs-site` / `atlas-web` | 非输入，随会话变化 |

`Launch behavior` 的实际启动规则：选择 `Restore last session` 且有已保存仓库时，按保存顺序恢复标签并恢复原活动仓库，不同时打开欢迎页；没有已保存仓库时只打开欢迎页。选择 `Show Welcome` 会跳过恢复，只打开欢迎页。关闭最后一个仓库后立即保存空会话，因此下一次启动仍进入欢迎页。关闭主窗口会保留当前会话；下次启动是否恢复仓库标签仍由此设置决定。

## 10. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 23b / 24 / 25 / 26 / 27 | 板内五个 `href`：`SettingsShortcuts.dc.html`、`SettingsAgent.dc.html`、`SettingsGit.dc.html`、`SettingsCredentials.dc.html`、`SettingsAbout.dc.html` |
| 04 Commits（Workspace） / 01 Welcome | `Done` 与标题栏齿轮双向；`Show Welcome` 落到欢迎页 |
| 10 Staging & commit | 提交身份与 File tree 两项在改动列表与文件树里生效 |
