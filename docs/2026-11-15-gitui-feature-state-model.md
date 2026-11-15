# gitui 功能、Git 支持与页面状态说明

**状态**: working draft  
**日期**: 2026-11-15  
**画布**: 30 页，`designs/graft-git/`  
**配套测试脚本**: [交互验收手册](2026-11-14-gitui-ui-operation-test.md)  
**源文档**: [SPEC.md](../SPEC.md)、[USE-CASES.md](USE-CASES.md)、[UI-SPEC.md](UI-SPEC.md)、[TEST-PLAN.md](TEST-PLAN.md)  
**当前代码**: 全部计划阶段已实现；15 Sync / 27 Credentials 为 PARTIAL；16/22 为 N/A-DESIGN；auth sign-in 为 BLOCKED；179 项测试通过，详见测试文档第 9 节。

> 本文件位于 `docs/`，是功能与 Git 支持的工作说明，不是需求源。与 `SPEC.md`、`USE-CASES.md`、`UI-SPEC.md` 冲突时，以这些文档为准。  
> 本文件回答“支持哪些 Git 能力、页面有哪些状态、状态之间怎样转换、如何构造测试数据”。  
> 按钮级点击路径和逐页控件表在配套测试脚本中，两份文档分工使用。

---

## 0. 使用说明

### 0.1 目的

本文件用于：

- 对齐产品、设计、开发和测试对“支持功能”的理解。
- 说明每项 Git 能力的支持级别、UI 入口、失败路径和当前状态。
- 说明页面的默认、空、加载、有数据、错误、认证、操作进行中状态。
- 说明从空状态进入有数据状态的路径。
- 说明数据构造方法和验收标准。

本文件不替代：

- `SPEC.md` 的需求编号和版本范围。
- `docs/USE-CASES.md` 的行为流程。
- `docs/UI-SPEC.md` 的页面状态和 AppKit 控件。
- `docs/TEST-PLAN.md` 的自动化测试和 fixtures。

### 0.2 支持级别

| 级别 | 含义 |
|---|---|
| `v0.1` | v0.1 必须支持；没有该能力不能发布 |
| `v0.1.1` | 追溯、恢复、Refs 管理能力；v0.1 后提供 |
| `v0.2` | Agent 能力 |
| `v0.3 / P2` | 与同类产品对齐和差异化能力，v0.1 不做 |
| `SHOULD / 未排期` | SPEC 标记 SHOULD，但未进入当前 release |
| `设计 / 待定` | 画板有入口，但没有明确 R/UC 或排期 |
| `deferred` | 已明确延后，例如 interactive rebase |
| `not supported` | 明确不做 |

一个功能只有同时具备以下条件，才能标记为“支持”：

1. 有明确入口和前置条件。
2. 有成功结果和界面反馈。
3. 有失败、取消、重试或恢复路径。
4. 有数据后果说明，尤其是 destructive 操作。
5. 有自动化测试或手工验收记录。

只有菜单项、只有画板、或只有设计稿，都不能算支持。

### 0.2.1 实现状态

| 状态 | 含义 |
|---|---|
| `PASS` | 入口和真实行为已验证；测试文档第 9 节记录 |
| `PARTIAL` | 入口存在，能力有限或真实流程延后 |
| `N/A-DESIGN` | 组件规格页或状态画廊，不属于独立功能页 |
| `BLOCKED` | 已明确延后或尚未连接 |
| `未实现` | 当前代码没有实现，不能按支持验收 |

### 0.3 状态词

| 状态 | 含义 |
|---|---|
| `default` | 有数据时的默认状态 |
| `empty` | 无仓库、无提交、无变更、无结果 |
| `loading` | 首次加载、刷新、分页加载 |
| `populated` | 有数据且可操作 |
| `filtered-empty` | 有数据，但当前过滤无结果 |
| `error` | Git 命令失败、路径不可读、仓库损坏 |
| `auth` | 需要登录、凭据失效、SSH 失败 |
| `offline` | 网络不可用 |
| `operation` | merge/rebase/cherry-pick/revert 进行中 |
| `detached` | HEAD 不在分支上 |
| `busy` | hook、fetch、push、pull 正在执行 |
| `disabled` | 当前上下文不允许操作 |

### 0.4 当前实现边界

- 设计画板覆盖 30 页；当前代码已完成计划中的实现阶段，逐板状态见测试文档第 9 节。
- 15 Sync 和 27 Credentials 为 `PARTIAL`：选项和只读状态已实现，真实凭据流程延后。
- 16 Repository states 和 22 Menus & dialogs 为 `N/A-DESIGN`：前者是状态画廊，后者是组件规格。
- auth sign-in 为 `BLOCKED`；Interactive Rebase（无独立画板）按 ADR-0007 延后。
- 本文件中的 `v0.1`/`v0.1.1`/`v0.2` 是目标支持级别；当前状态以测试文档第 9 节和 `docs/README.md` 为准。

### 0.5 术语

| 术语 | 含义 |
|---|---|
| repo | 一个 Git 仓库 |
| tab | 一个打开仓库的工作区标签 |
| index | Git 暂存区；commit 的唯一来源 |
| staged | 已进入 index 的变更 |
| changelist | 应用侧分组；只做过滤，不决定提交范围 |
| hook | 仓库定义的 Git 生命周期脚本 |
| refs | branches、tags、remotes 等 Git 引用 |
| operation | merge、rebase、cherry-pick、revert 等进行中的状态 |

---

## 1. 产品范围

### 1.1 Release 划分

