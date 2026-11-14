# gitui — 产品与技术规格（SPEC）

**状态**：Draft for technical spike  
**最后更新**：2026-11-14  
**工作名**：gitui（最终名在 M7 前确定）  
**设计源**：`designs/graft-git/project/canvas.json`（30 个画板，7 个功能组 + Details）  
**目标平台**：macOS 14+，Apple Silicon 为主；Intel 不在 v0.1 范围  
**实现形态**：Native Shell（NSH）原生客户端，**全部使用 Rust 开发**（NSH Rust API + AppKit 互操作）；不使用 TSX/JS/HTML 桌面壳

---

## 0. 如何使用本文档

- 本文档是 **v0.1 的实现契约**，不是概念稿。每个需求有 ID；实现、测试、验收都引用 ID。
- 设计画板是视觉与状态参考；**不能用 HTML/CSS 版本直接上线**。生产 UI 必须使用 NSH 的 Rust API（底层为 AppKit 原生控件），不使用 TSX/JS。
- 任何“这样做是否可行”的问题，先用 **Appendix A 的 Spike 清单**回答，再写 UI。
- 没有 TBD。存在多种选择的地方写成 **Decision（已选）** 或 **Proposed default（默认，允许在指定的里程碑改）**。
- 变更本文档时，在文末加一条 Changelog；不要静默改变已锁定的决策。

### 0.1 Companion docs

| 文档 | 内容 |
|---|---|
| [SPIKE-REPORT.md](docs/SPIKE-REPORT.md) | M0 实测结果、环境、通过/未通过项 |
| [USE-CASES.md](docs/USE-CASES.md) | 13 条可执行用例（主流程 + 失败路径 + 验收） |
| [UI-SPEC.md](docs/UI-SPEC.md) | 6 个核心屏幕的状态矩阵、动作、键盘、NSH 控件 |
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | NSH + Rust 单进程架构、服务、序列、线程、错误 |
| [TEST-PLAN.md](docs/TEST-PLAN.md) | fixture、分层测试、R-ID 追踪、发布门禁 |
| [DESIGN-HANDOFF.md](docs/DESIGN-HANDOFF.md) | 画板 → NSH/AppKit 控件、token、SF Symbols、本地化 |
| [AGENT-THREAT-MODEL.md](docs/AGENT-THREAT-MODEL.md) | Agent 权限、prompt injection、密钥与审计边界 |
| [ADR](docs/adr/README.md) | 7 条已锁定的架构决策记录 |

---

## 1. 决策记录（已锁定）

| ID | 决策 | 理由 |
|---|---|---|
| D1 | **Index-first**：`git add` 的 staged 内容是唯一提交来源；changelist 不决定提交范围 | 支持 hunk/line 级暂存与 partial commit；与 Fork 行为一致 |
| D2 | **Changelist 是应用侧分组/过滤器**，不是 Git 对象，不需要把文件移到 Default 才能提交 | 保留 IDEA 的组织习惯，同时不污染 index 语义 |
| D3 | **v0.1 使用系统 Git CLI** 执行写操作、hooks、认证、submodule/worktree；读操作也先走 CLI | hooks、credential helper、SSH、`.gitconfig`、rebase 状态都能直接复用，风险最低 |
| D4 | **直接签名 + notarize 分发**，不做 Mac App Store | Git CLI、SSH agent、Keychain、hooks 在 MAS 沙箱里限制过多 |
| D5 | **Agent 是 v0.2 功能**，默认 Ask first，不自动 push | 先把 Core Git 做对；Agent 依赖 staging/commit/hooks/permissions |
| D6 | **生产 UI 使用 NSH Rust API + AppKit 原生控件**；不使用 TSX/JS/HTML；画布只作视觉规格 | 原生焦点、键盘、VoiceOver、菜单、sheet、vibrancy；单一 Rust 技术栈 |
| D7 | **Commit hooks 必须有日志**：运行中、失败、重试、`--no-verify`，失败时 commit 被阻止 | 这是 Git 客户端可信度的一部分，不能只显示“失败” |
| D8 | **Launch 恢复上次所有 repo 的 tab**，不是只恢复最后一个；也可选择 Show Welcome | 多 repo 工作流 |
| D9 | **Interactive rebase 不在 v0.1**；当前画布不含该页；v0.2 再补设计与实现 | 范围控制；squash/rebase 菜单先保留 |
| D10 | **v0.1 不做** LFS、GPG 签名、PR/notifications、image diff、Git-flow、code search、shelf/local history、patch apply | 这些是 P2；不阻塞“每天不用终端” |
| D11 | **单一 Rust 代码库**：UI、状态、Git 服务、持久化、Agent 全部 Rust；NSH 提供 Rust 窗口/视图/菜单/事件循环；不引入 JS/TS 运行时 | 降低桥接复杂度、统一类型/错误/取消；NSH 官方支持 Rust 直接开发 |

---

## 2. Executive summary

gitui 是一个 **macOS 原生、index-first 的 Git 客户端**，面向同时维护多个 repo 的开发者。它把 Fork 的“快、原生、写路径完整”与 IDEA 的 changelist/冲突/rebase 工作流结合起来，并提供一个 **受权限约束的 AI Agent**。

v0.1 的目标不是“功能最多”，而是：

> 打开 repo → 看历史/diff → 精确 stage → commit（含 hook 日志）→ fetch/pull/push → branch/merge → 设置身份/凭据，全程不需要终端；失败可恢复，不丢数据。

设计画板已经覆盖 30 个页面/状态。**继续画页面的收益已经很低**；下一个风险是 Git 正确性、性能、hooks/冲突的真实行为，以及 **NSH 的 Rust API 是否能承载代码/合并编辑器**。

---

## 3. 目标、成功标准、护栏

### 3.1 v0.1 成功标准

- 开发者可以在 gitui 内完成 **90% 的日常 Git 操作**，不需要打开终端。
- 所有写操作都有 **明确的 staged 范围**、可读的错误、可恢复路径。
- 一个失败的 `pre-commit` 能在 commit 面板看到完整日志，并可以 Retry 或显式 `--no-verify`。
- merge/rebase/cherry-pick 冲突可以在三方编辑器中编辑并 continue/abort。
- 100k commits / 10k 行 diff / 1GB repo 下仍可用。
- 5 名 pilot 开发者连续使用 2 周；无数据丢失；无凭据泄漏。
- 崩溃/强退后，session、changelist、设置可恢复；index 不损坏。

### 3.2 非目标（v0.1）

- 不做 IDE、不做代码补全、不做完整代码搜索。
- 不做 Windows/Linux；不做插件系统；不做多窗口。
- 不做 LFS/GPG/PR/notifications/Git-flow/image diff/shelf/local history/patch apply。
- 不做 Agent 自动 push；不做 Agent 修改历史；不做无审批的 destructive 操作。
- 不做 Mac App Store 版本。

### 3.3 护栏

- 任何 destructive 操作（hard reset、discard、force push、删除 branch/tag、删除 worktree、`--no-verify`）必须二次确认，并记录审计日志。
- 凭据只进 Keychain；不进 SQLite、不进日志、不进 Agent 上下文。
- hooks 是仓库代码；只在用户发起 commit/push 时由 Git 执行；不能静默禁用或静默 `--no-verify`。
- Agent 的权限是 per-repo、可撤销、可审计；默认 Ask first。

