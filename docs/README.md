# docs

`docs/` 按主题分目录：一个主题一个文件夹，文件夹里的 `README.md` 是该主题的入口。

设置窗口的六个分区按 [pages/settings.md](pages/settings.md)、[pages/settings-shortcuts.md](pages/settings-shortcuts.md)、[pages/settings-agent.md](pages/settings-agent.md)、[pages/settings-git.md](pages/settings-git.md)、[pages/settings-credentials.md](pages/settings-credentials.md)、[pages/settings-about.md](pages/settings-about.md) 实现。Updates、Credentials 与 Agent 的跨系统动作只给事实性提示，实现边界以根 README 的「已知限制」为准。

仓库窗口与左侧引用面板的开发需求包括启动时恢复会话或显示欢迎页、仓库标签的打开与关闭及排序，以及引用分组的折叠与右键动作。设计入口见[欢迎页](pages/welcome.md)、[常规设置](pages/settings.md)、[工作区](pages/workspace.md)与[菜单和对话框](pages/menus.md)。

欢迎页历史仅记录成功打开过的仓库；Git 设置中的扫描复选框控制扫描结果能否进入命令面板搜索，扫描发现不自动加入打开历史。命令面板空查询显示最近打开仓库和常用命令，输入查询后也搜索已启用的扫描结果。页面背景见[欢迎页](pages/welcome.md)、[Git 设置](pages/settings-git.md)与[命令面板](pages/command-palette.md)，实现差异见[DEVIATIONS](design/DEVIATIONS.md)。

Rust 格式由根目录 `rustfmt.toml` 固定；Agent 修改 Rust 代码后运行 `cargo fmt --all`。执行要求见项目 [AGENTS.md](../AGENTS.md)。

| 目录 | 内容 | 入口 |
|---|---|---|
| [design/](design/README.md) | 设计系统：视觉语言、token 契约、组件层、偏离清单 | [design/README.md](design/README.md) |
| [pages/](pages/README.md) | 逐页说明：版式、状态与切换、按钮作用 | [pages/README.md](pages/README.md) |
| [development/](development/README.md) | 开发规范：页面分层、响应式、交互与状态、数据生命周期、自动及人工验收 | [development/README.md](development/README.md) |

新增一类文档时：在 `docs/` 下开一个同级目录（例如 `docs/architecture/`），把该主题的
入口写成 `<目录>/README.md`，再在本表加一行。不同主题的文件不要平铺在 `docs/` 根下。

历史文档在 [archive/2026-11-15-docs/](../archive/2026-11-15-docs/)，已冻结，只作参考。
