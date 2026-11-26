# store

本地持久化。应用偏好保存在 `settings.toml`；会话、最近仓库、变更清单和 Agent 审计使用独立文本文件。读写函数接收目录参数，便于测试与 `APP_HOME` 隔离；store 直接依赖 Serde 与 TOML。

## 目录

`default_dir(directory)`：

- 设置了 `APP_HOME` 时返回它；
- 否则返回 `~/.config/<directory>`。

应用传入 `brand::STORAGE_DIR` 的值（当前为 `gitui`，见 [apps/client](../../apps/client/README.md)）。

## 文件

| 文件 | 格式 | 内容 |
|---|---|---|
| `settings.toml` | 带 `schema_version = 1` 的 TOML | 通用、Git、快捷键和 Agent 偏好 |
| `recent.txt` | 每行 `<打开时间>\t<路径>\t<分支>`，最新的在前 | 最近仓库，上限 12 条；兼容旧格式 |
| `session.txt` | 第一行是当前 tab 下标，其后每行一个路径 | 打开的仓库与活动 tab |
| `changelists.txt` | 每行 `<仓库>\t<清单名>\t<路径>`；空路径记录表示空清单 | 按仓库分组的变更清单，空清单也会保留 |
| `agent-audit.txt` | 每行 `<时间>\t<动作>\t<结果>\t<详情>` | Agent 审计，最新的在前 |

`settings.toml` 的分区：

- `[general]`：界面语言、外观、日期、文件树、DiffView 显示方式、启动行为与更新偏好。
- `[git]`：Git 可执行文件、外部工具、网络偏好、扫描目录及扫描仓库的搜索选择。
- `[shortcuts]`：只保存与默认值不同的动作绑定。
- `[agent.provider]`：协议、端点、模型、采样温度、回复上限及隐私偏好。
- `[agent.runtime]`：自动化程度、执行预览时间与受保护分支。
- `[agent.permissions]`：Agent 动作标识到权限标识的映射。

Git 身份由 Git 配置管理。App key 不写入 `settings.toml`；当前只保存在应用进程内存。Credentials 页不保存账号令牌。

## 默认值与错误处理

- 缺少 `settings.toml` 或缺少版本字段之外的配置字段时使用应用默认值；新仓库的扫描搜索资格仍由应用默认设为启用。
- 旧版 `settings.txt`、`language.txt`、`appearance.txt`、`shortcuts.txt`、`agent-policies.txt`、`git-scan-folders.txt` 与 `git-scan-repositories.txt` 不读取、不改写、不删除。使用新版本时这些旧偏好不会生效；新版本只读写 `settings.toml`。
- 缺少 `schema_version`、TOML 语法错误、未知字段或不支持的 schema 版本会使设置读取回默认值；保存会跳过，不覆盖原文件。修复或移走该文件后，偏好读写恢复。当前 UI 不提示此类存储错误。
- 设置先写入 `settings.toml.tmp`，再用 `rename` 替换目标文件。各类偏好更新前读取当前 TOML，并只修改对应分区。
- 运行数据仍沿用文本文件及临时文件加改名写入；逐行格式不能表达含换行的字段。
- 保存变更清单时保留其他仓库条目；保存最近仓库时去重并截断至 12 条。
- 读取会话时，`active` 超出范围会夹到最后一个已打开的仓库。

## 扫描仓库

`scan_git_repositories(folders)` 在给定目录下最多下探三层：

- 命中 `.git` 后记录该目录，不再深入；
- 跳过 `node_modules`、`target` 与以 `.` 开头的目录；
- 结果排序并去重，保证同样的输入得到同样的顺序。

设置页把每个扫描结果的搜索资格保存在 `settings.toml`。新发现的仓库默认可搜索；逐项取消后保存停用状态。移出当前扫描结果的路径设置会保留，重新扫描到时继续使用。扫描只更新发现结果，不改最近打开列表。

## 测试

```sh
cd crates/store
cargo test
```

测试覆盖配置默认值、TOML 往返与分区更新、旧偏好文件忽略、损坏配置保护、Agent 权限和运行设置隔离、快捷键、扫描偏好，以及运行状态文本文件的读写。

## 使用方

- [apps/client](../../apps/client/README.md)：读写会话、设置、变更清单、快捷键与 Agent 状态。

## 相关

- [项目 README](../../README.md)
