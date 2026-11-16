# Agent 权限 — 布局、状态与按钮

- **画板**：13 Agent permissions（有 This repository · 勾选 4 项 · Automation 在 `Commit & Push` · 评审窗口 8 秒；无 All repositories、无保存结果、无权限联动）
- **来源**：`designs/graft-git/project/AgentPermissions.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

本页是**主窗口加一层浮层**：底层仍是 12 的 Agent 面板，遮罩从窗口顶 44 铺到底（`rgba(0,0,0,.20)`），中间挂一块 620 宽的下拉面板，上沿也在 44——压在工具条顶部 4px 处，上边不留圆角，向下 12 圆角。

## 1. 骨架

```
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│ ● ● ●  [gitui ×] [docs-site ×] [atlas-web ×] [+]     ● Fetched 2m ago   [gear]                   │
│                  ┌────────────────────────────────────────────────────────────┐                  │
│                  │ [sparkle] Agent Permissions            ×                   │                  │
│                  │ gitui · this repository                                    │                  │
│                  ├────────────────────────────────────────────────────────────┤                  │
│                  │ Scope        [This repository][All repositories]           │                  │
│                  │ The agent keeps these permissions per repository.          │                  │
│                  │ Repository permissions     Changes apply after Save        │                  │
│                  │ Read repository                              [☑]           │                  │
│                  │ Required to read refs, commits, and diffs.                 │                  │
│                  │ Stage changes                                [☑]           │                  │
│                  │ Create commits                               [☑]           │                  │
│                  │ Push to remote                               [☑]           │                  │
│                  │ Force push                                   [☐]           │                  │
│                  │ Discard working-tree changes                 [☐]           │                  │
│                  │ Automation     [Off][Commit][Commit & Push]                │                  │
│                  │ Review window 8 seconds        [-]      [+]                │                  │
│                  │ ☑ Require approval for protected branches                  │                  │
│                  │ [lock] Push is a write to the remote. Every action…        │                  │
│                  ├────────────────────────────────────────────────────────────┤                  │
│                  │ [Reset to Defaults]    [Cancel] [Save Changes]             │                  │
│                  └────────────────────────────────────────────────────────────┘                  │
│                                                                                                  │
│ [branch] main  ● Agent · awaiting approval   3 staged · 9 unstaged  UTF-8  LF  TypeScript        │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 标题栏 | 顶部通栏 | 高 40；内距 0×13 | 三个仓库页签、右端状态；**完全不被遮罩覆盖** |
| 工具条 | 标题栏下 | 高 44；内距 0×10 | 中段被面板压住，只留顶部约 4px 与左右两端可见 |
| 遮罩 | 窗口内 left/right 0、top 44、bottom 0 | z 40；`rgba(0,0,0,.20)` | 底层 Agent 面板压暗但仍见，遮罩本身没有内容 |
| 面板 | 水平居中，上沿 44 | 宽 620；高按内容（约 600）；1px `line-strong`、上边无描边、下圆角 12；z 50 | 头 54 + 体 + 脚 52；阴影 0 30 90 / 0 4 16 |
| 面板头 | 面板顶 | 高 54；内距 0×18；间距 9 | 26×26 图标、标题与副标、右端 `×` |
| 面板体 | 头下 | 内距上下 16、左右 20 | Scope、六个权限行、Automation、说明条 |
| 面板脚 | 面板底 | 高 52；内距 0×18；间距 8 | `Reset to Defaults`、`Cancel`、`Save Changes` |

窗口尺寸不变（1392×852）。图上不按比例：面板画成 62 列宽，实际占窗口宽度的 620/1392；标题栏以下只剩面板与它的两侧余量。

## 2. 面板头与面板脚

头 54 高、内距 0×18、间距 9、`titlebar` 底、下框 1：左端 26×26 圆角 7 的反白图标，中间标题 `Agent Permissions`（13/600）与副标 `gitui · this repository`（9.5 `text3`、上距 1），右端 `×` 24×24（内联 15 号、`aria-label="Close permissions"`，链到 12）。

