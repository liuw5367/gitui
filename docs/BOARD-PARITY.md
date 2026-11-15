# 逐板对照：画板控件 → 现有实现 → 缺口

**Status**: v2, 2026-11-15（P1–P4、P7 迁移完成后更新）
**覆盖**: P2–P7 的 21 块画板（04–09、10–19、20–22、23–27）
**已登记迁移的文件**: 见 `tools/ui-migrated.txt`（14 个文件，由 `guard_tests` 强制）
**用途**: 每组迁移前先查本表，避免「凭印象判断缺什么」。画板一侧是机械提取的 class 词表，实现一侧是代码实测。

数据口径：画板词表由 `designs/graft-git/project/*.dc.html` 的 `class` 属性统计（只取控件类，忽略布局类）；实现一侧的「字号 <11 / 裸按压 / 手写 mono」是文件级实测，见计划文件第 13 轮的量化记录。

---

## 1. 逐板对照

### P2 Review 组

| 板 | 画板要求的主要控件（词表） | 现有实现 | 缺口 |
|---|---|---|---|
| 04 History & diff | `side-row`×18、`commit`×7、`chipbtn`×4、`hist-bar`、`commit-cols`、`diffbar` | `history.rs`（1114 行）已换 `ui::table`（单列自绘图线/摘要/作者/时间）；`diff.rs`（257 行）正文已换 `ui::diff_surface` | ✅ 已就绪。历史表曾整行不可见的回归已修（列宽需给足 + render 根节点用显式行高） |
| 05 History filters | 同 04 + `field`×2 | 过滤浮层仍在 `shell`/`history` 内 | 过滤三字段换 `ui::field`（未完） |
| 06 Multi-commit diff | 同 04 + `selection-bar`、`badge-soft` | 多选对比已实现，对比栏已用 `ui::button` 变体表达选中 | ✅ 基本就绪；`selection-bar` 未单独抽组件 |
| 07 Compare refs | 同 04 + `seg`×2 | 对比引用选择片已换 `ui::button`（Secondary/Ghost） | ✅ 已就绪 |
| 08 Commit info | 同 04 + `commit-title` | `inspector.rs`（556 行），三页签 `ui::tabs`、Files 页 `ui::table` | ✅ 已就绪（`ui::key_value` 已建） |
| 09 Refs manager | `refs-row`×6、`btn`×6、`seg`、`chipbtn` | `refs_view.rs`（790 行），6 处菜单已换 `ui::menu` | ✅ 已就绪；`menu_row` 仍是自绘行（显示用，非表格） |

### P3 Worktree 组

| 板 | 画板要求的主要控件 | 现有实现 | 缺口 |
|---|---|---|---|
| 10 Staging & commit | `checkrow`×3、`btn`×5、`mi`×7、`hook-step`×4 | `changes_view.rs`（1049 行）已换 `ui::table`（目录标题为表格行，方案 A）+ `ui::check::indicator_action`；`commit.rs` 已换 `ui::button`/`ui::field` | ✅ 已就绪。**方向修正**：变更清单是锚定式 popover，不适用 `ui::dialog` |
| 11 Changelists | `checkrow`×3、`btn`×2、`empty`×12 | 变更清单弹层已用 `ui::row` + `ui::button` + `ui::text_input` | ✅ 已就绪 |
| 12 Agent panel | `agent-ic`×3、`agent-scope`×2、`badge-soft`×2、`btn`×2 | `agent.rs`（427 行），步骤按钮与权限循环已换 `ui::button`，「Add step」用 `ui::menu::menu_button` | ✅ 已就绪 |
| 13 Agent permissions | `seg`×3、`btn`×7、`agent-ic`×4 | 权限三态在设置页与面板各用 `ui::button`（同一写法） | ✅ 已就绪（未抽 `ui::segmented`，两处都是按钮循环） |

### P4 Operations / Details 组

