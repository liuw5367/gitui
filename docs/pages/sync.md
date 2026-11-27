# 同步与认证 — 布局、状态与按钮

- **画板**：15 Sync & auth（有 Push 进行到 68% · 有逐行 git 输出 · 有 `Authenticated · GitHub`；无失败、无凭据缺失、无 Fetch/Pull 内容）
- **来源**：`designs/graft-git/project/Sync.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

本页是**主窗口加居中浮层**：底层是 04 的提交视图（`3f2a1c9` 的 split diff、`7 files` 检查器），整窗盖 `rgba(0,0,0,.28)`，正中一块 **800×520** 的 `Sync` 面板；面板内左表单右日志两列，页签把 Fetch / Pull / Push 三种操作收在同一处。

## 1. 骨架

```
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│ ● ● ●  [gitui ×] [docs-site ×] [atlas-web ×] [+]     ● Fetched 2m ago   [gear]                   │
│ [=] | Fetch Pull Push | Branch Stash   [search commits ⌘F] | History [Changes 12] | Agent | [=]  │
│                                                                                                  │
│                     ┌──────────────────────────────────────────────────────┐                     │
│                     │ Sync  [Fetch][Pull][Push]                  ×         │                     │
│                     ├──────────────────────────────────────────────────────┤                     │
│                     │ Push                      │ Activity                 │                     │
│                     │ main > origin/main        │ Pushing to origin/main   │                     │
│                     │ Remote origin · github…   │ [===       ] 68%         │                     │
│                     │ ☑ Push tags               │ Writing objects · 68%    │                     │
│                     │ ☐ Force with lease        │ $ Enumerating objects    │                     │
│                     │ ☐ Push all branches       │ $ Counting objects: 100% │                     │
│                     │ ● Authenticated · GitHub  │ $ Writing objects: 68%   │                     │
│                     │ [ Pushing… ] [ Cancel ]   │ $ Total 24 (delta 9)     │                     │
│                     │ 2 commits · 24 objects    │ You can close this sheet │                     │
│                     ├──────────────────────────────────────────────────────┤                     │
│                     │ 2 commits ahead · 0 behind          [Close]          │                     │
│                     └──────────────────────────────────────────────────────┘                     │
│                                                                                                  │
│ [branch] main   ● 2 ahead · 0 behind   12 changes  UTF-8  LF  TypeScript                         │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 遮罩 | `inset:0`，z 60 | 内距 24；`rgba(0,0,0,.28)` | 铺满整窗含标题栏，底层压暗但仍见 |
| 面板 | 窗口正中 | **800×520**；1px `line`、圆角 12；阴影 0 30 80 | 头 46 + 主体 428 + 脚 46 |
| 面板头 | 面板顶 | 高 46；内距 0 12 0 14；间距 10 | `Sync`、`Fetch / Pull / Push` 三段页签、`×` |
| 左列 | 主体左 | 宽 399（798 的一半）；内距 14；间距 10 | Push 目标、Remote 行、三个复选、认证、动作、摘要 |
| 右列 | 主体右 | 宽 399；内距 14；间距 9；左边框 1；`panel` 底 | 活动行、进度条、阶段说明、git 日志、关闭提示 |
| 面板脚 | 面板底 | 高 46；内距 0×14；间距 7 | `2 commits ahead · 0 behind`、`Close` |

窗口 1392×852 不变；面板左 295、上 165（各自居中）。图不按比例：面板画成 56 列，上下各空一行表示 165 的空白。

## 2. 面板头：标题、三段页签、关闭

头 46 高、内距左 14 右 12、间距 10、`panel` 底、下框 1：`Sync` 13/600、字距 -.01；紧接 `sync-tabs` 容器（左距 4、内距 2、1px `line-soft`、圆角 6、`panel-2` 底、项间距 1），三个 `.sync-tab` 各 22 高、内距 0×10、圆角 4、10.5 号 `text2`；`Push` 是当前项（`content` 底 + 阴影 + 600），`Fetch` / `Pull` 是链到 28 / 29 的 `<a>`。右端 `×` 24×24、圆角 5、16 号、`margin-left:auto`，`aria-label="Close sync"`，链到 04。

## 3. 左列：Push 表单

实现中的分支右键菜单会打开 Push 表单并预选源分支。源分支从本地分支中选择，远端从仓库已配置的远端中选择，目标从所选远端的分支中选择，也可选「创建远端分支」并输入名称。单分支推送按所选源和目标提交，在本地不切换分支。

自上而下 7 段，内距 14、间距 10：

