# ADR 0006 — Agent ask-first; no auto push

**Status**: Accepted  
**Date**: 2026-11-14

## Context

An Agent can help with cleanup/commits, but it can also execute destructive or exfiltrating actions. Trust must be explicit and auditable.

## Decision

- Default `Ask first`; per-repo permissions for read/stage/commit/push/force/discard.
- Show a plan (files, message, push target) before execution; Approve/Edit/Cancel.
- Push is separately approved by default; no auto push.
- Agent cannot use `--no-verify`, read Keychain/secrets, force protected branches, or run raw shell.
- All tool calls/results audited and redacted.
- Automation (`commit`, `commit-and-push`) is explicit and revocable.

## Consequences

- Slower than full autonomy, but safer and reviewable.
- Permission UI and audit are required before Agent ships.
- Prompt injection remains a risk; treat repo text as data.

## Alternatives

- Auto-commit/push: rejected for v0.2.
- No Agent: misses product differentiation; deferred, not rejected.

## Verification

[../AGENT-THREAT-MODEL.md](../AGENT-THREAT-MODEL.md); T-AGENT/SEC tests.