| Release | 页面 | 能力 |
|---|---|---|
| v0.1 | 01–06、10、11、14、15、16、18、20–21、23–29 | open/history/diff/stage/commit/hooks/sync/branch/merge/settings |
| v0.1.1 | 07、08、09、17、19、30 | compare、commit info、Refs manager、reflog、blame、tags/remotes/stashes |
| v0.2 | 12、13 | Agent 和权限 |
| v0.3 / P2 | 不新增页面 | LFS、GPG、PR、image diff、Git-flow、search、shelf/local history、patch apply、interactive rebase |

### 1.2 已支持、设计、延后、不支持

| 类别 | 内容 |
|---|---|
| 目标 v0.1 | 仓库打开/恢复、clone/init、history/diff、stage hunk/line、commit/amend、hooks、fetch/pull/push/auth、branch、merge/rebase/cherry-pick/revert、settings、palette/context |
| 目标 v0.1.1 | compare refs、commit info、Refs manager、reflog/undo、blame/file history、tags/remotes/stashes；固定形态 reword/squash/reset |
| 目标 v0.2 | Agent panel、Agent permissions |
| 设计但排期未定 | worktree、submodule |
| 延后 | interactive rebase（ADR-0007）；fixed-form 历史重写已实现，交互式编辑器未做 |
| 不做 | LFS、GPG、PR/notifications、image diff、Git-flow、code search、shelf/local history、patch apply、bisect、filter-repo、sparse checkout、partial clone、notes、replace refs、archive、bundle、远程仓库创建 |
| 明确禁止 | Agent 自动 push、Agent 修改历史、Agent 读取 secrets、Agent 使用 `--no-verify` |

### 1.3 关键产品约束

- **staged 是提交真相**：commit 只提交 index；changelist 只做过滤/分组。
- **失败是一等状态**：hooks、auth、network、conflict、non-fast-forward 都要有页面和恢复路径。
- **不丢数据**：destructive 操作必须确认；discard/hard reset 说明后果；reflog 可恢复。
- **原生优先**：AppKit 控件、系统字体、标准菜单、键盘/VoiceOver、dark mode。
- **Agent 有边界**：Ask first、per-repo 权限、无 secrets、无自动 push。
- **多 repo 默认**：恢复上次退出时全部 repo tab，不只恢复最后一个。
- **本地优先**：没有服务端；设置/变更列表本地持久化；凭据只 Keychain。

---

## 2. Git 能力矩阵

### 2.1 填写规则

- `目标级别` 按 SPEC release 和 R-ID；未明确的条目写 `设计 / 待定`。
- `当前画板` 只表示画板是否有对应界面，不表示已实现。
- 当前实现状态集中记录在 2.8.1；矩阵只表达目标支持级别和入口，避免把设计写成已实现。
- `Git 操作` 是用户能力对应的底层命令，具体参数矩阵见 `SPEC.md` Appendix B，本文不重复 flags。
- `R/UC` 是正式文档的追溯入口；没有 R/UC 的条目必须在排期前补充需求或删除入口。

### 2.2 仓库与读路径

| 能力 | Git 操作 | 目标级别 | UI 入口 | 关键失败/确认 | 当前画板 | R/UC |
|---|---|---|---|---|---|---|
| 打开本地仓库 | `rev-parse`、`status --porcelain=v2` | v0.1 | 01/03/04 | 不是 repo、路径不存在、bare | 01/03/04 | R-REPO-07，UC-01 |
| 恢复上次 session | session store、`rev-parse` | v0.1 | 23/04 | missing repo、损坏 repo | 23/04 | R-REPO-01/08/09，UC-01 |
| Clone | `git clone` | v0.1 | 03 | auth、目录非空、网络、进度 | 03 | R-REPO-05 |
| Init | `git init` | v0.1 | 03 | 非空目录、默认分支、.gitignore | 03 | R-REPO-06 |
| History 列表 | `log --graph --date-order` | v0.1 | 04 | 空仓库、加载、分页 | 04 | R-HIST-01，UC-02 |
| History 过滤 | `log` + branch/date/author/path/message | v0.1 | 05 | 无结果、无效日期 | 05 | R-HIST-01 |
| Commit diff | `show`、`diff` | v0.1 | 04/06/18 | 二进制、超大 diff | 04/06/18 | R-HIST-02/03 |
| 多 commit 聚合 diff | `diff A..B`、`log` | v0.1 | 06 | 选择为空、范围无效 | 06 | R-HIST-02 |
| File at commit | `show commit:path` | v0.1 | 18 | 文件不存在、二进制 | 18 | R-HIST-03 |
| Compare refs | `diff A...B` | v0.1.1 | 07 | 引用无效、空范围 | 07 | R-HIST-05 |
| Commit info | `show --no-patch` | v0.1.1 | 08 | 签名状态、parent 缺失 | 08 | R-HIST-04 |
| Blame / file history | `blame`、`log --follow` | v0.1.1 | 19 | 文件不存在、重命名 | 19 | R-HIST-06 |
| Refs 列表 | `for-each-ref` | v0.1 | 09/30 | 空 refs、remote 不可用 | 09/30 | R-HIST-07 |

### 2.3 Index、commit 与 hooks

