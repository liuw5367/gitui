# gitui S-6 — NSH + Rust harness

**Purpose**: validate the NSH Rust UI stack before the real client starts. No TSX, no JS, no HTML UI.

## Run

```sh
nsh run --fps --trace
```

or:

```sh
cargo run
```

## What to test

1. Click **Load 10k** — Base / Result / Theirs each load 10,000 lines.
2. Scroll all three editors; type in **Result**; undo/redo; copy.
3. Click **Start task** — progress advances; UI stays responsive.
4. Click **Cancel** — the task stops; no hang.
5. Click **Save result** — writes `/tmp/gitui-s6-result.txt`.

## Acceptance (S-6)

- 10k lines load, scroll and edit without freezing.
- Three panes stay aligned and usable at 1280×820.
- Typing latency is acceptable (<16 ms target).
- Undo/redo, save, and task cancel work.
- No TSX/JS/HTML UI.

## Notes

- This is a harness, not the app; it uses `nsh_base::editor()` and `nsh_base::theme()`.
- **Start task** is owned by an `effect` (not a raw `on_press` spawn). The header shows `Task N%`; Cancel stops it. This is a cooperative cancellation test (AtomicBool), not a killed Git process.
- At 10k × 3 editors AppKit may log `-[NSTextView layout] failed to converge after 5 iterations`. Input still works; this is an open S-6 layout/performance risk.
- Exact NSH Rust APIs used here are pinned to NSH 0.5.0; update [ADR 0002](../../docs/adr/0002-nsh-rust-single-process.md) after the S-6 result.
