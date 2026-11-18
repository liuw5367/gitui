# 命令面板 — 布局、状态与按钮

- **画板**：20 Command palette（有 9 条结果 · 选中第一条 `gitui` · 空输入；无过滤输入、无空结果、无参数级命令）
- **来源**：`designs/graft-git/project/CommandPalette.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

本页是**主窗口加模态浮层**：底层是 04 的提交视图（中列 Split diff、右列 `Changes` 检查器），`.palette-layer` 绝对定位铺满整个窗口（含标题栏），z 60、`rgba(20,22,28,.18)` + 2px 模糊；面板 **620 宽**、顶端距窗口 92、水平居中，高 512。

## 1. 骨架

```
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ ● ● ●  [gitui x] [docs-site x] [atlas-web x] [+]     ● Fetched 2m ago  [gear]                │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ [=] Fetch Pull Push | Branch Stash   [search ⌘F] | History Changes 12 | Agent [=]            │
├────────────────┬──────────────────────────────────────────────────┬──────────────────────────┤
│ Branches 5     │    ┌────────────────────────────────────────────────┐                       │
│ ▣ main ✓       │    │ [o] Search repositories or run a command   esc │                       │
│ ▣ feature/     │    ├────────────────────────────────────────────────┤                       │
│ Remotes 2      │    │ OPEN REPOSITORIES                              │                       │
│   origin 8     │    │ [b] gitui  ~/Dev/gitui  main        Enter      │                       │
│ Tags 3         │    │ [f] docs-site        main                      │                       │
│ ▣ v1.4.0       │    │ [f] atlas-web        feature/hero              │                       │
│ Stashes 2      │    │ RECENT                                         │                       │
│ Worktrees 3    │    │ [f] dotfiles         main                      │                       │
│                │    │ [f] spectra          main                      │                       │
│                │    │ COMMANDS                                       │                       │
│                │    │ [b] New Branch…      Command · repository      │                       │
│                │    │ [t] New Tag…         Command · repository      │                       │
│                │    │ [s] Squash Commits…  Command · history         │                       │
│                │    │ [g] Open Settings…   Command · global          │                       │
│                │    ├────────────────────────────────────────────────┤                       │
│                │    │ ↑↓ Navigate  Enter Open  esc Close     ⌘P      │                       │
│                │    └────────────────────────────────────────────────┘                       │
│                │                                                  │                          │
├────────────────┴──────────────────────────────────────────────────┴──────────────────────────┤
│ main  ● 2 ahead · 0 behind        12 changes  UTF-8  LF  TypeScript                          │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 遮罩 | 整窗 | `inset:0`；z 60 | 压暗并模糊底层，标题栏与工具条仍可辨认 |
| 面板 | 距窗口顶 92、水平居中 | 宽 620（`max-width:calc(100% - 48px)`）；圆角 12；1px `line-strong` | 输入 50 + 列表 430 + 页脚 30 + 边框 2 |
| 输入区 | 面板顶 | 高 50；内距 0×14；gap 9 | 搜索图标 15、输入 14 号、`esc` 键帽 9 |
| 结果列表 | 输入下 | 上下内距 5；最大高 430（内联，样式表 390） | 3 个分组标题 23 + 9 行 44 |
| 页脚 | 面板底 | 高 30；内距 0×12；gap 10；9 号 | 三条键位提示 + 右端 `⌘P` |

面板左沿 385 = (1390 − 620) / 2（相对窗口的 padding box，1390 = 1392 − 2 边框）；高 512 = 50 + 430 + 30 + 2；430 的可视高度依次容下 5 内距 + 23 标题 + 3×44 + 23 + 2×44 + 23 + 3×44 = 426，第 9 行 `Open Settings…` 只剩 4px。

## 2. 遮罩与面板

遮罩只用 .18 的浅压暗（14 是 .34），底层的 diff 与检查器仍然读得清，用来提示这是窗口级浮层。面板底 `menu-bg`、1px `line-strong`、圆角 12、阴影 0 30 80（.34）+ 0 4 14（.16）、`overflow:hidden`。

面板按整个窗口居中，不与中列对齐：它属于窗口级浮层，不属于某个栏目。

## 3. 输入区

50 高、内距 0 14、gap 9、下框 1 `line`：搜索图标 15 `text3`、输入 14 号（`text`，占位同色 `text3`）、右端 `esc` 键帽（9 号、内距 1×5、圆角 4、1px `line`）。输入框没有可见标签，只有 `aria-label="Search repositories or run a command"`。

## 4. 结果列表

三个分组标题 `palette-label`：23 高、内距 0×8、9/600、字距 .05em、全大写 —— `OPEN REPOSITORIES` / `RECENT` / `COMMANDS`。

行 `palette-row` 44 高、内距 0×9、圆角 7、gap 9：左端 26×26 圆角 6 图标块（`panel` 底 + 1px `line`；当前行改 `content` 底 + `line-strong` 框 + `accent` 图标 14）；中间两行（名称 12/600 + 路径或分组 mono 9.5，行距 1）；右端标签 `pb`（9 号、内距 1×5、圆角 4、`panel` 底 + 1px `line`）或 `↵`（10 号、左距 4）。

九条内容：`gitui` / `~/Developer/gitui` / `main` + `↵`；`docs-site` / `~/Work/docs-site` / `main`；`atlas-web` / `~/Work/atlas-web` / `feature/hero`；`dotfiles` / `~/.dotfiles` / `main`；`spectra` / `~/Labs/spectra` / `main`；`New Branch…` / `Command · repository` / `⌘⇧B`；`New Tag…` / `Command · repository` / `⌘⇧T`；`Squash Commits…` / `Command · history` / `⌥⌘S`；`Open Settings…` / `Command · global` / `⌘,`。

