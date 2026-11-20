# TOKENS — 颜色与尺寸的取值契约

这份文档回答「值是多少、从哪儿取、改哪个文件」，不重复代码里的数字。文档里的
色值表由 [tools/design/tokens.py](../../tools/design/tokens.py) 从 `theme.json` 生成，
手改会在下一次校验时被覆盖。

同一套 token 的**用法**（哪个表面配哪一级文字、什么场合用 `sel` 而不是 `hover`）在
[DESIGN-SYSTEM.md](DESIGN-SYSTEM.md)；组件怎么取用它们见 [UI-KIT.md](UI-KIT.md)。

## 1. 取值链路

颜色从设计源到界面穿过三层，每层只做一件事：

| 层 | 文件 | 内容 | 改动方式 |
|---|---|---|---|
| 设计源 | `designs/graft-git/project/assets/0a859d3b…bin` 的 `:root` 与 `.theme-dark` | 68 / 65 个 `--var`，30 块画板共用 | 在 canvas 里改，或用编辑器改共享样式表 |
| 运行时 | `apps/client/theme.json`（shadcn `cssVars` 格式） | `light` 87 键、`dark` 84 键 | 与设计源同值地改两个模式 |
| 代码入口 | `apps/client/src/design/mod.rs` | 每个语义角色一个 `pub(crate) fn`，内部 `token("键名")` | 新增 token 时加函数；页面只调函数 |

两道校验，`cargo test` 里跑：

- `design/mod.rs` 的 `theme_tests` 逐键比对 `theme.json` 与设计共享样式表，
  light 与 dark 各一个测试。**设计改了而 `theme.json` 没跟，这里失败。**
- [tools/design/tokens.py](../../tools/design/tokens.py) 比对本文档的表与 `theme.json`。
  **`theme.json` 改了而文档没重新生成，这里失败。**

`theme_tests` 三处刻意排除：`accent`（设计里是主色，我们用 `blue` 存、`accent` 留给
选中底色）、`desk1` / `desk2`（画板的桌面渐变，不属于窗口内容，未移植）。

反方向不校验：`theme.json` 可以比设计修样式表多出键（shadcn 基座、`mono`、`focus`
等实现侧必需项），多出来的键不会让 `theme_tests` 失败。

## 2. 颜色

`theme.json` 比设计源多出的键集中在第一组（shadcn / NSH 主题基座）与少数实现侧
条目。`代码入口` 列是 `design` 模块里的函数名；`—` 表示没有封装，代码里直接
`design::token("键名")` 取——实际这样读的有 `popover`、`overlay`、`primary-bg`、
`primary-foreground`、`sel-strong`。

最后一组里的 `shadow-menu` 与 `focus` 是效果值（弹层阴影、焦点环），不是颜色；它们
同样随明暗切换，所以和状态色放在一张表里。`focus` 的 Dark 列是 `—`：深色块里没有这个
键，见 §7。

这两个效果值都**不能**当颜色用：`theme.color("focus")` 解析不出来，只会退回前景色。
焦点环是描边（`outline`），它取的是强调蓝 `blue`（`ui::focus_color()`），与输入框、
按钮聚焦时的边框同色。

<!-- tokens:colors:begin（由 tools/design/tokens.py 生成，勿手改） -->

#### 1. NSH / shadcn 基座

| Token | 代码入口 | Light | Dark |
|---|---|---|---|
| `background` | — | `#FFFFFF` | `#1B1B1D` |
| `foreground` | — | `#1D1D1F` | `#F2F2F4` |
| `card` | — | `#FFFFFF` | `#1F1F21` |
| `card-foreground` | — | `#1D1D1F` | `#F2F2F4` |
| `popover` | — | `#FCFCFD` | `#2C2C2E` |
| `popover-foreground` | — | `#1D1D1F` | `#F2F2F4` |
| `primary` | — | `#0A66E8` | `#0A84FF` |
| `primary-foreground` | — | `#FFFFFF` | `#FFFFFF` |
| `secondary` | — | `#F1F1F4` | `#262628` |
| `secondary-foreground` | — | `#1D1D1F` | `#F2F2F4` |
| `muted` | — | `#F1F1F4` | `#262628` |
| `muted-foreground` | — | `#6E6E76` | `#8E8E96` |
| `accent` | — | `#E7F0FF` | `#15324F` |
| `accent-foreground` | — | `#1D1D1F` | `#F2F2F4` |
| `destructive` | — | `#B3261E` | `#FF8A80` |
| `border` | — | `#E2E2E6` | `#343438` |
| `input` | — | `#D2D2D8` | `#3C3C42` |
| `ring` | — | `#007AFF` | `#0A84FF` |
| `sidebar` | `design::sidebar()` | `#F4F4F6` | `#232325` |
| `sidebar-foreground` | — | `#1D1D1F` | `#F2F2F4` |
| `sidebar-border` | — | `#E2E2E6` | `#343438` |

