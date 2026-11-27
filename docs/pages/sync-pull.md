# 同步 · Pull — 布局、状态与按钮

- **画板**：29 Sync · Pull（`Pull` 页签当前 · 三段 `Merge / Rebase / FF only` 且 `Rebase` 当前 · 两个复选 · 右列 `Incoming` 两条提交；无进行中、无冲突画面、无 Fetch/Push 内容）
- **来源**：`designs/graft-git/project/SyncPull.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

29 与 15 同骨架、与 28 左右对位：28 把远程引用拉进本地（只读远程），29 把远程提交合进当前分支（会改工作树与本分支），所以 29 比 15、28 各多一整块「整合方式」分段，左列从 7 段变 8 段；右列则从 15 的逐行 git 输出、28 的引用清单，换成「提交 SHA + 提交标题」两行。

外壳与 15、28 逐像素相同：底层是 04 的提交视图，`sync-layer` 铺满整窗（`inset:0`、z 60、内距 24、`rgba(0,0,0,.28)`），窗口 **1392×852** 不变，面板 **800×520** 居中（左 295、上 165）。图不按比例：面板画成 68 列。

## 1. 骨架

```
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│ ● ● ●  [gitui ×] [docs-site ×] [atlas-web ×] [+]     ● Fetched 2m ago   [gear]                   │
│ [=] | Fetch Pull Push | Branch Stash   [search ⌘F] | History [Changes 12] | Agent | [=]          │
│                                                                                                  │
│               ┌──────────────────────────────────────────────────────────────────┐               │
│               │ Sync  [Fetch] [Pull] [Push]                                    × │               │
│               ├─────────────────────────────────┬────────────────────────────────┤               │
│               │ Pull                            │ Incoming                       │               │
│               │ main < origin/main    2 behind  │ 2 commits from origin/main     │               │
│               │ Integrate 2 commits  fast-forw… │ 3f2a1c9  Fix diff hunk ali…    │               │
│               │ [Merge] [Rebase] [FF only]      │ a37c6b1  Add split/unified     │               │
│               │ ☑ Autostash local changes       │                                │               │
│               │ ☐ Rebase all local commits      │                                │               │
│               │ ● Authenticated · GitHub        │                                │               │
│               │ [ Pull ] [ Cancel ]             │                                │               │
│               │ Rebase replays your 2 local     │ Conflicts open the merge       │               │
│               │ commits on top of origin/main.  │ editor. Cancel leaves the      │               │
│               │                                 │ branch untouched.              │               │
│               ├─────────────────────────────────┴────────────────────────────────┤               │
│               │ 2 ahead · 2 behind                                       [Close] │               │
│               └──────────────────────────────────────────────────────────────────┘               │
│                                                                                                  │
│ main   ● 2 ahead · 0 behind        12 changes   UTF-8   LF   TypeScript                          │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 遮罩 | `inset:0`，z 60 | 内距 24；`rgba(0,0,0,.28)` | 与 15 / 28 相同 |
| 面板 | 窗口正中 | **800×520**；1px `line`、圆角 12 | 头 46 + 主体 428 + 脚 46 |
| 面板头 | 面板顶 | 高 46；内距 0 12 0 14；间距 10 | `Sync`、三段页签（`Pull` 当前）、`×` |
| 左列 | 主体左 | 宽 399；内距 14；间距 10 | 小标题、目标、Integrate 行、模式分段、两个复选、认证、动作、说明 |
| 右列 | 主体右 | 宽 399；内距 14；间距 9；左边框 1 | 小标题、活动行、两行提交、底部提示 |
| 面板脚 | 面板底 | 高 46；内距 0×14；间距 7 | `2 ahead · 2 behind`、`Close` |

左列 8 段比 15 多一段 29 高的 `seg`（内距 2 + 边框 1 + 项 23），仍能被 `margin-top:auto` 的认证行吸收在 428 内。

## 2. 面板头：与 15 同构，只换当前项

46 高、`panel` 底、下框 1、内距 `0 12 0 14`、间距 10：`Sync` 13/600、字距 -.01 → `sync-tabs`（左距 4、内距 2、1px `line-soft`、圆角 6、`panel-2`、项间距 1）→ 三个 `sync-tab` 各 22 高、内距 0×10、圆角 4、10.5 号 `text2` → 右端 `×` 24×24、圆角 5、16 号、`margin-left:auto`。本板 `Pull` 是当前项（`content` 底 + 阴影 + 600，`button`），`Fetch` / `Push` 是 `<a>`，分别链 28 / 15。面板 `aria-label` 为 `Pull from origin/main`。

## 3. 左列：Pull 表单（8 段，内距 14、间距 10）