1. `Push` 小标题，10.5、`text3`（末次覆盖去掉大写与字距）。
2. 目标行（间距 7、11 `text2`）：mono 10 的 `main` → `text3` 的箭头 → mono 10 的 `origin/main`，跟 `badge-soft` 的 `2 ahead`（18 高），末尾 `Upstream: origin/main`（10.5）。
3. `Remote origin · github.com/gitui/gitui`：10 号 `text3`，上框 1 `line-soft`、上内距 8；`Remote` 与 `origin` 之间是 `b`（`text`、500），右端 `Manage` 用同样的 10.5 `hint` 样式，板上是 `span`，不是链接。
4. 三个复选（`checkrow` 22 高、间距 7、11.5 `text2`、上下内距 8、上框 1）：`Push tags`（勾）、`Force with lease`（不勾）、`Push all branches`（不勾）。
5. 认证行被 `margin-top:auto` 推到底：`● Authenticated · GitHub`（9.5、`ok`、6×6 点）。
6. 动作行（间距 6）：`Pushing…`（`primary`、`flex:1` 居中，板上 `disabled` → `.45` 透明）与 `Cancel`（`btn` 26 高、10 号）。
7. 摘要 `2 commits · 24 objects · 68%`（10.5 `text3`）。

## 4. 右列：Activity 与 git 日志

右列底 `panel`、左边框 1、内距 14、间距 9，自上而下：

- `Activity` 小标题（10.5 `text3`）。
- 活动行（10 `text2`、间距 6）：10×10 的转圈（1.5px `line` 边框、顶边 `accent`、0.8s 线性旋转）+ `Pushing to origin/main`。
- 进度条 4 高、圆角 2、`panel-2` 底，内部 68% 宽、`accent` 填充。
- 阶段说明 `Writing objects · 68%`（10.5）。
- 日志块（`flex:1`、可滚）：mono 9/16、内距 7×8、1px `line-soft`、圆角 6、`pre-wrap`；六行，行首短语（`Enumerating objects`、`Counting objects`、`Compressing objects`、`Writing objects`、`Total`、`POST`）用 `text2` 500，其余 `text3`。
- 底部提示被 `margin-top:auto` 推到底：`You can close this sheet; the push continues in the background.`（10.5）。

## 5. 面板脚

脚 46 高、内距 0×14、间距 7、上框 1、`panel` 底：左 `2 commits ahead · 0 behind`（10.5 `text3`），右 `Close`（`a.btn`、26 高、10 号），链回 04。同一组数字在左列第 2 段（`2 ahead`）、左列末尾（`2 commits`）与这里各出现一次。

## 6. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 操作页签 | Fetch / Pull / Push | 只画 Push（另两页在 28 / 29） |
| 进行阶段 | 空闲 / 传输中 / 完成 / 失败 | 只画传输中 68% |
| 认证 | 已认证 / 需要凭据 / 失败 | 只画已认证（`ok` 点） |
| 主按钮 | 可点 / 禁用 | 只画禁用 `Pushing…` |
| 关闭后 | 前台等待 / 后台继续 | 只写文案，未画后台态 |

```
进入：04 工具条 Push → 本板（板上没有指向 Sync 的 href，入口只能靠按钮语义）
切换：Fetch ⇄ Pull ⇄ Push 只换左列，头与脚不动
离开：× / Close → 04；Cancel 只中止本次操作（结果未画）
```

### 当前实现行为

Fetch、Pull、Push 都在同一块 800×520 面板内显示各自选项、目标仓库、运行状态、输出与结果。Git 命令由后台子进程执行；右列持续显示实际 stdout/stderr，不解析不稳定的百分比，运行时用不定进度条。目标以稳定的 tab ID 和仓库路径固定，操作期间不能改选项或切换操作类型。

面板头的 ×、脚部 Close 都只隐藏面板，任务继续；运行中的 Cancel 会终止 Git 子进程。重开面板可继续查看任务，完成后显示成功、失败或取消状态。任务结束时按原 tab ID 与路径刷新仓库状态、分支、远端引用、工作区变更和未完成的 merge/rebase 操作；若目标标签已关闭，不会把结果写到其它仓库。

Push 遇到 non-fast-forward 时显示 Pull with Rebase、Pull with Merge 和 Cancel；选择一种 Pull 且成功后会重试原 Push。恢复链的 Activity 标题在 Pull 与重试 Push 时分别显示 `2/3`、`3/3`，进行期间为橙点，整条链成功后为绿点，失败为红点。单独的 Fetch、Pull、Push 不显示步数。Force with lease 只传 `--force-with-lease`，选中时提示会改写远端历史，不会自动使用 `--force`。取消 Pull 不会自动 abort 已开始的 merge/rebase；取消 Push 后远端可能已收到部分对象，需再 Fetch 确认。