#### 2. 表面（窗口骨架）

| Token | 代码入口 | Light | Dark |
|---|---|---|---|
| `content` | `design::content()` | `#FFFFFF` | `#1B1B1D` |
| `panel` | `design::panel()` | `#F7F7F9` | `#1F1F21` |
| `panel2` | `design::panel2()` | `#F1F1F4` | `#262628` |
| `bar` | `design::bar()` | `#F5F5F7` | `#2A2A2C` |
| `titlebar` | `design::titlebar()` | `#F0F0F2` | `#262628` |
| `tabbar` | — | `#E9E9EC` | `#202022` |
| `status` | `design::status()` | `#F7F7F8` | `#242426` |

#### 3. 文本

| Token | 代码入口 | Light | Dark |
|---|---|---|---|
| `text` | `design::text()` | `#1D1D1F` | `#F2F2F4` |
| `text2` | `design::text2()` | `#5D5D65` | `#B3B3BB` |
| `text3` | `design::text3()` | `#6E6E76` | `#8E8E96` |

#### 4. 线条与描边

| Token | 代码入口 | Light | Dark |
|---|---|---|---|
| `line` | `design::line()` | `#E2E2E6` | `#343438` |
| `line-soft` | `design::line_soft()` | `#ECECEF` | `#2C2C30` |
| `line-strong` | `design::line_strong()` | `#D2D2D8` | `#414147` |
| `win-border` | — | `rgba(0,0,0,0.16)` | `rgba(255,255,255,0.14)` |
| `overlay` | — | `rgba(0,0,0,0.12)` | `rgba(0,0,0,0.42)` |

#### 5. 交互反馈

| Token | 代码入口 | Light | Dark |
|---|---|---|---|
| `hover` | `design::hover()` | `rgba(0,0,0,0.045)` | `rgba(255,255,255,0.055)` |
| `press` | — | `rgba(0,0,0,0.085)` | `rgba(255,255,255,0.09)` |
| `sel` | `design::selected()` | `#E7F0FF` | `#15324F` |
| `sel-strong` | — | `#D5E6FF` | `#1B3D60` |
| `focus` | — | `0 0 0 3px rgba(0,122,255,.28)` | — |
| `shadow-menu` | — | `0 12px 34px rgba(0,0,0,.22),0 1px 4px rgba(0,0,0,.12)` | `0 14px 40px rgba(0,0,0,.55),0 1px 4px rgba(0,0,0,.4)` |

#### 6. 强调色

| Token | 代码入口 | Light | Dark |
|---|---|---|---|
| `blue` | `design::accent()` | `#007AFF` | `#0A84FF` |
| `accent-2` | `design::accent2()` | `#0A66E8` | `#409CFF` |
| `accent-soft` | `design::accent_soft()` | `#E8F1FF` | `#15324F` |
| `primary-bg` | — | `#0A66E8` | `#0A66E8` |

#### 7. 表单与按钮

| Token | 代码入口 | Light | Dark |
|---|---|---|---|
| `field` | `design::field()` | `#FFFFFF` | `#232326` |
| `field-border` | `design::field_border()` | `#D2D2D8` | `#3C3C42` |
| `btn` | `design::btn()` | `#FFFFFF` | `#2C2C2F` |
| `btn-border` | `design::btn_border()` | `#D2D2D8` | `#414147` |

#### 8. 语义状态

| Token | 代码入口 | Light | Dark |
|---|---|---|---|
| `warn` | `design::warn()` | `#A86400` | `#E8B04A` |
| `danger` | `design::danger()` | `#B3261E` | `#FF8A80` |
| `ok` | `design::ok()` | `#1F7A45` | `#6FD39A` |
| `purple` | — | `#7B4FD1` | `#BFA0FF` |

#### 9. 引用、标签与头像

| Token | 代码入口 | Light | Dark |
|---|---|---|---|
| `branch-bg` | `design::branch_bg()` | `#E8F1FF` | `#15324F` |
| `branch-fg` | `design::branch_fg()` | `#0B5FCC` | `#8CC2FF` |
| `tag-bg` | `design::tag_bg()` | `#FFF3DD` | `#3A2E16` |
| `tag-fg` | `design::tag_fg()` | `#835700` | `#F0C674` |
| `tag-border` | `design::tag_border()` | `#F0DEB8` | `#514020` |
| `remote-fg` | `design::remote_fg()` | `#5D5D65` | `#A9A9B1` |
| `avatar-bg` | — | `#DCE7F5` | `#2C3D52` |
| `avatar-fg` | — | `#2F5D8A` | `#BBD2EC` |