1. 小标题 `Pull`：10.5 `text3`（原 8.5/600/.06em/大写，末次声明改成 10.5、无字距、不大写）。
2. 目标行（间距 7、11 `text2`）：`main`（mono 10）→ `←`（内联 `text3`）→ `origin/main`（mono 10）→ 右端 `2 behind`（hint，`margin-left:auto`）。方向与 15 相反（15 是 `main → origin/main`），且没有 `badge-soft`、没有 `Upstream:`。
3. `Integrate 2 commits` 行：10 `text3`、上框 1 `line-soft`、上内距 8、间距 6；`Integrate` 常规、`2 commits` 为 `b`；右端 `fast-forward available`。15 的对应位置写 `Remote origin · …`，28 写 `Remote origin` + `Last fetched`。
4. **整合方式分段**（15 / 28 都没有这一块）：`.seg`、`role=group`、`aria-label="Pull mode"`、`margin-top:7`；容器内距 2、项间距 2、1px `line-soft`、圆角 7、`panel-2` 底；三项 `Merge` / `Rebase` / `FF only` 各 23 高、内距 0×10、圆角 5、12 号 500 `text2`，当前项 `Rebase` 为白底 + 阴影 + `text`。
5. 两个复选（内联 `border-top:0;padding-top:4px`，因为紧接分段）：`Autostash local changes`（勾）、`Rebase all local commits`（不勾）；`checkrow` 22 高、间距 7、11.5 `text2`、方框 13。15 与 28 都是三项。
6. 认证行 `margin-top:auto` 推底：`● Authenticated · GitHub`（9.5 `ok`、6×6 圆点、间距 6），与 15、28 一字不差。
7. 动作行（间距 6）：`Pull`（`primary`、`flex:1` 居中、可点）+ `Cancel`（`btn`）；两枚按末次 `.btn` 为 24 高 / 12 号。
8. 末尾说明 10.5 `text3`：`Rebase replays your 2 local commits on top of origin/main.` 15 这里是数字摘要，28 是 fetch 的语义边界句。

## 4. 右列：Incoming

底 `panel`、左边框 1、内距 14、间距 9：

- 小标题 `Incoming`（10.5 `text3`）。三板的右列小标题各不相同：15 `Activity`、28 `Remote refs`、29 `Incoming`。
- 活动行 `2 commits from origin/main`（10 `text2`、间距 6），**无转圈**（15 有 10×10 spinner）。
- 日志块 `sync-log`（`flex:1`、可滚）：mono 9/16、`content` 底、1px `line-soft`、圆角 6、内距 7×8、`pre-wrap`。两行，结构是 `<b>` 包短 SHA（`text2` 500）+ 两个空格 + 提交标题（`text3`）——与 28 的「引用名 + 空格列 + SHA」正好相反。板上源码在第二行后多一个换行，`pre-wrap` 会多显示一行空白。
- 底部提示 `margin-top:auto`：`Conflicts open the merge editor. Cancel leaves the branch untouched.`（10.5），是本板唯一提到冲突的地方。
- 仍然**没有**进度条、没有阶段说明、没有 `Pulling…` 禁用态。

## 5. 面板脚

46 高、内距 0×14、间距 7、上框 1、`panel` 底：左 `2 ahead · 2 behind`（10.5 `text3`），右 `Close`（`a.btn`、26 高 / 10 号、链 04）。15 写 `2 commits ahead · 0 behind`，28 写 `origin · 2 new commits`；三板的脚各报一件事：15 报 ahead/behind 且 behind 为 0，28 只报新提交数，本板把 behind 写成非零的 `2`。

## 6. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 操作页签 | Fetch / Pull / Push | 只画 `Pull` 当前 |
| 整合方式 | Merge / Rebase / FF only | 只画 `Rebase` 当前 |
| 进行阶段 | 空闲 / 传输中 / 完成 / 失败 | 只画空闲（按钮可点、无进度） |
| 认证 | 已认证 / 需凭据 / 失败 | 只画已认证 |
| 冲突 | 有 / 无 | 只在提示句里提到，未画冲突态 |

```
进入：15 的 Pull 页签 → 本板；工具条 Pull 按钮在遮罩下，板上无 href
切换：Fetch ⇄ Pull ⇄ Push 只换左右两列；Merge/Rebase/FF only 只换左列第 4 段，右列不跟着变
离开：× / Close → 04；Cancel 只中止本次 pull（提示句承诺分支不动，结果未画）
```

### 当前实现行为

Merge、Rebase 与 FF only 分别映射到 Git 的 `--no-rebase`、`--rebase` 与 `--ff-only`；Autostash 按勾选传入，Rebase all local commits 只在 Rebase 策略下可用，并映射到 `--rebase-merges`。Pull 运行期间选项锁定，右侧显示真实 Git 输出和不定进度。

完成、失败或取消后会刷新目标标签的分支、领先/落后数、变更、引用与 Git 操作状态。取消只停止当前 Git 子进程，不自动 abort 已开始的 merge/rebase；若产生冲突，主窗口继续交给现有冲突流程处理。关闭面板不会取消 Pull，重新打开可查看任务结果。

