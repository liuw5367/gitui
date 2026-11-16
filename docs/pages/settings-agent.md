# Agent 设置 — 布局、状态与按钮

- **画板**：23b Settings · Agent（设置窗口第 3 项 Agent；有 Provider protocol / Endpoint / Model / Privacy 四段十二行；无 Anthropic 变体、无连接失败、无密钥显隐）
- **来源**：`designs/graft-git/project/SettingsAgent.dc.html` 与共享样式表 `assets/0a859d3b7b7fd8e4958bfb63b7af439a.bin`。只按画板描述目标，不取自实现。
- **实现差异**：见 [../design/DEVIATIONS.md](../design/DEVIATIONS.md)，本文不重复。

本分区是设置窗口的第 3 项（`Application` 组的 `Agent`）。外壳与 23 General 同一套：`.settings .window`
`width:min(920px,100%)`、`height:min(700px,calc(100vh - 48px))`，画布 1440×900、`.desktop` 留白 24，因此窗口
**920×700**；正文 616，右栏 710，内容列 654，可视高 568。内容区只换成 Agent 的四段，切法仍是“标题 + 副标题 +
逐段表单，每段切成左侧标签列与右侧控件列”。

## 1. 骨架

```
┌──────────────────────────────────────────────────────────────────────────────────────────────┐
│ ● ● ●                                      Settings                   gitui 0.9.2            │
├─────────────────────────┬────────────────────────────────────────────────────────────────────┤
│                         │ Agent                                                              │
│ ▣ gitui                 │ Model endpoint, credentials, and protocol for the gitui agent.     │
│   Version 0.9.2         │                                                                    │
│                         │ Provider protocol                                                  │
│ Application             │ Pick the wire format your endpoint speaks.                         │
│  [gear]   General       │Protocol          [ OpenAI | Anthropic ]                            │
│  [keys]   Shortcuts     │Wire format       POST /v1/chat/completions                         │
│* [branch] Agent         │ Endpoint                                                           │
│ Git                     │ Any OpenAI- or Anthropic-compatible base URL,                      │
│  [branch] Git           │ including self-hosted gateways.                                    │
│  [lock]   Credentials   │Base URL          [ https://api.openai.com          ]               │
│ About                   │Resolved          https://api.openai.com/v1/chat/completions        │
│  [i]      About gitui   │App key           [ sk-proj-···············7d1f    ]                │
│                         │Connection        [Test Connection]  Connected · 42 models          │
│                         │ ... Model, Privacy ...                                             │
├──────────────────────────────────────────────────────────────────────────────────────────────┤
│ ● Changes are saved automatically                                                   [ Done ] │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
```

| 区域 | 位置 | 尺寸（画板值） | 内容 |
|---|---|---|---|
| 标题栏 | 顶部通栏 | 高 38；内距 0×13 | 窗口灯 12×3、居中 `Settings` 12.5/600、右端 `gitui 0.9.2` 11 |
| 左侧分区树 | 标题栏下左 | 宽 208；内距 12×8；行高 28 | 六项三组，`Agent` 是当前项 |
| 内容区 | 标题栏下右 | 宽 710；内距 24×28 → 内容列 654、可视高 568 | `Agent` 标题、副标题、四段十二行 |
| 页脚 | 底部通栏 | 高 44；内距 0×16 | 6 状态点、自动保存说明、右端 `Done` |

四段十二行按“行 57 / 段头 46 / 段距 37”累加约 1040，超过 568，右栏滚动。图中 `*` 是本分区，`...` 是被折叠的段。

## 2. 外壳与左侧分区树

左树六项三组与其它五块板逐字相同：`Application` 下 General（齿轮）、Shortcuts（键帽）、Agent（分支）；
`Git` 下 Git（分支）、Credentials（锁）；`About` 下 About gitui（信息）。本板的当前项是 `Agent`，`.on` 取
样式表末次声明 `background:var(--accent)` + 白字白图标 + 500。标题栏 38 高、标题绝对居中、右端版本号 11 `text3`；
组头 `.sg` 24 高、10.5/600 `text3`；行 28 高、内距 0×8、圆角 6、字号 13、图标盒 16 宽。本板没有二级页签。