| 板 | 画板要求的主要控件 | 现有实现 | 缺口 |
|---|---|---|---|
| 14 Merge conflict | `btn`×10、`seg`×2、`merge-pane`×3、`merge-code`×3 | `conflict_view.rs`（487 行），字号/mono 已清零，冲突 chip 用 `ui::button` | ⚠️ 整屏 overlay 与三栏仍是自绘；`ui::split` 已建但未接入（P4 剩余） |
| 15 Sync & auth | `checkrow`×3、`btn`×4 | `sync_dialog.rs`（517 行），遮罩与吞点击用 `ui::dialog::scrim`/`modality`，选项用 `ui::button`/`ui::check_row` | ✅ 主体就绪；未换 `ui::tabs` |
| 16 Repository states | `pill`×18、`mini-*`（8 类 ×6） | 三条横幅已收敛到 `ui::feedback::banner_sized`：操作条（Accent/28）、仓库状态四个分支（Neutral/26 ×3、Highlight/26 ×1） | ✅ 已就绪。`sync_banner` 例外：消息是 git 输出，用 `CODE` 等宽字号，改成横幅的 `SECONDARY` 会丢对齐 |
| 17 Reflog & undo | `commit`×3、`btn`×3、`kbd` | `refs_view.rs` 的 Reflog 段已用 `ui::menu`；撤销确认走 `ui::dialog::confirm_destructive` | ✅ 基本就绪 |
| 28 Sync · Fetch | `checkrow`×3、`btn`×4 | 同步弹窗 Fetch 选项用 `ui::check_row` | ✅ 已就绪 |
| 29 Sync · Pull | `seg`×2、`checkrow`×2、`btn`×4 | Pull 三选用 `ui::button` 变体 | ✅ 已就绪（未抽 `ui::segmented`） |
| 30 Refs · Tags/Remotes/Stashes | `chipbtn`×5、`badge-soft`×3、`seg` | 引用页各段已用 `ui::menu`/`ui::button` | ✅ 基本就绪 |

### P5 Files 组

| 板 | 画板要求的主要控件 | 现有实现 | 缺口 |
|---|---|---|---|
| 18 Files at commit | `btn`×2、`seg`、`badge-soft` | 检查器 Files 页已换 `ui::table` | ✅ 已就绪；视图切换仍是自绘分段（P5 剩余） |
| 19 Blame & file history | `blame-gutter`×16、`blame-src`×16、`blame-line`×16、`bh`/`ba`/`nm`×18、`tbtn`×6、`side-head`、`statusbar` | `inspector.rs` 的 `blame_body`（侧栏 320 宽）已有同样的三段结构：`short`(52) + `line` + `text`，数据来自 `tab.blame`（`short`/`author`/`timestamp`/`line`/`text` 齐备） | ❌ **主区宽版缺失**。不是样式问题：主区只有 4 个 `Mode`（History/Changes/Refs/Agent），没有 blame 入口；要加宽版必须先决定它挂在哪里（新增第 5 个 Mode，还是作为 Changes 的子视图）。画板还有 `bh`/`ba`（文件头/作者栏）与 `statusbar`，现有侧栏版没有 |

### P6 Navigation 组

| 板 | 画板要求的主要控件 | 现有实现 | 缺口 |
|---|---|---|---|
| 20 Command palette | `palette-row`×9、`palette-label`×3、`chipbtn`×3 | `palette.rs`（347 行），列表项已换 `ui::row`、输入框 `ui::text_input` | ⚠️ 遮罩仍是同层 on_press（面板顶部对齐，`scrim` 会改位置）；未用 `nsh_base::command` |
| 21 Dark & context menu | `mi`×15 | 菜单项统一走 `ui::menu::{action,danger,divider}` | ✅ 已就绪 |
| 22 Menus & dialogs | `mi`×45、`menu`×6、`dialog`×5、`field`×4 | `form_dialog.rs` 用 `ui::dialog::scrim`/`modality`/`ui::text_input`；确认框走 `confirm_destructive` | ✅ 主体就绪；45 个菜单项未逐项核对 |

### P7 Settings 组

| 板 | 画板要求的主要控件 | 现有实现 | 缺口 |
|---|---|---|---|
| 23 Settings | `radio-pills`、`btn`×2 | `settings.rs`（1711 行，**0 处字号字面量、0 处裸按压**），侧栏已用 `ui::row`、区块用 `ui::section`、开关行用 `ui::check_row` | ✅ 已就绪 |
| 24 Settings · Git | `btn`×7 | 同文件 git_page，字段 `ui::field`、按钮 `ui::button` | ✅ 已就绪 |
| 25 Settings · Shortcuts | `kbd`×37、`btn`×15 | 同文件 shortcuts_page，键帽 `ui::kbd` | ✅ 已就绪；快捷键表未换 `ui::table`（数据量小，自绘可接受） |
| 26 Settings · About | `btn`×1 | 同文件 about，外链 `ui::button` Small/Secondary | ✅ 已就绪 |
| 27 Settings · Credentials | `btn`×9 | 同文件 credentials_page，只读行 `ui::check_row` | ✅ 已就绪 |

