# gitui — documentation map

**Status**: v1, 2026-11-15  
**Product spec**: [../SPEC.md](../SPEC.md)  
**Stack**: **NSH (Native Shell) + Rust only** — no TSX, no JS runtime, no HTML UI. NSH 0.5.0 的 Rust API + AppKit 后端。
**Positioning**: 跨平台的原生 Git 客户端；当前接入 macOS，其他平台在 macOS 版本完成后接入。默认界面语言为英文。

## Reading order

1. [SPEC.md](../SPEC.md) — what/why/scope, decisions, release map.
2. [SPIKE-REPORT.md](SPIKE-REPORT.md) — what was actually proven; what is still blocking.
3. [USE-CASES.md](USE-CASES.md) — executable flows, failure paths, acceptance.
4. [UI-SPEC.md](UI-SPEC.md) — six core screens, state matrices, NSH controls.
5. [ARCHITECTURE.md](ARCHITECTURE.md) — Rust modules, services, sequences, threading.
6. [TEST-PLAN.md](TEST-PLAN.md) — fixtures, layered tests, R-ID traceability.
7. [2026-11-14-gitui-ui-operation-test.md](2026-11-14-gitui-ui-operation-test.md) — per-board interaction acceptance, click paths, dialogs and implementation status.
8. [2026-11-15-gitui-feature-state-model.md](2026-11-15-gitui-feature-state-model.md) — Git capability matrix, support levels, page states and fixture recipes.
9. [DESIGN-HANDOFF.md](DESIGN-HANDOFF.md) — canvas → NSH/AppKit mapping, native scale, tokens, icons, deviation list.
10. [UI-KIT.md](UI-KIT.md) — the app's `ui` component layer: inventory, signatures, selection rules.
11. [BOARD-PARITY.md](BOARD-PARITY.md) — per-board control inventory vs implementation, and the components still missing.
12. [AGENT-THREAT-MODEL.md](AGENT-THREAT-MODEL.md) — v0.2 Agent security boundary.
13. [adr/](adr/README.md) — seven accepted architecture decisions.

## Source-of-truth rules

- **SPEC.md owns scope and requirement IDs (R-*).** Do not fork requirements into other docs.
- **USE-CASES.md owns behaviour.** Every use case cites R-IDs.
- **UI-SPEC.md owns screen states.** No screen behaviour that is not in a use case.
- **UI-KIT.md owns the component layer.** Pages compose `ui`; they do not style controls themselves.
- **ARCHITECTURE.md owns module boundaries and data flow.**
- **TEST-PLAN.md owns verification.** Every R-ID must map to at least one test or manual check.
- A decision changes only through an ADR or a SPEC decision row; update all affected docs in one change.
- The 30 canvas boards are visual references, not production code. Production is Rust + NSH/AppKit.
- The two dated acceptance documents are derived test views. They do not create or override R-IDs; SPEC, USE-CASES and UI-SPEC remain authoritative.

## Current status

| Area | Status |
|---|---|
| Design canvas | 30 boards, accepted; no more pages planned |
| CLI spikes S-1/S-2/S-3 | **Pass** on macOS 15.7.7 / Git 2.56.0 |
| S-4 performance | **Partial** (20k commits micro-benchmark); 100k/1GB not yet run |
| S-5 auth/network | **Pending** (needs remote/token/SSH harness) |
| S-6 NSH Rust editor/bridge | **Done**: app is Rust-only on NSH 0.5.0, no TSX/QuickJS |
| v0.1 implementation | **All planned phases landed**: welcome window, workspace, history/diff, staging & commit, refs, reflog, conflicts, sync, agent, settings, shortcuts, command palette |
| Verification | `cargo test` 179 项通过（domain 20 / git 59 / store 13 / agent 12 / apps 75）；`nsh check` 与 `cargo check --all-targets` 零警告；几何与主题测试直接比对设计 CSS |
| Interaction acceptance | `docs/2026-11-14-gitui-ui-operation-test.md` 第 9 节逐板记录状态（PASS / PARTIAL / BLOCKED / N/A-DESIGN）；`docs/2026-11-15-gitui-feature-state-model.md` 记录 Git 支持矩阵、页面状态和 fixture 构造 |
| UI component layer | `apps/client/src/ui/` 20 个组件文件、原生刻度（DESIGN-HANDOFF §2.4）、`tools/check-ui.sh` 门禁、⌘⇧D 组件画廊。P1–P4 与 P7 的界面迁移完成，**14 个文件登记**在 `tools/ui-migrated.txt`（字号字面量、裸按压、手写 mono、`nsh_base` 直引四项归零，由 `guard_tests` 强制） |
| Known gaps | 逐板截图取证已可用（`tools/shots.sh` + `GITUI_MODE`/`GITUI_SYNC`/`GITUI_FORM`/`GITUI_PALETTE`/`GITUI_DIRTY`，按窗口 id 截图）；剩余：板 16 横幅未接 `ui::feedback::banner`、板 19 主区 blame 缺入口（主区只有 4 个 Mode）、板 14 `ui::split` 未接入、`widgets.rs` 与 `ui` 两套尺寸模型并存（见 DESIGN-HANDOFF §12）；凭据真实流程（按决定延后） |

## Document ownership

| Doc | Owner | Update trigger |
|---|---|---|
| SPEC | Product + tech lead | scope/decision change |
| USE-CASES | Product | new flow or acceptance change |
| UI-SPEC | Design + client | screen/state change |
| ARCHITECTURE | Tech lead | module/API/threading change |
| TEST-PLAN | QA + tech lead | new fixture/test/release gate |
| 2026-11-14 interaction test | QA + design | board/control or implementation status change |
| 2026-11-15 feature/Git support | Product + QA | scope, support level or state model change |
| DESIGN-HANDOFF | Design | token/control/board change |
| UI-KIT | Client | component added, changed or retired |
| BOARD-PARITY | Client | board migrated, component landed, or gap closed |
| AGENT-THREAT-MODEL | Security + tech lead | Agent capability change |
| ADR | Tech lead | irreversible or cross-cutting decision |
