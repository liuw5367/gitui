# docs

History 提交表与各 Diff 入口必须为内部虚拟化组件设置可见视口；提交行随中栏宽度调整，作者和日期保留在可视范围内。Changes 与 Stash 使用 NSH DiffView 的语法高亮，逐行、逐段暂存仍按原始 Git 补丁定位。具体行为见[历史筛选](pages/history-filters.md)与[暂存和提交](pages/changes.md)。

History 页面优化要求：标题后显示提交数，分支、作者与高级筛选入口成组；分支和作者浮层输入即搜索、关闭时应用选择，高级筛选移除重复条件。检查器统一文件视图切换，目录遵循设置，文件名与状态按变更类型着色，完整路径支持悬停查看。远端内部的分支目录组默认折叠。交互契约见[历史筛选](pages/history-filters.md)。

`docs/` 按主题分目录：一个主题一个文件夹，文件夹里的 `README.md` 是该主题的入口。

设置窗口的六个分区按 [pages/settings.md](pages/settings.md)、[pages/settings-shortcuts.md](pages/settings-shortcuts.md)、[pages/settings-agent.md](pages/settings-agent.md)、[pages/settings-git.md](pages/settings-git.md)、[pages/settings-credentials.md](pages/settings-credentials.md)、[pages/settings-about.md](pages/settings-about.md) 实现。Updates、Credentials 与 Agent 的跨系统动作只给事实性提示，实现边界以根 README 的「已知限制」为准。

仓库窗口与左侧引用面板的开发需求包括启动时恢复会话或显示欢迎页、仓库标签的打开与关闭及排序，以及引用分组的折叠与右键动作。设计入口见[欢迎页](pages/welcome.md)、[常规设置](pages/settings.md)、[工作区](pages/workspace.md)与[菜单和对话框](pages/menus.md)。

欢迎页历史仅记录成功打开过的仓库；空查询显示历史，输入后同时搜索历史与全部已扫描仓库，扫描独占项不能从历史移除。Git 设置中的扫描复选框只控制扫描结果能否进入命令面板搜索。命令面板默认把常用命令放在仓库之前，输入后仍按命令、打开仓库、最近仓库、已启用扫描仓库分组。页面背景见[欢迎页](pages/welcome.md)、[Git 设置](pages/settings-git.md)与[命令面板](pages/command-palette.md)，实现差异见[DEVIATIONS](design/DEVIATIONS.md)。

Fetch、Pull、Push 共用 800×520 同步面板，以后台 Git 子进程运行并显示实际输出；完成状态说明目标是否已是最新，或列出确认的引用变化和提交节点数，部分 Push 拒绝会同时列出成功与拒绝数量，无法确认的提交数明确标出。原始 Git 输出仍保留在日志区。切换操作类型或远端目标时，右侧内容随当前目标更新，不显示其它任务的旧输出；新输出和切换后的当前日志都会自动滚动到底部。Cancel 会结束同步操作并刷新仓库状态；关闭面板会保留任务。Welcome 与 Settings 的已开窗口可由重复入口重新聚焦；macOS 应用菜单提供 Settings 与 ⌘,。对应页面见[同步](pages/sync.md)、[同步 Fetch](pages/sync-fetch.md)、[同步 Pull](pages/sync-pull.md)、[工作区](pages/workspace.md)、[常规设置](pages/settings.md)与[菜单和对话框](pages/menus.md)。

工具栏、Reflog 与同步结果改进已完成：移除 Branch、Stash 工具按钮，增加 Reflog 直达；保留 Refs 总表和 Refs / Reflog 来回切换；选择 Reflog 记录后右栏只读显示提交信息；Fetch、Pull、Push 完成后说明目标是否已是最新或列出已确认的引用与提交数量，部分拒绝和未知计数也会明确呈现。设计源保持不变；工具栏、Reflog 右栏和同步完成摘要的偏离见[偏离清单](design/DEVIATIONS.md)，实施步骤与验收记录见本地计划。

History 默认展示全部引用可达提交与 graph，引用点击定位到提交；当前提交显示橙色 HEAD 徽标，分支徽标按名称稳定分配颜色。点击提交后工具栏切到 History，右侧检查器保留当前页签（默认 Files），作者行显示 email；检查器页签与 Blame 侧栏页签有对称的上下留白。点击文件可查看单文件差异，提交菜单可打开完整提交差异。NSH DiffView 按每个文件路径识别语法，支持 Unified/Split、上下文折叠、换行、空白可视化、Git 忽略空白、hunk 导航、复制与导出；原始 patch 保持不变。History 筛选支持分支、作者、日期、路径和消息组合条件；列表顶部筛选按钮右侧的快速搜索实时匹配提交，并可在输入框内清空，搜索与应用筛选分开。Compare 支持连续 commit、全部本地/远端分支、tag 与 stash 端点、工作区比较、2-dot/3-dot 与 Swap；工作区差异覆盖暂存和未暂存的 tracked 文件。无共同祖先、相同端点与零差异有反馈。Merge 冲突页以 index stage 1/2/3 构建三栏，Result 使用按路径识别语法的可编辑器，并在当前会话内保留逐文件草稿；二进制冲突选择一侧，逐文件标记解决，继续前重新核对 Git 状态。页面说明见[历史筛选](pages/history-filters.md)、[文件 diff](pages/file-diff-focus.md)、[多提交 diff](pages/multi-commit-diff.md)、[引用比较](pages/compare-refs.md)与[合并冲突](pages/merge-conflict.md)。

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
