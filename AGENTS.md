# AGENTS.md

会话开始先按全局规范读取共享规则入口；本文件只补充本仓库的事实、边界与验证方式，细节以 README 为准。

## 文档入口

- [README.md](README.md)：项目定位、当前能力、环境、运行、验证与已知限制。
- [apps/client/README.md](apps/client/README.md)：客户端结构、`ui` 层规则、存储、测试与改名点。
- crate README：[domain](crates/domain/README.md)、[git](crates/git/README.md)、[store](crates/store/README.md)、[agent](crates/agent/README.md)。
- 文档在 [docs/](docs/README.md)，按主题分目录：[docs/design/](docs/design/README.md)（视觉语言、token 契约、组件层、偏离清单）与 [docs/pages/](docs/pages/README.md)（逐页版式、状态与按钮）；[archive/2026-11-15-docs/](archive/2026-11-15-docs/) 已冻结，只作历史参考，不代表当前实现。

## 项目事实

- 跨平台原生 Git 客户端，Rust + NSH 0.5.0；当前只实现 macOS 端（AppKit），其他平台不展开。
- 仓库没有根 Cargo workspace。`apps/client` 用 path 依赖引用四个 crate，每个 crate 独立构建与测试；不要合并成根 workspace，除非用户明确要求。
- 产品名尚未最终确定。名称与标识只在 `apps/client/src/brand.rs`、`apps/client/nsh.json` 与 `apps/client/scripts/release-macos.sh` 定义；不要在代码或文档里另造名称。
- 设计稿在 `designs/graft-git`，由 design 单独维护。页面以 NSH 原生组件为基础，尽量采用平台原生样式；生产实现只引用设计源，不引入 TSX、JS 运行时、HTML 或 WebView。
- `fixtures/gen-repo.sh` 默认把测试仓库与 bare 远程写入 `.tmp/`；该目录已被 `.gitignore` 排除，不要提交生成物。

## 修改约定

- 先读对应 README；稳定的行为、契约或配置变化要同步更新最近的文档。
- UI 改动遵守 `src/ui` 层规则：已迁移的页面文件不直接引用 `nsh_base`，不写字号字面量或十六进制颜色；字号取 `ui` 刻度，颜色取 `design`。
- 不修改 `designs/` 与 `archive/`，除非用户明确要求。
- 代码文件头注释可能滞后：`apps/client/src/lib.rs`、`apps/client/src/shell/mod.rs` 仍描述早期阶段；`apps/client/src/design/metrics.rs` 仍引用 `tests/geometry.rs` 与已归档的计划。以代码行为和当前 README 为准。
- 已知限制以根 README 为准：凭据流程、交互式 rebase、Git 路径设置、diff 的 fold/whitespace、Agent 自动化档位等不要当成已实现能力。

## 验证

```sh
(cd crates/domain && cargo test)
(cd crates/git && cargo test)
(cd crates/store && cargo test)
(cd crates/agent && cargo test)
(cd apps/client && cargo test)
(cd apps/client && nsh check)
tools/check-ui.sh
python3 tools/design/tokens.py
```

- 测试需要 PATH 中有 `git`；`crates/git` 还用本机 bare 远程验证 fetch/pull/push，不依赖网络。
- 改 UI 必须运行 `tools/check-ui.sh`；改 `fixtures/gen-repo.sh` 后至少运行 `bash -n`，并在 `.tmp/` 下实际生成一次。
- 需要人工查看界面时才运行 `cd apps/client && nsh run`；不要为了验证文档或测试自动启动应用。可用 `APP_HOME=/tmp/app-home` 隔离应用数据。

## 提交

- 未获得明确授权不提交、不推送。提交信息使用 Conventional Commits，类型用英文，标题与正文用中文。