---

## 4. 用户与核心场景

### 4.1 用户

- **多 repo 的个人开发者**：每天在 3–8 个 repo 之间切换，关心速度、键盘、diff、commit 精度。
- **Review/维护者**：需要看历史、blame、compare、冲突解决。
- **Agent 辅助用户**：希望 Agent 做清理/提交/推送，但必须可控、可审。

### 4.2 核心任务

1. 从 Welcome/Repo Manager 打开、clone、init 一个 repo；恢复上次所有 repo。
2. 在 History 中过滤、比较、查看 commit/files/diff。
3. 在 Changes 中按 hunk/line 暂存，commit（hooks 可能失败），处理失败并重试。
4. Fetch/Pull/Push，处理认证、non-fast-forward、网络失败。
5. 创建/切换 branch、tag，管理 remote/stash。
6. 解决 merge/rebase/cherry-pick 冲突，continue/abort。
7. 用 reflog 恢复误操作；用 blame/file history 追溯。
8. 配置身份、凭据、SSH、外部工具；设置快捷键。
9. （v0.2）让 Agent 在权限内 stage/commit/push，审阅它的计划和日志。

---

## 5. 范围与发布

### 5.1 Release 划分

| Release | 内容 | 目的 |
|---|---|---|
| **v0.1** | 01–06、10、11、14、15、16、18、20–21、23–29；Core Git（open/history/diff/stage/commit/hooks/sync/branch/merge/settings） | 可每天使用的 Git 客户端 |
| **v0.1.1** | 07 Compare、08 Commit info、09 Refs manager、17 Reflog、19 Blame、30 Refs other | 追溯/恢复/管理补齐 |
| **v0.2** | 12 Agent、13 Agent permissions | Agent beta |
| **v0.3** | P2：LFS、GPG、PR/notifications、image diff、Git-flow、search、shelf/local history、patch apply、interactive rebase | parity/差异化 |

### 5.2 Board map（30 页）

| # | 画板 | 组 | 角色 | 目标 |
|---|---|---|---|---|
| 01 | Welcome | Start | App | v0.1 |
| 02 | Welcome (empty) | Start | App | v0.1 |
| 03 | Repository manager | Start | App | v0.1 |
| 04 | History & diff | Review | App | v0.1 |
| 05 | History filters | Review | App | v0.1 |
| 06 | Multi-commit diff | Review | App | v0.1 |
| 07 | Compare refs | Review | App | v0.1.1 |
| 08 | Commit info | Review | App | v0.1.1 |
| 09 | Refs manager | Review | App | v0.1.1 |
| 10 | Staging & commit | Worktree | App | v0.1 |
| 11 | Changelists | Worktree | App | v0.1 |
| 12 | Agent panel | Worktree | App | v0.2 |
| 13 | Agent permissions | Worktree | App | v0.2 |
| 14 | Merge conflict | Operations | App | v0.1 |
| 15 | Sync & auth | Operations | App | v0.1 |
| 16 | Repository states | Operations | State gallery | v0.1 |
| 17 | Reflog & undo | Operations | App | v0.1.1 |
| 18 | Files at commit | Files | App | v0.1 |
| 19 | Blame & file history | Files | App | v0.1.1 |
| 20 | Command palette | Navigation | App | v0.1 |
| 21 | Dark & context menu | Navigation | App | v0.1 |
| 22 | Menus & dialogs | Navigation | Component sheet | 设计规格 |
| 23 | Settings | Settings | App | v0.1 |
| 24 | Settings · Git | Settings | App | v0.1 |
| 25 | Settings · Shortcuts | Settings | App | v0.1 |
| 26 | Settings · About | Settings | App | v0.1 |
| 27 | Settings · Credentials & tools | Settings | App | v0.1 |
| 28 | Sync · Fetch | Details | App modal | v0.1 |
| 29 | Sync · Pull | Details | App modal | v0.1 |
| 30 | Refs · Tags, Remotes, Stashes | Details | App | v0.1.1 |

- **22** 是组件/菜单规格页，不是独立功能页；生产用 NSMenu/NSAlert/Sheet。
- **16** 是状态 gallery；生产要把这些状态放进 04/06/15 的 in-context banner。
- **28/29** 是真实 Sync sheet，不是卡片页；**30** 是 Refs 管理器页面。

---

## 6. 产品原则

1. **Staged 是真相**：commit 只提交 index；changelist 只过滤/分组。
2. **失败是一等状态**：hooks、auth、network、conflict、non-fast-forward 都要有页面和恢复路径，不能只 toast。
3. **不丢数据**：destructive 操作二次确认；discard/hard reset 说明后果；reflog 可恢复。
4. **原生优先**：AppKit 控件、系统字体、标准菜单、keyboard/VoiceOver、dark mode；不要用 HTML 控件仿造。
5. **Agent 有边界**：Ask first、per-repo 权限、无 secrets、无自动 push、可审计。
6. **多 repo 是默认**：tab 恢复所有上次 repo，不是只恢复最后一个。
7. **速度是功能**：CLI 并发受控、diff 虚拟化、history 分页；不要一次加载全部。
8. **本地优先**：没有服务端；设置/变更列表本地持久化；凭据 Keychain。

---

## 7. 功能需求

> 约定：`MUST` = v0.1 必须；`SHOULD` = v0.1.1；`MAY` = v0.2+。  
> 每条需求可测试；括号内是画板编号。

### 7.1 Repository lifecycle（01–03）

- **R-REPO-01 (MUST)** 启动时，用户可选择 **Show Welcome** 或 **Restore last session**；Restore 要恢复上次退出时 **所有 repo 的 tab 及顺序**，不是只恢复最后一个。（23）、（01）
- **R-REPO-02 (MUST)** Welcome 显示 recent repos、Open Repository、Clone Repository；Open 从左侧 Repositories 标题右侧进入，不在右侧顶部重复放 Clone。（01）
- **R-REPO-03 (MUST)** 空状态显示 “No repositories yet”，提供 Open、Clone、Scan a Folder。（02）
- **R-REPO-04 (MUST)** Repository Manager：recent 列表、路径、当前 branch、last opened；Open/Clone/Init；删除 recent 只删列表不删磁盘。（03）
- **R-REPO-05 (MUST)** Clone：URL、directory、name、branch、depth（Full/1/50）、recurse submodules、open after clone；认证复用 Credentials；显示 progress、可 cancel、失败可读。（03）
- **R-REPO-06 (MUST)** Init：directory、name、default branch、.gitignore template、README；不覆盖非空目录；失败可读。（03）
- **R-REPO-07 (MUST)** Open 校验路径是 Git repo/worktree；不是 repo 时给出 Init/Open parent；路径不存在时提示移除 recent。
- **R-REPO-08 (MUST)** Multiple tabs：每个 repo 独立状态；关闭 tab 不删除 repo；退出保存所有 tab；启动恢复所有 tab。
- **R-REPO-09 (MUST)** 上次 session 中某个 repo 丢失时，启动时提示 “Missing repository”，可 Locate/Remove；不能静默丢弃其他 repo。
- **R-REPO-10 (SHOULD)** Repo Manager 支持 group/pin、查看 worktree；不做远程 repo 创建。

