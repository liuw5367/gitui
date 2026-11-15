# UI-SPEC — gitui core screens

**Status**: v1, 2026-11-14  
**Stack**: NSH + Rust only. All UI is Rust/NSH with AppKit-backed controls. **No TSX, no JS, no HTML UI.**  
**Visual source**: 30-board canvas under `designs/graft-git/`  
**Tokens/control mapping**: [DESIGN-HANDOFF.md](DESIGN-HANDOFF.md)  
**Behaviour**: [USE-CASES.md](USE-CASES.md)  
**Architecture**: [ARCHITECTURE.md](ARCHITECTURE.md)

---

## 0. Global shell

### 0.1 Regions

区域划分与信息层级以设计板为准；字号、行高、控件高度按 [DESIGN-HANDOFF.md](DESIGN-HANDOFF.md) §2.4 的原生刻度，控件实现来自应用的 `ui` 层（[UI-KIT.md](UI-KIT.md)）。

| Region | NSH/AppKit | Rules |
|---|---|---|
| Window | NSWindow via NSH | 1 window, multiple repo tabs; min 1100×700; restore size/position |
| Titlebar | NSH titlebar + AppKit traffic lights | repo tabs, status pill, global actions; 38 px (44 with tabs) |
| Toolbar | NSToolbar/NSH toolbar | repo actions, sync, search, agent; 52 px target; 13 px labels |
| Sidebar | NSVisualEffectView + NSOutlineView | branches/remotes/stashes/tags/changelists; 28 px rows; source-list selection |
| Main | NSSplitView | history/table + diff/files; sidebar/inspector resizable |
| Inspector | NSSplitView pane | commit composer, file/commit details, conflict controls |
| Status bar | NSH status view | branch, operation, ahead/behind, sync state; 24 px |

### 0.2 Shell states

| State | UI |
|---|---|
| No repo | Welcome/Repo manager; toolbar repo actions disabled |
| Loading repo | skeleton rows; no stale data from previous repo |
| Ready | normal |
| Operation in progress | amber/red banner; continue/abort; disable conflicting actions |
| Offline | status pill; local actions allowed; network actions disabled |
| Auth required | banner with Sign in / Use SSH |
| Fatal repo error | error panel with Retry/Close; no partial actions |

### 0.3 Global keyboard

`⌘P` command palette, `⌘O` open repo, `⌘W` close tab, `⌘1…⌘9` tab, `⌘⇧[`/`]` prev/next tab, `⌘F` search, `⌘⇧F` filter history, `⌘⇧P` push, `⌘⇧K` commit, `⌘↵` primary action, `Esc` close sheet/palette. All rows reachable with arrow keys; focus ring visible.

### 0.4 Global rules

- All destructive actions: confirmation with object name and consequence; no “Don’t ask again” for force/discard.
- Every icon-only button has tooltip + accessibility label.
- Loading/empty/error/disabled are designed, not incidental.
- No blocking modal for background fetch/push; use non-modal progress + status.
- Dark mode uses system appearance; no hard-coded light colors.
- All user-visible strings are localization keys; English default, Simplified Chinese complete.
- Numbers/dates use locale; commit hashes/SHAs monospace.

---

## 1. S1 — Repository + History + Diff

**Boards**: 01 Welcome, 02 empty, 03 Repository manager, 04 History & diff, 05 filters, 06 multi-commit, 18 files at commit  
**R-IDs**: R-REPO-01…09, R-HIST-01…05, R-HIST-08  
**Use cases**: UC-01, UC-02

### 1.1 Regions

- **Repo tabs** in titlebar: repo name, dirty dot, close; overflow menu.
- **Sidebar**: Branches, Remotes, Tags, Stashes, Changelists; counts; context menus.
- **History table**: graph, message, refs, author, relative date; 28–30 px rows; virtualized.
- **Commit detail**: metadata header, file tree, diff; split/unified; fold; whitespace; word highlight.
- **Right side** (optional): commit list/filters; collapsed on narrow width.
- **Status bar**: branch, ahead/behind, operation, fetch state.

### 1.2 State matrix

| State | History | Detail | Enabled |
|---|---|---|---|
| Loading | skeleton 12 rows | skeleton | tabs, cancel load |
| Empty repo | “No commits yet” + init hint | empty state | open, settings |
| Ready | commits | selected diff | all read actions |
| Filtered empty | “No commits match” + Clear | empty | clear filters |
| Loading more | footer spinner | keep current | scroll; cancel |
| Diff error | list intact | error + Retry | reselect |
| Huge diff | list intact | first chunk + Load more | fold/scroll |
| Binary | list intact | “Binary file” + size | open externally |