行 hover 与当前行同色（`sel`），没有第三态。

## 5. 页脚

30 高、内距 0 12、gap 10、9 号 `text3`、上框 1、底 `panel`：`↑` `↓` Navigate · `↵` Open · `esc` Close · 弹性空隙 · `⌘P`。键帽 `kbd` 8.5 号、内距 0×4、圆角 3、行高 13、`content` 底 + 1px `line`。

## 6. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| 输入 | 空 / 有文本 | 只画空 |
| 选中行 | 一行 / 无 | 只画第 1 行 |
| 结果 | 9 条 / 0 条 | 只画 9 条 |
| 分组 | 仓库 / 最近 / 命令 | 三组都在 |
| 主题 | 浅色 / 深色 | 只画浅色 |

```
打开：⌘P（25 Settings · Shortcuts 的 Toggle Command Palette）
切换：↑ ↓ 移选中；输入过滤；↵ 执行当前行
关闭：esc；点遮罩是否关闭板上没有画
```

### 当前实现行为

空查询时先列 5 条常用命令，再列最多 4 条最近仓库；总行数为 9。输入后按「命令、当前打开、最近、已启用扫描」顺序分组，扫描仓库仍由 Git 设置中的搜索资格控制。路径按规范化仓库根去重；同一仓库只显示一次。所有行都能用键盘上下移动并由 Enter 执行，筛选、渲染与高亮使用同一结果顺序。

## 7. 画板没覆盖的组合（待补设计）

1. **输入后的样子**：过滤规则、命中高亮、排序都没有画，只有空输入的全量列表。
2. **空结果**：零匹配时的空态文案与是否回退到「最近」没画。
3. **命令参数**：`New Branch…` 这类带 `…` 的命令选中后是关面板再去 22，还是就地展开参数输入，没定。
4. **滚动**：9 条已超出 430 的可视高度，滚动条样式与首尾留白没画。
5. **焦点**：当前行与 hover 同一底色，键盘焦点行没有额外标记；打开瞬间的默认选中规则也没写。
6. **键位说明重复**：`esc` 在输入框右端与页脚各一次；`⌘P` 只在页脚出现，而它正是打开这个面板的键。
7. **名称不一致**：面板 `aria-label="Switch repository"`，输入框 `aria-label="Search repositories or run a command"`，12 Agent 的输入区又写 `⌘K commands`。
8. **命令范围**：`Command · repository / history / global` 三档只在副标里出现，哪档命令在哪个上下文可用没有定义。

## 8. 按钮与链接清单

9 行结果逐行列出；底层被遮罩压住的 04 按钮不计入。

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | 结果区 | `gitui` + `~/Developer/gitui` + `main` + `↵` | 行 44 高，当前 `on` | 切到该仓库（链 04） | 唯一带 `↵` 与 `href` 的行 |
| 2 | 结果区 | `docs-site` + `~/Work/docs-site` + `main` | 行 44 高 | 切仓库 | `<button>`，无 `↵` |
| 3 | 结果区 | `atlas-web` + `~/Work/atlas-web` + `feature/hero` | 行 44 高 | 切仓库 | 同上 |
| 4 | 结果区 | `dotfiles` + `~/.dotfiles` + `main` | 行 44 高 | 切仓库（Recent 组） | 同上 |
| 5 | 结果区 | `spectra` + `~/Labs/spectra` + `main` | 行 44 高 | 切仓库 | 同上 |
| 6 | 结果区 | `New Branch…` + `Command · repository` + `⌘⇧B` | 行 44 高 | 打开 22 | 链 `Menus.dc.html` |
| 7 | 结果区 | `New Tag…` + `Command · repository` + `⌘⇧T` | 行 44 高 | 打开 22 | 链 `Menus.dc.html` |
| 8 | 结果区 | `Squash Commits…` + `Command · history` + `⌥⌘S` | 行 44 高 | 打开 22 | 链 `Menus.dc.html` |
| 9 | 结果区 | `Open Settings…` + `Command · global` + `⌘,` | 行 44 高 | 打开 23 | 链 `Settings.dc.html` |

- 输入框与 `esc` 键帽不是按钮；页脚的 `↑↓` / `↵` / `esc` / `⌘P` 是键帽提示，整行不可点。
- 面板 `role="dialog"` + `aria-modal="true"`，遮罩本身没有点击处理。

## 9. 输入与选项

| 控件 | 形态 | 画板取值 |
|---|---|---|
| 搜索框 | 单行，14 号，无边框 | 空值；占位 `Search repositories or run a command` |
| `esc` 键帽 | 提示 | 9 号、内距 1×5、圆角 4 |
| 结果分组 | 三段标题 | Open repositories / Recent / Commands |
| 键位提示 | 页脚四条 | `↑↓` Navigate、`↵` Open、`esc` Close、`⌘P` |

## 10. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 22 Menus & dialogs | 三条命令（New Branch… / New Tag… / Squash Commits…）共 3 处 `href` |
| 23 Settings | 齿轮 + `Open Settings…` 共 2 处 `href` |
| 04 Commits | `gitui` 行与底层 `Commits` chip 共 2 处 `href`；面板就压在 04 的提交视图上 |
| 18 Files at commit | 底层检查器的 `Files` 页签链到它 |
| 10 Staging & commit / 12 Agent panel / 01 Welcome | `Changes 12` 分段项 / `Agent` / 页签 `+` |
| 25 Settings · Shortcuts | 把本页的开关定义为 `⌘P`，与本板页脚的 `⌘P` 一致 |
