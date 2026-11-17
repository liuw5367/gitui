# git

Git 执行层。仓库里只有这里拼装并运行 `git` 命令，并把输出解析成 `domain` 的类型。crate 名与 `git` 命令同名；运行时调用 PATH 中的系统 Git，不链接 libgit2。

## 执行约定

- 每条命令带 `-C <工作树>`，参数以独立 argv 传入，不拼接 shell 字符串。
- 读取命令设置 `GIT_TERMINAL_PROMPT=0` 与 `GIT_OPTIONAL_LOCKS=0`：凭据缺失时直接失败，只读命令不拿可选锁。stdout 超过 8 MiB 截断，错误详情截断到前 4096 个字符。
- fetch、pull、push、merge 等操作的进度与结果写在 stderr，执行层把 stdout 与 stderr 合并，成功后取最后 3 条有效行作为摘要；失败时把合并输出截断到前 4096 个字符作为 `detail`。
- 命令以非零状态退出时返回 `GitError::CommandFailed`，保留程序名、参数、退出码与截断后的输出。
- `reword_commit` 与 `squash_commits` 自行生成 rebase todo，并用临时脚本覆盖 `GIT_SEQUENCE_EDITOR` 与 `GIT_EDITOR`；命令失败后执行 `git rebase --abort`，不把仓库留在改写中间状态。`rebase_onto` 执行普通分支变基，冲突时保留 Git 的 rebase 状态供界面解决。

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
| 提交与改写 | `commit`、`commit_with`、`reword_commit`、`squash_commits`、`cherry_pick`、`revert`、`rebase_onto`、`reset_to`、`reset_hard`、`is_tip`、`can_rewrite` |
| 引用与配置 | `create_branch`、`checkout_new_branch`、`checkout_tracking_branch`、`checkout_branch`、`delete_branch`、`rename_branch`、`merge`、`create_tag`、`create_tag_with`、`delete_tag`、`add_remote`、`remove_remote`、`set_remote_url`、`remote_urls`、`remote_info`、`config_get`、`config_get_local`、`config_set`、`config_unset`、`stash_push`、`stash_push_with`、`stash_push_selected`、`stash_show`、`stash_pop`、`stash_apply`、`stash_drop` |
| 子模块 | `SubmoduleInfo`、`update_submodule`、`sync_submodule`、`remove_submodule` |
| 工作树 | `WorktreeInfo`、`add_worktree`、`remove_worktree`、`prune_worktrees` |
| 同步 | `fetch`、`fetch_with`、`pull`、`pull_with`、`pull_remote_branch`、`push`、`push_with`、`push_branch`、`delete_remote_branch`、`push_tag`、`upstream_incoming` |
| 冲突与多步操作 | `operation`、`conflict_content`、`conflicts`、`resolve_conflict`、`resolve_with`、`continue_operation`、`abort_operation` |
| 克隆与初始化 | `clone`、`clone_with`、`start_clone`、`init_repository`、`init_with` |
| 本机信息与底层执行 | `info`、`credential_helper`、`run_git` |

`diff_between`、`patch`、`patch_for_path`、`file_diff` 返回已解析的 `domain::diff::Patch`；Git 的原始文本保存在 `Patch::raw` 里，导出 patch 与复制直接用它，保证字节一致。

