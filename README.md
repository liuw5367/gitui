# 原生 Git 客户端

跨平台的原生 Git 客户端，用 Rust 与 NSH 0.5.0 编写。界面由 NSH 的 Rust API 在当前平台的原生后端渲染，仓库中没有 TSX、JS 运行时或 HTML 界面。当前实现 macOS 端（AppKit 后端），其他平台的细节暂不展开。

项目暂名 gitui，最终名称未定；名称与标识在 `apps/client/src/brand.rs`、`apps/client/nsh.json` 与[打包脚本](apps/client/scripts/release-macos.sh)中定义，改名位置见[客户端 README](apps/client/README.md) 的「改名」一节。

界面默认英文，可在设置页切换为简体中文。

## 当前能力

- **仓库**：打开本地仓库；克隆时可以指定分支、深度与子模块，克隆后可直接打开，过程中显示进度并可取消；初始化时可以生成 `.gitignore` 与 `README.md`；最近仓库、会话恢复与多标签；关联工作树可在独立原生窗口打开、创建、移除与清理。
- **历史与 diff**：按关键字、作者、时间与引用筛选；多提交对比；统一与并排 diff；软换行；导出补丁；单文件补丁；blame 与文件历史。
- **变更与提交**：对文件、hunk 与行执行暂存、取消暂存；丢弃改动；变更清单；提交时可选 amend、sign-off 与提交后推送；钩子失败后可以重试、跳过或查看完整日志。
- **引用与恢复**：分支、标签、远程、贮藏、子模块与工作树的常用操作；侧栏分组可折叠，远端组显示实际 Fetch/Push URL 和可折叠的远端跟踪分支树，支持指定远端获取、建立本地跟踪分支、定向拉取与删除远端分支；贮藏支持全量或选择文件保存、指定条目应用/弹出/删除与差异查看；子模块支持更新到记录版本或配置分支最新版本、同步 URL、在新标签打开、暂存 gitlink 变更与安全移除；工作树支持创建已有分支或新分支的关联工作树、安全移除干净且未打开的工作树、清理失效登记，并可在独立窗口使用；reflog 撤销；固定形态的 reword 与 squash。
- **冲突**：ours/theirs/result 三栏与可展开的 base；结果区可编辑；可以接受一侧或两侧、标记解决、继续或放弃。
- **同步**：Fetch、Pull、Push 及各自选项；强推警告；非快进时提供三种处理；同步失败会归类为 auth、offline 或 rejected。
- **Agent**：按仓库状态生成计划；逐条确认；Approve & Run 先列出范围；权限、受保护分支与审计。
- **设置与入口**：通用、Git、快捷键、凭据、Agent、About 六个分区；主题、语言、日期格式与启动行为；命令面板；可改键的快捷键；组件画廊（⌘⇧D）。

## 设计与实现

设计稿与样式由 design 单独维护，位于 `designs/graft-git`，包含 30 个画板与共享样式。页面开发以 NSH 原生组件为基础，尽量采用平台原生样式；设计板提供结构与信息层级。生产实现只引用设计源，不把 HTML/CSS 或 WebView 带进应用。

## 目录结构

| 路径 | 内容 |
|---|---|
| [apps/client/](apps/client/) | 客户端应用（包名 `nsh-app`）：界面、应用状态、语言资源与打包脚本 |
| [crates/domain/](crates/domain/) | 领域类型、diff/冲突/hunk 解析与错误分类 |
| [crates/git/](crates/git/) | Git 执行层：拼装并运行 `git`，解析输出 |
| [crates/store/](crates/store/) | 本地持久化：会话、最近仓库、设置、变更清单、快捷键、Agent 策略与审计 |
| [crates/agent/](crates/agent/) | Agent 的计划、权限与审计模型 |
| [designs/graft-git/](designs/graft-git/) | 设计稿、画板与共享样式；导出物不入库 |
| [tools/](tools/) | 设计几何提取、UI 约束检查与截图脚本 |
| [fixtures/](fixtures/) | 确定性测试仓库与 bare 远程的生成脚本；默认产物写入 `.tmp/` |
| [archive/](archive/) | 2026-11-15 归档的旧文档，冻结 |
| [docs/](docs/) | 按主题分目录的文档：`docs/design/`（设计系统）、`docs/pages/`（逐页说明） |

## 环境