脚 52 高、内距 0×18、间距 8、上框 1、`panel` 底：左 `Reset to Defaults`（ghost small），右侧 `Cancel` 与 `Save Changes`——两个都是 `a.btn`（24 高、12 号、圆角 6），后者是 `primary`（`#007AFF` 底白字，无描边），都链回 12。

## 3. Scope 与六个权限行

Scope 行左对齐一组：`Scope`（11/600，`text2`）+ `The agent keeps these permissions per repository.`（10.5，`text3`）；右端 `seg`（容器内距 2、圆角 7、`panel-2` 底 + 1px `line-soft`），项 23 高、内距 0×10、12/500，`This repository` 是当前项（`content` 底 + 阴影），`All repositories` 未选。

分隔段上距 15、上内距 13、1px `line-soft`。段首行：`Repository permissions`（11/600）与右端 `Changes apply after Save`（10.5），下距 2。

六个权限行都是 `<label>`，各 36 高、左侧文字与右侧原生 checkbox 间距 12、前五行带 1px `line-soft` 下框：

| 行 | 标题 | 说明 | 勾选 |
|---|---|---|---|
| 1 | Read repository | Required to read refs, commits, and diffs. | 勾 + `disabled`（`aria-label=Read repository is required`） |
| 2 | Stage changes | Add or remove files in the index. | 勾 |
| 3 | Create commits | Commit staged changes on the current branch. | 勾 |
| 4 | Push to remote | Send commits to the configured upstream. | 勾 |
| 5 | Force push | Overwrite remote history. `Not recommended.` | 不勾 |
| 6 | Discard working-tree changes | Permanently delete uncommitted changes. | 不勾 |

标题 11.5 `text`，说明 9.5 `text3`（上距 1）；第 5 行的 `Not recommended.` 是 `warn` 色，与说明同号。

## 4. Automation 与评审窗口

上距 13、上内距 13、上框 1。首行：`Automation`（11/600）+ `Run approved plans without asking each time.`（10.5）在左，右端 `seg tiny` 三档 `Off` / `Commit` / `Commit & Push`（项 21 高、内距 0×7、10.5），第三档是当前项。

下一行 32 高：`Review window`（11.5 `text`）+ `Time to cancel before the agent runs.`（10.5），弹性空隙，然后 `−`（`btn small` 22 高）、`8 seconds`（11、显示区宽 58、居中）、`+`——两个按钮各带无障碍名 `Decrease/Increase review window`。

再一行 `checkrow`（内联 28 高、间距 7、11.5 `text2`）：`Require approval for protected branches`，已勾选。

## 5. 说明条

上距 10、内距 8×10、1px `line`、圆角 7、`panel` 底、9.5/1.45：左端 12 的锁图标（`warn`），正文首句 `Push is a write to the remote.` 用 600 `text`，其余 `The agent will not force-push or discard working-tree changes. Every action is recorded in the agent log.` 用 `text2`。

## 6. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| Scope | This repository / All repositories | 只画前者 |
| 六个权限 | 勾 / 不勾 | 勾 4（其中 Read 另带 `disabled`）、不勾 2 |
| Automation | Off / Commit / Commit & Push | 只画第三档 |
| 评审窗口 | 任意秒数 | 只画 `8 seconds` |
| 受保护分支审批 | 勾 / 不勾 | 只画勾 |
| 保存结果 | 成功 / 失败 | 都没画（三个出口只做跳转） |

```
进入：12 面板头 Ask first ▾ → 本板
保存：Save Changes / Cancel / × 都回 12（三个都 href="Agent.dc.html"）
离开：遮罩没有点击关闭；Esc 未画
```

## 7. 画板没覆盖的组合（待补设计）

