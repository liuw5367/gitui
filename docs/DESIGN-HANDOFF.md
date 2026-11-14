# DESIGN-HANDOFF — canvas → NSH/AppKit

**Status**: v1, 2026-11-14  
**Stack**: NSH + Rust only; UI built with NSH's Rust API and AppKit-backed controls.  
**No** TSX, JS, HTML, or CSS in production. The canvas is a visual/state reference.  
**Source canvas**: `designs/graft-git/project/` (30 boards)  
**UI spec**: [UI-SPEC.md](UI-SPEC.md)

---

## 1. Principles

- **Native first**: AppKit controls/behaviour, system font, SF Symbols, standard menus, sheets, focus, VoiceOver.
- **Dense but legible**: Git client density is intentional; do not inflate rows to “web app” sizes.
- **Hierarchy through weight/colour**, not uppercase micro-labels or boxes everywhere.
- **One accent**: system blue; no second primary blue; destructive is a separate semantic colour.
- **State is design**: loading/empty/error/disabled/hook failure/operation are part of the screen, not afterthoughts.
- **Dark and light are equal**: no hard-coded light-only colours; test both.

---

## 2. Tokens

### 2.1 Colour

| Token | Light | Dark |
|---|---|---|
| Accent | `#007AFF` | `#0A84FF` |
| Primary button | accent | accent |
| Text | `#1D1D1F` | `#F5F5F7` |
| Secondary | `#5F636B` | `#A1A1A6` |
| Tertiary | `#86868B` | `#8E8E93` |
| Window/content | `#FFFFFF` | `#1E1E20` |
| Sidebar | `#F4F4F6` (material) | `#232325` |
| Panel | `#F7F7F8` | `#262628` |
| Line | `rgba(0,0,0,.10)` | `rgba(255,255,255,.10)` |
| Add | `#1F8A4C` / `#30D158` | same semantic |
| Delete | `#C7362F` / `#FF453A` | same semantic |
| Warning | `#B26A00` / `#FFD60A` | same semantic |
| Danger | `#C7362F` / `#FF453A` | same semantic |

- Accent is the **only** primary blue. `--primary-bg` = accent.
- Selection: source lists use accent fill + white text; tables use accent-soft + text for non-focused selection.
- Status is never colour-only: icon/text + colour.

### 2.2 Type

| Role | Size | Weight | Notes |
|---|---|---|---|
| Window title | 13 | semibold | titlebar |
| Body / labels | 13 | regular | native default |
| Secondary | 11 | regular | help/description |
| Metadata | 10.5 | regular | dates/counts |
| Table header | 10.5–11 | medium | **sentence case**, no uppercase/letter-spacing |
| Section header | 11–12 | semibold | sentence case |
| Code/diff | 11–12 | regular | SF Mono; 10.5 minimum |
| Button | 12–13 | medium | 22/24 px height |
| Badge | 9.5–10.5 | medium | no all-caps except known acronyms |

- Font: system SF Pro Text; code SF Mono. No custom display font.
- Avoid 8–9 px app labels; minimum 10.5 for metadata.
- Line height: 1.3–1.5; diff 1.5.

### 2.3 Spacing / shape

| Token | Value |
|---|---|
| Base unit | 4 |
| Gaps | 4 / 6 / 8 / 12 / 16 |
| Row height (tables/lists) | 28–30 |
| Sidebar row | 28 |
| Button height | 22 small / 24 regular / 28 large |
| Field height | 26 |
| Segmented height | 23–24 |
| Menu item | 24 |
| Toolbar | 52 |
| Titlebar | 38 (44 with tabs) |
| Radius | 6 controls / 8 panels / 10 window |
| Hairline | 1 px (0.5 pt on Retina) |

- No gratuitous pills/cards; use separators and spacing.
- Sheets: top-only rounded corners, attached to window; not free-floating shadow cards.
- Menus: NSMenu; 6–7 px radius; 24 px items; vibrancy.

---

## 3. Control mapping

| Canvas | Production |
|---|---|
| `.btn` / `.btn.primary` | `NSButton` (bezel styles); accent for primary; no gloss |
| `.btn.small` | `NSButton` small/regular 22 |
| `.chipbtn` | `NSButton` inline / borderless with background |
| `.seg` | `NSSegmentedControl` |
| `.field` | `NSSearchField` / `NSTextField` |
| `.checkrow` | `NSButton` checkbox / `NSSwitch` where appropriate |
| `.radio-pills` | `NSSegmentedControl` or radio group |
| `.side-row` | `NSOutlineView` row, source-list style |
| `.commit` / `.refs-row` / `.fhist-row` | `NSTableView` rows |
| `.diff` | `NSTextView` (read-only) in `NSScrollView` |
| `.commit-input` / description | `NSTextView` / `NSTextField` |
| `.hook-log` | `NSTextView` read-only, monospace |
| `.menu` / `.mi` | `NSMenu` / `NSMenuItem` |
| `.sync-sheet` / merge sheet | NSH sheet / `NSAlert` where appropriate |
| `.titlebar` tabs | native window tabs or NSH tab strip (product choice) |
| `.toolbar` | `NSToolbar` / NSH toolbar |
| `.statusbar` | NSH status view |
| traffic lights | system standard window buttons |

- Do not recreate AppKit controls with custom drawing unless the design requires a diff/graph/editor.
- Use SF Symbols for icons; inline SVG is canvas-only.

---

## 4. Icons

