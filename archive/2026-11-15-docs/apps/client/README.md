# 客户端

NSH 0.5.0 + Rust，无 TSX/JS/HTML 界面。定位是跨平台的原生 Git 客户端：同一个 crate
由 NSH 按目标平台构建，当前先接入 macOS（AppKit 后端），其他平台在 macOS 版本完成后再接。

## 运行

```sh
cd apps/client
nsh run                                   # 开发宿主（AppKit）
GITUI_HOME=/tmp/gitui-apphome nsh run     # 用独立的应用支持目录，便于验证
./target/debug/nsh-app                    # 直接跑调试构建
```

`APP_HOME` 覆盖应用支持目录（会话、最近仓库、界面语言），默认
`~/Library/Application Support/gitclient`（macOS）。

## 结构

| 路径 | 内容 |
|---|---|
| `src/lib.rs` | 根组件：窗口属性、主题、语言与路由 |
| `src/brand.rs` | 产品名、版本、构建号、存储目录名（**改名只改这里与 `nsh.json`**） |
| `src/design/` | 颜色 token、几何常量（`metrics.rs`）、图标（`icons.rs`） |
| `src/shell/` | 标题栏（仓库标签页）、工具栏、状态栏 |
| `src/screens/` | 欢迎页、工作区外壳 |
| `src/workspace/` | 侧栏、历史表、diff |
| `src/model.rs` | 标签页状态与装载（调用 `git` crate） |
| `src/i18n.rs` | 双语解析与相对时间格式化 |
| `locales/` | `en.json`、`zh-Hans.json`（键集合必须一致） |
| `theme.json` | 设计 token（light/dark），与设计 CSS 同值 |
| `tests/` | 见下 |

## 测试与验证

```sh
cargo test --workspace     # 几何断言（比对设计 CSS）、i18n 键集合、Git 读取
nsh check                  # 全量 cargo check
python3 ../../tools/design/geometry.py --board 04   # 某个画板的几何规格
```

- `src/design/metrics.rs` 的 `geometry_tests` 直接解析
  `designs/graft-git/project/assets/0a859d3b….bin`，与常量逐个比对；设计或实现漂移都会失败。
- 视觉验收：`nsh run` 后截图，与 `designs/graft-git/exports/…-png1x/` 的对应板比对
  （裁掉四边 24px，得到 1392×852）。
- P1 的截图留档在 `docs/verify/p1-welcome.png`。

## 当前进度

P1：欢迎页（含空态）、打开仓库、工作区外壳、只读历史与 diff、双语。工具栏的同步与
分支操作尚未接入，按禁用态渲染。后续阶段见 `plans.local/2026-11-14-design-parity-build.md`。
