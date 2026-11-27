# docs

当前能力与未接入项分别记录在根 README 的[「当前能力」](../README.md#当前能力)与[「已知限制」](../README.md#已知限制)中。页面目录包含 32 个画板条目与 30 份说明，Welcome 的 01b / 01c 合并在一份文档中；页面说明描述设计目标，不代表实现验收结果。页面完成标准见[页面验收规范](development/verification.md)。

History、Changes、Stash、Blame 与 Reflog 的差异面板共享 NSH DiffView 正文和工具栏，支持统一／分栏、切换变更片段与完整文件、按路径语法高亮；Unified/Split 由带悬停名称的单图标切换，工具栏图标均在悬停时显示名称。提交摘要与文件名各占一行并在空间不足时省略：摘要保留开头，文件名保留末尾；增删行数靠左，其余工具靠右；变更位置按实际改动段计数，增删行的红绿底色略加深。横向内容溢出使用 macOS 平台滚动指示器。NSH 0.6 DiffView 将横向滚动视图放在完整行内容内，平台指示器也随该内部视图定位；纵向滚动到文件底部时更容易看到指示器。Changes 的每个 hunk 使用复选框显示和切换暂存状态；显示段只要仍有重叠的暂存改动就保持勾选，取消时更新 Git index 并重新读取状态。文案按状态显示 `Stage Hunk` 或 `Unstage Hunk`，标签与复选框共用切换动作。行暂存仍使用悬浮操作且不占用代码行高度；完整文件模式的复选框作用于当前文件整体。文件树对部分暂存的单个文件显示半选；文件行非复选框区域打开对应 Diff，复选框只切换暂存状态，只有中栏当前显示同一文件时才更新 Diff。History 和 Reflog 的单文件差异标题支持前后文件切换，Reflog 从右栏文件树打开差异时保留检查器。所有差异入口显示变更行数并提供复制，移除 Export。History 提交表与各 Diff 入口必须为内部虚拟化组件设置可见视口；提交行随中栏宽度调整，作者和日期保留在可视范围内。Changes 清单可批量折叠，并通过右键菜单设置默认清单、修改名称、删除、暂存或取消暂存；筛选支持清空，平铺文件显示完整路径并提供悬停提示。管理弹层贴近 Manage 按钮，默认项的删除按钮禁用，筛选框清空按钮靠近右边界，已暂存复选框不显示边框。具体行为见[历史筛选](pages/history-filters.md)、[暂存和提交](pages/changes.md)与[引用日志](pages/reflog.md)。

History 页面优化要求：标题后显示提交数，分支、作者与高级筛选入口成组；分支和作者浮层输入即搜索、Apply 才应用选择，高级筛选移除重复条件。检查器统一文件视图切换，目录遵循设置，文件名与状态按变更类型着色，完整路径支持悬停查看。远端内部的分支目录组默认折叠。提交选择立即反馈，选择和详情读取不重建提交列表；目录展开保留文件树滚动状态。文件与目录的路径动作使用行右键菜单，统一 Reveal in Finder，文件还提供 History；移除顶部复制和编辑器按钮，增加展开／折叠全部按钮。提交 message 下方展示 description，统计与状态字母之间增加间距。快速搜索移到 Branch 左侧，宽度收窄到 128pt 并保留完整 placeholder，与 Author 等筛选按钮同高；Branch／Author 使用可滚动且有行间距的选项列表和右下角 Reset／Apply，关闭不应用，Reset 清空对应筛选后立即关闭生效，Apply 后台加载变化的条件，高级筛选日期标题按 `filter.whenLabel` 本地化且 Apply 位于最右。Inspector 移除底部 Cherry-pick／Revert 按钮栏；Compare 窗口的 Base／Target 选中项使用主题色，Swap／Clear 使用描边按钮。交互契约见[历史筛选](pages/history-filters.md)。

应用自绘弹窗统一采用 Compare reference 的标题栏和底栏，表单与 Compare 的操作按钮统一为 New Branch 的常规尺寸；分支、标签、远程、贮藏、工作树和身份表单均提供本地化 placeholder，未显示字段不占间距，输入不重建整张表单。Edit Commit Message、Squash、Hook 日志、凭据和设置确认沿用同一面板，同步窗口对齐标题栏／底栏；命令面板与轻量浮层保留专用结构。文件 History 列表左右增加内距。交互与样式入口见[菜单和对话框](pages/menus.md)、[组件层](design/UI-KIT.md)与[文件历史](pages/blame-history.md)。

`docs/` 按主题分目录：一个主题一个文件夹，文件夹里的 `README.md` 是该主题的入口。

设置窗口的六个分区按 [pages/settings.md](pages/settings.md)、[pages/settings-shortcuts.md](pages/settings-shortcuts.md)、[pages/settings-agent.md](pages/settings-agent.md)、[pages/settings-git.md](pages/settings-git.md)、[pages/settings-credentials.md](pages/settings-credentials.md)、[pages/settings-about.md](pages/settings-about.md) 实现。Updates 与 Agent 的跨系统动作只给事实性提示；凭据交互只在 Git 实际调用 AskPass 时出现，实现边界以根 README 的「已知限制」为准。

General 的日期格式设置统一用于 History、Refs / Reflog、Git 扫描状态与 Agent 日志；Unix 时间戳按系统本地时区显示，日期筛选边界由 Git 按本地时区解析。详细调用范围见[客户端 README](../apps/client/README.md#数据与语言)与[常规设置](pages/settings.md)。

Credentials 与 Git 执行能力的实施范围：移除 GitHub / GitLab 账号连接入口；只有 Git credential helper 未提供可用凭据、Git 实际调用 AskPass 时，应用才询问 HTTP(S) 凭据，保存交给 Git credential helper；Credentials 页面管理本地 `~/.ssh` 密钥对，并让 Git 可执行文件设置作用于应用发起的 Git 命令。规划已获批准；三个阶段均已实施，每阶段分别提交，完整记录见本地计划。

仓库窗口与左侧引用面板的开发需求包括启动时恢复会话或显示欢迎页、仓库标签的打开与关闭及排序，以及引用分组的折叠与右键动作；标签右键菜单可复制仓库路径、在 Finder 中显示目录并关闭单个或多个标签。无打开仓库时，主窗口默认显示不可关闭的 Welcome 模态层；打开仓库后可通过主窗口 `+`、Help > Welcome 或 Repository Manager 再次打开并关闭它。主窗口标题栏的仓库页签根据 NSH 提供的原生窗口按钮占位动态缩进，工具栏 Agent 仅显示图标。仓库会话恢复时优先后台加载活动仓库，其余标签在首次激活时再加载；切换到已加载标签或主窗口重新聚焦时，只刷新当前仓库，按 status、HEAD、引用 OID、操作和可见 diff 比较结果，数据未变时不写回仓库状态。页面状态与刷新验收边界见[页面验收规范](development/verification.md)。设计入口见[欢迎页](pages/welcome.md)、[常规设置](pages/settings.md)、[工作区](pages/workspace.md)与[菜单和对话框](pages/menus.md)。

欢迎页历史仅记录成功打开过的仓库；空查询显示历史，输入后同时搜索历史与全部已扫描仓库，扫描独占项不能从历史移除。Git 设置中的扫描复选框只控制扫描结果能否进入命令面板搜索。命令面板默认把常用命令放在仓库之前，输入后仍按命令、打开仓库、最近仓库、已启用扫描仓库分组。页面背景见[欢迎页](pages/welcome.md)、[Git 设置](pages/settings-git.md)与[命令面板](pages/command-palette.md)，实现差异见[DEVIATIONS](design/DEVIATIONS.md)。

Fetch、Pull、Push 共用 800×520 同步面板，以后台 Git 子进程运行并显示实际输出；完成状态说明目标是否已是最新，或列出确认的引用变化和提交节点数，部分 Push 拒绝会同时列出成功与拒绝数量，无法确认的提交数明确标出。原始 Git 输出仍保留在日志区。切换操作类型或远端目标时，右侧内容随当前目标更新，不显示其它任务的旧输出；新输出和切换后的当前日志都会自动滚动到底部。Cancel 会结束同步操作并刷新仓库状态；关闭面板会保留任务。工具栏 Fetch、Pull、Push 的结果在同步面板显示，主窗口工具栏下不再重复显示同步完成或失败横幅；提交区勾选 push 后的自动推送结果则显示在可关闭的提交状态块中。Welcome 模态层与 Settings 窗口可由重复入口重新打开或聚焦；macOS 应用菜单提供 Settings（⌘,），File 菜单可打开或清除最近仓库及关闭一个或全部窗口（⌥⌘W），关闭主窗口后可从 Dock 重开并依照恢复会话设置显示工作区或必需的 Welcome 模态层；View 菜单切换引用侧栏、检查器与全屏，Window 菜单提供上一／下一标签、Bring All to Front 和原生窗口列表（不列 Welcome 模态层），Help 菜单提供快捷键设置和 Welcome 入口。对应页面见[同步](pages/sync.md)、[同步 Fetch](pages/sync-fetch.md)、[同步 Pull](pages/sync-pull.md)、[工作区](pages/workspace.md)、[常规设置](pages/settings.md)与[菜单和对话框](pages/menus.md)。

工具栏、Reflog 与同步结果改进已完成：移除 Branch 工具按钮，Stash 位于 Reflog 左侧并打开 Stash Changes 表单；Stash、Reflog 与 Fetch／Pull／Push 使用相同按钮和颜色，Reflog 提供直达；保留 Refs 总表和 Refs / Reflog 来回切换，Reflog 页隐藏新建分支、标签、远程和贮藏操作；仓库加载时不读取 Reflog，打开列表后按每页 100 条读取，滚近底部时继续加载，分页追加后保留当前可见行，虚拟化表格占用筛选区下方的剩余视口；选择记录时只更新对应行的选中样式，保留表格实例与滚动位置；离开 Reflog 页面时清空列表与检查器数据；summary 单行显示并在过长时省略；选择 Reflog 记录后右栏复用 History 提交检查器的文件树、message／description 和提交信息，文件历史从该 Reflog SHA 打开；切回 Refs 总表后右栏使用 Changes 提交面板，具体行为见[引用日志](pages/reflog.md)；Fetch、Pull、Push 完成后说明目标是否已是最新或列出已确认的引用与提交数量，部分拒绝和未知计数也会明确呈现。设计源保持不变；工具栏、Reflog 右栏和同步完成摘要的偏离见[偏离清单](design/DEVIATIONS.md)，实施步骤与验收记录见本地计划。

History 默认展示全部引用可达提交与 graph，引用点击定位到提交；当前提交显示橙色 HEAD 徽标，分支徽标按名称稳定分配颜色。点击提交后工具栏切到 History，右侧检查器把提交摘要、作者与时间、短 SHA、复制入口和 Branches and refs 放在底部可滚动 Info 中，作者行显示 email。提交头底部留白收紧，让提交时间说明和引用行靠近上方提交信息。committer 与 author 身份或时间不同时才显示提交时间；身份相同时只显示日期和时间，不重复姓名与邮箱。Branches and refs 标签与引用值合并为同一文本，统一使用正文颜色和字阶，并排在提交时间说明之后。短 SHA 与复制图标合为一个按钮，放在作者时间行右侧。Info 不设额外外层内距，多选时相邻提交项之间以延伸至两侧的底部分割线分隔；多选时按提交列表顺序列出所有选中提交。点击文件可查看单文件差异，提交菜单可打开完整提交差异。右键已选中的提交会保留多选状态；选中项连续时可从右键菜单 Squash commits，交互细节见[历史筛选](pages/history-filters.md)。History 筛选支持分支、作者、日期、路径和消息组合条件；列表顶部筛选按钮右侧的快速搜索实时匹配提交，并可在输入框内清空，搜索与应用筛选分开；作者列默认采用 80pt 最小宽度，并可拖动加宽。Compare 支持连续 commit、全部本地/远端分支、tag 与 stash 端点、工作区比较、2-dot/3-dot 与 Swap；工作区差异覆盖暂存和未暂存的 tracked 文件。无共同祖先、相同端点与零差异有反馈。Merge 冲突页以 index stage 1/2/3 构建三栏，Result 使用按路径识别语法的可编辑器，并在当前会话内保留逐文件草稿；二进制冲突选择一侧，逐文件标记解决，继续前重新核对 Git 状态。页面说明见[历史筛选](pages/history-filters.md)、[文件 diff](pages/file-diff-focus.md)、[多提交 diff](pages/multi-commit-diff.md)、[引用比较](pages/compare-refs.md)与[合并冲突](pages/merge-conflict.md)。

提交区常显 Force 复选框，Push after commit 未启用时禁用；启用后采用 Git `--force-with-lease`。无暂存改动时不再显示提示行，提交、钩子及提交区自动推送的状态块可关闭；工具栏 Fetch、Pull、Push 的结果保留在同步弹窗，不再在工具栏下方重复显示。Changelists 底部提示缩短，Manage 按钮保持右对齐且不带省略号。详细说明见[暂存和提交](pages/changes.md)与[工作区](pages/workspace.md)。

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