- Use SF Symbols with consistent weight (regular/medium) and optical size.
- Suggested symbols:
  - repo/branch: `arrow.triangle.branch`
  - commit: `point.3.connected.trianglepath.dotted`
  - history: `clock.arrow.circlepath`
  - changes: `plus.forwardslash.minus`
  - sync: `arrow.triangle.2.circlepath`
  - fetch: `arrow.down.circle`
  - pull: `arrow.down.to.line`
  - push: `arrow.up.to.line`
  - merge: `arrow.triangle.merge`
  - conflict: `exclamationmark.triangle`
  - hook: `terminal`
  - tag: `tag`
  - remote: `network`
  - stash: `archivebox`
  - settings: `gearshape`
  - agent: `sparkles`
  - search: `magnifyingglass`
  - close: `xmark`
  - more: `ellipsis`
- Icon sizes: 13–16 in toolbars/rows; 12 in buttons; 16–18 toolbar.
- Every icon-only button: accessibility label + tooltip.

---

## 5. Screen handoff

### S1 Repository + History

- Sidebar source list (vibrancy), 28 px rows; selected accent fill.
- Commit table 28–30 px; graph column fixed; message truncates with tooltip.
- Diff: 11–12 px SF Mono; line numbers tertiary; add/delete backgrounds subtle; no rainbow.
- Filter bar: search field + branch/date/author chips; “Clear”.
- Empty/loading/error states per UI-SPEC.

### S2 Changes + Commit + Hooks

- Staged/Unstaged sections with counts; checkboxes/rows; hunk action affordance.
- Composer: 300–340 px inspector; summary 13 semibold; description 12; options 12.
- Hook panel: red-tinted but restrained; status + steps + log; Retry primary-ish; bypass danger.
- Commit button 28–30 px; disabled state clear; no “or Commit & Push” when hooks failed.

### S3 Sync

- Sheet 800 px wide; tabs Fetch/Pull/Push; target row; options; auth; primary + Cancel.
- Progress/log right side; background state after close.
- Force options behind warning; no default force.

### S4 Merge

- Base strip + Ours / Result / Theirs; Result editable; line numbers; conflict markers highlighted.
- Continue disabled until 0 conflicts; Abort always available; Skip only rebase.
- Hook failure panel can appear without losing Result.

### S5 Settings

- Preferences window; sidebar: Application (General, Shortcuts), Git (Git, Credentials), About.
- Labels 13 / help 11; controls 22–28; Keychain notes; no secrets displayed.
- Identity local vs global; Credentials does not override user.name/email.

### S6 Refs

- Tabs; table 28–30; inspector 300–340; actions confirm destructive.
- Branch/tag/remote/stash details; no mixed tables without headers.

---

## 6. Dark mode

- Use semantic tokens; no hard-coded `#fff`/`#000` in views.
- Diff add/delete have sufficient contrast in dark; line numbers tertiary.
- Accent #0A84FF; selected source list white text.
- Menus/sheets use material; borders visible but subtle.
- Test: hooks red, conflict red, warnings yellow, disabled controls, focus rings.

---

## 7. Localization

- English default; Simplified Chinese complete.
- Keys for every string; no sentence concatenation.
- Buttons/rows tolerate +30% length; truncate with tooltip rather than clip.
- Dates: relative/absolute/system/custom; no hard-coded “today” strings.
- Plurals: “1 file / 2 files” via plural rules; Chinese no plural suffix.
- Keyboard shortcuts use localized key names but physical keys.

---

## 8. Accessibility

- Every control keyboard reachable; focus ring visible; tab order visual.
- Tables/outlines expose rows, selection, sort; diff exposes line numbers/content.
- Hook status announced; errors have accessible descriptions; not colour-only.
- Contrast ≥ 4.5:1 text; 3:1 large; check both themes.
- Reduced motion: spinners/progress can be static; no essential animation.
- VoiceOver labels for tabs, icon buttons, branch rows, status pills.

---

## 9. Motion

- 120–180 ms for hover/selection; no bounce.
- Progress: indeterminate spinner only when no percentage; otherwise bar + text.
- Sheets slide from top; no full-screen fades.
- No animation for diff rendering/hook logs that could hide state.

---

## 10. Board map

| Group | Boards |
|---|---|
| Start | 01 Welcome, 02 empty, 03 Repo manager |
| Review | 04 History, 05 filters, 06 multi-commit, 07 compare, 08 commit info, 09 refs |
| Worktree | 10 Changes/commit, 11 changelists, 12 Agent, 13 permissions |
| Operations | 14 merge, 15 sync, 16 states, 17 reflog |
| Files | 18 files at commit, 19 blame/history |
| Navigation | 20 palette, 21 dark/context, 22 menus/dialogs |
| Settings | 23 general, 24 Git, 25 shortcuts, 26 about, 27 credentials |
| Details | 28 fetch, 29 pull, 30 refs other |

- 22 is a component sheet; 16 is a state gallery; 28/29 are real sync sheets; 30 is a refs screen.
- No new boards planned; implement screens, not more pictures.

---

## 11. Handoff checklist

- [ ] Every screen has loading/empty/error/disabled states.
- [ ] Every action maps to a Rust service command and a use case.
- [ ] Buttons 22/24; rows 28–30; no 8 px labels.
- [ ] All icons SF Symbols; labels/tooltips.
- [ ] Light/dark checked; contrast/focus checked.
- [ ] English/zh-Hans strings; +30% length checked.
- [ ] Destructive confirmations; no silent force/discard.
- [ ] Hook panel shows status, log, retry, bypass confirmation.
- [ ] No TSX/JS/HTML in production.