## 3. 内容头与行网格

标题 `Agent` 17/700、字距 −.015em；副标题 `Model endpoint, credentials, and protocol for the gitui agent.`
11.5 `text3`、上距 4。第一段 `Provider protocol` 带内联 `border-top:0;margin-top:14px;padding-top:0`，其余三段
上距 20、上内距 16、上边框 1px `--line`；段标题 12.5/600、下距 2，段说明 10.5 `text3`、下距 12。行间距 14、
行内距 `11px 0`、下边框 1px `--line-soft`；标签 12.5/500，说明 10.5 `text3`、上距 2、行高 1.4。控件列贴右，
因此标签列 = 654 − 控件宽 − 14：写死 280 的两个字段对应 360，写死 80 与 90 的分别对应 560 与 550。

## 4. Provider protocol 与 Endpoint

**Provider protocol**（`Pick the wire format your endpoint speaks. Requests are built to match it.`）两行：
`Protocol` → 分段 `OpenAI`（选中）/ `Anthropic`，说明 `OpenAI chat completions or Anthropic messages.`；
`Wire format` → `padding-top:0` 贴紧上行，右列只有一行 `.hint` `POST /v1/chat/completions`，说明
`Anthropic uses /v1/messages and x-api-key; OpenAI uses /v1/chat/completions and Bearer auth.` 这是协议的回显。

**Endpoint**（`Any OpenAI- or Anthropic-compatible base URL, including self-hosted gateways.`）四行：`Base URL` →
280 字段 `https://api.openai.com`，说明 `Scheme and host, with any path prefix. No trailing slash.`；
`Resolved endpoint` → `padding-top:0`，右列 mono `.hint` `https://api.openai.com/v1/chat/completions`，
说明 `What the first request will hit.`；`App key` → 280 宽密码字段，板上 20 个遮点 + 尾号 `7d1f`，
说明 `Stored in the macOS Keychain, never in the repository.`；`Connection` → `padding-top:0`，右列
`Test Connection`（`btn small` 22 高）+ `--ok` 绿字 `Connected · 42 models`，说明 `Verifies the key and lists models.`。

四行里两行是“输入行 + 回显行”成对贴紧的写法，280 是本板最宽的控件宽度。

## 5. Model 与 Privacy

**Model**（`Used for chat, commit messages, and change explanations.`）三行：`Model` → 原生 `select`（26 高、
内距 0×8、字号 11），板上 `gpt-4.1`（选中）/ `gpt-4o-mini` / `o4-mini`，说明 `Fetched from the endpoint above.`；
`Temperature` → 80 宽字段 `0.2`，说明 `Lower is more deterministic for commit messages.`；`Max tokens` → 90 宽
字段 `4096`，说明 `Cap for a single reply.`。两个数字字段同用 `set-field`（26 高、内距 0×8、圆角 6、input 11）。

**Privacy**（`What leaves this machine when the agent runs.`）三行：`Send diffs` → 勾选；`Send repository path` →
未勾选；`Local log` → 右列 `Open Log`（`btn small` 的 `<a>`，指向 `SettingsCredentials.dc.html`）。三句说明分别是
`Diff hunks are included with each request.`、`Absolute paths are replaced with the repository root when off.`、
`Requests and tool results are recorded for review.`。`Open Log` 落在 27，但该板没有日志区。

## 6. 状态与切换

| 轴 | 取值 | 画板 |
|---|---|---|
| Protocol | OpenAI / Anthropic | 只画 OpenAI |
| Connection | 未测 / 已连接 / 失败 | 只画已连接（`Connected · 42 models`） |
| Model | 端点返回的列表 | 只画三项，`gpt-4.1` 选中 |
| Send diffs / Send path | 各自勾 / 不勾 | 勾 / 不勾 |
| 左侧分区 | 六项 | 只画 Agent 选中 |

