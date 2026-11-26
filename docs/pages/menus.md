# 菜单与对话框 — 布局、状态与按钮

- **画板**：22 Menus & dialogs（有 6 个菜单 + 5 个对话框 + 1 张分支比较卡；无确认类对话框、无校验错误、无禁用项）
- **来源**：`designs/graft-git/project/Menus.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

本板是**独立规格页**，外壳与 16 同族：`gallery-page`（`min-height:100vh`、内距 30×34、gap 14、渐变底），预览 **1440×1400**。没有窗口骨架；页内是一行标题（只有标题与副标，**没有返回按钮**）加一个 12 张卡的栅格。样式表里 `gallery-grid` 默认 2 列，板内内联改成 3 列。

macOS 原生菜单栏不在本画板内：最左侧应用名称菜单取 `brand::PRODUCT_NAME`，包含 About、Settings（`⌘,`）、Hide App、Hide Others、Show All 和 Quit App；Services 不在当前菜单中。File 提供 Open Recent 与 Clear Recent，重名仓库附带父目录路径；Help 提供 Keyboard Shortcuts 并打开设置中的快捷键分区。无活动仓库时依赖仓库的菜单项置灰。About 使用系统 About 面板，Welcome 保留在 Help 菜单。Settings 不在 Repository 菜单重复出现；Welcome、主窗口齿轮与应用菜单再次调用 Settings 时都会聚焦已有窗口。

## 当前实现

应用自绘表单统一复用 Compare reference 的标题栏、正文与底栏，操作按钮使用 New Branch 的常规尺寸，具体刻度以[组件层](../design/UI-KIT.md)为准。该规范覆盖新建／重命名分支、新建标签、添加／编辑远程、全量／选中文件贮藏、创建工作树与身份输入；字段均提供本地化 placeholder，仅展示实际字段，空错误状态不占间距。文本编辑保持输入框身份。Edit Commit Message 与 Squash 使用同一面板和多行输入，保留目标提交、合并范围、阻塞与错误提示；Hook 日志、凭据和设置确认使用相同面板，同步窗口保留分段／日志结构并对齐标题栏与底栏。系统文件选择器和原生确认使用平台样式，命令面板与轻量浮层保留其专用结构。

## 1. 骨架

```
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│ Menus & dialogs                                                                                  │
│ IDEA / Fork-style commit, branch, remote, reset and filter interactions.                         │
├────────────────────────────────┬────────────────────────────────┬────────────────────────────────┤
│ BRANCH · CONTEXT MENU          │ RESET CURRENT BRANCH           │ SQUASH COMMITS                 │
│ menu 252 · 10 items            │ dialog: 3 radios               │ dialog: 3 commits              │
│ 2 separators                   │ Mixed selected                 │ textarea 56 high               │
│ Copy Branch Name key           │ Reset (danger text)            │ Squash (primary)               │
├──────────────────────────────────────────────────────────────────────────────────────────────────┤
│ REMOTE BRANCH · MENU           │ NEW BRANCH                     │ EDIT COMMIT MESSAGE            │
│ menu 252 · 7 items             │ Name + From                    │ 1 textarea                     │
│ 1 separator                    │ From readonly                  │ note: rewords only             │
│ no shortcut                    │ Create Branch                  │ Save                           │
├──────────────────────────────────────────────────────────────────────────────────────────────────┤
│ COMMIT FILTERS                 │ NEW TAG                        │ BRANCH COMPARISON              │
│ menu: 3 checkboxes             │ name + target + msg            │ card 442 wide                  │
│ 2 separators                   │ annotated checked              │ 2 branch refs                  │
│ Apply Filter                   │ Create Tag                     │ Open Comparison                │
├──────────────────────────────────────────────────────────────────────────────────────────────────┤
│ STASH · CONTEXT MENU           │ SUBMODULE · MENU               │ WORKTREE · MENU                │
│ menu 252 · 8 items             │ menu 252 · 8 items             │ menu 252 · 7 items             │
│ 2 separators                   │ 2 separators                   │ 2 separators                   │
│ Copy Stash Name key            │ no shortcut                    │ no shortcut                    │
└────────────────────────────────┴────────────────────────────────┴────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 页面 | 整页 | 1440×1400；内距 30×34；gap 14 | 标题行 + 12 张卡 |
| 栅格 | 标题下 | 3 列 `minmax(0,1fr)`；gap 22；`flex:1` | 4 行 × 3 列 |
| 卡 | 栅格内 | 宽 442.7；卡内 gap 7 | 说明行 9/600 全大写 + 控件 |
| 菜单卡 | 卡内 | 宽 252；内距 5；圆角 7 | 6 张 |
| 对话框卡 | 卡内 | 宽 = 卡宽 442.7（无自身 width）；圆角 10 | 5 张 |
| 比较卡 | 卡内 | `max-width:560`，实际 442.7；圆角 10 | 1 张 |

