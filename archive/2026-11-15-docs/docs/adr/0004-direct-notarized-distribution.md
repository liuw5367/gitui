# ADR 0004 — Direct signed/notarized distribution; no Mac App Store

**Status**: Accepted  
**Date**: 2026-11-14

## Context

gitui needs system Git, hooks, SSH agent, credential helper, Keychain, and arbitrary repo paths. MAS sandboxing restricts these.

## Decision

- Distribute a signed, notarized `.app` directly (DMG/ZIP), with Sparkle 2 updates.
- No Mac App Store build for v0.1.
- Hardened runtime; minimal entitlements; no sandbox for Git/hooks/SSH.
- Final bundle ID/name/domain before beta.

## Consequences

- Full Git/hooks/SSH/Keychain behaviour.
- We own update feed, signing, crash reporting, support.
- No MAS discovery/ payments; distribution/marketing is ours.

## Alternatives

- MAS sandbox: hooks/SSH/credential helper limited; rejected for v0.1.
- Both: doubles packaging/testing; defer.

## Verification

M7 packaging/signing/notarization; update feed smoke test.
