# gitui — local Git client (M1/M2)

**Rust only. NSH 0.5.x. No TSX, no JS, no HTML UI.**

## Works

- repository path input + Open
- recent repositories (persisted)
- repository tabs (open / activate / close, session persisted)
- status + 15 commits + read-only diff
- Stage All / Unstage All
- per-file Stage / Unstage
- commit message + Commit / No hooks / Amend
- commit and hook output panel

## Not yet

- fetch / pull / push
- merge / rebase conflict editor
- branches / tags / stashes UI
- settings
- per-hunk staging
- hook step list / retry UI

## Run

```sh
GITUI_REPO=/path/to/repo nsh run
```

Or type a path in the header and press **Open**.

## Verify

```sh
nsh check
nsh test nsh.json --trace
```

## Crates

- `src/lib.rs` — NSH Rust UI
- `src/main.rs` — macOS window
- `../../crates/gitui-git` — Git CLI: status/log/diff/stage/commit/amend/no-verify
