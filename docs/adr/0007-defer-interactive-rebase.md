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
