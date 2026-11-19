# 合并冲突 — 布局、状态与按钮

## 当前实现

实现从 Git index 读取每个未解决路径的 stage 1（Base）、stage 2（Ours）与 stage 3（Theirs），不再从工作区冲突标记解析左右内容。每侧按原始字节读取；缺失 stage 显示“此侧删除了该文件”，二进制侧提示无法预览。三侧文本与 Result 都使用 NSH 原生 Editor，并依据路径扩展名启用语法高亮。

Result 的文本编辑随当前仓库 Tab、文件路径保存；切换冲突文件或关闭冲突面板不会丢弃草稿。接受 Ours/Theirs 会从对应 index stage 取内容；该侧不存在时 Result 记录为删除。二进制 Result 必须先选择一侧。Accept Both 仅在原工作区含标准冲突标记的文本文件上可用。

Mark Resolved 会验证 Result 不含未解决标记，然后只写回并暂存当前路径；选择删除时用 `git rm` 暂存该路径。所有冲突解决后视图保留“全部解决”状态，Continue 可点击；操作横幅也提供 Resolve 或 Continue 入口，以便关闭视图后继续。Continue 会在执行前重新读取操作类型与未解决路径，并拒绝存在未解决文件或草稿的情况。成功继续或 Abort 后会清理冲突视图状态。

