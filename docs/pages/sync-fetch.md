# 同步 · Fetch — 布局、状态与按钮

- **画板**：28 Sync · Fetch（`Fetch` 页签当前 · 主按钮可点 · 右列 `Remote refs` 三行 · 两个复选已勾；无进度条、无转圈、无进行中与失败态、无 Pull/Push 内容）
- **来源**：`designs/graft-git/project/SyncFetch.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

28 与 15 是同一块 `sync-sheet` 的两个分支态，差异只落在左右两列的内容：头（`Sync` + 三段页签 + `×`）、脚框、1fr 1fr 双列网格、遮罩与 **800×520** 全部不动；15 画 Push 传输中（68% 进度条 + 逐行 git 输出），28 画 Fetch 的空闲态（三个 fetch 选项 + 一份远程引用清单），所以本板没有进度条、没有转圈、主按钮可点。

外壳与 15 逐像素相同——两板除面板外没有一处差异：底层是 04 的提交视图，`sync-layer` 铺满整窗（`inset:0`、z 60、内距 24、`rgba(0,0,0,.28)`），窗口仍是 **1392×852**，面板 800×520 居中（左 295、上 165）。图不按比例：面板画成 68 列。

## 1. 骨架

```
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│ ● ● ●  [gitui ×] [docs-site ×] [atlas-web ×] [+]     ● Fetched 2m ago   [gear]                   │
│ [=] | Fetch Pull Push | Branch Stash   [search ⌘F] | History [Changes 12] | Agent | [=]          │
│                                                                                                  │
│               ┌──────────────────────────────────────────────────────────────────┐               │
│               │ Sync  [Fetch] [Pull] [Push]                                    × │               │
│               ├─────────────────────────────────┬────────────────────────────────┤               │
│               │ Fetch                           │ Remote refs                    │               │
│               │ origin · github.com/gitui/gitui │ origin/main · 2 new commits    │               │
│               │ Remote origin  Last fetched 2m… │ origin/main     3f2a1c9  2 new │               │
│               │ ☑ Prune deleted branches        │ origin/feature/agent  c72d9a4  │               │
│               │ ☑ Fetch tags                    │ origin/fix/wrapped   e19a3f2   │               │
│               │ ☐ Fetch all remotes             │                                │               │
│               │ ● Authenticated · GitHub        │                                │               │
│               │ [ Fetch ] [ Cancel ]            │                                │               │
│               │ Fetch updates remote-tracking   │                                │               │
│               │ refs only. The working tree is  │ No merge, no working tree      │               │
│               │ untouched.                      │ changes.                       │               │
│               ├─────────────────────────────────┴────────────────────────────────┤               │
│               │ origin · 2 new commits                                   [Close] │               │
│               └──────────────────────────────────────────────────────────────────┘               │
│                                                                                                  │
│ main   ● 2 ahead · 0 behind        12 changes   UTF-8   LF   TypeScript                          │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 遮罩 | `inset:0`，z 60 | 内距 24；`rgba(0,0,0,.28)` | 铺满整窗含标题栏；底层压暗但仍可见 |
| 面板 | 窗口正中 | **800×520**；1px `line`、圆角 12；阴影 0 30 80 | 头 46 + 主体 428 + 脚 46 |
| 面板头 | 面板顶 | 高 46；内距 0 12 0 14；间距 10 | `Sync`、`Fetch / Pull / Push`、`×` |
| 左列 | 主体左 | 宽 399（798 的一半）；内距 14；间距 10 | 小标题、目标、Remote 行、三个复选、认证、动作、说明 |
| 右列 | 主体右 | 宽 399；内距 14；间距 9；左边框 1；`panel` 底 | 小标题、活动行、日志块、底部提示 |
| 面板脚 | 面板底 | 高 46；内距 0×14；间距 7 | `origin · 2 new commits`、`Close` |

面板内宽 798 = 800 − 2 边框，两个 1fr 各 399。左列自上而下 7 段之和小于主体 428，空隙由认证行的 `margin-top:auto` 吸收。

## 2. 面板头：与 15 同构，只换当前项

头 46 高、`panel` 底、下框 1、内距 `0 12 0 14`、间距 10：`Sync` 13/600、字距 -.01 → `sync-tabs`（左距 4、内距 2、1px `line-soft`、圆角 6、`panel-2` 底、项间距 1）→ 三个 `sync-tab` 各 22 高、内距 0×10、圆角 4、10.5 号 `text2`（末次覆盖把 9.5 提到 10.5）→ 右端 `×` 24×24、圆角 5、16 号、`margin-left:auto`。本板 `Fetch` 是当前项（`content` 底 + 0 1px 2px 阴影 + 600，`button`），`Pull` / `Push` 是 `<a>`，分别链 29 / 15。`×` 带 `aria-label="Close sync"`，链回 04。面板自身的 `aria-label` 从 15 的 `Sync with origin` 换成 `Fetch from origin`。

