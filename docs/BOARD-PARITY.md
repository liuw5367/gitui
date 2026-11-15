# 逐板对照：画板控件 → 现有实现 → 缺口

**Status**: v1, 2026-11-15
**覆盖**: P2–P7 的 21 块画板（04–09、10–19、20–22、23–27）
**用途**: 每组迁移前先查本表，避免「凭印象判断缺什么」。画板一侧是机械提取的 class 词表，实现一侧是代码实测。

数据口径：画板词表由 `designs/graft-git/project/*.dc.html` 的 `class` 属性统计（只取控件类，忽略布局类）；实现一侧的「字号 <11 / 裸按压 / 手写 mono」是文件级实测，见计划文件第 13 轮的量化记录。

---

## 1. 逐板对照

### P2 Review 组

| 板 | 画板要求的主要控件（词表） | 现有实现 | 缺口 |
|---|---|---|---|
| 04 History & diff | `side-row`×18、`commit`×7、`chipbtn`×4、`hist-bar`、`commit-cols`、`diffbar` | `workspace/history.rs`（1143 行）+ `workspace/diff.rs`（544 行），自绘列头与行、自绘图线、自绘 diff | 表格换 `ui::table`；diff 换 `diff_view`；筛选浮层换 `ui::dialog`/`ui::field` |
| 05 History filters | 同 04 + `field`×2 | 同上，过滤浮层自绘 | 同上；过滤三字段换 `ui::field` |
| 06 Multi-commit diff | 同 04 + `selection-bar`、`badge-soft` | 多选对比已实现（对比栏自绘） | 对比栏换 `ui::banner` + `ui::badge` |
| 07 Compare refs | 同 04 + `seg`×2 | 对比引用控件自绘 | 两个分段换 `ui::segmented` |
| 08 Commit info | 同 04 + `commit-title` | `workspace/inspector.rs`（564 行），三页签已换 `ui::tabs` | 键值行换 `ui::field`/`ui::key_value`（后者未建，需补） |
| 09 Refs manager | `refs-row`×6、`btn`×6、`seg`、`chipbtn` | `screens/refs_view.rs`（770 行），自绘分段表 + 每行菜单 | 表换 `ui::table`；行菜单换 `ui::menu`（未建） |

### P3 Worktree 组

| 板 | 画板要求的主要控件 | 现有实现 | 缺口 |
|---|---|---|---|
| 10 Staging & commit | `checkrow`×3、`btn`×5、`mi`×7、`hook-step`×4 | `screens/changes_view.rs`（1049 行，字号 <11 有 20 处）、`workspace/commit.rs`（485 行） | 文件行换 `ui::row`/`ui::table`；勾选换 `ui::check_row`；`hook-step` 换 `ui::section`；钩子日志换 `ui::code` |
| 11 Changelists | `checkrow`×3、`btn`×2、`empty`×12 | 变更清单弹层自绘 | 弹层换 `ui::dialog` |
| 12 Agent panel | `agent-ic`×3、`agent-scope`×2、`badge-soft`×2、`btn`×2 | `workspace/agent.rs`（479 行） | 步骤行换 `ui::row`；范围块换 `ui::card`；徽章换 `ui::badge` |
| 13 Agent permissions | `seg`×3、`btn`×7、`agent-ic`×4 | 权限三态在设置页与面板各一份 | 三态换 `ui::segmented`，两处统一 |

### P4 Operations / Details 组

| 板 | 画板要求的主要控件 | 现有实现 | 缺口 |
|---|---|---|---|
| 14 Merge conflict | `btn`×10、`seg`×2、`merge-pane`×3、`merge-code`×3 | `screens/conflict_view.rs`（505 行），整屏 overlay + 自绘三栏 | 换 `ui::dialog`（或 sheet）+ `ui::split`（未建）；三栏代码区换 `ui::code` |
| 15 Sync & auth | `checkrow`×3、`btn`×4 | `screens/sync_dialog.rs`（588 行），自绘浮层与选项 | 换 `ui::dialog` + `ui::tabs` + `ui::check_row` |
| 16 Repository states | `pill`×18、`mini-*`（8 类 ×6） | `shell/mod.rs` 三条横幅各写一套 | 收成一个 `ui::banner`（三态 + 动作槽） |
| 17 Reflog & undo | `commit`×3、`btn`×3、`kbd` | `screens/refs_view.rs` 的 Reflog 段 | 表换 `ui::table`；键帽换 `ui::kbd` |
| 28 Sync · Fetch | `checkrow`×3、`btn`×4 | 同步弹窗 Fetch 选项 | 同 15 |
| 29 Sync · Pull | `seg`×2、`checkrow`×2、`btn`×4 | 同步弹窗 Pull 三选 | 同 15；三选换 `ui::segmented` |
| 30 Refs · Tags/Remotes/Stashes | `chipbtn`×5、`badge-soft`×3、`seg` | 引用页各段 | 同 09 |

