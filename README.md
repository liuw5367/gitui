# 原生 Git 客户端

跨平台的原生 Git 客户端，用 Rust 与 NSH 0.6.0 编写。界面由 NSH 的 Rust API 在当前平台的原生后端渲染，仓库中没有 TSX、JS 运行时或 HTML 界面。当前实现 macOS 端（AppKit 后端），其他平台的细节暂不展开。

项目暂名 gitui，最终名称未定；名称与标识在 `apps/client/src/brand.rs`、`apps/client/nsh.json` 与[打包脚本](apps/client/scripts/release-macos.sh)中定义，改名位置见[客户端 README](apps/client/README.md) 的「改名」一节。

界面默认英文，可在设置页切换为简体中文。

## 当前能力

- **仓库**：打开本地仓库；克隆时可以指定分支、深度与子模块，克隆后可直接打开，过程中显示进度并可取消；初始化时可以生成 `.gitignore` 与 `README.md`。欢迎页空查询显示最近打开记录，输入后同时搜索最近记录与全部已扫描仓库；成功打开才写入历史。命令面板默认把常用命令排在仓库前，输入后也可搜索已启用的扫描仓库。另有会话恢复与多标签；关联工作树可在独立原生窗口打开、创建、移除与清理。
- **历史与 diff**：History 默认展示本地与远端分支、tag 可达的完整 commit 列表，以及 stash 列表中的每个提交，并绘制 branch graph；点击左侧分支、远端分支、tag 或 stash 会定位到对应 commit。顶部可多选筛选分支和作者，日期区间、路径与消息筛选通过浮层设置。右侧检查器展示变更文件目录树，底部可滚动 Info 显示提交摘要、作者与时间、短 SHA、复制入口和 Branches and refs；只有 committer 与 author 身份或时间不同时才显示提交者信息。History 多选时 Info 按提交列表顺序展示所有选中提交。点击提交只更新检查器；点击变更文件后，中栏整栏切换为 Diff，顶部按钮可返回 History。提交菜单可显示完整提交差异。Diff 支持 Unified/Split、按文件类型多语言语法高亮、上下文折叠、软换行、显示空白与忽略空白差异、复制和导出。Compare 支持连续提交、分支/远端分支/tag/stash、工作区端点、2-dot/3-dot、Swap 与无差异/无共同祖先反馈。
- **变更与提交**：对文件、hunk 与行执行暂存、取消暂存；丢弃改动；变更清单；提交时可选 amend、sign-off 与提交后推送；钩子失败后可以重试、跳过或查看完整日志。
- **引用与恢复**：分支、标签、远程、贮藏、子模块与工作树的常用操作；侧栏分组可折叠，远端组显示实际 Fetch/Push URL 和可折叠的远端跟踪分支树，支持指定远端获取、建立本地跟踪分支、定向拉取与删除远端分支；贮藏支持全量或选择文件保存、指定条目应用/弹出/删除与差异查看；子模块支持更新到记录版本或配置分支最新版本、同步 URL、在新标签打开、暂存 gitlink 变更与安全移除；工作树支持创建已有分支或新分支的关联工作树、安全移除干净且未打开的工作树、清理失效登记，并可在独立窗口使用。Fetch、Pull、Push、远端分支/标签推送、删除远端分支、子模块更新与 Agent 发起的同步动作在 Git 实际请求输入时使用应用凭据窗口。工具栏可直接打开 Stash Changes 表单与 Reflog；中栏保留 Refs 总表与 Reflog 页签互切，选择记录后右栏显示提交信息与文件变更统计。Reflog 可撤销；支持固定形态的 reword 与 squash。
- **冲突**：从 Git index stage 1/2/3 显示 Base、Ours、Theirs，三侧按文件类型语法高亮；Result 可编辑并按路径保留草稿。缺少的 stage 显示为删除，二进制文件可选择一侧；标记解决只写回并暂存当前文件，残留冲突标记会阻止标记，继续操作前会重新读取 Git 状态。
- **同步**：Fetch、Pull、Push 及各自选项；以后台 Git 子进程执行并显示实时输出；关闭弹窗后任务继续，Cancel 会终止子进程。只有 credential helper 没有提供可用凭据、Git 实际调用 AskPass 时，应用才询问用户名或密码/令牌；取消会中止当前操作，凭据持久化仍由 Git credential helper 决定。成功后说明目标是否已是最新，或列出更新引用和提交节点数；Push 部分拒绝时同时显示已推送与被拒绝数量，无法确认的计数明确标为未知。面板继续保留原始 Git 输出；强推警告与非快进处理保留现有安全语义。
- **Agent**：根据当前仓库状态生成本地计划；可逐项编辑与确认，Approve & Run 会先列出执行范围，再运行已有的 Git 动作；权限、受保护分支与审计持续生效。Agent 发起的 Fetch、Pull、Push 也使用按需 AskPass，执行期间界面保持响应。模型接入状态见下方「已知限制」。
- **设置与入口**：通用、Git、快捷键、凭据、Agent、About 六个分区；主题、语言、日期格式与启动行为；凭据页显示 Git credential helper，不提供 GitHub / GitLab 账号连接；在 `~/.ssh` 只读列出本地密钥并可复制公钥；欢迎页和主窗口设置入口、macOS 应用菜单中的 Settings（⌘,）；命令面板；可改键的快捷键；独立窗口中的组件画廊（⌘⇧D）。

