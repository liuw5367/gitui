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

- 应用支持目录由 `src/brand.rs` 的 `STORAGE_DIR` 决定，`APP_HOME` 可以直接覆盖；解析规则见 [store README](../../crates/store/README.md)。
- 窗口初始内容区 1392×852，与设计板的内容区一致；最小尺寸为 1100×700。
- 打包用 `scripts/release-macos.sh`：脚本调用 `nsh package --target macos`，设置 `APP_SIGN_IDENTITY` 与 `APP_NOTARY_PROFILE` 后继续签名、公证与 stapling。

## 结构

| 路径 | 内容 |
|---|---|
| `src/lib.rs` | 根组件：窗口、主题、语言、屏幕路由与动作集合 |
| `src/main.rs` | 原生后端入口；指定窗口标题、初始尺寸与平台分支 |
| `src/brand.rs` | 产品名、版本、构建号与存储目录名 |
| `src/design/` | 颜色 token（`mod.rs`）、几何与原生刻度（`metrics.rs`）、图标 sprite（`icons.rs`） |
| `src/shell/` | 标题栏（仓库标签页）、工具栏、状态栏与状态横幅 |
| `src/screens/` | 欢迎页、设置、引用、变更、冲突、blame、同步面板、命令面板、表单与钩子日志 |
| `src/workspace/` | 侧栏、历史表、diff、检查器、提交面板与 Agent 面板 |
| `src/ui/` | 应用自有的组件层：按钮、输入、表格、树、分段、菜单、弹层与反馈 |
| `src/model.rs` | 标签页状态与数据装载；把 `git` 与 `store` 的结果整理成界面可渲染的结构 |
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
- 已迁移的页面文件登记在 `tools/ui-migrated.txt`，不得直接引用 `nsh_base`、不得给裸 `view()` 挂 `.on_press`、不得自带色板或字体族。
- `tools/check-ui.sh` 与 `src/ui` 的守卫测试检查同一组约束，两者都通过才算满足。

设计稿位于 [designs/graft-git](../../designs/graft-git/)，由 design 单独维护；页面实现只引用画板与共享样式，不把 HTML/CSS 或 WebView 带进应用。

`ui` 层采用 macOS 原生刻度：正文 13、次要文字 11、元数据 11、代码 12/18、控件 22/28/32、行高 24/28、工具栏 52、状态栏 24；系统刻度决定字号与控件高度。

组件画廊用于逐项检查 `ui` 层外观，快捷键为 ⌘⇧D。

## 数据与语言

- 语言：`src/i18n.rs` 用进程内信号保存当前语言，设置页切换后立即生效；缺键时回退英文，再缺失则返回键名。`locales` 两边的键集合由测试保证一致。
- 主题：`theme.json` 提供 light/dark token，经 `theme_overrides` 注入；深色模式与浅色模式各有一组值，`theme_tests` 与设计 CSS 逐键比对。
- 存储：会话、最近仓库、设置、变更清单、快捷键、Agent 策略与审计由 `store` 读写；文件格式见 [crates/store/README.md](../../crates/store/README.md)。
- 截图验收：`tools/shots.sh` 用重定向的 HOME 启动应用并截图；`GITUI_REPO`、`GITUI_MODE`、`GITUI_SYNC`、`GITUI_FORM`、`GITUI_PALETTE`、`GITUI_SETTINGS`、`GITUI_INSPECTOR`、`GITUI_GALLERY` 等环境变量可以直接进入指定页面或浮层，完整列表见脚本头注释。

## 测试

```sh
cd apps/client
cargo test
nsh check
```

`cargo test` 当前 80 项通过。测试会创建真实 Git 仓库，因此需要 PATH 中有 `git`；覆盖范围包括：

- `model`：仓库装载与错误页、暂存与 hunk 暂存、提交与钩子、同步、改写、reflog、引用操作、变更清单、冲突解决、过滤与对比、树形选项。
- `design`：`theme.json` 与 `designs/graft-git` 共享样式表的颜色比对；原生刻度的顺序与最低字号。
- `i18n`：两种语言的键集合一致、占位符替换、缺键回退、相对时间与日期格式。
- `ui`：守卫测试、图标 sprite、表格列宽、树节点、组件尺寸、diff 布局。
- 其余：快捷键注册表、命令面板、菜单文案、筛选换算、确认框与变更行解析。

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
