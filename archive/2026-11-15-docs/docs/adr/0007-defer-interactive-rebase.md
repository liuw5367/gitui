# ADR 0007 — Defer interactive rebase to v0.3

**Status**: Accepted  
**Date**: 2026-11-14

## Context

Interactive rebase (pick/squash/reword/drop/exec) needs a dedicated UI, conflict flow, todo editing, and careful abort/recovery. The 30-board canvas does not include it.

## Decision

- v0.1 supports non-interactive rebase and conflict continue/abort/skip.
- Interactive rebase is deferred to v0.3; no board/page until then.
- Menu entries may be disabled or absent; no fake UI.

## Consequences

- v0.1 scope is achievable; no half-built todo editor.
- Users needing squash/reword must use CLI until v0.3.
- v0.3 requires design + threat review (exec command) before implementation.

## Alternatives

- Ship a minimal todo list: rejected; conflict/abort complexity and data-loss risk.
- No rebase at all: too limiting; non-interactive rebase stays.

## Verification

SPEC D9; no v0.1 tests.

## Amendment 2026-11-14 — fixed-shape rewrite shipped

Squash and reword are now implemented, but **not** as an interactive editor:

- `Repo::reword_commit(sha, message)` and `Repo::squash_commits(from, to, message)`
  write their own todo list and run `git rebase -i` with a generated
  `GIT_SEQUENCE_EDITOR` and `GIT_EDITOR`.
- The shape is fixed: one `reword` line, or one `pick` followed by `squash` lines.
  There is no todo editor, no `drop`, no `exec`, no `edit`. Those stay deferred to v0.3.
- Preconditions are checked in Rust before rebase starts: no operation in progress,
  clean worktree, target on the current branch, contiguous range, and at least two
  commits for a squash. Any failure returns a readable error without touching history.
- A failed rebase is aborted automatically, and a successful rewrite is never pushed.

The `exec` threat that v0.3 needed a review for does not apply: the generated scripts
contain only `cat <file> > "$1"`.
