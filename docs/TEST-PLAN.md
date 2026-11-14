# TEST-PLAN — gitui

**Status**: v1, 2026-11-14  
**Stack**: NSH + Rust only. Tests include Rust unit/integration, NSH UI tests, Git fixtures.  
**Spec**: [../SPEC.md](../SPEC.md)  
**Use cases**: [USE-CASES.md](USE-CASES.md)  
**Architecture**: [ARCHITECTURE.md](ARCHITECTURE.md)

---

## 1. Strategy

| Layer | Scope | Tool |
|---|---|---|
| L0 Golden parsers | porcelain v2, log, diff, reflog, blame | Rust insta/golden files |
| L1 Domain | index, refs, operations, hooks, auth classification | Rust integration tests |
| L2 Process | real `git` CLI against temp repos | Rust + `tempfile` |
| L3 Store | SQLite migrations, session, changelists, audit | Rust |
| L4 UI | NSH views, state matrices, keyboard | NSH test harness |
| L5 End-to-end | vertical slice on real repo | NSH UI + Git fixture |
| L6 Performance | 100k commits, 10k diff, 1GB | benchmark harness |
| L7 Security | injection, secrets, path traversal, Agent | Rust + manual |
| L8 Accessibility/localization | labels, focus, contrast, zh/en | manual + audits |

**Principle**: the test suite is the contract. A requirement without a test/manual check is not done.

---

## 2. Fixtures

All fixtures are generated locally; never use a user’s real repo.

| Fixture | Purpose |
|---|---|
| `empty` | no commits, init flow |
| `basic` | 10 commits, 3 branches, 1 remote |
| `dirty` | staged/unstaged/untracked mix |
| `partial` | two hunks in one file; line/hunk staging |
| `binary` | binary add/modify |
| `rename` | rename + modify |
| `delete` | deleted file, staged/unstaged |
| `conflict-merge` | merge conflict with 3 stages |
| `conflict-rebase` | rebase conflict |
| `conflict-cherry` | cherry-pick conflict |
| `hooks-fail` | pre-commit fail |
| `hooks-timeout` | pre-commit sleep |
| `hooks-post` | post-commit fail |
| `hooks-push` | pre-push fail |
| `detached` | detached HEAD |
| `bare` | bare repo |
| `worktree` | linked worktree |
| `submodule` | submodule |
| `large-log` | 100k commits |
| `large-diff` | 10k-line diff |
| `large-repo` | ~1 GB worktree |
| `auth-local` | local HTTP Git with basic auth |
| `ssh-local` | local SSH server/key |
| `non-ff` | remote ahead; push rejected |
| `protected` | remote protected branch (mock) |

Each fixture has a `README` and a deterministic setup script. No network by default.

---

## 3. Unit / golden tests

### 3.1 Parsers

- `status --porcelain=v2 -z`: modified/staged/renamed/deleted/untracked/conflict; paths with spaces/UTF-8; branch headers.
- `log --format=... -z`: multiline messages, refs, authors, dates, merges.
- `diff`: hunk boundaries, no-newline, CRLF, binary, rename, mode change.
- `reflog`: HEAD/branch entries; date; selector.
- `blame`: porcelain groups; boundary commits.

**Pass**: golden output byte-equal; malformed input returns typed error, never panic.

### 3.2 Index

- hunk apply/reverse; line-level; partial commit; failed patch leaves index unchanged.
- binary/rename/delete file-level only.
- index refresh after external change.
- patch generation from stale diff rejected.

### 3.3 Operations

- state detection for merge/rebase/cherry-pick/revert.
- stages :1/:2/:3; result save; continue/abort/skip.
- operation state survives restart (disk-derived).
- hook failure during merge commit preserves result.

### 3.4 Hooks

- pass/fail/post-fail/timeout/cancel.
- output cap/redaction.
- first-commit notice.
- `--no-verify` audit; Agent denied.

### 3.5 Auth

- classification of stderr → `auth_required`/`auth_failed`/`host_key`/`network`/`non_ff`.
- credential helper invocation; token never in argv/log.
- known_hosts mismatch; no auto-accept.

### 3.6 Store

- migrations from v1; rollback on failure.
- session all-repo restore; missing repo.
- changelist move does not change index.
- audit append-only.

---

## 4. Integration tests (real Git)

Each test uses a temp repo and asserts Git state, not just UI.

| ID | Scenario | Assert |
|---|---|---|
| T-STAGE-01 | stage one hunk | `git show` has only hunk; working tree keeps rest |
| T-STAGE-02 | failed patch | index unchanged |
| T-STAGE-03 | partial commit | commit file count/contents exact |
| T-HOOK-01 | pre-commit fail | HEAD unchanged; stderr captured |
| T-HOOK-02 | post-commit fail | commit exists; warning |
| T-HOOK-03 | timeout | process killed; no commit; no lock |
| T-MERGE-01 | merge conflict | stages correct; result commit |
| T-MERGE-02 | abort | pre-merge HEAD/worktree |
| T-REBASE-01 | rebase conflict | continue/abort/skip |
| T-SYNC-01 | fetch | worktree unchanged |
| T-SYNC-02 | pull autostash | changes restored |
| T-SYNC-03 | non-ff push | rejected; no force |
| T-HIST-02 | commit info | fields match `git show`; parents/refs/signature |
| T-COMPARE-01 | compare refs | two/three-dot counts and file list match |
| T-REF-01 | branch checkout | HEAD/sidebar/history |
| T-REF-02 | delete branch | current branch refused |
| T-REC-01 | reflog restore | commit reachable; worktree safe |