---

## 2. ui 组件就绪情况（P0–P7 之后）

`src/ui/` 现有 21 个模块：`badge button check dialog diff_surface feedback field gallery
icon key_value menu row section segmented setting_row split table tabs text_input tree`。

所有原计划要新建的组件都已落地。**仍有缺口的不是「组件没建」，而是「组件没接入」**：

| 缺什么 | 影响 | 说明 |
|---|---|---|
| ~~`ui::banner`~~ 已就绪并接入（5 处使用） | 板 16 | `banner_sized(tone, height, icon, message, actions)`；`sync_banner` 因等宽字号例外 |
| `ui::segmented` 未接入 | 板 18/23 的分段 | 组件已建，页面用 `ui::button` 循环代替（行为一致） |
| `ui::split` 未接入 | 板 14 冲突三栏 | 组件已建，冲突页仍是自绘 overlay |
| 主区宽版 blame | 板 19 | 功能缺失，不只是样式 |
| 过滤面板 `ui::field` | 板 05 | 未迁移 |
| `widgets.rs` 与 `ui` 合并 | 全局 | 97 处调用跨 20+ 文件，且图标尺寸用 8 种，需分批 |

### 旧表（保留供对照）

原计划的组件清单，均已就绪：

| 组件 | 首次需要 | 基于 | 说明 |
|---|---|---|---|
| ~~`key_value`~~ 已就绪（待接入） | 08 | `view` + `text` | 左标签 + 右值的两列行 |
| ~~`menu`~~ 构造器已就绪（待接入） | 09 / 21 / 22 | 系统 `NSMenu` | 菜单项分四类构造；外观由系统决定 |
| ~~`split`~~ 已就绪（待接入） | 14 | `nsh_base` 的 `h_resizable`/`v_resizable` | 检查器与冲突三栏 |
| ~~`setting_row`~~ 已就绪（待接入） | 23 | `view` + `text` | 标题 + 说明 + 右侧控件 + 分隔线 |
| ~~`tree`~~ 已就绪（待接入） | 23 | `nsh_base::tree` | 设置侧栏；文件树也可复用 |
| `command` | 20 | `nsh_base::command` | 命令面板 |
| ~~`diff_surface`~~ 已就绪（待接入） | 04 | `nsh_base::DiffView` | diff 与其开关 |
| `toolbar` | 04 | `nsh_base::toolbar` | 工作区工具栏 52 高 |
| `tab_strip` | 04 | `ui::tabs` 变体 | 仓库页签（⌘1…9、溢出菜单） |

## 3. 已核实的例外（不是遗漏）

以下几处保留裸 `on_press`，每一处都核实过「`ui` 层没有对应能力」，不是漏改：

| 位置 | 为什么不能换 |
|---|---|
| `shell/mod.rs` 仓库页签 | **嵌套按压**：页签本体激活 + 内部 `×` 关闭。`ui::button` 不支持嵌套 |
| `shell/mod.rs` `mode_tab` | 带 count 徽章，`ui::button` 只接文案 |
| `shell/mod.rs` 两个图标按钮 | 有 active 态（`accent_soft` 底 + 强调色图标），`ui::icon_button` 的 bg 恒透明 |
| `palette.rs` 遮罩 | 命令面板是「遮罩与面板同层 + 顶部对齐」，`scrim` 会把内容居中、改变位置 |
| `diff.rs` 图标开关 | 同 shell 的图标按钮（active 态） |
| `changes_view.rs` 行内方框 | 已用 `ui::check::indicator_action` 解决，此处留档说明它为什么需要单独原语 |

## 4. 逐板截图索引

截图在 `docs/verify/native/`，每张对应一个界面（同界面只留最新一张）。
生成方式见第 5 节。

