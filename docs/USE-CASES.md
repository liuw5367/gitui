# USE-CASES — gitui v0.1 / v0.1.1 / v0.2

**Status**: v1, 2026-11-14  
**Scope**: executable flows for the vertical slice and core v0.1; v0.1.1/v0.2 marked.  
**Stack**: NSH + Rust only; UI in Rust via NSH; Git in Rust core via system Git CLI.  
**Spec**: [../SPEC.md](../SPEC.md)  
**UI**: [UI-SPEC.md](UI-SPEC.md)  
**Tests**: [TEST-PLAN.md](TEST-PLAN.md)

---

## 0. Conventions

- **Actor**: developer using gitui. **System**: Rust core + NSH UI. **Git**: system Git CLI.
- Each use case has: goal, preconditions, main flow, alternate/failure, postconditions, acceptance, R-IDs, screen.
- `MUST` here means required by SPEC; `SHOULD` means v0.1.1 unless noted.
- All destructive actions require explicit confirmation and are audited. Agent never gets `--no-verify`.
- UI copy is English by default; Simplified Chinese is a full localization, not a partial one.

---

## UC-01 — Open a repository and restore the last session

**Goal**: open a repo and return to the same multi-repo workspace after relaunch.  
**Screen**: 01 Welcome, 03 Repository manager, 04 Workspace  
**R-IDs**: R-REPO-01, R-REPO-02, R-REPO-07, R-REPO-08, R-REPO-09, R-SET-01, R-SET-08

**Preconditions**

- At least one valid Git repository exists.
- The app has a persisted session (or no session on first launch).

**Main flow**

1. User launches gitui.
2. On launch setting is **Restore last session**; system reads the persisted tab list (all repo paths, order, active tab).
3. For each repo: system validates path, `.git`, and worktree; opens a `RepositoryController`; loads status + default history.
4. System restores tabs in order; active tab is the last active repo; each tab keeps its own selection/scroll where possible.
5. If a repo is missing, system opens a non-blocking banner naming it with **Locate** and **Remove**; other repos still open.
6. User can switch tabs with `⌘1…⌘9` and `⌘⇧[` / `⌘⇧]`.

**Alternate / failure**

- **A1 — Show Welcome**: if setting is Show Welcome, launch shows 01 with recent repos; opening a repo creates a tab.
- **A2 — not a repo**: path exists but is not a Git worktree → show “Not a Git repository” with **Init here**, **Open parent**, **Cancel** (R-REPO-07).
- **A3 — missing path**: show Locate/Remove; do not delete the session entry until Remove.
- **A4 — bare repo**: open read-only with a banner; disable working-tree actions (R-REPO-07).
- **A5 — corrupted/unsupported repo**: show diagnostic; do not crash; allow Close tab.

**Postconditions**

- All valid repos are open in tabs; missing repos are reported; no repo state is mixed.
- The session is saved on quit and after tab changes.

**Acceptance**

- Given 3 repos in session, when relaunching, then 3 tabs restore in the same order and the active tab is restored.
- Given one repo path is deleted, when relaunching, then the other repos open and a Locate/Remove banner appears for the missing one.
- Given a non-repo folder, when opening, then no tab is created and Init/Open parent are offered.

---

## UC-02 — Browse history, filter commits, and read a diff

**Goal**: find a commit and inspect its files/diff without changing the repo.  
**Screen**: 04 History & diff, 05 History filters, 06 Multi-commit diff, 18 Files at commit  
**R-IDs**: R-HIST-01, R-HIST-02, R-HIST-03, R-HIST-08

**Preconditions**

- A repo is open with at least one commit.

**Main flow**

1. User selects the History view; system loads the first page (e.g. 100 commits) and renders graph/message/refs/author/date.
2. User filters by branch, date range, author, path, or message; system applies the filter and shows result count.
3. User selects a commit; system loads commit metadata + changed files + diff; default split view.
4. User toggles split/unified, folds hunks, toggles whitespace/word highlight.
5. User selects a file; system shows that file’s diff at the commit.
6. User selects multiple commits; system shows aggregate diff and multi-commit selection bar.
7. User copies SHA, opens the commit on the remote, or opens **Files at commit**.