`Repo::refs` 返回 `Refs { branches, remotes, remote_info, remote_branches, tags, stashes, submodules, worktrees }`。`RemoteInfo` 保存全部 fetch/push URL；`RemoteBranch` 分开保存远端名和远端内的分支名，`full_name()` 返回 `origin/feature/topic` 形式。远端 HEAD 符号别名不会作为远端分支返回。`Branch` 带 `current`、`ahead`、`behind`；`Stash` 保存显示序号 `reference`、提交 OID `oid` 与摘要，操作可按 OID 在序号变化后重新定位。`SubmoduleInfo` 保存 `.gitmodules` 的路径、URL、可选跟踪分支，以及初始化状态、当前提交、父仓库 index 中的 gitlink 提交和子模块自身是否有改动。`update_submodule(path, false)` 初始化并检出父仓库记录的版本，`update_submodule(path, true)` 要求配置跟踪分支并按 `--remote` 更新；`sync_submodule` 同步配置 URL，`remove_submodule` 会拒绝删除含本地改动或未记录提交的已初始化子模块。`WorktreeInfo` 记录关联工作树路径、分支、锁定、失效登记、目录存在状态及工作区是否干净；`add_worktree` 可检出现有分支或从起点新建分支，`remove_worktree` 只移除干净、未锁定的登记路径且不使用强制选项，`prune_worktrees` 立即清理 Git 标为失效的登记。`stash_push_selected` 以 literal pathspec 只保存指定路径，`stash_show` 可读取包含未跟踪文件的差异。

`rebase_onto` 把当前分支变基到指定引用；若 Git 因冲突退出，仓库会保留 rebase 状态供界面继续解决。`checkout_tracking_branch` 创建并检出跟踪指定远端分支的本地分支。`pull_remote_branch` 拉取指定远端分支并合入当前分支。`push_branch` 推送指定本地分支到远端同名分支，不要求当前检出该分支；`delete_remote_branch` 通过非强制删除 refspec 移除服务器分支。

## 选项

| 结构 | 字段 |
|---|---|
| `CommitOptions` | `amend`、`signoff`、`no_verify`、`allow_empty` |
| `FetchOptions` | `prune`、`tags`、`all_remotes` |
| `PullOptions` | `strategy`（`Merge` / `Rebase` / `FfOnly`）、`autostash`、`rebase_local` |
| `PushOptions` | `tags`、`force_with_lease`、`all_branches` |
| `CloneOptions` | `branch`、`depth`（0 表示完整克隆）、`recurse_submodules` |
| `InitOptions` | `gitignore`、`readme`、`license`、`name`、`branch` |
| `LogFilter` | `query`、`author`、`since`、`until`、`reference` |

`PushOptions::force_with_lease` 对应 `--force-with-lease`，远端被别人改动时拒绝覆盖。`PullOptions::strategy` 对应 `--no-rebase`、`--rebase` 与 `--ff-only`。

## 克隆

`start_clone` 返回 `CloneRun`，用于不阻塞界面的克隆：

- `progress()` 读取 Git 最近一行进度；
- `try_finish()` 轮询结果，仍在运行时返回 `None`；
- `cancel()` 结束子进程，并删除本次创建的目录。

目标目录非空时直接报错，不覆盖已有内容。`clone_with` 是同步版本，返回 `Result<(), GitError>`。

## 初始化

`init_repository` 用 `main` 建仓库；`init_with` 按 `InitOptions` 指定分支、README 标题与附带文件：

- `branch` 为空时回退 `main`；
- `name` 为空时用目录名作 README 标题；
- `gitignore` 写 macOS + Swift 的最小集合；
- `license` 写 MIT，版权人依次取 `git config user.name`、`user.email`，都为空时写 `The <name> authors`；
- 已存在的同名文件不覆盖。

## 测试

```sh
cd crates/git
cargo test
```

测试在临时目录创建真实 Git 仓库，并用本机 bare 远程验证 fetch、pull 与 push，因此需要 PATH 中有 `git`，不依赖网络。覆盖范围包括补丁解析、暂存与 hunk/行级补丁、提交与钩子、固定形态改写、分支/标签/远程/贮藏/子模块/工作树、冲突解决与多步操作、同步选项与强推保护、推送未检出的分支、普通分支变基与变基冲突处理、按指定远端跟踪分支创建/拉取/删除、克隆选项与取消、blame、reflog 与错误路径。

## 依赖

- [domain](../domain/README.md)（path 依赖）。
- 不依赖 `store`、`agent` 或界面 crate。

## 相关

- [项目 README](../../README.md)
- [客户端](../../apps/client/README.md)
