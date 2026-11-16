# design — 设计系统文档

本目录只回答界面**长什么样、为什么这样**。`docs/` 下的其它目录放别的主题，与这里无关；
需要新增一类文档时，在 `docs/` 下另开一个同级目录，并更新 [docs/README.md](../README.md)。

这里只放**代码里读不出来的判断**：为什么这样分层、哪个 token 用在什么场合、哪些差异
是已决定的。数值本身不在这里维护——色值从 `theme.json` 生成，尺寸指向 `metrics.rs`。

四份文档按内容类型分开，互不重复：

| 文档 | 回答什么 | 什么时候读 | 维护方式 |
|---|---|---|---|
| [DESIGN-SYSTEM.md](DESIGN-SYSTEM.md) | 视觉语言：八层表面、四档文字、间距形状、状态与语义色、深色与动效 | 新画界面、判断「这块该用什么底色/字级」 | 手写；画板或原则变了才改 |
| [TOKENS.md](TOKENS.md) | 取值契约：token 从哪来、每个键的语义与代码入口、同名不同义的坑、怎么加 token | 改颜色、加 token、排查「颜色不对但没报错」 | 色值表脚本生成，其余手写 |
| [UI-KIT.md](UI-KIT.md) | 组件层：为什么有这一层、页面规则、组件清单、选用规则、增改流程 | 写页面、选组件、往 `ui/` 加东西 | 手写；组件增减时同步 |
| [DEVIATIONS.md](DEVIATIONS.md) | 设计稿与实现的已知差异，以及复核过「不算差异」的项 | 发现「和画板不一样」时，先查这张表 | 手写；画板改版后逐条复核 |

## 三个常见入口

- **改一处颜色**：`theme.json` 的 `light` 与 `dark` → 设计共享样式表同步 → `cargo test`
  （`theme_tests` 会拦）→ `python3 tools/design/tokens.py --write`。见 TOKENS §6。
- **加一个控件或改外观**：先 `ui/gallery.rs` 加示例，再改 `ui/`，跑 `cargo test` 与
  `tools/check-ui.sh`，最后才在页面用。见 UI-KIT §6。
- **「画板上不是这样」**：查 DEVIATIONS；不在表里的就是缺陷，按缺陷处理。新增差异要写清
  画板值与实现值、原因、能不能回退。

## 校验

```sh
(cd apps/client && cargo test)          # theme_tests、guard_tests、native_scale_tests
python3 tools/design/tokens.py          # TOKENS 的色值表与 theme.json 是否一致
tools/check-ui.sh                       # ui 层与已迁移页面的机械约束
```

## 不属于这里

- 产品范围、需求与能力边界：根 [README.md](../../README.md) 与各 crate 的 README。- 页面与画板的逐个对照进度：当前没有账本。要恢复，先决定它是「现状」还是「证据」，
  不要重新变成多份状态互相覆盖。
- 历史结论：2026-11-15 的旧文档在 [archive/2026-11-15-docs/](../../archive/2026-11-15-docs/)，
  已冻结，只作参考。
- 画板本身与导出物：[designs/graft-git/README.md](../../designs/graft-git/README.md)。
