# ADR 0001 — Git CLI first for the write path

**Status**: Accepted  
**Date**: 2026-11-14

## Context

gitui needs index manipulation, hooks, credential helpers, SSH, rebase/merge state, and submodule/worktree support. Reimplementing Git internals is large, risky, and would diverge from user `.gitconfig`/hooks/credential behaviour.

## Decision

- Use the system `git` CLI for **all write operations** and for hooks/auth/network.
- Use CLI for read operations in v0.1; parsing is NUL-delimited (`status --porcelain=v2 -z`, `log --format=... -z`).
- Do not shell out; use arg arrays and a controlled environment.
- A read-only `gix`/libgit2 layer may be added later **only if** performance requires it; writes stay CLI.

## Consequences

- Hooks, credential helper, SSH agent, `.gitconfig`, rebase/merge state behave like the user's Git.
- Process overhead and parsing are real costs; paginate and virtualize.
- We depend on Git 2.39+ and must detect missing/old Git.
- Errors are classified from exit code/stderr; must redact secrets.

## Alternatives

- libgit2 for everything: no hooks/credential helper parity; high risk.
- `gix` write path: promising but not yet proven for hooks/rebase/credentials.
- Reimplement Git: rejected.

## Verification

S-1/S-2/S-3 in [../SPIKE-REPORT.md](../SPIKE-REPORT.md) pass; S-4/S-5 remain.