| 截图 | 覆盖的板 | 截到的内容 |
|---|---|---|
| `gallery.png` | —（组件层） | 全部 `ui` 组件的实际外观，⌘⇧D 打开 |
| `history-final.png` | 04 / 06 / 07 | 提交行（图线、摘要、作者、相对时间）、Compare 按钮、分支标签、分支侧栏 |
| `diffview.png` | 04 / 05 | diff 正文（行号、+−）、Unified/Split 切换、折叠与换行开关；左栏文件列表 |
| `changes-final.png` | 10 / 11 | 分区标题（Unstaged 1）、文件行（方框 + 名称 + M + +1 −0）、取消全部暂存、变更清单入口 |
| `refs.png` | 09 / 17 / 30 | 分支/标签/远程/贮藏四段，每行的名称、说明与快捷提示 |
| `agent.png` | 12 / 13 | Agent 面板的标题、说明、步骤列表与权限按钮（Ask/Allow/Deny） |
| `palette.png` | 20 | 命令面板：搜索框、七行命令、每行左侧放大镜与右侧快捷键 |
| `syncpanel.png` | 15 / 28 / 29 | 同步面板：遮罩暗化、Fetch/Pull/Push 页签、三个复选行、页脚按钮 |
| `formpanel.png` | 22 | 表单弹层：标题、字段标签与输入框、勾选行、页脚两按钮 |
| `inspector.png` | 08 / 18 | 检查器：顶部三页签（Files 选中 / Info / Blame）、「1 file changed」与文件行（`diff.rs` + M） |
| `settings.png` | 23 / 24 / 25 / 26 / 27 | 设置页：左侧六分区（General/Git/Shortcuts/Themes/Credentials/About）、右侧 Appearance 区块、主题三态（System 选中）、说明文字与分隔线 |

**未截到的板**：14（冲突三栏，仍是自绘 overlay）、16（三条横幅，未接入
`ui::banner`）、19（主区 blame 缺入口）、21（右键菜单）。

关于 21：试过加「启动即弹菜单」的入口，但 `file_menu` 需要 12 个参数（repo 路径、
文件路径、变更清单、以及 7 个回调），为验收单独构造这一整套数据不划算；而把菜单
项渲染成静态列表只是自绘一个「看起来像菜单」的东西，属于假控件，不做。菜单的
真实验证靠人工点击（`refs_view` 等 6 处都在用同一套 `ui::menu` 构造器）。

## 5. 验收手段

截图不是「看有没有排错版」，而是「看内容是否真的在」。历史表曾连续几轮被判通过，
实际整行不可见（列宽 120pt + render 根节点 `h_full` 塌成 0）。

可用的入口（`tools/shots.sh`）：

```sh
./tools/shots.sh gallery                                   # 组件画廊
GITUI_MODE=history|changes|refs|agent ...                  # 四个主视图
GITUI_DIRTY=1 GITUI_MODE=changes ...                       # 制造未提交改动，看 diff
GITUI_SYNC=1 / GITUI_FORM=1 / GITUI_PALETTE=1 / GITUI_SETTINGS=1 / GITUI_INSPECTOR=1
```

判定方法：对截图做像素统计（颜色数 / 目标色像素数），而不是目测。`data_table`
还有一个决定性约束：**必须作为 `dynamic` 的直接返回值才渲染**，放进 `flex_col`
的 child 里就是空白——这一条是靠「同一位置放红块能显示、放表格不能」的对照实验
定位的，不是靠调参数。

## 5.1 四项门禁

目标要求的验收门禁与当前实测结果：

```
cargo test --lib              80 passed; 0 failed
cargo check --all-targets     0 error / 0 warning
nsh check                     ✓ checked nsh-app
tools/check-ui.sh             check-ui: 通过
```

`nsh check` 检查 NSH 工程配置与 API 用法，与 `cargo check` 互补。首跑含编译约
40s，再跑约 170ms。

## 6. 用法

1. 每组开工前查本表第 1 节，确认该组要新建哪些组件（第 2 节），先建组件再改页面。
2. 组件落地后，把对应行从第 2 节删除，并在 `UI-KIT.md` 第 3 节登记。
3. 页面迁完，把文件加进 `tools/ui-migrated.txt`，让 `tools/check-ui.sh` 开始约束它。