### 7.2 Read path（04–09、18–19）

- **R-HIST-01 (MUST)** History：graph、commit message、refs、author、date；支持 branch 过滤、date range、author/path/message refine；分支多选；筛选结果 count；Reset/Apply。（04、05）
- **R-HIST-02 (MUST)** 点击 commit 显示 commit diff；多选 commit 显示 aggregate diff + changed-file tree；点击文件显示 diff。（04、06）
- **R-HIST-03 (MUST)** Diff：split/unified、fold、word highlight、whitespace toggle、tree/list；file at commit；read-only source。（04、06、18）
- **R-HIST-04 (SHOULD)** Commit info：hash、parents、tree、refs、author/committer、dates、signature status、message；Copy SHA/Open on GitHub/Cherry-pick/Revert（后两者写操作需确认）。（08）
- **R-HIST-05 (SHOULD)** Compare refs：选两个 branch/tag/commit；two-dot/three-dot；ahead/behind；files/diff；Swap；只读。（07）
- **R-HIST-06 (SHOULD)** Blame：每行最后 commit/author/time；选中行显示 commit；file history 列表。（19）
- **R-HIST-07 (SHOULD)** Refs manager：Branches/Tags/Remotes/Stashes 表格；upstream、ahead/behind、last commit；详情。（09、30）
- **R-HIST-08 (MUST)** 大 repo：history 分页/增量，不能一次加载 100k commit；diff 按文件/行虚拟化。

### 7.3 Write path / Index（10、11）

- **R-STAGE-01 (MUST)** Working tree 分为 **Staged / Unstaged**；支持 file、hunk、line 级 stage/unstage；支持 stage all/unstage all。（10）
- **R-STAGE-02 (MUST)** 行级 staging 必须只影响目标 hunk/line；不能污染 index；操作后刷新 staged/unstaged 计数。
- **R-STAGE-03 (MUST)** File context：Revert、Open in Editor、Reveal in Finder、Copy Path、Move to Changelist、Discard；destructive 确认。（10）
- **R-STAGE-04 (MUST)** Changelist：应用侧分组；文件可移动；filter/active 只影响新文件归属，不影响 commit；不能要求移动到 Default 才能提交。（10、11）
- **R-STAGE-05 (MUST)** Commit composer：显示 staged 文件；summary/description；amend、sign-off、push after commit；commit 只对 staged；无 staged 时禁用；提示 “只有 staged 文件会被提交”。（10）
- **R-STAGE-06 (MUST)** 部分提交：只提交选中的 staged hunks/lines；commit 后未 staged 的修改仍在工作区。（10）
- **R-STAGE-07 (MUST)** Index 刷新：外部修改、hooks 修改 index、branch checkout 后，staged/unstaged 必须重新读取，不能缓存过期。
- **R-STAGE-08 (MUST)** 二进制/重命名/删除/新增文件有正确状态；行级 staging 仅对文本；二进制走 file-level。

### 7.4 Commit hooks（10）

- **R-HOOK-01 (MUST)** commit 触发的 hooks 必须显示在 commit 面板：`pre-commit`、`prepare-commit-msg`、`commit-msg`、`post-commit`；显示每个 hook 的 status、duration、exit code。
- **R-HOOK-02 (MUST)** 失败时：commit 被阻止；日志显示 stdout/stderr、命令、exit code；后续 hook 标记 skipped；主 Commit 按钮禁用；提供 **Retry hooks** 和 **Commit without hooks**（显式确认，`--no-verify`）。
- **R-HOOK-03 (MUST)** `post-commit` 失败不回滚 commit；显示 warning 和日志；不能假装成功。
- **R-HOOK-04 (MUST)** hooks 输出有上限（1 MB/次）；完整日志写临时文件；UI 可 View full log；日志不得包含凭据（脱敏 URL userinfo/token）。
- **R-HOOK-05 (MUST)** hooks 超时（默认 5 分钟）可取消；取消要 kill 进程组；commit 不执行。
- **R-HOOK-06 (MUST)** hooks 是仓库代码；第一次在该 repo commit 时提示 “此仓库定义了 hooks”；不自动 `--no-verify`；Agent 没有 `--no-verify` 权限。
- **R-HOOK-07 (SHOULD)** push 的 `pre-push` hook 日志显示在 Sync 的 activity log，不与 commit hooks 混淆。
- **R-HOOK-08 (MUST)** hook 失败后重新读取 `git status`；如果 hook 修改了 index，UI 要反映新的 staged 状态。

### 7.5 Sync / auth（15、28、29）

- **R-SYNC-01 (MUST)** Fetch：remote、prune、tags、all remotes；progress/log；不改工作区。
- **R-SYNC-02 (MUST)** Pull：merge/rebase/ff-only、autostash、rebase local commits；ahead/behind；冲突进入 Merge 流程。
- **R-SYNC-03 (MUST)** Push：remote/branch、set upstream、tags、force-with-lease（确认）；progress/log；non-fast-forward/auth 失败有可操作提示。
- **R-SYNC-04 (MUST)** 认证：HTTPS 走系统 credential helper/Keychain；SSH 走 ssh-agent/known_hosts；`GIT_TERMINAL_PROMPT=0`；不把 token 放进 argv/日志。
- **R-SYNC-05 (MUST)** 认证失败/网络离线/远程拒绝/非快进要有独立错误态；提供 Retry、Sign in、Use SSH、Copy diagnostic。
- **R-SYNC-06 (MUST)** 操作可取消/后台继续；关闭 sheet 不应悄悄杀掉 push；取消要明确。
- **R-SYNC-07 (MUST)** 成功后刷新 branch/ahead-behind/history；失败不改变本地 commit。

### 7.6 Branch / tag / remote / stash / worktree（09、22、30）

- **R-REF-01 (MUST)** Branch：checkout、new、rename、delete（确认）、upstream set、compare、merge、rebase；显示 current/ahead/behind。
- **R-REF-02 (SHOULD)** Tag：new（annotated/lightweight）、push、delete（确认）、details；本地/远程区分。
- **R-REF-03 (SHOULD)** Remote：add/edit/remove（确认）、fetch/push URL、fetch now；凭据走 7.5。
- **R-REF-04 (SHOULD)** Stash：create、apply、pop、drop（确认）、diff；冲突进入 Merge 流程。
- **R-REF-05 (SHOULD)** Submodule：list、update、sync、open、commit change；remove 确认。
- **R-REF-06 (SHOULD)** Worktree：list、open、new、remove（确认）、prune；detached HEAD 明确显示。
- **R-REF-07 (SHOULD)** Refs manager：Branches/Tags/Remotes/Stashes 表格与详情；所有写操作复用 R-REF。
- **R-REF-08 (MUST)** 所有 ref 写操作后刷新 history/sidebar/tab 状态；并发操作按 repo 串行。

### 7.7 Merge / rebase / cherry-pick / revert（14）