## 3. 左列：Fetch 表单（7 段，内距 14、间距 10）

1. 小标题 `Fetch`：10.5 `text3`。样式表初值是 8.5/600/.06em/大写，末次声明把字号提到 10.5 并去掉字距与大写，`600` 保留。
2. 目标行（间距 7、11 `text2`）：`origin`（mono 10、`text`）→ `·`（内联 `text3`）→ `github.com/gitui/gitui`（hint）。15 的同一行是 `main → origin/main` 加 `badge-soft` 的 `2 ahead` 与 `Upstream: origin/main`；本板没有箭头、没有徽章、没有 Upstream。
3. `Remote origin` 行：10 `text3`、上框 1 `line-soft`、上内距 8、间距 6；`Remote` 常规，`origin` 是 `b`（`text` 500）；右端 `Last fetched 2m ago`。15 写 `Remote origin · github.com/gitui/gitui` + 右端 `Manage`。
4. 三个复选：上框 1、上下内距 8、项间距 4；`checkrow` 22 高、间距 7、11.5 `text2`，方框 13：`Prune deleted branches`（勾）、`Fetch tags`（勾）、`Fetch all remotes`（不勾）。
5. 认证行被 `margin-top:auto` 推到底：`● Authenticated · GitHub`，9.5 `ok`、6×6 圆点、间距 6 —— 与 15 一字不差。
6. 动作行（间距 6）：`Fetch`（`primary`、`flex:1` 居中，**可点**）+ `Cancel`（`btn`）。两枚按钮按末次 `.btn` 为 **24 高 / 12 号**；15 的主按钮是禁用的 `Pushing…`，同样的 `.btn` 尺寸规则在 15 也成立（面板脚另有 `.sync-foot .btn` 的 26 高 / 10 号，选择器更具体，只作用于 `Close`）。
7. 末尾说明 10.5 `text3`：`Fetch updates remote-tracking refs only. The working tree is untouched.`；15 这里放的是三合一摘要 `2 commits · 24 objects · 68%`。

## 4. 右列：Remote refs

底 `panel`、左边框 1、内距 14、间距 9：

- 小标题 `Remote refs`（10.5 `text3`）。
- 活动行 `origin/main · 2 new commits`（10 `text2`、间距 6）。**没有转圈**——15 的活动行前有 10×10 的 spinner。
- 日志块 `sync-log`（`flex:1`、可滚）：mono 9/16、`content` 底、1px `line-soft`、圆角 6、内距 7×8、`pre-wrap`。三行，`<b>` 只包引用名（`text2` 500），其余 `text3`；列与列全靠空格对齐，`2 new` 只挂在第一行。ASCII 里省掉了日志块自身的边框。
- 底部提示 `margin-top:auto`：`No merge, no working tree changes.`（10.5）。
- **没有** 15 的 4 高进度条、`68%` 填充与 `Writing objects · 68%` 阶段说明：本板整列只表达「拉到什么」，不表达「进行到哪」。

## 5. 面板脚

46 高、内距 0×14、间距 7、上框 1、`panel` 底：左 `origin · 2 new commits`（10.5 `text3`），右 `Close`（`a.btn`、26 高 / 10 号、链 04）。15 的左端是 `2 commits ahead · 0 behind`；同一个 `2` 在本板出现在右列活动行、日志首行与脚三处。

## 6. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 操作页签 | Fetch / Pull / Push | 只画 `Fetch` 当前 |
| 进行阶段 | 空闲 / 传输中 / 完成 / 失败 | 只画空闲（按钮可点、无进度） |
| 认证 | 已认证 / 需凭据 / 失败 | 只画已认证（`ok` 点） |
| 复选项 | Prune / Fetch tags / Fetch all remotes | 前两项勾、第三项不勾 |
| 关闭后 | 前台等待 / 后台继续 | 未写（15 才有那句后台提示） |

```
进入：15 的 Fetch 页签 → 本板；工具条 Fetch 按钮在遮罩下，板上无 href
切换：Fetch ⇄ Pull ⇄ Push 只换左右两列，头与脚不动
离开：× / Close → 04；Cancel 只中止本次 fetch（结果未画）
```

### 当前实现行为

Fetch 的 Prune、Fetch tags 与 Fetch all remotes 选项分别映射到 Git 参数；运行时显示真实 Git 输出，完成后刷新目标仓库的远端引用与同步状态。Fetch all remotes 启用时按 Git 的 `--all` 执行，不影响工作区文件。