**Alternate / failure**

- **A1 — empty repo**: show “No commits yet” with Init/first-commit guidance.
- **A2 — filtered empty**: show “No commits match” and Clear filters.
- **A3 — binary file**: show “Binary file” with size; no text diff.
- **A4 — huge diff**: show first chunk + “Load more”; never freeze; cancel on selection change.
- **A5 — history load error**: show error + Retry; keep previous list.

**Postconditions**

- No write to the repo; selection/filter state is per tab.

**Acceptance**

- Given 10k commits, when opening History, then first page renders under the budget and scrolling/paging works.
- Given a commit with 3 files, when selected, then the file tree and diff match `git show`.
- Given a filter that matches nothing, then an empty state with Clear filters appears.

---

## UC-03 — Stage a hunk/line and make a partial commit

**Goal**: commit only selected changes; leave other working-tree changes untouched.  
**Screen**: 10 Staging & commit  
**R-IDs**: R-STAGE-01, R-STAGE-02, R-STAGE-05, R-STAGE-06, R-STAGE-07

**Preconditions**

- Repo has unstaged changes; no operation in progress (or user accepts merge commit context).

**Main flow**

1. User opens Changes; system reads `git status --porcelain=v2` and shows Staged/Unstaged.
2. User expands a file, selects one hunk (or line range), and clicks **Stage hunk/line**.
3. Rust core generates a patch against the current index and runs `git apply --cached --unidiff-zero`.
4. System refreshes status; the selected hunk moves to Staged; other changes remain unstaged.
5. User enters summary/description; composer shows **staged file count** and “Only staged files are committed.”
6. User clicks **Commit N Staged Files**; system runs hooks (UC-04), then `git commit -F -`.
7. On success, system refreshes status/history; tabs/sidebar update; commit appears in History.

**Alternate / failure**

- **A1 — patch does not apply**: show “Index changed; refresh and retry”; do not partially apply; refresh.
- **A2 — binary/rename/delete**: only file-level staging is offered; line staging disabled.
- **A3 — no staged files**: commit disabled; hint “Stage something first.”
- **A4 — hook fails**: see UC-04; commit is blocked.
- **A5 — external index change**: refresh before apply; if still stale, abort safely.
- **A6 — commit fails after hook pass**: show Git error + Retry; working tree/index preserved.

**Postconditions**

- Commit contains exactly the staged hunks; unstaged changes remain; no index lock; history refreshed.

**Acceptance**

- Given a file with two separate changes, when staging only the first hunk, then `git show HEAD` contains only that change and the second remains in the working tree.
- Given a failed patch apply, then the index is unchanged and the error is actionable.
- Given no staged files, the commit button is disabled.

---

## UC-04 — Commit blocked by pre-commit; view log and retry

**Goal**: understand why the commit failed and retry safely.  
**Screen**: 10 Staging & commit (commit panel)  
**R-IDs**: R-HOOK-01, R-HOOK-02, R-HOOK-04, R-HOOK-05, R-HOOK-06, R-HOOK-08

**Preconditions**

- Staged changes exist; repo defines a `pre-commit` hook that fails.

**Main flow**

1. User clicks Commit.
2. Rust core starts `git commit -F -`; streams hook output.
3. `pre-commit` exits non-zero; commit is not created; subsequent hooks are skipped.
4. UI shows **Hooks · Pre-commit failed · duration**, step chips (`pre-commit` failed; `prepare-commit-msg`, `commit-msg`, `post-commit` skipped), and a scrollable log with command, stdout/stderr, exit code.
5. Commit button is disabled; **Retry hooks** and **Commit without hooks** are offered.
6. User fixes the issue outside/inside gitui and clicks **Retry hooks**.
7. Hook passes; commit completes; UI shows success and refreshes.