1. **All repositories 档**：副标写死 `gitui · this repository`，切到全仓库后标题与六个权限是否同一套没画。
2. **权限依赖**：Force push 依赖 Push、Create commits 依赖 Stage、Push 依赖 Create commits；板上没有任何联动、置灰或提示。
3. **保存结果**：三个出口都只是跳回 12；成功、失败、校验错误（例如 Automation 与勾选项冲突）放哪没画。
4. **与 12 的档位对应**：Automation 三档、composer 的 `Auto` 徽章、面板头 `Ask first` chip、`read + write + push` 四处措辞没有对照。
5. **评审窗口**：取值范围与步长没画（只有 `−`/`+` 和 `8 seconds`）；`Off` 档下这一行是否隐藏也没画。
6. **面板高度**：内容约 600 高且没有滚动区；窗口更矮时的处理、或字号放大后的换行都没画。
7. **遮罩行为**：点遮罩是否关闭、Esc 是否等同 Cancel 没画；面板压在工具条上 4px，工具条高度变化时是否跟随没画。
8. **默认值**：`Reset to Defaults` 会把六项与 Automation 复位成什么组合，板上没有。

## 8. 按钮与链接清单

6 个按钮；分段、复选与整行可点的权限行不计入，见本节末。

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 面板头右 | `×`（`aria-label=Close permissions`） | 图标 24×24 | 关闭并回 12 | 常驻 |
| 2 | 面板脚左 | `Reset to Defaults` | ghost small 22 高 | 复位为默认权限 | 常驻 |
| 3 | 评审窗口行 | `−`（`aria-label=Decrease review window`） | small 22 高 | 减少秒数 | 1 秒时是否禁用未画 |
| 4 | 评审窗口行 | `+`（`aria-label=Increase review window`） | small 22 高 | 增加秒数 | 上限未画 |
| 5 | 面板脚右 | `Cancel` | `a.btn` 24 高 | 放弃改动并回 12 | 常驻 |
| 6 | 面板脚右 | `Save Changes` | `a.btn.primary` 24 高 | 保存并回 12 | 无禁用态、无保存中 |

- **分段**：`Scope`（`role=group`，项 23 高、12 号）与 `Automation`（`role=group`，项 21 高、10.5 号）；当前项都是白底 + 阴影。
- **复选**：六个权限行各一个原生 checkbox，第七个是 `Require approval for protected branches`；`Read repository` 带 `disabled`。
- **整行可点**：六个权限行是 `<label>`，点整行即切换；分段与复选不混进上面的按钮计数。

## 9. 输入与选项

| 控件 | 形态 | 画板取值 |
|---|---|---|
| Scope | 两选一分段 | `This repository` |
| 六个权限 | 复选 | 勾 4 / 不勾 2，Read 置灰 |
| Automation | 三选一分段 | `Commit & Push` |
| Review window | `−` / 数值 / `+` | `8 seconds` |
| Require approval for protected branches | 复选 | 勾选 |

没有文本输入；面板里唯一会变的数字是评审窗口秒数。

## 10. 文案要点

- 副标 `gitui · this repository` 与 Scope 的 `This repository` 重复限定范围——副标是作用域，Scope 是切换器，两者措辞相同容易读成两件事。
- `Changes apply after Save` 明确不是即时生效；`Not recommended.` 是整页唯一带态度的说明，且只挂在 Force push 上。
- 说明条三段式：先定性（`Push is a write to the remote.`），再给保证（不强推、不丢工作区改动），最后给去处（`agent log`）。
- `Review window` 的副句 `Time to cancel before the agent runs.` 说明它是反悔时间，不是超时。

## 11. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 12 Agent panel | `Ask first ▾` 从 12 进来；`×`、`Cancel`、`Save Changes` 三处 `href="Agent.dc.html"` 回去 |
| 10 Changes | 底层 Agent 面板的页签仍指向 `Changes.dc.html`，关闭后落在这里 |
| 04 Commits | 底层窗口是三栏 + 状态栏，全部被遮罩压暗 |
| 23b Settings · Agent | 12 的脚注写 `Settings › Agent`；本板是同一组权限的浮层版，两处是否同源板上没说 |
