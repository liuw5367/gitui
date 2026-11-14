# designs/graft-git

30 个画板的设计源。生产实现只引用本目录，**不把 HTML/CSS 带进应用**。

## 目录

| 路径 | 内容 | 是否入库 |
|---|---|---|
| `project/canvas.json` | 画板定义（顺序、标题、分组） | 是 |
| `project/*.dc.html` | 每个画板一个 Design Component | 是 |
| `project/assets/*.bin` | 共享样式表（`0a859d3b…` 是可读 CSS）与图片资源 | 是 |
| `spec/*.json` | `tools/design/geometry.py` 从样式表与内联 style 提取的几何规格 | 是 |
| `exports/` | 独立 HTML（4 MB）与 PNG 1x/2x | **否**（`.gitignore` 忽略） |
| `reference/` | 需要就地比对时放 1x 参考图 | **否** |

## 重新导出（验收前确认 `exports/` 存在）

`project/*.dc.html` 通过 `/_blob/<css>` 引用共享样式表，需要 canvas 服务才能渲染；
`exports/` 里的独立 HTML 与 PNG 是脱机版本，由 canvas 编辑器导出：

1. 在 canvas 编辑器里打开 `designs/graft-git`（`.artifact-server.json` 记录了本地服务的
   地址与端口，形如 `http://127.0.0.1:<port>/design/<id>/`）。
2. 导出一次「独立 HTML」，得到 `exports/gitui — macOS Git 客户端.html`（可脱机打开）。
3. 导出「PNG 1x」到 `exports/gitui — macOS Git 客户端-png1x/`，文件名形如
   `04 · History & diff.png`，尺寸 1440×900。
4. 需要核对细节时再导出 PNG 2x 到 `...-png2x/`（2880×1800）。

验收使用 1x：四周各 24px 是画布桌面留白，窗口内容区为 **1392×852**；比较时裁掉这
24px，并排除标题栏左侧约 78×44pt 的原生窗口控制区（设计板画的是装饰圆点）。

## 编号与文件

画板编号（`01`–`30`）与 PNG 文件名一致；编号到 `.dc.html` 的对应关系写在
`tools/design/geometry.py` 的 `BOARD_FILES` 里，`spec/<编号>.json` 按编号命名。
