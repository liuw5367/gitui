# ADR 0005 — Commit hooks are first-class with logs

**Status**: Accepted  
**Date**: 2026-11-14

## Context

Many repos run pre-commit/commit-msg hooks. A Git client that only says “commit failed” is not trustworthy; users need the hook output and a safe retry/bypass path.

## Decision

- Commit panel shows `pre-commit`, `prepare-commit-msg`, `commit-msg`, `post-commit` status, duration, exit code, and streamed log.
- Failed pre-commit/commit-msg blocks commit; Retry is safe (index intact).
- post-commit failure is a warning; commit survives.
- `--no-verify` only by explicit confirmation; audited; Agent denied.
- Timeout 5 min; kill process group; output cap 1 MB live; full log temp.
- First commit in a repo with hooks shows a one-time notice; no silent bypass.

## Consequences

- Users see real hook errors and can fix/retry.
- We must redact secrets and handle large output.
- Hook execution is user-initiated; no auto-run on open.

## Alternatives

- Hide hook output / just show failure: rejected.
- Auto `--no-verify`: rejected (unsafe).

## Verification

S-2 pass; T-HOOK-01/02/03; UI-SPEC S2.