### 1.3 Actions

- Select commit/file; multi-select; compare; copy SHA; open on remote; files at commit; filter; clear filter.
- Context menu: Copy SHA, Copy message, Open on Remote, Compare with current, Cherry-pick/Revert (confirm; write path), Create branch/tag here.
- No write action is enabled during merge/rebase unless the flow allows it.

### 1.4 NSH controls

`NSOutlineView` (sidebar/files), `NSTableView` (commits), `NSTextView`/custom diff view (diff), `NSSearchField` (filter), `NSSegmentedControl` (split/unified), `NSMenu` (context), `NSProgressIndicator` (load).

### 1.5 Keyboard / accessibility

Up/Down selects commits; Left/Right folds/expands files; `Space` preview; `⌘C` copies selection; `⌘F` filter. Graph is decorative; message/author/date are text. Diff lines expose line numbers and +/-; color is not the only signal.

### 1.6 Acceptance

- 10k commits: first page renders under 500 ms; scrolling smooth; filter cancels previous load.
- Selecting a commit shows exactly `git show --stat` file counts.
- Empty/filtered-empty/error states are reachable and testable.

---

## 2. S2 — Changes + Commit + Hooks

**Boards**: 10 Staging & commit, 11 Changelists  
**R-IDs**: R-STAGE-01…08, R-HOOK-01…08  
**Use cases**: UC-03, UC-04, UC-05, UC-10

### 2.1 Regions

- **Changes header**: staged/unstaged counts, stage all/unstage all, tree/list toggle.
- **Change list**: Staged section, Unstaged section; directories; file status; hunk/line controls.
- **Diff pane**: hunk/line selection with stage/unstage actions.
- **Commit composer** (inspector):
  - target branch; staged count;
  - summary + description; amend/sign-off/push-after;
  - changelist hint (“filters; staging commits”);
  - **Hooks panel**;
  - primary commit button; hint.
- **File context menu**: Revert, Stage/Unstage, Open, Reveal, Copy path, Move to changelist, Discard (confirm).

### 2.2 Commit composer states

| State | Composer | Hooks panel | Primary button |
|---|---|---|---|
| No staged | disabled message | hidden/“No hooks run” | disabled |
| Ready | editable | collapsed “Hooks will run” | enabled |
| Staging | busy | — | disabled |
| Hooks running | read-only | spinner + step list + streaming log | disabled |
| Hook failed | editable | red status, log, Retry, bypass | disabled |
| Committing | read-only | last status | “Committing…” |
| Committed | success | collapsed | disabled until new staged |
| Commit error | editable | last log | Retry |

### 2.3 Hooks panel

- Header: `Hooks` + status + duration + `View full log`.
- Steps: `pre-commit`, `prepare-commit-msg`, `commit-msg`, `post-commit`; passed/failed/skipped/running/cancelled.
- Log: monospace 12 px, scroll, max 1 MB live; full log in temp file.
- Actions: **Retry hooks**, **Commit without hooks** (danger, confirm, `--no-verify`), **Cancel** while running.
- Post-commit failure: yellow warning, commit still successful; no rollback.
- Hook error must include command, exit code, stderr excerpt; redact tokens/URLs.

### 2.4 State matrix

| State | List | Diff | Composer |
|---|---|---|---|
| Loading | skeleton | skeleton | disabled |
| No changes | empty “Working tree clean” | empty | disabled |
| Unstaged only | files | diff | “Stage something first” |
| Staged | sections | diff | enabled |
| Partial stage | updated counts | selected hunk | enabled |
| Hook failed | unchanged | unchanged | blocked + log |
| Operation in progress | banner | diff | commit allowed only for merge/rebase continuation |

### 2.5 NSH controls

`NSOutlineView`/`NSTableView` (changes), `NSTextView` (diff/commit message/log), `NSSegmentedControl` (tree/list, split/unified), `NSButton` (stage/commit/retry), `NSScrollView`, `NSProgressIndicator`, `NSAlert` (bypass/discard).

### 2.6 Acceptance

- Stage one hunk → only that hunk in staged diff; commit contains only it.
- Failing pre-commit → primary disabled; log visible; HEAD unchanged.
- Retry → commit succeeds; log status updates.
- `--no-verify` → confirmation + audit.
- No staged → primary disabled; no accidental empty commit.

---

## 3. S3 — Sync (Fetch / Pull / Push)

