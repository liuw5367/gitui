# agent

Agent 的模型层：把仓库状态整理成可审阅的计划，按权限决定哪些步骤可以执行，并记录审计。crate 不启动进程、不读写凭据、不直接调用 Git；执行由应用层完成，因此 Agent 的权限边界与手动操作一致。

## 动作与权限

共有 9 个动作，顺序与设置页一致：`StageAll`、`Commit`、`UnstageAll`、`Fetch`、`Pull`、`Push`、`Discard`、`ReadFiles`、`ForcePush`。

| 权限 | 含义 |
|---|---|
| `Allow` | 模型允许不经询问执行；默认只给 `Fetch` 与 `ReadFiles` |
| `Ask` | 模型要求执行前询问；其余 7 个动作的默认值 |
| `Deny` | 模型禁止执行 |

`Policies` 按动作保存权限，未出现的动作取 `Action::default_permission`；`may_run` 用 `Allow` 判断能否直接执行。当前应用把可执行步骤（`Ask` 与 `Allow`，排除 `Deny` 与受保护分支上的写动作）在 Approve & Run 里统一列出并确认。`Action::is_read_only` 标记 `Fetch` 与 `ReadFiles`；`Action::is_destructive` 标记 `Discard`、`Push`、`Commit` 与 `ForcePush`，界面据此区分危险动作。

## 计划

- `Step::new(action, label)` 创建一步；`with_argument` 附加参数，例如提交说明。
- `Plan::new(summary, steps)`、`push_step`、`remove_step` 编辑计划；删除只影响这份建议，不回滚已执行的步骤。
- `Plan::runnable(policies, settings, branch)` 返回可以执行的步骤下标，跳过 `Deny` 的动作与受保护分支上的写动作；Approve & Run 用它列出范围。
- `Plan::touches_worktree` 判断计划是否包含除 `Fetch` 之外的步骤。
- `propose(summary, staged, unstaged, ahead, behind, commit_message)` 按固定规则生成计划：有未暂存改动先暂存全部；有已暂存改动（或有未暂存改动且提供了提交说明）则提交；落后远程则拉取；领先远程则推送。没有步骤时说明仓库干净。

## 设置与审计

- `Settings { automation, review_millis, protected }`；默认 `Automation::Off`、预览 8000 毫秒、`main` 受保护。
- `Automation` 有 `Off`、`Commit`、`CommitAndPush` 三档；`may_push` 只对 `CommitAndPush` 返回真。
- `blocks(action, branch)` 返回阻止原因：只读动作不受受保护分支影响，写动作在受保护分支上会被挡住。
- `Audit` 最多保留最近 200 条，最新的在前；`AuditEntry` 记录时间、动作、结果与详情。

## 测试

```sh
cd crates/agent
cargo test
```

当前 12 项通过，覆盖默认权限、允许与拒绝、受保护分支、计划顺序与增删、自动化对推送的限制、审计上限、干净仓库不产生步骤，以及设置的读写。

## 边界

- 计划只描述动作，不执行动作；[crates/git](../git/README.md) 负责运行 Git 命令，[apps/client](../../apps/client/README.md) 负责在权限内调用它。
- 不读取 token、SSH 密钥或环境凭据；凭据相关的动作不在 `Action` 列表里。
- 受保护分支与 `Deny` 在 `runnable` 阶段过滤，应用层仍应遵守同一结果。

## 相关

- [项目 README](../../README.md)
