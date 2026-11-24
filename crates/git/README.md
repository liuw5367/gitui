# git

Git 执行层。仓库里只有这里拼装并运行 `git` 命令，并把输出解析成 `domain` 的类型。crate 名与 `git` 命令同名；不链接 libgit2。兼容入口默认调用 PATH 中的 `git`；客户端可为每个仓库创建指定 `GitExecutable` 的 `Repo`，同一仓库的查询、写操作、同步和关联仓库查询都沿用该 executable。

## 执行约定

- 每条命令带 `-C <工作树>`，参数以独立 argv 传入，不拼接 shell 字符串。
- 读取命令设置 `GIT_TERMINAL_PROMPT=0` 与 `GIT_OPTIONAL_LOCKS=0`：没有配置 AskPass 时凭据缺失会直接失败，只读命令不拿可选锁。客户端可通过 `GitExecutable::with_env` 给单次操作附加 AskPass 环境，credential helper 仍由 Git 优先调用。stdout 超过 8 MiB 截断，错误详情截断到前 4096 个字符。
- fetch、pull、push、merge 等同步调用把 stdout 与 stderr 合并，成功后取最后 3 条有效行作为摘要；失败时把合并输出截断到前 4096 个字符作为 `detail`。后台同步仍把两路合并供日志显示，同时独立保留 stdout 状态记录；Push 使用 porcelain 行，Fetch 由执行前后的本地远端引用与 tag 快照判断变化。
- 命令以非零状态退出时返回 `GitError::CommandFailed`，保留程序名、参数、退出码与截断后的输出。
- `Repo::open` 使用 PATH 中的 `git`；`Repo::open_with_executable` 接收明确的 executable，并把该选择保存在 `Repo` 内。`GitExecutable::with_env` 可在不改变程序路径的前提下给进程及其 Git 子进程附加环境变量。`init_with_executable`、`init_repository_with_executable`、`clone_with_executable`、`start_clone_with_executable` 与 `info_with` 也可显式选择程序。设置值由客户端传入，本 crate 不读取应用配置。
- `reword_commit` 与 `squash_commits` 自行生成 rebase todo，并用临时脚本覆盖 `GIT_SEQUENCE_EDITOR` 与 `GIT_EDITOR`；命令失败后执行 `git rebase --abort`，不把仓库留在改写中间状态。`rebase_onto` 执行普通分支变基，冲突时保留 Git 的 rebase 状态供界面解决。

## 打开仓库

`Repo::open` 与 `Repo::open_with_executable` 用 `git rev-parse --show-toplevel` 校验工作树：

- 路径不存在或不可读时返回 `GitError::Unreadable`；
- 路径存在但不是仓库时返回 `GitError::NotARepository`；
- 通过后保存工作树根目录，后续命令都在该目录上运行。

## API

| 分组 | 方法 |
|---|---|
| 打开与状态 | `Repo::open`、`Repo::open_with_executable`、`GitExecutable`、`root`、`name`、`summary`、`abbrev_branch`、`head_state`、`ahead_behind`、`status`、`refs`、`ref_oid_snapshot`、`sync_ref_snapshot`、`upstream_ref`、`head_oid`、`resolve_commit_oid`、`commit_oids_between`、`is_ancestor` |
| 历史与内容 | `log`、`log_filtered`、`log_all`、`log_history`、`resolve_commit`、`authors`、`commit_details`、`commits_between`、`merge_base`、`diff_between`、`diff_between_with_whitespace`、`diff_trees_with_whitespace`、`diff_worktree_with_whitespace`、`patch`、`patch_with_whitespace`、`patch_for_path`、`patch_for_path_with_whitespace`、`numstat`、`blame`、`file_history`、`reflog` |
| 暂存区 | `stage`、`stage_all`、`unstage`、`unstage_all`、`discard`、`apply_cached`、`numstat_section`、`file_diff`、`file_diff_with_whitespace` |
| 提交与改写 | `commit`、`commit_with`、`reword_commit`、`squash_commits`、`cherry_pick`、`revert`、`rebase_onto`、`reset_to`、`reset_hard`、`is_tip`、`can_rewrite` |
| 引用与配置 | `create_branch`、`checkout_new_branch`、`checkout_tracking_branch`、`checkout_branch`、`delete_branch`、`rename_branch`、`merge`、`create_tag`、`create_tag_with`、`delete_tag`、`add_remote`、`remove_remote`、`set_remote_url`、`remote_urls`、`remote_info`、`config_get`、`config_get_local`、`config_set`、`config_unset`、`stash_push`、`stash_push_with`、`stash_push_selected`、`stash_show`、`stash_show_with_whitespace`、`stash_pop`、`stash_apply`、`stash_drop` |
| 子模块 | `SubmoduleInfo`、`update_submodule`、`sync_submodule`、`remove_submodule` |
| 工作树 | `WorktreeInfo`、`add_worktree`、`remove_worktree`、`prune_worktrees` |
| 同步 | `fetch`、`fetch_with`、`start_fetch_with`、`pull`、`pull_with`、`start_pull_with`、`pull_remote_branch`、`push`、`push_with`、`start_push_with`、`push_branch`、`delete_remote_branch`、`push_tag`、`upstream_incoming`、`parse_push_porcelain`、`PushRefUpdate`、`SyncRun` |
| 冲突与多步操作 | `operation`、`conflict_content`、`conflict_worktree_content`、`conflict_sides`、`conflicts`、`resolve_conflict`、`resolve_conflict_bytes`、`resolve_conflict_delete`、`resolve_with`、`continue_operation`、`abort_operation` |
| 克隆与初始化 | `clone`、`clone_with`、`clone_with_executable`、`start_clone`、`start_clone_with_executable`、`init_repository`、`init_repository_with_executable`、`init_with`、`init_with_executable` |
| 本机信息与底层执行 | `info`、`info_with`、`credential_helper`、`credential_helper_with` |