| 能力 | Git 操作 | 目标级别 | UI 入口 | 关键失败/确认 | 当前画板 | R/UC |
|---|---|---|---|---|---|---|
| Stage file/hunk/line | `add -p`、`reset -p` | v0.1 | 10 | 二进制、index 过期、行级只文本 | 10 | R-STAGE-01/02/08，UC-03 |
| Unstage file/hunk/line | `reset -p` | v0.1 | 10 | 目标已变化 | 10 | R-STAGE-01 |
| Discard | `restore`、`checkout --` | v0.1 | 10 | 不可恢复；必须确认 | 10 | R-STAGE-03 |
| Partial commit | `commit`（只提交 index） | v0.1 | 10 | 无 staged、commit hook 失败 | 10 | R-STAGE-05/06 |
| Amend / sign-off / push after | `commit --amend`、`-s` | v0.1 | 10 | amend 无 HEAD、sign-off trailer | 10 | R-STAGE-05 |
| Changelist 分组 | 应用侧；不改变 Git index | v0.1 | 10/11 | 不能要求移动到 Default 才提交 | 10/11 | R-STAGE-04，UC-10 |
| Commit hooks | `pre-commit`、`prepare-commit-msg`、`commit-msg`、`post-commit` | v0.1 | 10 | stdout/stderr、exit code、超时、skipped | 10 | R-HOOK-01…08，UC-04 |
| Commit without hooks | `commit --no-verify` | v0.1 | 10 | 强确认；Agent 无此权限 | 10 | R-HOOK-02/06，UC-05 |
| Pre-push hook | `pre-push` | v0.1 / SHOULD | 15 | 阻止 push；日志在 sync activity | 15 | R-HOOK-07，UC-07 |
| 二进制/重命名/删除 | `status`、`add` | v0.1 | 10 | 行级 staging 只对文本 | 10 | R-STAGE-08 |

### 2.4 Sync 与认证

| 能力 | Git 操作 | 目标级别 | UI 入口 | 关键失败/确认 | 当前画板 | R/UC |
|---|---|---|---|---|---|---|
| Fetch | `fetch --prune --tags` | v0.1 | 28 | auth、offline、timeout、进度 | 28 | R-SYNC-01，UC-06 |
| Pull | `pull --rebase`/`--ff-only` | v0.1 | 29 | conflict、autostash 冲突、diverged | 29 | R-SYNC-02，UC-06 |
| Push | `push` | v0.1 | 15 | auth、non-fast-forward、protected、pre-push | 15 | R-SYNC-03，UC-07 |
| Force-with-lease | `push --force-with-lease` | v0.1 | 15 | 强确认；Agent 不自动 force | 15 | R-SYNC-03，UC-07 |
| HTTPS/SSH 认证 | credential helper、ssh-agent | v0.1 | 27 | 不把 token 写入 argv/日志 | 27 | R-SYNC-04/05 |
| 取消/后台继续 | 进程组、状态栏 | v0.1 | 15 | 关闭 sheet 不静默杀 push | 15 | R-SYNC-06 |

### 2.5 Branch、tag、remote、stash、worktree、submodule

| 能力 | Git 操作 | 目标级别 | UI 入口 | 关键失败/确认 | 当前画板 | R/UC |
|---|---|---|---|---|---|---|
| Branch checkout/new/rename/delete | `checkout`、`branch`、`switch` | v0.1 | 09/22 | 脏工作区、当前分支、detached | 09/22 | R-REF-01，UC-02 |
| Branch upstream/compare/merge/rebase | `branch --set-upstream-to`、`diff`、`merge`、`rebase` | v0.1 | 09/22 | 非 fast-forward、冲突、rebase 进行中 | 09/22 | R-REF-01 |
| Tag new/push/delete/details | `tag`、`push` | v0.1.1 | 30 | 重名、未推送、删除确认 | 30 | R-REF-02 |
| Remote add/edit/remove/fetch | `remote`、`fetch` | v0.1.1 | 30 | URL 校验、凭据、删除确认 | 30 | R-REF-03 |
| Stash create/apply/pop/drop/diff | `stash` | v0.1.1 | 30 | pop/drop 确认、冲突进入 Merge | 30 | R-REF-04 |
| Worktree list/open/new/remove/prune | `worktree` | SHOULD / 未排期 | 09/22 | detached、remove 确认 | 09/22 | R-REF-06 |
| Submodule list/update/sync/open/commit/remove | `submodule` | SHOULD / 未排期 | 09/22 | remove 确认、嵌套 repo | 09/22 | R-REF-05 |
| Refs manager | `for-each-ref` | v0.1.1 | 09/30 | 空 refs、并发写 | 09/30 | R-REF-07 |

### 2.6 Merge、冲突与恢复

| 能力 | Git 操作 | 目标级别 | UI 入口 | 关键失败/确认 | 当前画板 | R/UC |
|---|---|---|---|---|---|---|
| Merge | `merge` | v0.1 | 14 | 冲突、in-progress、hooks | 14 | R-MERGE-01…07，UC-08 |
| Rebase | `rebase` | v0.1 | 14 | 冲突、step 计数、skip | 14 | R-MERGE-01…07，UC-09 |
| Cherry-pick / revert | `cherry-pick`、`revert` | v0.1 | 21/14 | 冲突、确认 | 21/14 | R-MERGE-01…05 |
| Conflict editor | `show :1/:2/:3`、`add` | v0.1 | 14 | 未解决时 Continue 禁用 | 14 | R-MERGE-02/03/04 |
| Merge options | `--ff/--no-ff/--squash` | v0.1 基础 / SHOULD 选项 | 14/29 | 非 fast-forward、策略选择 | 14/29 | R-MERGE-06 |
| Interactive rebase | `rebase -i` | deferred | —（无画板） | ADR-0007；不在 v0.1 | — | R-MERGE-06 |
| Reflog | `reflog` | v0.1.1 | 17 | 恢复确认、working tree 策略 | 17 | R-REC-01，UC-11 |
| Undo reset/checkout | `reflog`、`reset`、`branch` | v0.1.1 | 17 | 不静默 hard reset | 17 | R-REC-02 |
| Reset soft/mixed/hard | `reset` | v0.1.1 / 固定形态 | 22 | 强确认；硬重置不可恢复 | 22 | 无明确 R/UC |
| Squash / edit message | `rebase`/`commit --amend` | v0.1.1 / 固定形态 | 22 | 历史重写确认 | 22 | 无明确 R/UC |
| Restore lost commit | `reflog`、`branch` | v0.1.1 | 17 | 创建分支或 reset | 17 | R-REC-01 |

