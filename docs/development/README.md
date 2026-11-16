# development — 开发规范

本目录回答两件事：新增或修改一个页面时按什么结构写，以及怎样算完成。视觉语言在
[../design/](../design/README.md)，单页版式与按钮在 [../pages/](../pages/README.md)，
这里不重复。

| 文档 | 回答什么 | 什么时候读 |
|---|---|---|
| [page-development.md](page-development.md) | 分层、状态、响应式、UI、文案与能力边界 | 写页面前、评审页面改动 |
| [verification.md](verification.md) | 测试、检查、截图与完成定义 | 提交前、验收页面 |

## 与其它文档的分工

- 视觉语言、token、组件层与已决定差异：[../design/](../design/README.md)
- 单页版式、状态轴与按钮表：[../pages/](../pages/README.md)
- 项目能力与已知限制：[../../README.md](../../README.md)

## 截图与本地产物

页面截图写入 `files.local/shots/`。该目录已被 `.gitignore` 排除，不提交到仓库。
截图只作为行为证据，不作为像素基线。

## 变更

修改本目录的规则时，同步 [AGENTS.md](../../AGENTS.md) 的文档入口与验证段，以及
[docs/README.md](../README.md)。能机械检查的约束优先写进 `tools/check-ui.sh` 与
`apps/client/src/ui/mod.rs` 的 `guard_tests`。
