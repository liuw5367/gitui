# Agent 面板 — 布局、状态与按钮

- **画板**：12 Agent panel（有 Agent 面板 · 有 Working 态 · 有 `Needs approval` 提案；无空会话、无执行中、无工具失败）
- **来源**：`designs/graft-git/project/Agent.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

本页是**主窗口换主体**：标题栏、工具条、侧栏、状态栏沿用 04，右列由 360 的提交检查器换成 400 宽的 Agent 面板，中列因此从 798 收到 758。预览 1440×900、桌面四周留白 24，窗口仍是 **1392×852**（圆角 12、1px 描边）。

## 1. 骨架

```
┌───────────────┬───────────────────────────────────────────────┬──────────────────────────────────┐
│ ● ● ●  [gitui ×] [docs-site ×] [atlas-web ×] [+]     ● Fetched 2m ago   [gear]                   │
├──────────────────────────────────────────────────────────────────────────────────────────────────┤
│ [=] | Fetch Pull Push | Branch Stash   [search commits ⌘F] | History [Changes 12] | Agent | [=]  │
├───────────────┬───────────────────────────────────────────────┬──────────────────────────────────┤
│ Branches 5    │ [<] Commits [Wrapped-line fix] +149 -90       │ Commit 12  [sparkle] Agent    ×  │
│ ● main ✓ ↑2   │ DiffView.tsx +31 -18 [Split][Unified] 1 of 2 ►│ [sparkle] gitui Agent            │
│ ● feature/dif…│ @@ -38,7 +38,10 @@ export function DiffView({ │ ● Working · read + write + push  │
│ ● feature/ag… │  38  38   export function DiffView({ file,    │                     [Ask first v]│
│ ● fix/wrap…   │  39  39     const [wrap, setWrap] = useState  │ Prepare the diff-view fix, commit│
│ ● release/1.4 │  40  40     const rows = useMemo(() => align  │ and push to origin.              │
│ Remotes 2     │  41      -   const split = mode === 'split'   │ [sparkle] 12 files changed. Three│
│   origin 8    │  41      -   const columns = split ? 2 : 1    │   belong to the wrapped-line fix │
│ Tags 3        │  41      +   const split = mode === 'split' ||│ $ git status --short     12 files│
│ ▣ v1.4.0      │  42      +   const columns = split ? 2 : 1    │ M  src/git/diff-engine.ts        │
│ Stashes 2     │  43      +   const density = wrap ? 'relaxed' │ M  src/ui/DiffView.tsx           │
│ Submodules 2  │  44  47   }                                   │ M  …9 unrelated changes          │
│ Worktrees 3   │ v 24 unchanged lines v                        │ Proposed plan      Needs approval│
│               │                                               │ 1. Stage diff-engine.ts          │
│               │                                               │ 2. Stage DiffView.tsx            │
│               │                                               │ 4. Commit "Fix wrapped-line…"    │
│               │                                               │    3 files · +76 -38             │
│               │                                               │ 5. Push to origin/main   allowed │
│               │                                               │ 1 commit · 1 push                │
│               │                                               │ [Edit]      [Approve & Run]      │
│               │                                               │ [sparkle] Auto ⌘K     [send]     │
│               │                                               │ [lock] Read + commit + push      │
├───────────────┴───────────────────────────────────────────────┴──────────────────────────────────┤
│ [branch] main   ● Agent · awaiting approval      3 staged · 9 unstaged  UTF-8  LF  TS            │
└───────────────┴───────────────────────────────────────────────┴──────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 标题栏 | 顶部通栏 | 高 40（内联 `height:40px`，压过样式表的 44）；内距 0×13、间距 9 | 窗口灯 12×3、三个仓库页签、`+`、右端 `● Fetched 2m ago` 与齿轮 |
| 工具条 | 标题栏下 | 高 44；内距 0×10；间距 6 | 侧栏开关、Fetch/Pull/Push、Branch/Stash、搜索、`History / Changes 12`、`Agent`、检查器开关 |
| 侧栏 | 主体左 | 宽 232（含右框 1）；内距 2/8/12 | Branches 5 / Remotes 2 / Tags 3 / Stashes 2 / Submodules 2 / Worktrees 3 |
| 中列 | 主体中 | 宽 758（= 1390 − 232 − 400）；34 返回条 + 38 diff 条 | `src/ui/DiffView.tsx` 的 unified diff，右上 `12 files · +149 −90` |
| Agent 面板 | 主体右 | 宽 400（`.insp.wide` 覆盖 360，含左框 1）；页签 33、头约 48、脚约 114 | 页签、身份行、消息流、composer |
| 状态栏 | 底部通栏 | 高 24；内距 0×12、间距 9；字号 11 | 分支、`Agent · awaiting approval`、暂存计数、编码/换行/语言 |

主体高 742 = 852 − 2 − 40 − 44 − 24（本页没有横幅）。两个推导值：中列 1390 − 232 − 400 = 758；面板消息区 742 − 33 − 48 − 114 ≈ 547。图为示意（面板列略放大），`[<]` `[=]` `[sparkle]` `[gear]` `[lock]` 是图标占位。

## 2. 标题栏与工具条

页签 `rtab` 高 27、圆角 6、内距左 10 右 9、间距 4、最大宽 178（超出省略），字号 12；活动页签 `content` 底 + `line-strong` 描边 + 5×5 `accent` 圆点，字重 500。`×` 的显隐不对称：活动页签平时透明度 0、hover 才现，另两个是内联 0.55 常显。右端 `title-right` 间距 4、11 `text3`：6×6 `ok` 点 + `Fetched 2m ago` + 齿轮 26×24。

工具条 `tbtn` 高 26（`v2` 覆盖基础的 27）、内距 0×9、图标 13、间距 5、12/500；`tb-icon` 26×24、圆角 6；`tb-sep` 1×18、外距 3，把按钮切成四簇。搜索框 176×26、内距 0 7 0 8、占位 `Search commits`、右端 `⌘F` 键帽（10、行高 15、圆角 4）。分段 `mode-pill`：容器内距 2、圆角 7、`panel-2` 底 + 1px `line-soft` 描边；项高 22、内距 0×8、11.5/500、间距 3。本板 `History` 是链到 04 的 `<a>`，`Changes 12` 是当前项（`content` 底、计数 9.5 转 `accent-2`）。`Agent` 在 `tbtn.ai`（`accent-soft` 底、`accent-2` 字、600）之外还有内联 `box-shadow: inset 0 0 0 1px accent` 的 1px 内描边与 `aria-pressed="true"`——13 板相同，14/15 板那颗没有描边。

## 3. 中列：返回条与 diff

返回条是内联的 34 高条（内距 0×11、间距 8、`panel` 底、下框 1）：`Commits` 是 22 高的 `chipbtn`（内距 0×7、圆角 5、11，左端 10 的 chevron），随后 `badge-soft`（18 高、内距 0×6、圆角 4、9.5）写 `Wrapped-line fix`，再跟路径 `src/ui/DiffView.tsx`，右端 `12 files · +149 −90`（10.5）。diff 条 38 高、内距 0×11、间距 8：crumb 是 `badge-soft` `Wrapped-line fix` → 10 的 chevron → mono 11 的 `src/ui/DiffView.tsx` → mono 9.5 的 `+31 −18`；右端依次 `seg tiny`（项 21 高、内距 0×7、10.5，`Unified` 当前项）、`tb-sep`、三个 24×24 图标按钮（收起 hunk / 换行 / `¶`）、`1 of 2`（10.5，tnum）、`Next change`。

diff 体 mono 11、行高 19；表头 `dhead` 被内联改成单列 `grid-template-columns:1fr`，内距 4×9、9.5，写 `a/src/ui/DiffView.tsx → b/src/ui/DiffView.tsx`。行号列 42 宽（内距 0 7 0 4、9.5、右框 1 `line-soft`）；hunk 行 21 高、10 号 `hunk-fg`，右端 `Unstage Hunk` 是 17 高的 mini 标签（内距 0×5、9.5，板上是 `span`）；删行整行 `del-bg`、词级 `word-del`，增行 `add-bg` / `word-add`；折行 20 高、10、居中 `24 unchanged lines`。

## 4. Agent 面板：页签与头部

页签条 33 高、内距 0×14（板上另加右内距 8）、间距 14、下框 1、`panel` 底：`Commit 12` 是链到 10 的页签（12 `text2`，计数标签 9.5、`panel-2` 底、圆角 4、行高 14）；`Agent` 是当前项（12/600、底部 2px `accent`、`accent-soft` 计数底）；右端 `×` 是 24×24 图标按钮（15 号、`aria-label="Close agent panel"`、链到 10）。头部 `agent-head` 内距 10×12、下框 1、`panel` 底、间距 8：左侧 24×24 圆角 7 的反白图标（`text` 底、`content` 图标），右侧 `gitui Agent`（12/600、行高 1.2）与 `● Working · read + write + push`（9.5、6×6 点、间距 3）；右端 `Ask first ▾` 是 22 高的 `chipbtn`（11、chevron 9，链到 13）。

## 5. Agent 面板：消息流与 composer

消息区内距 12、间距 10、`content` 底、自己滚动。用户一句靠右、最大宽 88%、`primary-bg` 底白字、圆角 11 11 3 11、内距 7×10、11.5/1.4。Agent 消息左侧 20×20 圆角 6 图标 + 气泡 11.5/1.45；气泡里的 `<b>` 是 600。

工具卡 `toolcard`：1px `line` 框、圆角 8、`panel` 底。头 27 高、内距 0×8，命令 mono 9.5、右端 `12 files`（9、`ok`）；体 5/9/7、mono 9.5/1.55，末行 `M  …9 unrelated changes` 用 `text3`。

计划卡 `plan`：圆角 9、`panel` 底。头 29 高、10.5/600，右端 `Needs approval`（8.5/600、`tag-bg`、圆角 4、内距 1×4）；五步各 10.5/1.35、内距 5×9，序号是 14×14 圆圈、8 号数字（1–3 标了已完成：`ok` 底白字；4–5 只有描边）；第 4 步副行 9.5，第 5 步尾随 `allowed`（9/600 `ok`）；脚 7×9、上框，左 `1 commit · 1 push`（9），右 `Edit`（ghost）与 `Approve & Run`（primary）。作用域行上距 6、9 号 `text3`：`Push is enabled for origin/main · approval runs both steps`。

页脚内距 9×10、上框 1、`panel` 底：composer 1px `field-border`、圆角 8、内距 7/8/6，`textarea` 2 行、11.5/1.4、`min-height` 30、占位 `Ask the agent to stage, commit, push, or explain…`；composer 条上距 3、间距 5，左起 sparkle 12、`Auto`（17 高的 `badge-soft`、9 号）、`⌘K commands`（9.5），右端发送按钮 23×23、圆角 6、`primary-bg` 白图标（`aria-label="Send message"`）；最后一行 9 号 `text3`，10 的锁 + `Read + stage + commit + push · change in Settings › Agent`。

## 6. 侧栏与状态栏

侧栏逐项与 04 相同：组头 27 高、10.5/600、**不转大写**（末次覆盖 `text-transform:none`）、右端计数 10.5/500；行 25 高、内距 0×7、圆角 6、字号 13，`ind1` 12.5、URL 行内联 10.5。当前分支 `main` 是 `accent` 底白字（末次 `side-row.sel`），`✓` 11 白，`↑2`/`↓0` 10 号转 `rgba(255,255,255,.86)`。状态栏 24 高、内距 0×12、间距 9、11 号 `text3`：`main`（11 的 `accent` 分支图标）、`● Agent · awaiting approval`（6×6 `ok` 点）、弹性空隙、`3 staged · 9 unstaged`、`UTF-8`、`LF`、`TypeScript`。这一行是本板独有——04 那里是 `2 ahead · 0 behind` 与 `12 changes`。

## 7. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 右列内容 | Agent 面板 / 提交检查器 | 12、13 画 Agent；14、15 画检查器（360） |
| Agent 运行态 | Working / 空闲 / 已停止 | 只画 Working |
| 提案状态 | Needs approval / 已批准 / 执行中 | 只画 Needs approval |
| 权限档 | `Ask first` / 更宽的档 | 只画 `Ask first` |
| 仓库视图 | Changes 12 / History | 12、13 画 Changes；14、15 画 History |

```
进入：10 的 Changes 页签 / 04 工具条 Agent → 本板（Agent 变 accent 底 + 1px 内描边）
切换：Commit 12 ⇄ Agent 只换右列；History ⇄ Changes 12 只换中列
离开：面板 × → 10；Ask first ▾ → 13；工具条 History → 04
```

## 8. 画板没覆盖的组合（待补设计）

1. **空会话**：既没有消息也没有提案时消息区放什么（空态文案、示例 prompt），板上没有。
2. **执行中**：`Approve & Run` 之后五步逐步跑时，序号圆圈除“已完成”外的进行/失败/跳过三态没画，按钮是否变 Stop 也没画。
3. **失败与拒绝**：工具卡只画成功（`12 files`）；命令非零退出、push 被拒、权限不足三种都没有对应样式。
4. **“已完成”与“待批准”共存**：计划卡头部写 `Needs approval`，1–3 步却已标成 done；哪几步在批准前就算完成，板上读不出来。
5. **三处权限措辞的映射**：头写 `read + write + push`、chip 写 `Ask first`、脚写 `Read + stage + commit + push`，是同一档还是三档，没有对照表。
6. **窄窗与多仓库**：样式表在 ≤1080 把 `.insp.wide` 收到 350、≤860 直接隐藏检查器，这两种宽度下的换行与计划卡没画；页签切到 `docs-site` 时 Agent 会话是否跟着换、`⌘K commands` 打开的面板本身，也只有键帽没有画面。

## 9. 按钮与链接清单

共 15 组；分段、页签、侧栏整行不计入，见本节末。

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 标题栏页签右 | `+`（`aria-label=Open repository`） | 图标 27×27 | 打开 01 | 常驻 |
| 2 | 标题栏右 | 齿轮（`aria-label=Settings`） | 图标 26×24 | 打开 23 | 常驻 |
| 3 | 工具条最左 | 侧栏图标（`aria-label=Toggle sidebar`） | 图标 26×24 | 收起/展开侧栏 | 板上无按下态 |
| 4 | 工具条 | Fetch / Pull / Push / Branch / Stash | 文字+图标 26 高 | 同步、建分支、存 stash | 板上都不禁用 |
| 5 | 工具条 | Agent | 文字+图标 26 高，accent-soft | 打开本面板（板上是按钮，不是链） | `aria-pressed` + 1px 内描边 |
| 6 | 工具条最右 | 镜像侧栏图标（`aria-label=Toggle inspector`） | 图标 26×24 | 收起/展开右列 | 板上无按下态 |
| 7 | 中列返回条 | `Commits` | chip 22 高 + chevron 10 | 回 04 | 常驻 |
| 8 | diff 条右 | `Collapse all hunks` / `Toggle word wrap` / `Show whitespace`（`¶`） | 图标 24×24 | 折叠、换行、显示空白 | 都无 `on` 态 |
| 9 | diff 条右 | `Next change`（`aria-label`） | 图标 24×24 | 跳到下一处改动 | `1 of 2` 说明还有一处 |
| 10 | hunk 行右 | `Unstage Hunk` | mini 17 高 | 取消暂存该 hunk | 板上是 `span`，不接收点击 |
| 11 | 面板页签右 | `×`（`aria-label=Close agent panel`） | 图标 24×24 | 回 10 | 常驻 |
| 12 | 面板头右 | `Ask first ▾` | chip 22 高 + chevron 9 | 打开 13 | `aria-label=Agent permissions` |
| 13 | 计划卡脚 | `Edit` | small ghost 22 高 | 改计划步骤 | 仅提案卡存在时 |
| 14 | 计划卡脚 | `Approve & Run` | small primary 22 高 | 批准并执行 4–5 步 | 板上可点（未画禁用） |
| 15 | composer 右下 | 纸飞机（`aria-label=Send message`） | 23×23 圆角 6 | 发送输入 | 空输入时是否禁用未画 |

- **分段**：工具条 `History / Changes 12`（`role=group`）；diff 条 `Split / Unified`（`role=group`，项 21 高、10.5，当前项白底）。
- **页签与整行**：面板 `Commit 12 / Agent`（`role=tablist`，前者链 10）与顶部三个仓库页签都整块可点，`aria-label` 形如 `gitui repository`；侧栏 30 行引用也是整行可点，消息气泡与计划步骤不是。

## 10. 输入与选项

工具条搜索 176×26（占位 `Search commits`，聚焦转 `accent` 描边 + 3px 光晕）。composer 是唯一多行输入：2 行、11.5/1.4、占位 `Ask the agent to stage, commit, push, or explain…`；`⌘K commands` 只是键帽提示。`Ask first` 是下拉入口，展开后的选项板上没有。

## 11. 文案要点

- 用户只说结果（`Prepare the diff-view fix, commit it, and push to origin.`），Agent 先用两句交代判断（12 个改动里 3 个属于这次修复），再给下一步。
- 工具卡用原命令 `$ git status --short` 加结果 `12 files`，末行把剩余 9 个改动折叠成 `M  …9 unrelated changes`。
- “需要批准”写在卡片头部徽章（`Needs approval`），“已放行”写在单步（push 后的 `allowed`），两个词不要互换。
- 页脚固定三段：`Read + stage + commit + push`、`change in Settings › Agent`、`Auto` 档。

## 12. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 10 Staging & commit | 面板页签 `Commit 12` 与关闭 `×` 都链到 `Changes.dc.html` |
| 13 Agent permissions | 面板头 `Ask first ▾` 链到 `AgentPermissions.dc.html` |
| 04 Commits | 返回条 `Commits`、分段 `History` 链到 `Workspace.dc.html` |
| 23 Settings | 脚注 `Settings › Agent` 与标题栏齿轮 |
| 14 / 15 | 两板工具条 `Agent` 链到本板（`<a class="tbtn ai">`），本板是按下态 |
