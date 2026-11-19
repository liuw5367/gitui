# 客户端

`apps/client` 是应用层：一个 Rust crate（包名 `nsh-app`），同一个 crate 由 NSH 按目标平台构建，当前接入 macOS 的 AppKit 宿主。界面、应用状态、语言资源与打包脚本都在这里；Git 行为在 `crates/git`，持久化在 `crates/store`。

## 运行

```sh
cd apps/client
nsh run
```

`nsh run` 启动 AppKit 开发宿主；NSH 的命令与文档见 [nsh.dev 文档](https://nsh.dev/llms.txt)。需要隔离应用数据时用 `APP_HOME`：

```sh
APP_HOME=/tmp/app-home nsh run
```

- 应用数据目录默认是 `~/.config/gitui`，由 `src/brand.rs` 的 `STORAGE_DIR` 决定；`APP_HOME` 可以直接覆盖。解析规则见 [store README](../../crates/store/README.md)。
- 窗口初始内容区 1392×852，与设计板的内容区一致；最小尺寸为 1100×700。
- 启动行为由设置的 `Launch behavior` 决定：恢复上次会话时按保存顺序打开主窗口标签且不打开欢迎页；空会话或 `Show Welcome` 只显示欢迎窗口。主窗口 `+` 打开欢迎窗口；关闭最后一个标签会清空会话并返回欢迎页，关闭主窗口不会清空标签，会保存当前顺序和活动仓库，供下次启动恢复；应用退出也会保存会话。
- 主窗口默认进入 Changes，中栏显示变更列表、右栏显示 Commit，左侧引用栏也默认打开。左右栏与弹性中栏都可拖拽调整宽度；工具栏按钮可分别隐藏或恢复左右栏，宽度保留到当前窗口关闭。Settings 是独立普通窗口，支持系统最小化、缩放和关闭；欢迎页、主窗口和 macOS 应用菜单（⌘,）均可打开 Settings，重复调用会聚焦已有设置窗口。⌘W 在 Settings、Welcome 与关联工作树窗口关闭当前窗口，在主窗口关闭活动仓库标签页。
- Worktrees 侧栏的 `Open in New Window` 为关联工作树打开独立单仓库窗口，不显示仓库标签栏或加号；重复打开同一路径会聚焦已有工作树窗口，关闭辅助窗口不会改动主窗口会话。
- 打包用 `scripts/release-macos.sh`：脚本调用 `nsh package --target macos`，设置 `APP_SIGN_IDENTITY` 与 `APP_NOTARY_PROFILE` 后继续签名、公证与 stapling。

## 结构

| 路径 | 内容 |
|---|---|
| `src/lib.rs` | 根组件：模块声明、`App` 转发与根导出（引导见 `src/app/bootstrap.rs`） |
| `src/main.rs` | 原生后端入口；指定窗口标题、初始尺寸与平台分支 |
| `src/brand.rs` | 产品名、版本、构建号与存储目录名 |
| `src/design/` | 颜色 token（`mod.rs`）、几何与原生刻度（`metrics.rs`）、图标 sprite（`icons.rs`） |
| `src/shell/` | 标题栏（仓库标签页）、工具栏、状态栏与状态横幅 |
| `src/features/` | 功能切片：`state`（信号集合）、`commands`（用例）、`view`（渲染）与 `ops`（对 `Tab` 的写操作）；欢迎、历史、变更、提交、引用、同步、冲突、恢复、Agent、设置与搜索各一目录 |
| `src/shared/` | 跨功能件：diff 视图与显示开关（`diff.rs`）、平台查询（`os.rs`） |
| `src/ui/` | 应用自有的组件层：按钮、输入、选择、表格、树、分段、菜单、弹层与反馈 |
| `src/app/` | 应用组装层：引导与首屏（`bootstrap.rs`）、主窗口版式（`workspace.rs`）、菜单（`menu.rs`）、快捷键（`hotkeys.rs`）、窗口与会话持久化（`windows.rs`）、标签页壳（`session.rs`）、句柄集合（`handles.rs`）与跨功能写操作（`ops.rs`） |
| `src/i18n.rs` | 读取 `locales/*.json`，按当前语言取文案 |
| `src/shortcuts.rs` | 快捷键注册表：默认值、改键与显示 |
| `src/filters.rs` | 历史筛选条件到 `git log` 参数的转换 |
| `src/dates.rs` | 日期显示格式；持久化在 `store::Settings` |
| `src/interactions.rs` | 确认框、剪贴板、外部打开与状态栏提示 |
| `src/widgets.rs` | 尚未迁入 `ui` 的 3 处尺寸自由图标 |
| `locales/` | `en.json` 与 `zh-Hans.json`；两边的键集合一致 |
| `theme.json` | light/dark 颜色 token，与设计 CSS 的颜色变量同值；`accent`、`desk1`、`desk2` 三处语义不同，测试单独排除 |

## ui 层与界面规则

页面与 `workspace` 以 NSH 原生组件为基础；键盘、焦点与无障碍行为由组件提供，`src/ui` 统一尺寸、颜色与必要外观，优先保留平台原生样式。规则如下：

- 字号取 `src/ui/mod.rs` 的刻度常量，颜色取 `src/design`；`src/ui` 内不写字号字面量与十六进制颜色。
- 输入框获得焦点时显示 2pt `focus` 主题 outline，颜色由 `ui::focus_color()` 提供；页面不单独覆盖。
- 已迁移的页面文件登记在 `tools/ui-migrated.txt`，不得直接引用 `nsh_base`、不得给裸 `view()` 挂 `.on_press`、不得自带色板或字体族。
- `tools/check-ui.sh` 与 `src/ui` 的守卫测试检查同一组约束，两者都通过才算满足。

页面开发流程、响应式契约与完成门槛见 [docs/development/page-development.md](../../docs/development/page-development.md)。

History 默认中心区域展示所有分支与 tag 可达的提交、stash 列表中的每个提交，并按父提交绘制 branch graph；引用侧栏的分支、远端分支、tag、stash 叶行按完整 ref 或 stash OID 定位并滚动到目标提交。History 列表占满中栏剩余高度，由虚拟表格滚动。顶部的分支与作者下拉菜单支持多选并显示已选数量，工具栏快速搜索独立于筛选草稿。日期区间、文件/目录路径与消息子串通过筛选浮层组合设置。右侧检查器展示提交信息和文件变更目录树；点击提交只更新检查器，点击其中的变更文件后，中栏整栏切换为占满高度的 Diff，顶部返回按钮可回到 History 并保留当前提交。提交右键菜单也可显式显示整个提交的差异。Diff 使用 NSH DiffView 按文件路径自动识别语法，支持 Unified/Split、折叠、换行、显示空白、忽略空白差异、hunk 导航、复制与导出；差异基于第一父提交，根提交与空 tree 比较。Compare 支持连续提交、所有本地/远端分支、tag、stash 端点与工作区对比，并提供 2-dot/3-dot 语义和退化态反馈；工作区比较包含暂存和未暂存的 tracked 文件。

Merge 冲突页从 Git index 的 stage 1/2/3 读取 Base、Ours、Theirs，并按文件路径语法高亮；可编辑 Result 草稿按冲突路径留在当前仓库会话中。缺失的 stage 表示该侧删除文件，二进制冲突通过选择一侧解决。标记解决只写入并暂存当前路径；有未解决文件或未处理草稿时不能继续，继续动作还会重新读取 Git 操作与冲突状态。详细交互见[合并冲突页说明](../../docs/pages/merge-conflict.md)。

设计稿位于 [designs/graft-git](../../designs/graft-git/)，由 design 单独维护；页面实现只引用画板与共享样式，不把 HTML/CSS 或 WebView 带进应用。

`ui` 层采用 macOS 原生刻度：正文 13、次要文字 11、元数据 11、代码 12/18、控件 22/28/32、行高 24/28、原生工具栏基准 52、主工作区紧凑工具栏 40、状态栏 24；系统刻度决定字号与控件高度。

组件画廊用于逐项检查 `ui` 层外观，作为独立窗口打开，快捷键为 ⌘⇧D。

## 数据与语言

- 语言：`src/i18n.rs` 用进程内信号保存当前语言，设置页切换后立即生效；缺键时回退英文，再缺失则返回键名。`locales` 两边的键集合由测试保证一致。
- 主题：`theme.json` 提供 light/dark token，经 `theme_overrides` 注入；深色模式与浅色模式各有一组值，`theme_tests` 与设计 CSS 逐键比对。
- 存储：会话、成功打开过的最近仓库、Git 扫描发现结果及命令面板的扫描搜索资格、设置、变更清单、快捷键、Agent 策略与审计由 `store` 读写；扫描结果与最近打开历史分开保存。欢迎页查询全部扫描结果，命令面板查询由 Git 设置启用的扫描结果。文件格式见 [crates/store/README.md](../../crates/store/README.md)。
- 截图验收：`tools/shots.sh` 用重定向的 HOME 启动应用并截图；产物写入 `files.local/shots/`（已被 `.gitignore` 排除，不提交）。`GITUI_REPO`、`GITUI_MODE`、`GITUI_SYNC`、`GITUI_FORM`、`GITUI_PALETTE`、`GITUI_SETTINGS`、`GITUI_INSPECTOR`、`GITUI_GALLERY`、`GITUI_NO_WELCOME`、`GITUI_WELCOME_FORM`、`GITUI_WELCOME_STATE`、`GITUI_WELCOME_RECENT` 可以直接进入指定页面或浮层，完整列表见脚本头注释。这些开关**只在 debug 构建里读取**（`src/acceptance.rs`）：`nsh run` 是 debug，读；`nsh package` 的发布二进制不读任何 `GITUI_*`。

## 测试

```sh
cargo test -p nsh-app        # 等价于在本目录运行 cargo test
nsh check
```

仓库根是 Cargo workspace，客户端是其中一个成员（`nsh-app`），共用根 `Cargo.lock` 与 `target/`；在仓库根直接 `cargo test` 会跑全部五个成员。`cargo test -p nsh-app` 当前 106 项通过。测试会创建真实 Git 仓库，因此需要 PATH 中有 `git`；覆盖范围包括：

- `features/*/ops`（连同 `app/session.rs`、`app/ops.rs`）：仓库装载与错误页、暂存与 hunk 暂存、提交与钩子、同步及其稳定仓库目标、改写、reflog、引用操作、分支和远端跟踪分支斜杠树排序与折叠键、变更清单、冲突解决、过滤与对比、树形选项。
- `design`：`theme.json` 与 `designs/graft-git` 共享样式表的颜色比对；原生刻度的顺序与最低字号。
- `i18n`：两种语言的键集合一致、占位符替换、缺键回退、相对时间与日期格式。
- `ui`：守卫测试、图标 sprite、表格列宽、树节点、组件尺寸、diff 布局。
- 其余：快捷键注册表、命令面板（常用操作、打开历史、扫描仓库搜索与去重）、菜单文案、筛选换算、确认框与变更行解析。

`theme_tests` 会读取 `designs/graft-git/project/assets/` 下的共享样式表；设计源缺失时这组测试失败。`nsh check` 对客户端执行 `cargo check`；`tools/check-ui.sh` 从仓库根目录运行。

## 改名

产品名尚未最终确定。名称与标识分布在三处，改名时一起修改：

1. `src/brand.rs`：`PRODUCT_NAME`、`STORAGE_DIR`。
2. `nsh.json`：`name`、`id`，NSH 读取这两个字段。
3. `scripts/release-macos.sh`：`nsh package` 的 `--name`、`--id` 以及产物文件名。

## 已知限制

- 部分文件头注释滞后于实现：`src/lib.rs` 仍描述只读阶段，`src/shell/mod.rs` 仍描述 P1 的禁用按钮；`src/design/metrics.rs` 仍引用 `tests/geometry.rs` 与已归档的计划。修改前以代码行为为准；设计系统口径见 [docs/design/](../../docs/design/README.md)。
- 产品范围与用户可见限制见[项目 README](../../README.md) 的「已知限制」一节。

## 相关

- [项目 README](../../README.md)
- [domain](../../crates/domain/README.md)、[git](../../crates/git/README.md)、[store](../../crates/store/README.md)、[agent](../../crates/agent/README.md)