### 2.7 Settings、Agent 与高级能力

| 能力 | Git 操作 | 目标级别 | UI 入口 | 关键失败/确认 | 当前画板 | R/UC |
|---|---|---|---|---|---|---|
| Commit identity | `config user.name/user.email` | v0.1 | 23/27 | 空值、全局/local 冲突 | 23/27 | R-SET-01/02 |
| Git executable/scan | `git --version`、目录扫描 | v0.1 | 24 | Git 缺失、版本过低 | 24 | R-SET-03 |
| Credentials/SSH/tools | credential helper、ssh、外部工具 | v0.1 | 27 | secrets、工具不存在 | 27 | R-SET-04 |
| Shortcuts | 应用侧 | v0.1 | 25 | 冲突、恢复默认 | 25 | R-SET-05 |
| Theme/language/date/launch/updates | 应用侧 | v0.1 | 23 | 立即生效或重启 | 23 | R-SET-01/06/07/08 |
| Command palette/context | 应用侧 + Git 命令 | v0.1 | 20/21 | 键盘、危险项确认 | 20/21 | R-NAV-01/02 |
| Agent | stage/commit/push 包装 | v0.2 | 12/13 | Ask first、无 no-verify、无 secrets | 12/13 | R-AGENT-01…06，UC-12 |
| Native/a11y/i18n | AppKit/NSH | v0.1 | 全局 | focus、VoiceOver、dark、语言 | 全局 | R-NAV-03/04 |
| Performance | 分页、虚拟化、并发控制 | v0.1 | 04/06/10 | 100k commit、10k diff | 04/06/10 | SPEC 12 |
| LFS/GPG/PR/image/Git-flow/search/shelf/patch | 各工具/扩展 | v0.3 / P2 | — | 不在 v0.1 | — | 非目标 |
| bisect/filter-repo/sparse/partial clone/notes/replace/archive/bundle/远程仓库创建 | 各命令/API | not supported | — | 不在范围 | — | 非目标 |

### 2.8 当前实现与支持决策

#### 2.8.1 当前实现状态

| 范围 | 状态 | 说明 |
|---|---|---|
| 01–14、17–21、23–26、28–30 | `PASS` | 入口和真实行为已验证；19 主区 blame 入口和 14 `ui::split` 是已知缺口 |
| 15 Sync | `PARTIAL` | 三个 tab 和选项已实现；无进度条，真实凭据流程延后 |
| 27 Credentials | `PARTIAL` | 凭据助手和 SSH 计数只读；Sign in 尚未连接账号 |
| 16 / 22 | `N/A-DESIGN` | 状态画廊 / 组件规格，不是独立功能页 |
| Interactive rebase | `BLOCKED` | 无独立画板；ADR-0007 延后；fixed-form reword/squash/reset 已实现 |
| worktree / submodule | `未实现` | 09/30 覆盖 branches/tags/remotes/stashes；这两类仍是设计入口 |
| Native / a11y | `PARTIAL` | A-04 `aria-label` 未全覆盖；`widgets.rs` 与 `ui` 两套尺寸模型并存 |
| 截图 / 像素比对 | `未完成` | 当前环境没有图形会话；证据是几何测试和主题测试 |

#### 2.8.2 支持决策

| 项目 | 当前状态 | 建议 |
|---|---|---|
| Reset / Squash / Edit message | 固定形态已实现（9.2 PASS） | 保留 v0.1.1；补充 R/UC 或明确由 ADR 覆盖；interactive 仍 deferred |
| Worktree / Submodule | 09/22 有入口，R-REF-05/06 SHOULD | 当前未实现；不要标记 v0.1，排到 v0.1.1 或 v0.3 |
| Merge options | Pull 三种模式已实现；Merge ff/no-ff/squash 需核对 | v0.1 基础 merge/rebase；选项按 R-MERGE-06 的 SHOULD 确定 |
| Pre-push hook | 15 `PARTIAL` | v0.1 显示失败日志；完整日志和独立验证放 v0.1.1 |
| Tag / remote / stash | 30 PASS，按 v0.1.1 | 已实现；保持 v0.1.1，不写成 v0.1 |
| Interactive rebase | `BLOCKED` | 保持 ADR-0007 延后；不把 fixed-form 当成 interactive |

---

## 3. 全局模型

### 3.1 仓库 session 与 tab

```
无 session
  → 02 Welcome (empty)
  → Open / Clone / Init
  → 04 Workspace（新建 tab）

有 session
  → 恢复全部 tab
  → 缺失 repo：banner（Locate / Remove）
  → 其他 repo 正常打开

打开多个 repo
  → 每个 tab 独立 controller
  → 切换 tab 保留 selection/scroll/diff
  → 关闭 tab 不删除 recent
```

规则：

- Restore 恢复全部 repo，不只恢复最后一个。
- 关闭 tab 不等于删除 recent。
- 每个 repo 的 staged、filters、selection 不串到其他 tab。
- 退出和 tab 变化时保存 session。

### 3.2 页面层级

