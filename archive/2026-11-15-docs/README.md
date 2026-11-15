# 文档归档 2026-11-15

**原因**: 重新开始设计文档；按用户决定，把非 local 目录下的文档全部归档。  
**归档位置**: `archive/2026-11-15-docs/`  
**状态**: 冻结，不作为当前需求源或当前实现状态。  
**新文档目录**: `docs/`（当前为空，重新开始）。

## 归档范围

路径保持原相对位置：

- `SPEC.md`
- `docs/` 下的全部文档，包括 `USE-CASES.md`、`UI-SPEC.md`、`DESIGN-HANDOFF.md`、`ARCHITECTURE.md`、`TEST-PLAN.md`、`UI-KIT.md`、`BOARD-PARITY.md`、`SPIKE-REPORT.md`、`AGENT-THREAT-MODEL.md`、两份 dated 验收文档，以及 `docs/adr/` 的 7 条 ADR 和索引。
- `apps/client/README.md`
- `designs/graft-git/README.md`

## 未归档

以下目录属于本地工作区，未被移动到 archive：

- `plans.local/`
- `files.local/`

其中 `files.local/` 仍有两份旧文档副本；`plans.local/` 仍有旧计划和本轮文档收束草案。它们不是当前设计文档的入口。

## 使用规则

1. 新设计文档只写入 `docs/`。
2. 本目录中的旧文档不再作为当前状态依据；需要历史决策时再查。
3. 如需恢复某个文件，从本目录移回原路径，不要直接改写归档内容。
4. 本目录不包含新的产品决策；归档不等于批准旧结论。

## 验证

- 非 local 目录下已没有 `.md` 文件。
- `docs/` 为空。
- 归档文件的相对路径与原文一致。
- 没有修改 `apps/client`、`crates/`、`designs/` 的产品代码；移动的是文档。
- 变更未提交。