运行中的 Cancel 结束 Git 子进程并显示已捕获输出；已经完成的远端更新不会回滚。× 或 Close 只隐藏面板，Fetch 会继续；重新打开同步面板可以查看进度与最终结果。认证失败、远端拒绝和网络错误显示本地化摘要，详细 Git 输出保留在右侧日志。

## 7. 画板没覆盖的组合（待补设计）

1. **进行中**：没有 `Fetching…` 禁用态、没有进度条、没有转圈，也没有阶段说明；fetch 传输中到底长什么样没画。
2. **完成与失败**：`2 new` 之后侧栏、状态栏、`Last fetched` 是否自动刷新没画；网络中断、认证失败、远程不存在三种失败也没有画面。
3. **`Fetch all remotes` 勾上后**：右列标题是单 remote 语义（`Remote refs`），多个 remote 时是否分组、`origin/main · 2 new commits` 的活动行怎么写没画。
4. **`Prune deleted branches` 的后果**：会删掉哪些远程跟踪引用，没有预览也没有二次确认。
5. **`0 new`**：没有新提交时活动行、日志块（会是空的）、脚注怎么写没画。
6. **凭据缺失**：`Authenticated · GitHub` 只有正面，未登录 / 令牌过期 / SSH key 缺失时这一行换成什么没画。
7. **时间戳来源**：`Last fetched 2m ago` 与标题栏 `Fetched 2m ago` 说的是同一件事，谁负责刷新、多久算过期没画。
8. **日志边界**：`sync-log` 固定 `flex:1` 可滚，长输出是否自动滚到底、能否复制没画。

## 8. 按钮与链接清单

4 个按钮；页签与复选不计入，见本节末。

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 面板头右 | `×`（`aria-label=Close sync`） | 图标 24×24 | 关闭面板回 04 | 常驻 |
| 2 | 左列动作行 | `Fetch` | `primary` 24 高 / 12，`flex:1` | 执行 fetch | 板上可点；无进行中/禁用态 |
| 3 | 左列动作行 | `Cancel` | `btn` 24 高 / 12 | 中止本次 fetch | 空闲时是否隐藏未画 |
| 4 | 面板脚右 | `Close` | `a.btn` 26 高 / 10 | 关闭并回 04 | 常驻 |

- **页签**：`Fetch`（`button`，`on`）/ `Pull`（`a` → 29）/ `Push`（`a` → 15），`role=tablist`，项 22 高、10.5。
- **复选**：`Prune deleted branches`、`Fetch tags`、`Fetch all remotes`，`checkrow` 22 高，只勾前两项。
- **整行/整块**：底层侧栏行、提交行、检查器文件行都在遮罩下，不可点，不计入。

## 9. 输入与选项

| 控件 | 形态 | 画板取值 |
|---|---|---|
| 操作 | 三段页签 | `Fetch` |
| Prune deleted branches | 复选 | 勾 |
| Fetch tags | 复选 | 勾 |
| Fetch all remotes | 复选 | 不勾 |
| 目标 | 只读文本 | `origin · github.com/gitui/gitui` |

没有文本输入；唯一可变的是页签与三个复选。

## 10. 文案要点

- 同一个 `2` 出现三处、两种写法：`2 new commits`（活动行）、`2 new`（日志首行）、`2 new commits`（脚）；日志后两行干脆不带计数。
- `Fetch` 在面板里出现三次（页签、左列小标题、主按钮），三处同一个词、三种角色。
- 语义边界写两遍：左列末句说只动远程跟踪引用、不动工作区，右列末句说不 merge、不动工作树。15 是把数字摘要放左列、把「关掉也继续」放右列。
- `Last fetched 2m ago` 与标题栏 `Fetched 2m ago` 措辞不同但指同一件事；日志保留 git 原样输出风格（短 SHA、大小写），没有本地化。

## 11. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 15 Sync & auth / 29 Sync · Pull | 头里 `Push` 链 `Sync.dc.html`、`Pull` 链 `SyncPull.dc.html`；两板都链回本板 |
| 04 Commits | `×`、`Close` 与底层返回条 `Commits` 三处都链 `Workspace.dc.html`（板内 3 个 href） |
| 10 Changes / 12 Agent / 18 Files at commit | 底层工具条 `Changes 12`、`Agent`，检查器 `Files` 页签 |
| 23 Settings / 01 Welcome | 底层标题栏齿轮、页签 `+`；认证行的凭据归属板上没给链接 |
| 14 Merge conflict | 同为浮层，但 14 是 .34 遮罩、顶对齐 1320×772；本板 .28 且居中 800×520 |