| 层级 | 用途 | 例子 | 关闭方式 |
|---|---|---|---|
| 主页面 | 独立工作区 | 04 History、10 Changes、09 Refs、07 Compare | titlebar/tab |
| 页面内面板 | 同页切换 | History/Changes、Branches/Tags | 切换控件 |
| Popover | 锚定控件、无遮罩 | 05 filters、14 file tree | 点击外部或 Esc |
| Menu | 右键/下拉 | 21 context menu、22 branch menu | 点击外部或 Esc |
| Sheet | 需要上下文的操作 | 15 Sync、28 Fetch、29 Pull、13 permissions | Cancel/Close/Esc |
| Alert/确认 | destructive 或不可逆 | discard、hard reset、force push、no-verify | 必须选择按钮 |
| 独立窗口 | 设置/关于 | 23–27 Settings | Done/关闭 |
| 状态画廊 | 状态参考 | 16 Repository states | 不属于生产路由 |
| 组件规格 | 菜单/弹窗规格 | 22 Menus & dialogs | 不属于生产路由 |

层级规则：

- 同一时间只允许一个 modal（sheet 或 alert）。
- Alert 永远在 sheet 之上；Esc 先关闭最上层。
- Popover/menu 点击外部关闭；sheet 不因点击外部静默关闭。
- destructive 操作不能只用 toast，必须有确认。
- 关闭 sheet 不应静默杀掉后台 fetch/push；取消必须明确。

### 3.3 全局状态与转换

| 当前状态 | 触发 | 新状态 | 页面反馈 |
|---|---|---|---|
| 无仓库 | 启动且 Restore 有 repo | loading | 骨架 tab/历史 |
| 无仓库 | Show Welcome | 02 empty | No repositories yet |
| 无仓库 | Open/Clone/Init 成功 | 04 populated | 新 tab，History 选中 |
| 04 ready | 打开 repo 但无 commit | 04 empty | No commits yet |
| 04 loading | 加载完成 | 04 populated | 第一页 commit + diff |
| 04 populated | 过滤无结果 | filtered-empty | No commits match + Clear |
| 04 ready | branch/date/author 改变 | loading → populated | 列表和 count 更新 |
| 04 ready | merge/rebase 进行中 | operation | banner + Continue/Abort/Skip |
| 04 ready | auth 失败 | auth | banner + Sign in/Use SSH |
| 04 ready | 网络断开 | offline | 状态栏/横幅；网络操作禁用 |
| 04 ready | HEAD 脱离分支 | detached | banner + Create branch/Checkout |
| 10 ready | stage/unstage | 10 populated | staged/unstaged 计数更新 |
| 10 ready | commit 触发 hook | busy | Commit disabled；hook 面板 |
| 10 busy | hook 失败 | hook-failed | 日志、Retry、no-verify 确认 |
| 10 hook-failed | Retry 成功 | 10 ready | Commit 重新启用 |
| 15 ready | Push 开始 | pushing | 进度/日志；可后台/取消 |
| 15 pushing | auth 失败 | auth | Sign in/Use SSH；本地 commit 保留 |
| 15 pushing | non-fast-forward | rejected | Pull (rebase/merge)；不自动 force |
| 14 conflict | 全部 resolved | ready-to-continue | Apply & Continue 启用 |
| 14 ready-to-continue | Continue | hooks → 04 | merge/rebase 完成或 hook 失败 |
| 14 conflict | Abort | 04 ready | 恢复 pre-merge |
| 17 ready | Restore reflog | confirm | 恢复后 04 更新 |

### 3.4 工作区左侧

默认顺序：

1. `Branches`：当前分支、local branches、count。
2. `Remotes`：origin、remote branches。
3. `Tags`：本地/远程 tags。
4. `Stashes`：stash 列表。
5. `Submodules`：submodule 列表。
6. `Worktrees`：worktree 列表。

默认状态：

- 当前分支选中；`main` 在 04 中显示为选中行。
- 分区可折叠；计数与实际 refs 一致。
- 空分区是否隐藏：当前设计隐藏空内容，只保留有内容的 section；生产需要固定规则。
- 选中 branch 过滤 history；选中 tag/stash 显示对应内容。
- 右键分支/remote/tag/stash 打开 22 的上下文菜单。
- 切换 repo tab 后，左侧 selection 属于该 repo，不串状态。

### 3.5 History 与 Diff 模型

进入 04 时：

1. 默认模式 `History`；Changes 是同级切换。
2. 先显示 loading 骨架，不显示上一个 repo 的旧行。
3. 加载第一页 commit；显示 graph、message、refs、author、date。
4. 默认选中 HEAD 对应的 commit；画板中第一行 `3f2a1c9` 为选中示例。
5. 选中 commit 后加载该 commit 的 changed files 和 diff。
6. 单击 commit：单选并更新 diff。
7. Shift/⌘ 多选：进入 06 聚合 diff。
8. 单击右侧 changed file：只更新 diff，不改变 commit 选择。
9. 双击文件或菜单 `Open in Editor`：外部编辑器；不属于 diff 更新。
10. `Files` tab → 18；`Info` tab → 08；`Blame` → 19。

History 状态：

- `loading`：骨架；可取消。
- `empty`：No commits yet；引导 init/first commit。
- `populated`：commit 列表 + diff。
- `filtered-empty`：No commits match；Clear。
- `error`：仓库不可读；Retry/Close。
- `operation`：banner；冲突操作禁用。
- `detached`：banner；不显示当前 branch。
- `auth/offline`：顶部状态；读本地历史仍可用。

### 3.6 交互语义