- **R-MERGE-01 (MUST)** 检测 in-progress 操作（merge/rebase/cherry-pick/revert）；显示 banner（Continue/Abort/Skip）和冲突数。
- **R-MERGE-02 (MUST)** 冲突编辑器：Base/Ours/Theirs + **可编辑 Result**；Accept Ours/Theirs/Both；Mark Resolved；文件列表。
- **R-MERGE-03 (MUST)** 保存 result 后 `git add`；未解决时 Continue 禁用；Abort 恢复；Skip 仅 rebase。
- **R-MERGE-04 (MUST)** 冲突文件必须从 unmerged stages 读取（`:1:`/`:2:`/`:3:`），不能只解析工作区 markers。
- **R-MERGE-05 (MUST)** merge/rebase/cherry-pick/revert 的 commit 也走 hooks（R-HOOK）；冲突解决后 commit 的 hook 失败要能重试。
- **R-MERGE-06 (SHOULD)** 支持 merge 选项（ff/no-ff/squash）与 rebase 选项（autostash）；interactive rebase 不在 v0.1。
- **R-MERGE-07 (MUST)** 冲突/操作中禁止切换 branch 或 reset，除非先 abort；UI 要明确阻止。

### 7.8 Recovery / trace（17、19、07、08）

- **R-REC-01 (SHOULD)** Reflog：列出 HEAD/branch reflog；选择 entry 查看 commit；Restore（创建 branch 或 reset）需确认；不删除 reflog。
- **R-REC-02 (SHOULD)** Undo 最近 reset/checkout：从 reflog 恢复；working tree 处理策略明确；不静默 hard reset。
- **R-REC-03 (SHOULD)** Blame/File history：按行/文件查看；跳转 commit；打开 diff。
- **R-REC-04 (SHOULD)** Compare refs 全屏；Refs manager；Commit info。

### 7.9 Agent（12、13，v0.2）

- **R-AGENT-01 (MUST)** 默认 **Ask first**；每个 repo 独立权限：read/stage/commit/push/force/discard；automation Off/Commit/Commit & Push；protected branches。
- **R-AGENT-02 (MUST)** Agent 计划必须先展示：要 stage 哪些文件、commit message、是否 push；Approve & Run 才执行；可 Edit/Cancel。
- **R-AGENT-03 (MUST)** Agent 只能操作 staged/index 的可见范围；不能 `--no-verify`；不能 force push protected branch；不能读 Keychain secrets。
- **R-AGENT-04 (MUST)** 日志/审计：工具调用、命令、结果、错误；可查看/复制；失败可重试；不记录 token。
- **R-AGENT-05 (MUST)** 数据边界：发往模型的内容需明确；默认不发送完整 repo；可配置；敏感文件排除。
- **R-AGENT-06 (MUST)** Agent 不自动 push；push 需权限 + 审批；失败不重试到无限。

### 7.10 Settings（23–27）

- **R-SET-01 (MUST)** General：Theme（Light/Dark/Auto 三按钮）、Language（English/简体中文，默认 English）、Date format（Relative/Absolute/System/Custom pattern，无单独 Time format）、File tree、On launch（Show Welcome / Restore last session + Last session 列表）、Commit identity（Name/Email/Use global config）、Updates（select + Check Now）。（23）
- **R-SET-02 (MUST)** Identity 是唯一提交身份来源；27 的 “Use system Git config for other settings” 不得覆盖 user.name/email，避免冲突。（23、27）
- **R-SET-03 (MUST)** Git：executable path、Use PATH、Choose、Scan folders、Found/Add；Git 版本检查；缺失时提示安装 CLT。（24）
- **R-SET-04 (MUST)** Credentials：GitHub/GitLab 连接、credential helper、SSH keys（add/generate/copy/remove）、external editor/diff/merge/terminal、proxy、SSL、system config；secrets 只 Keychain。（27）
- **R-SET-05 (MUST)** Shortcuts：分组列表、当前快捷键、Record、冲突提示、Restore Defaults；`⌘P`、`⌘O`、`⌘⇧F`、`⌘⇧P`、`⌘⇧K`、`⌘↵` 等。（25）
- **R-SET-06 (MUST)** About：版本、build、Apple Silicon、Up to date、Links、Credits；不再放第二套更新控件。（26）
- **R-SET-07 (MUST)** 设置持久化；跨 repo 全局 vs repo-local 明确；修改后立即生效或标注重启。
- **R-SET-08 (MUST)** Launch restore 所有 repo；缺失 repo 提示；不能只恢复最后一个。

### 7.11 Navigation / native

- **R-NAV-01 (MUST)** Command palette：⌘P 搜索 repo + 执行命令（New Branch/New Tag/Squash/Open Settings…）；键盘上下/回车/Esc。（20）
- **R-NAV-02 (MUST)** Context menus 用 NSMenu；dark mode 与系统一致；危险项有确认。（21、22）
- **R-NAV-03 (MUST)** Focus ring、VoiceOver label、键盘可达；所有 icon-only 按钮有 aria-label/tooltip。（全局）
- **R-NAV-04 (MUST)** 语言切换：英文/简体中文；日期格式；错误/空状态/按钮全部本地化；不硬编码。（全局）
- **R-NAV-05 (MUST)** 多 repo tabs：切换保留 scroll/selection；关闭/恢复；不串 repo 状态。

---

## 8. 架构

### 8.1 总体

```
NSH Rust UI（AppKit 互操作）
  ├─ RepositoryController (per repo, actor)
  │    ├─ GitService (CLI process runner, parsers, cancellable)
  │    ├─ IndexService (patches, stage/unstage, refresh)
  │    ├─ OperationService (merge/rebase/cherry-pick/revert state)
  │    ├─ HookService (run/stream/log/timeout)
  │    ├─ AuthService (credential helper/askpass/SSH)
  │    ├─ SyncService (fetch/pull/push/progress)
  │    ├─ WatchService (FSEvents, debounce)
  │    └─ RefsService (branches/tags/remotes/stashes/worktrees)
  ├─ AppStore (SQLite/JSON: settings, session, changelists, audit)
  ├─ KeychainService (tokens/SSH passphrases; no SQLite)
  └─ AgentService (v0.2; permissions, planner, tools)
```

- **每个 repo 一个 actor/serial queue**：写操作串行；读操作可并发；切 repo 取消旧操作。
- **Git 是真相**：refs/index/status/conflict 状态从 Git 读；AppStore 只存应用元数据。
- **UI 不直接拼 Git 命令**；全部走 GitService/领域服务。
- **Global app**：tabs、settings、session；**RepoController**：当前 repo 的 index/history/operation。
- **Agent** 通过受限工具调用 RepoController，不直接拿 shell。

### 8.2 Git engine（D3）

- v0.1：系统 `git` CLI（`/usr/bin/git` 或 Settings 指定），最低 **Git 2.39**。
- 读：`git log`、`git show`、`git diff`、`git status --porcelain=v2`、`git blame --line-porcelain`、`git reflog`。
- 写：`git apply --cached`、`git add/restore`、`git commit -F -`、`git fetch/pull/push`、`git branch/tag/remote/stash`、`git merge/rebase/cherry-pick/revert`。
- 不自己实现 Git 对象/协议；不解析 packfile；不做 libgit2 写路径。
- 读性能不够时，v0.1.1 再评估 `gix`/libgit2 **只读**层；写路径仍走 CLI（hooks/credentials/SSH 复用）。

### 8.3 Process runner

