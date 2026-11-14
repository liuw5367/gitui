# ARCHITECTURE — NSH + Rust

**Status**: v1, 2026-11-14  
**Stack**: **NSH (Native Shell) + Rust only.** No TSX, no JS runtime, no HTML UI, no WebView for core screens.  
**Spec**: [../SPEC.md](../SPEC.md)  
**UI contract**: [UI-SPEC.md](UI-SPEC.md)  
**Tests**: [TEST-PLAN.md](TEST-PLAN.md)  
**ADRs**: [adr/](adr/README.md)

---

## 1. Goals and constraints

### 1.1 Goals

- One Rust codebase for UI, state, Git services, persistence, and Agent.
- Native macOS behaviour via NSH's Rust API and AppKit interop.
- Correct index-first Git semantics; hooks/auth/conflicts are first-class.
- Responsive with 100k commits and 10k-line diffs.
- Crash-safe: no index corruption, no lost conflict resolution, no secret leakage.

### 1.2 Constraints

- **No TypeScript/JS bridge.** NSH is used from Rust directly.
- **System Git CLI** for write path and hooks; no reimplementation of Git protocol.
- macOS 14+; Apple Silicon primary; Intel not v0.1.
- Direct signed/notarized distribution; no Mac App Store v0.1.
- Exact NSH Rust API names must be pinned to the NSH version used by the app; this document uses role names, not invented method names.

### 1.3 Non-goals

- Cross-platform UI, plugin runtime, web embedding for core screens.
- LFS/GPG/PR/notifications/Git-flow/code search in v0.1.
- Agent autonomy beyond explicit permissions/approval.

---

## 2. System context

```
┌─────────────────────────────────────────────────────────────┐
│ NSH Rust App (single process)                               │
│                                                             │
│  NSH Rust UI ── view models/events ── AppState             │
│       │                                   │                 │
│       └──────── commands ────────────────┘                 │
│                                           │                 │
│  RepositoryController (per repo, actor)   │                 │
│    ├─ GitService (CLI runner + parsers)   │                 │
│    ├─ IndexService                        │                 │
│    ├─ OperationService                    │                 │
│    ├─ HookService                         │                 │
│    ├─ SyncService                         │                 │
│    ├─ AuthService                         │                 │
│    ├─ RefsService                         │                 │
│    ├─ WatchService                        │                 │
│    └─ Store (SQLite, app metadata)         │                 │
│                                           │                 │
│  KeychainService (Security.framework)     │                 │
│  AgentService (v0.2; tool-gated)          │                 │
└─────────────────────────────────────────────────────────────┘
              │
              ├── system `git` CLI (process group, streaming)
              ├── ssh / credential helper
              └── filesystem (repo, .git, FSEvents)
```

- **Single process**: no UI↔core IPC. NSH hosts the event loop and windows; Rust services run on worker threads/runtime.
- **No JS/TS**: all views are NSH Rust components; all commands/state are Rust types.
- **Git is truth**: refs/index/operation state are read from Git; SQLite stores only app metadata.
- **UI never spawns Git or touches secrets**; it sends typed commands to controllers.

---

## 3. Crate / module layout

```
apps/client/                NSH 应用入口、窗口、标题栏、屏幕与工作区视图、locales（同一 crate 供各平台构建）
crates/core/                领域类型、错误分类、补丁解析（无 UI/进程依赖）
crates/git/                 进程执行与输出解析：status/log/show/diff/refs；后续写路径、同步、操作状态机
crates/store/               会话、最近仓库、界面语言（本地 JSON/文本文件）
```


Rules:

- 应用 crate 拥有每个标签页的界面状态；`core` 不依赖 UI、AppKit 或进程。
- `git` 是唯一构造 Git 命令的 crate；写路径、同步与冲突状态机在后续阶段加入同一 crate。
- 凭据只经系统 Git 的 credential helper 与 Keychain，应用不自行读取；`store` 不保存秘密。
- Agent（v0.2）接入时只能调用应用实现的能力接口，不直接获得 `std::process` 或 Keychain。
- 13 个 crate 的早期拆分对 v0.1 属于过度设计，按实测需要再拆。