内容宽 1372 = 1440 − 68；列宽 442.7 = (1372 − 44) / 3。菜单固定 252 宽，卡内留 190 空白；对话框没有 width，被 flex 拉伸到 442.7。

## 2. 菜单规格

`.menu.flat`：宽 **252**（`.menu.flat` 两个类，压过文件末尾 `.menu{width:260}`）、内距 5、1px `menu-line`、圆角 7、`--shadow-menu`、`position:relative` 以留在卡内。21 的同一个菜单是 260 宽，两处不一致。

菜单项 `.mi`：24 高、内距 0×8、圆角 5、gap 6、字号 12.5；hover 换 `primary-bg` 底 + 白字。快捷键 `.sc` 10 号右对齐。分组标题 `.mlabel`：内距 4 8 3、9/600、字距 .04em、全大写。分隔 `.msep`：1px、上下 5、左右 7。禁用态 `.disabled` 有定义，本板没有用到。

`Commit filters` 把复选直接放进菜单项（`role="menuitemcheckbox"`），用的是原生 `<input type="checkbox">`，没有尺寸与对齐规格。

## 3. 对话框规格

`.dialog`：底 `content`、1px `line-strong`、圆角 10、阴影 0 14 34（.16）+ 0 2 6（.08）、`overflow:hidden`。三段固定高：头 44、脚 42，body 内距 11×14。

头：标题 12/600、副标 9 `text3`（与标题同块，上下排）、`×` 14 号靠右。脚：左 `Cancel`，弹性空隙，右主按钮。控件：`label` 11/600 块级、下距 4；`field` 26 高；`textarea` 最小高 58、11 号、内距 6×8、圆角 6（本板两处内联改成 56 与 44）；`checkrow` 22 高、11.5；`dialog-note` 9/1.45、上距 5。整套控件只有浅色版本，深色只在 21 的提交菜单上有一份。

## 4. 六张菜单卡

| 卡 | 条目（按组） | 数 | 快捷键项 |
|---|---|---|---|
| Branch · context menu | `Checkout` · `New Branch from 'main'…` · `Compare with Current Branch` ｜ `Rebase Current onto 'main'` · `Merge 'main' into Current` · `New Tag…` ｜ `Rename…` · `Delete…` · `Copy Branch Name` · `Push 'main' to origin` | 10，2 分隔 | `Copy Branch Name ⌥⌘C` |
| Remote branch · context menu | `Checkout as New Local Branch` · `Compare with Current` · `Merge into Current` · `Rebase Current onto` ｜ `Pull into Current` · `Delete Remote Branch…` · `Copy Name` | 7，1 分隔 | 无 |
| Commit filters | 标题 `Show commits`；`☑ Only my commits` · `☑ Hide merge commits` · `☐ Hide tagged commits` ｜ `Last 7 days` · `Author · Mia Chen` · `Path · src/` · `Message · hunk` ｜ `Apply Filter` | 8，2 分隔 | `⌘7`、`Apply Filter ↵` |
| Stash · context menu | `Stash Changes…` · `Stash Selected Files…` ｜ `Apply Stash` · `Pop Stash` · `Drop Stash…` ｜ `Show Diff` · `Copy Stash Name` | 8，2 分隔 | `Copy Stash Name ⌥⌘C` |
| Submodule · context menu | `Update Submodule` · `Update to Latest` · `Sync URL` ｜ `Open in New Tab` · `Commit Submodule Change` ｜ `Remove Submodule…` · `Copy Path` | 8，2 分隔 | 无 |
| Worktree · context menu | `Open in New Window` · `New Worktree…` ｜ `Remove Worktree…` · `Prune Worktrees` ｜ `Reveal in Finder` · `Copy Path` | 7，2 分隔 | 无 |