- **画板**：14 Merge conflict（有 3 个冲突文件的第 1 个 · 有 Base 条 · 有 Files 浮层 · `Apply & Continue` 禁用；无已解决态、无失败态、无二进制冲突）
- **来源**：`designs/graft-git/project/MergeConflict.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

本页是**主窗口加模态浮层**：底层是 04 的提交视图（中列 diff、右列检查器全在），整窗盖 `rgba(0,0,0,.34)`，正中放一块 **1320×772** 的 `Resolve Conflicts` 面板；面板里不是 04 的三栏，而是「Base 横条 + Ours / Result / Theirs 三列」。

## 1. 骨架

```
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│ ● ● ●  [gitui ×] [docs-site ×] [atlas-web ×] [+]     ● Fetched 2m ago   [gear]                   │
│  ┌────────────────────────────────────────────────────────────────────────────────────────────┐  │
│  │ [MERGE] Resolve Conflicts                                3 files · 3 conflicts    ×        │  │
│  │ Merging origin/main into main                                                              │  │
│  ├────────────────────────────────────────────────────────────────────────────────────────────┤  │
│  │ [Base] │ [Accept Ours] [Accept Theirs] [Accept Both] │ [Mark Resolved]  ◄ ► 1 of 3 [Files] │  │
│  ├────────────────────────────────────────────────────────────────────────────────────────────┤  │
│  │ BASE 3f2a1c9 │ export function buildHunks(lines: string[], context = 3) {     [Hide]       │  │
│  │              │   const hunks: Hunk[] = []; let start = 0; return hunks; }                  │  │
│  ├────────────────────────────────────────────────────────────────────────────────────────────┤  │
│  │ Ours      HEAD      22 lines │ Result     editable 24 lines │ Theirs origin/main  22 lines │  │
│  │ 18  export function build…   │ <<<<<<< HEAD                 │ 18  export function build…   │  │
│  │ 19    const hunks: Hunk[]…   │ 21    for (let i = 0; i <…   │ 19    const hunks: Hunk[]…   │  │
│  │ 20    let start = 0;         │ =======                      │ 20    const changed = fin…   │  │
│  │ 21    for (let i = 0; i <…   │ 21    const changed = fi…    │ 21    for (const line of…    │  │
│  │ 22      if (isChange(line…   │ 22    for (const line of…    │ 24    return hunks;          │  │
│  │ 23        hunks.push(make…   │ >>>>>>> origin/main          │ 25  }                        │  │
│  │ 26    return hunks;          │ 26    return hunks;          │                              │  │
│  │ 27  }                        │ 27  }                        │                              │  │
│  ├────────────────────────────────────────────────────────────────────────────────────────────┤  │
│  │ ● 2 unresolved   Result is editable   [Abort Merge]   [Apply & Continue]                   │  │
│  └────────────────────────────────────────────────────────────────────────────────────────────┘  │
│                                                                                                  │
│ [branch] main   ● 2 ahead · 0 behind   12 changes  UTF-8  LF  TypeScript                         │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 遮罩 | `inset:0`，z 60 | 内距 22×34；`rgba(0,0,0,.34)` | 铺满整个窗口，含标题栏；底层内容全部压暗但仍可见 |
| 面板 | 水平居中、顶端对齐 | **1320×772**；1px `line`、圆角 12；z 在遮罩之上 | 头 46 + 工具条 38 + 主体 642 + 脚 46 |
| 面板头 | 面板顶 | 高 46；内距 0×14；间距 9 | `MERGE` 徽章、标题与副标、`3 files · 3 conflicts`、`×` |
| 面板工具条 | 头下 | 高 38；内距 0×12；间距 6 | Base、三个 Accept、Mark Resolved、翻页、`1 of 3`、Files |
| Base 条 | 工具条下 | 高 62 | 86 宽的标签列 + 基准代码 + `Hide` |
| 三列 | Base 条下 | `1fr / 1.1fr / 1fr`（1318 内 → 约 425 / 468 / 425） | Ours / Result / Theirs，每列自带头 30 与代码区 |
| Files 浮层 | 面板内 top 84、right 12 | 宽 260；z 20 | 冲突文件树，压住三列右上方 |
| 面板脚 | 面板底 | 高 46；内距 0×14；间距 8 | `2 unresolved`、说明、`Abort Merge`、`Apply & Continue` |

窗口 1392×852 不变。面板上沿在窗口顶下方 22、左右各 35、下沿 794——所以它压住标题栏下缘 18px 和整条工具条，底部还剩 56 的遮罩（其中 24 是状态栏，另 32 是裸主体）。图不按比例：面板画成 94 列，遮罩只剩两侧 2 列。

## 2. 面板头与工具条

头 46 高、内距 0×14、间距 9、`panel` 底、下框 1：`MERGE` 徽章（18 高、内距 0×6、圆角 4、`accent-soft` 底 `accent-2` 字、8.5/700、字距 .05em）；标题 `Resolve Conflicts`（13/600、字距 -.01）；副标 `Merging origin/main into main`（10 `text3`、上距 1）；`3 files · 3 conflicts`（10.5）左距 8；右端 `×` 24×24、圆角 5、16 号、`aria-label="Close merge editor"`，链到 04。

工具条 38 高、内距 0×12、间距 6，里面按钮统一 22 高、9.5 号、内距 0×7（比通用 `small` 更矮一档）：`Base` ｜ `Accept Ours`、`Accept Theirs`、`Accept Both` ｜ `Mark Resolved`，两组之间是 1×18 的 `tb-sep`。右端依次 `◄` `►` 两个 24×24 图标按钮（`Previous conflict` / `Next conflict`）、`1 of 3`（10.5 tnum）、`Files` 按钮——带 12 号文件夹图标、`aria-expanded="true"`、`aria-controls="merge-files"`，并用内联样式改成 `accent-soft` 底、`accent-2` 字、无描边。

## 3. Base 条与三列

Base 条 62 高、`panel-2` 底、下框 1，横向三段：左 86 宽的标签列（内距 8×10、右边框 1，`Base` 9/600 全大写 + `3f2a1c9` mono 9）；中间基准代码 mono 9.5/15、内距 7×10、`white-space:pre`；右端 8 的内距里放 `Hide`（ghost small、22 高）。

三列各带 30 高的列头（内距 0×9、`panel` 底、下框 1）：名字 10/600、ref mono 8.5、行数在右端 9 号——`Ours / HEAD / 22 lines`、`Result / editable / 24 lines`、`Theirs / origin/main / 22 lines`。Result 列宽是 1.1 份（约 468，两侧各约 425），左右各有一条 1px `accent` 边，可编辑（`contenteditable`，`aria-label="Editable merge result"`），聚焦时加 1px 内描边。

代码区 mono 10、行高 18、上下内距 7，行内 `gap 8`：行号列 22 宽、右对齐、`text3`。着色按来源分行：Ours 行 `accent` 9% 混色、Theirs 行（也含 `add`）`ok` 10%、`del` 行 `danger` 8%；冲突标记行 9 号、高 18——`<<<<<<< HEAD` 用 `accent-2`、`=======` 用 `text3`、`>>>>>>> origin/main` 用 `ok`。Result 列的光标是 1.5×14 的 `accent` 竖条，1.05s 闪烁。

## 4. Files 浮层

绝对定位在面板内 `top:84`（= 头 46 + 工具条 38）、`right:12`，宽 260，z 20，1px `line`、圆角 8、阴影 0 16 40，不透明——压住三列右上方，下方与左侧的代码仍可见。

头 34 高、内距 0 10 0 8、`panel` 底、下框 1：`Files` 10/600，右端 `seg tiny` 的 `Tree / List`（项 21 高、10.5，`Tree` 当前项）。体 内距 4、最大高 260、可滚：分组头 23 高、9.5/600 `text3`（10 的 chevron + 12 的文件夹）；文件行 23 高、内距 0×6、圆角 4、9.5 号，左端 5×5 圆点（未解决 `danger`、已解决 `ok`），右端计数 8.5。

板上三行：`src/git/diff.ts` `U` `2 conflicts`（当前行 `sel` 底）、`merge.ts` `U` `1 conflict`、`src/ui/DiffView.tsx` `R` `resolved`（右端文字转 `ok`）。

## 5. 面板脚与计数

脚 46 高、内距 0×14、间距 8、上框 1、`panel` 底：左 `● 2 unresolved`（6×6 `danger` 点、10 号 `text2`）+ `Result is editable · Mark resolved when done`（10.5）；弹性空隙；右 `Abort Merge`（`btn` 26 高、10 号）与 `Apply & Continue`（`primary` 同样 26/10，板上带 `disabled`，透明度 .45、无阴影）。

三处计数口径不一致：头部 `3 files · 3 conflicts`、工具条 `1 of 3`、脚 `2 unresolved`，而浮层里两个 `U` 行写 `2 conflicts` 与 `1 conflict`（合计 3 个），另加 1 个 `resolved` 文件。哪个数是准的，板上没有对齐。

## 6. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 冲突文件 | 3 个 | 只画第 1 个 `diff.ts` |
| 冲突块 | 每文件 1–2 个 | 只画第 1 个（`1 of 3`） |
| 单块状态 | 未解决 / 已解决 | 都画了，但已解决只在浮层的 `resolved` 行 |
| Base 条 | 显示 / 收起 | 只画显示（有 `Hide`） |
| Files 浮层 | 开 / 关 | 只画开（`aria-expanded="true"`） |
| 主按钮 | 可用 / 禁用 | 只画禁用（`2 unresolved`） |

```
进入：10 Changes 的冲突横幅 Resolve… → 本板（唯一入链，Changes.dc.html）
切换：◄ ► 换冲突块；Files 浮层换文件；Tree / List 换分组
离开：× 或 Abort Merge → 04；Apply & Continue 在两个冲突未解决时禁用
```

## 7. 画板没覆盖的组合（待补设计）

1. **已解决态**：`Mark Resolved` 之后该块的行色、`2 unresolved` 怎么减、主按钮何时解禁，都没有画。
2. **三个 Accept 的差别**：`Accept Ours` / `Accept Theirs` / `Accept Both` 只在文案上区分，各自往 Result 里写什么、是否保留标记行没画。
3. **常驻冲突列表**：样式表里定义了 224 宽的冲突列表（行、计数、汇总），板上 `merge-body` 只渲染了三列，左侧列表没有出现；它是否被 Files 浮层取代没定。
4. **底图选错**：合并冲突属于工作区状态，底层却画的是提交 `3f2a1c9` 的 diff（`7 files`），而不是 Changes。
5. **其它冲突类型**：二进制、删除/重命名、整文件冲突、块很长时的滚动同步都没画。
6. **Base 的两个入口**：工具条的 `Base` 按钮与 Base 条右端的 `Hide` 是同一动作还是两个，没画。
7. **写回与外部编辑**：编辑 Result 后写回工作区的时机、能否用外部编辑器（对比 04 的 `Open in Editor`）没画。
8. **窄窗**：面板宽 1320，可用宽 1322，只剩 2px 余量；≤1080 时三列是否变两列、是否横向滚动没画。

## 8. 按钮与链接清单

12 组；底层被遮罩压住的 04 按钮不计入，分段与整行见本节末。

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 面板头右 | `×`（`aria-label=Close merge editor`） | 图标 24×24 | 关闭面板回 04 | 常驻 |
| 2 | 工具条左 | `Base` | btn 22 高 / 9.5 | 切换基准区 | 无按下态 |
| 3 | 工具条 | `Accept Ours` | btn 22 高 | 用 HEAD 侧填 Result | 无禁用态 |
| 4 | 工具条 | `Accept Theirs` | btn 22 高 | 用 origin/main 侧填 Result | 无禁用态 |
| 5 | 工具条 | `Accept Both` | btn 22 高 | 两侧都留 | 无禁用态 |
| 6 | 工具条 | `Mark Resolved` | btn 22 高 | 标记当前块已解决 | 已解决后是否置灰未画 |
| 7 | 工具条右 | `◄`（`aria-label=Previous conflict`） | 图标 24×24 | 上一处冲突 | 第 1 处时是否禁用未画 |
| 8 | 工具条右 | `►`（`aria-label=Next conflict`） | 图标 24×24 | 下一处冲突 | 第 3 处时是否禁用未画 |
| 9 | 工具条右 | `Files`（`aria-expanded`） | btn 22 高 + 文件夹 12 | 开关冲突文件浮层 | 开时 accent-soft 底 |
| 10 | Base 条右 | `Hide` | ghost small 22 高 | 收起基准区 | 收起后文案是否变 `Show` 未画 |
| 11 | 面板脚右 | `Abort Merge` | btn 26 高 / 10 | 放弃合并回 04 | 是否二次确认未画 |
| 12 | 面板脚右 | `Apply & Continue` | primary 26 高 | 写回并完成合并 | 板上 `disabled`（`.45` 透明） |

- **分段**：Files 浮层里的 `Tree / List`（`role=group`，项 21 高、10.5，`Tree` 当前项）。
- **整行可点**：浮层里 3 个文件行与 2 个分组头；Result 是 `contenteditable` 的编辑区，不是按钮。
- 底层窗口的 `Commits`、Files 页签、`Open in Editor` 等仍在 DOM 里，但被模态遮罩压住，不计入本页。

## 9. 输入与选项

| 控件 | 形态 | 画板取值 |
|---|---|---|
| Result | `contenteditable` 多行代码区 | 含 `<<<<<<<` / `=======` / `>>>>>>>` 三行标记，24 行 |
| Base 区 | 只读代码块 | `3f2a1c9` 的 4 行 |
| 冲突导航 | `◄` / `1 of 3` / `►` | 第 1 处 |
| 文件分组 | `Tree / List` | `Tree` |

Result 列可编辑但没有保存按钮——保存含义落在脚上的 `Mark Resolved` 与 `Apply & Continue`。

## 10. 文案要点

- 副标 `Merging origin/main into main` 给出方向（谁的改动进谁），比标题更重要。
- `Result is editable · Mark resolved when done` 一句说明 Result 不是预览；`Apply & Continue` 表明是“继续”而不是“完成”。
- 冲突标记原样保留 `<<<<<<< HEAD`、`=======`、`>>>>>>> origin/main`，没有换成中文说明。
- 浮层用 `U` / `R` 单字母表示未解决/已解决，与 04 检查器里 `M` `A` `D` `R` 是同一套字母。

## 11. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 10 Staging & commit | 冲突横幅的 `Resolve…` 是唯一入链（`MergeConflict.dc.html`） |
| 04 Commits | `×` 与 `Abort Merge` 都链回 `Workspace.dc.html`；底层就是该板的提交视图 |
| 12 Agent panel | 底层工具条的 `Agent` 链到 `Agent.dc.html` |
| 18 Files at commit | 底层检查器的 `Files` 页签链到 `CommitFiles.dc.html` |
| 15 Sync & auth | 同属操作类浮层，但一个是模态、一个是居中面板，遮罩浓淡不同（.34 / .28） |
| 01 Welcome / 23 Settings | 底层标题栏的 `+` 与齿轮 |