| 操作 | 语义 |
|---|---|
| 单击 commit | 选中并加载 diff；不清空左侧 filter |
| Shift/⌘ 单击 commit | 多选；进入 06 聚合 diff |
| 单击文件 | 更新当前 diff；保留 commit 选择 |
| 双击文件 | 打开文件/外部编辑器；由设置决定 |
| 右键 commit | 21 上下文菜单 |
| 右键 branch/remote/tag/stash | 22 上下文菜单 |
| 单击 sidebar branch | 过滤 history；当前设计通过 branch chip/05，生产需一致 |
| 单击 filter/chip | 打开 05 popover；不跳页 |
| 点击外部 | 关闭 popover/menu；不关闭 sheet |
| Esc | 关闭最上层；有未保存修改时确认 |
| `⌘P` | 20 palette；画布未绑定 |
| `⌘O` | 打开仓库；画布未绑定 |
| `⌘⇧F` | 05 filters；画布未绑定 |
| `⌘⇧P` | 15 Push；画布未绑定 |
| `⌘⇧K` | 10 Commit；画布未绑定 |
| `⌘↵` | 当前主操作；画布未绑定 |

### 3.7 弹窗与确认规则

- **Popover**：过滤器、文件树、分支选择；不遮罩；点击外部关闭。
- **Menu**：commit/refs 右键；危险项用分隔线；确认后执行。
- **Sheet**：Sync、Fetch、Pull、Agent permissions；必须显式 Close/Cancel。
- **Alert**：discard、hard reset、delete、force push、`--no-verify`、abort operation；主按钮写动作和后果。
- **独立窗口**：Settings；Done 返回 04。
- 一次只开一个 modal；Alert 在 Sheet 之上。
- destructive 操作没有“Don’t ask again”。
- 操作进行中关闭 sheet 不是取消；取消必须点击 Cancel 并确认后台状态。

---

## 4. 页面状态

### 4.1 30 页状态矩阵

| # | 页面 | 默认/有数据 | 空 | 加载 | 错误/认证 | 操作中 | 进入有数据 |
|---|---|---|---|---|---|---|---|
| 01 | Welcome | recent + clone | — | — | 缺失/无效 repo | — | Open/recent |
| 02 | Welcome empty | no repo | No repositories yet | — | 扫描失败 | — | Open/Scan/Clone |
| 03 | Repository manager | repo 列表 + Clone | 无 recent | — | 非 repo、clone 失败 | clone 进度 | Open/Clone/Init |
| 04 | History & diff | commits + diff | No commits yet | skeleton | repo error/auth/offline | merge/rebase banner | 打开仓库 |
| 05 | History filters | filter 选项 | 无匹配 | — | 日期/作者无效 | — | 04 打开过滤 |
| 06 | Multi-commit diff | aggregate diff | 未选择 | loading | diff 失败 | — | 多选 commit |
| 07 | Compare refs | 两个 ref + 14 files | 空范围 | loading | 无效 ref | — | Compare |
| 08 | Commit info | metadata | 无 commit | loading | 签名/网络失败 | — | 选中 commit |
| 09 | Refs manager | branches 表 | 无 refs | loading | 写操作失败 | operation banner | 打开 Refs |
| 10 | Staging & commit | 3 staged/9 unstaged | clean | loading | hook fail/auth | hook running | 打开 Changes |
| 11 | Changelists | 3/6/3 groups | 无变更 | — | 写失败 | — | 打开 Changelists |
| 12 | Agent panel | plan | 无变更 | thinking | Agent 错误 | running | 打开 Agent |
| 13 | Agent permissions | 权限表 | 无权限 | — | 保存失败 | — | Ask first |
| 14 | Merge conflict | 3 files/3 conflicts | 无冲突 | loading | 保存失败 | merge/rebase | 冲突产生 |
| 15 | Sync & auth | Push | 无 remote | pushing | auth/non-FF/offline | push | 打开 Sync |
| 16 | Repository states | 6 states | — | — | — | — | 状态参考 |
| 17 | Reflog & undo | 42 entries | 无 reflog | loading | 恢复失败 | — | 打开 Reflog |
| 18 | Files at commit | 文件 + source | 无文件 | loading | 文件不可读 | — | Files tab |
| 19 | Blame & file history | blame + history | 无历史 | loading | 文件不可读 | — | 18 Blame |
| 20 | Command palette | repos + commands | 无匹配 | — | — | — | overflow/⌘P |
| 21 | Dark & context menu | commit menu | 未选中 commit | — | — | — | 右键 commit |
| 22 | Menus & dialogs | 12 组件 | — | — | — | — | 组件规格 |
| 23 | Settings | General | 默认值 | — | 保存失败 | — | gear |
| 24 | Settings Git | path/scan | 未检测 | scanning | Git 缺失 | — | 23 Git |
| 25 | Shortcuts | 行 + keys | 默认 | — | 冲突 | recording | 23 Shortcuts |
| 26 | About | 版本 | — | — | 链接失败 | — | 23 About |
| 27 | Credentials | 账号/SSH | 未连接 | loading | auth/secret 错误 | — | 23/04 Sign in |
| 28 | Sync Fetch | Fetch 表单 | 无 remote | fetching | auth/offline | fetch | 15 Fetch |
| 29 | Sync Pull | Pull 表单 | 无 upstream | pulling | conflict/auth | pull | 15 Pull |
| 30 | Refs other | Tags/Remotes/Stashes | 空 refs | loading | 写失败 | — | 09 details |

### 4.2 核心流程

#### 4.2.1 Welcome 空状态到有数据

