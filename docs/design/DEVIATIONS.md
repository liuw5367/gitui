# DEVIATIONS — 设计稿与实现的已知差异

设计稿（`designs/graft-git`，30 块画板）与实现之间的可见差异登记在此。**没登记的差异
视为缺陷**，不因为「画板上是那样」就放过，也不因为「实现已经这样很久了」就默认合理。

每条差异必须能复核：画板侧给选择器与属性，实现侧给常量或 `文件:行`。不做「看起来不太
一样」的模糊条目。编号稳定，删除的条目不回收。

## 1. 刻度类

这些差异来自一个已批准的决定：**字号、行高、控件高以 macOS 系统刻度为准**，画板决定
结构与层级，不决定这些数值。画板的 8.5–10.5px 字号低于可读下限。

| # | 位置 | 画板值 | 实现值 | 原因 | 可回退 |
|---|---|---|---|---|---|
| 1 | 全库字号 | `.side-head` 10.5、`.commit-cols` 9.5、`.statusbar` 10.5、`.diffbody` 11 | 11–13（`native::FONT_*`，`metrics.rs:145–176`） | 11pt 是可读下限；界面中不存在小于 11 的用户可见文字 | 否（已决定） |
| 2 | 侧栏行高 | `.side-row` 25 | `native::ROW` 28 | 系统列表行高 | 是 |
| 3 | 按钮高 | `.btn` 24 | `native::CONTROL` 28（小号仍 22） | 系统控件高度 | 是 |
| 4 | 输入框高 | `.field` 26 | `native::CONTROL` 28 | 系统控件高度 | 是 |
| 5 | 工具栏高 | `.toolbar.v2` 44 | `native::TOOLBAR` 52 | 系统工具栏高度 | 是 |
| 6 | 分段圆角 | `.seg` 7 | `metrics::RADIUS_CONTROL` 6 | 与应用其它控件统一 | 是 |
| 7 | 欢迎页标题 | `.wtitle` 31 / `.wside-title` 17 | `ui::TITLE_LARGE` 26 / `ui::TITLE` 15（`ui/mod.rs:54`、`:56`） | 这两级没进 `native` 刻度，是手工定的；比画板小 | 是（要严格对齐需先补进 `native`） |
| 8 | 图标尺寸 | 画板同一屏混用 13 / 14 / 15 / 16 | `IconSize` 四档 11 / 12 / 13 / 16，另有三处 9 / 15 / 20 | nsh 0.6.0 没有供普通视图使用的 SF Symbols 图标接口；iOS 导航栏菜单项可用 system symbol；三处例外是插画式用法（行内折叠箭头、设置页分支、错误横幅） | 部分 |

## 2. 结构类

这些是「画板画得出来、NSH 0.6.0 装不下」或「按画板做会产生假控件」的差异，逐条记录了
替代方案。

| # | 位置 | 画板值 | 实现值 | 原因 | 可回退 |
|---|---|---|---|---|---|
| 9 | 历史表 | `.commit` 四列网格 `64 / minmax / 96 / 58` | 单列随中栏可用宽度伸缩，整行在 `render` 里自绘；外部已有列头，内部标题为空且标题带缩至 1pt（`features/history/view.rs`、`ui/table.rs`） | 保留图线、引用徽章和对比底色，避免固定 1200pt 宽度将作者、日期裁出中栏；NSH 0.6.0 的标题带最小为 1pt | 是（改为多列需重画这些元素） |
| 10 | 变更清单 | 贴按钮的自定义弹层 | 系统菜单 `popup_menu_at`（`features/changes/view.rs:468`） | 它由按钮触发、贴在其下方，不是模态；套 `ui::dialog` 会变成居中遮罩，语义相反 | 是 |
| 12 | 分块暂存方框 | 随行高变化的可点方框 | 12×12（`ui/check.rs:14–22` 的 `indicator_action`；`indicator` 同尺寸） | 表格单元格空间有限，且要与 `indicator` 一致 | 是 |