`diff_between`、`patch`、`patch_for_path`、`file_diff` 返回已解析的 `domain::diff::Patch`；Git 的原始文本保存在 `Patch::raw` 里，复制直接用它，保证字节一致。带 `with_whitespace` 的方法可附加 Git `--ignore-all-space`，只改变展示查询；提交差异固定以第一父提交为基准，根提交与空 tree 比较，merge commit 不使用 combined diff。

`diff_between_with_whitespace(base, target, ...)` 用 `git diff <base> <target>` 比较两端 tree；它不计算 `git log base..target` 的提交集合。`diff_trees_with_whitespace` 是这项 tree 比较的明确入口。`merge_base(left, right)` 返回共同祖先 OID，无共同祖先时返回 `None`。`diff_worktree_with_whitespace(revision, ...)` 比较指定 revision 与当前 index/工作区，覆盖已暂存和未暂存的 tracked 文件；Git diff 不包含未跟踪文件。

`Repo::refs` 返回 `Refs { branches, remotes, remote_info, remote_branches, tags, stashes, submodules, worktrees }`。`ref_oid_snapshot` 返回全部非符号引用到 OID 的映射，可在不读取提交历史的情况下检测当前分支之外的引用是否移动；`sync_ref_snapshot` 仍只包含远端跟踪引用与 tags，供同步前后比较。`RemoteInfo` 保存全部 fetch/push URL；`RemoteBranch` 分开保存远端名和远端内的分支名，`full_name()` 返回 `origin/feature/topic` 形式。远端 HEAD 符号别名不会作为远端分支返回。`Branch` 带 `current`、`ahead`、`behind`；`Stash` 保存显示序号 `reference`、提交 OID `oid` 与摘要，操作可按 OID 在序号变化后重新定位。`SubmoduleInfo` 保存 `.gitmodules` 的路径、URL、可选跟踪分支，以及初始化状态、当前提交、父仓库 index 中的 gitlink 提交和子模块自身是否有改动。`update_submodule(path, false)` 初始化并检出父仓库记录的版本，`update_submodule(path, true)` 要求配置跟踪分支并按 `--remote` 更新；`sync_submodule` 同步配置 URL，`remove_submodule` 会拒绝删除含本地改动或未记录提交的已初始化子模块。`WorktreeInfo` 记录关联工作树路径、主工作树标记、分支、锁定、失效登记、目录存在状态及工作区是否干净；`add_worktree` 可检出现有分支或从起点新建分支，`remove_worktree` 只移除干净、未锁定、非主工作树的登记路径且不使用强制选项，`prune_worktrees` 立即清理 Git 标为失效的登记。`stash_push_selected` 以 literal pathspec 只保存指定路径，`stash_show` 与 `stash_show_with_whitespace` 可读取包含未跟踪文件的差异，后者支持忽略空白差异。

`rebase_onto` 把当前分支变基到指定引用；若 Git 因冲突退出，仓库会保留 rebase 状态供界面继续解决。`checkout_tracking_branch` 创建并检出跟踪指定远端分支的本地分支。`pull_remote_branch` 拉取指定远端分支并合入当前分支。`push_branch` 推送指定本地分支到远端同名分支，不要求当前检出该分支；`delete_remote_branch` 通过非强制删除 refspec 移除服务器分支。

`conflict_sides(path)` 从 `git ls-files -u` 读取 index stage 1/2/3，再用对象 ID 读取各侧原始 blob；缺失 stage 返回 `None`，不把删除侧伪装成空文件。`conflict_worktree_content` 读取当前工作区字节。解决文本或二进制内容时，`resolve_conflict_bytes` 只写回并暂存给定路径；`resolve_conflict_delete` 以 `git rm` 暂存删除。`operation()` 通过 `git rev-parse --git-dir` 查找操作标记，因此也适用于 `.git` 是文件的关联工作树。