- 使用 `Process`/`posix_spawn`，**不经过 shell**；参数是数组。
- 环境：
  - `GIT_TERMINAL_PROMPT=0`
  - `GIT_ASKPASS=<helper>`
  - `GIT_PAGER=cat` / `--no-pager`
  - `GIT_OPTIONAL_LOCKS=0`（只读命令）
  - `LC_ALL=C` 仅用于解析型命令；commit message 用 UTF-8 stdin
  - `core.quotepath=false`、`color.ui=false`
- stderr/stdout 分开流式读取；有上限（如 10 MB）；完整日志落临时文件。
- 取消：kill 进程组；不留下 `index.lock`；清理。
- 解析：porcelain v2 用 NUL 分隔；`git log` 用 `--format` + `-z`；不要解析人类可读输出做主数据。
- 错误：保留 exit code、stderr、命令（脱敏）、duration；分类到 §10。

### 8.4 Index / staging

- 读取：`git status --porcelain=v2 --branch -z` + `git diff --cached` / `git diff`。
- stage hunk/line：生成 unified patch，`git apply --cached --unidiff-zero`；失败要回滚并提示。
- unstage hunk/line：`git apply --cached --reverse`。
- file-level：`git add -- <path>` / `git restore --staged -- <path>`。
- discard：`git restore --source=HEAD --worktree -- <path>`（确认；不自动 stash）。
- 二进制/重命名/删除/新增：file-level；行级只对文本。
- 每次写后 `git update-index --refresh` 或重新读 status；hooks 可能修改 index，必须重读。
- **Changelist** 存 AppStore（SQLite）：id/name/active/color；文件 -> changelist；不写 Git config。
- Commit 只读取 index；changelist 仅过滤。

### 8.5 Operations / conflict

- 检测：`.git/MERGE_HEAD`、`rebase-merge/`、`rebase-apply/`、`CHERRY_PICK_HEAD`、`REVERT_HEAD`、`sequencer/`；结合 `git status`。
- 冲突：`git status --porcelain=v2` 的 unmerged entries；用 `git show :1:path`、`:2:`、`:3:` 读 base/ours/theirs。
- 保存 result 到工作区；`git add -- path`；`git merge --continue` / `git rebase --continue` / `git cherry-pick --continue`；abort 用对应 `--abort`；skip 仅 rebase。
- 三方编辑器：可编辑 result；Accept ours/theirs/both 是插入操作；Mark Resolved 只在 result 保存后可用。
- 操作状态机：idle → running → conflicted → continue/abort/skip；错误回到 conflicted；不能丢冲突。
- 冲突中的 commit 也走 hooks；失败重试不丢 result。

### 8.6 Hooks

- commit 用 `git commit -F -`；hooks 由 Git 执行；stdout/stderr 流式捕获。
- 记录：hook name、startedAt、duration、exitCode、output、status（passed/failed/skipped/cancelled）、commitId。
- 失败：不执行 commit；UI 显示日志；后续 hooks skipped；重新读 status。
- `--no-verify` 只在用户显式确认时；记录 audit；Agent 禁用。
- 超时 kill 进程组；不执行 commit。
- post-commit 失败：commit 已成功；显示 warning；不要回滚。
- 日志脱敏：URL userinfo、token、Authorization、SSH passphrase；完整日志本地临时文件。
- 第一次 commit 时提示 repo 有 hooks；可在设置里查看 hooks path（`core.hooksPath`）。

### 8.7 Auth / network

- HTTPS：系统 credential helper；`GIT_ASKPASS` helper 从 Keychain 取；不在 argv/log 中出现 token。
- SSH：系统 `ssh` + ssh-agent；known_hosts；host key 变更提示；不自动接受。
- Proxy/SSL：遵循 `.gitconfig`/环境/设置；证书错误可读。
- 认证失败分类：`auth_required`、`auth_failed`、`host_key`、`network`、`non_fast_forward`、`remote_rejected`。
- 网络操作可取消；关闭 sheet 不杀后台 push（UI 显示后台任务）；cancel 明确。
- `pre-push` hook 日志在 Sync activity；失败阻止 push。

### 8.8 File watching / refresh

- FSEvents watch worktree + `.git`（HEAD/index/refs/operation files）；debounce 150–250ms。
- 外部修改、branch checkout、hook 修改 index、rebase 进度 → 刷新 status/history/operation。
- 避免自触发循环：操作期间暂停/合并事件；操作结束后强制刷新。
- 大 repo：只 watch 必要路径；ignore `.git/objects` 内部；不 watch node_modules（Git 不跟踪）。

### 8.9 Session / persistence

- AppStore（SQLite）：
  - settings（JSON）
  - session：repo paths、tab order、active tab、scroll/selection（可选）
  - changelists + file membership
  - audit（destructive/agent/--no-verify）
  - hook run metadata（不含 secret）
- Keychain：HTTPS token、SSH passphrase（如需要）、OAuth refresh（v0.2）。
- 不缓存 Git refs/index 作为真相；缓存只读展示，事件后失效。
- 迁移：schema version；失败回退不破坏设置。

---

## 9. 数据模型（应用侧）

```ts
type Repository = {
  id: string;
  path: string;          // worktree
  gitDir: string;
  name: string;
  lastOpenedAt: Date;
  isBare?: boolean;
};

type Tab = { repoId: string; order: number };

type Change = {
  path: string;
  oldPath?: string;
  status: 'M'|'A'|'D'|'R'|'C'|'U'|'?';
  staged: boolean;
  changelistId?: string;
  conflict?: 'both'|'ours'|'theirs'|'added';
};

type Changelist = {
  id: string;
  name: string;
  active: boolean;
  color?: string;
};

type HookRun = {
  id: string;
  repoId: string;
  hook: 'pre-commit'|'prepare-commit-msg'|'commit-msg'|'post-commit'|'pre-push';
  status: 'running'|'passed'|'failed'|'skipped'|'cancelled';
  exitCode?: number;
  durationMs?: number;
  output: string;        // 脱敏后；完整日志另存
  commitId?: string;
  startedAt: Date;
};

type Operation = {
  type: 'merge'|'rebase'|'cherry-pick'|'revert';
  state: 'running'|'conflicted'|'continue'|'abort'|'skip';
  conflictCount: number;
  step?: { current: number; total: number };
};

type AgentRun = {
  id: string;
  repoId: string;
  plan: { files: string[]; message: string; push: boolean };
  permissions: AgentPermissions;
  state: 'planning'|'awaiting-approval'|'running'|'done'|'error'|'cancelled';
  log: string;
};

type AgentPermissions = {
  read: boolean;
  stage: boolean;
  commit: boolean;
  push: boolean;
  forcePush: boolean;
  discard: boolean;
  automation: 'off'|'commit'|'commit-and-push';
  protectedBranches: string[];
};
```

- `Change` 是 UI 派生；Git index 是真相。
- `Changelist` 不进入 Git；删除只删分组。
- `HookRun` 的完整 output 存临时文件/日志；SQLite 存摘要。
- `AgentRun` 的 log 脱敏；不存 token。
- `Operation` 与 Git 文件系统状态双向校验；不要只信内存。

---

## 10. 状态与错误

### 10.1 必要状态