**Alternate / failure**

- **A1 — hook timeout**: after 5 minutes (configurable), kill the process group; show **Timed out**; no commit.
- **A2 — hook output too large**: cap live UI at 1 MB; write full log to a temp file; **View full log** opens it.
- **A3 — hook modifies index**: refresh status after hook; UI reflects new staged state; commit may proceed with the new index only after re-confirmation.
- **A4 — post-commit fails**: commit survives; show warning, not failure (R-HOOK-03).
- **A5 — first commit in repo with hooks**: show a one-time notice “This repository defines hooks” before running; do not auto-bypass.

**Postconditions**

- No commit if pre-commit/commit-msg failed; hook log accessible; retry safe; no orphan process.

**Acceptance**

- Given a failing pre-commit, when committing, then HEAD does not change and the log contains the hook stderr.
- Given Retry after fixing, then the commit succeeds.
- Given a 5-minute timeout, then the hook is killed and no commit exists.

---

## UC-05 — Bypass hooks explicitly (`--no-verify`)

**Goal**: allow a deliberate bypass with clear risk and audit, never silently.  
**Screen**: 10 Staging & commit (hook failure panel)  
**R-IDs**: R-HOOK-02, R-HOOK-06, R-AGENT-03, R-SET-05

**Preconditions**

- A hook failed or user explicitly chooses bypass; user has permission (not Agent).

**Main flow**

1. User clicks **Commit without hooks**.
2. Confirmation sheet states: hooks will not run; `git commit --no-verify` will be used; this is audited.
3. User confirms; Rust runs `git commit --no-verify -F -`.
4. Commit completes; audit entry records repo, user, timestamp, message SHA, reason if provided.
5. UI shows a subtle “Committed without hooks” badge in the result; History commit info shows the same audit flag.

**Alternate / failure**

- **A1 — user cancels**: no commit; return to hook failure.
- **A2 — commit fails**: show error; audit records attempt? Record only actual bypass invocation, with result.
- **A3 — Agent**: the action is unavailable; Agent cannot pass `--no-verify`.

**Postconditions**

- Commit exists; audit entry exists; no silent bypass.

**Acceptance**

- Given confirmation, then `--no-verify` is used and audit records it.
- Given Agent context, then the action is disabled and an API call is rejected.

---

## UC-06 — Fetch and pull with autostash / rebase

**Goal**: update remote refs and integrate upstream without losing local work.  
**Screen**: 15 Sync & auth, 28 Sync Fetch, 29 Sync Pull  
**R-IDs**: R-SYNC-01, R-SYNC-02, R-SYNC-04, R-SYNC-05, R-SYNC-06, R-SYNC-07

**Preconditions**

- Remote configured; credentials available or user can sign in; working tree may be dirty.

**Main flow**

1. User opens Sync → Fetch; chooses remote, prune/tags/all-remotes; clicks Fetch.
2. Rust runs `git fetch --progress --prune`; streams progress/log; working tree untouched.
3. UI updates ahead/behind and remote-tracking refs.
4. User opens Pull; chooses Merge/Rebase/FF-only; autostash on; clicks Pull.
5. Rust runs the corresponding pull; on conflict, transitions to UC-08/UC-09.
6. On success, UI refreshes history/status/branch; shows result.

**Alternate / failure**

- **A1 — auth required/failed**: show Sign in / Use SSH; preserve operation; no token in log.
- **A2 — network offline/timeout**: show Retry/Offline; cancel clean.
- **A3 — non-fast-forward**: show Pull/Rebase; never auto-force.
- **A4 — autostash conflict**: show stash apply conflict; keep stash; explain.
- **A5 — user closes sheet**: operation continues in background with a status indicator; explicit Cancel kills it.

**Postconditions**

- Fetch does not change worktree; pull either integrates or leaves a clear conflict/abort path; no lost local changes.

**Acceptance**

