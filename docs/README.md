# docs

`docs/` 按主题分目录：一个主题一个文件夹，文件夹里的 `README.md` 是该主题的入口。

设置窗口的六个分区按 [pages/settings.md](pages/settings.md)、[pages/settings-shortcuts.md](pages/settings-shortcuts.md)、[pages/settings-agent.md](pages/settings-agent.md)、[pages/settings-git.md](pages/settings-git.md)、[pages/settings-credentials.md](pages/settings-credentials.md)、[pages/settings-about.md](pages/settings-about.md) 实现。Updates、Credentials 与 Agent 的跨系统动作只给事实性提示，实现边界以根 README 的「已知限制」为准。

仓库窗口与左侧引用面板的开发需求包括启动时恢复会话或显示欢迎页、仓库标签的打开与关闭及排序，以及引用分组的折叠与右键动作。设计入口见[欢迎页](pages/welcome.md)、[常规设置](pages/settings.md)、[工作区](pages/workspace.md)与[菜单和对话框](pages/menus.md)。

欢迎页历史仅记录成功打开过的仓库；空查询显示历史，输入后同时搜索历史与全部已扫描仓库，扫描独占项不能从历史移除。Git 设置中的扫描复选框只控制扫描结果能否进入命令面板搜索。命令面板默认把常用命令放在仓库之前，输入后仍按命令、打开仓库、最近仓库、已启用扫描仓库分组。页面背景见[欢迎页](pages/welcome.md)、[Git 设置](pages/settings-git.md)与[命令面板](pages/command-palette.md)，实现差异见[DEVIATIONS](design/DEVIATIONS.md)。

Fetch、Pull、Push 共用 800×520 同步面板，以后台 Git 子进程运行并显示实际输出。关闭面板会保留任务，Cancel 才终止它。Welcome 与 Settings 的已开窗口可由重复入口重新聚焦；macOS 应用菜单提供 Settings 与 ⌘,。对应页面见[同步](pages/sync.md)、[同步 Fetch](pages/sync-fetch.md)、[同步 Pull](pages/sync-pull.md)、[工作区](pages/workspace.md)、[常规设置](pages/settings.md)与[菜单和对话框](pages/menus.md)。

History 默认展示全部引用可达提交与 graph，引用点击定位到提交；右侧检查器显示提交信息和变更文件目录树，点击文件可查看单文件差异，提交菜单可打开完整提交差异。NSH DiffView 按每个文件路径识别语法，支持 Unified/Split、上下文折叠、换行、空白可视化、Git 忽略空白、hunk 导航、复制与导出；原始 patch 保持不变。History 筛选支持分支、作者、日期、路径和消息组合条件，工具栏快速搜索与应用筛选分开。Merge 冲突页以 index stage 1/2/3 构建三栏，Result 使用按路径识别语法的可编辑器，并在当前会话内保留逐文件草稿；二进制冲突选择一侧，逐文件标记解决，继续前重新核对 Git 状态。页面说明见[历史筛选](pages/history-filters.md)、[文件 diff](pages/file-diff-focus.md)、[多提交 diff](pages/multi-commit-diff.md)、[引用比较](pages/compare-refs.md)与[合并冲突](pages/merge-conflict.md)。

应用偏好统一保存在 `settings.toml`；会话、最近仓库、变更清单与 Agent 审计仍使用独立文本记录。旧版设置 `.txt` 不迁移、不删除；格式与失败行为见 [store 持久化说明](../crates/store/README.md)。

Rust 格式由根目录 `rustfmt.toml` 固定；Agent 修改 Rust 代码后运行 `cargo fmt --all`。执行要求见项目 [AGENTS.md](../AGENTS.md)。

| 目录 | 内容 | 入口 |
|---|---|---|
| [design/](design/README.md) | 设计系统：视觉语言、token 契约、组件层、偏离清单 | [design/README.md](design/README.md) |
| [pages/](pages/README.md) | 逐页说明：版式、状态与切换、按钮作用 | [pages/README.md](pages/README.md) |
| [development/](development/README.md) | 开发规范：页面分层、响应式、交互与状态、数据生命周期、自动及人工验收 | [development/README.md](development/README.md) |

新增一类文档时：在 `docs/` 下开一个同级目录（例如 `docs/architecture/`），把该主题的
入口写成 `<目录>/README.md`，再在本表加一行。不同主题的文件不要平铺在 `docs/` 根下。

历史文档在 [archive/2026-11-15-docs/](../archive/2026-11-15-docs/)，已冻结，只作参考。