---

## 5. NSH UI tests

- Launch with no repo → Welcome.
- Open 3 repos → 3 tabs, state isolated.
- Changes: stage hunk → staged/unstaged sections update; commit disabled/enabled.
- Hook failure → panel visible, commit disabled, Retry enabled.
- Sync sheet → tabs; auth error banner; cancel.
- Merge → result editable; Continue disabled until resolved.
- Settings → identity/global; shortcut conflict; Keychain denial.
- Dark/light: no hard-coded colors; focus visible.
- Localization: zh/en strings; no clipped buttons at +30% length.
- VoiceOver: labels on icon buttons, table rows, hook status.

Use NSH test harness/snapshot where possible; avoid brittle pixel tests. Assert state/accessibility, not screenshots only.

---

## 6. Performance tests

| Test | Budget |
|---|---|
| Cold start (cached repos) | < 1.5 s |
| Open 100k repo | < 2 s first screen |
| History first 100 | < 500 ms |
| History page N | < 300 ms |
| 10k-line diff | < 600 ms; 60 fps scroll |
| Status refresh | < 200 ms |
| Stage hunk | < 300 ms |
| Tab switch | < 150 ms |
| Memory 100k repo | < 500 MB |

Run on M1 Pro and one Intel/older Mac if supported. Record p50/p95. Fail if main thread blocks > 100 ms during scroll/typing.

---

## 7. Security tests

- Command injection: paths with `;`, `$()`, newlines, leading `-`; args array; `--`.
- Path traversal: `../`, symlink, repo boundary; no writes outside repo.
- Secret leakage: token in URL, stderr, log, crash report, Agent context; redaction.
- Host key: changed fingerprint → no auto-accept.
- Hook execution: explicit commit only; no auto-run on open; first-use notice.
- Agent: no `--no-verify`, no Keychain, no protected branch force, no auto push; prompt injection → refusal/audit.
- Audit: destructive/`--no-verify` entries; immutable.

---

## 8. Accessibility / localization

- Keyboard-only pass for all core flows; no mouse-only action.
- Focus ring visible; focus order matches visual order.
- Contrast: text ≥ 4.5:1; large ≥ 3:1; status not color-only.
- VoiceOver: window/tab/table/editor/hook status; errors announced.
- Reduced motion respected.
- English/Simplified Chinese complete; dates/number locale; no concatenation.

---

## 9. Manual acceptance (v0.1)

- 5 pilot developers, 2 weeks; daily real work.
- Tasks: open 3 repos; stage hunk; commit with failing hook; retry; push auth failure; merge conflict; settings identity/SSH.
- Record blockers, data loss, confusion, time-to-task.
- No P0 data loss; no secret leak; no unrecoverable state.

---

## 10. Traceability

| R-ID | UC | Screen | Test |
|---|---|---|---|
| R-REPO-01/08/09 | UC-01 | S1 | T-SESSION-01, UI-01 |
| R-HIST-01/02/03/08 | UC-02 | S1 | T-HIST-01, PERF-01 |
| R-STAGE-01/02/05/06/07 | UC-03 | S2 | T-STAGE-01/02/03 |
| R-HOOK-01/02/04/05/06/08 | UC-04 | S2 | T-HOOK-01/02/03 |
| R-HOOK-02/06 | UC-05 | S2 | T-HOOK-04, SEC-05 |
| R-SYNC-01/02/04/05/06/07 | UC-06 | S3 | T-SYNC-01/02, AUTH-01 |
| R-SYNC-03/04/05/06 | UC-07 | S3 | T-SYNC-03, AUTH-02 |
| R-MERGE-01…05/07 | UC-08 | S4 | T-MERGE-01/02 |
| R-MERGE-01…06 | UC-09 | S4 | T-REBASE-01 |
| R-STAGE-03/04 | UC-10 | S2 | T-CHANGE-01 |
| R-REC-01/02 | UC-11 | Reflog (17) | T-REC-01 |
| R-AGENT-01…06 | UC-12 | Agent (12/13) | T-AGENT-01, SEC-06 |
| R-REF-01…08 | UC-02/UC-06 | S6 | T-REF-01/02 |
| R-HIST-04/05 | UC-02 | S1 (v0.1.1) | T-HIST-02, T-COMPARE-01 |
| R-SET-01…08 | UC-13 | S5 | T-SET-01/02, SEC-04 |

Missing mapping = not ready.

---

## 11. CI

- `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`.
- Integration tests on macOS runner with Git installed; no network by default.
- Golden tests; fixture setup; deterministic dates/timezones.
- UI tests on NSH harness; smoke on each PR; full nightly.
- Performance weekly on fixed hardware; compare baseline.
- Security audit on release branch.

---

## 12. Release gates

### v0.1

- S-1…S-6 pass (or documented fallback accepted).
- All P0 R-IDs have tests/manual checks.
- 5 pilot users, 2 weeks; no P0 data loss/secret leak.
- Signed/notarized; update feed; crash-free sessions > 99.5%.
- No open `E_*` without user action.

### v0.1.1

- Reflog/blame/compare/refs tests; no regression in core.

### v0.2

- Agent threat model reviewed; permission tests; redaction; no auto-push.