- Repo：loading、ready、empty repo（无 commit）、detached HEAD、operation in progress、offline、auth required、error。
- History：loading、empty、filtered-empty、load-more、error。
- Diff：loading、binary、too-large、error。
- Commit：idle、staging、hooks-running、hook-failed、committing、committed、push-after、error。
- Sync：idle、fetching、pulling、pushing、auth-required、non-fast-forward、error、background。
- Merge：running、conflicted、resolved、continue、abort、skip。
- Agent：planning、awaiting-approval、running、error、cancelled、done。
- Hooks：passed、failed、skipped、cancelled、timeout。

### 10.2 错误分类

| Code | 含义 | UI 行动 |
|---|---|---|
| `E_GIT_MISSING` | 找不到 Git/版本低 | 设置 Git path / 安装 CLT |
| `E_NOT_A_REPO` | 路径不是 repo | Init / Open parent / 取消 |
| `E_REPO_MISSING` | recent 路径丢失 | Locate / Remove |
| `E_INDEX_LOCK` | index.lock 存在 | 显示进程/重试；不要自动删 |
| `E_AUTH_REQUIRED` | 需要登录 | Sign in / Use SSH |
| `E_AUTH_FAILED` | 凭据失败 | 重试/更新凭据 |
| `E_HOST_KEY` | SSH host key 变更 | 显示指纹；不自动接受 |
| `E_NETWORK` | 离线/超时 | Retry / 离线模式 |
| `E_NON_FAST_FORWARD` | push 被拒 | Pull/Rebase/Force-with-lease |
| `E_CONFLICT` | 合并冲突 | 打开冲突编辑器 |
| `E_HOOK_FAILED` | hook 失败 | 日志 / Retry / --no-verify |
| `E_HOOK_TIMEOUT` | hook 超时 | 日志 / Retry / 取消 |
| `E_OPERATION_IN_PROGRESS` | 有 merge/rebase | 先 continue/abort |
| `E_PERMISSION` | 文件/Keychain 权限 | 打开系统设置 |
| `E_DISK` | 磁盘满/写失败 | 显示路径/空间 |
| `E_INVALID_PATH` | 路径非法 | 修正输入 |

- 错误必须包含：用户可读摘要、下一步、Details（脱敏后的 stderr/命令/exit code）。
- 不允许只显示 “something went wrong”。

---

## 11. 设计系统 / 原生 UI

### 11.1 规则

- 生产 UI 使用 AppKit：`NSWindow`、`NSToolbar`、`NSSplitView`、`NSOutlineView`、`NSTableView`、`NSTextView`、`NSButton`、`NSSegmentedControl`、`NSSearchField`、`NSPopUpButton`、`NSMenu`、`NSVisualEffectView`、`NSProgressIndicator`、`NSAlert`/sheet。
- 字体：SF Pro Text / SF Mono；正文 13，次要 11，元数据 10–10.5；代码 11–12；不要 8–9px 的 app 标签。
- 颜色：accent `#007AFF`（dark `#0A84FF`）；primary 统一 accent；不出现两套蓝。
- 布局：28–30px 行高；按钮 22/24；segmented 23–24；菜单项 24；圆角 6–8；hairline。
- Sidebar 使用 source list 语义 + vibrancy；选中用 accent fill；settings 侧栏同理。
- 菜单/对话框：NSMenu；sheet 贴窗口顶部，top-only 圆角；不要自由浮动大阴影卡片（除非明确是 dialog）。
- Keyboard：⌘P/⌘O/⌘F/⌘⇧F/⌘⇧P/⌘⇧K/⌘↵；所有行可键盘操作；focus ring。
- Dark mode：所有 app 屏；gallery 组件板可保留 light。
- VoiceOver：所有 icon-only 按钮有 label；状态/错误可读；不只用颜色。
- 对比度：正文/标签至少 4.5:1；11px 灰字也要过。
- Reduced motion：spinner/progress 可关。

### 11.2 画布与生产的关系

- 画布是 **视觉 + 状态规格**；`30` 页已经足够。不要继续加页面。
- 画布里的 HTML/CSS 不是生产实现；生产用 **NSH Rust API + AppKit**，不使用 TSX/JS。
- 28/29 是真实 Sync sheet；30 是 Refs 页面；22 是组件规格；16 是状态 gallery。

---

## 12. 性能预算

| 场景 | 预算 |
|---|---|
| 冷启动到可交互 | < 1.5s（已缓存 repo 列表） |
| 打开 repo（status + 首屏 history） | < 800ms（普通 repo）；大 repo < 2s |
| Status 刷新（FSEvents） | < 200ms，debounce 150–250ms |
| 首屏 history（100 commits） | < 500ms；后续分页 < 300ms |
| 打开 10k 行 diff | < 600ms；滚动 60fps |
| 100k commits 的 repo | 不一次加载；分页/虚拟化；内存 < 500MB |
| 1GB repo | 可打开/status；不阻塞主线程 |
| Stage hunk/line | < 300ms；失败回滚 |
| Commit + hooks | 受 hook 影响；UI 不冻结；可取消 |
| Fetch/pull/push | 进度可见；可取消；不阻塞其他 repo |
| 多 repo 切换 | < 150ms（保留状态） |

实现要求：CLI 并发上限 2–4；读/写队列分离；diff 虚拟化；syntax highlight 按需；缓存按 repo/文件失效；取消传播。

---

## 13. 测试策略

### 13.1 自动化

- **Parsers**：porcelain v2、log format、diff、reflog、blame 的 golden tests。
- **Index**：hunk/line stage/unstage/discard；partial commit；新增/删除/重命名/二进制。
- **Hooks**：pass/fail/timeout/post-commit fail/--no-verify。
- **Conflict**：merge/rebase/cherry-pick/revert；continue/abort/skip；result 编辑。
- **Sync**：fetch/pull/push、non-fast-forward、auth mock、cancel。
- **Session**：多 repo 恢复、缺失 repo、退出/崩溃。
- **Agent**：权限、审批、日志脱敏、no auto push（v0.2）。
- **Security**：命令注入、path traversal、token 泄漏、host key、hook 提示。
- **Accessibility**：VoiceOver labels、键盘路径、对比度。

### 13.2 Fixtures

- 空 repo、普通 repo、100k commits、10k 行 diff、1GB、merge conflict、rebase conflict、submodule、worktree、hooks（fail/pass/timeout）、binary、rename、detached、shallow、bare。
- 所有 fixture 本地生成；不依赖外网（auth 用本地 HTTP/test token）。
- 测试不写入用户真实 repo；临时目录。

### 13.3 手工验收

- 5 名开发者 pilot，2 周；任务见 Appendix A/验收。
- Dogfood gitui 自己；每天真实 commit/push；记录失败。
- 与 Fork/IDEA 对照：同一 repo、同一操作，确认结果一致。

---

## 14. Milestones（设计与实现顺序）

