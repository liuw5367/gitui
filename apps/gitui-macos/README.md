# gitui — local Git client (M1/M2 + early M3/M4)

**Rust only. NSH 0.5.x. No TSX, no JS, no HTML UI.**

## Works

- repository path input + Open; recent repositories and tabs (session persisted)
- status + 15 commits + read-only diff; ahead/behind (`main ↑N ↓M`)
- Stage All / Unstage All; per-file Stage / Unstage
- per-hunk Stage / Unstage with unstaged/staged diff preview
- commit message + Commit / No hooks / Amend; hook output panel
- fetch / pull (ff-only) / push against `origin`
- branches: list, checkout, create, delete (two-step), rename, merge, rebase
- tags: list, create, delete (two-step)
- remotes: list, add, remove (two-step)
- stashes: push, apply, pop, drop
- merge/rebase/cherry-pick/revert operation banner with Continue / Skip / Abort
- cherry-pick / revert selected commit
- conflicts: list, three-way editor (Base / Ours / Theirs / Result)
- Accept Ours / Theirs / Both; Mark Resolved
- repository-local identity (`user.name` / `user.email`)

## Not yet

- auth UI / Keychain / SSH host-key verification
- hook step list / retry UI
- commit graph, reflog/undo, blame, compare (M5)
- agent workflows (M6)
- signing / notarize / Sparkle (M7)
- interactive rebase (deferred by ADR)
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