```
02 Welcome empty
  ├─ Open… → 文件选择 → 04 Workspace
  ├─ Scan a Folder… → 24 Settings Git → 扫描 → 03
  ├─ Clone… → 03 Clone → 成功 04
  └─ Init… → 03 Init → 成功 04

01 Welcome (recent)
  ├─ recent repo → 04
  ├─ Open… → 04
  └─ Settings → 23

启动 Restore last session
  └─ 04，恢复全部 tab；缺失 repo 显示 Locate/Remove
```

Welcome 空状态显示：

- 左侧 `Repositories` 标题和 `No repositories yet`。
- 右侧克隆表单：URL、Directory、Clone Repository。
- `Open…` 在左侧标题右侧；`Scan a Folder…` 指向 24。
- 不显示假的 recent 行。

进入有数据状态：

- Open 本地 repo：校验路径后创建 tab，进入 04。
- Clone：成功后创建 tab，进入 04。
- Init：成功后创建 tab，进入 04。
- Restore：启动时恢复全部 repo；缺失项保留 banner，其他 tab 正常。

#### 4.2.2 Workspace 左侧、History 和 Diff

```
04 打开
  → loading：骨架
  → empty：No commits yet
  → populated：History 默认，加载第一页
  → 选中 HEAD
  → 加载 commit 文件树 + diff
  → 单击 commit：更新 diff
  → 单击文件：更新 diff
  → 多选 commit：06 聚合
  → 过滤：05 → Apply → 04 filtered
```

左侧显示：

- Branches、Remotes、Tags、Stashes、Submodules、Worktrees。
- 当前分支选中；计数与 refs 一致。
- branch/tag/stash 的右键菜单来自 22。
- 选中 branch 过滤 history；选中 tag/stash 显示对应内容。

History：

- 默认 History 模式，不通过按钮“打开”；打开 repo 后立即加载。
- loading 时显示骨架；empty 时显示 No commits yet；filtered-empty 显示 No commits match。
- 单击 commit：diff；多选：06；右键：21；Files：18；Info：08。

Diff：

- 默认 split；可切 unified、fold、word highlight、whitespace。
- 文件树/列表切换只改变导航，不改变 commit 选择。
- 只读 diff 不允许编辑；文件 at commit 走 18。

#### 4.2.3 Staging、Commit、Hooks

```
10 打开
  → clean：无变更
  → populated：3 staged / 9 unstaged
  → stage hunk/line → 计数更新
  → 输入 message
  → Commit → hook running（Commit disabled）
  → hook failed：日志 + Retry + no-verify 确认
  → Retry success：Commit 重新启用
  → 成功：toast/刷新；未 staged 保留
```

规则：

- commit 只提交 index；changelist 不决定范围。
- hook 失败阻止 commit；后续 hook 标记 skipped。
- `Commit without hooks` 必须强确认；Agent 没有该入口。
- post-commit 失败不回滚 commit，显示 warning。
- hook 修改 index 后重新读取 `git status`。

#### 4.2.4 Sync：Fetch / Pull / Push

```
15 Push
  ├─ Fetch → 28
  ├─ Pull → 29
  └─ Push → pushing/日志/取消/后台
       ├─ auth fail → Sign in/Use SSH → 27
       ├─ non-FF → Pull (rebase/merge) → 29
       └─ success → 04 更新 ahead/behind/history
```

- Fetch 不改工作区；只更新 remote refs。
- Pull 支持 merge/rebase/ff-only/autostash；冲突进 14。
- Push 支持 tags/upstream/force-with-lease；force 需确认。
- 关闭 sheet 不静默杀操作；Cancel 需要明确。

#### 4.2.5 Merge / Rebase 冲突

```
merge/rebase 开始
  → 14 冲突视图
  → 三方：Base / Ours / Result / Theirs
  → 编辑 Result 或 Accept
  → Mark Resolved → git add
  → 全部解决 → Apply & Continue
  → hooks → 04
  → Abort → 恢复 pre-merge
```

- 未解决时 Continue disabled。
- 关闭/取消不丢失 Result；未解决时确认。
- Skip 只在 rebase 中出现。
- 冲突期间的 branch/reset 操作被阻止。

#### 4.2.6 Refs、Reflog、Compare

```
09 Branches
  ├─ Checkout → 04 更新
  ├─ New Branch → 22
  ├─ Compare → 07
  ├─ Merge/Rebase → 14
  ├─ Delete → 确认 → 09 刷新
  └─ Open details → 30

17 Reflog
  ├─ 选中 reset → Undo
  ├─ Restore → 确认 → 04
  ├─ Create branch → 确认/表单 → 04
  └─ Show in History → 04

07 Compare
  → 两个 ref → 2-dot/3-dot → 文件树 + diff
  → Swap → 重新计算
```

- branch 删除不能删当前分支。
- tag/remote/stash 删除必须确认。
- reflog restore 不静默 hard reset；working tree 策略明确。

---

## 5. 数据状态与构造

### 5.1 基线 fixture

| 项目 | 值 |
|---|---|
| 主 repo | `gitui`，`~/Developer/gitui` |
| tabs | 3：gitui、docs-site、atlas-web |
| recent | 6；3 open |
| commits | 128 |
| branches | 5 local + origin/main |
| tags | 3 |
| stashes | 2 |
| submodules | 2 |
| worktrees | 3 |
| changes | 12：3 staged / 9 unstaged |
| reflog | 42 |
| ahead/behind | 2 / 0；Pull 状态 2 / 2 |
| conflict | 3 files / 3 conflicts；2 unresolved |
| hook | pre-commit exit 1 |
| locale | English / 简体中文；2026-11-14 |

### 5.2 状态构造