| Milestone | 内容 | 退出标准 | 估算 |
|---|---|---|---|
| **M0 Validation** | Appendix A spike | 6 项全部通过/有明确 fallback | 1 周 |
| **M1 Shell + read** | 01–06/18；open/clone/init；history/diff；settings 基础 | 可打开真实 repo、浏览、diff；大 repo 可接受 | 2 周 |
| **M2 Write** | 10/11/14；index/stage/commit/hooks/branch/conflict | 行级 stage + commit + hook fail/retry；冲突可解 | 3 周 |
| **M3 Sync** | 15/28/29；fetch/pull/push/auth/progress/errors | 真实 remote 成功/失败；不泄 token | 2 周 |
| **M4 Merge/rebase** | 14；三方编辑、continue/abort/skip；merge/rebase/cherry-pick/revert | 冲突编辑正确；hooks 在 merge commit 生效 | 2 周 |
| **M5 Recovery/trace** | 07/08/09/17/19/30 | reflog/undo、blame/history、compare、refs | 2 周 |
| **M6 Agent** | 12/13 | Ask-first、权限、日志、无 auto push | 3–4 周 |
| **M7 Beta** | 设置、签名、notarize、Sparkle、pilot | 5 人 2 周；无 P0 数据丢失 | 2 周 |

- **v0.1** = M1–M4 + 23–27 + 28/29 + packaging；M5 作为 v0.1.1；M6 作为 v0.2。
- 每个 milestone 独立可用；不要等 Agent 才发 v0.1。
- 20 交互式 rebase、P2 全部在 v0.3/later。

---

## 15. 风险与开放决策

### 15.1 最脆弱假设

> 这套计划假设 **NSH 的 Rust API 能承载真实代码/三方合并编辑器**，并且 **系统 Git CLI 能满足写路径/hooks/conflict/性能**。  
> 如果只能包 `git log/diff/commit`，staging/conflict/rebase 页面就是 fiction；如果 NSH Rust 不能嵌编辑器，冲突页要换 AppKit `NSTextView` 或原生组件。

**验证方式**：Appendix A 的 S-3/S-6；不通过就不要进入 M2。

### 15.2 风险表

| 风险 | 影响 | 缓解 |
|---|---|---|
| CLI 解析/进程开销 | 大 repo 卡顿 | 只读分页；后台队列；v0.1.1 评估 gix read-only |
| hooks 任意代码/超时 | 安全/卡死 | 用户发起；日志；超时 kill；首次提示；不自动 --no-verify |
| 冲突编辑复杂 | 数据丢失 | 三方 stages 读取；result 保存；abort；fixtures |
| Auth/SSH/Keychain | 无法 push | 系统 Git/credential helper/askpass；spike |
| NSH Rust 编辑器能力 | 冲突/blame 不可用 | S-6；fallback AppKit `NSTextView` / 原生组件 |
| MAS 沙箱 | Git/hooks/SSH 受限 | 直接 notarize；不做 MAS v0.1 |
| 大 repo 内存 | OOM | 分页/虚拟化/缓存上限 |
| Agent prompt injection | 误操作/泄密 | 权限/审批/不读 secrets/审计 |
| 多 repo 状态 | tab 串状态 | per-repo actor；恢复测试 |
| 名称/域名/更新 | 发布阻塞 | M7 前定名/域名；Sparkle feed |

### 15.3 开放决策（有默认）

- **Git 引擎**：已选 CLI-first；若 S-4 失败，fallback `gix` 只读 + CLI 写。
- **更新机制**：Proposed Sparkle 2；M7 定 feed URL/签名。
- **Agent provider**：v0.2；Proposed provider-agnostic interface；模型/数据政策在 M6 前定。
- **App 名**：gitui 工作名；M7 前定最终名、bundle ID、域名。
- **Intel**：v0.1 不做；v0.2 评估。
- **Interactive rebase**：v0.3；当前无画板；届时补设计。
- **Submodule/worktree 写操作**：v0.1 只读列表；v0.1.1 写。

---

## 16. 非目标 / 不做的

- 继续加画板；30 页已足够。
- LFS、GPG 签名、PR/notifications、image diff、Git-flow、code search、shelf/local history、patch apply。
- 跨平台（Windows/Linux）、插件系统、多窗口、主题市场。
- 自研 Git 协议/对象存储。
- MAS 版本（v0.1）。
- Agent 自动 push、Agent 修改历史、Agent 读 secrets、Agent `--no-verify`。
- 把 diff/commit graph 换成“大行高”的 native 表格；密度是产品特性。

---

## 17. 验收与 Definition of Done（v0.1）

- [ ] 打开/clone/init 真实 repo；恢复上次所有 repo tab；缺失 repo 有提示。
- [ ] 100k commit repo 首屏可用；10k diff 可滚动；1GB repo 可 status。
- [ ] 行级 stage/unstage 正确；partial commit 只提交选中的 staged；discard 有确认。
- [ ] commit 显示 hooks；失败阻止；Retry；`--no-verify` 确认；post-commit 失败有 warning。
- [ ] fetch/pull/push 真实 remote 成功；auth/网络/non-fast-forward 有可操作错误；token 不泄漏。
- [ ] branch/tag/remote/stash 基本写操作；refs 刷新。
- [ ] merge/rebase/cherry-pick/revert 冲突可三方编辑；continue/abort/skip；不丢 result。
- [ ] 设置身份/凭据/SSH/工具；语言/主题/日期/启动恢复；快捷键。
- [ ] 键盘/VoiceOver/对比度/dark mode 过基本检查。
- [ ] 签名/notarize/更新；5 人 2 周 pilot；无 P0 数据丢失。

---

## Appendix A — Spike Checklist（M0）

**原则**：spike 不是产品 UI；用 CLI harness 或最小 **NSH Rust + AppKit** 测试窗口；每个 spike 产出命令、耗时、失败、结论。  
**时间盒**：每项 0.5–1 天；总 1 周。

### S-1 Partial staging / line staging

**目标**：证明能只 stage 一个 hunk/line，commit 只包含它，index 不坏。

**Setup**
```sh
rm -rf /tmp/spike-stage && mkdir /tmp/spike-stage && cd /tmp/spike-stage
git init -q
printf 'one\ntwo\nthree\nfour\n' > a.txt
git add a.txt && git commit -qm base
printf 'one\nTWO\nthree\nFOUR\n' > a.txt
```

**Steps**
1. `git diff -U0 -- a.txt`，提取一个 hunk（或手工构造 patch）。
2. `git apply --cached --unidiff-zero <hunk.patch>`。
3. `git diff --cached` 确认只 stage 目标；`git diff` 确认其余仍在工作区。
4. `git commit -m partial`；`git show --stat --oneline HEAD`；`git status`。
5. 反向：`git apply --cached --reverse <hunk.patch>` 验证 unstage。
6. 新增/删除/重命名/二进制各一次。

**Pass**
- 只提交目标 hunk/line；其余修改仍在；无 index.lock；重复操作稳定。
- 失败 patch 不改变 index；错误可读。

**Record**：stage 命令、耗时、失败 patch、结论。

### S-2 Hooks lifecycle

**目标**：证明 commit 能捕获 hooks 的 stdout/stderr/exit，失败阻止 commit，重试/--no-verify 可用，post-commit 失败不回滚。

**Setup**
```sh
rm -rf /tmp/spike-hooks && mkdir /tmp/spike-hooks && cd /tmp/spike-hooks
git init -q
printf 'x\n' > a.txt && git add a.txt && git commit -qm base
mkdir -p .git/hooks
cat > .git/hooks/pre-commit <<'SH'
#!/bin/sh
echo "linting..."
echo "src/a.ts:1: error: bad" >&2
exit 1
SH
chmod +x .git/hooks/pre-commit
```