### P5 Files 组

| 板 | 画板要求的主要控件 | 现有实现 | 缺口 |
|---|---|---|---|
| 18 Files at commit | `btn`×2、`seg`、`badge-soft` | 检查器 Files 页 | 视图切换换 `ui::segmented`；`+/−` 换 `ui::badge` |
| 19 Blame & file history | `btn`×2、`badge-soft`、`statusbar` | 只有 320 宽侧栏 blame，画板是主区宽版 | 需补主区宽版 blame（`ui::table`） |

### P6 Navigation 组

| 板 | 画板要求的主要控件 | 现有实现 | 缺口 |
|---|---|---|---|
| 20 Command palette | `palette-row`×9、`palette-label`×3、`chipbtn`×3 | `screens/palette.rs`（377 行），自绘列表与高亮 | 换 `nsh_base::command` + `ui::row` |
| 21 Dark & context menu | `mi`×15 | 6 处 `popup_menu_at` 各写一套菜单项 | 统一 `ui::menu`（未建），深色核对 |
| 22 Menus & dialogs | `mi`×45、`menu`×6、`dialog`×5、`field`×4 | `screens/form_dialog.rs` + `alert_dialog` | 统一 `ui::dialog` + `ui::field`；45 个菜单项逐项核对 |

### P7 Settings 组

| 板 | 画板要求的主要控件 | 现有实现 | 缺口 |
|---|---|---|---|
| 23 Settings | `radio-pills`、`btn`×2 | `screens/settings.rs`（1876 行，字号 <11 有 33 处、裸按压 10 处），侧栏与行自绘 | 侧栏换 `ui::tree`/`ui::list`；行换 `ui::setting_row`（未建）；主题三态换 `ui::segmented` |
| 24 Settings · Git | `btn`×7 | 同文件 git_page | 字段换 `ui::field`；按钮换 `ui::button` |
| 25 Settings · Shortcuts | `kbd`×37、`btn`×15 | 同文件 shortcuts_page，快捷键表自绘 | 表换 `ui::table`；键帽换 `ui::kbd` |
| 26 Settings · About | `btn`×1 | 同文件 about | 外链换 `ui::button`（ghost） |
| 27 Settings · Credentials | `btn`×9 | 同文件 credentials_page | 只读行换 `ui::row` |

---

## 2. 仍需新建的 ui 组件（按首次使用排序）

| 组件 | 首次需要 | 基于 | 说明 |
|---|---|---|---|
| ~~`key_value`~~ 已就绪（待接入） | 08 | `view` + `text` | 左标签 + 右值的两列行 |
| ~~`menu`~~ 构造器已就绪（待接入） | 09 / 21 / 22 | 系统 `NSMenu` | 菜单项分四类构造；外观由系统决定 |
| ~~`split`~~ 已就绪（待接入） | 14 | `nsh_base` 的 `h_resizable`/`v_resizable` | 检查器与冲突三栏 |
| ~~`setting_row`~~ 已就绪（待接入） | 23 | `view` + `text` | 标题 + 说明 + 右侧控件 + 分隔线 |
| ~~`tree`~~ 已就绪（待接入） | 23 | `nsh_base::tree` | 设置侧栏；文件树也可复用 |
| `command` | 20 | `nsh_base::command` | 命令面板 |
| `diff_surface` | 04 | `nsh_base::diff` | diff 与其开关 |
| `toolbar` | 04 | `nsh_base::toolbar` | 工作区工具栏 52 高 |
| `tab_strip` | 04 | `ui::tabs` 变体 | 仓库页签（⌘1…9、溢出菜单） |

## 3. 用法

1. 每组开工前查本表第 1 节，确认该组要新建哪些组件（第 2 节），先建组件再改页面。
2. 组件落地后，把对应行从第 2 节删除，并在 `UI-KIT.md` 第 3 节登记。
3. 页面迁完，把文件加进 `tools/ui-migrated.txt`，让 `tools/check-ui.sh` 开始约束它。