---

## 4. Runtime and threading

### 4.1 Threads

| Thread | Owns | Rules |
|---|---|---|
| Main thread | AppKit/NSH window, view updates, menus | never block on Git/network/disk |
| Tokio runtime | async I/O, process streams, timers | bounded workers |
| Per-repo actor | serialized write operations/state | one writer per repo |
| Git worker pool | process wait/parse | 2–4 concurrent reads; writes serialized |
| Watcher | FSEvents callback | debounce, send invalidation only |

- AppKit calls happen on the main thread only; view models are `Send` snapshots.
- Long operations (fetch/push/commit/hook) run off-main; UI receives progress/events.
- Cancellation is a `CancellationToken` per operation; process runner kills the process group.
- Switching tabs never cancels another repo’s operation unless user closes that repo/tab with confirmation.

### 4.2 State flow

```
User action (NSH view)
  → Command (typed)
  → AppState / RepositoryController
  → Service (async)
  → Event / ViewModel snapshot
  → NSH view update (main thread)
```

- Commands are idempotent where possible; destructive commands carry an explicit confirmation token/nonce.
- Events are ordered per repo; UI ignores stale generations (repo id + generation).
- No global mutable singleton; `AppState` is owned by the app and passed explicitly.

### 4.3 Refresh

- After any write: refresh status + refs + operation state; invalidate history if HEAD/refs changed.
- Watch events debounce 150–250 ms; a write operation suppresses self-events and forces a refresh at the end.
- Hooks can modify the index; always re-read status after a hook run.

---

## 5. NSH Rust UI layer

### 5.1 View construction

- Build views with NSH's Rust view/component API; use AppKit-backed primitives for tables, outlines, text, split views, toolbars, menus, sheets.
- Use declarative state → view; avoid manual `NSView` mutation except for interop holes (e.g. `NSTextView` configuration).
- Pin the NSH version; record exact API names in `docs/adr/0002-nsh-rust-single-process.md` once S-6 is run.

### 5.2 Native components

| Need | Preferred |
|---|---|
| Sidebar | `NSOutlineView` + `NSVisualEffectView` |
| Commit/file table | `NSTableView` (view-based, reusable rows) |
| Diff/editor | `NSTextView` in `NSScrollView` (read-only/edit modes) |
| Split layout | `NSSplitView` |
| Toolbar | `NSToolbar`/NSH toolbar |
| Menus | `NSMenu` |
| Sheets/alerts | NSH sheet / `NSAlert` |
| Search | `NSSearchField` |
| Segmented | `NSSegmentedControl` |
| Progress | `NSProgressIndicator` |
| Settings | AppKit controls/NSH wrappers |

### 5.3 Large data

- `NSTableView` reuse + row height fixed; no view per commit beyond visible rows.
- Diff rendering chunked by hunk/paragraph; syntax highlight off for >N lines, on demand.
- Avoid re-rendering the whole diff on selection; update only changed regions.
- Images/binary are placeholders, not decoded inline.

### 5.4 Accessibility / localization

- Every NSH view exposes accessibility label/role/value.
- Strings via localization keys; no concatenation.
- Dynamic type is not a macOS concept, but support system text size/contrast; avoid fixed heights that clip.

---

## 6. Git service

### 6.1 Process runner

- Use Rust process APIs (`std::process` or `tokio::process`); **no shell**, args array only.
- Set:
  - `GIT_TERMINAL_PROMPT=0`
  - `GIT_ASKPASS=<auth helper>`
  - `GIT_PAGER=cat`, `--no-pager`
  - `GIT_OPTIONAL_LOCKS=0` for read-only
  - `LC_ALL=C` for parsed commands; UTF-8 stdin for commit messages
  - `core.quotepath=false`, `color.ui=false`