#### 10. diff 与折叠

| Token | 代码入口 | Light | Dark |
|---|---|---|---|
| `add-bg` | `design::add_bg()` | `#EAF7EF` | `rgba(46,160,90,0.17)` |
| `add-fg` | `design::add_fg()` | `#1B7442` | `#7EE2A8` |
| `add-word` | — | `#CBEAD6` | `rgba(84,200,130,.28)` |
| `add-ln` | — | `#E2F1E8` | `rgba(46,160,90,.10)` |
| `del-bg` | `design::del_bg()` | `#FDEEEC` | `rgba(214,72,60,0.15)` |
| `del-fg` | `design::del_fg()` | `#AE241C` | `#FF9B93` |
| `del-word` | — | `#F8D0CB` | `rgba(235,100,90,.26)` |
| `del-ln` | — | `#FBE6E3` | `rgba(214,72,60,.09)` |
| `hunk-bg` | — | `#F2F6FA` | `#1E2833` |
| `hunk-fg` | — | `#465F78` | `#A8C0D8` |
| `fold-bg` | — | `#F8F8FA` | `#202024` |

#### 11. 代码高亮

| Token | 代码入口 | Light | Dark |
|---|---|---|---|
| `code-keyword` | — | `#9B2393` | `#FC5FA3` |
| `code-string` | — | `#C41A16` | `#FC6A5D` |
| `code-comment` | — | `#697B8C` | `#7F8C98` |
| `code-number` | — | `#1C00CF` | `#D0BF69` |
| `code-type` | — | `#0F68A0` | `#5DD8FF` |
| `code-fn` | — | `#316D74` | `#67B7A4` |

#### 12. 提交图线

| Token | 代码入口 | Light | Dark |
|---|---|---|---|
| `g1` | — | `#0A84FF` | `#4CA3FF` |
| `g2` | — | `#8E5AD6` | `#C084FC` |
| `g3` | — | `#2E9E6B` | `#4ADE9A` |
| `g4` | — | `#C26A00` | `#F0B44A` |

#### 13. 菜单

| Token | 代码入口 | Light | Dark |
|---|---|---|---|
| `menu-bg` | — | `#FCFCFD` | `#2C2C2E` |
| `menu-line` | — | `#E5E5EA` | `#3D3D42` |

<!-- tokens:colors:end -->

## 3. 字体族

<!-- tokens:fonts:begin（由 tools/design/tokens.py 生成，勿手改） -->

| Token | 代码入口 | 值 |
|---|---|---|
| `mono` | — | `"SF Mono",ui-monospace,SFMono-Regular,Menlo,monospace` |
| `sans` | — | `-apple-system,BlinkMacSystemFont,"SF Pro Text","Helvetica Neue",Helvetica,Arial,sans-serif` |

`字体族` 只有一组值，不随明暗切换。

<!-- tokens:fonts:end -->

`ui::MONO` 是 `"SF Mono"` 的字面量，不走这两个 token；`ui::code(..)` 是唯一应该
设置等宽字族的入口。

## 4. 尺寸不是颜色 token

尺寸没有 `theme.json` 这一层：它不随明暗切换，也不由主题文件注入。唯一来源是
[`design/metrics.rs`](../../apps/client/src/design/metrics.rs) 的 `native` 模块，
`ui` 层的常量是它的别名。

| 角色 | 常量 | 值 |
|---|---|---|
| 正文 / 行高 | `native::FONT_BODY` / `LINE_BODY` | 13 / 18 |
| 次要文字 / 行高 | `native::FONT_SECONDARY` / `LINE_SECONDARY` | 11 / 15 |
| 元数据、表头 | `native::FONT_META` | 11 |
| 代码 / 行高 | `native::FONT_CODE` / `LINE_CODE` | 12 / 18 |
| 控件高 | `native::CONTROL_SMALL` / `CONTROL` / `CONTROL_LARGE` | 22 / 28 / 32 |
| 行高 | `native::ROW` / `ROW_COMPACT` | 28 / 24 |
| 工具栏 / 状态栏 | `native::TOOLBAR` / `STATUS` | 52 / 24 |
| 标题栏 | `ui::TITLEBAR` | 44 |
| 侧栏宽 | `metrics::SIDEBAR_W` | 232 |
| 间距 | `ui::GAP_XS` … `GAP_XL` | 4 / 6 / 8 / 12 / 16 |
| 圆角 | `metrics::RADIUS_CONTROL` / `RADIUS_PANEL` / `RADIUS_APPMARK` | 6 / 8 / 15 |
| 焦点环 | `ui::FOCUS_WIDTH` | 2 |