**Boards**: 15 Sync & auth, 28 Fetch, 29 Pull  
**R-IDs**: R-SYNC-01…07  
**Use cases**: UC-06, UC-07

### 3.1 Regions

- Sheet with tabs Fetch / Pull / Push; close; title.
- Target row: local branch ↔ remote branch; ahead/behind.
- Options per tab; auth status; primary action + Cancel.
- Right activity: progress, log, incoming/outgoing commits.
- Background state: if sheet closes, status bar shows operation + reopen.

### 3.2 State matrix

| State | Sheet | Primary | Notes |
|---|---|---|---|
| Idle | options | enabled | |
| Running | progress/log | disabled + Cancel | can background |
| Auth required | auth banner | Sign in / Use SSH | no secret prompt in log |
| Auth failed | error | Retry / Sign in | no loop |
| Non-fast-forward | warning | Pull/Rebase/Force (confirm) | never auto-force |
| Network error | error | Retry | offline mode |
| Success | result | Close | refresh refs |
| Hook (pre-push) failed | log | Retry | push blocked |

### 3.3 Rules

- Fetch never touches worktree.
- Pull modes: Merge / Rebase / FF-only; autostash clear; conflict → S4.
- Push: never auto-force; force-with-lease requires typed confirmation on protected branch.
- Cancel kills process group; no orphan; no partial ref update hidden.
- Token never appears in argv/log/UI; auth uses credential helper/Keychain/SSH agent.

### 3.4 NSH controls

Sheet (`NSWindow` sheet/NSH), `NSSegmentedControl` (tabs), `NSButton`, `NSProgressIndicator`, `NSTextView` (log), `NSPopUpButton` (remote/branch), `NSAlert` (force).

### 3.5 Acceptance

- Fetch leaves worktree unchanged.
- Pull conflict opens S4 with correct operation.
- Push non-FF offers Pull, not force.
- Bad auth shows Sign in and no token leak.

---

## 4. S4 — Merge / Rebase Conflict

**Boards**: 14 Merge conflict  
**R-IDs**: R-MERGE-01…07  
**Use cases**: UC-08, UC-09

### 4.1 Regions

- Operation banner: merging/rebase/cherry-pick/revert; source → target; step; files/conflicts.
- File list: path, status, unresolved/resolved; next/prev.
- Three-way view: Base strip + Ours / Result / Theirs; Result editable.
- Toolbar: Accept Ours/Theirs/Both, Mark Resolved, Undo; split/unified? (code editor).
- Footer: unresolved count; Continue disabled until zero; Abort; Skip (rebase only).

### 4.2 State matrix

| State | Editor | Continue | Abort/Skip |
|---|---|---|---|
| Loading stages | disabled | disabled | enabled |
| Unresolved | editable | disabled | enabled |
| All resolved | editable | enabled | enabled |
| Saving | disabled | disabled | disabled |
| Save error | content kept | disabled | enabled |
| Continue running | read-only | spinner | disabled |
| Hook failed | read-only + hook log | Retry | enabled |
| Aborted | closed | — | — |

### 4.3 Rules

- Base/Ours/Theirs are read-only; Result is editable.
- Accept actions insert text; they do not immediately write; Save/Mark Resolved persists.
- Mark Resolved writes result then `git add`.
- Continue only when all conflicts resolved; re-read status before continue.
- Abort confirms; restores pre-operation state; no data loss.
- Merge/rebase commit runs hooks; hook failure must not lose Result.

### 4.4 NSH controls

`NSSplitView` (3 columns), `NSTextView` (read-only + editable), `NSButton`, `NSSegmentedControl`, `NSProgressIndicator`, `NSAlert` (abort), `NSMenu` (Accept).

### 4.5 Acceptance

- Result `B-merged` → merge commit content correct.
- Continue disabled with unresolved files.
- Abort restores HEAD/worktree and removes operation dir.
- Hook failure keeps result and shows log.

---

## 5. S5 — Settings

**Boards**: 23 General, 24 Git, 25 Shortcuts, 27 Credentials  
**R-IDs**: R-SET-01…08  
**Use case**: UC-13

### 5.1 Regions

- Preferences window with sidebar: Application (General, Shortcuts), Git (Git, Credentials), About.
- General: Appearance, Language, Date, File tree, On launch, Commit identity, Updates.
- Git: executable/PATH/scan/version.
- Shortcuts: grouped list, recorder, conflict, restore defaults.
- Credentials: GitHub/GitLab, credential helper, SSH keys, tools, network.
- Footer: Done; changes apply immediately unless marked restart.