```
分区：General ⇄ Shortcuts ⇄ Agent ⇄ Git ⇄ Credentials ⇄ About gitui（只替换右栏）
协议：OpenAI ⇄ Anthropic（只改 Wire format 与 Resolved endpoint 的回显）
离开：Done → Workspace.dc.html；Open Log → SettingsCredentials.dc.html
```

## 7. 画板没覆盖的组合（待补设计）

1. **Anthropic 变体**：只有说明文字交代 `/v1/messages` 与 `x-api-key`；切过去后 Wire format、Resolved endpoint
   与模型列表各显示什么，没有画。
2. **密钥操作**：没有显示/隐藏、替换、清除、复制，也没有“Keychain 里已有旧 key”的提示。
3. **连接失败**：401、超时、DNS 失败、自签证书分别显示在哪，失败后 `Test Connection` 是否仍可用，没有画。
4. **测试中**：按下后的进度、按钮是否禁用、`Connected · 42 models` 是否清空，没有画。
5. **模型列表为空 / 数值越界**：还没连过端点时下拉显示什么；`Temperature` 与 `Max tokens` 的范围与非法输入，没有画。
6. **窄窗口**：280 字段与 80/90 字段在窗口缩到 920 以下时如何收，没有画。

## 8. 按钮与链接清单

| # | 位置 | 文案 | 形态 | 作用 | 随状态变化 |
|---|---|---|---|---|---|
| 1 | Endpoint 段 Connection 行 | Test Connection | `btn small` 22 高 | 校验 key 并列出模型 | 只有常态，结果以右侧绿字呈现 |
| 2 | Privacy 段 Local log 行 | Open Log | `btn small` 的 `<a>` | 去 `SettingsCredentials.dc.html` | 常驻 |
| 3 | 页脚右端 | Done | `btn` 24 高、字号 12 | 关设置回主窗口 | 常驻 |
| 4 | 左侧树 | 六项 | 整行链接 28 高，图标 14 | 切分区，替换右栏 | 当前项 `--accent` 底 + 白字 |

- 分段 `OpenAI | Anthropic` 是选项组，不计入按钮；`Wire format` 与 `Resolved endpoint` 是只读 `.hint`，
  板上没给 `aria-label`，无障碍名按可见文本取。

## 9. 输入与选项

| 字段 | 形态 | 默认值（画板） | 说明 |
|---|---|---|---|
| Protocol | 分段两项 | `OpenAI` | 另一项 `Anthropic`，切换只改回显 |
| Wire format | 只读 hint | `POST /v1/chat/completions` | 随协议变化，不是输入 |
| Base URL | 字段 280 宽 | `https://api.openai.com` | 不带尾斜杠，可含路径前缀 |
| Resolved endpoint | 只读 mono hint | `https://api.openai.com/v1/chat/completions` | 由 Base URL 与协议拼出 |
| App key | 密码字段 280 宽 | `sk-proj-` + 20 个遮点 + `7d1f` | 存 Keychain，不进仓库 |
| Model | 原生 select 26 高 | `gpt-4.1` | 另两项 `gpt-4o-mini`、`o4-mini` |
| Temperature | 字段 80 宽 | `0.2` | 越低越确定，用于提交信息 |
| Max tokens | 字段 90 宽 | `4096` | 单次回复上限 |
| Send diffs | 原生复选 | 勾 | diff hunk 随请求发出 |
| Send repository path | 原生复选 | 不勾 | 关闭时绝对路径换为仓库根 |

## 10. 与其它画板的关系

| 画板 | 关系 |
|---|---|
| 23 / 24 / 25 / 26 / 27 | 板内五个 `href`：`Settings.dc.html`、`SettingsShortcuts.dc.html`、`SettingsGit.dc.html`、`SettingsCredentials.dc.html`、`SettingsAbout.dc.html` |
| 27 Settings · Credentials & tools | `Local log` 的 `Open Log` 是第二个 `href="SettingsCredentials.dc.html"`（该板没有日志区） |
| 12 Agent panel / 13 Agent permissions | 本板的协议、模型与隐私开关是那两块板的运行前提 |
| 25 Settings · Shortcuts | `Agent` 组的 `Focus Agent`、`Approve & Run`、`Agent Permissions` |