分支菜单把分支名写进文案（`'main'` 带单引号），远端菜单刻意留空宾语（`Rebase Current onto`），两条口径并存。

## 5. 五张对话框与比较卡

| 卡 | 标题 / 副标 | 控件 | 主按钮 |
|---|---|---|---|
| Reset current branch | `Reset Current Branch` / `main → 3f2a1c9` | 一句说明 + 3 个单选（`Soft` `Mixed` `Hard`，`Mixed` 选中）+ 红色注记 | `Reset`（`btn`，字色 `danger`） |
| Squash commits | `Squash 3 Commits` / `into 3f2a1c9` | 3 行提交（sha + 标题，10 号）+ `Resulting message` 文本域（预填 `Fix wrapped-line hunk alignment and split view`，最小高 56） | `Squash`（primary） |
| New branch | `New Branch` / 无副标 | `Name` = `feature/history-filters`；`From` = `3f2a1c9 · main`（只读）；`☑ Checkout after create` | `Create Branch`（primary） |
| Edit commit message | `Edit Commit Message` / 无副标 | `Message` 文本域 = `Fix diff hunk alignment on wrapped lines`；注记 `Rewords 3f2a1c9 only. Descendant commits are unchanged.` | `Save`（primary） |
| New tag | `New Tag` / 无副标 | `Tag name` = `v1.4.1`；`Target` = `3f2a1c9 · Fix wrapped-line hunk alignment`（只读）；`Message` = `Release 1.4.1`（最小高 44）；`☑ Annotated tag` | `Create Tag`（primary） |
| Branch comparison | `Compare Branches` + 右端 `⌘⇧C` | `main` ↔ `feature/diff-view`；`12 commits ahead` / `0 behind` / `14 files changed`；两个按钮 | `Open Comparison`（`btn small primary`，链 07）+ `Swap` |

Reset 的三个选项各自带一句后果：`Move HEAD only. Keep the index and working tree.` / `Move HEAD and reset the index. Keep working-tree changes.` / `Discard the index and all uncommitted changes.`，下方红字再确认一次 `Hard reset permanently discards uncommitted changes.`

## 6. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 菜单种类 | 分支 / 远端 / 过滤 / stash / 子模块 / worktree | 6 种各一张 |
| 对话框种类 | reset / squash / 分支 / 改信息 / 标签 | 5 种各一张 |
| 单选 | Soft / Mixed / Hard | 只画 Mixed 选中 |
| 复选 | 勾 / 不勾 | 菜单里 2 勾 1 不勾；两个对话框各 1 个，都是勾选 |

```
定位：每张卡都是静态规格，卡与卡之间没有跳转
入链：20 的 New Branch… / New Tag… / Squash Commits… 三条命令 href 到本板
出链：只有 Open Comparison → 07
```

## 7. 画板没覆盖的组合（待补设计）

