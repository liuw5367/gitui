# SPIKE-REPORT — M0 validation

**Date**: 2026-11-14  
**Status**: **Conditional — not a full Go**  
**Environment**: macOS 15.7.7 (24G720), Apple M1 Pro, Git 2.56.0  
**Target stack**: NSH + Rust only (no TSX/JS/HTML UI)  
**Product spec**: [../SPEC.md](../SPEC.md)

---

## 1. Summary

| Spike | Scope | Result |
|---|---|---|
| **S-1** | hunk/line staging, partial commit, reverse unstage | **PASS** |
| **S-2** | pre-commit / post-commit / `--no-verify` | **PASS** (timeout not tested) |
| **S-3** | merge conflict stages, editable result, abort; rebase conflict/abort | **PASS** (cherry-pick/revert not tested) |
| **S-4** | performance micro-benchmark | **PARTIAL** (20k commits, not 100k; no 10k diff / 1 GB) |
| **S-5** | HTTPS/SSH auth, credential helper, failure classes | **PENDING** |
| **S-6** | NSH Rust view + 3-pane editor + 10k-line rendering | **PENDING — BLOCKING** |
| **S-7** | CLI vs `gix` read path (optional) | **NOT RUN** |

**Go/No-Go**

- S-1/S-2/S-3 prove the **Git CLI write path is viable**.
- S-4 is promising but does **not** prove the 100k/1 GB budget.
- S-5 and S-6 are **not closed**. M1 shell work may begin only after a basic S-6 pass. **M2/M4 must not begin until S-5 and S-6 pass.**
- No result below is fabricated; S-5/S-6 are explicitly pending.

---

## 2. S-1 — line staging / partial commit

**Fixture**: `a.txt` with `two` changed to `TWO` and `four` changed to `FOUR`; a single hunk patch for line 2 only.

**Observed**

- `git apply --cached --unidiff-zero one.patch`: **34.4 ms**.
- After apply: `git diff --cached` contained only `two → TWO`; `git diff` still contained `four → FOUR`.
- `git commit` produced `819756f partial`; `git show --stat` showed **1 file changed, 1 insertion, 1 deletion**.
- `git show HEAD:a.txt` contained `TWO` and the original `four`; the working tree still had `FOUR`.
- Reverse apply on a fresh equivalent state left `MM a.txt` (staged line reverted, working tree still modified). This is expected, not corruption.
- Patch application failed safely when the patch did not match; index was unchanged.

**Conclusion**: PASS for hunk/line staging and partial commit. Binary, rename, delete, and conflict-adjacent staging remain fixture work.

**Risks**

- UI must generate patches from **the current index**, not from a stale diff snapshot.
- Patch generation for line-level operations needs careful context handling (`--unidiff-zero`).
- Always refresh `git status` after apply; hooks and external tools may change the index.

---

## 3. S-2 — commit hooks lifecycle

**Observed**

| Case | Result |
|---|---|
| `pre-commit` exits 1 | `git commit` exit **1**; commit not created; `HEAD` still `base`; stderr contained the hook output |
| `pre-commit` exits 0 | commit succeeds |
| `post-commit` exits 1 | `git commit` exit **0**; commit **survives**; hook stderr is emitted as a warning |
| `--no-verify` | commit succeeds; both pre-commit and post-commit bypassed |

- Hook stdout/stderr are available to the process runner; no special Git plumbing required.
- A failing `pre-commit` leaves the index intact, so Retry is safe.
- Git does **not** propagate `post-commit` failure as a commit failure. The UI must show a separate warning; it must not claim the commit failed.
- Timeout and process-group cancellation were **not** tested; S-2 is not fully closed for the 5-minute/kill requirement.

**Conclusion**: PASS for the main lifecycle; timeout/cancel test still required.

---

## 4. S-3 — merge / rebase conflicts

**Merge conflict observed**

- `git status --porcelain=v2` reported `u UU ... f.txt`.
- `git show :1:f.txt` → `a b c` (base).
- `git show :2:f.txt` → `a B-main c` (ours).
- `git show :3:f.txt` → `a B-feature c` (theirs).
- Result file edited to `a B-merged c`; `git add f.txt`; `git commit` succeeded; `git show HEAD:f.txt` contained `B-merged`.
- Clean abort test: after `git merge --abort`, worktree clean; `HEAD` back at `main`; `.git/MERGE_HEAD` absent.

