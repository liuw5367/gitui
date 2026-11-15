# git

Git 执行层。仓库里只有这里拼装并运行 `git` 命令，并把输出解析成 `domain` 的类型。crate 名与 `git` 命令同名；运行时调用 PATH 中的系统 Git，不链接 libgit2。

## 执行约定

- 每条命令带 `-C <工作树>`，参数以独立 argv 传入，不拼接 shell 字符串。
- 读取命令设置 `GIT_TERMINAL_PROMPT=0` 与 `GIT_OPTIONAL_LOCKS=0`：凭据缺失时直接失败，只读命令不拿可选锁。stdout 超过 8 MiB 截断，错误详情截断到前 4096 个字符。
- fetch、pull、push、merge 等操作的进度与结果写在 stderr，执行层把 stdout 与 stderr 合并，成功后取最后 3 条有效行作为摘要；失败时把合并输出截断到前 4096 个字符作为 `detail`。
- 命令以非零状态退出时返回 `GitError::CommandFailed`，保留程序名、参数、退出码与截断后的输出。
- 改写历史时自行生成 rebase todo，并用临时脚本覆盖 `GIT_SEQUENCE_EDITOR` 与 `GIT_EDITOR`；命令失败后执行 `git rebase --abort`，不把仓库留在改写中间状态。

## 打开仓库

`Repo::open` 用 `git rev-parse --show-toplevel` 校验工作树：

- 路径不存在或不可读时返回 `GitError::Unreadable`；
- 路径存在但不是仓库时返回 `GitError::NotARepository`；
- 通过后保存工作树根目录，后续命令都在该目录上运行。

## API

| 分组 | 方法 |
|---|---|
| 打开与状态 | `Repo::open`、`root`、`name`、`summary`、`abbrev_branch`、`head_state`、`ahead_behind`、`status`、`refs` |
| 历史与内容 | `log`、`log_filtered`、`authors`、`commit_details`、`commits_between`、`diff_between`、`patch`、`patch_for_path`、`numstat`、`blame`、`file_history`、`reflog` |
| 暂存区 | `stage`、`stage_all`、`unstage`、`unstage_all`、`discard`、`apply_cached`、`numstat_section`、`file_diff` |
| 提交与改写 | `commit`、`commit_with`、`reword_commit`、`squash_commits`、`cherry_pick`、`revert`、`reset_to`、`reset_hard`、`is_tip`、`can_rewrite` |
| 引用与配置 | `create_branch`、`checkout_new_branch`、`checkout_branch`、`delete_branch`、`rename_branch`、`merge`、`create_tag`、`create_tag_with`、`delete_tag`、`add_remote`、`remove_remote`、`set_remote_url`、`remote_urls`、`config_get`、`config_get_local`、`config_set`、`config_unset`、`stash_push`、`stash_push_with`、`stash_pop`、`stash_apply`、`stash_drop` |
| 同步 | `fetch`、`fetch_with`、`pull`、`pull_with`、`push`、`push_with`、`push_tag`、`upstream_incoming` |
| 冲突与多步操作 | `operation`、`conflict_content`、`conflicts`、`resolve_conflict`、`resolve_with`、`continue_operation`、`abort_operation` |
| 克隆与初始化 | `clone`、`clone_with`、`start_clone`、`init_repository`、`init_with` |
| 本机信息与底层执行 | `info`、`credential_helper`、`run_git` |

`Repo::refs` 返回 `Refs { branches, remotes, tags, stashes, submodules, worktrees }`。`Branch` 带 `current`、`ahead`、`behind`；`Stash` 的 `reference` 形如 `stash@{0}`。

## 选项

| 结构 | 字段 |
|---|---|
| `CommitOptions` | `amend`、`signoff`、`no_verify`、`allow_empty` |
| `FetchOptions` | `prune`、`tags`、`all_remotes` |
| `PullOptions` | `strategy`（`Merge` / `Rebase` / `FfOnly`）、`autostash`、`rebase_local` |
| `PushOptions` | `tags`、`force_with_lease`、`all_branches` |
| `CloneOptions` | `branch`、`depth`（0 表示完整克隆）、`recurse_submodules` |
| `InitOptions` | `gitignore`、`readme` |
| `LogFilter` | `query`、`author`、`since`、`until`、`reference` |

`PushOptions::force_with_lease` 对应 `--force-with-lease`，远端被别人改动时拒绝覆盖。`PullOptions::strategy` 对应 `--no-rebase`、`--rebase` 与 `--ff-only`。

## 克隆

`start_clone` 返回 `CloneRun`，用于不阻塞界面的克隆：

- `progress()` 读取 Git 最近一行进度；
- `try_finish()` 轮询结果，仍在运行时返回 `None`；
- `cancel()` 结束子进程，并删除本次创建的目录。

目标目录非空时直接报错，不覆盖已有内容。`clone_with` 是同步版本，返回 `Result<(), GitError>`。

## 测试

```sh
cd crates/git
cargo test
```

当前 59 项通过。测试在临时目录创建真实 Git 仓库，并用本机 bare 远程验证 fetch、pull 与 push，因此需要 PATH 中有 `git`，不依赖网络。覆盖范围包括暂存与 hunk/行级补丁、提交与钩子、固定形态改写、分支/标签/远程/贮藏、冲突解决与多步操作、同步选项与强推保护、克隆选项与取消、blame、reflog 与错误路径。

## 依赖

- [domain](../domain/README.md)（path 依赖）。
- 不依赖 `store`、`agent` 或界面 crate。

## 相关

- [项目 README](../../README.md)
- [客户端](../../apps/client/README.md)