## 7. 画板没覆盖的组合（待补设计）

1. **失败态**：认证失败、non-fast-forward 被拒、网络中断、hook 拒绝四种都没有画面；`Force with lease` 勾上后的二次确认也没画。
2. **凭据缺失**：`Authenticated · GitHub` 的反面（未登录、令牌过期、SSH key 缺失）没画，27 凭据页只有入口没有反向状态。
3. **`Manage`**：板上是 `span.hint`，既不是按钮也没有 href，去 27 还是就地展开没定。
4. **100% 之后**：成功提示、面板是否自动关、`Pushing…` 如何复原没画。
5. **后台继续**：关闭面板后重新打开时怎么接上进度、状态栏是否显示同步中（12 的状态栏只留 Agent）没画。
6. **无 upstream / 空仓库**：目标行写死 `origin/main` 与 `Upstream: origin/main`，缺 upstream 时换成什么没画。
7. **Fetch / Pull 的差异**：三页共用头脚，但左列字段（如 Pull 的 rebase/merge 选项）只能看 28 / 29，本板无法判断复用范围。
8. **日志边界**：`sync-log` 固定 `flex:1`，长输出的滚动、是否自动滚到底、能否复制都没画。

## 8. 按钮与链接清单

4 个按钮；页签、复选与底层按钮不计入，见本节末。

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 面板头右 | `×`（`aria-label=Close sync`） | 图标 24×24 | 关闭面板回 04 | 常驻 |
| 2 | 左列动作行 | `Pushing…` | `primary` 26 高 / 10 | 开始推送 | 板上 `disabled`（`.45`） |
| 3 | 左列动作行 | `Cancel` | `btn` 26 高 / 10 | 中止本次推送 | 空闲时是否隐藏未画 |
| 4 | 面板脚右 | `Close` | `a.btn` 26 高 | 关闭并回 04 | 常驻 |

- **页签**：`Fetch / Pull / Push`（`role=tablist`，项 22 高、10.5，当前项白底 600）；`Fetch`→`SyncFetch.dc.html`、`Pull`→`SyncPull.dc.html`，`Push` 是按钮。
- **复选**：`Push tags`、`Force with lease`、`Push all branches`，`checkrow` 22 高，只勾第一项。
- **整行/整块**：底层窗口的侧栏行、提交行、检查器文件行都在遮罩下，不可点，不计入。

## 9. 输入与选项

| 控件 | 形态 | 画板取值 |
|---|---|---|
| 操作 | 三段页签 | `Push` |
| Push tags | 复选 | 勾 |
| Force with lease | 复选 | 不勾 |
| Push all branches | 复选 | 不勾 |
| 目标 | 只读文本 | `main → origin/main`，`2 ahead` |

没有文本输入；唯一可变的是三个复选与页签。

## 10. 文案要点

- 同一事实写三遍：`2 ahead`、`2 commits · 24 objects · 68%`、`2 commits ahead · 0 behind`——分别是徽章、传输摘要、脚注，措辞不统一。
- `Pushing…` 用进行时并禁用，`Cancel` 保持可点：说明取消是允许的，但板上没画取消后的落点。
- 日志原样保留 git 输出（大小写、`done.`、`(chunked)`），没有本地化。
- `You can close this sheet; the push continues in the background.` 是本页唯一交代“关掉之后”的句子。

## 11. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 28 Sync · Fetch / 29 Sync · Pull | 头里的两个页签分别链到 `SyncFetch.dc.html` / `SyncPull.dc.html`；两板又都链回 `Sync.dc.html` |
| 04 Commits | `×` 与 `Close` 链回 `Workspace.dc.html`；底层就是该板的提交视图 |
| 10 Changes | 底层分段 `Changes 12` 链到 `Changes.dc.html` |
| 12 Agent panel | 底层工具条 `Agent` 链到 `Agent.dc.html` |
| 18 Files at commit | 底层检查器 `Files` 页签链到 `CommitFiles.dc.html` |
| 23 Settings / 27 凭据 | 底层标题栏齿轮；认证行的凭据归属板上没给链接 |
| 14 Merge conflict | 同为浮层，但遮罩 .28 且居中、面板 800×520，与 14 的 .34 顶对齐 1320×772 不同 |