**Rebase conflict observed**

- `git rebase main` on `feature` exited 1; `.git/rebase-merge/` existed; status showed `UU f.txt`.
- `git rebase --abort` restored `feature`; `.git/rebase-merge/` absent; status clean.

**Conclusion**: PASS for merge and rebase conflict read/abort and merge resolution. Cherry-pick, revert, `--skip`, and hook retry inside a merge commit were **not** tested.

**Implication for NSH/Rust**: the Rust core must read stages via `git show :1:/:2:/:3:` (not only parse markers), and the NSH editor must allow editing the result before `git add`.

---

## 5. S-4 — performance micro-benchmark

**Fixture**: generated **20,001 commits**, 4.6 MB `.git`; single 5-byte blob. Not the 100k/1 GB target.

**Observed**

| Measurement | Result |
|---|---|
| `git log --max-count=100` + parse | **37.8 ms** |
| `git status --porcelain=v2 --branch` | **35.2 ms** |
| 20k commit count | 20,001 |
| `.git` size | 4.6 MB |

**Conclusion**: PARTIAL. 20k log/status is fast enough to be promising; this does **not** validate 100k commits, 10k-line diffs, 1 GB repos, or UI rendering. A real S-4 run is still required.

**Risks**

- Git log parse and cross-process overhead can dominate before UI work.
- 10k-line diff rendering is an NSH/AppKit view problem, not a Git CLI problem.
- Large working trees and FSEvents load need separate tests.

---

## 6. S-5 — auth / network (PENDING)

**Not run.** No disposable remote/token/SSH test host in this workspace.

**Required before M3**

- HTTPS clone/fetch/push with a test token; verify token never appears in argv, logs, or UI.
- Bad token → `auth_failed`; no infinite retry.
- SSH success; host-key change → explicit warning; no auto-accept.
- Offline/timeout → `network`; cancel is clean.
- Non-fast-forward → Pull/Rebase/Force-with-lease prompt; no automatic force.
- Credential helper / Keychain integration on macOS.

**Decision**: M3 is blocked until this report is updated with real results.

---

## 7. S-6 — NSH Rust editor / bridge (PENDING, BLOCKING)

**Not run.** There is no NSH Rust app harness in this workspace yet, and the final stack is **Rust-only NSH**, not TSX.

**Required before M2/M4**

1. Build a minimal **NSH Rust** window with:
   - a 3-pane `NSTextView` (or NSH wrapper) editor;
   - 10k-line file load, scroll, input, undo/redo;
   - a Rust view model driving the panes;
   - save from result pane to disk.
2. Verify Rust ↔ AppKit interop for:
   - `NSTextView`, `NSScrollView`, `NSSplitView`;
   - menu commands, sheets, focus, keyboard;
   - large text without UI stalls.
3. Verify NSH Rust can host:
   - long-running Git operations off the main thread;
   - event/notification delivery to views;
   - cancellation and progress updates.
4. If S-6 fails: fallback to AppKit `NSTextView`/native editor, or reduce the merge editor to a single result pane. **Do not** fall back to a WebView/HTML editor without a new ADR.

**Decision**: M2 (write path) and M4 (merge/rebase) are blocked until S-6 passes or a documented fallback is accepted.

---

## 8. S-7 — CLI vs `gix` read path (optional)

Not run. Revisit only if S-4 fails the 100k/1 GB budget. Write path remains system Git CLI (hooks, credentials, SSH, rebase state).

---

## 9. Decisions confirmed by M0

- **D3 CLI-first** for write path: confirmed by S-1/S-2/S-3.
- **D7 hooks are first-class**: confirmed; post-commit needs a warning, not a failure.
- **D11 Rust-only NSH**: no TSX/JS bridge; all UI/state/Git in Rust.
- **Merge editor** must read unmerged stages and support an editable result.
- **S-4** needs a real fixture before claiming performance; 20k is not enough.
- **S-5/S-6** remain open; M2/M4 blocked.

---

## 10. Next actions

1. Build the NSH Rust harness (S-6) — highest priority.
2. Build a local Git HTTP/SSH fixture and close S-5.
3. Re-run S-4 at 100k commits, 10k-line diff, 1 GB repo.
4. Add timeout/cancel test for S-2.
5. Add cherry-pick/revert/`--skip` cases for S-3.
6. Update this report and [SPEC.md](../SPEC.md) Appendix A after each.