1. **尺寸**：对话框没有 width 与最大高，板上靠栅格 442.7 撑开；真实弹窗多宽、内容超高如何滚动、是否居中在窗口都没写。
2. **校验与错误**：分支名非法、tag 已存在、消息为空、重名远端分支都没有错误态；主按钮也从不置灰。
3. **确认类对话框**：`Delete…`、`Rename…`、`Drop Stash…`、`Remove Submodule…`、`Remove Worktree…` 只有菜单项，没有二次确认框。
4. **复选规格**：过滤菜单用原生 checkbox，没有尺寸、勾选样式与键盘行为；macOS 菜单惯例是左侧 `✓`。
5. **改选入口**：`From` 与 `Target` 是只读字段，没有像 01 那样的 `Choose…` 或分支选择器。
6. **快捷键冲突**：`⌥⌘C` 同时给 `Copy Branch Name` 与 `Copy Stash Name`，21 的 `Copy SHA` 也是 `⌥⌘C`；`⌘⇧C`（比较）与 20 的 `⌘⇧B` / `⌘⇧T` 没有统一规则。
7. **深层交互与深色**：菜单项与对话框之间只靠卡标题暗示，没有任何 `href`；`Branch comparison` 既不是菜单也不是对话框，却占一张卡。6 个菜单与 5 个对话框也没有深色版本。

## 8. 按钮与控件清单

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | Reset 脚左 | `Cancel` | `btn` 24 高 / 12 | 关闭 | 常驻 |
| 2 | Reset 脚右 | `Reset` | `btn` 24 高，字色 `danger` | 执行重置 | 没有禁用态 |
| 3 | Squash 脚左 | `Cancel` | `btn` 24 高 | 关闭 | 常驻 |
| 4 | Squash 脚右 | `Squash` | `btn primary` 24 高 | 压缩 3 个提交 | 没有禁用态 |
| 5 | New Branch 脚左 | `Cancel` | `btn` 24 高 | 关闭 | 常驻 |
| 6 | New Branch 脚右 | `Create Branch` | `btn primary` 24 高 | 建分支并按勾选检出 | 没有禁用态 |
| 7 | Edit message 脚左 | `Cancel` | `btn` 24 高 | 关闭 | 常驻 |
| 8 | Edit message 脚右 | `Save` | `btn primary` 24 高 | 保存改写后的信息 | 没有禁用态 |
| 9 | New Tag 脚左 | `Cancel` | `btn` 24 高 | 关闭 | 常驻 |
| 10 | New Tag 脚右 | `Create Tag` | `btn primary` 24 高 | 建标签 | 没有禁用态 |
| 11 | 比较卡 | `Open Comparison` | `btn small primary` 22 高 | 去 07 | 链 `CompareRefs.dc.html` |
| 12 | 比较卡 | `Swap` | `btn small` 22 高 | 交换两侧分支 | 常驻 |
| 13 | 5 个对话框头右 | `×` | 14 号文本，不是 `button` | 关闭 | 5 处，均无 `aria-label` |

- **菜单项**：6 组共 48 项、11 条分隔，文案与分组见 §4；菜单项形态统一 24 高、12.5 号，`role=menuitem` / `menuitemcheckbox`。
- **复选**：`Checkout after create`、`Annotated tag`、`Only my commits`、`Hide merge commits` 勾选，`Hide tagged commits` 未勾。
- **单选**：`Soft` / `Mixed` / `Hard`，`Mixed` 选中。

## 9. 文案要点

- 菜单项用动词开头，宾语用单引号包分支名（`Push 'main' to origin`），远端菜单省宾语以求短。
- 对话框副标只写对象与目标提交（`main → 3f2a1c9`、`into 3f2a1c9`），不重复标题。
- 说明句交代后果而不是复述控件：`Creates a new commit and replaces the selected 3 commits.`、`Rewords 3f2a1c9 only. Descendant commits are unchanged.`；Reset 是唯一用红色文字按钮的破坏性操作，`Hard` 的注记再用 `danger` 强调一次。

## 10. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 07 Compare refs | 比较卡的 `Open Comparison`（唯一 `href`） |
| 20 Command palette | 三条命令（New Branch… / New Tag… / Squash Commits…）从 20 链到本板 |
| 21 Dark & context menu | 同一族菜单；21 是深色 260 宽，本板菜单 252 宽 |
| 04 Commits | 这些菜单与对话框的触发上下文（提交行、分支行），但板上没有回链 |
| 09 Refs manager / 30 Refs | 分支、远端、标签、stash 的完整列表页，菜单是它们的操作层 |