## 设计与实现

设计稿与样式由 design 单独维护，位于 `designs/graft-git`，包含 32 个画板条目与共享样式；逐页说明共 30 份，Welcome 的 01b / 01c 两种状态合并在 `docs/pages/welcome.md`。页面说明记录设计目标，不表示实现已逐项验收；当前没有逐页完成度账本，验收方式见[页面验收规范](docs/development/verification.md)。页面开发以 NSH 原生组件为基础，尽量采用平台原生样式；生产实现只引用设计源，不把 HTML/CSS 或 WebView 带进应用。

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
- Rust，各包从 workspace 根目录继承 edition 2024；rustfmt 风格版固定在根目录 `rustfmt.toml`。
- NSH（[nsh.dev 文档](https://nsh.dev/llms.txt)）0.6.0。`apps/client/Cargo.toml` 把 `nsh-core`、`nsh-base` 与 `nsh-platform-macos` 固定为 `=0.6.0`。
- PATH 中有 `git`；执行层固定调用它。

开发验证所用版本：rustc 1.100.0、nsh 0.6.0、git 2.56.0。

## 运行

```sh
cd apps/client
nsh run
```

`nsh run` 启动 AppKit 开发宿主。需要隔离应用数据时用 `APP_HOME`：

```sh
APP_HOME=/tmp/app-home nsh run
```

应用数据默认在 `~/.config/gitui`，目录名取 `apps/client/src/brand.rs` 的 `STORAGE_DIR`；`APP_HOME` 可以覆盖。其中保存会话、成功打开过的最近仓库、扫描发现结果及其命令面板搜索资格、设置、变更清单、快捷键、语言、外观与 Agent 状态。扫描结果与打开历史分别保存。

打包用 `apps/client/scripts/release-macos.sh`。脚本调用 `nsh package --target macos`；设置 `APP_SIGN_IDENTITY` 与 `APP_NOTARY_PROFILE` 后继续签名、公证与 stapling。

## 验证

根目录是 Cargo workspace：`crates/domain`、`crates/git`、`crates/store`、`crates/agent` 与 `apps/client` 是成员，共用一份 `Cargo.lock` 与一个 `target/`。crate 边界不变，单独验证某个 crate 用 `cargo test -p <name>`。

```sh
cargo fmt --all                 # 格式化整个 workspace 的 Rust 代码
cargo test                       # 全部：domain 17 / git 92 / store 22 / agent 11 / nsh-app 148
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

- 设置窗口的六个分区已按 `docs/pages/settings*.md` 实现。General、Shortcuts、Git 有对应的本地读写；Updates 与 Agent 保存本地偏好，尚未接入相应的发布或模型服务。
- 应用不连接 GitHub / GitLab 账号，也不保存 HTTP(S) 凭据。Git 先使用本机 credential helper；只有 Git 实际调用 AskPass 时，应用才询问所需内容。用户回答只返回给当前 Git 操作，保存、缓存与再次获取由 Git credential helper 决定。
- 更新检查没有发布源。General 与 About 的更新状态显示 `Not checked`，Check Now 只给说明。
- Credentials 的凭据助手读取本机 `git config`。SSH 密钥区只读列出当前用户 `~/.ssh` 中可识别的公钥、指纹与相邻私钥状态，并可复制公钥；页面不会添加、生成或删除本地密钥文件。私钥内容不会显示、复制到剪贴板或写入日志。这里不管理 SSH agent、SSH config、系统钥匙串或 known_hosts。外部编辑器、diff / merge 工具、终端、代理、SSL 校验、系统 Git 配置和新远端的 SSH 偏好目前只保存为本地设置，尚未影响对应的外部工具或网络操作。
- 设置里的 Git 路径（`use_system_git`、`git_path`）会用于应用直接启动的 Git 命令，包括仓库操作、克隆、初始化、同步、子模块、工作树与版本探测。自定义路径无效时操作报错，不静默回退到 PATH；已启动操作继续使用启动时的 executable。
- 历史改写只有固定形态的 reword（单条提交）与 squash（连续区间），交互式 rebase 的 todo 编辑未实现。
- Agent 根据仓库状态生成本地计划；批准后可运行已有的暂存、提交与同步动作，权限拒绝和受保护分支规则会拦截操作并记录审计。协议、端点、模型、温度与隐私偏好会保存，App key 只保存在会话内存；模型 provider、网络请求与模型生成计划尚未实现。
- 同步用本机 bare 远程测试；第三方远程、HTTP(S) 凭据实际交互及 SSH 加密私钥的手动端到端操作尚未验收。应用不提供 GitHub / GitLab 账号服务。
- DiffView 的语言高亮由文件扩展名识别；未知扩展名按纯文本展示。
- 其他平台尚未适配，构建与验证只在 macOS 上完成。