- Given a dirty worktree and autostash, when pull succeeds, then local changes are restored.
- Given a conflict, then the merge/rebase flow opens with the correct stages.
- Given a bad token, then an auth error with Sign in appears and no secret is logged.

---

## UC-07 — Push rejected by auth or non-fast-forward

**Goal**: recover from the two most common push failures without terminal use.  
**Screen**: 15 Sync & auth (Push), 27 Settings · Credentials  
**R-IDs**: R-SYNC-03, R-SYNC-04, R-SYNC-05, R-SYNC-06, R-SET-04

**Preconditions**

- Local commits ahead; remote configured.

**Main flow**

1. User opens Sync → Push; target branch/remote shown; set upstream if needed; tags/force-with-lease options.
2. User clicks Push; Rust runs `git push --porcelain --progress`.
3. On success, UI updates ahead/behind and history.

**Alternate / failure**

- **A1 — auth failed**: show “Authentication failed” + **Sign in** / **Use SSH**; preserve local commits; no retry loop.
- **A2 — non-fast-forward**: show “Remote has new commits” + **Pull (rebase)** / **Pull (merge)**; force-with-lease only after explicit confirmation.
- **A3 — protected branch / remote rejected**: show server message; do not auto-force.
- **A4 — network/timeout**: show Retry; cancel clean.
- **A5 — pre-push hook fails**: show hook log in Sync activity; push blocked; no `--no-verify` for Agent.

**Postconditions**

- Local commits preserved; remote unchanged on failure; clear next action.

**Acceptance**

- Given a rejected non-fast-forward, then Pull is offered and force is not automatic.
- Given bad credentials, then Sign in appears and no token is written to logs.
- Given a pre-push failure, then the push does not occur.

---

## UC-08 — Resolve a merge conflict with an editable result

**Goal**: resolve conflicts correctly and continue the merge.  
**Screen**: 14 Merge conflict  
**R-IDs**: R-MERGE-01, R-MERGE-02, R-MERGE-03, R-MERGE-04, R-MERGE-05, R-MERGE-07

**Preconditions**

- A merge is in progress with at least one unmerged path.

**Main flow**

1. System detects `.git/MERGE_HEAD`; shows merge banner (files/conflicts) and opens conflict view.
2. For the selected file, Rust reads `git show :1:path`, `:2:path`, `:3:path` and renders Base/Ours/Theirs/Result.
3. Result is editable; user can Accept Ours/Theirs/Both, or type a manual resolution.
4. User marks resolved; system saves result and runs `git add -- path`.
5. Continue is enabled only when all conflicts are resolved; user clicks **Apply & Continue**.
6. Rust runs `git merge --continue`; commit hooks run (UC-04); on success, merge completes.
7. Abort is always available with confirmation; `git merge --abort` restores pre-merge state.

**Alternate / failure**

- **A1 — unresolved files**: Continue disabled; count visible; clicking explains.
- **A2 — save fails**: keep editor content; show error; do not mark resolved.
- **A3 — hook fails on merge commit**: keep result; show hook panel; retry; do not lose resolution.
- **A4 — abort**: confirm; restore; close conflict view.
- **A5 — external conflict edit**: refresh/merge; warn if file changed on disk.

**Postconditions**

- Merge commit contains the resolved result; no MERGE_HEAD; hooks honored; abort restores.

**Acceptance**

- Given a conflict, when editing Result to `B-merged`, then the merge commit contains `B-merged`.
- Given unresolved files, Continue is disabled.
- Given abort, worktree and HEAD return to pre-merge.

---

## UC-09 — Resolve a rebase conflict; continue/abort/skip

**Goal**: complete or safely abandon a rebase.  
**Screen**: 14 Merge conflict (rebase variant)  
**R-IDs**: R-MERGE-01, R-MERGE-02, R-MERGE-03, R-MERGE-04, R-MERGE-05, R-MERGE-06

**Preconditions**

- Rebase in progress; `.git/rebase-merge/` or `rebase-apply/` exists.

**Main flow**

