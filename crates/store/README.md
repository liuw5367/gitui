# store

本地持久化。把会话、最近仓库、设置、变更清单、快捷键、Agent 策略与审计写成文本文件，放在应用支持目录下。没有第三方依赖；读写函数都接收目录参数，便于测试与 `APP_HOME` 隔离。

## 目录

`default_dir(directory)`：

- 设置了 `APP_HOME` 时返回它；
- 否则返回 `~/Library/Application Support/<directory>`。

应用传入 `brand::STORAGE_DIR` 的值（见 [apps/client](../../apps/client/README.md)）。

## 文件

| 文件 | 格式 | 内容 |
|---|---|---|
| `recent.txt` | 每行 `<打开时间>\t<路径>`，最新的在前 | 最近仓库，上限 12 条；兼容只有路径的旧格式 |
| `session.txt` | 第一行是当前 tab 下标，其后每行一个路径 | 打开的仓库与活动 tab |
| `language.txt` | 一行语言标识 | `en` 或 `zh-Hans` |
| `appearance.txt` | 一行 | `system`、`light` 或 `dark` |
| `settings.txt` | 每行 `键=值` | 文件树、启动、更新方式、Git 路径、日期格式 |
| `changelists.txt` | 每行 `<仓库>\t<清单名>\t<路径>` | 按仓库分组的变更清单 |
| `shortcuts.txt` | 每行 `<动作 id>=<组合>` | 只保存与默认值不同的改键 |
| `agent-policies.txt` | 每行 `<动作>=<权限>` | Agent 每个动作的权限 |
| `agent-audit.txt` | 每行一条，格式为 `<时间>\t<动作>\t<结果>\t<详情>` | Agent 审计，最新的在前 |
| `git-scan-folders.txt` | 每行一个绝对路径 | 设置页扫描 Git 仓库的目录 |

`settings.txt` 的键包括 `file_tree`、`merge_empty_dirs`、`compact_single_child`、`restore_session`、`update_mode`、`use_system_git`、`git_path`、`date_format` 与 `date_pattern`。

## 写入约定

- 先写同名临时文件（`settings.txt` 写为 `settings.txt.tmp`），再用 `rename` 覆盖目标文件，避免中途失败留下半个文件。
- 含换行或回车的路径会跳过，逐行格式无法表达这类路径。
- 文件缺失或内容不符合格式时回退空状态或默认值，不返回错误。
- 保存变更清单时保留其他仓库的条目；保存最近仓库时去重并截断到 12 条。
- 读取会话时，`active` 超出范围会夹到最后一个已打开的仓库。

## 扫描仓库

`scan_git_repositories(folders)` 在给定目录下最多下探三层：

- 命中 `.git` 后记录该目录，不再深入；
- 跳过 `node_modules`、`target` 与以 `.` 开头的目录；
- 结果排序并去重，保证同样的输入得到同样的顺序。

## 测试

```sh
cd crates/store
cargo test
```

当前 13 项通过，覆盖各文件的往返读写、缺失文件的默认值、最近仓库的去重与上限、会话下标夹取、变更清单按仓库隔离、快捷键覆盖项、Agent 策略与审计、扫描深度与噪音目录过滤。

## 使用方

- [apps/client](../../apps/client/README.md)：读写会话、设置、变更清单、快捷键与 Agent 状态。

## 相关

- [项目 README](../../README.md)