`log_all` 返回所有引用可达的完整历史，并把 `git stash list` 中的每个 stash OID 加入范围。`log_history` 默认从全部引用与 stash 读取；`LogFilter.references` 可用多个本地或远端 ref 做 OR 范围，旧的单引用 `reference` 仍可用。它支持多作者 email、日期端点、literal 文件/目录 pathspec 与提交消息子串；条件维度之间按 AND 组合。`log_history_graph` 只应用引用范围，给筛选后的提交图重连祖先使用。完整历史读取不会套用普通 `run_git` 的 8 MiB stdout 截断。`authors_with_email` 返回按 email 去重的作者身份，`authors` 保留只取显示名的接口。日志解析包含 author email 和完整提交消息；`resolve_commit` 对完整引用追加 `^{commit}` 后调用 `rev-parse`，可把 annotated tag 解析到其 commit。

`Repo::blame(path, revision)` 按给定 revision 读取行归属，并用 `--follow` 回溯重命名。`Repo::file_history(path, revision, limit)` 同样限定 revision 并跟随重命名，返回 `FileHistoryEntry`：`path` 是该提交中实际存在的路径，`deleted` 标记该提交是否删除了文件。界面查看删除提交时，内容 blame 使用该提交的第一父版本；其他历史项使用提交本身。工作区入口明确传 `HEAD`，不会把历史提交文件误读成当前版本。

## 选项

| 结构 | 字段 |
|---|---|
| `CommitOptions` | `amend`、`signoff`、`no_verify`、`allow_empty` |
| `FetchOptions` | `prune`、`tags`、`all_remotes` |
| `PullOptions` | `strategy`（`Merge` / `Rebase` / `FfOnly`）、`autostash`、`rebase_local` |
| `PushOptions` | `tags`、`force_with_lease`、`all_branches` |
| `CloneOptions` | `branch`、`depth`（0 表示完整克隆）、`recurse_submodules` |
| `InitOptions` | `gitignore`、`readme`、`license`、`name`、`branch` |
| `LogFilter` | 快速查询、单引用/多引用、单作者/多作者 email、since/until、literal path、消息子串 |

`PushOptions::force_with_lease` 对应 `--force-with-lease`，远端被别人改动时拒绝覆盖。`PullOptions::strategy` 对应 `--no-rebase`、`--rebase` 与 `--ff-only`。

## 克隆

`start_clone` 返回 `CloneRun`，用于不阻塞界面的克隆：

- `progress()` 读取 Git 最近一行进度；
- `try_finish()` 轮询结果，仍在运行时返回 `None`；
- `cancel()` 结束子进程，并删除本次创建的目录。

目标目录非空时直接报错，不覆盖已有内容。`clone_with` 是同步版本，返回 `Result<(), GitError>`。

## 后台同步

`start_fetch_with`、`start_pull_with` 与 `start_push_with` 返回 `SyncRun`，供界面在后台运行 Git 同步命令：

- 子进程启用 Git 的 `--progress`，分别捕获 stdout 与 stderr；`progress()` 返回当前合并日志，`status_output()` 返回 stdout 状态记录，二者最多保留约 256 KiB。`status_output_truncated()` 用于识别状态记录是否被截断。
- `start_push_with` 使用 `--porcelain`；`parse_push_porcelain` 解析成功、创建、强制更新、删除、已最新与拒绝的 ref 行，忽略人类进度文字。Fetch 通过 `sync_ref_snapshot()` 比较 `refs/remotes` 与 `refs/tags` 的完整 OID，过滤 `origin/HEAD` 符号别名。`upstream_ref()`、`head_oid()` 与 `commit_oids_between()` 支持 Pull 的前后端点和去重提交数确认。
- `try_finish()` 非阻塞地检查进程；运行中返回 `None`，结束后返回 `Some(Result<String, GitError>)`。成功值是 Git 输出摘要，失败保留命令与输出详情。
- 同步命令在独立进程组运行；Git 退出或取消时会停止输出读取器，避免遗留辅助进程占住输出管道后卡住完成与取消。
- `cancel()` 终止同步进程组、等待退出，并返回已捕获的输出；已完成的远端或工作区变更不会由执行层回滚。
- 命令关闭终端交互提示；默认认证由系统 Git 与已配置的 credential helper 处理，客户端按需提供 AskPass 时也使用 Git 原生认证流程。

旧的 `fetch_with`、`pull_with` 与 `push_with` 同步方法继续供非交互调用使用。

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

测试在临时目录创建真实 Git 仓库，并用本机 bare 远程验证 fetch、pull 与 push，因此需要 PATH 中有 `git`，不依赖网络。覆盖范围包括补丁解析、暂存与 hunk/行级补丁、提交与钩子、固定形态改写、分支/标签/远程/贮藏/子模块/工作树、冲突解决与多步操作、同步选项与强推保护、Push porcelain 状态解析与 stdout/stderr 分流、推送未检出的分支、普通分支变基与变基冲突处理、按指定远端跟踪分支创建/拉取/删除、克隆选项与取消、blame、reflog 与错误路径。

## 依赖

- [domain](../domain/README.md)（path 依赖）。
- 不依赖 `store`、`agent` 或界面 crate。

## 相关

- [项目 README](../../README.md)
- [客户端](../../apps/client/README.md)