1. System detects rebase and shows step count (`2 of 5`) and conflicts.
2. User resolves each file (same 3-pane editor); marks resolved.
3. User clicks **Continue**; Rust runs `git rebase --continue`.
4. If more conflicts, next step opens; if done, branch updated.

**Alternate / failure**

- **A1 — Skip**: available only for rebase; confirm; `git rebase --skip`; continue.
- **A2 — Abort**: confirm; `git rebase --abort`; original branch restored; rebase dir removed.
- **A3 — empty commit/step**: show “Nothing to commit”; Continue/Skip guidance.
- **A4 — hook fails**: show hook log; do not lose result; retry.
- **A5 — branch changed externally**: refresh; block until resolved.

**Postconditions**

- Rebase completes or aborts cleanly; no rebase dir left; history correct.

**Acceptance**

- Given a rebase conflict, when resolving and continuing, then the rebased branch contains the resolution.
- Given abort, then original branch/HEAD restored and rebase dir gone.
- Given skip, then the commit is skipped and the rebase advances.

---

## UC-10 — Organize changes with changelists (without changing commit scope)

**Goal**: group/filter files while keeping index-first commit semantics.  
**Screen**: 10 Changes, 11 Changelists  
**R-IDs**: R-STAGE-03, R-STAGE-04

**Preconditions**

- A repo with modified files; at least one changelist exists or Default.

**Main flow**

1. User creates/renames a changelist; assigns files via context menu or drag.
2. Active changelist filters the Changes list; it does not stage or unstage.
3. User stages a file/hunk; the staged item remains in its changelist group but commit scope is still the index.
4. Composer shows “3 staged” regardless of active changelist; “Changelists are filters; staging commits.”
5. User can move a file between changelists without touching the index.

**Alternate / failure**

- **A1 — active changelist has no staged files**: commit uses staged files from all changelists; no forced move to Default.
- **A2 — delete changelist**: files move to Default; staging unchanged.
- **A3 — file deleted on disk**: group entry removed; no crash.

**Postconditions**

- Changelists persist; commit scope equals index; no file movement required.

**Acceptance**

- Given a staged file in changelist “UI”, when committing with “Docs” active, then the UI file is still committed.
- Given moving a file between changelists, then the index is unchanged.

---

## UC-11 — Recover a mistaken reset/checkout with reflog (v0.1.1)

**Goal**: restore a previous HEAD/branch state without losing current work.  
**Screen**: 17 Reflog & undo  
**R-IDs**: R-REC-01, R-REC-02

**Preconditions**

- Reflog entries exist; working tree state known.

**Main flow**

1. User opens Reflog; sees HEAD/branch entries with message/time/hash.
2. User selects an entry; sees commit details/diff.
3. User chooses **Restore** → **Create branch here** (default, safest) or **Reset current branch** (destructive, confirm).
4. Rust creates branch or runs reset; refreshes history/status; audit entry.

**Alternate / failure**

- **A1 — dirty worktree + reset**: warn; offer stash or cancel; never silently discard.
- **A2 — entry pruned**: show unavailable; refresh.
- **A3 — wrong branch**: show current branch and target hash before confirming.

**Postconditions**

- Recovery creates a branch or resets with explicit consent; reflog preserved.

**Acceptance**

- Given a reset, when restoring via new branch, then the old commit is reachable and current work is untouched.
- Given dirty worktree + reset, then a stash/cancel choice appears.

---

## UC-12 — Approve an Agent plan before it commits (v0.2)

**Goal**: let Agent propose work while the user keeps control.  
**Screen**: 12 Agent, 13 Agent permissions  
**R-IDs**: R-AGENT-01, R-AGENT-02, R-AGENT-03, R-AGENT-04, R-AGENT-05, R-AGENT-06

**Preconditions**

- Agent enabled for this repo; permissions set; user asks for a task.

**Main flow**