- Capture stdout/stderr separately; stream lines; cap memory (10 MB); full log to temp file.
- Timeout per operation; kill process group; reap; remove stale `.git/index.lock` only if we created it and process is dead (never blindly).
- Never log argv containing credentials; redact URLs/tokens.

### 6.2 Read path

| Data | Command |
|---|---|
| status | `git status --porcelain=v2 --branch -z` |
| log page | `git log --max-count=N --skip=M --format=... -z` |
| commit | `git show -s --format=... <sha>` |
| diff | `git diff --no-ext-diff --unified=... <base> -- <path>` |
| staged diff | `git diff --cached ...` |
| file at commit | `git show <sha>:<path>` |
| blame | `git blame --line-porcelain` (v0.1.1) |
| reflog | `git reflog --format=...` (v0.1.1) |

- Parse NUL-delimited porcelain v2; do not parse human output for primary data.
- Treat paths as bytes/UTF-8 with lossy display; preserve original for Git commands.
- Pagination: 100–200 commits/page; cancel on filter/tab change.

### 6.3 Write path

| Action | Command |
|---|---|
| stage hunk/line | `git apply --cached --unidiff-zero <patch>` |
| unstage hunk/line | `git apply --cached --reverse <patch>` |
| stage file | `git add -- <path>` |
| unstage file | `git restore --staged -- <path>` |
| discard | `git restore --source=HEAD --worktree -- <path>` (confirm) |
| commit | `git commit -F -` |
| amend | `git commit --amend -F -` |
| branch | `git switch`, `git branch` |
| fetch/pull/push | §8 |
| merge/rebase | §9 |

- Generate patches from a fresh diff; apply; if fail, refresh and retry once, then report.
- After write, re-read status/refs; never trust in-memory index.

---

## 7. Index / changelists

- **Index is truth**: `staged` means in `git diff --cached`.
- Changelists are `gitui-store` metadata: id/name/active/color; path → changelist; no Git objects.
- Moving a file between changelists never changes the index.
- Commit scope = index, regardless of active changelist.
- File-level operations for binary/rename/delete; line-level only for text.
- Discard requires confirmation; never auto-stash silently.
- On external index change: refresh; if a patch no longer applies, abort safely.

---

## 8. Sync / auth

### 8.1 Sync

- Fetch: `git fetch --progress --prune [--tags] <remote>`; stream progress from stderr.
- Pull: `git pull --rebase|--no-rebase [--ff-only] [--autostash]`; conflict → Operations.
- Push: `git push --porcelain --progress [--force-with-lease]`; never implicit force.
- Progress parser tolerates Git locale; prefer `--porcelain` for push result.
- Background operation state per repo; closing sheet does not cancel; explicit Cancel does.

### 8.2 Auth

- HTTPS: `git credential fill/approve/reject` + Keychain; `GIT_ASKPASS` helper reads a one-shot secret; never argv.
- SSH: system `ssh` + ssh-agent; `known_hosts`; host-key change requires explicit confirmation; no auto-accept.
- Classify failures: `auth_required`, `auth_failed`, `host_key`, `network`, `non_fast_forward`, `remote_rejected`.
- Redact URLs with userinfo, tokens, Authorization headers, SSH passphrases.
- `pre-push` hook output goes to Sync activity; failure blocks push.

---

## 9. Operations / conflict

### 9.1 Detection

Check, in order: `.git/MERGE_HEAD`, `.git/rebase-merge/`, `.git/rebase-apply/`, `.git/CHERRY_PICK_HEAD`, `.git/REVERT_HEAD`, `.git/sequencer/`; confirm with `git status`.

### 9.2 Model

```
Idle → Running → Conflicted → (Continue → Running)* → Done
                         └→ Abort → Idle
                         └→ Skip (rebase) → Running
```

