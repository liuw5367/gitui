# gitui — M1 read-only (slice 1)

**Rust only. NSH 0.5.x. No TSX, no JS, no HTML UI.**

Slice 1 opens one repository and shows:

- branch + staged/unstaged counts + raw `git status --porcelain=v2`
- the 15 newest commits
- the selected commit's patch, read-only

There are **no write operations**: no stage, commit, push, merge, settings or tabs.

## Run

```sh
GITUI_REPO=/path/to/repo nsh run
```

Without `GITUI_REPO`, the current directory is opened.

## Verify

```sh
nsh check
GITUI_REPO=/path/to/repo nsh test nsh.json --trace
```

## Layout

- `src/lib.rs` — NSH Rust UI
- `src/main.rs` — macOS window
- `gitui-git` — read-only status/log/diff crate

## Next

- repository path input (open any repo)
- tabs / recent repositories
- then S-5 auth and S-4 performance before M2
