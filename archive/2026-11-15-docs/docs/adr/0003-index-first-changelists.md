# ADR 0003 — Index-first; changelists are metadata

**Status**: Accepted  
**Date**: 2026-11-14

## Context

IDEA-style changelists can imply commit scope; Git commit scope is the index. Users want both organization and precise partial commits.

## Decision

- Commit scope is **always** the Git index (`git diff --cached`).
- Changelists are app-side metadata (SQLite): grouping/filtering only.
- Moving a file between changelists never stages/unstages it.
- No requirement to move files to a “Default” changelist before committing.
- UI copy states: “Changelists are filters; staging commits.”

## Consequences

- Predictable partial commits; no hidden scope.
- Changelists survive branch changes as metadata; stale paths are reconciled.
- Users familiar with changelist-as-commit-scope need clear affordance/copy.

## Alternatives

- Changelist = commit scope: conflicts with index and partial commit; rejected.
- No changelists: fails IDEA-style organization.

## Verification

T-CHANGE-01, UC-03/UC-10.