1. Agent reads allowed context (no secrets), proposes a plan: files to stage, commit message, push yes/no.
2. UI shows plan with **Approve & Run**, **Edit**, **Cancel**.
3. On approve, Agent stages within its permission; commit runs hooks; if hook fails, log shown; Agent cannot bypass.
4. Push requires separate approval unless automation explicitly allows commit-and-push; protected branches blocked.
5. All tool calls/results are logged (redacted); user can stop at any time.

**Alternate / failure**

- **A1 — no commit permission**: Agent can propose but not commit; user executes.
- **A2 — hook failure**: Agent stops; log shown; no auto-retry with `--no-verify`.
- **A3 — prompt injection asks for secrets/push**: refused; audit; no tool call.
- **A4 — protected branch**: push blocked; explain.
- **A5 — network/auth failure**: stop; report; no infinite retry.

**Postconditions**

- Commit/push only within permissions and approval; audit complete; no secrets in logs.

**Acceptance**

- Given no push permission, when Agent plans a push, then push is not executed.
- Given hook failure, then Agent cannot bypass and reports the log.
- Given a malicious instruction, then secrets are not read/sent.

---

## UC-13 — Configure identity, credentials, SSH, and launch behavior

**Goal**: make commits/push work with the right identity and auth.  
**Screen**: 23 Settings, 27 Settings · Credentials, 24 Settings · Git, 25 Shortcuts  
**R-IDs**: R-SET-01, R-SET-02, R-SET-03, R-SET-04, R-SET-05, R-SET-07, R-SET-08

**Preconditions**

- App installed; Git available or installable.

**Main flow**

1. User opens Settings → General; sets name/email or uses global; selects theme/language/date/launch.
2. User opens Credentials; connects GitHub/GitLab or configures credential helper; adds SSH key; sets editor/diff/merge/terminal.
3. User opens Git; chooses Git executable/PATH; scans folders.
4. User records shortcuts; conflicts are flagged.
5. Changes apply immediately or show “restart required”; secrets go to Keychain.

**Alternate / failure**

- **A1 — invalid identity**: commit blocked with a clear message; Settings deep-link.
- **A2 — Git missing**: banner + Install CLT/Choose path.
- **A3 — SSH key invalid**: show fingerprint/error; do not delete key.
- **A4 — Keychain denied**: show permission guidance; no plaintext fallback.
- **A5 — shortcut conflict**: block save until resolved or Replace.

**Postconditions**

- Identity/auth persist; no secret in SQLite/log; launch behavior saved.

**Acceptance**

- Given a local identity, then commits use it; if “use global” is off, global is ignored.
- Given a bad SSH key, then push shows an actionable error and the key remains.
- Given a shortcut conflict, then saving is blocked with a clear choice.

---

## 14. Traceability

| Use case | Screens | Release | Key R-IDs |
|---|---|---|---|
| UC-01 | 01/03/04 | v0.1 | R-REPO-01/02/07/08/09, R-SET-08 |
| UC-02 | 04/05/06/18 | v0.1 | R-HIST-01/02/03/08 |
| UC-03 | 10 | v0.1 | R-STAGE-01/02/05/06/07 |
| UC-04 | 10 | v0.1 | R-HOOK-01/02/04/05/06/08 |
| UC-05 | 10 | v0.1 | R-HOOK-02/06, R-AGENT-03 |
| UC-06 | 15/28/29 | v0.1 | R-SYNC-01/02/04/05/06/07 |
| UC-07 | 15/27 | v0.1 | R-SYNC-03/04/05/06, R-SET-04 |
| UC-08 | 14 | v0.1 | R-MERGE-01…05/07 |
| UC-09 | 14 | v0.1 | R-MERGE-01…06 |
| UC-10 | 10/11 | v0.1 | R-STAGE-03/04 |
| UC-11 | 17 | v0.1.1 | R-REC-01/02 |
| UC-12 | 12/13 | v0.2 | R-AGENT-01…06 |
| UC-13 | 23/24/25/27 | v0.1 | R-SET-01…08 |