| 目标状态 | 构造方法 | 预期页面 |
|---|---|---|
| 无 repo / 02 | 清空 session 和 recent；首次启动 | 02 |
| 有 recent / 01 | 打开 3 个 repo，保存 session | 01 |
| 空仓库 / 04 empty | `git init`，无 commit | 04 No commits yet |
| 有数据 / 04 | fixture repo；128 commits；打开 | 04 History |
| 过滤为空 / 05 | 选择不存在的 author 或非法日期 | 05 No commits match |
| 3 staged / 9 unstaged | 修改 12 文件；`git add` 3 个 | 10 |
| hook 失败 | `.git/hooks/pre-commit` 写入 `exit 1` 并 `chmod +x` | 10 hook failed |
| hook 成功 | 同上，脚本 `exit 0` 并输出日志 | 10 Commit enabled |
| auth 失败 | 使用错误 HTTPS 凭据或无效 SSH key | 04/15/27 |
| non-fast-forward | 本地和 remote 各有一个新 commit | 15 rejected |
| merge 冲突 | 两个分支修改同一行；merge | 14 |
| rebase 冲突 | 同上；`pull --rebase` 或 rebase | 14 |
| detached HEAD | `git checkout <hash>` | 04/16 |
| reflog reset | `git reset --hard HEAD~1`，打开 17 | 17 |
| tags/remotes/stashes | 创建 3 tags、1 remote、2 stashes | 09/30 |
| 缺失 repo | 打开 session 后删除路径 | 启动 banner |
| 非 repo | 打开普通目录 | 03 错误 |
| bare repo | `git clone --bare` | 04 只读 banner |
| offline | 断网后 fetch/push | 15/16 |
| loading | 打开大 repo 或延迟 fixture | 04 skeleton |

约束：

- 不使用真实 token、私钥或用户隐私；使用占位凭据。
- 测试后清理临时目录、session 和 stash，避免共享状态。
- 时间固定为 2026-11-14 Asia/Shanghai；测试时冻结时区。
- fixtures 的完整策略参考 `docs/TEST-PLAN.md`。

---

## 6. 功能验收

### 6.1 支持定义

一个 Git 能力要标记为“支持”，必须验证：

1. 入口可达，前置条件明确。
2. 正向操作成功，数据结果正确。
3. 失败/取消/重试/恢复路径完整。
4. destructive 操作有确认和后果说明。
5. 操作后相关页面、计数、状态栏刷新。
6. 有测试记录（自动或手工）。

### 6.2 领域验收

| 领域 | 验收条件 | 测试 |
|---|---|---|
| 仓库 | 打开、restore 全部 tab、missing repo、非 repo、bare | TC-01/02/03 |
| History/Diff | 加载、空、过滤、多选、文件 diff、compare、info、blame | TC-04/05 |
| Index/Commit | stage file/hunk/line、partial commit、discard、amend、changelist 不改变范围 | TC-06 |
| Hooks | 4 类 commit hooks、日志、失败阻止、Retry、post-commit warning、no-verify 确认 | TC-07 |
| Sync/Auth | fetch、pull、push、auth、non-FF、progress、cancel、后台 | TC-08 |
| Merge | 三方、编辑 Result、Mark、Continue、Abort、Skip、hooks | TC-09 |
| Refs | branch/tag/remote/stash 表、创建/删除/刷新、确认 | TC-12 |
| Reflog | 恢复、create branch、working tree 策略 | TC-10 |
| Settings | identity、Git path、credentials、SSH、tools、shortcuts、theme/lang/date/launch | TC-13 |
| Agent | Ask first、权限、plan、approve、无 no-verify、无 secrets | TC-11 |
| Native/A11y | focus、label、键盘、dark、英文/中文 | TC-14 |

### 6.3 不可接受的“假支持”

- 只有按钮，没有 Git 操作。
- 只有成功 toast，没有失败/取消路径。
- destructive 操作没有确认。
- hook 失败仍允许 commit。
- Agent 自动 push 或读取 secrets。
- tag/remote/stash 只显示菜单，没有真实列表和刷新。
- 把 22 的组件卡片当成生产页面。

---

## 7. 追溯与维护

### 7.1 追溯

- 需求编号：`SPEC.md` 的 R-*。
- 行为流程：`docs/USE-CASES.md` 的 UC-*。
- 页面状态：`docs/UI-SPEC.md`。
- 测试：`docs/TEST-PLAN.md` 和配套 `2026-11-14-gitui-ui-operation-test.md`。
- 本文件的矩阵行必须引用 R/UC；没有引用的行在排期前补充 R/UC；fixed-form reset/squash 已实现，仍需补追溯。

### 7.2 待决策

- worktree/submodule 是否进入 v0.1.1 或 v0.3；当前未实现。
- Merge 的 ff/no-ff/squash 选项边界；Pull 三种模式已实现。
- pre-push hook 的完整日志与独立验证范围。
- fixed-form reword/squash/reset 的 R/UC 追溯，避免只有实现没有需求编号。
- tag/remote/stash 已按 v0.1.1 实现，保持版本口径。

### 7.3 维护规则

- 本文件是 `docs/` 中的功能与 Git 支持说明；正式行为以 `SPEC.md`、`USE-CASES.md`、`UI-SPEC.md` 为准。
- 支持级别或 release 变化时，先更新 `SPEC.md` 和对应 ADR/UC，再更新本文件。
- 新增按钮或页面时，同时更新本文件的页面状态和配套测试脚本。
- 不在本文件记录真实 token、私钥、用户隐私或未脱敏日志。
- 测试发现缺口时标记画板编号、阻塞原因和影响范围。
