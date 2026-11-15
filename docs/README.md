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
7. [DESIGN-HANDOFF.md](DESIGN-HANDOFF.md) — canvas → NSH/AppKit mapping, native scale, tokens, icons, deviation list.
8. [UI-KIT.md](UI-KIT.md) — the app's `ui` component layer: inventory, API rules, states.
9. [AGENT-THREAT-MODEL.md](AGENT-THREAT-MODEL.md) — v0.2 Agent security boundary.
10. [adr/](adr/README.md) — seven accepted architecture decisions.

## Source-of-truth rules

- **SPEC.md owns scope and requirement IDs (R-*).** Do not fork requirements into other docs.
- **USE-CASES.md owns behaviour.** Every use case cites R-IDs.
- **UI-SPEC.md owns screen states.** No screen behaviour that is not in a use case.
- **UI-KIT.md owns the component layer.** Pages compose `ui`; they do not style controls themselves.
- **ARCHITECTURE.md owns module boundaries and data flow.**
- **TEST-PLAN.md owns verification.** Every R-ID must map to at least one test or manual check.
- A decision changes only through an ADR or a SPEC decision row; update all affected docs in one change.
- The 30 canvas boards are visual references, not production code. Production is Rust + NSH/AppKit.

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
| Interaction acceptance | `docs/2026-11-14-gitui-ui-operation-test.md` 第 9 节逐板记录状态（PASS / PARTIAL / BLOCKED / N/A-DESIGN）；样式结论按原生刻度口径修订中 |
| UI component layer | **P0 完成**：`apps/client/src/ui/` 15 个组件、原生刻度（DESIGN-HANDOFF §2.4）、`tools/check-ui.sh` 门禁、⌘⇧D 组件画廊；P1–P7 按 canvas 八组迁移 |
| Known gaps | 逐板截图取证（本环境无图形会话，`nsh run` 被 dev session 锁阻断）、表格/diff/命令面板尚未换成 `data_table`/`diff`/`command`（P2/P3/P6）、凭据真实流程（按决定延后） |

## Document ownership

| Doc | Owner | Update trigger |
|---|---|---|
| SPEC | Product + tech lead | scope/decision change |
| USE-CASES | Product | new flow or acceptance change |
| UI-SPEC | Design + client | screen/state change |
| ARCHITECTURE | Tech lead | module/API/threading change |
| TEST-PLAN | QA + tech lead | new fixture/test/release gate |
| DESIGN-HANDOFF | Design | token/control/board change |
| UI-KIT | Client | component added, changed or retired |
| AGENT-THREAT-MODEL | Security + tech lead | Agent capability change |
| ADR | Tech lead | irreversible or cross-cutting decision |