| 14 | 欢迎窗口标题栏 | 38 高自定义栏（窗口灯、齿轮、问号） | 原生标题栏 | 平台原生，不伪造窗口灯；Settings 在应用菜单、主窗口工具栏与左栏底栏（文字入口），Help 在应用菜单 | 是（隐藏标题栏后需自绘拖拽区） |
| 15 | 欢迎页最近仓库分组 | Today / Previous 7 days | 增加 Earlier | 画板只有两组，12 条记录里超过 7 天的会被隐藏 | 是（会重新隐藏旧记录） |
| 16 | 克隆动作行摘要 | 仓库大小、可见性与环境提示 | 固定 hint；`http://` 时追加本地镜像提示 | 不访问远端，避免假数据 | 是（接入远端查询后替换） |
| 17 | 欢迎页删除最近仓库 | 19×19 | `ui` 小号图标按钮 22×22 | 原生小号按钮刻度；画板 19 低于系统控件 | 是（需新增 20 档） |
| 18 | 克隆表单 Depth | 150 宽 | 按内容取宽（约 210，`features/welcome/view.rs` 的 depth 容器 `flex_shrink_0`） | 三个分段在 150 内放不下；写死宽度会把最右侧的 `Shallow 50` 裁掉 | 是（改回 150 需缩写标签） |
| 19 | 欢迎页空态 | 01b 虚线卡片：图标块 + 标题 + 说明 | 同左，另加 `Scan Repositories` 次按钮 | 扫描能力尚未接入，按反馈先提供入口；点击给真实状态说明 | 是（删按钮） |
| 20 | Init 的 `.gitignore` | `Add a .gitignore (macOS + Swift)`，生成 Xcode / SwiftPM 条目 | `Add a .gitignore`，生成通用最小集合（`.DS_Store`、编辑器目录、`*.log`、`.env`） | 初始化面板只提供一个通用勾选项，替用户决定技术栈会写出错误的忽略规则 | 是 |
| 21 | Open 表单主操作 | 字段下方独立 `Open Repository` 行 + hint + meta 行（右端 `⌘O` 键帽） | 路径字段占满整行，`Open Repository` 在下一行；不带 `⌘O` 提示，也不再有 hint 与 meta 行 | 按反馈简化：输入框占满、按钮另起一行、隐藏快捷键提示；空值时同一按钮先选目录 | 是 |
| 22 | 欢迎页尺寸 | 表单最大宽 600、窗口 1040×690 | 表单 520、窗口 900×600（最小 800×520） | 按反馈收窄：一倍屏幕下 900×600 足够 | 是 |
| 23 | 设置窗口 | 主窗口里的一屏 + 自绘 38 高标题栏（窗口灯、居中标题、右端版本号） | 独立普通窗口（`WindowKind::Normal`）、原生标题栏、标题 `Settings`、920×700 居中于屏幕；支持最小化、缩放与关闭，⌘W 关闭设置窗口 | 按反馈要求 Settings 使用自己的普通原生窗口，不作为主窗口子对话框；独立窗口在窗口切换器中单独显示 | 是（恢复主窗口内的一屏） |
| 24 | 欢迎窗口标题 | `gitui`（与主窗口同名） | `Welcome`（`welcome.windowTitle`，设置窗口为 `Settings`） | 两扇窗同名时无法在窗口菜单与切换器里区分；按反馈统一窗口模型 | 是 |
| 25 | 欢迎页底部提示条 | `.welcome-tip`：恢复上次会话 + 去 Settings › General | 不显示 | 按反馈去掉三个表单底部的提示条；相关文案键一并删除 | 是（恢复 `tipLead` / `tipRest` 与 `tip()`） |
| 26 | 最近仓库行 | 名称 + 分支 chip + 相对时间 + `×` 删除 | 名称 + 分支 tag + `更多` 菜单（Reveal in Finder / Copy Path / Remove from List），无时间 | 按反馈：移除时间、删除按钮改为菜单并承载更多动作 | 是（恢复时间与 `×`） |
| 27 | 齿轮图标 | 设计 sprite 的 `gear`：中心小圆 + 八条细放射线（更像星芒） | 双圆（轮体 r4.4 + 轴心 r1.7）+ 八颗厚齿（`design/icons.rs` 的 `gear`） | 按反馈"设置应该使用齿轮图标"；细放射线不像齿轮 | 是（换回原路径） |
| 28 | 欢迎页英雄区图标 | 64（圆角 15，图标 30） | 44（圆角 10，图标 20，`features/welcome/view.rs` 的 `APP_MARK`） | 按反馈"应用 Logo 太大了"；缩小后与标题、副标题的高度更均衡 | 是（改回 `APP_MARK = 64.0` 与 30/15） |
| 29 | 设置页分段线间距 | `.settings-sec` 上距 20、上内距 16；`.set-row` 下内距 11 | 分段线上距 9、上内距 16（`ui/setting_row.rs` 的 `SECTION_GAP_ABOVE` / `SECTION_PAD_TOP` / `ROW_PAD_V`） | 按反馈：分隔线上下视觉等距；上一行自带 11 的底部内距，所以分段上边距要比下内距小 | 是（改回 20/16） |
| 30 | 设置页分段标题 | `.settings-sec` 段标题 12.5/600 | 15/600、行高 20（`ui::TITLE` / `ui::TITLE_LINE`，`ui/setting_row.rs`） | 按反馈：分段标题不够明显；与 13pt 行标题拉开层级 | 是（改回 `ui::BODY`） |
| 31 | 设置窗口 Esc | 设置窗口默认可由 Esc 关闭 | Settings 普通窗口显式不因 Esc 关闭；快捷键页录制中 Esc 只取消录制（`app/windows.rs` 的 `settings_window_attributes`） | Esc 在快捷键页承担取消键，避免误关窗口 | 是（去掉 `close_on_escape: Some(false)`） |
| 32 | 快捷键录制卡位置 | `.shortcut-recorder` 在工具栏与三段列表之间 | 插在正在录制的那一行下方；该行被搜索过滤掉时才回落到列表顶部（`features/settings/shortcuts.rs` 的 `body`） | 按反馈：录制区应贴在要改键的 item 下方，而不是页面最上方 | 是 |
| 33 | 录制期间的菜单加速键与 ⌘W | 画板不含菜单栏，未覆盖 | 录制中临时移除应用菜单的可重映射加速键；Close Tab 菜单项不带加速键，⌘W 按窗口焦点关闭当前窗口或主窗口标签页 | 原生菜单加速键先于窗口按键表消费组合键；窗口焦点决定 ⌘W 的目标 | 是（删除 `shortcuts_enabled`） |
| 34 | 设置页页脚 | 底边 44 高通栏：左「自动保存」说明、右 `Done` | 整行移除，关窗只用系统关闭按钮（`features/settings/view.rs`） | 按反馈：去掉这一整行 | 是（恢复 `FOOTER_H` 与页脚子视图） |
| 35 | 录制卡的光标柱 | 画板里是静态竖条 | 录制中按 530ms 亮灭交替、结束恢复常亮（定时器在 `app/hotkeys.rs` 的 `install_settings_window`，卡片读 `blink_on`）；高度取 `ui::SECONDARY_LINE`（15）而不是画板的 16，并放进行高盒子里 | 按反馈：录制中光标应当闪烁，静态竖条看不出「正在等按键」；15 才能与 11pt 文字、键帽共用一条垂直中线 | 是（恒为 accent 并改回 16 会偏出文字行盒） |
| 36 | 仓库页签的状态点 | `.rtab` 活动页签的 5×5 `--accent` 圆点 | 6×6 的 `ok` 绿点，且未选中页签不画点（`ui/tabs.rs`） | 按反馈：选中项要能一眼看出来；`accent` 是极浅的选中底色，白底上看就是灰点。绿色与状态栏的 `ok` 一致 | 是（改回 5×5 `accent`） |
| 37 | 仓库页签的 × 显隐 | 活动页签平时透明度 0、hover 才出现 | 活动页签 0.35、非活动 0.55，指针进入页签（含页签内部视图）时 1.0（`ui/tabs.rs`） | 按反馈 + 可用性：0 不透明度一旦 hover 事件不达（窗口刚激活、指针未移动）就没有关闭入口；0.35 既保留层次又始终可点 | 是（把活动页签改回 0） |
| 38 | 标题栏 `+` 的命中区 | `.titlebar` 的 `+` 与图标同大 | 视觉仍 26×24，命中区向右/上/下各扩 4pt（`shell/mod.rs` 的 `hit_slop`） | 按反馈：加号在原尺寸下常被点到边上而无反应；向左不扩，避免盖住页签末尾的 × | 是（去掉 `hit_slop`） |
| 39 | 引用侧栏分支与远端行 | 分支名称显示完整斜杠路径并带分支图标；remote 展开后显示 fetch / push URL 行 | 分支树叶节点只显示当前层级名称且不显示分支图标；remote 下隐藏 URL 行，URL 仍可从 remote 右键菜单复制 | 按反馈简化分支与 remote 条目，树分组已表达路径层级 | 是（恢复完整路径、图标与 URL 行） |
| 40 | Git 扫描结果与命令面板 | 扫描结果卡有 `Select All` 与 `Add N Repositories`；命令面板画板展示打开仓库、最近仓库与命令 | 扫描复选框只控制命令面板搜索资格；空查询最多显示 5 条常用命令与 4 条仓库，输入后按「命令、打开、最近、已启用扫描」分组；欢迎页输入后搜索最近记录与全部扫描结果 | 扫描负责发现，实际打开动作负责更新历史；欢迎页查询所有发现结果，命令面板则遵守用户勾选的搜索资格 | 是（恢复批量加入历史与画板命令面板样例） |