- Running/Conflicted is derived from disk on startup, not only memory.
- Conflict files from `git status --porcelain=v2` unmerged entries.
- Base/Ours/Theirs from `git show :1:path`, `:2:`, `:3:`; never parse only markers.
- Save result → `git add -- path`; Continue only if unmerged count is 0.
- Continue/abort/skip commands match operation type.
- Merge/rebase commit runs hooks; hook failure leaves operation conflicted, result preserved.

### 9.3 Data safety

- Never auto-resolve.
- Never discard Result on hook failure.
- Abort requires confirmation; show target branch and step.
- External file changes while editing → warn/merge; do not silently overwrite.

---

## 10. Hooks

- Hook names: `pre-commit`, `prepare-commit-msg`, `commit-msg`, `post-commit`, `pre-push`.
- Git executes hooks; Rust streams stdout/stderr and records exit/duration.
- Failure semantics:
  - pre-commit/commit-msg: commit not created; index intact.
  - post-commit: commit created; warning only (S-2 confirmed).
  - pre-push: push blocked.
- Timeout default 5 min; kill process group; mark cancelled/timeout; no commit.
- Output cap 1 MB live; full log temp; redact secrets.
- `--no-verify` only explicit user confirmation; audit; Agent denied.
- First commit in repo with hooks: one-time notice; do not auto-bypass.
- After hook: re-read status; hook may modify index.

---

## 11. Persistence / security

### 11.1 Store

SQLite (`rusqlite`):

- settings (JSON or typed table)
- session: repo paths, tab order, active tab, scroll/selection (optional)
- changelists + membership
- audit: destructive, `--no-verify`, Agent actions
- hook run metadata (no secrets)

Migrations versioned; failure rolls back; no silent data loss.

### 11.2 Keychain

- HTTPS tokens, SSH passphrases, OAuth refresh (v0.2) in Keychain.
- Never in SQLite, logs, crash reports, Agent context.
- Access via `security-framework`; errors surface with guidance, no plaintext fallback.

### 11.3 Security

- No shell interpolation; all args arrays.
- Paths canonicalized; `--` before paths; reject NUL.
- Symlink/repo boundary checks for file operations.
- Hook output and Git stderr redacted before UI/log.
- Agent tool API is allow-listed; no raw process/Keychain.
- Audit log is append-only; UI can view.

---

## 12. Watchers / invalidation

- FSEvents on worktree + `.git` (HEAD, index, refs, operation dirs).
- Debounce 150–250 ms; coalesce; ignore `.git/objects` churn and our own writes.
- On event: status refresh; if HEAD/refs changed, history/branch refresh; if operation dir changed, operation state refresh.
- On write completion: force refresh; do not rely on event.
- Large repos: watch only relevant roots; skip ignored/untracked-heavy dirs; cap event rate.

---

## 13. Agent (v0.2)

- `AgentService` plans with a provider; emits a `Plan` (files, message, push) for approval.
- Tool API: `read_status`, `read_diff`, `stage_paths`, `commit`, `push` (if permission), `open_file`? No secrets.
- Permissions per repo: read/stage/commit/push/force/discard; automation; protected branches.
- Agent never calls `--no-verify`, never reads Keychain, never pushes protected branch, never force without explicit user approval.
- All tool calls/results audited; redact.
- Details: [AGENT-THREAT-MODEL.md](AGENT-THREAT-MODEL.md).

---

## 14. Sequence diagrams

### 14.1 Stage hunk

```
UI → App: StageHunk(repo, path, hunk)
App → RepoActor: stage_hunk
RepoActor → Git: diff (fresh)
RepoActor → Index: build patch
Index → Git: apply --cached
Git → Index: ok/error
Index → RepoActor: status refresh
RepoActor → App: ChangesUpdated
App → UI: render staged/unstaged
```

### 14.2 Commit + hook failure

```
UI → App: Commit(message)
App → RepoActor: commit
RepoActor → Git: commit -F -
Git → Hook: pre-commit (stream)
Hook → RepoActor: output, exit 1
Git → RepoActor: exit 1, no commit
RepoActor → App: HookFailed(log)
App → UI: show hooks panel, disable Commit, enable Retry
```