`theme.json` 的 `cssVars.theme.radius`（`6px`）只作用于 NSH 基线的默认圆角，应用
自绘的圆角一律取 `metrics::RADIUS_*`；两处没有打通，改圆角时别只改一处。

`metrics.rs` 上半部分（`FONT_BASE`、`FONT_META`、`FONT_SECONDARY`、`FONT_WELCOME_TITLE`、
`BUTTON_H`、`DIFF_FONT` 等）是「设计稿逐值还原」阶段的产物，原生刻度之后已不再驱动界面：
`FONT_*` 一组与 `BUTTON_H`、`DIFF_FONT` 目前没有外部引用，`FIELD_H`、`COMMIT_ROW_H`、
`HISTORY_H`、`SIDE_ROW_H` 只剩个别调用点。**调尺寸改 `native`，不要改这些**；它们留着是
因为画板与 `docs/pages/` 的逐页文档还按这些值说话，删掉就少了一份对照依据。

注意同名不同层：`metrics::FONT_META` 是画板值 9.5，`native::FONT_META` 是实现的 11。
没有前缀的那个属于旧基准。

## 5. 同名不同义：改之前先看这一节

| 名字 | 这里的含义 | 容易误用成 |
|---|---|---|
| `accent` | shadcn 语义的**浅强调底**（light `#E7F0FF`），等同于选中底色 | 主色蓝。主色在 `blue`（`#007AFF`），`design::accent()` 读的是 `blue` |
| `blue` | 链接、图标、焦点的系统蓝 `#007AFF` | 按钮实心底色。实心主按钮用 `primary-bg`（`#0A66E8`） |
| `accent-2` | 次级强调蓝 `#0A66E8`（浅色）/ `#409CFF`（深色），用于可点击文字 | 主按钮。它和 `primary-bg` 浅色同值，但深色分叉 |
| `primary-bg` | 主按钮实心底，明暗都是 `#0A66E8` | 跟随 `primary` 变亮。深色模式主按钮**不**变亮，是设计稿原样 |
| `text3` | 三级文字（元数据）`#6E6E76` | `muted-foreground`（同值不同名，给基线组件用）。界面代码用 `design::text3()` |
| `panel2` | 设计 CSS 里叫 `--panel-2` | 拼成 `panel-2` 会取不到值，`token()` 静默退回前景色 |

`token()` 取不到键时返回 `theme.foreground` 而不 panic，所以键名写错不会崩，只会
让那块区域变成正文色。看到「颜色不对但没报错」先查键名拼写。

另一处会随模式换写法的是 diff 底色：`add-word` / `del-word` / `add-ln` / `del-ln` 浅色是
实色（`#CBEAD6` 一类），深色是半透明（`rgba(84,200,130,.28)` 一类）。改这四组要两个模式
一起看——alpha 叠在不同表面上结果不同。

## 6. 加或改一个 token

1. 画板共享样式表的 `:root` 与 `.theme-dark` 各加一行（两处都要，缺一个 `theme_tests` 会失败）。
2. `theme.json` 的 `light` 与 `dark` 同步同值。
3. `design/mod.rs` 加取用函数，命名按语义（`add_bg()`）而不是颜色（`green_light()`）。
4. 在 [tokens.py](../../tools/design/tokens.py) 的 `GROUPS` 里登记所属分组——没登记的键会让
   `tokens.py` 直接失败，这是刻意的：新 token 必须顺便回答「它属于哪一层」。
5. `python3 tools/design/tokens.py --write` 重新生成第 2 节的表。
6. `(cd apps/client && cargo test)` 与 `python3 tools/design/tokens.py` 都应通过。

只在代码里用的键（`focus`、`shadow-menu` 这类效果值）可以只进 `theme.json`，但仍要
在 `GROUPS` 登记。

## 7. 已知缺口

- `desk1` / `desk2` 只在画板的桌面背景里，窗口内容不用，未移植。
- 圆角两处来源（`theme.cssVars.theme.radius` 与 `metrics::RADIUS_*`）没有合流。
- `focus` 只在 `light` 里有值，nsh 的规则是「只写一侧的键，两个模式共用该值」
  （`theme.rs`：“Set in one appearance only, the color is both's”）。它是 box-shadow
  效果值、不是颜色，因此没有代码入口：焦点环改用 `blue` 描边，深浅两个模式各取自己的
  强调蓝，这条缺口不再影响界面。
- 几个键目前没有读取点：`press`、`purple`、`shadow-menu`。菜单走系统 `NSMenu` 绘制，
  阴影用不到；`press` 与 `purple` 是设计稿里有、界面还没用上的语义。
- `metrics.rs` 上半部分的画板逐值常量与 `native` 刻度并存，前者已不再驱动界面，
  但没有删除；`docs/pages/` 的逐页文档仍按这些画板值描述尺寸。