| 41 | History 提交浏览与检查器 | History 画板把提交列表、选中提交 diff 和平铺文件列表并列显示；文件树没有完整设计 | 默认中心区只显示提交列表与 branch graph；历史行只在直接指向提交的分支节点显示徽标。右侧文件区在中间滚动，固定底部 Info 显示原提交头样式与精简元信息，不设额外外层内距；提交头底部留白收紧，使提交时间说明和引用行靠近提交摘要；多选项之间的底部分割线延伸至 Inspector 两侧。committer 与 author 身份或时间不同时显示提交时间，身份相同时省略重复姓名与邮箱；Branches and refs 标签与引用值合并为同一文本，统一使用正文颜色和字阶，并排在提交时间说明之后。短 SHA 与复制图标合为一个按钮，位于作者时间行右侧。单选保留 Cherry-pick / Revert，多选按 History 顺序逐项显示提交信息。文件树使用公共 Tree / Row，提供树/平铺切换、选中态、逐文件增删统计、复制路径与编辑器动作；Blame 放在单文件 Diff 工具栏并进入独立 board 19。点文件后中栏从 History 整栏切换为 Diff，点目录只展开或收起；提交右键菜单先同步选中提交再显示原生动作菜单；Diff 顶部返回保留提交和文件选择。左侧本地分支、远端分支、tag、stash 定位到相应 commit（`features/history/view.rs`、`inspector.rs`、`app/workspace.rs`、`shared/diff.rs`） | 图表只用分支尖端徽标表达 ref 位置；检查器将相关提交信息集中在可滚动 Info 中，同时保留文件树与引用；多选时逐项呈现元信息 | 是（恢复并列面板与文件平铺补丁选择）
| 42 | Diff 空白筛选 | File diff / Commit info 画板只有一个 `Show whitespace` 图标 | 除显示空格与 tab 的开关外，再提供独立 `Ignore whitespace changes` 开关；后者通过 Git `--ignore-all-space` 重新生成展示差异，复制、导出与统计仍用原始 patch（`shared/diff.rs`、`crates/git/src/diff.rs`） | 可视化空白与忽略空白影响的是不同数据：一个只改显示文本，一个改变 Git 差异匹配；把两者合并会造成筛选含义不清 | 是（移除单独图标并保留 Git 查询能力） |
| 43 | History 组合筛选 | 画板以分支多选、单作者选择与日历范围呈现；作者没有身份标识说明 | 作者以 email 区分并支持多选；顶栏用分支数、作者数显示已选数量，点击后打开窗口内锚定的多选浮层；逐项勾选只改草稿，点外部、按 Esc 或点关闭按钮后统一应用；`All x` 清除当前维度并立即应用、关闭浮层。日期端点仍用 `YYYY-MM-DD` 输入字段，路径、消息和日期在完整筛选浮层内编辑并应用（`features/history/view.rs`、`features/history/commands.rs`、`filters.rs`） | 顶栏空间有限，菜单显示已选数量即可；同名作者需要 email 身份区分 | 是（可换为日历范围控件） |
| 44 | Merge 冲突代码显示 | Ours / Result / Theirs 以冲突块为单位并排标注来源行，冲突标记有单独着色 | Base 条置于三列上方；三侧直接展示 index stage 文件并按路径语法高亮；Result 独立可编辑。Files 浮层使用公共 Tree / Segmented，在操作会话中保留已解决路径及只读结果快照。导航按文件计数，行级冲突块数量不显示（`features/merge/view.rs`、`features/merge/view/files.rs`、`ui/text_input.rs`） | NSH 没有三方合并 diff 组件，Git index 也不提供冲突块计数；展示真实 stage 内容与文件级解决状态，避免将行级差异误作写回结果 | 是（替换三侧显示组件） |
| 45 | 引用比较条与端点选择 | 比较板未画 ref 选择器、工作区端点与空差异反馈 | History 顶栏唯一的 `Compare` 按钮打开可搜索、可滚动的窗口，提供本地/远端分支、tag、stash 端点，2-dot/3-dot、Swap、工作区切换与清除；Done 一次应用草稿（`features/history/view.rs`、`app/workspace.rs`） | 引用数量较多时可搜索定位，并在确认后一次更新比较结果；退化态继续提供明确反馈 | 是（恢复原生菜单需重做比较窗口与对应状态） |
| 46 | 欢迎页路径输入提示 | Open 路径、Clone 目标目录、Init 目录各显示 `⌘V` 键帽 | 三个字段均不显示键帽（`features/welcome/view.rs`） | 按反馈减少输入框旁的快捷键提示；系统输入框仍支持粘贴 | 是（为三个字段恢复 `⌘V` 参数） |
| 47 | 命令面板遮罩 | 画板为 18% 压暗并模糊 2px | 遮罩完全透明，不模糊底层（features/search/view.rs、ui/dialog.rs） | 按本次明确要求优先采用普通弹窗的透明遮罩，保留模态拦截与 Escape 关闭 | 是（改回画板遮罩） |
| 48 | 命令面板空结果 | 画板未提供零匹配状态 | 居中搜索图标、未找到仓库或命令的提示和输入建议（features/search/view.rs、locales/*.json） | 按反馈明确搜索范围并改善无匹配时的反馈 | 是（按后续空态设计统一） |
| 49 | 命令面板搜索结果与键盘滚动 | 画板最多展示 9 条结果，未定义列表滚动 | 输入搜索时显示全部匹配项，结果区可滚动；↑/↓ 选中移出可视区时自动滚入视图。空查询仍显示 5 条常用命令与最多 4 个最近/已打开仓库 | 按反馈移除搜索结果数量上限，并保证键盘导航可到达全部匹配项 | 是（恢复 9 条截断） |
| 50 | Blame / 文件历史页面 | Board 19 没有 Diff 内容、历史行联动、重命名追踪与过滤行为 | Blame 占据完整中栏，右栏按 Changes / History 切换；历史项切换所选 revision，blame 行聚焦右侧提交；Diff 复用共享 `DiffView`。当前不显示没有筛选规则的漏斗按钮；删除提交的 blame 回退到第一父版本，文件历史使用 `--follow` 并逐提交记录实际路径（`features/history/blame.rs`、`ops.rs`、`crates/git`） | 沿用已验证的历史和差异组件，明确画板未覆盖的 revision / rename 行为，并隐藏无实现行为的过滤入口 | 是（为文件历史补充筛选规则后可恢复漏斗入口） |
| 50 | Changes 启动与工作区 Diff | Changes 画板展示已选文件的 Diff，未画启动时的提交列表 | Changes 工具栏保持选中，中栏初始显示 History 提交列表；右栏 Commit 面板包含可暂存、筛选、拖放移动的 Changelists 树，选择文件后中栏整栏切成 Diff（`app/workspace.rs`、`features/changes/view.rs`、`features/commit/view.rs`） | 沿用设计板的单视图 Diff 与 Commit / Changelists 样式，同时按明确的启动流程先展示提交图 | 是（回到独立 Changes 主区与 Diff 堆叠） |

| 51 | Changes 暂存动作 | 画板在 hunk 右端与代码行尾显示暂存按钮 | 逐行按钮位于对应变更行下方，原始 Git hunk 首个变更行还提供整段按钮（`ui/diff_surface.rs` 的 `staging_surface`） | NSH 0.6.0 的 DiffView 提供行下注释槽，没有任意代码行尾插槽；复用内置语法高亮，保留键盘可达的行与 hunk 操作 | 是（需要扩展 DiffView 行尾插槽） |
| 52 | History 作者与日期列宽 | 画板固定 96 / 58 | 默认 152 / 112，可通过列标题右缘拖拽调整（作者 80–320、日期 80–250；`features/history/view.rs`、`ui/table.rs`） | 为作者名和本地化日期提供可读空间，同时允许用户按仓库内容分配列宽 | 是（恢复固定画板宽度并移除拖拽手柄） |
| 53 | Blame 侧栏页签留白 | 页签栏高 33 | 40 高，垂直内距 6，保证 Changes / History 页签上下留白对称（`features/history/blame/panel.rs`） | 按反馈增大页签上方留白并保持上下节奏一致 | 是（恢复 33 高栏） |
| 54 | History 的 ⌘ 提交多选 | 06 Multi-commit diff 未画进入手势与单纯文件列表状态 | `⌘` 点击只在 History 列表切换提交多选，右侧 Inspector 汇总所选提交的文件路径与增删统计，并在 Info 中按 History 顺序逐项显示提交头、committer 差异与统一正文样式的 Branches and refs 标签与引用值；提交项底部的分割线延伸至 Inspector 两侧；中栏保持提交列表，不进入比较 diff（`features/history/commands.rs`、`features/history/inspector.rs`） | 按用户确认将提交多选与引用比较、完整 diff 分开；无需切换主视图即可查看多个提交影响的文件与元信息 | 是（恢复多提交 diff 入口与 Inspector 单提交文件树）
| 55 | Refs / Reflog 工具栏与右栏 | 工具栏显示 Branch / Stash；Reflog 画板右栏显示 Undo / History 页签和恢复卡 | 工具栏移除 Branch，Stash 位于 Reflog 左侧并打开 Stash Changes，Reflog 与 Fetch／Pull／Push 使用相同按钮外观；Refs 主区保留 Refs / Reflog 页签，Reflog 页隐藏创建操作；Reflog 选择记录后与 History 共用提交检查器的文件树、Tree／Flat 和展开／折叠控件、message／description 与底部信息（`shell/mod.rs`、`features/refs/view.rs`、`features/history/inspector.rs`、`app/workspace.rs`） | 按用户要求为 Reflog 增加直接入口与提交详情，并统一检查器 | 是（可恢复 Branch / Stash 并切回 Undo 检查器） |
| 56 | 同步面板完成状态 | 成功状态显示通用 `Done`，具体输出只在日志区 | Fetch / Pull / Push 的完成行显示已是最新、确认到的引用与提交数量；Push 部分拒绝列出已更新与被拒绝数量，无法确认的提交数标记为未确认；原始 Git 输出保留在日志区（`features/sync/commands.rs`、`features/sync/state.rs`、`features/sync/view.rs`） | 让操作完成状态直接回答远端目标和结果，不要求用户从 Git 日志里推断 | 是（恢复通用 `Done` 完成行） |
| 57 | Credentials provider 行 | Credentials 画板提供 GitHub / GitLab 账号连接与连接状态 | 只显示 Git credential helper、SSH 偏好与本机 SSH 密钥；Git 按需调用 AskPass，不建立 provider 账号连接（`apps/client/src/features/settings/credentials.rs`、`apps/client/src/askpass.rs`） | 按用户确定的产品范围只依赖 Git CLI，避免呈现没有 provider API 的账号连接入口 | 否（恢复该入口需另行实现 GitHub / GitLab OAuth 与账号管理） |
| 58 | 应用自绘弹窗统一 | 22 对话框页脚 42、body 内距 11×14，各画板分别定义表单与同步面板 | 通用表单、Edit Commit Message、Squash、Hook 日志、凭据和设置确认共用 `ui/dialog.rs::dialog_panel`；标题 44、底栏 48、正文内距 16，操作按钮 Regular 28；同步对齐同一标题栏／底栏；Compare 的操作按钮使用 Regular（`features/search/form_dialog.rs`、`features/commit`、`features/sync/view.rs`） | 按用户要求统一应用弹窗的结构与控件比例，去除不可见字段的额外间距，并为全部输入提供本地化 placeholder | 是（恢复各页面自绘面板与原尺寸） |

## 3. 已澄清：不算偏离

复核过、画板与实现一致或差异不成立的条目记在这里，避免下次重新讨论。

| 项 | 结论 | 依据 |
|---|---|---|
| 克隆表单协议分段 | 已恢复为 HTTPS / SSH 分段，只切换 URL 占位形态，不改克隆参数 | 原 #13 于 2026-11-16 删除；协议仍由 URL 决定 |
| 侧栏宽 | 画板 232，实现 232，一致 | `.sidebar{width:232px}`；`metrics.rs:18` |
| 状态栏高 | 画板 24，实现 24，一致 | `.statusbar{height:24px}`；`native::STATUS` |
| 提交行高 | 画板末次覆盖 28，实现 28，一致 | `.commit` 的四次声明依次为 38 / 32 / 30 / 28；`metrics::COMMIT_ROW_H` |
| 弹窗圆角 | 画板 10，实现 10，一致 | `.dialog{border-radius:10px}`；`RADIUS_PANEL + 2`（`ui/dialog.rs`） |
| #11 命令面板位置 | 位置按画板保持 620×512、距顶 92、水平居中；透明遮罩按 #47 记录 | `features/search/view.rs` 使用 `modal_open_transparent` 与固定面板尺寸 |
| 侧栏材质 | 画板与实现都是纯色，未用 `macos_backdrop` | 全库无 `macos_backdrop` 调用；材质留待评估 |
| 桌面渐变 `desk1` / `desk2` | 不进窗口内容，不移植 | 画板 `.desktop` 的背景，只属于画板外壳 |

## 4. 维护

- 新增偏离：先确认是「装不下」还是「没做」。「没做」进待办，不进这张表。
- 回退成本写清楚：改常量能回去的写「是」，要重画组件的写清代价。
- 画板改版后，逐条过一遍本表：可能有些条目已经不需要了，删掉并在提交信息里说明。
- 逐页的临时差异（某页少一个按钮之类）不进这张表，写在该页代码的注释里；这里只收
  设计系统级别的、会影响多个页面的差异。
