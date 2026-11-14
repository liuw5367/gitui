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
| **S-4** | performance benchmark | **PARTIAL** — 100k commits and real 10k diff pass; 1 GB and UI mount pending |
| **S-5** | HTTP basic auth, credential helper, failure classes | **PARTIAL — HTTP basic pass; SSH/network pending** |
| **S-6** | NSH Rust view + 3-pane editor + 10k-line rendering | **PARTIAL — 10k headless mount passes; interactive pending** |
| **S-7** | CLI vs `gix` read path (optional) | **NOT RUN** |

**Go/No-Go**

- S-1/S-2/S-3 prove the **Git CLI write path is viable**.
- S-4 is promising but does **not** prove the 100k/1 GB budget.
- S-6 now has a **basic headless pass**: 3 editors × 10k lines mount with virtualization, `nsh check` passes, and the test mounts successfully. Interactive typing/scroll/FPS is still pending, so M1 may begin only after that manual pass. **M2/M4 must not begin until S-5 and S-6 are fully closed.**
- No result below is fabricated; S-5 is pending, S-6 is partially closed.

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

## 5. S-4 — performance benchmark

**Fixtures**

- 100,001 commits, 23 MB `.git`.
- 10,000-line file with **all 10,000 lines changed**.

**Observed**

| Measurement | Result |
|---|---|
| `git log --max-count=100` + parse | **37.1 ms** |
| `git status --porcelain=v2 --branch` | **37.9 ms** |
| `git diff` for 10k changed lines | **43.3 ms** |
| diff output | **20,005 lines**, 504 KB |
| `.git` size | 23 MB |

**Conclusion**: the CLI read path is comfortably inside the S-4 targets for 100k commits and a real 10k-line diff. This does **not** yet validate a 1 GB working tree or NSH rendering a 100k-commit repo / 10k-line diff.

**Still pending**

- ~1 GB working tree status/open.
- NSH app mount against the 100k-commit fixture.
- UI diff rendering at 10k lines.

**Risk**: process/parse timings are fine; the remaining risk is UI virtualization and memory, not Git CLI.

## 6. S-5 — auth / network (PARTIAL)

**HTTP basic fixture**: `spikes/s5-auth/run.py` runs a local `git http-backend` with basic auth; credentials come from `GIT_ASKPASS`, never the URL or argv.

**Observed**

| Case | Result |
|---|---|
| no credentials | exit 128; `could not read Username ... terminal prompts disabled` |
| wrong password | exit 128; authentication failed |
| correct password | exit 0; **2 refs** returned |
| non-fast-forward push | exit 1; push rejected |
| server stopped | exit 128; connection refused |
| credentials in argv | **no** (askpass only) |

**Conclusion**: HTTP basic auth, non-fast-forward rejection and a stopped server are distinguishable. This does not yet validate:
- SSH + host-key change
- proxy / TLS certificate errors
- timeout and cancellation
- Keychain / credential-helper integration in the Rust app

**Risk**: the app-level auth service is still unbuilt; the CLI path is proven, the UI/error classification is not.

## 7. S-6 — NSH Rust editor / bridge (PARTIAL)

**Harness**: `spikes/s6-nsh` — Rust-only NSH app, no TSX/JS/HTML. NSH 0.5.0, Rust 1.100.0, macOS 15.7.7, M1 Pro.

**Passed**

- `nsh check` passes.
- Headless mount with `S6_LOAD_10K=1 nsh test nsh.json --trace`: **3 editors × 10,000 lines**; only **100 visible text rows per editor** (300 text nodes) → virtualization is working.
- Test result: `1 passed`; app test 1.7 s; 6,725 nodes; `perf commit: 0 ops, 204 µs`; `perf turn: 7,380 µs` (timers/tasks 7,160 µs).
- Rust-only UI compiles and mounts; `#[native_module]` is required by NSH's test/describe host even without TSX (it is Rust, not TSX).
- The harness loads 10k in Base/Result/Theirs, has an editable Result, save, and a cooperative task/cancel control.
- **Task bug found and fixed**: `spawn` directly inside an `on_press` handler had no reactive owner and did not run. It is now started by an `effect` with `cleanup(task.cancel())`; the header shows `Task N%` so progress is visible.
- **AppKit warning observed**: `-[NSTextView layout] failed to converge after 5 iterations` at 10k with three panes. Input still works, but this is an open layout/performance risk and must be rechecked after the interactive retest.

**Still pending**

- Interactive typing latency while the 10k editor is focused.
- 60 fps scrolling on all three panes at 1280×820.
- Undo/redo and save round-trip on the result.
- Task cancel responsiveness and main-thread stalls.
- The `NSTextView layout` convergence warning at 10k with three panes.
- Visual alignment/legibility at the target window size.

**Decision**: S-6 is **not fully closed**. M1 shell work is allowed after the manual interactive pass; M2/M4 remain blocked until S-5 and S-6 are fully closed. No TSX/WebView fallback has been used.

**Next**: rebuild/re-run `S6_LOAD_10K=1 nsh run --fps --trace` in `spikes/s6-nsh`; press **Start task** and confirm the header advances `Task N%`; exercise scroll/type/undo/save/cancel; watch for the `NSTextView layout` warning. Record the result.

## 8. S-7 — CLI vs `gix` read path (optional)

Not run. Revisit only if S-4 fails the 100k/1 GB budget. Write path remains system Git CLI (hooks, credentials, SSH, rebase state).

---

## 9. Decisions confirmed by M0

- **D3 CLI-first** for write path: confirmed by S-1/S-2/S-3.
- **D7 hooks are first-class**: confirmed; post-commit needs a warning, not a failure.
- **D11 Rust-only NSH**: no TSX/JS bridge; all UI/state/Git in Rust.
- **Merge editor** must read unmerged stages and support an editable result.
- **S-4** needs a real fixture before claiming performance; 20k is not enough.
- **S-5 SSH/network** remain open; M2/M4 blocked.

---

## 10. Next actions

1. Run the S-6 harness interactively — highest priority.
2. Build a local Git HTTP/SSH fixture and close S-5.
3. Re-run S-4 at 100k commits, 10k-line diff, 1 GB repo.
4. Add timeout/cancel test for S-2.
5. Add cherry-pick/revert/`--skip` cases for S-3.
6. Update this report and [SPEC.md](../SPEC.md) Appendix A after each.
