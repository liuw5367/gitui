# 关于 gitui — 布局、状态与按钮

- **画板**：26 Settings · About（设置窗口第 6 项 About gitui；有标识块、Links 四行、Credits 一行；无更新按钮、无第三方许可清单、无构建渠道）
- **来源**：`designs/graft-git/project/SettingsAbout.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

本分区是设置窗口的第 6 项（`About` 组里唯一一项）。外壳与 23 General 同一套：`.settings .window`
`width:min(920px,100%)`、`height:min(700px,calc(100vh - 48px))`，画布 1440×900、`.desktop` 留白 24，因此窗口
**920×700**。内容区只两段加一个标识块，是六个分区里唯一不需要滚动的一块。

## 1. 骨架

```
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ ● ● ●                                      Settings                   gitui 0.9.2            │
├─────────────────────────┬────────────────────────────────────────────────────────────────────┤
│                         │ About gitui                                                        │
│ ▣ gitui                 │ A native Git client for macOS.                                     │
│   Version 0.9.2         │                                                                    │
│                         │ ▣ gitui                                                            │
│ Application             │ Version 0.9.2 (412) · Apple Silicon · Up to date                   │
│  [gear]   General       │ Links                                                              │
│  [keys]   Shortcuts     │Website                  gitui.dev                 Visit            │
│  [branch] Agent         │Source code              github.com/gitui/gitui    GitHub           │
│ Git                     │Report an issue          Open a GitHub issue with diagnostics.      │
│  [branch] Git           │                         Issues                                     │
│  [lock]   Credentials   │Licenses                 Apache-2.0 · third-party notices   View    │
│ About                   │ Credits                                                            │
│* [i]      About gitui   │Built with Native Shell  AppKit views on macOS · Rust + TSX.        │
│                         │                         © 2026 gitui contributors                  │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ ● Changes are saved automatically                                                   [ Done ] │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 标题栏 | 顶部通栏 | 高 38；内距 0×13 | 窗口灯 12×3、居中 `Settings` 12.5/600、右端 `gitui 0.9.2` 11 |
| 左侧分区树 | 标题栏下左 | 宽 208；内距 12×8；行高 28 | 六项三组，`About gitui` 是当前项 |
| 内容区 | 标题栏下右 | 宽 710；内距 24×28 → 内容列 654、可视高 568 | 标题、副标题、标识块、Links 与 Credits 两段 |
| 页脚 | 底部通栏 | 高 44；内距 0×16 | 6 状态点、自动保存说明、右端 `Done` |

正文 616 = 700 − 2 − 38 − 44，内容列 654 = 710 − 28×2，可视高 568 = 616 − 24×2。标识块 54（上距 18 + 36 图标）、
Links 段约 260、Credits 段约 115，加上标题与副标题累计约 470，短于 568，右栏不滚动。

## 2. 外壳与左侧分区树

左树六项三组与其它五块板逐字相同：`Application` 下 General（齿轮）、Shortcuts（键帽）、Agent（分支）；`Git` 下
Git（分支）、Credentials（锁）；`About` 下 About gitui（信息）。本板当前项是 `About gitui`，`.on` 取末次声明
`background:var(--accent)` + 白字白图标 + 500，图标是信息圈。行 28 高、内距 0×8、圆角 6、字号 13、图标盒 16 宽；
组头 `.sg` 24 高、10.5/600 `text3`、sentence case。

注意树项文案与页面标题：树里写 `About gitui`，页面标题也是 `About gitui`（17/700），与 27 那块“树里写 Credentials、
标题写 Credentials & tools”的不一致情况不同，本板两处相同。

## 3. 标识块

标题下 18 是标识行：内联 `display:flex;align-items:center;gap:12px`，左边 `appmark` 36×36、圆角 9、去投影、
图标 18 白色（`--accent-2` 蓝底）；右边两行——应用名 `gitui` 20/700、字距 −.025em，下一行 `.hint` 10.5 `text3`、
上距 2：`Version 0.9.2 (412) · Apple Silicon · Up to date`。

这三段用 `·` 连成一句：版本号带构建号 412、架构 `Apple Silicon`、更新状态 `Up to date`。`Up to date` 在 23 General
的 Updates 段里对应 `gitui 0.9.2 (412) is up to date.`，两处版本号一致，但本板没有 `Check Now`，检查入口只在 General。

## 4. Links 段

Links 是第一个 `.settings-sec`，因此吃到 `:first-of-type` 的 `border-top:0;margin-top:14px;padding-top:0`；段标题
12.5/600、下距 2。这一段**没有段说明**（`.settings-sec-sub` 缺失），所以标题下只有 2 的间距就接第一行，
与其它分区的段头留白不一致。

四行 `.set-row`（间距 14、内距 `11px 0`、下边框 `--line-soft`、末行无边框）：

| 行 | 左列（标签 + 说明） | 右列 |
|---|---|---|
| 1 | `Website` + mono `gitui.dev` | `.hint` `Visit` |
| 2 | `Source code` + mono `github.com/gitui/gitui` | `.hint` `GitHub` |
| 3 | `Report an issue` + `Open a GitHub issue with diagnostics.` | `.hint` `Issues` |
| 4 | `Licenses` + `Apache-2.0 · third-party notices` | `.hint` `View` |

