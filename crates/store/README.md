# store

本地持久化。把会话、成功打开过的最近仓库、扫描仓库的命令面板搜索资格、设置、变更清单、快捷键、Agent 策略与审计写成文本文件，放在应用支持目录下。没有第三方依赖；读写函数都接收目录参数，便于测试与 `APP_HOME` 隔离。

## 目录

`default_dir(directory)`：

- 设置了 `APP_HOME` 时返回它；
- 否则返回 `~/.config/<directory>`。

应用传入 `brand::STORAGE_DIR` 的值（当前为 `gitui`，见 [apps/client](../../apps/client/README.md)）。

## 文件

| 文件 | 格式 | 内容 |
|---|---|---|
| `recent.txt` | 每行 `<打开时间>\t<路径>\t<分支>`，最新的在前 | 最近仓库，上限 12 条；分支列可缺省，兼容只有路径或只有时间与路径的旧格式 |
| `session.txt` | 第一行是当前 tab 下标，其后每行一个路径 | 打开的仓库与活动 tab |
| `language.txt` | 一行语言标识 | `en` 或 `zh-Hans` |
| `appearance.txt` | 一行 | `system`、`light` 或 `dark` |
| `settings.txt` | 每行 `键=值` | 文件树、启动、更新、Git、日期、Agent、工具与网络偏好 |
| `changelists.txt` | 每行 `<仓库>\t<清单名>\t<路径>` | 按仓库分组的变更清单 |
| `shortcuts.txt` | 每行 `<动作 id>=<组合>` | 只保存与默认值不同的改键 |
| `agent-policies.txt` | 每行 `<动作>=<权限>` | Agent 每个动作的权限 |
| `agent-audit.txt` | 每行一条，格式为 `<时间>\t<动作>\t<结果>\t<详情>` | Agent 审计，最新的在前 |
| `git-scan-folders.txt` | 每行一个绝对路径 | 设置页扫描 Git 仓库的目录 |
| `git-scan-repositories.txt` | 每行 `+<绝对路径>` 或 `-<绝对路径>` | 扫描发现的仓库是否允许命令面板搜索 |

`settings.txt` 的键包括 `file_tree`、`merge_empty_dirs`、`compact_single_child`、`restore_session`、`update_mode`、`use_system_git`、`git_path`、`date_format`、`date_pattern`、`agent_protocol`、`agent_base_url`、`agent_model`、`agent_temperature`、`agent_max_tokens`、`agent_send_diffs`、`agent_send_repository_path`、`ssh_for_new_remotes`、`external_editor`、`diff_tool`、`merge_tool`、`terminal`、`external_editor_for_messages`、`proxy_mode`、`verify_ssl` 与 `use_system_git_config`。旧文件缺少新键时使用默认值。

## 写入约定

- 先写同名临时文件（`settings.txt` 写为 `settings.txt.tmp`），再用 `rename` 覆盖目标文件，避免中途失败留下半个文件。
- 含换行或回车的路径会跳过，逐行格式无法表达这类路径。
- 文件缺失或内容不符合格式时回退空状态或默认值，不返回错误。
- 保存变更清单时保留其他仓库的条目；保存最近仓库时去重并截断到 12 条。
- 同一路径再次打开时保留已有的分支记录；分支为空时仍写两列，旧版本可以继续读取。
- 扫描搜索资格文件缺失或没有某条仓库记录时，由应用将该扫描仓库默认设为可搜索；未知格式的行会忽略。
- 读取会话时，`active` 超出范围会夹到最后一个已打开的仓库。

## 扫描仓库

`scan_git_repositories(folders)` 在给定目录下最多下探三层：

- 命中 `.git` 后记录该目录，不再深入；
- 跳过 `node_modules`、`target` 与以 `.` 开头的目录；
- 结果排序并去重，保证同样的输入得到同样的顺序。

设置页把每个结果的搜索资格写入 `git-scan-repositories.txt`。文件缺失或新发现的仓库默认可搜索；逐项取消后会保存停用状态。移出当前扫描结果的路径设置会保留，重新扫描到时继续使用。扫描只更新发现结果，不改最近打开列表。

## 测试

```sh
cd crates/store
cargo test
```

当前 18 项通过，覆盖默认目录规则、各文件的往返读写、缺失文件的默认值、最近仓库的去重与上限、分支列与旧格式兼容、会话下标夹取、变更清单按仓库隔离、快捷键覆盖项、Agent 策略与审计、扫描深度与噪音目录过滤、扫描仓库命令面板搜索资格的读写。

## 使用方

- [apps/client](../../apps/client/README.md)：读写会话、设置、变更清单、快捷键与 Agent 状态。

## 相关

- [项目 README](../../README.md)
