# ADR 0002 — NSH + Rust single process, no TSX/JS

**Status**: Accepted  
**Date**: 2026-11-14

## Context

The client is a native macOS app. NSH supports Rust directly; the product needs native controls, performance, cancellation, and one type system. An earlier prototype considered TSX + Rust; the final direction is Rust-only.

## Decision

- Build the app with **NSH's Rust API** and AppKit interop. No TSX, no JS runtime, no HTML UI, no WebView for core screens.
- Single process: UI, state, Git services, persistence, Agent all Rust.
- NSH owns the event loop/windows/views; Rust services run on worker threads/runtime.
- Exact NSH APIs are pinned to the NSH version; S-6 validates editor/tables/bridge feasibility.
- If NSH cannot host a required view (e.g. 3-pane editor), fall back to AppKit `NSTextView`/native component via NSH interop, **not** WebView, unless a new ADR accepts it.

## Consequences

- One language/type system; no serialization boundary for UI state.
- Smaller surface; no JS toolchain.
- NSH Rust API maturity is a risk; S-6 is blocking.
- AppKit interop must be carefully contained (main thread, memory, delegates).

## Alternatives

- TSX + Rust bridge: rejected by product direction.
- Swift/AppKit UI + Rust core: viable but two languages and a bridge; not chosen.
- WebView UI: rejected for native feel/performance.

## Verification

S-6: harness built (`spikes/s6-nsh`), `nsh check` passes, 10k-line headless mount passes; interactive performance pending.