第 1、2 行的说明是 mono（`gitui.dev`、`github.com/gitui/gitui`），第 3、4 行是 10.5 `text3` 的普通说明。右列四个词
都是 `<span class="hint">`，在板上没有 `href`、没有下划线、没有箭头，视觉上与只读回显相同。

## 5. Credits 段与行度量

Credits 段上距 20、上内距 16、上边框 1px `--line`，段标题 `Credits` 12.5/600、下距 2，同样没有段说明。唯一一行：
左列 `Built with Native Shell`（12.5/500）+ 说明 `AppKit views on macOS · Rust + TSX.`（10.5 `text3`、上距 2）；
右列 `.hint` `© 2026 gitui contributors`。这一行是段内末行，因此没有下边框。

行网格与其它分区一致：`flex` 两列、间距 14、标签 12.5/500、说明 10.5/1.4；右列贴右、`gap:6`。按行高累加，整块
内容约 470，比 568 的可视高少约 100，因此内容列底部留白、页脚仍在 44 的位置。

## 6. 状态与切换

本分区没有开关、没有输入，也没有分段，只有一条左侧分区轴与一条版本信息：

| 轴 | 取值 | 画板 |
|---|---|---|
| 左侧分区 | 六项 | 只画 About gitui 选中 |
| 版本 | 有更新 / 已是最新 | 只画 `Up to date` |
| 架构 | Apple Silicon / Intel | 只画 `Apple Silicon` |
| 构建号 | 412 | 只画 412 |

```
分区：General ⇄ Shortcuts ⇄ Agent ⇄ Git ⇄ Credentials ⇄ About gitui（只替换右栏）
离开：Done → Workspace.dc.html
```

## 7. 画板没覆盖的组合（待补设计）

1. **有更新**：`Up to date` 换成“有新版本”时这行长什么样、是否就地给按钮，没有画；本板也没有检查动作。
2. **链接行为**：`Visit` / `GitHub` / `Issues` / `View` 在板上是纯文本，点击后是打开浏览器、应用内打开还是不可点，没有画。
3. **许可清单**：`Licenses` 写 `Apache-2.0 · third-party notices`，但清单页、`View` 的去处没有画。
4. **诊断**：`Report an issue` 说 `with diagnostics`，诊断内容、复制入口与隐私提示都没有画。
5. **构建信息**：只有架构与构建号，渠道（稳定版/测试版）、提交哈希、构建时间都没有画；Intel 机器上的那一行也没有画。

## 8. 按钮与链接清单

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 页脚右端 | Done | `btn` 24 高、内距 0×10、字号 12 | 关设置回主窗口 | 常驻 |
| 2 | 左侧树 | 六项 | 整行链接 28 高，图标 14 | 切分区，替换右栏 | 当前项 `--accent` 底 + 白字白图标 |
| 3 | Links 四行右端 | Visit / GitHub / Issues / View | `.hint` 纯文本 10.5 | 板上没有链接目标 | 板上不随状态变化 |

- `Visit`、`GitHub`、`Issues`、`View` 不是按钮：它们是 `.hint` 的 `<span>`，没有 `href`、`aria-label` 与 hover 态。
  按可点元素计，本分区只有 `Done` 与六项导航。

## 9. 输入与选项

本分区没有输入控件，也没有可切换的选项。可读的静态字段有七个：

| 字段 | 形态 | 取值（画板） | 说明 |
|---|---|---|---|
| 应用名 | 20/700 文本 | `gitui` | 与标题栏、左树同字 |
| 版本行 | 10.5 `text3` 文本 | `Version 0.9.2 (412) · Apple Silicon · Up to date` | 三段用 `·` 分隔 |
| Website | 12.5 标签 + mono 说明 | `gitui.dev` | 右列 `Visit` 是纯文本 |
| Source code | 12.5 标签 + mono 说明 | `github.com/gitui/gitui` | 右列 `GitHub` 是纯文本 |
| Report an issue | 12.5 标签 + 说明 | `Open a GitHub issue with diagnostics.` | 右列 `Issues` 是纯文本 |
| Licenses | 12.5 标签 + 说明 | `Apache-2.0 · third-party notices` | 右列 `View` 是纯文本 |
| Credits | 12.5 标签 + 说明 | `Built with Native Shell` / `AppKit views on macOS · Rust + TSX.` | 右列 `© 2026 gitui contributors` |

## 10. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 23 Settings | 版本号与更新状态来自 `Updates` 段；`Done` 与其它五板同指主窗口 |
| 23b / 24 / 25 / 27 | 板内五个 `href`：`Settings.dc.html`、`SettingsShortcuts.dc.html`、`SettingsAgent.dc.html`、`SettingsGit.dc.html`、`SettingsCredentials.dc.html` |
| 04 Commits（Workspace） | `Done` 指回主窗口，主窗口标题栏齿轮再指回 23 |
| 01 Welcome | 欢迎页底栏也显示 `gitui 0.9.2 (412)`，与本板标识块同源 |
