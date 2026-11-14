# gitui — local Git client (M1/M2 + early M3/M4)

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
- fetch / pull (ff-only) / push against `origin`
- ahead / behind (`main ↑N ↓M`)
- branch list + checkout
- new branch / delete branch (two-step inline confirmation)
- merge / rebase operation banner
- merge Continue / Abort
- conflict list + three-way editor (Base / Ours / Theirs / Result)
- Accept Ours / Theirs / Both; Mark Resolved

## Not yet

- tags / remotes / stashes UI
- settings / credentials
- per-hunk staging
- hook step list / retry UI
- SSH/Keychain auth integration
- rebase/cherry-pick continue/abort (merge only)
- conflict editor polish (Base/Ours/Theirs insertion, not only accept)

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

## Structure

- `src/lib.rs` — App state, actions, composition
- `src/model.rs` — `Loaded`, load, recent/session persistence
- `src/widgets.rs` — shared small widgets
- `src/sections/` — header, tabs, recent, operation, status, history, diff, commit, conflict, statusbar
- `src/main.rs` — macOS window
- `../../crates/gitui-git` — Git CLI: status/log/diff/stage/branch/commit/amend/no-verify/fetch/pull/push/conflicts

## Limits

- no auth UI; HTTPS/SSH credentials come from the system Git setup
- no confirmation for push
- `nsh`/AppKit may log NSTextView layout diagnostics while editing large files
- the app is a prototype, not a production Git client