### 14.3 Push auth failure

```
UI → App: Push
App → Sync: push
Sync → Git: push --porcelain
Git → Sync: stderr "Authentication failed"
Sync → Auth: classify
Auth → Sync: AuthFailed
Sync → App: error + action(SignIn/UseSSH)
App → UI: auth banner; local commits preserved
```

### 14.4 Merge conflict

```
App → Ops: detect MERGE_HEAD
Ops → Git: status v2
Git → Ops: unmerged paths
Ops → Git: show :1/:2/:3
Git → Ops: base/ours/theirs
Ops → UI: conflict model
UI → Ops: save result + mark resolved
Ops → Git: add path
Ops → Git: merge --continue
Git → Hook: pre-commit/commit-msg
```

---

## 15. Errors

Use one typed error enum in `gitui-core`; map Git exit/stderr to a category + user action + details.

| Category | Example | UI |
|---|---|---|
| `GitMissing` | binary/version | Settings deep link |
| `NotARepo` | no `.git` | Init/Open parent |
| `RepoMissing` | path gone | Locate/Remove |
| `IndexLock` | `.git/index.lock` | retry/kill guidance |
| `AuthRequired` | credential needed | Sign in |
| `AuthFailed` | bad token | update credentials |
| `HostKey` | SSH mismatch | show fingerprint |
| `Network` | offline/timeout | Retry |
| `NonFastForward` | push rejected | Pull/Rebase |
| `Conflict` | unmerged | conflict view |
| `HookFailed` | exit != 0 | log/retry/bypass |
| `HookTimeout` | 5 min | log/retry |
| `OperationInProgress` | merge/rebase | continue/abort |
| `Permission` | FS/Keychain | system settings |
| `Disk` | full/write fail | path/space |
| `Internal` | bug | diagnostic + report |

Every error has: code, summary, detail, retryable, actions. No silent failure.

---

## 16. Performance

- UI main thread never blocks > 16 ms; Git/IO off-main.
- History pages 100–200; diff chunks; syntax highlight on demand.
- Commit/file tables virtualized; no view per row.
- Process output cap; avoid unbounded strings.
- Per-repo caches invalidated by generation; no cross-repo reuse.
- Budgets in [SPEC.md](../SPEC.md) §12; verify in [TEST-PLAN.md](TEST-PLAN.md).

---

## 17. Observability / diagnostics

- Structured logs (`tracing`) with repo id, operation id, duration, exit code; no secrets.
- In-app Diagnostics: Git version/path, repo state, recent operations, redacted logs; Copy.
- Crash reporting opt-in; no repo contents/tokens.
- Audit log for destructive/`--no-verify`/Agent.

---

## 18. Deployment

- Single signed/notarized app bundle; Sparkle 2 feed; no MAS v0.1.
- Bundle ID/name finalized before beta (ADR).
- macOS 14+; Apple Silicon.
- Update channel stable/beta; signature verification.

---

## 19. Open architecture questions

1. Exact NSH Rust APIs for windows/views/tables/editor/sheets; pin after S-6.
2. Single process vs sidecar: **single process is the decision**; sidecar only if NSH cannot host long-running operations, via new ADR.
3. `gix` read-only fallback only if S-4 fails; write stays CLI.
4. Sparkle integration details (NSH/AppKit bridge) in M7.
5. Agent provider interface; v0.2.

---

## 20. ADR index

- [0001 Git CLI first](adr/0001-git-cli-first.md)
- [0002 NSH + Rust single process](adr/0002-nsh-rust-single-process.md)
- [0003 Index-first / changelists metadata](adr/0003-index-first-changelists.md)
- [0004 Direct notarized distribution](adr/0004-direct-notarized-distribution.md)
- [0005 Hooks first-class](adr/0005-hooks-first-class.md)
- [0006 Agent ask-first](adr/0006-agent-ask-first.md)
- [0007 Defer interactive rebase](adr/0007-defer-interactive-rebase.md)