- 当前构建目标为 macOS；`apps/client/src/main.rs` 在非 macOS 目标上只打印提示，不启动界面。
- Rust，各 crate 使用 edition 2024。
- NSH（[nsh.dev 文档](https://nsh.dev/llms.txt)）0.5.0。`apps/client/Cargo.toml` 把 `nsh-core`、`nsh-base` 与 `nsh-platform-macos` 固定为 `=0.5.0`。
- PATH 中有 `git`；执行层固定调用它。

开发验证所用版本：rustc 1.100.0、nsh 0.5.0、git 2.56.0。

## 运行

```sh
cd apps/client
nsh run
```

`nsh run` 启动 AppKit 开发宿主。需要隔离应用数据时用 `APP_HOME`：

```sh
APP_HOME=/tmp/app-home nsh run
```

应用数据默认在 `~/.config/gitui`，目录名取 `apps/client/src/brand.rs` 的 `STORAGE_DIR`；`APP_HOME` 可以覆盖。其中保存会话、最近仓库、设置、变更清单、快捷键、语言、外观与 Agent 状态。

打包用 `apps/client/scripts/release-macos.sh`。脚本调用 `nsh package --target macos`；设置 `APP_SIGN_IDENTITY` 与 `APP_NOTARY_PROFILE` 后继续签名、公证与 stapling。

## 验证

根目录是 Cargo workspace：`crates/domain`、`crates/git`、`crates/store`、`crates/agent` 与 `apps/client` 是成员，共用一份 `Cargo.lock` 与一个 `target/`。crate 边界不变，单独验证某个 crate 用 `cargo test -p <name>`。

```sh
cargo test                       # 全部：domain 17 / git 70 / store 17 / agent 12 / nsh-app 96
(cd apps/client && cargo test)   # 只跑客户端，等价于 cargo test -p nsh-app
(cd apps/client && nsh check)
tools/check-ui.sh
python3 tools/design/tokens.py
```

`cargo test -p <crate>` 里的 crate 名是 `domain`、`git`、`store`、`agent`、`nsh-app`。

`crates/git` 与 `apps/client` 的测试会在临时目录创建真实仓库，需要 PATH 中有 `git`。`crates/git` 还用本机 bare 远程验证 fetch/pull/push，不依赖网络。`nsh check` 对客户端执行 `cargo check`。`tools/check-ui.sh` 检查 `src/ui` 与已迁移页面的字号、颜色和直接引用约束；`tools/design/tokens.py` 校验 `docs/design/TOKENS.md` 的色值表与 `theme.json` 一致。

`fixtures/gen-repo.sh` 生成人工验收用的测试仓库与 bare 远程，默认写入 `.tmp/fixture` 与 `.tmp/fixture-remote.git`；`.tmp/` 已在 `.gitignore` 中忽略。需要改位置时把目标路径作为第一个参数传入；清理时删除 `.tmp/` 即可。

## 文档状态

当前入口是这份 README、[客户端 README](apps/client/README.md) 与四个 crate README：[domain](crates/domain/README.md)、[git](crates/git/README.md)、[store](crates/store/README.md)、[agent](crates/agent/README.md)。

设计系统文档在 [docs/design/](docs/design/README.md)：[DESIGN-SYSTEM.md](docs/design/DESIGN-SYSTEM.md)（视觉语言）、[TOKENS.md](docs/design/TOKENS.md)（token 契约，色值表由 `tools/design/tokens.py` 从 `theme.json` 生成）、[UI-KIT.md](docs/design/UI-KIT.md)（组件层规则）、[DEVIATIONS.md](docs/design/DEVIATIONS.md)（与画板的已知差异）；页面开发规范与验收门槛在 [docs/development/](docs/development/README.md)；[docs/README.md](docs/README.md) 是文档目录索引。2026-11-15 的旧文档整体归档在 [archive/2026-11-15-docs/](archive/2026-11-15-docs/)，该目录冻结，只作历史参考；归档中的状态结论不代表当前实现。

## 已知限制

- 设置窗口的六个分区已按 `docs/pages/settings*.md` 重做。General、Shortcuts、Git 真实读写；Updates、Credentials、Agent 只做界面与本地偏好，跨系统动作只给事实性提示。
- 账号连接与登录流程没有接入。Credentials 页的 GitHub / GitLab 显示真实空态，Connect / Sign out 只给提示；不保存 token，同步失败仍归类为 auth、offline 或 rejected。
- 更新检查没有发布源。General 与 About 的更新状态显示 `Not checked`，Check Now 只给说明。
- Credentials 的凭据助手来自本机 `git config`，SSH 密钥只读取 `~/.ssh` 的文件名与公钥指纹；不生成、不删除、不上传，也不读取私钥内容。
- 设置里的 Git 路径（`use_system_git`、`git_path`）会保存并用于探测，`crates/git` 仍固定调用 PATH 中的 `git`。
- 历史改写只有固定形态的 reword（单条提交）与 squash（连续区间），交互式 rebase 的 todo 编辑未实现。
- Agent 的协议、端点、模型、温度与隐私偏好会保存，App key 只保存在会话内存；没有 provider 与网络运行时，因此不会发起请求。旧的动作权限与受保护分支仍然生效。
- 同步用本机 bare 远程测试；第三方远程与真实凭据流程未验收，OAuth 与 SSH 密钥管理未实现。
- diff 工具栏的折叠（fold）与空白（whitespace）开关会切换图标状态，但没有传入渲染层；当前只有软换行（wrap）生效。
- 其他平台尚未适配，构建与验证只在 macOS 上完成。