### 5.2 State matrix

| State | UI |
|---|---|
| Loading | skeleton; controls disabled |
| Ready | values |
| Dirty | Save/Done enabled; “unsaved” marker |
| Saving | controls disabled; spinner |
| Validation error | inline error; focus field; no save |
| Git missing | banner + Install/Choose |
| Keychain denied | inline guidance; no plaintext fallback |
| Restart required | badge + explanation |

### 5.3 Rules

- Identity: local name/email is the only commit identity when “use global” off; credentials page must not override it.
- Secrets: Keychain only; never SQLite/log/UI beyond masked value.
- Shortcut conflicts block save or require Replace.
- Language/theme/date apply immediately; Git path may require restart.
- On launch: Show Welcome or Restore all repos; missing repos handled per UC-01.

### 5.4 NSH controls

`NSToolbar`/sidebar, `NSTextField`, `NSPopUpButton`, `NSSegmentedControl`/radio, `NSSwitch`/checkbox, `NSButton`, `NSTableView` (shortcuts), `NSAlert`.

### 5.5 Acceptance

- Identity change affects new commits; global off is respected.
- Bad Git path shows error; valid path rescans.
- Shortcut conflict cannot be saved silently.
- Keychain denial does not fall back to plaintext.

---

## 6. S6 — Refs manager (v0.1.1)

**Boards**: 09 Refs manager, 30 Tags/Remotes/Stashes  
**R-IDs**: R-REF-01…08, R-HIST-07  
**Use case**: UC-02/UC-06 (read), ref operations

### 6.1 Regions

- Tabs: Branches / Tags / Remotes / Stashes.
- Table: name, upstream, ahead/behind, last commit, date, actions.
- Inspector: details + actions (checkout, rename, delete, set upstream, compare, merge, rebase; tag push/delete; remote edit/fetch; stash apply/pop/drop).
- Search/filter; New button.

### 6.2 State matrix

| State | Table | Inspector |
|---|---|---|
| Loading | skeleton | disabled |
| Empty | empty + New | hidden |
| Ready | rows | details |
| Operation | banner | actions disabled |
| Error | error + Retry | last selection |

### 6.3 Rules

- Delete/reset/force actions confirm; protected branch warning.
- Branch checkout with dirty worktree: stash/ carry/ cancel; never silent.
- Tag/remote/stash operations refresh refs/history after.
- Refs manager is read/write only when no conflicting operation; otherwise banner.

### 6.4 Acceptance

- Branch checkout updates HEAD/sidebar/history.
- Delete branch refuses current branch; confirmation.
- Tag/remote/stash tables match `git` state.
- All writes refresh UI.

---

## 7. Cross-screen requirements

### 7.1 State coverage

Every screen must define: loading, empty, ready, busy, disabled, error, permission/auth, operation-in-progress. Skeleton must not shift layout; errors must include a next action and a Details disclosure.

### 7.2 Focus / keyboard

- Tab order follows visual order; no focus traps except modal sheets.
- `Esc` closes sheet/palette, not destructive confirmation.
- Focus ring visible on dark/light; selected row + focus distinguish.
- Diff/editor supports standard text navigation/selection/copy.

### 7.3 Accessibility

- Every icon-only control has an accessibility label and tooltip.
- Status is not color-only: text/icon + color.
- Contrast ≥ 4.5:1 for text; 3:1 for large text.
- VoiceOver announces operation state, errors, hook result.
- Reduced motion: disable spinners/slide animations.

### 7.4 Localization

- All strings from keys; no concatenated sentences.
- English and Simplified Chinese complete; dates per locale; no hard-coded “AM/PM” assumptions.
- Layout tolerates +30% string length; buttons wrap or truncate with tooltip.

### 7.5 Error presentation

- Inline for field errors; banner for repo/operation; sheet for blocking auth/conflict; status bar for background.
- Error text: what happened, why, what to do; Details with redacted command/stderr/exit code.
- Never show raw token or full URL with credentials.

---

## 8. Board → screen map

| Screen | Boards |
|---|---|
| S1 | 01, 02, 03, 04, 05, 06, 18 |
| S2 | 10, 11 |
| S3 | 15, 28, 29 |
| S4 | 14 |
| S5 | 23, 24, 25, 26, 27 |
| S6 | 09, 30 |
| Shell/navigation | 20, 21, 22, 16 |
| Deferred | 07, 08, 17, 19 (v0.1.1), 12/13 (v0.2) |
