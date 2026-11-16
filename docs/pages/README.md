# pages — 逐页说明

一个页面一份文档，讲清楚三件事：**长什么样、有哪几个状态、每个按钮做什么**。内容以
`designs/graft-git` 的画板为准，不从实现里反向读——实现与画板的差异统一记在
[../design/DEVIATIONS.md](../design/DEVIATIONS.md)，这里不重复。

与 `../design/` 的分工：`design/` 是跨页面的规则（表面层级、字级、token、组件选型）；
`pages/` 是单页的版式与操作。两者冲突时，视觉规则以 `design/` 为准，版式以画板为准。

## 写法约定

1. **一页一份 md**，文件名用画板的主题名（`welcome.md`），不按编号命名——编号会变。
2. 开头写清**画板编号与文件名**，并声明只按画板描述。
3. **§1 骨架**：一张 ASCII 总图 + 区域表。表里给该区域的尺寸（画板值，如「宽 312」
   「高 47」）与内容；不要写实现里的常量名。
4. **逐区展开**：每个区域一节，画出内部结构，标出字号、行高、内距这类**画板能量到的值**。
5. **状态**：先列「几条独立的轴」，再标出画板覆盖了哪几个组合；把**没画的组合**单列一节，
   那是待补的设计，不是实现的自由。
6. **按钮表**：一行一个可点元素，列 位置 / 文案 / 形态 / 作用 / 随状态如何变化；
   图标按钮写无障碍名。整行可点的行、分段控件单独说明，别混进按钮计数。
7. **输入表**：字段名、形态、占位文案、默认值。
8. **与其它画板的连接**：哪些按钮跳到哪块板。

约定不是死的：某一页没有多状态，就把状态节写成「本页无状态分支」。但**骨架、按钮表、
未覆盖组合**三节每页都要有。

## 画图约定

- 框线内只用单宽字符，**不放汉字**（汉字是双宽，会把框挤歪）；中文说明写在图外。
- 同一个框内所有带 `│` 的行显示宽度一致；通栏行（标题栏、工具条、状态栏）本来就是整条，
  没有内部竖线。
- 改完图跑一遍检查（East Asian width：W/F 记 2）：

```python
import re, pathlib, unicodedata
def dw(s): return sum(2 if unicodedata.east_asian_width(c) in 'WF' else 1 for c in s)
for f in sorted(pathlib.Path('docs/pages').glob('*.md')):
    for i, block in enumerate(re.findall(r'```\n(.*?)```', f.read_text(), re.S), 1):
        run = []
        def flush(run):
            if len(run) > 1 and len({dw(l) for l in run}) > 1:
                print('宽度不一致', f.name, i, [(dw(l), l[:40]) for l in run])
        for l in block.rstrip('\n').split('\n'):
            if l and l[0] in '┌│├└': run.append(l)
            else: flush(run); run = []
        flush(run)
```

## 页面清单

画板共 32 块，成文 30 份：01b / 01c 合进 `welcome.md`，其余各一份；顺序取自 `project/canvas.json`。

| 组 | 画板 | 文档 |
|---|---|---|
| Start | 01 Welcome | [welcome.md](welcome.md)（含 01b、01c 两个状态） |
| Review | 04 Commits | [workspace.md](workspace.md) |
| Review | 04b File diff (focused) | [file-diff-focus.md](file-diff-focus.md) |
| Review | 05 History filters | [history-filters.md](history-filters.md) |
| Review | 06 Multi-commit diff | [multi-commit-diff.md](multi-commit-diff.md) |
| Review | 07 Compare refs | [compare-refs.md](compare-refs.md) |
| Review | 08 Commit info | [commit-info.md](commit-info.md) |
| Review | 09 Refs manager | [refs-manager.md](refs-manager.md) |
| Worktree | 10 Staging & commit | [changes.md](changes.md) |
| Worktree | 11 Changelists | [changelists.md](changelists.md) |
| Worktree | 12 Agent panel | [agent.md](agent.md) |
| Worktree | 13 Agent permissions | [agent-permissions.md](agent-permissions.md) |
| Operations | 14 Merge conflict | [merge-conflict.md](merge-conflict.md) |
| Operations | 15 Sync & auth | [sync.md](sync.md) |
| Operations | 16 Repository states | [repository-states.md](repository-states.md) |
| Operations | 17 Reflog & undo | [reflog.md](reflog.md) |
| Files | 18 Files at commit | [commit-files.md](commit-files.md) |
| Files | 19 Blame & file history | [blame-history.md](blame-history.md) |
| Navigation | 20 Command palette | [command-palette.md](command-palette.md) |
| Navigation | 21 Dark & context menu | [context-menu.md](context-menu.md) |
| Navigation | 22 Menus & dialogs | [menus.md](menus.md) |
| Settings | 23 Settings | [settings.md](settings.md) |
| Settings | 23b Settings · Agent | [settings-agent.md](settings-agent.md) |
| Settings | 24 Settings · Git | [settings-git.md](settings-git.md) |
| Settings | 25 Settings · Shortcuts | [settings-shortcuts.md](settings-shortcuts.md) |
| Settings | 26 Settings · About | [settings-about.md](settings-about.md) |
| Settings | 27 Settings · Credentials & tools | [settings-credentials.md](settings-credentials.md) |
| Details | 28 Sync · Fetch | [sync-fetch.md](sync-fetch.md) |
| Details | 29 Sync · Pull | [sync-pull.md](sync-pull.md) |
| Details | 30 Refs · Tags, Remotes, Stashes | [refs-other.md](refs-other.md) |

01b / 01c 是 01 的两个状态（空列表 + Open、有仓库 + Init），写在 `welcome.md` 里；
28 / 29 是 15 的两个分支态、30 是 09 的另外三类引用，各自单独成文并在开头写明与姐妹板的差异。

16 是状态画廊、22 是菜单与对话框规格页，它们各自成文，但结构按「卡片/控件清单」写，
不按单页流程写；两者与 `../design/UI-KIT.md` 的组件层互为参照。

## 待补的设计

每份文档末尾都有一节「画板没覆盖的组合」。这些是**画板层面缺的内容**，不是实现自由：
新增画板前先看那一节，避免同一状态在实现里各写各的。