## 7. 画板没覆盖的组合（待补设计）

1. **进行中与完成**：没有 `Pulling…`、没有进度条、没有 fetch→integrate 的分阶段输出；成功后面板是否自动关、左列目标行的 `2 behind` 是否归零都没画。
2. **冲突**：提示句写 `Conflicts open the merge editor`，但板上既没有冲突画面，也没有到 14 的链接。
3. **换模式**：`Merge` 与 `FF only` 选上之后第 3 段说明、第 5 段复选是否还适用（`Autostash` 对 merge 也成立，`Rebase all local commits` 对 merge 不成立）没画。
4. **`Rebase all local commits`**：勾上是否会重写已推送提交、是否需要 force-push 提示，没画。
5. **分叉**：`fast-forward available` 是单一路径；既 ahead 又 behind（diverged）时这一格换成什么没画。
6. **`Autostash` 失败**：贮藏能存不能还（冲突）时怎么提示没画。
7. **数字不自洽**：面板目标行 `2 behind`、脚 `2 ahead · 2 behind`，底层状态栏仍 `2 ahead · 0 behind`；哪一处是真值板上没给答案。
8. **日志边界**：`sync-log` 固定 `flex:1`，两行提交之外的滚动、复制、自动滚底都没画。

## 8. 按钮与链接清单

4 个按钮；页签、分段与复选不计入，见本节末。

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 面板头右 | `×`（`aria-label=Close sync`） | 图标 24×24 | 关闭面板回 04 | 常驻 |
| 2 | 左列动作行 | `Pull` | `primary` 24 高 / 12，`flex:1` | 执行 pull | 板上可点；无进行中/禁用态 |
| 3 | 左列动作行 | `Cancel` | `btn` 24 高 / 12 | 中止本次 pull | 空闲时是否隐藏未画 |
| 4 | 面板脚右 | `Close` | `a.btn` 26 高 / 10 | 关闭并回 04 | 常驻 |

- **页签**：`Fetch`（`a` → 28）/ `Pull`（`button`，`on`）/ `Push`（`a` → 15），`role=tablist`，项 22 高、10.5。
- **分段**：`Merge / Rebase / FF only`，`role=group`、`aria-label="Pull mode"`，项 23 高、12 号，`Rebase` 为白底当前项；三段都是 `button`，没有 href。
- **复选**：`Autostash local changes`（勾）、`Rebase all local commits`（不勾）。
- **整行/整块**：底层侧栏行、提交行、检查器文件行都在遮罩下，不可点，不计入。

## 9. 输入与选项

| 控件 | 形态 | 画板取值 |
|---|---|---|
| 操作 | 三段页签 | `Pull` |
| 整合方式 | 三段分段 | `Rebase` |
| Autostash local changes | 复选 | 勾 |
| Rebase all local commits | 复选 | 不勾 |
| 目标 | 只读文本 | `main ← origin/main`，`2 behind` |

没有文本输入；可变的是页签、分段三项与两个复选。

## 10. 文案要点

- `behind` 三处两个值：目标行 `2 behind`、脚 `2 ahead · 2 behind`，而底层状态栏仍 `2 ahead · 0 behind`——面板与底窗显然取了两份数据，板上没解释以谁为准。
- `Integrate 2 commits` 指远程那 2 条，`Rebase replays your 2 local commits` 指本地那 2 条，都用「2」但一个是 incoming、一个是 local；`commits` 与 `local commits` 是唯一区分。
- `fast-forward available` 与当前选中的 `Rebase` 并列：既然可快进，默认却选重放提交——两个信息互相抵消，板上没有解释。
- 右列标题从 `Activity`（15）、`Remote refs`（28）变成 `Incoming`，是全板唯一按「进入方向」命名的一栏；脚注 `2 ahead · 2 behind` 也把两个方向并排，说明这页关心的是双向差值而非进度。

## 11. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 15 Sync & auth / 28 Sync · Fetch | 头里 `Fetch` 链 `SyncFetch.dc.html`、`Push` 链 `Sync.dc.html`；两板又都链回本板 |
| 14 Merge conflict | 提示句承诺「conflicts open the merge editor」，但板上没给 `MergeConflict.dc.html` 的 href |
| 04 Commits | `×`、`Close` 与底层返回条 `Commits` 三处都链 `Workspace.dc.html` |
| 10 Changes / 12 Agent / 18 Files at commit | 底层工具条 `Changes 12`、`Agent`，检查器 `Files` 页签 |
| 23 Settings / 01 Welcome | 底层标题栏齿轮、页签 `+`；凭据归属板上没给链接 |

启用自动贮藏后，可以选择「拉取后自动恢复贮藏」，默认启用。取消勾选时，已跟踪文件的改动会先存为普通 Stash，拉取结束后可从 Stashes 手动恢复。
