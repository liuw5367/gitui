# gitui — documentation map

**Status**: v1, 2026-11-14  
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
7. [DESIGN-HANDOFF.md](DESIGN-HANDOFF.md) — canvas → NSH/AppKit mapping, tokens, icons.
8. [AGENT-THREAT-MODEL.md](AGENT-THREAT-MODEL.md) — v0.2 Agent security boundary.
9. [adr/](adr/README.md) — seven accepted architecture decisions.

## Source-of-truth rules

- **SPEC.md owns scope and requirement IDs (R-*).** Do not fork requirements into other docs.
- **USE-CASES.md owns behaviour.** Every use case cites R-IDs.
- **UI-SPEC.md owns screen states.** No screen behaviour that is not in a use case.
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
| S-6 NSH Rust editor/bridge | **Pending, blocking** for M2/M4 |
| v0.1 implementation | **P1 done**: welcome page, open repository, workspace shell, read-only history/diff (see `plans.local/2026-11-14-design-parity-build.md`) |
| P1 verification | `cargo test --workspace` 通过（几何断言比对设计 CSS）；截图 `docs/verify/p1-welcome.png` |

## Document ownership

| Doc | Owner | Update trigger |
|---|---|---|
| SPEC | Product + tech lead | scope/decision change |
| USE-CASES | Product | new flow or acceptance change |
| UI-SPEC | Design + client | screen/state change |
| ARCHITECTURE | Tech lead | module/API/threading change |
| TEST-PLAN | QA + tech lead | new fixture/test/release gate |
| DESIGN-HANDOFF | Design | token/control/board change |
| AGENT-THREAT-MODEL | Security + tech lead | Agent capability change |
| ADR | Tech lead | irreversible or cross-cutting decision |