**Steps**
1. 修改文件并 `git add`；执行 `git commit -F - <<< "test"`；记录 exit/stderr/stdout。
2. 确认 commit 未产生（`git log -1` 仍是 base）。
3. 配置通过 hook（exit 0）；commit 成功；记录 duration。
4. 配置 `commit-msg` 失败；确认 commit 被阻止。
5. 配置 `post-commit` 失败；确认 commit 成功但 warning。
6. `git commit --no-verify`；确认绕过；记录 audit。
7. 配置 sleep 10 的 hook；验证 app 可 kill 进程组；不留下 commit。

**Pass**
- 失败/成功/超时/--no-verify 四态正确；日志完整；post-commit 不回滚。
- 无 orphan 进程；无 index.lock；重试稳定。

### S-3 Merge conflict / three-way editable

**目标**：证明能读 base/ours/theirs，编辑 result，mark resolved，continue/abort，且不丢数据。

**Setup**
```sh
rm -rf /tmp/spike-conflict && mkdir /tmp/spike-conflict && cd /tmp/spike-conflict
git init -q -b main
printf 'a\nb\nc\n' > f.txt; git add f.txt; git commit -qm base
git checkout -qb feature
printf 'a\nB-feature\nc\n' > f.txt; git commit -qam feature
git checkout -q main
printf 'a\nB-main\nc\n' > f.txt; git commit -qam main
git merge feature || true
```

**Steps**
1. `git status --porcelain=v2` 读 unmerged；`git show :1:f.txt`、`:2:`、`:3:`。
2. 在 result 编辑（如 `B-merged`）；保存；`git add f.txt`。
3. `git merge --continue`；确认 commit/parents。
4. 重做冲突，`git merge --abort`；确认工作区恢复。
5. rebase/cherry-pick 冲突各一次；`--continue`/`--abort`/`--skip`。

**Pass**
- 三方内容正确；result 编辑后 commit 内容正确；abort 恢复；无残留 MERGE_HEAD/rebase 目录。
- 冲突中 hooks 失败可重试；不丢 result。

### S-4 Performance / large repo

**目标**：证明 100k commits、10k 行 diff、1GB repo 可用。

**Setup**
- 用 `git fast-import` 生成 100k commits（脚本化；小文件）。
- 生成 10k 行 diff（大文件单次修改）。
- 1GB repo（大文件/LFS-like；不提交二进制到 app repo）。
- 测：
  - `git log --max-count=100 --format=...` + parse + 首屏渲染。
  - `git diff` 10k 行 + virtualized render。
  - `git status` 在 1GB/大文件 repo。
  - 内存/CPU/主线程阻塞。

**Pass**
- 首屏 history < 500ms；10k diff < 600ms；status < 500ms；滚动 60fps；内存 < 500MB。
- 不一次加载全部 commit；取消有效。

**Fallback**：读层换 `gix`/libgit2；分页更激进；diff 按块加载。

### S-5 Auth / network failure

**目标**：证明 HTTPS/SSH、credential helper、Keychain、错误分类、cancel。

**Setup**
- 一个 disposable private repo + test token（只存 Keychain/临时环境；不进 repo）。
- 或本地 HTTP Git server + basic auth。
- SSH：临时 key + ssh-agent；known_hosts 测试。

**Steps**
1. HTTPS clone/fetch/push 成功；token 不出现在日志/argv。
2. 错误 token → `auth_failed`；无无限重试。
3. SSH 成功；host key 变更 → 明确提示；不自动接受。
4. 离线/超时 → `network`；Retry。
5. non-fast-forward → 提示 Pull/Rebase/Force-with-lease；不自动 force。
6. Cancel push → 进程结束；无残留。

**Pass**
- 四类认证/网络错误可区分；secrets 不泄漏；cancel 干净。

### S-6 NSH editor / merge view feasibility

**目标**：证明 **NSH Rust API + AppKit** 能承载可用的代码/三方编辑器和 10k 行渲染；不使用 TSX/WebView。

**Steps**
1. 最小 NSH Rust 窗口：三列 AppKit `NSTextView`（或 NSH 包装）编辑器；加载 10k 行；滚动/输入/undo。
2. Result 可编辑；保存；与 S-3 的 result 文件互通。
3. 语法高亮按需；大文件不卡；内存可控。
4. 键盘/VoiceOver 基本可用。

**Pass**
- 10k 行可滚动/编辑；输入延迟可接受；undo/redo；三列同步；不崩。
- 如果失败：fallback 到 webview 或原生单列编辑器；重画 14。

### S-7（可选）CLI vs gix 读路径

- 在 S-4 的 100k repo 上对比 `git log` 解析 vs `gix` 读 commit graph 的延迟/内存。
- 只读比较；写仍 CLI。
- 结论写入 spike report。

### Spike report 模板

```md
# Spike Report
Date:
Repo/fixtures:
- S-1: pass/fail, timings, notes
- S-2: pass/fail, logs, notes
- S-3: pass/fail, conflict cases
- S-4: pass/fail, p50/p95, memory
- S-5: pass/fail, auth cases, secret audit
- S-6: pass/fail, editor latency
- Decision: CLI-first / gix read fallback / editor fallback
- Remaining risks:
```

**Go/No-Go**：S-1/S-2/S-3/S-4/S-5/S-6 全部 pass，或每个 fail 有明确 fallback；否则不进入 M2。

---

## Appendix B — 命令矩阵（v0.1）

| 能力 | 命令（示例） | 备注 |
|---|---|---|
| status | `git status --porcelain=v2 --branch -z` | 主数据；NUL 解析 |
| history | `git log --max-count=... --format=... -z` | 分页 |
| commit | `git show --stat --format=...` | 详情 |
| diff | `git diff --no-ext-diff --unified=... -- <path>` | 文件级 |
| staged diff | `git diff --cached -- ...` | commit 前 |
| stage hunk | `git apply --cached --unidiff-zero <patch>` | 生成 patch |
| unstage | `git apply --cached --reverse <patch>` | |
| commit | `git commit -F -` | hooks；message stdin |
| amend | `git commit --amend -F -` | hooks |
| fetch | `git fetch --progress --prune <remote>` | stderr progress |
| pull | `git pull --rebase/--no-rebase --autostash` | 冲突 |
| push | `git push --porcelain --progress` | 失败分类 |
| branch | `git branch`, `git switch`, `git branch -d` | 确认 |
| tag | `git tag -a/-l/-d`, `git push <remote> <tag>` | |
| remote | `git remote add/set-url/remove` | |
| stash | `git stash push/apply/pop/drop` | diff |
| merge | `git merge --no-ff/--ff-only` | 冲突 |
| rebase | `git rebase` | 冲突；interactive 不在 v0.1 |
| cherry-pick | `git cherry-pick` | 冲突 |
| revert | `git revert` | 冲突 |
| reflog | `git reflog --format=...` | v0.1.1 |
| blame | `git blame --line-porcelain` | v0.1.1 |
| config | `git config --get` / `git config --local` | 设置 |
| credential | `git credential fill/approve/reject` | 认证 |

- 全部 `--no-pager`；禁止 shell；路径用 `--`；输出上限；错误脱敏。

---

## Appendix C — Changelog

- 2026-11-14：初版。锁定 D1–D10；定义 v0.1/v0.1.1/v0.2；30 页 board map；hooks 日志；spike checklist。
